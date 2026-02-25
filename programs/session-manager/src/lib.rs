use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{commit, delegate, ephemeral};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::{commit_accounts, commit_and_undelegate_accounts};
pub mod constants;
pub mod errors;
pub mod state;
use constants::{DUEL_CAMPAIGN_LEVEL, GAUNTLET_CAMPAIGN_LEVEL};

use errors::SessionManagerError;
use gameplay_state::program::GameplayState;
use gameplay_state::state::{GameState, MapEnemies};
use map_generator::program::MapGenerator;
use map_generator::state::{GeneratedMap, MapConfig as MapConfigAccount};
use player_inventory::program::PlayerInventory;
use state::{GameSession, SessionCounter, EMPTY_STATE_HASH};

declare_id!("6w1XVMSTRmZU9AWCKVvKohGAHSFMENhda7vqhKPQ8TPn");

/// Player Profile program ID for cross-program account validation
/// Must match the declare_id! in player-profile/src/lib.rs
/// Ch3bbL1oQk2z5rX1jiun3KuSWZqnXZ1MnrfrtKj4MKun
pub const PLAYER_PROFILE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xad, 0xb2, 0xb9, 0x6d, 0x55, 0xae, 0x29, 0xf0, 0x53, 0xa1, 0x15, 0xe4, 0xae, 0x95, 0x65, 0xc0,
    0x75, 0x77, 0xfe, 0x3d, 0x37, 0xc8, 0x4b, 0xb3, 0x7e, 0xd7, 0x82, 0x79, 0x48, 0x5a, 0x98, 0x0d,
]);

/// POI System program ID for manual CPI.
/// Must match the declare_id! in poi-system/src/lib.rs
/// KiT25b86BSAF8yErcWwyuuWNaoXMpNf859NjH41TpSj
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x04, 0xcb, 0x52, 0x15, 0x87, 0x19, 0x4d, 0x2b, 0xbe, 0x24, 0xa5, 0xa7, 0xae, 0xc7, 0xc2, 0x79,
    0x1e, 0xa8, 0x59, 0xd6, 0xc2, 0x7b, 0x44, 0x05, 0x0d, 0x53, 0x85, 0xb7, 0x4b, 0x8b, 0xc2, 0x60,
]);

/// Map Generator program ID for CPI.
/// Must match the declare_id! in map-generator/src/lib.rs
/// GCy5GqvnJN99rgGtV6fMn8NtL9E7RoAyHDGzQv8me65j
pub const MAP_GENERATOR_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xe1, 0xf0, 0x18, 0x72, 0xcf, 0x4e, 0x1d, 0xea, 0xe0, 0x2f, 0x0a, 0xb0, 0xe8, 0xbf, 0x4b, 0x0c,
    0xf5, 0xb2, 0x05, 0xc5, 0x47, 0x61, 0x12, 0x2d, 0x49, 0xda, 0x54, 0xc1, 0xf5, 0xd0, 0xac, 0x6e,
]);

/// Discriminator for player_profile::consume_run instruction.
/// Computed as sha256("global:consume_run")[..8].
///
/// NOTE: This is manually specified because session-manager already has a
/// manual PlayerProfile struct (avoiding circular deps). If player-profile's
/// consume_run instruction changes, this must be updated.
pub const CONSUME_RUN_DISCRIMINATOR: [u8; 8] = [0x6b, 0x65, 0x36, 0x52, 0x84, 0x9c, 0x0f, 0x22];

/// Discriminator for poi_system::initialize_map_pois instruction.
/// Computed as sha256("global:initialize_map_pois")[..8].
///
/// NOTE: This is manually specified because session-manager cannot depend on poi-system
/// (circular dependency). If poi-system's initialize_map_pois instruction changes, this must be updated.
pub const INITIALIZE_MAP_POIS_DISCRIMINATOR: [u8; 8] =
    [0xa8, 0xec, 0xff, 0x37, 0xee, 0xd2, 0x19, 0xfb];
pub const DISCOVER_VISIBLE_WAYPOINTS_DISCRIMINATOR: [u8; 8] =
    [0x3b, 0x26, 0x6a, 0x00, 0x3a, 0xb1, 0x50, 0xfc];
pub const SESSION_MANAGER_AUTHORITY_SEED: &[u8] = b"session_manager_authority";
fn local_delegate_config() -> DelegateConfig {
    DelegateConfig::default()
}

#[ephemeral]
#[program]
pub mod session_manager {
    use super::*;

    /// Initializes the global session counter (one-time admin operation).
    pub fn initialize_counter(ctx: Context<InitializeCounter>) -> Result<()> {
        let counter = &mut ctx.accounts.session_counter;
        counter.count = 0;
        counter.bump = ctx.bumps.session_counter;

        Ok(())
    }

    /// Starts a new game session with all dependencies initialized (Game State, Inventory, etc.).
    ///
    /// Validates:
    /// - Player has available runs > 0
    /// - Campaign level is within player's unlocked range (1 to highest_level_unlocked)
    /// - No existing session for this (player, level) pair
    ///
    /// Actions:
    /// - Creates session with snapshot of player's active_item_pool
    /// - Generates the map via CPI to map-generator
    /// - Initializes game state via CPI to gameplay-state
    /// - Initializes inventory via CPI to player-inventory
    /// - Emits SessionStarted event
    pub fn start_session(ctx: Context<StartSession>, campaign_level: u8) -> Result<()> {
        let player_profile = &ctx.accounts.player_profile;

        // Validate campaign level is within range
        require!(
            (1..=40).contains(&campaign_level),
            SessionManagerError::InvalidCampaignLevel
        );

        // Validate player has available runs
        require!(
            player_profile.available_runs > 0,
            SessionManagerError::NoAvailableRuns
        );

        // Validate level is unlocked
        require!(
            campaign_level <= player_profile.highest_level_unlocked,
            SessionManagerError::LevelNotUnlocked
        );

        // Consume one run from player profile via CPI
        consume_run_cpi(
            &ctx.accounts.player_profile_program,
            &ctx.accounts.player_profile.to_account_info(),
            &ctx.accounts.player.to_account_info(),
        )?;

        let counter = &mut ctx.accounts.session_counter;
        let clock = Clock::get()?;
        let session_player = ctx.accounts.player.key();
        let session_signer_key = ctx.accounts.session_signer.key();

        // Increment counter and get new session ID
        counter.count = counter
            .count
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;

        {
            let session = &mut ctx.accounts.game_session;
            session.player = session_player;
            session.session_id = counter.count;
            session.campaign_level = campaign_level;
            session.started_at = clock.unix_timestamp;
            session.last_activity = clock.unix_timestamp;
            session.is_delegated = false;
            session.state_hash = EMPTY_STATE_HASH;
            session.bump = ctx.bumps.game_session;
            // Copy active_item_pool from profile to session
            session.active_item_pool = player_profile.active_item_pool;
            // Store session key signer pubkey
            session.session_signer = session_signer_key;
            session.settled = false;
            session.settled_victory = false;
            session.settled_at = 0;
        }

        // 1. Generate Map
        map_generator::cpi::generate_map(
            CpiContext::new(
                ctx.accounts.map_generator_program.to_account_info(),
                map_generator::cpi::accounts::GenerateMap {
                    payer: ctx.accounts.player.to_account_info(),
                    session: ctx.accounts.game_session.to_account_info(),
                    map_config: ctx.accounts.map_config.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
        )?;

        // 2. Deserialize Generated Map to get dimensions and spawn
        // We borrow the account info to read the data just written
        let map_info = ctx.accounts.generated_map.to_account_info();
        let map_data = map_info.try_borrow_data()?;
        let mut map_slice: &[u8] = &map_data;
        // Skip 8-byte discriminator
        if map_slice.len() < 8 {
            return Err(ProgramError::InvalidAccountData.into());
        }
        // AccountDeserialize handles discriminator check, so we can just pass the slice?
        // GeneratedMap implements AccountDeserialize via #[account]
        // But the data in map_slice starts with discriminator.
        // Let's use try_deserialize directly.
        let generated_map = GeneratedMap::try_deserialize(&mut map_slice)?;

        let width = generated_map.width;
        let height = generated_map.height;
        let start_x = generated_map.spawn_x;
        let start_y = generated_map.spawn_y;

        // Drop the borrow so we can use the account in next CPI
        drop(map_data);

        // 3. Initialize Game State
        gameplay_state::cpi::initialize_game_state(
            CpiContext::new(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::InitializeGameState {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    game_session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    map_enemies: ctx.accounts.map_enemies.to_account_info(),
                    player: ctx.accounts.player.to_account_info(),
                    session_signer: ctx.accounts.session_signer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            width,
            height,
            start_x,
            start_y,
        )?;

        // 4. Initialize Inventory for this session
        // Each session gets its own inventory (PDA derived from session key).
        // This ensures clean inventory state per run and allows concurrent sessions.
        // IMPORTANT: Use session_signer as the inventory owner since all gameplay
        // transactions (equip, fuse, etc.) are signed by the session key signer.
        player_inventory::cpi::initialize_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::InitializeInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                session: ctx.accounts.game_session.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
        ))?;

        // 5. Initialize POI System via manual CPI (to avoid circular dependency)
        // Act is 1-4, derived from campaign level (10 levels per act)
        let act = (campaign_level - 1) / 10 + 1;
        let week = 1u8; // Always start at week 1
        let poi_seed = clock.unix_timestamp as u64; // Use timestamp as seed for POI generation

        initialize_map_pois_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            act,
            week,
            poi_seed,
        )?;

        // Apply spawn-time waypoint discovery using radius 6 around initial position.
        discover_visible_waypoints_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.session_signer.to_account_info(),
            6,
        )?;

        emit!(SessionStarted {
            player: session_player,
            session_id: counter.count,
            campaign_level,
            session_signer: session_signer_key,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Starts a duel session with fixed mid difficulty.
    ///
    /// This path is intentionally decoupled from campaign unlock progression:
    /// - Uses a fixed campaign level (DUEL_CAMPAIGN_LEVEL) for balance.
    /// - Derives duel seed on-chain.
    /// - Does not consume campaign runs.
    pub fn start_duel_session(ctx: Context<StartDuelSession>) -> Result<()> {
        let player_profile = &ctx.accounts.player_profile;
        let campaign_level = DUEL_CAMPAIGN_LEVEL;

        let counter = &mut ctx.accounts.session_counter;
        let clock = Clock::get()?;
        let session_player = ctx.accounts.player.key();
        let session_signer_key = ctx.accounts.session_signer.key();

        counter.count = counter
            .count
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;

        let duel_seed = derive_pvp_seed(
            b"duel_seed",
            &clock,
            counter.count,
            &session_player,
            &session_signer_key,
        );

        {
            let session = &mut ctx.accounts.game_session;
            session.player = session_player;
            session.session_id = counter.count;
            session.campaign_level = campaign_level;
            session.started_at = clock.unix_timestamp;
            session.last_activity = clock.unix_timestamp;
            session.is_delegated = false;
            session.state_hash = EMPTY_STATE_HASH;
            session.bump = ctx.bumps.game_session;
            session.active_item_pool = player_profile.active_item_pool;
            session.session_signer = session_signer_key;
            session.settled = false;
            session.settled_victory = false;
            session.settled_at = 0;
        }

        map_generator::cpi::generate_map_with_seed(
            CpiContext::new(
                ctx.accounts.map_generator_program.to_account_info(),
                map_generator::cpi::accounts::GenerateMap {
                    payer: ctx.accounts.player.to_account_info(),
                    session: ctx.accounts.game_session.to_account_info(),
                    map_config: ctx.accounts.map_config.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            duel_seed,
        )?;

        let map_info = ctx.accounts.generated_map.to_account_info();
        let map_data = map_info.try_borrow_data()?;
        let mut map_slice: &[u8] = &map_data;
        if map_slice.len() < 8 {
            return Err(ProgramError::InvalidAccountData.into());
        }
        let generated_map = GeneratedMap::try_deserialize(&mut map_slice)?;

        let width = generated_map.width;
        let height = generated_map.height;
        let start_x = generated_map.spawn_x;
        let start_y = generated_map.spawn_y;
        drop(map_data);

        gameplay_state::cpi::initialize_game_state(
            CpiContext::new(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::InitializeGameState {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    game_session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    map_enemies: ctx.accounts.map_enemies.to_account_info(),
                    player: ctx.accounts.player.to_account_info(),
                    session_signer: ctx.accounts.session_signer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            width,
            height,
            start_x,
            start_y,
        )?;

        let session_manager_authority_signer: &[&[&[u8]]] = &[&[
            SESSION_MANAGER_AUTHORITY_SEED,
            &[ctx.bumps.session_manager_authority],
        ]];
        gameplay_state::cpi::configure_run_mode(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ConfigureRunMode {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    session_manager_authority: ctx
                        .accounts
                        .session_manager_authority
                        .to_account_info(),
                },
                session_manager_authority_signer,
            ),
            gameplay_state::state::RunMode::Duel,
            3,
        )?;

        player_inventory::cpi::initialize_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::InitializeInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                session: ctx.accounts.game_session.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
        ))?;

        let act = (campaign_level - 1) / 10 + 1;
        let week = 1u8;
        let poi_seed = duel_seed;

        initialize_map_pois_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            act,
            week,
            poi_seed,
        )?;
        discover_visible_waypoints_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.session_signer.to_account_info(),
            6,
        )?;

        emit!(SessionStarted {
            player: session_player,
            session_id: counter.count,
            campaign_level,
            session_signer: session_signer_key,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Starts a gauntlet session using the same fixed difficulty profile as Duels.
    pub fn start_gauntlet_session(ctx: Context<StartGauntletSession>) -> Result<()> {
        let player_profile = &ctx.accounts.player_profile;
        let campaign_level = GAUNTLET_CAMPAIGN_LEVEL;

        let counter = &mut ctx.accounts.session_counter;
        let clock = Clock::get()?;
        let session_player = ctx.accounts.player.key();
        let session_signer_key = ctx.accounts.session_signer.key();

        counter.count = counter
            .count
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;

        let gauntlet_seed = derive_pvp_seed(
            b"gauntlet_seed",
            &clock,
            counter.count,
            &session_player,
            &session_signer_key,
        );

        {
            let session = &mut ctx.accounts.game_session;
            session.player = session_player;
            session.session_id = counter.count;
            session.campaign_level = campaign_level;
            session.started_at = clock.unix_timestamp;
            session.last_activity = clock.unix_timestamp;
            session.is_delegated = false;
            session.state_hash = EMPTY_STATE_HASH;
            session.bump = ctx.bumps.game_session;
            session.active_item_pool = player_profile.active_item_pool;
            session.session_signer = session_signer_key;
            session.settled = false;
            session.settled_victory = false;
            session.settled_at = 0;
        }

        map_generator::cpi::generate_map_with_seed(
            CpiContext::new(
                ctx.accounts.map_generator_program.to_account_info(),
                map_generator::cpi::accounts::GenerateMap {
                    payer: ctx.accounts.player.to_account_info(),
                    session: ctx.accounts.game_session.to_account_info(),
                    map_config: ctx.accounts.map_config.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            gauntlet_seed,
        )?;

        let map_info = ctx.accounts.generated_map.to_account_info();
        let map_data = map_info.try_borrow_data()?;
        let mut map_slice: &[u8] = &map_data;
        if map_slice.len() < 8 {
            return Err(ProgramError::InvalidAccountData.into());
        }
        let generated_map = GeneratedMap::try_deserialize(&mut map_slice)?;

        let width = generated_map.width;
        let height = generated_map.height;
        let start_x = generated_map.spawn_x;
        let start_y = generated_map.spawn_y;
        drop(map_data);

        gameplay_state::cpi::initialize_game_state(
            CpiContext::new(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::InitializeGameState {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    game_session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    map_enemies: ctx.accounts.map_enemies.to_account_info(),
                    player: ctx.accounts.player.to_account_info(),
                    session_signer: ctx.accounts.session_signer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            width,
            height,
            start_x,
            start_y,
        )?;

        let session_manager_authority_signer: &[&[&[u8]]] = &[&[
            SESSION_MANAGER_AUTHORITY_SEED,
            &[ctx.bumps.session_manager_authority],
        ]];
        gameplay_state::cpi::configure_run_mode(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ConfigureRunMode {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    session_manager_authority: ctx
                        .accounts
                        .session_manager_authority
                        .to_account_info(),
                },
                session_manager_authority_signer,
            ),
            gameplay_state::state::RunMode::Gauntlet,
            5,
        )?;

        player_inventory::cpi::initialize_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::InitializeInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                session: ctx.accounts.game_session.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
        ))?;

        let act = (campaign_level - 1) / 10 + 1;
        let week = 1u8;
        let poi_seed = gauntlet_seed;

        initialize_map_pois_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            act,
            week,
            poi_seed,
        )?;
        discover_visible_waypoints_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.session_signer.to_account_info(),
            6,
        )?;

        emit!(SessionStarted {
            player: session_player,
            session_id: counter.count,
            campaign_level,
            session_signer: session_signer_key,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Delegates gameplay-state account to the MagicBlock delegation program.
    pub fn delegate_game_state(ctx: Context<DelegateGameState>, campaign_level: u8) -> Result<()> {
        let game_session_key =
            derive_campaign_session_pda(&ctx.accounts.player.key(), campaign_level);
        let (expected_game_state, _) = Pubkey::find_program_address(
            &[b"game_state", game_session_key.as_ref()],
            &gameplay_state::ID,
        );
        require_keys_eq!(
            ctx.accounts.game_state.key(),
            expected_game_state,
            SessionManagerError::Unauthorized
        );
        let game_state_seeds: &[&[u8]] = &[b"game_state", game_session_key.as_ref()];
        ctx.accounts.delegate_game_state(
            &ctx.accounts.player,
            game_state_seeds,
            local_delegate_config(),
        )?;
        Ok(())
    }

    /// Delegates map-enemies account to the MagicBlock delegation program.
    pub fn delegate_map_enemies(
        ctx: Context<DelegateMapEnemies>,
        campaign_level: u8,
    ) -> Result<()> {
        let game_session_key =
            derive_campaign_session_pda(&ctx.accounts.player.key(), campaign_level);
        let (expected_map_enemies, _) = Pubkey::find_program_address(
            &[MapEnemies::SEED_PREFIX, game_session_key.as_ref()],
            &gameplay_state::ID,
        );
        require_keys_eq!(
            ctx.accounts.map_enemies.key(),
            expected_map_enemies,
            SessionManagerError::Unauthorized
        );
        let map_enemies_seeds: &[&[u8]] = &[MapEnemies::SEED_PREFIX, game_session_key.as_ref()];
        ctx.accounts.delegate_map_enemies(
            &ctx.accounts.player,
            map_enemies_seeds,
            local_delegate_config(),
        )?;
        Ok(())
    }

    /// Delegates generated-map account to the MagicBlock delegation program.
    pub fn delegate_generated_map(
        ctx: Context<DelegateGeneratedMap>,
        campaign_level: u8,
    ) -> Result<()> {
        let game_session_key =
            derive_campaign_session_pda(&ctx.accounts.player.key(), campaign_level);
        let (expected_generated_map, _) = Pubkey::find_program_address(
            &[GeneratedMap::SEED_PREFIX, game_session_key.as_ref()],
            &map_generator::ID,
        );
        require_keys_eq!(
            ctx.accounts.generated_map.key(),
            expected_generated_map,
            SessionManagerError::Unauthorized
        );
        let generated_map_seeds: &[&[u8]] = &[GeneratedMap::SEED_PREFIX, game_session_key.as_ref()];
        ctx.accounts.delegate_generated_map(
            &ctx.accounts.player,
            generated_map_seeds,
            local_delegate_config(),
        )?;
        Ok(())
    }

    /// Delegates inventory account to the MagicBlock delegation program.
    pub fn delegate_inventory(ctx: Context<DelegateInventory>, campaign_level: u8) -> Result<()> {
        let game_session_key =
            derive_campaign_session_pda(&ctx.accounts.player.key(), campaign_level);
        let (expected_inventory, _) = Pubkey::find_program_address(
            &[b"inventory", game_session_key.as_ref()],
            &player_inventory::ID,
        );
        require_keys_eq!(
            ctx.accounts.inventory.key(),
            expected_inventory,
            SessionManagerError::Unauthorized
        );
        let inventory_seeds: &[&[u8]] = &[b"inventory", game_session_key.as_ref()];
        ctx.accounts.delegate_inventory(
            &ctx.accounts.player,
            inventory_seeds,
            local_delegate_config(),
        )?;
        Ok(())
    }

    /// Delegates map-pois account to the MagicBlock delegation program.
    pub fn delegate_map_pois(ctx: Context<DelegateMapPois>, campaign_level: u8) -> Result<()> {
        let game_session_key =
            derive_campaign_session_pda(&ctx.accounts.player.key(), campaign_level);
        let (expected_map_pois, _) = Pubkey::find_program_address(
            &[b"map_pois", game_session_key.as_ref()],
            &POI_SYSTEM_PROGRAM_ID,
        );
        require_keys_eq!(
            ctx.accounts.map_pois.key(),
            expected_map_pois,
            SessionManagerError::Unauthorized
        );
        let map_pois_seeds: &[&[u8]] = &[b"map_pois", game_session_key.as_ref()];
        ctx.accounts.delegate_map_pois(
            &ctx.accounts.player,
            map_pois_seeds,
            local_delegate_config(),
        )?;
        Ok(())
    }

    /// Marks the session delegated and delegates the session account itself.
    pub fn delegate_session(ctx: Context<DelegateSession>, campaign_level: u8) -> Result<()> {
        let clock = Clock::get()?;
        let game_session_info = ctx.accounts.game_session.to_account_info();
        let game_session_key = game_session_info.key();
        require_keys_eq!(
            *game_session_info.owner,
            crate::ID,
            SessionManagerError::Unauthorized
        );

        let (session_player, session_signer, session_id) = {
            let data = game_session_info.try_borrow_data()?;
            let mut data_slice: &[u8] = &data;
            let session = GameSession::try_deserialize(&mut data_slice)?;

            require_keys_eq!(
                session.player,
                ctx.accounts.player.key(),
                SessionManagerError::Unauthorized
            );
            require_keys_eq!(
                session.session_signer,
                ctx.accounts.session_signer.key(),
                SessionManagerError::Unauthorized
            );
            require!(
                session.campaign_level == campaign_level,
                SessionManagerError::InvalidCampaignLevel
            );
            require!(
                !session.is_delegated,
                SessionManagerError::SessionAlreadyDelegated
            );

            (session.player, session.session_signer, session.session_id)
        };

        let campaign_seed = [campaign_level];
        let campaign_session_seeds: &[&[u8]] = &[
            GameSession::SEED_PREFIX,
            session_player.as_ref(),
            &campaign_seed,
        ];
        let duel_session_seeds: &[&[u8]] =
            &[GameSession::DUEL_SEED_PREFIX, session_player.as_ref()];
        let gauntlet_session_seeds: &[&[u8]] =
            &[GameSession::GAUNTLET_SEED_PREFIX, session_player.as_ref()];

        let (campaign_session_pda, _) =
            Pubkey::find_program_address(campaign_session_seeds, &crate::ID);
        let (duel_session_pda, _) = Pubkey::find_program_address(duel_session_seeds, &crate::ID);
        let (gauntlet_session_pda, _) =
            Pubkey::find_program_address(gauntlet_session_seeds, &crate::ID);

        let session_seeds: &[&[u8]] = if game_session_key == campaign_session_pda {
            campaign_session_seeds
        } else if game_session_key == duel_session_pda {
            duel_session_seeds
        } else if game_session_key == gauntlet_session_pda {
            gauntlet_session_seeds
        } else {
            return Err(SessionManagerError::Unauthorized.into());
        };

        {
            let mut data = game_session_info.try_borrow_mut_data()?;
            let mut data_slice: &[u8] = &data;
            let mut session = GameSession::try_deserialize(&mut data_slice)?;
            require_keys_eq!(
                session.session_signer,
                session_signer,
                SessionManagerError::Unauthorized
            );
            session.is_delegated = true;
            session.last_activity = clock.unix_timestamp;
            let mut data_ref: &mut [u8] = &mut data;
            session.try_serialize(&mut data_ref)?;
        }

        ctx.accounts.delegate_game_session(
            &ctx.accounts.session_signer,
            session_seeds,
            local_delegate_config(),
        )?;

        emit!(SessionDelegated {
            player: session_player,
            session_id,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Commits the current game state from the ephemeral rollup.
    /// This must be sent to the ephemeral rollup connection.
    pub fn commit_session(
        ctx: Context<CommitSession>,
        campaign_level: u8,
        state_hash: [u8; 32],
    ) -> Result<()> {
        let clock = Clock::get()?;
        let game_session_info = ctx.accounts.game_session.to_account_info();
        let mut session = load_game_session_unchecked(&game_session_info)?;
        require_keys_eq!(
            session.player,
            ctx.accounts.player.key(),
            SessionManagerError::Unauthorized
        );
        require!(
            session.campaign_level == campaign_level,
            SessionManagerError::InvalidCampaignLevel
        );
        require!(
            session.is_delegated,
            SessionManagerError::SessionNotDelegated
        );

        let game_session_key = game_session_info.key();
        validate_gameplay_runtime_accounts(
            &game_session_key,
            &ctx.accounts.game_state,
            &ctx.accounts.map_enemies,
        )?;
        validate_secondary_runtime_accounts(
            &game_session_key,
            &ctx.accounts.generated_map,
            &ctx.accounts.inventory,
            &ctx.accounts.map_pois,
        )?;

        session.state_hash = state_hash;
        session.last_activity = clock.unix_timestamp;
        store_game_session_unchecked(&game_session_info, &session)?;

        let game_state_info = ctx.accounts.game_state.to_account_info();
        let map_enemies_info = ctx.accounts.map_enemies.to_account_info();
        let generated_map_info = ctx.accounts.generated_map.to_account_info();
        let inventory_info = ctx.accounts.inventory.to_account_info();
        let map_pois_info = ctx.accounts.map_pois.to_account_info();
        commit_accounts(
            &ctx.accounts.player.to_account_info(),
            vec![
                &game_session_info,
                &game_state_info,
                &map_enemies_info,
                &generated_map_info,
                &inventory_info,
                &map_pois_info,
            ],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
        )?;

        Ok(())
    }

    /// Commits and undelegates the session account from the ephemeral rollup.
    /// This must be sent to the ephemeral rollup connection.
    pub fn undelegate_session(
        ctx: Context<UndelegateSession>,
        campaign_level: u8,
        state_hash: [u8; 32],
    ) -> Result<()> {
        let clock = Clock::get()?;
        let game_session_info = ctx.accounts.game_session.to_account_info();
        let session = load_game_session_unchecked(&game_session_info)?;
        require_keys_eq!(
            session.player,
            ctx.accounts.player.key(),
            SessionManagerError::Unauthorized
        );
        require_keys_eq!(
            session.session_signer,
            ctx.accounts.session_signer.key(),
            SessionManagerError::Unauthorized
        );
        require!(
            session.campaign_level == campaign_level,
            SessionManagerError::InvalidCampaignLevel
        );

        commit_and_undelegate_accounts(
            &ctx.accounts.session_signer.to_account_info(),
            vec![&game_session_info],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
        )?;

        // NOTE: After scheduling commit+undelegate, this program may no longer own `game_session`
        // during this instruction, so we must not mutate account data here.
        let _ = state_hash;
        let _ = clock;

        Ok(())
    }

    /// Ends the session after death or level completion.
    /// Only callable by session key signer when player is dead OR has completed the level.
    /// Also closes the player's inventory via CPI to ensure fresh inventory for next session.
    ///
    /// This is designed to be called automatically by the frontend after combat,
    /// signed only by the session key signer (no user interaction required).
    pub fn end_session(ctx: Context<EndSession>, _campaign_level: u8) -> Result<()> {
        let clock = Clock::get()?;
        let authority_bump = ctx.bumps.session_manager_authority;
        let authority_signer_seeds: &[&[u8]] = &[SESSION_MANAGER_AUTHORITY_SEED, &[authority_bump]];
        let signer_seeds: &[&[&[u8]]] = &[authority_signer_seeds];
        let game_session_key = ctx.accounts.game_session.key();

        validate_gameplay_runtime_accounts(
            &game_session_key,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.map_enemies.to_account_info(),
        )?;
        validate_secondary_runtime_accounts(
            &game_session_key,
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.inventory.to_account_info(),
            &ctx.accounts.map_pois.to_account_info(),
        )?;

        let game_state = read_game_state_unchecked(&ctx.accounts.game_state.to_account_info())?;

        // Do not trust `session.is_delegated` bit here; legacy undelegate flows can leave
        // the flag stale even after ownership returns to session-manager.

        // Determine victory from game state (completed and not dead means victory).
        // Any other state closes as defeat so cleanup can recover stuck sessions.
        let victory = game_state.completed && !game_state.is_dead;

        if !ctx.accounts.game_session.settled {
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                victory,
                signer_seeds,
            )?;
            let session = &mut ctx.accounts.game_session;
            session.settled = true;
            session.settled_victory = victory;
            session.settled_at = clock.unix_timestamp;
            emit!(SessionResultSettled {
                player: session.player,
                session_id: session.session_id,
                campaign_level: session.campaign_level,
                victory,
                timestamp: clock.unix_timestamp,
            });
        }

        let session = &ctx.accounts.game_session;

        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            victory,
            final_state_hash: session.state_hash,
            timestamp: clock.unix_timestamp,
        });

        // Close all session-related accounts via CPI
        // Order matters: close child accounts before parent accounts

        // 1. Close map_pois (depends on session)
        close_map_pois_via_session_signer_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 2. Close generated_map (depends on session)
        close_generated_map_cpi(
            &ctx.accounts.map_generator_program,
            &ctx.accounts.generated_map,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 3. Close map_enemies (depends on game_state)
        close_map_enemies_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.map_enemies,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 4. Close game_state
        close_game_state_via_session_signer_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 5. Close inventory via CPI to ensure fresh inventory for next session
        // Use session_signer since it's the inventory owner (set during start_session)
        player_inventory::cpi::close_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::CloseInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
            },
        ))?;

        // 6. Session account will be closed by Anchor (close = player constraint)
        Ok(())
    }

    /// Settles run outcome into player-profile without requiring account closure.
    /// This is idempotent and can be retried independently when close fails.
    pub fn settle_session_result(ctx: Context<SettleSessionResult>, _campaign_level: u8) -> Result<()> {
        let clock = Clock::get()?;

        let (expected_game_state, _) =
            Pubkey::find_program_address(&[b"game_state", ctx.accounts.game_session.key().as_ref()], &gameplay_state::ID);
        require_keys_eq!(
            ctx.accounts.game_state.key(),
            expected_game_state,
            SessionManagerError::Unauthorized
        );

        let game_state = read_game_state_unchecked(&ctx.accounts.game_state)?;
        require!(
            game_state.is_dead || game_state.completed,
            SessionManagerError::RunNotTerminal
        );
        let victory = game_state.completed && !game_state.is_dead;

        let authority_bump = ctx.bumps.session_manager_authority;
        let authority_signer_seeds: &[&[u8]] = &[SESSION_MANAGER_AUTHORITY_SEED, &[authority_bump]];
        let signer_seeds: &[&[&[u8]]] = &[authority_signer_seeds];

        if !ctx.accounts.game_session.settled {
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                victory,
                signer_seeds,
            )?;
            let session = &mut ctx.accounts.game_session;
            session.settled = true;
            session.settled_victory = victory;
            session.settled_at = clock.unix_timestamp;
            emit!(SessionResultSettled {
                player: session.player,
                session_id: session.session_id,
                campaign_level: session.campaign_level,
                victory,
                timestamp: clock.unix_timestamp,
            });
        }

        Ok(())
    }

    /// Emergency fallback: settle run result (if needed) and close only the game_session account.
    /// This allows users to recover from ER child-account undelegation failures and start new runs.
    /// Child runtime accounts may remain delegated/stuck and must be cleaned up separately.
    pub fn close_session_only(ctx: Context<CloseSessionOnly>) -> Result<()> {
        let clock = Clock::get()?;

        let (expected_game_state, _) =
            Pubkey::find_program_address(&[b"game_state", ctx.accounts.game_session.key().as_ref()], &gameplay_state::ID);
        require_keys_eq!(
            ctx.accounts.game_state.key(),
            expected_game_state,
            SessionManagerError::Unauthorized
        );

        // Best-effort terminal detection from base-layer game_state.
        // If ER commit lag leaves base game_state non-terminal/stale, force-close
        // as defeat so the player is never permanently blocked from starting a new run.
        let victory = match read_game_state_unchecked(&ctx.accounts.game_state) {
            Ok(game_state) if game_state.is_dead || game_state.completed => {
                game_state.completed && !game_state.is_dead
            }
            _ => false,
        };

        let authority_bump = ctx.bumps.session_manager_authority;
        let authority_signer_seeds: &[&[u8]] = &[SESSION_MANAGER_AUTHORITY_SEED, &[authority_bump]];
        let signer_seeds: &[&[&[u8]]] = &[authority_signer_seeds];

        if !ctx.accounts.game_session.settled {
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                victory,
                signer_seeds,
            )?;
            let session = &mut ctx.accounts.game_session;
            session.settled = true;
            session.settled_victory = victory;
            session.settled_at = clock.unix_timestamp;
            emit!(SessionResultSettled {
                player: session.player,
                session_id: session.session_id,
                campaign_level: session.campaign_level,
                victory,
                timestamp: clock.unix_timestamp,
            });
        }

        let session = &ctx.accounts.game_session;
        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            victory,
            final_state_hash: session.state_hash,
            timestamp: clock.unix_timestamp,
        });

        // game_session closes via `close = player` account constraint.
        Ok(())
    }

    /// Tolerant session close: settles result (defeat if unreadable) and closes whichever
    /// child accounts are on base layer. Delegated/missing children are skipped.
    /// This prevents the permanent soft-lock where close_session_only leaves orphaned
    /// child accounts that block start_session (which uses `init`, not `init_if_needed`).
    pub fn force_close_session(ctx: Context<ForceCloseSession>) -> Result<()> {
        let clock = Clock::get()?;
        let game_session_key = ctx.accounts.game_session.key();

        // PDA validate game_state
        let (expected_game_state, _) = Pubkey::find_program_address(
            &[b"game_state", game_session_key.as_ref()],
            &gameplay_state::ID,
        );
        require_keys_eq!(
            ctx.accounts.game_state.key(),
            expected_game_state,
            SessionManagerError::Unauthorized
        );

        // Best-effort terminal detection from base-layer game_state.
        // If ER commit lag leaves base game_state non-terminal/stale, force-close
        // as defeat so the player is never permanently blocked from starting a new run.
        let victory = match read_game_state_unchecked(&ctx.accounts.game_state) {
            Ok(game_state) if game_state.is_dead || game_state.completed => {
                game_state.completed && !game_state.is_dead
            }
            _ => false,
        };

        let authority_bump = ctx.bumps.session_manager_authority;
        let authority_signer_seeds: &[&[u8]] = &[SESSION_MANAGER_AUTHORITY_SEED, &[authority_bump]];
        let signer_seeds: &[&[&[u8]]] = &[authority_signer_seeds];

        if !ctx.accounts.game_session.settled {
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                victory,
                signer_seeds,
            )?;
            let session = &mut ctx.accounts.game_session;
            session.settled = true;
            session.settled_victory = victory;
            session.settled_at = clock.unix_timestamp;
            emit!(SessionResultSettled {
                player: session.player,
                session_id: session.session_id,
                campaign_level: session.campaign_level,
                victory,
                timestamp: clock.unix_timestamp,
            });
        }

        let session = &ctx.accounts.game_session;
        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            victory,
            final_state_hash: session.state_hash,
            timestamp: clock.unix_timestamp,
        });

        // Close child accounts that are on base layer (owned by their respective programs).
        // Skip any that are still delegated (owned by delegation program) or missing.
        // Order: map_pois, generated_map, map_enemies (needs game_state), game_state, inventory.
        let game_state_closeable = *ctx.accounts.game_state.owner == gameplay_state::ID;
        let map_enemies_closeable =
            *ctx.accounts.map_enemies.owner == gameplay_state::ID && game_state_closeable;

        if *ctx.accounts.map_pois.owner == POI_SYSTEM_PROGRAM_ID {
            close_map_pois_via_session_signer_cpi(
                &ctx.accounts.poi_system_program,
                &ctx.accounts.map_pois,
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        if *ctx.accounts.generated_map.owner == MAP_GENERATOR_PROGRAM_ID {
            close_generated_map_cpi(
                &ctx.accounts.map_generator_program,
                &ctx.accounts.generated_map,
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        if map_enemies_closeable {
            close_map_enemies_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &ctx.accounts.map_enemies,
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        if game_state_closeable {
            close_game_state_via_session_signer_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        if *ctx.accounts.inventory.owner == player_inventory::ID {
            player_inventory::cpi::close_inventory(CpiContext::new(
                ctx.accounts.player_inventory_program.to_account_info(),
                player_inventory::cpi::accounts::CloseInventory {
                    inventory: ctx.accounts.inventory.to_account_info(),
                    player: ctx.accounts.session_signer.to_account_info(),
                },
            ))?;
        }

        // game_session closes via `close = player` account constraint.
        Ok(())
    }

    /// Close orphaned child accounts after force_close_session already freed the session PDA.
    /// Session PDA no longer exists, so we validate via game_state (which stores session_signer
    /// and player). Only closes accounts that are on base layer (owned by their programs).
    /// Call order: map_pois → map_enemies → game_state (game_state last since others depend on it).
    pub fn close_orphaned_accounts(ctx: Context<CloseOrphanedAccounts>) -> Result<()> {
        // game_state is the auth source — it stores session_signer (validated via has_one)
        // and player (validated via address constraint on player account).

        // Close map_pois if on base layer (owned by poi-system)
        if *ctx.accounts.map_pois.owner == POI_SYSTEM_PROGRAM_ID {
            close_map_pois_orphaned_cpi(
                &ctx.accounts.poi_system_program,
                &ctx.accounts.map_pois,
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // Close map_enemies if on base layer (owned by gameplay-state)
        if *ctx.accounts.map_enemies.owner == gameplay_state::ID {
            close_map_enemies_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &ctx.accounts.map_enemies,
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // Close game_state last (others depend on it for auth)
        if *ctx.accounts.game_state.owner == gameplay_state::ID {
            close_game_state_via_session_signer_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        emit!(OrphanedAccountsClosed {
            player: ctx.accounts.session_signer.key(),
        });

        Ok(())
    }

    /// Abandons a session at any time (user-initiated).
    /// Requires the main wallet signature.
    /// Used when player wants to quit a session early.
    /// Closes all session-related accounts to allow starting a new session on the same level.
    pub fn abandon_session(ctx: Context<AbandonSession>, _campaign_level: u8) -> Result<()> {
        let clock = Clock::get()?;

        // Do not trust `session.is_delegated` bit here; legacy undelegate flows can leave
        // the flag stale even after ownership returns to session-manager.

        // Abandon settles as defeat unless already settled.
        let authority_bump = ctx.bumps.session_manager_authority;
        let authority_signer_seeds: &[&[u8]] = &[SESSION_MANAGER_AUTHORITY_SEED, &[authority_bump]];
        let signer_seeds: &[&[&[u8]]] = &[authority_signer_seeds];
        if !ctx.accounts.game_session.settled {
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                false,
                signer_seeds,
            )?;
            let session = &mut ctx.accounts.game_session;
            session.settled = true;
            session.settled_victory = false;
            session.settled_at = clock.unix_timestamp;
            emit!(SessionResultSettled {
                player: session.player,
                session_id: session.session_id,
                campaign_level: session.campaign_level,
                victory: false,
                timestamp: clock.unix_timestamp,
            });
        }

        let session = &ctx.accounts.game_session;

        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            victory: false, // Abandoning counts as a loss
            final_state_hash: session.state_hash,
            timestamp: clock.unix_timestamp,
        });

        // Close all session-related accounts via CPI
        // Order matters: close child accounts before parent accounts

        // 1. Close map_pois (depends on session)
        close_map_pois_via_session_signer_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 2. Close generated_map (depends on session)
        close_generated_map_cpi(
            &ctx.accounts.map_generator_program,
            &ctx.accounts.generated_map,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 3. Close map_enemies (depends on game_state)
        close_map_enemies_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.map_enemies,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 4. Close game_state
        close_game_state_via_session_signer_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.session_signer.to_account_info(),
        )?;

        // 5. Close inventory via CPI
        player_inventory::cpi::close_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::CloseInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
            },
        ))?;

        // 6. Session account will be closed by Anchor (close = player constraint)
        Ok(())
    }
}

// ============================================================================
// Account Contexts
// ============================================================================

#[derive(Accounts)]
pub struct InitializeCounter<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + SessionCounter::INIT_SPACE,
        seeds = [SessionCounter::SEED_PREFIX],
        bump
    )]
    pub session_counter: Account<'info, SessionCounter>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// PlayerProfile account for reading player state during session creation.
/// We need to reference it to validate runs and level access.
#[derive(Clone)]
pub struct PlayerProfileRef;

impl anchor_lang::Id for PlayerProfileRef {
    fn id() -> Pubkey {
        PLAYER_PROFILE_PROGRAM_ID
    }
}

/// PlayerProfile account - mirrors the structure from player-profile program.
/// IMPORTANT: The account name MUST be "PlayerProfile" (not PlayerProfileAccount)
/// to generate the correct Anchor discriminator (sha256("account:PlayerProfile")[..8]).
/// We use AnchorDeserialize manually since we can't use #[account] with a custom owner.
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct PlayerProfile {
    pub owner: Pubkey,
    pub name: String,
    pub total_runs: u32,
    pub highest_level_unlocked: u8,
    pub available_runs: u32,
    pub created_at: i64,
    pub bump: u8,
    pub unlocked_items: [u8; 10],
    pub active_item_pool: [u8; 10],
}

impl PlayerProfile {
    /// Anchor discriminator for "PlayerProfile" account
    /// sha256("account:PlayerProfile")[..8]
    pub const DISCRIMINATOR: [u8; 8] = [82, 226, 99, 87, 164, 130, 181, 80];
}

impl anchor_lang::AccountDeserialize for PlayerProfile {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        // Skip discriminator
        if buf.len() < 8 {
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorNotFound.into());
        }
        let discriminator = &buf[..8];
        if discriminator != Self::DISCRIMINATOR {
            return Err(anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch.into());
        }
        *buf = &buf[8..];
        Self::deserialize(buf)
            .map_err(|_| anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into())
    }
}

impl anchor_lang::AccountSerialize for PlayerProfile {}

impl anchor_lang::Owner for PlayerProfile {
    fn owner() -> Pubkey {
        PLAYER_PROFILE_PROGRAM_ID
    }
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct StartSession<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref(), &[campaign_level]],
        bump
    )]
    pub game_session: Account<'info, GameSession>,

    #[account(
        mut,
        seeds = [SessionCounter::SEED_PREFIX],
        bump = session_counter.bump
    )]
    pub session_counter: Account<'info, SessionCounter>,

    /// Player profile for validation and run consumption (from player-profile program)
    #[account(
        mut,
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub player: Signer<'info>,

    /// Session key signer that will own all session-specific accounts (inventory, etc.)
    /// Must be a signer so it can be set as the inventory owner for gameplay transactions.
    #[account(mut)]
    pub session_signer: Signer<'info>,

    /// Map configuration for map generation
    pub map_config: Account<'info, MapConfigAccount>,

    #[account(mut)]
    /// CHECK: PDA created by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub map_enemies: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by poi-system CPI (PDA derived from session)
    pub map_pois: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    pub map_generator_program: Program<'info, MapGenerator>,
    pub gameplay_state_program: Program<'info, GameplayState>,
    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for manual CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,
    pub player_inventory_program: Program<'info, PlayerInventory>,
    #[account(address = PLAYER_PROFILE_PROGRAM_ID)]
    /// CHECK: Player profile program for manual CPI, validated by address constraint
    pub player_profile_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartDuelSession<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GameSession::DUEL_SEED_PREFIX, player.key().as_ref()],
        bump
    )]
    pub game_session: Account<'info, GameSession>,

    #[account(
        mut,
        seeds = [SessionCounter::SEED_PREFIX],
        bump = session_counter.bump
    )]
    pub session_counter: Account<'info, SessionCounter>,

    #[account(
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub player: Signer<'info>,

    #[account(mut)]
    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA signer used to authorize configure_run_mode CPI.
    pub session_manager_authority: UncheckedAccount<'info>,

    pub map_config: Account<'info, MapConfigAccount>,

    #[account(mut)]
    /// CHECK: PDA created by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub map_enemies: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by poi-system CPI
    pub map_pois: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    pub map_generator_program: Program<'info, MapGenerator>,
    pub gameplay_state_program: Program<'info, GameplayState>,
    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for manual CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,
    pub player_inventory_program: Program<'info, PlayerInventory>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartGauntletSession<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GameSession::GAUNTLET_SEED_PREFIX, player.key().as_ref()],
        bump
    )]
    pub game_session: Account<'info, GameSession>,

    #[account(
        mut,
        seeds = [SessionCounter::SEED_PREFIX],
        bump = session_counter.bump
    )]
    pub session_counter: Account<'info, SessionCounter>,

    #[account(
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub player: Signer<'info>,

    #[account(mut)]
    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA signer used to authorize configure_run_mode CPI.
    pub session_manager_authority: UncheckedAccount<'info>,

    pub map_config: Account<'info, MapConfigAccount>,

    #[account(mut)]
    /// CHECK: PDA created by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub map_enemies: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by poi-system CPI
    pub map_pois: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    pub map_generator_program: Program<'info, MapGenerator>,
    pub gameplay_state_program: Program<'info, GameplayState>,
    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for manual CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,
    pub player_inventory_program: Program<'info, PlayerInventory>,

    pub system_program: Program<'info, System>,
}

#[delegate]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateSession<'info> {
    #[account(mut, del)]
    /// CHECK: Validated in handler (owner/player/level/delegation status and PDA seeds).
    pub game_session: AccountInfo<'info>,

    /// CHECK: Must match game_session.player, but does not need to sign.
    pub player: AccountInfo<'info>,
    pub session_signer: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateGameState<'info> {
    #[account(mut, del)]
    /// CHECK: Validated in handler as gameplay-state PDA for the delegated session.
    pub game_state: AccountInfo<'info>,

    pub player: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateMapEnemies<'info> {
    #[account(mut, del)]
    /// CHECK: Validated in handler as map-enemies PDA for the delegated session.
    pub map_enemies: AccountInfo<'info>,

    pub player: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateGeneratedMap<'info> {
    #[account(mut, del)]
    /// CHECK: Validated in handler as generated-map PDA for the delegated session.
    pub generated_map: AccountInfo<'info>,

    pub player: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateInventory<'info> {
    #[account(mut, del)]
    /// CHECK: Validated in handler as inventory PDA for the delegated session.
    pub inventory: AccountInfo<'info>,

    pub player: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateMapPois<'info> {
    #[account(mut, del)]
    /// CHECK: Validated in handler as map-pois PDA for the delegated session.
    pub map_pois: AccountInfo<'info>,

    pub player: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct CommitSession<'info> {
    #[account(mut)]
    /// CHECK: Deserialized and validated in handler to support delegated owner (DELeGG).
    pub game_session: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated in handler as gameplay-state PDA for the delegated session.
    pub game_state: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated in handler as map-enemies PDA for the delegated session.
    pub map_enemies: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated in handler as generated-map PDA for the delegated session.
    pub generated_map: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated in handler as inventory PDA for the delegated session.
    pub inventory: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated in handler as map-pois PDA for the delegated session.
    pub map_pois: UncheckedAccount<'info>,

    pub player: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct UndelegateSession<'info> {
    #[account(mut)]
    /// CHECK: Deserialized and validated in handler to support delegated owner (DELeGG).
    pub game_session: UncheckedAccount<'info>,

    /// CHECK: Must match game_session.player, but does not need to sign.
    pub player: AccountInfo<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

fn validate_gameplay_runtime_accounts(
    game_session_key: &Pubkey,
    game_state: &AccountInfo<'_>,
    map_enemies: &AccountInfo<'_>,
) -> Result<()> {
    let (expected_game_state, _) = Pubkey::find_program_address(
        &[b"game_state", game_session_key.as_ref()],
        &gameplay_state::ID,
    );
    require_keys_eq!(
        game_state.key(),
        expected_game_state,
        SessionManagerError::Unauthorized
    );

    let (expected_map_enemies, _) = Pubkey::find_program_address(
        &[MapEnemies::SEED_PREFIX, game_session_key.as_ref()],
        &gameplay_state::ID,
    );
    require_keys_eq!(
        map_enemies.key(),
        expected_map_enemies,
        SessionManagerError::Unauthorized
    );

    Ok(())
}

fn validate_secondary_runtime_accounts(
    game_session_key: &Pubkey,
    generated_map: &AccountInfo<'_>,
    inventory: &AccountInfo<'_>,
    map_pois: &AccountInfo<'_>,
) -> Result<()> {
    let (expected_generated_map, _) = Pubkey::find_program_address(
        &[GeneratedMap::SEED_PREFIX, game_session_key.as_ref()],
        &map_generator::ID,
    );
    require_keys_eq!(
        generated_map.key(),
        expected_generated_map,
        SessionManagerError::Unauthorized
    );

    let (expected_inventory, _) = Pubkey::find_program_address(
        &[b"inventory", game_session_key.as_ref()],
        &player_inventory::ID,
    );
    require_keys_eq!(
        inventory.key(),
        expected_inventory,
        SessionManagerError::Unauthorized
    );

    let (expected_map_pois, _) = Pubkey::find_program_address(
        &[b"map_pois", game_session_key.as_ref()],
        &POI_SYSTEM_PROGRAM_ID,
    );
    require_keys_eq!(
        map_pois.key(),
        expected_map_pois,
        SessionManagerError::Unauthorized
    );

    Ok(())
}

fn derive_campaign_session_pda(player: &Pubkey, campaign_level: u8) -> Pubkey {
    let campaign_seed = [campaign_level];
    let seeds: &[&[u8]] = &[GameSession::SEED_PREFIX, player.as_ref(), &campaign_seed];
    Pubkey::find_program_address(seeds, &crate::ID).0
}

fn load_game_session_unchecked(game_session_info: &AccountInfo<'_>) -> Result<GameSession> {
    let data = game_session_info.try_borrow_data()?;
    let mut data_slice: &[u8] = &data;
    GameSession::try_deserialize(&mut data_slice)
}

fn store_game_session_unchecked(
    game_session_info: &AccountInfo<'_>,
    session: &GameSession,
) -> Result<()> {
    let mut data = game_session_info.try_borrow_mut_data()?;
    let mut data_ref: &mut [u8] = &mut data;
    session.try_serialize(&mut data_ref)?;
    Ok(())
}

fn read_game_state_unchecked(game_state_info: &AccountInfo<'_>) -> Result<GameState> {
    let data = game_state_info.try_borrow_data()?;
    let mut data_slice: &[u8] = &data;
    GameState::try_deserialize(&mut data_slice)
}

/// End session after death or level completion.
/// Only session key signer needs to sign - player just receives rent refund.
/// Closes all session-related accounts: session, game_state, generated_map, map_enemies, map_pois, inventory.
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct EndSession<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
        close = player
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account to validate death/completion status (closed via gameplay-state CPI)
    #[account(mut)]
    /// CHECK: Validated by PDA derivation in handler and deserialized via read_game_state_unchecked.
    pub game_state: UncheckedAccount<'info>,

    /// Map enemies account (closed via gameplay-state CPI)
    #[account(mut)]
    /// CHECK: Validated by gameplay-state CPI
    pub map_enemies: UncheckedAccount<'info>,

    /// Generated map account (closed via map-generator CPI)
    #[account(mut)]
    /// CHECK: Validated by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    /// Map POIs account (closed via poi-system CPI)
    #[account(mut)]
    /// CHECK: Validated by poi-system CPI
    pub map_pois: UncheckedAccount<'info>,

    /// Player profile for recording run result
    #[account(
        mut,
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    /// Player wallet - receives rent refund but does NOT need to sign.
    /// CHECK: Validated by has_one constraint on game_session.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    /// Session key signer - must sign to authorize session end and close inventory
    #[account(mut)]
    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA signer used to authorize player-profile run-result CPI.
    pub session_manager_authority: UncheckedAccount<'info>,

    /// Player's inventory account (closed via CPI to ensure fresh inventory next session)
    #[account(mut)]
    /// CHECK: Validated by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    pub player_inventory_program: Program<'info, PlayerInventory>,
    pub gameplay_state_program: Program<'info, GameplayState>,

    #[account(address = PLAYER_PROFILE_PROGRAM_ID)]
    /// CHECK: Player profile program for manual CPI, validated by address constraint
    pub player_profile_program: UncheckedAccount<'info>,

    #[account(address = MAP_GENERATOR_PROGRAM_ID)]
    /// CHECK: Map generator program for CPI, validated by address constraint
    pub map_generator_program: UncheckedAccount<'info>,

    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,
}

/// Settles run result into player-profile without closing any accounts.
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct SettleSessionResult<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account can still be delegated; validated in handler.
    #[account(mut)]
    /// CHECK: Validated by PDA derivation and deserialized in handler.
    pub game_state: UncheckedAccount<'info>,

    /// Player profile for recording run result
    #[account(
        mut,
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    /// Player wallet - validated by has_one constraint.
    /// CHECK: Has-one relation on game_session ensures this is the session owner.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    /// Session key signer - authorizes settlement without wallet popup
    #[account(mut)]
    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA signer used to authorize player-profile run-result CPI.
    pub session_manager_authority: UncheckedAccount<'info>,

    #[account(address = PLAYER_PROFILE_PROGRAM_ID)]
    /// CHECK: Player profile program for manual CPI, validated by address constraint
    pub player_profile_program: UncheckedAccount<'info>,
}

/// Emergency fallback: close only the game_session account after terminal state settlement.
#[derive(Accounts)]
pub struct CloseSessionOnly<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
        close = player
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account can still be delegated; validated in handler.
    #[account(mut)]
    /// CHECK: Validated by PDA derivation and deserialized in handler.
    pub game_state: UncheckedAccount<'info>,

    /// Player profile for recording run result (if not settled yet)
    #[account(
        mut,
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    /// CHECK: Validated by has_one constraint on game_session.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA signer used to authorize player-profile run-result CPI.
    pub session_manager_authority: UncheckedAccount<'info>,

    #[account(address = PLAYER_PROFILE_PROGRAM_ID)]
    /// CHECK: Player profile program for manual CPI, validated by address constraint
    pub player_profile_program: UncheckedAccount<'info>,
}

/// Tolerant session close: settles result and closes whichever child accounts are on base layer.
/// Delegated/missing children are skipped. This prevents the soft-lock where close_session_only
/// leaves orphaned child accounts that block start_session.
#[derive(Accounts)]
pub struct ForceCloseSession<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
        close = player
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account — may be delegated; validated by PDA derivation in handler.
    #[account(mut)]
    /// CHECK: Validated by PDA derivation and deserialized in handler.
    pub game_state: UncheckedAccount<'info>,

    /// Map enemies account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub map_enemies: UncheckedAccount<'info>,

    /// Generated map account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub generated_map: UncheckedAccount<'info>,

    /// Map POIs account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub map_pois: UncheckedAccount<'info>,

    /// Player inventory account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub inventory: UncheckedAccount<'info>,

    /// Player profile for recording run result (if not settled yet)
    #[account(
        mut,
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    /// CHECK: Validated by has_one constraint on game_session.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA signer used to authorize player-profile run-result CPI.
    pub session_manager_authority: UncheckedAccount<'info>,

    pub player_inventory_program: Program<'info, PlayerInventory>,
    pub gameplay_state_program: Program<'info, GameplayState>,

    #[account(address = PLAYER_PROFILE_PROGRAM_ID)]
    /// CHECK: Player profile program for manual CPI, validated by address constraint
    pub player_profile_program: UncheckedAccount<'info>,

    #[account(address = MAP_GENERATOR_PROGRAM_ID)]
    /// CHECK: Map generator program for CPI, validated by address constraint
    pub map_generator_program: UncheckedAccount<'info>,

    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,
}

/// Close orphaned child accounts after force_close_session freed the session PDA.
/// Session PDA no longer exists. Validates via game_state (stores session_signer + player).
#[derive(Accounts)]
pub struct CloseOrphanedAccounts<'info> {
    /// GameState is the auth source — stores session_signer and player.
    /// May be delegated (skipped if so).
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI. Validated by child program CPIs.
    pub game_state: UncheckedAccount<'info>,

    /// Map enemies account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub map_enemies: UncheckedAccount<'info>,

    /// Map POIs account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub map_pois: UncheckedAccount<'info>,

    /// Player wallet receives rent refunds.
    /// CHECK: Validated by child program CPIs via game_state.player.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    /// Session key signer — validated by child program CPIs via game_state.session_signer.
    pub session_signer: Signer<'info>,

    pub gameplay_state_program: Program<'info, GameplayState>,

    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,
}

/// Abandon session at any time (user-initiated).
/// Requires both main wallet and session key signer signatures.
/// Main wallet authorizes the abandonment, session key signer is needed to close sub-accounts.
/// Closes all session-related accounts: session, game_state, generated_map, map_enemies, map_pois, inventory.
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct AbandonSession<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
        close = player
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account (closed via gameplay-state CPI)
    #[account(mut)]
    /// CHECK: Validated by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

    /// Map enemies account (closed via gameplay-state CPI)
    #[account(mut)]
    /// CHECK: Validated by gameplay-state CPI
    pub map_enemies: UncheckedAccount<'info>,

    /// Generated map account (closed via map-generator CPI)
    #[account(mut)]
    /// CHECK: Validated by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    /// Map POIs account (closed via poi-system CPI)
    #[account(mut)]
    /// CHECK: Validated by poi-system CPI
    pub map_pois: UncheckedAccount<'info>,

    /// Player wallet - must sign to authorize abandonment
    #[account(mut)]
    pub player: Signer<'info>,

    /// Session key signer - must sign to close sub-accounts (owns the inventory)
    #[account(mut)]
    pub session_signer: Signer<'info>,

    /// Player profile for recording run result (defeat) if not settled yet
    #[account(
        mut,
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump
    )]
    /// CHECK: PDA signer used to authorize player-profile run-result CPI.
    pub session_manager_authority: UncheckedAccount<'info>,

    /// Player's inventory account (closed via CPI)
    #[account(mut)]
    /// CHECK: Validated by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    pub player_inventory_program: Program<'info, PlayerInventory>,
    pub gameplay_state_program: Program<'info, GameplayState>,

    #[account(address = PLAYER_PROFILE_PROGRAM_ID)]
    /// CHECK: Player profile program for manual CPI, validated by address constraint
    pub player_profile_program: UncheckedAccount<'info>,

    #[account(address = MAP_GENERATOR_PROGRAM_ID)]
    /// CHECK: Map generator program for CPI, validated by address constraint
    pub map_generator_program: UncheckedAccount<'info>,

    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct SessionStarted {
    pub player: Pubkey,
    pub session_id: u64,
    pub campaign_level: u8,
    pub session_signer: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct SessionDelegated {
    pub player: Pubkey,
    pub session_id: u64,
    pub timestamp: i64,
}

#[event]
pub struct SessionEnded {
    pub player: Pubkey,
    pub session_id: u64,
    pub campaign_level: u8,
    pub victory: bool,
    pub final_state_hash: [u8; 32],
    pub timestamp: i64,
}

#[event]
pub struct SessionResultSettled {
    pub player: Pubkey,
    pub session_id: u64,
    pub campaign_level: u8,
    pub victory: bool,
    pub timestamp: i64,
}

#[event]
pub struct OrphanedAccountsClosed {
    pub player: Pubkey,
}

/// The discriminator for end_session instruction.
/// This is exported so other programs can validate their manual CPI discriminators.
/// Computed as sha256("global:end_session")[..8].
///
/// IMPORTANT: If you rename the `end_session` instruction, you must:
/// 1. Update this constant
/// 2. Update gameplay-state's END_SESSION_DISCRIMINATOR constant
pub const END_SESSION_DISCRIMINATOR: [u8; 8] = [0x0b, 0xf4, 0x3d, 0x9a, 0xd4, 0xf9, 0x0f, 0x42];

fn derive_pvp_seed(
    domain: &[u8],
    clock: &Clock,
    session_counter: u64,
    player: &Pubkey,
    session_signer: &Pubkey,
) -> u64 {
    let mut acc = session_counter
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(clock.slot)
        .wrapping_add(clock.unix_timestamp as u64);

    for byte in domain {
        acc = acc.wrapping_mul(0x100_0000_01B3).wrapping_add(*byte as u64);
    }
    for byte in player.as_ref() {
        acc = acc.wrapping_mul(0x100_0000_01B3).wrapping_add(*byte as u64);
    }
    for byte in session_signer.as_ref() {
        acc = acc.wrapping_mul(0x100_0000_01B3).wrapping_add(*byte as u64);
    }

    acc
}

// ============================================================================
// Manual CPI Helper
// ============================================================================

/// Generic manual CPI invocation. Each account tuple is `(info, is_writable, is_signer)`.
fn invoke_manual_cpi<'info>(
    program: &AccountInfo<'info>,
    program_id: Pubkey,
    discriminator: &[u8; 8],
    extra_data: &[u8],
    accounts: &[(&AccountInfo<'info>, bool, bool)],
) -> Result<()> {
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
    use anchor_lang::solana_program::program::invoke;

    let mut data = Vec::with_capacity(8 + extra_data.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(extra_data);

    let metas: Vec<AccountMeta> = accounts
        .iter()
        .map(|(info, writable, signer)| {
            if *writable {
                AccountMeta::new(info.key(), *signer)
            } else {
                AccountMeta::new_readonly(info.key(), *signer)
            }
        })
        .collect();

    let mut invoke_infos: Vec<AccountInfo<'info>> = accounts
        .iter()
        .map(|(info, _, _)| (*info).clone())
        .collect();
    invoke_infos.push(program.clone());

    invoke(
        &Instruction {
            program_id,
            accounts: metas,
            data,
        },
        &invoke_infos,
    )?;
    Ok(())
}

/// Generic manual CPI invocation with PDA signer seeds.
fn invoke_manual_cpi_signed<'info>(
    program: &AccountInfo<'info>,
    program_id: Pubkey,
    discriminator: &[u8; 8],
    extra_data: &[u8],
    accounts: &[(&AccountInfo<'info>, bool, bool)],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
    use anchor_lang::solana_program::program::invoke_signed;

    let mut data = Vec::with_capacity(8 + extra_data.len());
    data.extend_from_slice(discriminator);
    data.extend_from_slice(extra_data);

    let metas: Vec<AccountMeta> = accounts
        .iter()
        .map(|(info, writable, signer)| {
            if *writable {
                AccountMeta::new(info.key(), *signer)
            } else {
                AccountMeta::new_readonly(info.key(), *signer)
            }
        })
        .collect();

    let mut invoke_infos: Vec<AccountInfo<'info>> = accounts
        .iter()
        .map(|(info, _, _)| (*info).clone())
        .collect();
    invoke_infos.push(program.clone());

    invoke_signed(
        &Instruction {
            program_id,
            accounts: metas,
            data,
        },
        &invoke_infos,
        signer_seeds,
    )?;
    Ok(())
}

// ============================================================================
// CPI Functions
// ============================================================================

fn consume_run_cpi<'info>(
    program: &AccountInfo<'info>,
    player_profile: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        PLAYER_PROFILE_PROGRAM_ID,
        &CONSUME_RUN_DISCRIMINATOR,
        &[],
        &[(player_profile, true, false), (owner, false, true)],
    )
}

#[allow(clippy::too_many_arguments)]
fn initialize_map_pois_cpi<'info>(
    program: &AccountInfo<'info>,
    map_pois: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    generated_map: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    act: u8,
    week: u8,
    seed: u64,
) -> Result<()> {
    let mut extra = Vec::with_capacity(10);
    extra.push(act);
    extra.push(week);
    extra.extend_from_slice(&seed.to_le_bytes());
    invoke_manual_cpi(
        program,
        POI_SYSTEM_PROGRAM_ID,
        &INITIALIZE_MAP_POIS_DISCRIMINATOR,
        &extra,
        &[
            (map_pois, true, false),
            (session, false, false),
            (generated_map, false, false),
            (game_state, false, false),
            (payer, true, true),
            (system_program, false, false),
        ],
    )
}

fn discover_visible_waypoints_cpi<'info>(
    program: &AccountInfo<'info>,
    map_pois: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
    visibility_radius: u8,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        POI_SYSTEM_PROGRAM_ID,
        &DISCOVER_VISIBLE_WAYPOINTS_DISCRIMINATOR,
        &[visibility_radius],
        &[
            (map_pois, true, false),
            (game_state, false, false),
            (session_signer, false, true),
        ],
    )
}

pub const RECORD_RUN_RESULT_CPI_DISCRIMINATOR: [u8; 8] =
    [0x09, 0xaf, 0xf6, 0x09, 0x1f, 0x62, 0x79, 0x45];

#[allow(clippy::too_many_arguments)]
fn record_run_result_cpi<'info>(
    program: &AccountInfo<'info>,
    player_profile: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
    session_manager_authority: &AccountInfo<'info>,
    level_completed: u8,
    victory: bool,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let extra = [level_completed, if victory { 1 } else { 0 }];
    invoke_manual_cpi_signed(
        program,
        PLAYER_PROFILE_PROGRAM_ID,
        &RECORD_RUN_RESULT_CPI_DISCRIMINATOR,
        &extra,
        &[
            (player_profile, true, false),
            (session, false, false),
            (session_signer, false, true),
            (session_manager_authority, false, true),
        ],
        signer_seeds,
    )
}

// ============================================================================
// Close CPI Functions for end_session
// ============================================================================

pub const CLOSE_GAME_STATE_VIA_SESSION_SIGNER_DISCRIMINATOR: [u8; 8] =
    [199, 166, 186, 238, 90, 16, 234, 79];
pub const CLOSE_MAP_ENEMIES_DISCRIMINATOR: [u8; 8] = [192, 111, 190, 66, 236, 132, 252, 88];
pub const CLOSE_GENERATED_MAP_DISCRIMINATOR: [u8; 8] = [249, 208, 241, 231, 57, 214, 174, 103];
pub const CLOSE_MAP_POIS_VIA_SESSION_SIGNER_DISCRIMINATOR: [u8; 8] =
    [35, 38, 19, 18, 250, 66, 39, 150];
pub const CLOSE_MAP_POIS_ORPHANED_DISCRIMINATOR: [u8; 8] =
    [218, 44, 98, 133, 139, 114, 27, 98];

fn close_game_state_via_session_signer_cpi<'info>(
    program: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        gameplay_state::ID,
        &CLOSE_GAME_STATE_VIA_SESSION_SIGNER_DISCRIMINATOR,
        &[],
        &[
            (game_state, true, false),
            (player, true, false),
            (session_signer, false, true),
        ],
    )
}

fn close_map_enemies_cpi<'info>(
    program: &AccountInfo<'info>,
    map_enemies: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        gameplay_state::ID,
        &CLOSE_MAP_ENEMIES_DISCRIMINATOR,
        &[],
        &[
            (map_enemies, true, false),
            (game_state, false, false),
            (player, true, false),
            (session_signer, false, true),
        ],
    )
}

fn close_generated_map_cpi<'info>(
    program: &AccountInfo<'info>,
    generated_map: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        MAP_GENERATOR_PROGRAM_ID,
        &CLOSE_GENERATED_MAP_DISCRIMINATOR,
        &[],
        &[
            (generated_map, true, false),
            (session, false, false),
            (player, true, false),
            (session_signer, false, true),
        ],
    )
}

fn close_map_pois_via_session_signer_cpi<'info>(
    program: &AccountInfo<'info>,
    map_pois: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        POI_SYSTEM_PROGRAM_ID,
        &CLOSE_MAP_POIS_VIA_SESSION_SIGNER_DISCRIMINATOR,
        &[],
        &[
            (map_pois, true, false),
            (session, false, false),
            (player, true, false),
            (session_signer, false, true),
        ],
    )
}

fn close_map_pois_orphaned_cpi<'info>(
    program: &AccountInfo<'info>,
    map_pois: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        POI_SYSTEM_PROGRAM_ID,
        &CLOSE_MAP_POIS_ORPHANED_DISCRIMINATOR,
        &[],
        &[
            (map_pois, true, false),
            (game_state, false, false),
            (player, true, false),
            (session_signer, false, true),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates that END_SESSION_DISCRIMINATOR matches sha256("global:end_session")[..8].
    /// Computes the hash at test time so a rename is caught immediately.
    #[test]
    fn test_end_session_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:end_session");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            END_SESSION_DISCRIMINATOR, expected,
            "END_SESSION_DISCRIMINATOR doesn't match sha256(\"global:end_session\")[..8]"
        );
    }

    /// Validates that CONSUME_RUN_DISCRIMINATOR matches sha256("global:consume_run")[..8].
    /// Computes the hash at test time so a rename is caught immediately.
    #[test]
    fn test_consume_run_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:consume_run");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CONSUME_RUN_DISCRIMINATOR, expected,
            "CONSUME_RUN_DISCRIMINATOR doesn't match sha256(\"global:consume_run\")[..8]"
        );
    }

    /// Validates that INITIALIZE_MAP_POIS_DISCRIMINATOR matches sha256("global:initialize_map_pois")[..8].
    /// Computes the hash at test time so a rename is caught immediately.
    #[test]
    fn test_initialize_map_pois_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:initialize_map_pois");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            INITIALIZE_MAP_POIS_DISCRIMINATOR, expected,
            "INITIALIZE_MAP_POIS_DISCRIMINATOR doesn't match sha256(\"global:initialize_map_pois\")[..8]"
        );
    }

    /// Validates that RECORD_RUN_RESULT_CPI_DISCRIMINATOR matches sha256("global:record_run_result_cpi")[..8].
    /// Computes the hash at test time so a rename is caught immediately.
    #[test]
    fn test_record_run_result_cpi_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:record_run_result_cpi");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            RECORD_RUN_RESULT_CPI_DISCRIMINATOR, expected,
            "RECORD_RUN_RESULT_CPI_DISCRIMINATOR doesn't match sha256(\"global:record_run_result_cpi\")[..8]"
        );
    }

    #[test]
    fn test_close_game_state_via_session_signer_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_game_state_via_session_signer");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_GAME_STATE_VIA_SESSION_SIGNER_DISCRIMINATOR, expected,
            "CLOSE_GAME_STATE_VIA_SESSION_SIGNER_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_map_enemies_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_map_enemies");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_MAP_ENEMIES_DISCRIMINATOR, expected,
            "CLOSE_MAP_ENEMIES_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_generated_map_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_generated_map");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_GENERATED_MAP_DISCRIMINATOR, expected,
            "CLOSE_GENERATED_MAP_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_map_pois_via_session_signer_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_map_pois_via_session_signer");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_MAP_POIS_VIA_SESSION_SIGNER_DISCRIMINATOR, expected,
            "CLOSE_MAP_POIS_VIA_SESSION_SIGNER_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_map_pois_orphaned_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_map_pois_orphaned");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_MAP_POIS_ORPHANED_DISCRIMINATOR, expected,
            "CLOSE_MAP_POIS_ORPHANED_DISCRIMINATOR doesn't match"
        );
    }
}
