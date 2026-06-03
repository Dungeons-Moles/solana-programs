use anchor_lang::prelude::*;
use er_compat::DelegateConfig;
pub mod constants;
pub mod errors;
pub mod state;
use constants::{DUEL_CAMPAIGN_LEVEL, GAUNTLET_CAMPAIGN_LEVEL};

use errors::SessionManagerError;
use gameplay_state::program::GameplayState;
use gameplay_state::state::{DuelEntry, GameState};
use map_generator::program::MapGenerator;
use map_generator::state::GeneratedMap;
use player_inventory::program::PlayerInventory;
use state::{
    GameSession, SessionCounter, SessionNonces, SessionRelicEntry, MAX_SESSION_RELICS,
    EMPTY_STATE_HASH,
};

declare_id!("CrU4bUFreKy2XsoU2oksdJWKim11w2VpagKBQ2MTkyMz");

/// Player Profile program ID for cross-program account validation
/// Must match the declare_id! in player-profile/src/lib.rs
/// GSLNDrNoHeZXVxB7Yu7tUe8417PpZ5XV7JPYupPw9WQy
pub const PLAYER_PROFILE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xe5, 0x5c, 0xb8, 0xa0, 0xc0, 0xde, 0x56, 0xac, 0x2e, 0xc2, 0xd5, 0xd3, 0xc9, 0x2d, 0x21, 0xb2,
    0x46, 0x6d, 0xa1, 0x4e, 0xef, 0x0f, 0x74, 0xd1, 0x24, 0x1a, 0x99, 0x3e, 0xe5, 0x87, 0x67, 0xa2,
]);

/// POI System program ID for manual CPI.
/// Must match the declare_id! in poi-system/src/lib.rs
/// 7rTRqR6H8ztxpcPVKtAwXGi7PQFDYLgMkWSBRLPcYMH2
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x65, 0xd1, 0x76, 0xb1, 0x94, 0xe7, 0xc4, 0x89, 0xa6, 0x09, 0xbd, 0xa7, 0x8c, 0x0c, 0x0a, 0xe6,
    0xf6, 0xae, 0xd1, 0x4e, 0xf0, 0xba, 0xe9, 0x21, 0xbb, 0xde, 0x72, 0x90, 0xf1, 0x04, 0xcc, 0xf9,
]);

/// Map Generator program ID for CPI.
/// Must match the declare_id! in map-generator/src/lib.rs
/// E6kc5Edg1s3AXVQQFRoYdAq4vPAFbkYbP7B5ujiuZwz4
pub const MAP_GENERATOR_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xc2, 0xa1, 0x23, 0x59, 0xb5, 0x1b, 0xb6, 0xab, 0x78, 0x0b, 0x8a, 0x7e, 0xfe, 0x9a, 0xc0, 0x32,
    0x1b, 0x52, 0x47, 0x61, 0xfa, 0xb6, 0x73, 0x57, 0xc8, 0xa3, 0x1f, 0xbd, 0x67, 0xe8, 0x8d, 0xe5,
]);

/// NFT marketplace program ID for validating relic metadata proofs.
pub const NFT_MARKETPLACE_PROGRAM_ID: Pubkey = pubkey!("GLKxBpZ8hc7qzvD9VHAVsJEjHSu2JVp1HaPrGH4fpTci");

/// Metaplex Core program ID for validating asset ownership proofs.
pub const MPL_CORE_PROGRAM_ID: Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

/// Discriminator for player_profile::consume_run instruction.
/// Computed as sha256("global:consume_run")[..8].
///
/// NOTE: This is manually specified because session-manager already has a
/// manual PlayerProfile struct (avoiding circular deps). If player-profile's
/// consume_run instruction changes, this must be updated.
pub const CONSUME_RUN_DISCRIMINATOR: [u8; 8] = [0x6b, 0x65, 0x36, 0x52, 0x84, 0x9c, 0x0f, 0x22];
/// Discriminator for player_profile::sync_relic_ownership instruction.
/// Computed as sha256("global:sync_relic_ownership")[..8].
pub const SYNC_RELIC_OWNERSHIP_DISCRIMINATOR: [u8; 8] =
    [35, 216, 49, 188, 212, 247, 12, 202];

/// Discriminator for poi_system::initialize_map_pois instruction.
/// Computed as sha256("global:initialize_map_pois")[..8].
///
/// NOTE: This is manually specified because session-manager cannot depend on poi-system
/// (circular dependency). If poi-system's initialize_map_pois instruction changes, this must be updated.
pub const INITIALIZE_MAP_POIS_DISCRIMINATOR: [u8; 8] =
    [0xa8, 0xec, 0xff, 0x37, 0xee, 0xd2, 0x19, 0xfb];
pub const SESSION_MANAGER_AUTHORITY_SEED: &[u8] = b"session_manager_authority";
fn local_delegate_config(validator: Option<Pubkey>) -> DelegateConfig {
    DelegateConfig {
        validator: validator.map(|v| unsafe { std::mem::transmute(v) }),
        ..DelegateConfig::default()
    }
}

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
        // Store bump on first creation (idempotent for existing accounts)
        ctx.accounts.session_nonces.bump = ctx.bumps.session_nonces;

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
        if let Some(pool) = ctx.accounts.player_relic_pool.as_mut() {
            let owned_item_ids =
                collect_owned_relic_item_ids(ctx.accounts.player.key(), ctx.remaining_accounts)?;
            sync_relic_ownership_cpi(
                &ctx.accounts.player_profile_program.to_account_info(),
                &pool.to_account_info(),
                &ctx.accounts.player.to_account_info(),
                owned_item_ids,
            )?;
            pool.reload()?;
        } else {
            require!(
                ctx.remaining_accounts.is_empty(),
                SessionManagerError::InvalidRelicOwnershipProofs
            );
        }
        let (active_relic_count, active_relics) =
            session_relic_snapshot(ctx.accounts.player_relic_pool.as_ref().map(|v| &**v));

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
            session.active_relic_count = active_relic_count;
            session.active_relics = active_relics;
            // Store session key signer pubkey
            session.session_signer = session_signer_key;
            session.settled = false;
            session.settled_victory = false;
            session.settled_at = 0;
        }

        // 1. Allocate empty GeneratedMap (no maze generation — map is filled on ER via fill_map_with_seed)
        map_generator::cpi::init_map_account(
            CpiContext::new(
                ctx.accounts.map_generator_program.key(),
                map_generator::cpi::accounts::InitMapAccount {
                    payer: ctx.accounts.session_signer.to_account_info(),
                    session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
        )?;

        // 1b. Allocate empty SessionDiscovery (populated on ER during map generation).
        // Optional: when omitted, the frontend must call init_session_discovery separately.
        if let Some(ref sd) = ctx.accounts.session_discovery {
            map_generator::cpi::init_session_discovery(
                CpiContext::new(
                    ctx.accounts.map_generator_program.key(),
                    map_generator::cpi::accounts::InitSessionDiscovery {
                        payer: ctx.accounts.session_signer.to_account_info(),
                        session: ctx.accounts.game_session.to_account_info(),
                        session_discovery: sd.to_account_info(),
                        system_program: ctx.accounts.system_program.to_account_info(),
                    },
                ),
            )?;
        }

        // 2. Initialize Game State with placeholder map dimensions (50x50, spawn 0,0).
        // Map dimensions and spawn position will be synced after fill_map_with_seed on ER.
        gameplay_state::cpi::initialize_game_state(
            CpiContext::new(
                ctx.accounts.gameplay_state_program.key(),
                gameplay_state::cpi::accounts::InitializeGameState {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    game_session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    payer: ctx.accounts.session_signer.to_account_info(),
                    player: ctx.accounts.player.to_account_info(),
                    session_signer: ctx.accounts.session_signer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            50, // MAP_WIDTH — fixed 50x50 map
            50, // MAP_HEIGHT
            0,  // placeholder spawn_x
            0,  // placeholder spawn_y
        )?;

        // 4. Initialize Inventory for this session
        // Each session gets its own inventory (PDA derived from session key).
        // This ensures clean inventory state per run and allows concurrent sessions.
        // IMPORTANT: Use session_signer as the inventory owner since all gameplay
        // transactions (equip, fuse, etc.) are signed by the session key signer.
        player_inventory::cpi::initialize_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.key(),
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
            &ctx.accounts.session_signer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            act,
            week,
            poi_seed,
        )?;

        // TODO(VRF): When ephemeral-vrf-sdk is available, optionally CPI into
        // map_generator::request_map_vrf and poi_system::request_poi_vrf if the
        // frontend passes VRF oracle accounts via remaining_accounts.
        // Pattern: if ctx.remaining_accounts.len() >= EXPECTED_VRF_ACCOUNT_COUNT { ... }

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
        // Store bump on first creation (idempotent for existing accounts)
        ctx.accounts.session_nonces.bump = ctx.bumps.session_nonces;

        let player_profile = &ctx.accounts.player_profile;
        let campaign_level = DUEL_CAMPAIGN_LEVEL;

        let counter = &mut ctx.accounts.session_counter;
        let clock = Clock::get()?;
        let session_player = ctx.accounts.player.key();
        let session_signer_key = ctx.accounts.session_signer.key();
        if let Some(pool) = ctx.accounts.player_relic_pool.as_mut() {
            let owned_item_ids =
                collect_owned_relic_item_ids(ctx.accounts.player.key(), ctx.remaining_accounts)?;
            sync_relic_ownership_cpi(
                &ctx.accounts.player_profile_program.to_account_info(),
                &pool.to_account_info(),
                &ctx.accounts.player.to_account_info(),
                owned_item_ids,
            )?;
            pool.reload()?;
        } else {
            require!(
                ctx.remaining_accounts.is_empty(),
                SessionManagerError::InvalidRelicOwnershipProofs
            );
        }
        let (active_relic_count, active_relics) =
            session_relic_snapshot(ctx.accounts.player_relic_pool.as_ref().map(|v| &**v));

        counter.count = counter
            .count
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;

        // Derive session PDA key (duel uses fixed seed prefix + nonce)
        let duel_nonce_bytes = ctx.accounts.session_nonces.duel_nonce.to_le_bytes();
        let (_session_pda, _) = Pubkey::find_program_address(
            &[
                GameSession::DUEL_SEED_PREFIX,
                session_player.as_ref(),
                &duel_nonce_bytes,
            ],
            &crate::ID,
        );

        // POI seed: set to 0 (placeholder). All POI offer generation uses VRF
        // directly at interaction time (enter_shop, interact_pick_item, etc.) on ER.
        // This field is stored in MapPois but no longer drives randomness.
        let poi_seed = 0u64;

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
            session.active_relic_count = active_relic_count;
            session.active_relics = active_relics;
            session.session_signer = session_signer_key;
            session.settled = false;
            session.settled_victory = false;
            session.settled_at = 0;
        }

        // Allocate empty GeneratedMap (filled on ER via generate_map_with_vrf after VRF fulfillment)
        map_generator::cpi::init_map_account(
            CpiContext::new(
                ctx.accounts.map_generator_program.key(),
                map_generator::cpi::accounts::InitMapAccount {
                    payer: ctx.accounts.session_signer.to_account_info(),
                    session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
        )?;

        // Allocate empty SessionDiscovery (populated on ER during map generation).
        // Optional: when omitted, the frontend must call init_session_discovery separately.
        if let Some(ref sd) = ctx.accounts.session_discovery {
            map_generator::cpi::init_session_discovery(
                CpiContext::new(
                    ctx.accounts.map_generator_program.key(),
                    map_generator::cpi::accounts::InitSessionDiscovery {
                        payer: ctx.accounts.session_signer.to_account_info(),
                        session: ctx.accounts.game_session.to_account_info(),
                        session_discovery: sd.to_account_info(),
                        system_program: ctx.accounts.system_program.to_account_info(),
                    },
                ),
            )?;
        }

        // Initialize Game State with placeholder dimensions; actual spawn set after map fill on ER.
        gameplay_state::cpi::initialize_game_state(
            CpiContext::new(
                ctx.accounts.gameplay_state_program.key(),
                gameplay_state::cpi::accounts::InitializeGameState {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    game_session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    payer: ctx.accounts.session_signer.to_account_info(),
                    player: ctx.accounts.player.to_account_info(),
                    session_signer: ctx.accounts.session_signer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            50, // MAP_WIDTH placeholder
            50, // MAP_HEIGHT placeholder
            0,  // placeholder spawn_x
            0,  // placeholder spawn_y
        )?;

        let session_manager_authority_signer: &[&[&[u8]]] = &[&[
            SESSION_MANAGER_AUTHORITY_SEED,
            &[ctx.bumps.session_manager_authority],
        ]];
        gameplay_state::cpi::configure_run_mode(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.key(),
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
            ctx.accounts.player_inventory_program.key(),
            player_inventory::cpi::accounts::InitializeInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                session: ctx.accounts.game_session.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
        ))?;

        let act = (campaign_level - 1) / 10 + 1;
        let week = 1u8;

        initialize_map_pois_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.session_signer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            act,
            week,
            poi_seed,
        )?;
        // NOTE: map VRF is NOT consumed here — it remains in Fulfilled state
        // for ER to use during generate_map_with_vrf after delegation.

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
        // Store bump on first creation (idempotent for existing accounts)
        ctx.accounts.session_nonces.bump = ctx.bumps.session_nonces;

        let player_profile = &ctx.accounts.player_profile;
        let campaign_level = GAUNTLET_CAMPAIGN_LEVEL;

        let counter = &mut ctx.accounts.session_counter;
        let clock = Clock::get()?;
        let session_player = ctx.accounts.player.key();
        let session_signer_key = ctx.accounts.session_signer.key();
        if let Some(pool) = ctx.accounts.player_relic_pool.as_mut() {
            let owned_item_ids =
                collect_owned_relic_item_ids(ctx.accounts.player.key(), ctx.remaining_accounts)?;
            sync_relic_ownership_cpi(
                &ctx.accounts.player_profile_program.to_account_info(),
                &pool.to_account_info(),
                &ctx.accounts.player.to_account_info(),
                owned_item_ids,
            )?;
            pool.reload()?;
        } else {
            require!(
                ctx.remaining_accounts.is_empty(),
                SessionManagerError::InvalidRelicOwnershipProofs
            );
        }
        let (active_relic_count, active_relics) =
            session_relic_snapshot(ctx.accounts.player_relic_pool.as_ref().map(|v| &**v));

        counter.count = counter
            .count
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;

        // Derive session PDA key (gauntlet uses prefix + nonce)
        let gauntlet_nonce_bytes = ctx.accounts.session_nonces.gauntlet_nonce.to_le_bytes();
        let (_session_pda, _) = Pubkey::find_program_address(
            &[
                GameSession::GAUNTLET_SEED_PREFIX,
                session_player.as_ref(),
                &gauntlet_nonce_bytes,
            ],
            &crate::ID,
        );

        // POI seed: set to 0 (placeholder). All POI offer generation uses VRF
        // directly at interaction time (enter_shop, interact_pick_item, etc.) on ER.
        // This field is stored in MapPois but no longer drives randomness.
        let poi_seed = 0u64;

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
            session.active_relic_count = active_relic_count;
            session.active_relics = active_relics;
            session.session_signer = session_signer_key;
            session.settled = false;
            session.settled_victory = false;
            session.settled_at = 0;
        }

        // Allocate empty GeneratedMap (filled on ER via generate_map_with_vrf after VRF fulfillment)
        map_generator::cpi::init_map_account(
            CpiContext::new(
                ctx.accounts.map_generator_program.key(),
                map_generator::cpi::accounts::InitMapAccount {
                    payer: ctx.accounts.session_signer.to_account_info(),
                    session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
        )?;

        // Allocate empty SessionDiscovery (populated on ER during map generation).
        // Optional: when omitted, the frontend must call init_session_discovery separately.
        if let Some(ref sd) = ctx.accounts.session_discovery {
            map_generator::cpi::init_session_discovery(
                CpiContext::new(
                    ctx.accounts.map_generator_program.key(),
                    map_generator::cpi::accounts::InitSessionDiscovery {
                        payer: ctx.accounts.session_signer.to_account_info(),
                        session: ctx.accounts.game_session.to_account_info(),
                        session_discovery: sd.to_account_info(),
                        system_program: ctx.accounts.system_program.to_account_info(),
                    },
                ),
            )?;
        }

        // Initialize Game State with placeholder dimensions; actual spawn set after map fill on ER.
        gameplay_state::cpi::initialize_game_state(
            CpiContext::new(
                ctx.accounts.gameplay_state_program.key(),
                gameplay_state::cpi::accounts::InitializeGameState {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    game_session: ctx.accounts.game_session.to_account_info(),
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    payer: ctx.accounts.session_signer.to_account_info(),
                    player: ctx.accounts.player.to_account_info(),
                    session_signer: ctx.accounts.session_signer.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                },
            ),
            campaign_level,
            50, // MAP_WIDTH placeholder
            50, // MAP_HEIGHT placeholder
            0,  // placeholder spawn_x
            0,  // placeholder spawn_y
        )?;

        let session_manager_authority_signer: &[&[&[u8]]] = &[&[
            SESSION_MANAGER_AUTHORITY_SEED,
            &[ctx.bumps.session_manager_authority],
        ]];
        gameplay_state::cpi::configure_run_mode(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.key(),
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
            ctx.accounts.player_inventory_program.key(),
            player_inventory::cpi::accounts::InitializeInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                session: ctx.accounts.game_session.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
        ))?;

        let act = (campaign_level - 1) / 10 + 1;
        let week = 1u8;

        initialize_map_pois_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.session_signer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            act,
            week,
            poi_seed,
        )?;
        // NOTE: map VRF is NOT consumed here — it remains in Fulfilled state
        // for ER to use during generate_map_with_vrf after delegation.

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
        let game_session_key = derive_campaign_session_pda(
            &ctx.accounts.player.key(),
            campaign_level,
            ctx.accounts.session_nonces.campaign_nonce,
        );
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
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.game_state,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_game_state,
            &ctx.accounts.delegation_record_game_state,
            &ctx.accounts.delegation_metadata_game_state,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            game_state_seeds,
            local_delegate_config(None),
        )?;
        Ok(())
    }

    /// Delegates generated-map account to the MagicBlock delegation program.
    pub fn delegate_generated_map(
        ctx: Context<DelegateGeneratedMap>,
        campaign_level: u8,
    ) -> Result<()> {
        let game_session_key = derive_campaign_session_pda(
            &ctx.accounts.player.key(),
            campaign_level,
            ctx.accounts.session_nonces.campaign_nonce,
        );
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
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.generated_map,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_generated_map,
            &ctx.accounts.delegation_record_generated_map,
            &ctx.accounts.delegation_metadata_generated_map,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            generated_map_seeds,
            local_delegate_config(None),
        )?;
        Ok(())
    }

    /// Delegates inventory account to the MagicBlock delegation program.
    pub fn delegate_inventory(ctx: Context<DelegateInventory>, campaign_level: u8) -> Result<()> {
        let game_session_key = derive_campaign_session_pda(
            &ctx.accounts.player.key(),
            campaign_level,
            ctx.accounts.session_nonces.campaign_nonce,
        );
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
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.inventory,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_inventory,
            &ctx.accounts.delegation_record_inventory,
            &ctx.accounts.delegation_metadata_inventory,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            inventory_seeds,
            local_delegate_config(None),
        )?;
        Ok(())
    }

    /// Delegates map-pois account to the MagicBlock delegation program.
    pub fn delegate_map_pois(ctx: Context<DelegateMapPois>, campaign_level: u8) -> Result<()> {
        let game_session_key = derive_campaign_session_pda(
            &ctx.accounts.player.key(),
            campaign_level,
            ctx.accounts.session_nonces.campaign_nonce,
        );
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
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.map_pois,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_map_pois,
            &ctx.accounts.delegation_record_map_pois,
            &ctx.accounts.delegation_metadata_map_pois,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            map_pois_seeds,
            local_delegate_config(None),
        )?;
        Ok(())
    }

    /// Delegates PoiVrfState account to the MagicBlock delegation program.
    /// Only needed for PvP sessions that use VRF for POI offers.
    pub fn delegate_poi_vrf_state(
        ctx: Context<DelegatePoiVrfState>,
        session_key: Pubkey,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let (expected_poi_vrf, _) = Pubkey::find_program_address(
            &[b"poi_vrf", session_key.as_ref()],
            &POI_SYSTEM_PROGRAM_ID,
        );
        require_keys_eq!(
            ctx.accounts.poi_vrf_state.key(),
            expected_poi_vrf,
            SessionManagerError::Unauthorized
        );
        let poi_vrf_seeds: &[&[u8]] = &[b"poi_vrf", session_key.as_ref()];
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.poi_vrf_state,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_poi_vrf_state,
            &ctx.accounts.delegation_record_poi_vrf_state,
            &ctx.accounts.delegation_metadata_poi_vrf_state,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            poi_vrf_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Marks the session delegated and delegates the session account itself.
    pub fn delegate_session(
        ctx: Context<DelegateSession>,
        campaign_level: u8,
        validator: Option<Pubkey>,
    ) -> Result<()> {
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
        let campaign_nonce_bytes = ctx.accounts.session_nonces.campaign_nonce.to_le_bytes();
        let duel_nonce_bytes = ctx.accounts.session_nonces.duel_nonce.to_le_bytes();
        let gauntlet_nonce_bytes = ctx.accounts.session_nonces.gauntlet_nonce.to_le_bytes();

        let campaign_session_seeds: &[&[u8]] = &[
            GameSession::SEED_PREFIX,
            session_player.as_ref(),
            &campaign_seed,
            &campaign_nonce_bytes,
        ];
        let duel_session_seeds: &[&[u8]] = &[
            GameSession::DUEL_SEED_PREFIX,
            session_player.as_ref(),
            &duel_nonce_bytes,
        ];
        let gauntlet_session_seeds: &[&[u8]] = &[
            GameSession::GAUNTLET_SEED_PREFIX,
            session_player.as_ref(),
            &gauntlet_nonce_bytes,
        ];

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

        er_compat::delegate_account(
            &ctx.accounts.session_signer.to_account_info(),
            &ctx.accounts.game_session,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_game_session,
            &ctx.accounts.delegation_record_game_session,
            &ctx.accounts.delegation_metadata_game_session,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            session_seeds,
            local_delegate_config(validator),
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
        let generated_map_info = ctx.accounts.generated_map.to_account_info();
        let inventory_info = ctx.accounts.inventory.to_account_info();
        let map_pois_info = ctx.accounts.map_pois.to_account_info();
        let poi_vrf_info = ctx
            .accounts
            .poi_vrf_state
            .as_ref()
            .map(|a| a.to_account_info());
        let mut accounts_to_commit = vec![
            game_session_info,
            game_state_info,
            generated_map_info,
            inventory_info,
            map_pois_info,
        ];
        if let Some(info) = poi_vrf_info {
            accounts_to_commit.push(info);
        }
        er_compat::commit_accounts(
            ctx.accounts.player.to_account_info(),
            ctx.accounts.magic_context.to_account_info(),
            ctx.accounts.magic_program.to_account_info(),
            &accounts_to_commit,
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

        er_compat::commit_and_undelegate(
            ctx.accounts.session_signer.to_account_info(),
            ctx.accounts.magic_context.to_account_info(),
            ctx.accounts.magic_program.to_account_info(),
            &[game_session_info],
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
        )?;
        validate_secondary_runtime_accounts(
            &game_session_key,
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.inventory.to_account_info(),
            &ctx.accounts.map_pois.to_account_info(),
        )?;

        let game_state = read_game_state_unchecked(&ctx.accounts.game_state.to_account_info())?;
        require!(
            game_state.is_dead || game_state.completed,
            SessionManagerError::RunNotTerminal
        );

        if game_state.run_mode == gameplay_state::state::RunMode::Duel {
            let duel_entry = ctx
                .accounts
                .duel_entry
                .as_ref()
                .ok_or(SessionManagerError::DuelSettlementRequired)?;
            require!(
                duel_entry.settled || duel_entry.entry_lamports == 0,
                SessionManagerError::DuelSettlementRequired
            );
        }

        // Do not trust `session.is_delegated` bit here; legacy undelegate flows can leave
        // the flag stale even after ownership returns to session-manager.

        // Determine victory from game state (completed and not dead means victory).
        // Any other state closes as defeat so cleanup can recover stuck sessions.
        let victory = game_state.completed && !game_state.is_dead;

        if !ctx.accounts.game_session.settled {
            let unlock_randomness = extract_unlock_randomness(
                &ctx.accounts.gameplay_vrf_state,
                &ctx.accounts.map_vrf_state,
                &game_session_key,
            );
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                victory,
                &unlock_randomness,
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
        // Order matters: close VRF states first, then child accounts, then parent accounts

        // 0a. Close MapVrfState if present (PvP sessions only)
        if let Some(ref map_vrf) = ctx.accounts.map_vrf_state {
            close_map_vrf_state_cpi(
                &ctx.accounts.map_generator_program,
                &map_vrf.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 0b. Close PoiVrfState if present (PvP sessions only)
        if let Some(ref poi_vrf) = ctx.accounts.poi_vrf_state {
            close_poi_vrf_state_cpi(
                &ctx.accounts.poi_system_program,
                &poi_vrf.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 0c. Close GameplayVrfState if present
        if let Some(ref gameplay_vrf) = ctx.accounts.gameplay_vrf_state {
            close_gameplay_vrf_state_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &gameplay_vrf.to_account_info(),
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 1. Close map_pois — skip CPI if account is corrupted (e.g., after failed ER commit)
        {
            let info = ctx.accounts.map_pois.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_map_pois_via_session_signer_cpi(
                    &ctx.accounts.poi_system_program,
                    &ctx.accounts.map_pois,
                    &ctx.accounts.game_session.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 1b. Close session_discovery — skip CPI if corrupted
        {
            let info = ctx.accounts.session_discovery.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_session_discovery_cpi(
                    &ctx.accounts.map_generator_program,
                    &ctx.accounts.session_discovery,
                    &ctx.accounts.game_session.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 2. Close generated_map — skip CPI if corrupted
        {
            let info = ctx.accounts.generated_map.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_generated_map_cpi(
                    &ctx.accounts.map_generator_program,
                    &ctx.accounts.generated_map,
                    &ctx.accounts.game_session.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 3. Close gauntlet_echoes if present (depends on game_state)
        if let Some(ref ge) = ctx.accounts.gauntlet_echoes {
            close_gauntlet_echoes_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &ge.to_account_info(),
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 4. Close game_state — skip CPI if corrupted
        {
            let info = ctx.accounts.game_state.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_game_state_via_session_signer_cpi(
                    &ctx.accounts.gameplay_state_program.to_account_info(),
                    &ctx.accounts.game_state.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 5. Close inventory via CPI to ensure fresh inventory for next session
        // Use session_signer since it's the inventory owner (set during start_session)
        player_inventory::cpi::close_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.key(),
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
    pub fn settle_session_result(
        ctx: Context<SettleSessionResult>,
        _campaign_level: u8,
    ) -> Result<()> {
        let clock = Clock::get()?;

        let (expected_game_state, _) = Pubkey::find_program_address(
            &[b"game_state", ctx.accounts.game_session.key().as_ref()],
            &gameplay_state::ID,
        );
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
            // No VRF accounts available in settle_session_result context
            let unlock_randomness = [0u8; 32];
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                victory,
                &unlock_randomness,
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

        let (expected_game_state, _) = Pubkey::find_program_address(
            &[b"game_state", ctx.accounts.game_session.key().as_ref()],
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
            // No VRF accounts available in close_session_only context
            let unlock_randomness = [0u8; 32];
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                victory,
                &unlock_randomness,
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
        // No VRF accounts available in force_close_session context
        let unlock_randomness = [0u8; 32];

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
                &unlock_randomness,
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
        // Order: map_pois, generated_map, game_state, inventory.
        let game_state_closeable = *ctx.accounts.game_state.owner == gameplay_state::ID;

        if *ctx.accounts.map_pois.owner == POI_SYSTEM_PROGRAM_ID {
            close_map_pois_via_session_signer_cpi(
                &ctx.accounts.poi_system_program,
                &ctx.accounts.map_pois,
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        if *ctx.accounts.session_discovery.owner == MAP_GENERATOR_PROGRAM_ID {
            close_session_discovery_cpi(
                &ctx.accounts.map_generator_program,
                &ctx.accounts.session_discovery,
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
                ctx.accounts.player_inventory_program.key(),
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
    /// Call order: map_pois, session_discovery, generated_map, inventory → game_state last.
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

        // Close session_discovery if on base layer (owned by map-generator)
        if *ctx.accounts.session_discovery.owner == MAP_GENERATOR_PROGRAM_ID {
            close_session_discovery_orphaned_cpi(
                &ctx.accounts.map_generator_program,
                &ctx.accounts.session_discovery,
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // Close generated_map if on base layer (owned by map-generator)
        if *ctx.accounts.generated_map.owner == MAP_GENERATOR_PROGRAM_ID {
            close_generated_map_orphaned_cpi(
                &ctx.accounts.map_generator_program,
                &ctx.accounts.generated_map,
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // Close inventory if on base layer (owned by player-inventory)
        if *ctx.accounts.inventory.owner == player_inventory::ID {
            player_inventory::cpi::close_inventory(CpiContext::new(
                ctx.accounts.player_inventory_program.key(),
                player_inventory::cpi::accounts::CloseInventory {
                    inventory: ctx.accounts.inventory.to_account_info(),
                    player: ctx.accounts.session_signer.to_account_info(),
                },
            ))?;
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
            let game_session_key = ctx.accounts.game_session.key();
            let unlock_randomness = extract_unlock_randomness(
                &ctx.accounts.gameplay_vrf_state,
                &ctx.accounts.map_vrf_state,
                &game_session_key,
            );
            record_run_result_cpi(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.player_profile.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.session_signer.to_account_info(),
                &ctx.accounts.session_manager_authority.to_account_info(),
                ctx.accounts.game_session.campaign_level,
                false,
                &unlock_randomness,
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
        // Order matters: close VRF states first, then child accounts, then parent accounts

        // 0a. Close MapVrfState if present (PvP sessions only)
        if let Some(ref map_vrf) = ctx.accounts.map_vrf_state {
            close_map_vrf_state_cpi(
                &ctx.accounts.map_generator_program,
                &map_vrf.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 0b. Close PoiVrfState if present (PvP sessions only)
        if let Some(ref poi_vrf) = ctx.accounts.poi_vrf_state {
            close_poi_vrf_state_cpi(
                &ctx.accounts.poi_system_program,
                &poi_vrf.to_account_info(),
                &ctx.accounts.game_session.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 0c. Close GameplayVrfState if present
        if let Some(ref gameplay_vrf) = ctx.accounts.gameplay_vrf_state {
            close_gameplay_vrf_state_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &gameplay_vrf.to_account_info(),
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 1. Close map_pois — skip CPI if account is corrupted (e.g., after failed ER commit)
        {
            let info = ctx.accounts.map_pois.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_map_pois_via_session_signer_cpi(
                    &ctx.accounts.poi_system_program,
                    &ctx.accounts.map_pois,
                    &ctx.accounts.game_session.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 1b. Close session_discovery — skip CPI if corrupted
        {
            let info = ctx.accounts.session_discovery.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_session_discovery_cpi(
                    &ctx.accounts.map_generator_program,
                    &ctx.accounts.session_discovery,
                    &ctx.accounts.game_session.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 2. Close generated_map — skip CPI if corrupted
        {
            let info = ctx.accounts.generated_map.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_generated_map_cpi(
                    &ctx.accounts.map_generator_program,
                    &ctx.accounts.generated_map,
                    &ctx.accounts.game_session.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 3. Close gauntlet_echoes if present (depends on game_state)
        if let Some(ref ge) = ctx.accounts.gauntlet_echoes {
            close_gauntlet_echoes_cpi(
                &ctx.accounts.gameplay_state_program.to_account_info(),
                &ge.to_account_info(),
                &ctx.accounts.game_state.to_account_info(),
                &ctx.accounts.player,
                &ctx.accounts.session_signer.to_account_info(),
            )?;
        }

        // 4. Close game_state — skip CPI if corrupted
        {
            let info = ctx.accounts.game_state.to_account_info();
            let data = info.try_borrow_data()?;
            let valid = data.len() >= 8 && data[..8] != [0u8; 8];
            drop(data);
            if valid {
                close_game_state_via_session_signer_cpi(
                    &ctx.accounts.gameplay_state_program.to_account_info(),
                    &ctx.accounts.game_state.to_account_info(),
                    &ctx.accounts.player,
                    &ctx.accounts.session_signer.to_account_info(),
                )?;
            }
            // else: account corrupted (e.g., after failed ER commit) — skip close,
            // rent lamports are forfeit. Session can still be abandoned.
        }

        // 5. Close inventory via CPI
        player_inventory::cpi::close_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.key(),
            player_inventory::cpi::accounts::CloseInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                player: ctx.accounts.session_signer.to_account_info(),
            },
        ))?;

        // 6. Session account will be closed by Anchor (close = player constraint)
        Ok(())
    }

    /// Override a stuck campaign session by incrementing the campaign nonce.
    /// Wallet-only — no session key required. After calling this, start_session
    /// will create at a new PDA, bypassing the stuck session.
    pub fn override_campaign_session(ctx: Context<OverrideSession>) -> Result<()> {
        let nonces = &mut ctx.accounts.session_nonces;
        nonces.bump = ctx.bumps.session_nonces;
        nonces.campaign_nonce = nonces
            .campaign_nonce
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;
        emit!(SessionOverridden {
            player: ctx.accounts.player.key(),
            mode: "campaign".to_string(),
            new_nonce: nonces.campaign_nonce,
        });
        Ok(())
    }

    /// Override a stuck duel session by incrementing the duel nonce.
    pub fn override_duel_session(ctx: Context<OverrideSession>) -> Result<()> {
        let nonces = &mut ctx.accounts.session_nonces;
        nonces.bump = ctx.bumps.session_nonces;
        nonces.duel_nonce = nonces
            .duel_nonce
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;
        emit!(SessionOverridden {
            player: ctx.accounts.player.key(),
            mode: "duel".to_string(),
            new_nonce: nonces.duel_nonce,
        });
        Ok(())
    }

    /// Override a stuck gauntlet session by incrementing the gauntlet nonce.
    pub fn override_gauntlet_session(ctx: Context<OverrideSession>) -> Result<()> {
        let nonces = &mut ctx.accounts.session_nonces;
        nonces.bump = ctx.bumps.session_nonces;
        nonces.gauntlet_nonce = nonces
            .gauntlet_nonce
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;
        emit!(SessionOverridden {
            player: ctx.accounts.player.key(),
            mode: "gauntlet".to_string(),
            new_nonce: nonces.gauntlet_nonce,
        });
        Ok(())
    }

    /// Rotates the session key on a game session and its child accounts.
    /// Requires the player wallet AND the new session signer to both sign.
    /// Used when the original session key is lost but the wallet is available.
    pub fn rotate_session_key(ctx: Context<RotateSessionKey>) -> Result<()> {
        let old_signer = ctx.accounts.game_session.session_signer;
        let new_signer = ctx.accounts.new_session_signer.key();

        // 1. Update GameSession
        ctx.accounts.game_session.session_signer = new_signer;

        // 2. CPI to gameplay-state to update GameState.session_signer
        let authority_bump = ctx.bumps.session_manager_authority;
        let signer_seeds: &[&[&[u8]]] = &[&[SESSION_MANAGER_AUTHORITY_SEED, &[authority_bump]]];

        gameplay_state::cpi::rotate_game_state_session_key(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.key(),
                gameplay_state::cpi::accounts::RotateGameStateSessionKey {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    session_manager_authority: ctx
                        .accounts
                        .session_manager_authority
                        .to_account_info(),
                },
                signer_seeds,
            ),
            new_signer,
        )?;

        // 3. CPI to player-inventory to update PlayerInventory.player
        player_inventory::cpi::rotate_inventory_owner(
            CpiContext::new_with_signer(
                ctx.accounts.player_inventory_program.key(),
                player_inventory::cpi::accounts::RotateInventoryOwner {
                    inventory: ctx.accounts.inventory.to_account_info(),
                    session_manager_authority: ctx
                        .accounts
                        .session_manager_authority
                        .to_account_info(),
                },
                signer_seeds,
            ),
            new_signer,
        )?;

        emit!(SessionKeyRotated {
            player: ctx.accounts.player.key(),
            session_id: ctx.accounts.game_session.session_id,
            old_session_signer: old_signer,
            new_session_signer: new_signer,
        });
        Ok(())
    }

    /// Processes undelegation (replaces #[ephemeral] macro output).
    pub fn process_undelegation(ctx: Context<InitializeAfterUndelegation>, account_seeds: Vec<Vec<u8>>) -> Result<()> {
        er_compat::undelegate_account(
            &ctx.accounts.base_account,
            &crate::id(),
            &ctx.accounts.buffer,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            account_seeds,
        )
    }
}

/// Context for undelegation processing (replaces #[ephemeral] macro output).
#[derive(Accounts)]
pub struct InitializeAfterUndelegation<'info> {
    /// CHECK: Account being undelegated
    #[account(mut)]
    pub base_account: UncheckedAccount<'info>,
    /// CHECK: Delegation buffer
    pub buffer: UncheckedAccount<'info>,
    /// CHECK: Payer
    #[account(mut)]
    pub payer: UncheckedAccount<'info>,
    /// CHECK: System program
    pub system_program: UncheckedAccount<'info>,
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

/// PlayerProfile account decoded from the Quasar zero-copy player-profile bytes.
#[derive(Clone)]
pub struct PlayerProfile {
    pub owner: Pubkey,
    pub total_runs: u32,
    pub highest_level_unlocked: u8,
    pub available_runs: u32,
    pub created_at: i64,
    pub bump: u8,
    pub unlocked_items: [u8; 10],
    pub active_item_pool: [u8; 10],
    pub equipped_skin: Option<Pubkey>,
    pub gauntlet_boosters: u8,
    pub name_len: u8,
    pub name: [u8; 32],
}

impl PlayerProfile {
    pub const DISCRIMINATOR: [u8; 8] = [82, 226, 99, 87, 164, 130, 181, 80];
    pub const LEN: usize = 145;

    fn decode(buf: &[u8]) -> anchor_lang::Result<Self> {
        require!(
            buf.len() >= Self::LEN,
            anchor_lang::error::ErrorCode::AccountDidNotDeserialize
        );
        require!(
            &buf[..8] == Self::DISCRIMINATOR.as_slice(),
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
        );

        let name_len = buf[112];
        require!(
            name_len <= 32,
            anchor_lang::error::ErrorCode::AccountDidNotDeserialize
        );

        let mut unlocked_items = [0u8; 10];
        unlocked_items.copy_from_slice(&buf[58..68]);

        let mut active_item_pool = [0u8; 10];
        active_item_pool.copy_from_slice(&buf[68..78]);

        let equipped_skin = match buf[78] {
            0 => None,
            1 => Some(read_quasar_pubkey(buf, 79)?),
            _ => return Err(anchor_lang::error::ErrorCode::AccountDidNotDeserialize.into()),
        };

        let mut name = [0u8; 32];
        name.copy_from_slice(&buf[113..145]);

        Ok(Self {
            owner: read_quasar_pubkey(buf, 8)?,
            total_runs: read_quasar_u32(buf, 40)?,
            highest_level_unlocked: buf[44],
            available_runs: read_quasar_u32(buf, 45)?,
            created_at: read_quasar_i64(buf, 49)?,
            bump: buf[57],
            unlocked_items,
            active_item_pool,
            equipped_skin,
            gauntlet_boosters: buf[111],
            name_len,
            name,
        })
    }
}

impl anchor_lang::AccountDeserialize for PlayerProfile {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Self::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Self::decode(*buf)
    }
}

impl anchor_lang::AccountSerialize for PlayerProfile {}

impl anchor_lang::Owner for PlayerProfile {
    fn owner() -> Pubkey {
        PLAYER_PROFILE_PROGRAM_ID
    }
}

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for PlayerProfile {}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct PlayerRelicEntry {
    pub item_id: [u8; 8],
    pub owned_count: u16,
    pub in_active_pool: bool,
}

const MAX_PLAYER_RELICS: usize = 32;
const PLAYER_RELIC_ENTRY_SIZE: usize = 11;

fn read_quasar_pubkey(buf: &[u8], offset: usize) -> anchor_lang::Result<Pubkey> {
    let end = offset
        .checked_add(32)
        .ok_or(anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
    require!(
        buf.len() >= end,
        anchor_lang::error::ErrorCode::AccountDidNotDeserialize
    );
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&buf[offset..end]);
    Ok(Pubkey::new_from_array(bytes))
}

fn read_quasar_u32(buf: &[u8], offset: usize) -> anchor_lang::Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or(anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
    require!(
        buf.len() >= end,
        anchor_lang::error::ErrorCode::AccountDidNotDeserialize
    );
    Ok(u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]))
}

fn read_quasar_i64(buf: &[u8], offset: usize) -> anchor_lang::Result<i64> {
    let end = offset
        .checked_add(8)
        .ok_or(anchor_lang::error::ErrorCode::AccountDidNotDeserialize)?;
    require!(
        buf.len() >= end,
        anchor_lang::error::ErrorCode::AccountDidNotDeserialize
    );
    Ok(i64::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
        buf[offset + 4],
        buf[offset + 5],
        buf[offset + 6],
        buf[offset + 7],
    ]))
}

#[derive(Clone)]
pub struct PlayerRelicPoolRef;

impl anchor_lang::Id for PlayerRelicPoolRef {
    fn id() -> Pubkey {
        PLAYER_PROFILE_PROGRAM_ID
    }
}

#[derive(Clone)]
pub struct PlayerRelicPool {
    pub owner: Pubkey,
    pub count: u8,
    pub bump: u8,
    pub relics: [PlayerRelicEntry; MAX_PLAYER_RELICS],
}

impl PlayerRelicPool {
    pub const DISCRIMINATOR: [u8; 8] = [1, 105, 67, 203, 111, 254, 159, 128];
    pub const LEN: usize = 42 + (MAX_PLAYER_RELICS * PLAYER_RELIC_ENTRY_SIZE);

    fn decode(buf: &[u8]) -> anchor_lang::Result<Self> {
        require!(
            buf.len() >= Self::LEN,
            anchor_lang::error::ErrorCode::AccountDidNotDeserialize
        );
        require!(
            &buf[..8] == Self::DISCRIMINATOR.as_slice(),
            anchor_lang::error::ErrorCode::AccountDiscriminatorMismatch
        );

        let count = buf[40].min(MAX_PLAYER_RELICS as u8);
        let mut relics = [PlayerRelicEntry::default(); MAX_PLAYER_RELICS];
        let mut index = 0usize;
        while index < MAX_PLAYER_RELICS {
            let offset = 42 + (index * PLAYER_RELIC_ENTRY_SIZE);
            let mut item_id = [0u8; 8];
            item_id.copy_from_slice(&buf[offset..offset + 8]);
            relics[index] = PlayerRelicEntry {
                item_id,
                owned_count: u16::from_le_bytes([buf[offset + 8], buf[offset + 9]]),
                in_active_pool: buf[offset + 10] != 0,
            };
            index += 1;
        }

        Ok(Self {
            owner: read_quasar_pubkey(buf, 8)?,
            count,
            bump: buf[41],
            relics,
        })
    }
}

impl anchor_lang::AccountDeserialize for PlayerRelicPool {
    fn try_deserialize(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Self::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
        Self::decode(*buf)
    }
}

impl anchor_lang::AccountSerialize for PlayerRelicPool {}

impl anchor_lang::Owner for PlayerRelicPool {
    fn owner() -> Pubkey {
        PLAYER_PROFILE_PROGRAM_ID
    }
}

#[cfg(feature = "idl-build")]
impl anchor_lang::IdlBuild for PlayerRelicPool {}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
struct RelicAssetAccount {
    pub asset: Pubkey,
    pub item_id: [u8; 8],
    pub bump: u8,
}

impl RelicAssetAccount {
    const DISCRIMINATOR: [u8; 8] = [222, 160, 151, 133, 226, 88, 154, 91];
}

impl anchor_lang::AccountDeserialize for RelicAssetAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> anchor_lang::Result<Self> {
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

fn read_metaplex_asset_owner(asset_info: &AccountInfo<'_>) -> Result<Pubkey> {
    require!(
        *asset_info.owner == MPL_CORE_PROGRAM_ID,
        SessionManagerError::InvalidRelicOwnershipProofs
    );
    let data = asset_info.try_borrow_data()?;
    require!(
        data.len() >= 33 && data[0] == 1,
        SessionManagerError::InvalidRelicOwnershipProofs
    );
    let mut owner_bytes = [0u8; 32];
    owner_bytes.copy_from_slice(&data[1..33]);
    Ok(Pubkey::new_from_array(owner_bytes))
}

fn collect_owned_relic_item_ids(
    owner: Pubkey,
    remaining_accounts: &[AccountInfo<'_>],
) -> Result<Vec<[u8; 8]>> {
    require!(
        remaining_accounts.len().is_multiple_of(2),
        SessionManagerError::InvalidRelicOwnershipProofs
    );

    let mut owned_item_ids = Vec::with_capacity(remaining_accounts.len() / 2);
    for proof_accounts in remaining_accounts.chunks_exact(2) {
        let asset_info = &proof_accounts[0];
        let relic_asset_info = &proof_accounts[1];

        require!(
            read_metaplex_asset_owner(asset_info)? == owner,
            SessionManagerError::InvalidRelicOwnershipProofs
        );
        require!(
            *relic_asset_info.owner == NFT_MARKETPLACE_PROGRAM_ID,
            SessionManagerError::InvalidRelicOwnershipProofs
        );

        let expected_relic_asset = Pubkey::find_program_address(
            &[b"relic_asset", asset_info.key.as_ref()],
            &NFT_MARKETPLACE_PROGRAM_ID,
        )
        .0;
        require!(
            expected_relic_asset == *relic_asset_info.key,
            SessionManagerError::InvalidRelicOwnershipProofs
        );

        let mut relic_asset_data: &[u8] = &relic_asset_info.try_borrow_data()?;
        let relic_asset = RelicAssetAccount::try_deserialize_unchecked(&mut relic_asset_data)
            .map_err(|_| SessionManagerError::InvalidRelicOwnershipProofs)?;
        require!(
            relic_asset.asset == *asset_info.key,
            SessionManagerError::InvalidRelicOwnershipProofs
        );

        owned_item_ids.push(relic_asset.item_id);
    }

    Ok(owned_item_ids)
}

fn session_relic_snapshot(
    pool: Option<&Account<'_, PlayerRelicPool>>,
) -> (u8, [SessionRelicEntry; MAX_SESSION_RELICS]) {
    let mut relics = [SessionRelicEntry::default(); MAX_SESSION_RELICS];
    let mut count = 0usize;

    if let Some(pool) = pool {
        for entry in pool
            .relics
            .iter()
            .take(pool.count as usize)
            .filter(|entry| entry.in_active_pool)
        {
            if count >= MAX_SESSION_RELICS {
                break;
            }
            relics[count] = SessionRelicEntry {
                asset: Pubkey::default(),
                item_id: entry.item_id,
            };
            count += 1;
        }
    }

    (count as u8, relics)
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct StartSession<'info> {
    #[account(
        init_if_needed,
        payer = session_signer,
        space = 8 + SessionNonces::INIT_SPACE,
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump
    )]
    pub session_nonces: Box<Account<'info, SessionNonces>>,

    #[account(
        init,
        payer = session_signer,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref(), &[campaign_level], &session_nonces.campaign_nonce.to_le_bytes()],
        bump
    )]
    pub game_session: Box<Account<'info, GameSession>>,

    #[account(
        mut,
        seeds = [SessionCounter::SEED_PREFIX],
        bump = session_counter.bump
    )]
    pub session_counter: Box<Account<'info, SessionCounter>>,

    /// Player profile for validation and run consumption (from player-profile program)
    #[account(
        mut,
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,

    #[account(
        mut,
        seeds = [b"player_relics", player.key().as_ref()],
        bump,
        seeds::program = PlayerRelicPoolRef::id()
    )]
    pub player_relic_pool: Option<Box<Account<'info, PlayerRelicPool>>>,

    #[account(mut)]
    pub player: Signer<'info>,

    /// Session key signer — pays for all session account rents (funded by wallet in same tx).
    /// Also set as the inventory owner for gameplay transactions.
    #[account(mut)]
    pub session_signer: Signer<'info>,

    #[account(mut)]
    /// CHECK: PDA created by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    /// CHECK: PDA created by map-generator CPI. Optional to keep combined TX under size limit.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

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
        init_if_needed,
        payer = session_signer,
        space = 8 + SessionNonces::INIT_SPACE,
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump
    )]
    pub session_nonces: Box<Account<'info, SessionNonces>>,

    #[account(
        init,
        payer = session_signer,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GameSession::DUEL_SEED_PREFIX, player.key().as_ref(), &session_nonces.duel_nonce.to_le_bytes()],
        bump
    )]
    pub game_session: Box<Account<'info, GameSession>>,

    #[account(
        mut,
        seeds = [SessionCounter::SEED_PREFIX],
        bump = session_counter.bump
    )]
    pub session_counter: Box<Account<'info, SessionCounter>>,

    #[account(
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,

    #[account(
        mut,
        seeds = [b"player_relics", player.key().as_ref()],
        bump,
        seeds::program = PlayerRelicPoolRef::id()
    )]
    pub player_relic_pool: Option<Box<Account<'info, PlayerRelicPool>>>,

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

    #[account(mut)]
    /// CHECK: PDA created by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    /// CHECK: PDA created by map-generator CPI. Optional to keep combined TX under size limit.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by poi-system CPI
    pub map_pois: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    /// Optional MapVrfState for VRF-derived map/POI seeds.
    /// CHECK: Validated via PDA derivation and manual deserialization in handler.
    #[account(mut)]
    pub map_vrf_state: Option<UncheckedAccount<'info>>,

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
pub struct StartGauntletSession<'info> {
    #[account(
        init_if_needed,
        payer = session_signer,
        space = 8 + SessionNonces::INIT_SPACE,
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump
    )]
    pub session_nonces: Box<Account<'info, SessionNonces>>,

    #[account(
        init,
        payer = session_signer,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GameSession::GAUNTLET_SEED_PREFIX, player.key().as_ref(), &session_nonces.gauntlet_nonce.to_le_bytes()],
        bump
    )]
    pub game_session: Box<Account<'info, GameSession>>,

    #[account(
        mut,
        seeds = [SessionCounter::SEED_PREFIX],
        bump = session_counter.bump
    )]
    pub session_counter: Box<Account<'info, SessionCounter>>,

    #[account(
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,

    #[account(
        mut,
        seeds = [b"player_relics", player.key().as_ref()],
        bump,
        seeds::program = PlayerRelicPoolRef::id()
    )]
    pub player_relic_pool: Option<Box<Account<'info, PlayerRelicPool>>>,

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

    #[account(mut)]
    /// CHECK: PDA created by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    /// CHECK: PDA created by map-generator CPI. Optional to keep combined TX under size limit.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,

    #[account(mut)]
    /// CHECK: Initialized by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by poi-system CPI
    pub map_pois: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Initialized by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    /// Optional MapVrfState for VRF-derived map/POI seeds.
    /// CHECK: Validated via PDA derivation and manual deserialization in handler.
    #[account(mut)]
    pub map_vrf_state: Option<UncheckedAccount<'info>>,

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
#[instruction(campaign_level: u8)]
pub struct DelegateSession<'info> {
    #[account(mut)]
    /// CHECK: Validated in handler (owner/player/level/delegation status and PDA seeds).
    pub game_session: UncheckedAccount<'info>,

    /// CHECK: Must match game_session.player, but does not need to sign.
    pub player: UncheckedAccount<'info>,
    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump = session_nonces.bump
    )]
    pub session_nonces: Account<'info, SessionNonces>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, game_session.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_game_session: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, game_session.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_game_session: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, game_session.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_game_session: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateGameState<'info> {
    #[account(mut)]
    /// CHECK: Validated in handler as gameplay-state PDA for the delegated session.
    pub game_state: UncheckedAccount<'info>,

    pub player: Signer<'info>,

    #[account(
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump = session_nonces.bump
    )]
    pub session_nonces: Account<'info, SessionNonces>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, game_state.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_game_state: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, game_state.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_game_state: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, game_state.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_game_state: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateGeneratedMap<'info> {
    #[account(mut)]
    /// CHECK: Validated in handler as generated-map PDA for the delegated session.
    pub generated_map: UncheckedAccount<'info>,

    pub player: Signer<'info>,

    #[account(
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump = session_nonces.bump
    )]
    pub session_nonces: Account<'info, SessionNonces>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, generated_map.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_generated_map: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, generated_map.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_generated_map: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, generated_map.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_generated_map: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateInventory<'info> {
    #[account(mut)]
    /// CHECK: Validated in handler as inventory PDA for the delegated session.
    pub inventory: UncheckedAccount<'info>,

    pub player: Signer<'info>,

    #[account(
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump = session_nonces.bump
    )]
    pub session_nonces: Account<'info, SessionNonces>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, inventory.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_inventory: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, inventory.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_inventory: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, inventory.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_inventory: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct DelegateMapPois<'info> {
    #[account(mut)]
    /// CHECK: Validated in handler as map-pois PDA for the delegated session.
    pub map_pois: UncheckedAccount<'info>,

    pub player: Signer<'info>,

    #[account(
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump = session_nonces.bump
    )]
    pub session_nonces: Account<'info, SessionNonces>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, map_pois.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_map_pois: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, map_pois.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_map_pois: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, map_pois.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_map_pois: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DelegatePoiVrfState<'info> {
    #[account(mut)]
    /// CHECK: Validated in handler as poi-vrf PDA for the delegated session.
    pub poi_vrf_state: UncheckedAccount<'info>,

    pub player: Signer<'info>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, poi_vrf_state.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_poi_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, poi_vrf_state.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_poi_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, poi_vrf_state.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_poi_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

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
    /// CHECK: Validated in handler as generated-map PDA for the delegated session.
    pub generated_map: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated in handler as inventory PDA for the delegated session.
    pub inventory: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated in handler as map-pois PDA for the delegated session.
    pub map_pois: UncheckedAccount<'info>,

    /// CHECK: Optional PoiVrfState from poi-system. Only present for PvP sessions using VRF.
    pub poi_vrf_state: Option<UncheckedAccount<'info>>,

    pub player: Signer<'info>,
    /// CHECK: Magic program
    #[account(address = er_compat::MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
    /// CHECK: Magic context
    #[account(mut, address = er_compat::MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct UndelegateSession<'info> {
    #[account(mut)]
    /// CHECK: Deserialized and validated in handler to support delegated owner (DELeGG).
    pub game_session: UncheckedAccount<'info>,

    /// CHECK: Must match game_session.player, but does not need to sign.
    pub player: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
    /// CHECK: Magic program
    #[account(address = er_compat::MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
    /// CHECK: Magic context
    #[account(mut, address = er_compat::MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,
}

fn validate_gameplay_runtime_accounts(
    game_session_key: &Pubkey,
    game_state: &AccountInfo<'_>,
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

fn derive_campaign_session_pda(player: &Pubkey, campaign_level: u8, nonce: u64) -> Pubkey {
    let campaign_seed = [campaign_level];
    let nonce_bytes = nonce.to_le_bytes();
    let seeds: &[&[u8]] = &[
        GameSession::SEED_PREFIX,
        player.as_ref(),
        &campaign_seed,
        &nonce_bytes,
    ];
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
/// Closes all session-related accounts: session, game_state, generated_map, map_pois, inventory.
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct EndSession<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
        close = session_signer
    )]
    pub game_session: Box<Account<'info, GameSession>>,

    /// Game state account to validate death/completion status (closed via gameplay-state CPI)
    #[account(mut)]
    /// CHECK: Validated by PDA derivation in handler and deserialized via read_game_state_unchecked.
    pub game_state: UncheckedAccount<'info>,

    /// Generated map account (closed via map-generator CPI)
    #[account(mut)]
    /// CHECK: Validated by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    /// SessionDiscovery account (closed via map-generator CPI)
    #[account(mut)]
    /// CHECK: Validated by map-generator CPI
    pub session_discovery: UncheckedAccount<'info>,

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
    pub player_profile: Box<Account<'info, PlayerProfile>>,

    /// Player wallet — validated by has_one constraint. Does NOT need to sign.
    /// CHECK: Validated by has_one constraint on game_session.
    pub player: UncheckedAccount<'info>,

    /// Session key signer — signs to authorize session end, receives all rent refunds.
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

    /// Duel entry for duel-session settlement validation.
    #[account(
        seeds = [gameplay_state::constants::DUEL_ENTRY_SEED, game_session.key().as_ref()],
        bump,
        seeds::program = gameplay_state::ID
    )]
    pub duel_entry: Option<Box<Account<'info, DuelEntry>>>,

    /// Optional MapVrfState (only for PvP sessions using VRF)
    /// CHECK: Validated by map-generator close CPI
    #[account(mut)]
    pub map_vrf_state: Option<UncheckedAccount<'info>>,

    /// Optional PoiVrfState (only for PvP sessions using VRF)
    /// CHECK: Validated by poi-system close CPI
    #[account(mut)]
    pub poi_vrf_state: Option<UncheckedAccount<'info>>,

    /// Optional GameplayVrfState (only for sessions using VRF)
    /// CHECK: Validated by gameplay-state close CPI
    #[account(mut)]
    pub gameplay_vrf_state: Option<UncheckedAccount<'info>>,

    /// Optional GauntletEchoes (only for gauntlet sessions)
    /// CHECK: Validated by gameplay-state close CPI
    #[account(mut)]
    pub gauntlet_echoes: Option<UncheckedAccount<'info>>,

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
    pub player: UncheckedAccount<'info>,

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
        close = session_signer
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
    pub player: UncheckedAccount<'info>,

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

/// Tolerant session close: settles result and closes whichever child accounts are on base layer.
/// Delegated/missing children are skipped. This prevents the soft-lock where close_session_only
/// leaves orphaned child accounts that block start_session.
#[derive(Accounts)]
pub struct ForceCloseSession<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
        close = session_signer
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account — may be delegated; validated by PDA derivation in handler.
    #[account(mut)]
    /// CHECK: Validated by PDA derivation and deserialized in handler.
    pub game_state: UncheckedAccount<'info>,

    /// Generated map account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub generated_map: UncheckedAccount<'info>,

    /// SessionDiscovery account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub session_discovery: UncheckedAccount<'info>,

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
    pub player: UncheckedAccount<'info>,

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

    /// Map POIs account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub map_pois: UncheckedAccount<'info>,

    /// SessionDiscovery account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub session_discovery: UncheckedAccount<'info>,

    /// GeneratedMap account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub generated_map: UncheckedAccount<'info>,

    /// Inventory account — may be delegated.
    #[account(mut)]
    /// CHECK: Owner checked in handler before CPI.
    pub inventory: UncheckedAccount<'info>,

    /// Player wallet receives rent refunds.
    /// CHECK: Validated by child program CPIs via game_state.player.
    #[account(mut)]
    pub player: UncheckedAccount<'info>,

    /// Session key signer — validated by child program CPIs via game_state.session_signer.
    pub session_signer: Signer<'info>,

    pub gameplay_state_program: Program<'info, GameplayState>,

    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    /// CHECK: POI system program for CPI, validated by address constraint
    pub poi_system_program: UncheckedAccount<'info>,

    #[account(address = MAP_GENERATOR_PROGRAM_ID)]
    /// CHECK: Map generator program for CPI, validated by address constraint
    pub map_generator_program: UncheckedAccount<'info>,

    /// CHECK: Player inventory program for CPI, validated by address constraint
    pub player_inventory_program: Program<'info, PlayerInventory>,
}

/// Abandon session at any time (user-initiated).
/// Requires both main wallet and session key signer signatures.
/// Main wallet authorizes the abandonment, session key signer is needed to close sub-accounts.
/// Closes all session-related accounts: session, game_state, generated_map, map_pois, inventory.
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct AbandonSession<'info> {
    #[account(
        mut,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = session_signer @ SessionManagerError::Unauthorized,
        close = session_signer
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account (closed via gameplay-state CPI)
    #[account(mut)]
    /// CHECK: Validated by gameplay-state CPI
    pub game_state: UncheckedAccount<'info>,

    /// Generated map account (closed via map-generator CPI)
    #[account(mut)]
    /// CHECK: Validated by map-generator CPI
    pub generated_map: UncheckedAccount<'info>,

    /// SessionDiscovery account (closed via map-generator CPI)
    #[account(mut)]
    /// CHECK: Validated by map-generator CPI
    pub session_discovery: UncheckedAccount<'info>,

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

    /// Optional MapVrfState (only for PvP sessions using VRF)
    /// CHECK: Validated by map-generator close CPI
    #[account(mut)]
    pub map_vrf_state: Option<UncheckedAccount<'info>>,

    /// Optional PoiVrfState (only for PvP sessions using VRF)
    /// CHECK: Validated by poi-system close CPI
    #[account(mut)]
    pub poi_vrf_state: Option<UncheckedAccount<'info>>,

    /// Optional GameplayVrfState (only for sessions using VRF)
    /// CHECK: Validated by gameplay-state close CPI
    #[account(mut)]
    pub gameplay_vrf_state: Option<UncheckedAccount<'info>>,

    /// Optional GauntletEchoes (only for gauntlet sessions)
    /// CHECK: Validated by gameplay-state close CPI
    #[account(mut)]
    pub gauntlet_echoes: Option<UncheckedAccount<'info>>,

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
// Override & Rotation Contexts
// ============================================================================

#[derive(Accounts)]
pub struct OverrideSession<'info> {
    #[account(
        init_if_needed,
        payer = player,
        space = 8 + SessionNonces::INIT_SPACE,
        seeds = [SessionNonces::SEED_PREFIX, player.key().as_ref()],
        bump
    )]
    pub session_nonces: Account<'info, SessionNonces>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RotateSessionKey<'info> {
    #[account(mut, has_one = player @ SessionManagerError::Unauthorized)]
    pub game_session: Account<'info, GameSession>,

    #[account(mut)]
    /// CHECK: Validated by PDA derivation in handler
    pub game_state: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Validated by PDA derivation in handler
    pub inventory: UncheckedAccount<'info>,

    #[account(mut)]
    pub player: Signer<'info>,

    /// New session key — must sign to prove possession of private key
    pub new_session_signer: Signer<'info>,

    #[account(seeds = [SESSION_MANAGER_AUTHORITY_SEED], bump)]
    /// CHECK: PDA signer for CPI
    pub session_manager_authority: UncheckedAccount<'info>,

    pub gameplay_state_program: Program<'info, GameplayState>,
    pub player_inventory_program: Program<'info, PlayerInventory>,
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

#[event]
pub struct SessionOverridden {
    pub player: Pubkey,
    pub mode: String,
    pub new_nonce: u64,
}

#[event]
pub struct SessionKeyRotated {
    pub player: Pubkey,
    pub session_id: u64,
    pub old_session_signer: Pubkey,
    pub new_session_signer: Pubkey,
}

/// The discriminator for end_session instruction.
/// This is exported so other programs can validate their manual CPI discriminators.
/// Computed as sha256("global:end_session")[..8].
///
/// IMPORTANT: If you rename the `end_session` instruction, you must:
/// 1. Update this constant
/// 2. Update gameplay-state's END_SESSION_DISCRIMINATOR constant
pub const END_SESSION_DISCRIMINATOR: [u8; 8] = [0x0b, 0xf4, 0x3d, 0x9a, 0xd4, 0xf9, 0x0f, 0x42];

/// Try to extract VRF randomness from gameplay_vrf_state or map_vrf_state (in that order).
/// Returns 32 bytes of randomness if either VRF account is available and fulfilled,
/// otherwise returns 32 zero bytes.
fn extract_unlock_randomness(
    gameplay_vrf: &Option<UncheckedAccount>,
    map_vrf: &Option<UncheckedAccount>,
    session_key: &Pubkey,
) -> [u8; 32] {
    // Try gameplay VRF first
    if let Some(ref vrf_account) = gameplay_vrf {
        if let Ok(data) = vrf_account.try_borrow_data() {
            use gameplay_state::state::GameplayVrfState;
            let (expected_pda, _) = Pubkey::find_program_address(
                &[GameplayVrfState::SEED_PREFIX, session_key.as_ref()],
                &gameplay_state::ID,
            );
            if vrf_account.key() == expected_pda
                && vrf_account.owner == &gameplay_state::ID
            {
                let mut data_slice: &[u8] = &data;
                if let Ok(vrf_state) = <GameplayVrfState as anchor_lang::AccountDeserialize>::try_deserialize(&mut data_slice) {
                    if vrf_state.session == *session_key
                        && vrf_state.status == vrf_rng::VrfStatus::Fulfilled
                    {
                        return vrf_state.randomness;
                    }
                }
            }
        }
    }

    // Fall back to map VRF
    if let Some(ref vrf_account) = map_vrf {
        if let Ok(data) = vrf_account.try_borrow_data() {
            use map_generator::state::MapVrfState;
            let (expected_pda, _) = Pubkey::find_program_address(
                &[MapVrfState::SEED_PREFIX, session_key.as_ref()],
                &map_generator::ID,
            );
            if vrf_account.key() == expected_pda
                && vrf_account.owner == &map_generator::ID
            {
                let mut data_slice: &[u8] = &data;
                if let Ok(vrf_state) = <MapVrfState as anchor_lang::AccountDeserialize>::try_deserialize(&mut data_slice) {
                    if vrf_state.session == *session_key
                        && vrf_state.status == vrf_rng::VrfStatus::Fulfilled
                    {
                        return vrf_state.randomness;
                    }
                }
            }
        }
    }

    // No VRF available — return zeroes
    [0u8; 32]
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

    require_keys_eq!(
        *program.key,
        program_id,
        SessionManagerError::Unauthorized
    );

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

    require_keys_eq!(
        *program.key,
        program_id,
        SessionManagerError::Unauthorized
    );

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
        &[(owner, false, true), (player_profile, true, false)],
    )
}

fn sync_relic_ownership_cpi<'info>(
    program: &AccountInfo<'info>,
    player_relic_pool: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    owned_relic_item_ids: Vec<[u8; 8]>,
) -> Result<()> {
    let extra = borsh::to_vec(&owned_relic_item_ids)
        .map_err(|_| anchor_lang::error::ErrorCode::InstructionDidNotSerialize)?;

    invoke_manual_cpi(
        program,
        PLAYER_PROFILE_PROGRAM_ID,
        &SYNC_RELIC_OWNERSHIP_DISCRIMINATOR,
        &extra,
        &[(owner, false, true), (player_relic_pool, true, false)],
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
    unlock_randomness: &[u8; 32],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut extra = [0u8; 34]; // 1 (level) + 1 (victory) + 32 (randomness)
    extra[0] = level_completed;
    extra[1] = if victory { 1 } else { 0 };
    extra[2..34].copy_from_slice(unlock_randomness);
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
pub const CLOSE_GENERATED_MAP_DISCRIMINATOR: [u8; 8] = [249, 208, 241, 231, 57, 214, 174, 103];
pub const CLOSE_SESSION_DISCOVERY_DISCRIMINATOR: [u8; 8] =
    [0x60, 0xf6, 0x1c, 0x60, 0x1a, 0xa9, 0x44, 0x7f];
pub const CLOSE_GAUNTLET_ECHOES_DISCRIMINATOR: [u8; 8] =
    [0x85, 0xa2, 0xe1, 0xee, 0x33, 0x0d, 0xbf, 0xb2];
pub const CLOSE_MAP_POIS_VIA_SESSION_SIGNER_DISCRIMINATOR: [u8; 8] =
    [35, 38, 19, 18, 250, 66, 39, 150];
pub const CLOSE_MAP_POIS_ORPHANED_DISCRIMINATOR: [u8; 8] = [218, 44, 98, 133, 139, 114, 27, 98];
pub const CLOSE_GENERATED_MAP_ORPHANED_DISCRIMINATOR: [u8; 8] =
    [0x7e, 0xd6, 0xdf, 0xfd, 0x9c, 0xb4, 0xb3, 0x0e];
pub const CLOSE_SESSION_DISCOVERY_ORPHANED_DISCRIMINATOR: [u8; 8] =
    [0x0e, 0x15, 0x4e, 0x08, 0x9e, 0x1e, 0x07, 0x54];

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
            (player, false, false),
            (session_signer, true, true),
        ],
    )
}

fn close_gauntlet_echoes_cpi<'info>(
    program: &AccountInfo<'info>,
    gauntlet_echoes: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        gameplay_state::ID,
        &CLOSE_GAUNTLET_ECHOES_DISCRIMINATOR,
        &[],
        &[
            (gauntlet_echoes, true, false),
            (game_state, false, false),
            (player, false, false),
            (session_signer, true, true),
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
            (player, false, false),
            (session_signer, true, true),
        ],
    )
}

fn close_session_discovery_cpi<'info>(
    program: &AccountInfo<'info>,
    session_discovery: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        MAP_GENERATOR_PROGRAM_ID,
        &CLOSE_SESSION_DISCOVERY_DISCRIMINATOR,
        &[],
        &[
            (session_discovery, true, false),
            (session, false, false),
            (player, false, false),
            (session_signer, true, true),
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
            (player, false, false),
            (session_signer, true, true),
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
            (player, false, false),
            (session_signer, true, true),
        ],
    )
}

fn close_generated_map_orphaned_cpi<'info>(
    program: &AccountInfo<'info>,
    generated_map: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        MAP_GENERATOR_PROGRAM_ID,
        &CLOSE_GENERATED_MAP_ORPHANED_DISCRIMINATOR,
        &[],
        &[
            (generated_map, true, false),
            (game_state, false, false),
            (player, true, false),
            (session_signer, false, true),
        ],
    )
}

fn close_session_discovery_orphaned_cpi<'info>(
    program: &AccountInfo<'info>,
    session_discovery: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        MAP_GENERATOR_PROGRAM_ID,
        &CLOSE_SESSION_DISCOVERY_ORPHANED_DISCRIMINATOR,
        &[],
        &[
            (session_discovery, true, false),
            (game_state, false, false),
            (player, true, false),
            (session_signer, false, true),
        ],
    )
}

// ============================================================================
// Close VRF State CPI Functions
// ============================================================================

pub const CLOSE_MAP_VRF_STATE_DISCRIMINATOR: [u8; 8] = [81, 161, 130, 150, 241, 141, 167, 205];
pub const CLOSE_POI_VRF_STATE_DISCRIMINATOR: [u8; 8] = [27, 145, 120, 63, 58, 59, 103, 44];
pub const CLOSE_GAMEPLAY_VRF_STATE_DISCRIMINATOR: [u8; 8] = [88, 29, 10, 60, 204, 65, 96, 59];

fn close_map_vrf_state_cpi<'info>(
    program: &AccountInfo<'info>,
    vrf_state: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        MAP_GENERATOR_PROGRAM_ID,
        &CLOSE_MAP_VRF_STATE_DISCRIMINATOR,
        &[],
        &[
            (vrf_state, true, false),
            (session, false, false),
            (player, false, false),
            (session_signer, true, true),
        ],
    )
}

fn close_poi_vrf_state_cpi<'info>(
    program: &AccountInfo<'info>,
    vrf_state: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        POI_SYSTEM_PROGRAM_ID,
        &CLOSE_POI_VRF_STATE_DISCRIMINATOR,
        &[],
        &[
            (vrf_state, true, false),
            (session, false, false),
            (player, false, false),
            (session_signer, true, true),
        ],
    )
}

fn close_gameplay_vrf_state_cpi<'info>(
    program: &AccountInfo<'info>,
    vrf_state: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        gameplay_state::ID,
        &CLOSE_GAMEPLAY_VRF_STATE_DISCRIMINATOR,
        &[],
        &[
            (vrf_state, true, false),
            (game_state, false, false),
            (player, false, false),
            (session_signer, true, true),
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
    fn test_close_gauntlet_echoes_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_gauntlet_echoes");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_GAUNTLET_ECHOES_DISCRIMINATOR, expected,
            "CLOSE_GAUNTLET_ECHOES_DISCRIMINATOR doesn't match"
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
    fn test_close_session_discovery_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_session_discovery");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_SESSION_DISCOVERY_DISCRIMINATOR, expected,
            "CLOSE_SESSION_DISCOVERY_DISCRIMINATOR doesn't match"
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

    #[test]
    fn test_close_generated_map_orphaned_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_generated_map_orphaned");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_GENERATED_MAP_ORPHANED_DISCRIMINATOR, expected,
            "CLOSE_GENERATED_MAP_ORPHANED_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_session_discovery_orphaned_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_session_discovery_orphaned");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_SESSION_DISCOVERY_ORPHANED_DISCRIMINATOR, expected,
            "CLOSE_SESSION_DISCOVERY_ORPHANED_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_map_vrf_state_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_map_vrf_state");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_MAP_VRF_STATE_DISCRIMINATOR, expected,
            "CLOSE_MAP_VRF_STATE_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_poi_vrf_state_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_poi_vrf_state");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_POI_VRF_STATE_DISCRIMINATOR, expected,
            "CLOSE_POI_VRF_STATE_DISCRIMINATOR doesn't match"
        );
    }

    #[test]
    fn test_close_gameplay_vrf_state_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_gameplay_vrf_state");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_GAMEPLAY_VRF_STATE_DISCRIMINATOR, expected,
            "CLOSE_GAMEPLAY_VRF_STATE_DISCRIMINATOR doesn't match"
        );
    }
}
