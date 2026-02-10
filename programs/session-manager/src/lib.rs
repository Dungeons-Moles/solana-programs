use anchor_lang::prelude::*;
pub mod constants;
pub mod errors;
pub mod state;

use errors::SessionManagerError;
use gameplay_state::program::GameplayState;
use gameplay_state::state::GameState;
use map_generator::program::MapGenerator;
use map_generator::state::{GeneratedMap, MapConfig as MapConfigAccount};
use player_inventory::program::PlayerInventory;
use state::{GameSession, SessionCounter, EMPTY_STATE_HASH};

declare_id!("FcMT7MzBLVQGaMATEMws3fjsL2Q77QSHmoEPdowTMxJa");

/// Player Profile program ID for cross-program account validation
/// Must match the declare_id! in player-profile/src/lib.rs
/// 29DPbP1zuCCRg63PiShMjxAmZos97BR5TmhpijUYQzze
pub const PLAYER_PROFILE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x10, 0xf6, 0x57, 0xa0, 0x04, 0x5a, 0x5f, 0x50, 0x16, 0x53, 0xbe, 0xb6, 0x73, 0x24, 0xd6, 0xab,
    0x76, 0x10, 0x4d, 0xb5, 0x58, 0x07, 0x9f, 0xc8, 0x38, 0xd3, 0x07, 0x21, 0xce, 0x96, 0x44, 0x7b,
]);

/// POI System program ID for manual CPI.
/// Must match the declare_id! in poi-system/src/lib.rs
/// 6E27r1Cyo2CNPvtRsonn3uHUAdznS3cMXEBX4HRbfBQY
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x4d, 0xa0, 0x3f, 0xd1, 0xb6, 0x38, 0x95, 0xb5, 0x02, 0xc3, 0xad, 0x5f, 0x41, 0x88, 0x58, 0x7a,
    0xeb, 0xa6, 0xeb, 0xd8, 0xf1, 0x6b, 0x02, 0x23, 0xb9, 0x0e, 0xb1, 0x15, 0x96, 0x67, 0xd7, 0x4d,
]);

/// Map Generator program ID for CPI.
/// Must match the declare_id! in map-generator/src/lib.rs
/// BYdGuEGf8NqtLnHpSRuZFrPGEgvdxMfGfTt71QVBxYHa
pub const MAP_GENERATOR_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    156, 174, 227, 192, 77, 77, 237, 57, 57, 229, 227, 42, 100, 51, 52, 5, 241, 68, 44, 141, 222,
    59, 35, 223, 249, 8, 30, 121, 140, 38, 69, 149,
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
        let burner_wallet_key = ctx.accounts.burner_wallet.key();

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
            // Store burner wallet pubkey
            session.burner_wallet = burner_wallet_key;
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
                    burner_wallet: ctx.accounts.burner_wallet.to_account_info(),
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
        // IMPORTANT: Use burner_wallet as the inventory owner since all gameplay
        // transactions (equip, fuse, etc.) are signed by the burner wallet.
        player_inventory::cpi::initialize_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::InitializeInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                session: ctx.accounts.game_session.to_account_info(),
                player: ctx.accounts.burner_wallet.to_account_info(),
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
            &ctx.accounts.burner_wallet.to_account_info(),
            6,
        )?;

        emit!(SessionStarted {
            player: session_player,
            session_id: counter.count,
            campaign_level,
            burner_wallet: burner_wallet_key,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Delegates the session to the MagicBlock ephemeral rollup.
    /// NOTE: MagicBlock SDK is blocked due to Rust edition 2024 requirement.
    /// This is a stub that just sets the is_delegated flag.
    pub fn delegate_session(ctx: Context<DelegateSession>, _campaign_level: u8) -> Result<()> {
        let session = &mut ctx.accounts.game_session;
        let clock = Clock::get()?;

        require!(
            !session.is_delegated,
            SessionManagerError::SessionAlreadyDelegated
        );

        // In production: Call ephemeral_rollups_sdk::cpi::delegate_account
        // For now, just mark as delegated
        session.is_delegated = true;
        session.last_activity = clock.unix_timestamp;

        emit!(SessionDelegated {
            player: session.player,
            session_id: session.session_id,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Commits the current game state from the ephemeral rollup.
    /// Updates the state hash and last activity timestamp.
    pub fn commit_session(
        ctx: Context<CommitSession>,
        _campaign_level: u8,
        state_hash: [u8; 32],
    ) -> Result<()> {
        let session = &mut ctx.accounts.game_session;
        let clock = Clock::get()?;

        require!(
            session.is_delegated,
            SessionManagerError::SessionNotDelegated
        );

        session.state_hash = state_hash;
        session.last_activity = clock.unix_timestamp;

        // In production: Call ephemeral_rollups_sdk::cpi::commit_accounts

        Ok(())
    }

    /// Ends the session after death or level completion.
    /// Only callable by burner wallet when player is dead OR has completed the level.
    /// Also closes the player's inventory via CPI to ensure fresh inventory for next session.
    ///
    /// This is designed to be called automatically by the frontend after combat,
    /// signed only by the burner wallet (no user interaction required).
    pub fn end_session(ctx: Context<EndSession>, _campaign_level: u8) -> Result<()> {
        let session = &ctx.accounts.game_session;
        let game_state = &ctx.accounts.game_state;
        let clock = Clock::get()?;

        // Validate: session can only be ended if player is dead OR completed the level
        require!(
            game_state.is_dead || game_state.completed,
            SessionManagerError::SessionNotEndable
        );

        // Determine victory from game state (completed = true means victory)
        let victory = game_state.completed && !game_state.is_dead;

        // Record run result via CPI to player-profile
        // This updates total_runs, and on first-time victory unlocks next level + random item
        record_run_result_cpi(
            &ctx.accounts.player_profile_program,
            &ctx.accounts.player_profile.to_account_info(),
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.burner_wallet.to_account_info(),
            session.campaign_level,
            victory,
        )?;

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
        close_map_pois_via_burner_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 2. Close generated_map (depends on session)
        close_generated_map_cpi(
            &ctx.accounts.map_generator_program,
            &ctx.accounts.generated_map,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 3. Close map_enemies (depends on game_state)
        close_map_enemies_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.map_enemies,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 4. Close game_state
        close_game_state_via_burner_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 5. Close inventory via CPI to ensure fresh inventory for next session
        // Use burner_wallet since it's the inventory owner (set during start_session)
        player_inventory::cpi::close_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::CloseInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                player: ctx.accounts.burner_wallet.to_account_info(),
            },
        ))?;

        // 6. Session account will be closed by Anchor (close = player constraint)
        Ok(())
    }

    /// Abandons a session at any time (user-initiated).
    /// Requires the main wallet signature.
    /// Used when player wants to quit a session early.
    /// Closes all session-related accounts to allow starting a new session on the same level.
    pub fn abandon_session(ctx: Context<AbandonSession>, _campaign_level: u8) -> Result<()> {
        let session = &ctx.accounts.game_session;
        let clock = Clock::get()?;

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
        close_map_pois_via_burner_cpi(
            &ctx.accounts.poi_system_program,
            &ctx.accounts.map_pois,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 2. Close generated_map (depends on session)
        close_generated_map_cpi(
            &ctx.accounts.map_generator_program,
            &ctx.accounts.generated_map,
            &ctx.accounts.game_session.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 3. Close map_enemies (depends on game_state)
        close_map_enemies_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.map_enemies,
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 4. Close game_state
        close_game_state_via_burner_cpi(
            &ctx.accounts.gameplay_state_program.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.player,
            &ctx.accounts.burner_wallet.to_account_info(),
        )?;

        // 5. Close inventory via CPI
        player_inventory::cpi::close_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::CloseInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                player: ctx.accounts.burner_wallet.to_account_info(),
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

    /// Burner wallet that will own all session-specific accounts (inventory, etc.)
    /// Must be a signer so it can be set as the inventory owner for gameplay transactions.
    #[account(mut)]
    pub burner_wallet: Signer<'info>,

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
#[instruction(campaign_level: u8)]
pub struct DelegateSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref(), &[campaign_level]],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized
    )]
    pub game_session: Account<'info, GameSession>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct CommitSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref(), &[campaign_level]],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized
    )]
    pub game_session: Account<'info, GameSession>,

    pub player: Signer<'info>,
}

/// End session after death or level completion.
/// Only burner wallet needs to sign - player just receives rent refund.
/// Closes all session-related accounts: session, game_state, generated_map, map_enemies, map_pois, inventory.
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct EndSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref(), &[campaign_level]],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = burner_wallet @ SessionManagerError::Unauthorized,
        close = player
    )]
    pub game_session: Account<'info, GameSession>,

    /// Game state account to validate death/completion status (closed via gameplay-state CPI)
    #[account(
        mut,
        seeds = [b"game_state", game_session.key().as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

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

    /// Player wallet - receives rent refund but does NOT need to sign
    /// CHECK: Validated by has_one constraint on game_session
    #[account(mut)]
    pub player: AccountInfo<'info>,

    /// Burner wallet - must sign to authorize session end and close inventory
    #[account(mut)]
    pub burner_wallet: Signer<'info>,

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

/// Abandon session at any time (user-initiated).
/// Requires both main wallet and burner wallet signatures.
/// Main wallet authorizes the abandonment, burner wallet is needed to close sub-accounts.
/// Closes all session-related accounts: session, game_state, generated_map, map_enemies, map_pois, inventory.
#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct AbandonSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref(), &[campaign_level]],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized,
        has_one = burner_wallet @ SessionManagerError::Unauthorized,
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

    /// Burner wallet - must sign to close sub-accounts (owns the inventory)
    #[account(mut)]
    pub burner_wallet: Signer<'info>,

    /// Player's inventory account (closed via CPI)
    #[account(mut)]
    /// CHECK: Validated by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

    pub player_inventory_program: Program<'info, PlayerInventory>,
    pub gameplay_state_program: Program<'info, GameplayState>,

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
    pub burner_wallet: Pubkey,
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

/// The discriminator for end_session instruction.
/// This is exported so other programs can validate their manual CPI discriminators.
/// Computed as sha256("global:end_session")[..8].
///
/// IMPORTANT: If you rename the `end_session` instruction, you must:
/// 1. Update this constant
/// 2. Update gameplay-state's END_SESSION_DISCRIMINATOR constant
pub const END_SESSION_DISCRIMINATOR: [u8; 8] = [0x0b, 0xf4, 0x3d, 0x9a, 0xd4, 0xf9, 0x0f, 0x42];

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
            (payer, true, true),
            (system_program, false, false),
        ],
    )
}

fn discover_visible_waypoints_cpi<'info>(
    program: &AccountInfo<'info>,
    map_pois: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    burner_wallet: &AccountInfo<'info>,
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
            (burner_wallet, false, true),
        ],
    )
}

pub const RECORD_RUN_RESULT_CPI_DISCRIMINATOR: [u8; 8] =
    [0x09, 0xaf, 0xf6, 0x09, 0x1f, 0x62, 0x79, 0x45];

fn record_run_result_cpi<'info>(
    program: &AccountInfo<'info>,
    player_profile: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    burner_wallet: &AccountInfo<'info>,
    level_completed: u8,
    victory: bool,
) -> Result<()> {
    let extra = [level_completed, if victory { 1 } else { 0 }];
    invoke_manual_cpi(
        program,
        PLAYER_PROFILE_PROGRAM_ID,
        &RECORD_RUN_RESULT_CPI_DISCRIMINATOR,
        &extra,
        &[
            (player_profile, true, false),
            (session, false, false),
            (burner_wallet, false, true),
        ],
    )
}

// ============================================================================
// Close CPI Functions for end_session
// ============================================================================

pub const CLOSE_GAME_STATE_VIA_BURNER_DISCRIMINATOR: [u8; 8] = [71, 137, 243, 70, 95, 193, 114, 51];
pub const CLOSE_MAP_ENEMIES_DISCRIMINATOR: [u8; 8] = [192, 111, 190, 66, 236, 132, 252, 88];
pub const CLOSE_GENERATED_MAP_DISCRIMINATOR: [u8; 8] = [249, 208, 241, 231, 57, 214, 174, 103];
pub const CLOSE_MAP_POIS_VIA_BURNER_DISCRIMINATOR: [u8; 8] = [96, 5, 252, 241, 226, 138, 10, 215];

fn close_game_state_via_burner_cpi<'info>(
    program: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    burner_wallet: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        gameplay_state::ID,
        &CLOSE_GAME_STATE_VIA_BURNER_DISCRIMINATOR,
        &[],
        &[
            (game_state, true, false),
            (player, true, false),
            (burner_wallet, false, true),
        ],
    )
}

fn close_map_enemies_cpi<'info>(
    program: &AccountInfo<'info>,
    map_enemies: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    burner_wallet: &AccountInfo<'info>,
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
            (burner_wallet, false, true),
        ],
    )
}

fn close_generated_map_cpi<'info>(
    program: &AccountInfo<'info>,
    generated_map: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    burner_wallet: &AccountInfo<'info>,
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
            (burner_wallet, false, true),
        ],
    )
}

fn close_map_pois_via_burner_cpi<'info>(
    program: &AccountInfo<'info>,
    map_pois: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    burner_wallet: &AccountInfo<'info>,
) -> Result<()> {
    invoke_manual_cpi(
        program,
        POI_SYSTEM_PROGRAM_ID,
        &CLOSE_MAP_POIS_VIA_BURNER_DISCRIMINATOR,
        &[],
        &[
            (map_pois, true, false),
            (session, false, false),
            (player, true, false),
            (burner_wallet, false, true),
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
    fn test_close_game_state_via_burner_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_game_state_via_burner");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_GAME_STATE_VIA_BURNER_DISCRIMINATOR, expected,
            "CLOSE_GAME_STATE_VIA_BURNER_DISCRIMINATOR doesn't match"
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
    fn test_close_map_pois_via_burner_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:close_map_pois_via_burner");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            CLOSE_MAP_POIS_VIA_BURNER_DISCRIMINATOR, expected,
            "CLOSE_MAP_POIS_VIA_BURNER_DISCRIMINATOR doesn't match"
        );
    }
}
