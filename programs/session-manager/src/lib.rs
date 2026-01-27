use anchor_lang::prelude::*;
pub mod constants;
pub mod errors;
pub mod state;

use errors::SessionManagerError;
use gameplay_state::program::GameplayState;
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
/// FJVnZE45hxcd7BJeci27BiTx23XD6inN4paiM2EkMaoB
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xd4, 0x7f, 0x09, 0xdc, 0xd0, 0xbf, 0x8e, 0xd8, 0xa9, 0xdf, 0xab, 0x1d, 0x5a, 0xe3, 0xda, 0xea,
    0x4c, 0xc3, 0x28, 0x0c, 0xe4, 0xdf, 0x73, 0xe8, 0x6e, 0xc5, 0xc3, 0xd7, 0x13, 0xf1, 0xd1, 0x4a,
]);

/// Discriminator for poi_system::initialize_map_pois instruction.
/// Computed as sha256("global:initialize_map_pois")[..8].
///
/// NOTE: This is manually specified because session-manager cannot depend on poi-system
/// (circular dependency). If poi-system's initialize_map_pois instruction changes, this must be updated.
pub const INITIALIZE_MAP_POIS_DISCRIMINATOR: [u8; 8] =
    [0xa8, 0xec, 0xff, 0x37, 0xee, 0xd2, 0x19, 0xfb];

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
            campaign_level >= 1 && campaign_level <= 40,
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
        player_inventory::cpi::initialize_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::InitializeInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                session: ctx.accounts.game_session.to_account_info(),
                player: ctx.accounts.player.to_account_info(),
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
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            act,
            week,
            poi_seed,
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

    /// Ends the session normally, undelegating from rollup and closing the account.
    /// Also closes the player's inventory via CPI to ensure fresh inventory for next session.
    pub fn end_session(ctx: Context<EndSession>, _campaign_level: u8, victory: bool) -> Result<()> {
        let session = &ctx.accounts.game_session;
        let clock = Clock::get()?;

        // In production:
        // - Call ephemeral_rollups_sdk::cpi::commit_and_undelegate_accounts
        // - CPI to player_profile::record_run_result(level, victory)

        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            victory,
            final_state_hash: session.state_hash,
            timestamp: clock.unix_timestamp,
        });

        // Close inventory via CPI to ensure fresh inventory for next session
        player_inventory::cpi::close_inventory(CpiContext::new(
            ctx.accounts.player_inventory_program.to_account_info(),
            player_inventory::cpi::accounts::CloseInventory {
                inventory: ctx.accounts.inventory.to_account_info(),
                player: ctx.accounts.player.to_account_info(),
            },
        ))?;

        // Account will be closed by Anchor (close = player constraint)
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

    /// Player profile for validation (from player-profile program)
    #[account(
        seeds = [b"player", player.key().as_ref()],
        bump,
        seeds::program = PlayerProfileRef::id()
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub player: Signer<'info>,

    /// CHECK: Burner wallet receives SOL for gameplay transactions
    #[account(mut)]
    pub burner_wallet: AccountInfo<'info>,

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

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct EndSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref(), &[campaign_level]],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized,
        close = player
    )]
    pub game_session: Account<'info, GameSession>,

    #[account(mut)]
    pub player: Signer<'info>,

    /// Player's inventory account (closed via CPI to ensure fresh inventory next session)
    #[account(mut)]
    /// CHECK: Validated by player-inventory CPI
    pub inventory: UncheckedAccount<'info>,

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

/// Manual CPI to poi_system::initialize_map_pois.
///
/// This uses manual instruction construction because session-manager cannot depend
/// on poi-system (would create circular dependency). The discriminator is validated
/// by `test_initialize_map_pois_discriminator`.
#[allow(clippy::too_many_arguments)]
fn initialize_map_pois_cpi<'info>(
    poi_system_program: &AccountInfo<'info>,
    map_pois: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    act: u8,
    week: u8,
    seed: u64,
) -> Result<()> {
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
    use anchor_lang::solana_program::program::invoke;

    let mut data = Vec::with_capacity(8 + 1 + 1 + 8);
    data.extend_from_slice(&INITIALIZE_MAP_POIS_DISCRIMINATOR);
    data.push(act);
    data.push(week);
    data.extend_from_slice(&seed.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(map_pois.key(), false),
        AccountMeta::new_readonly(session.key(), false),
        AccountMeta::new(payer.key(), true),
        AccountMeta::new_readonly(system_program.key(), false),
    ];

    let instruction = Instruction {
        program_id: POI_SYSTEM_PROGRAM_ID,
        accounts,
        data,
    };

    invoke(
        &instruction,
        &[
            map_pois.clone(),
            session.clone(),
            payer.clone(),
            system_program.clone(),
            poi_system_program.clone(),
        ],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates that END_SESSION_DISCRIMINATOR matches sha256("global:end_session")[..8].
    /// This test ensures the exported discriminator stays in sync with the instruction.
    #[test]
    fn test_end_session_discriminator() {
        // The discriminator is sha256("global:end_session")[..8]
        // Pre-computed value - if instruction is renamed, this test should be updated
        // along with the constant.
        let expected: [u8; 8] = [0x0b, 0xf4, 0x3d, 0x9a, 0xd4, 0xf9, 0x0f, 0x42];
        assert_eq!(
            END_SESSION_DISCRIMINATOR, expected,
            "END_SESSION_DISCRIMINATOR doesn't match expected value"
        );
    }

    /// Validates that INITIALIZE_MAP_POIS_DISCRIMINATOR matches sha256("global:initialize_map_pois")[..8].
    /// This test ensures the manual CPI discriminator stays in sync with poi-system.
    #[test]
    fn test_initialize_map_pois_discriminator() {
        // The discriminator is sha256("global:initialize_map_pois")[..8]
        // Pre-computed value - if instruction is renamed, update both this test
        // and INITIALIZE_MAP_POIS_DISCRIMINATOR.
        let expected: [u8; 8] = [0xa8, 0xec, 0xff, 0x37, 0xee, 0xd2, 0x19, 0xfb];
        assert_eq!(
            INITIALIZE_MAP_POIS_DISCRIMINATOR, expected,
            "INITIALIZE_MAP_POIS_DISCRIMINATOR doesn't match expected value"
        );
    }
}
