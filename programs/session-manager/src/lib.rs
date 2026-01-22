use anchor_lang::prelude::*;
use anchor_lang::system_program;

pub mod constants;
pub mod errors;
pub mod state;

use constants::*;
use errors::SessionManagerError;
use state::{GameSession, SessionCounter};

declare_id!("FcMT7MzBLVQGaMATEMws3fjsL2Q77QSHmoEPdowTMxJa");

/// Player Profile program ID for cross-program account validation
/// Must match the declare_id! in player-profile/src/lib.rs
/// 29DPbP1zuCCRg63PiShMjxAmZos97BR5TmhpijUYQzze
pub const PLAYER_PROFILE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x10, 0xf6, 0x57, 0xa0, 0x04, 0x5a, 0x5f, 0x50, 0x16, 0x53, 0xbe, 0xb6, 0x73, 0x24, 0xd6, 0xab,
    0x76, 0x10, 0x4d, 0xb5, 0x58, 0x07, 0x9f, 0xc8, 0x38, 0xd3, 0x07, 0x21, 0xce, 0x96, 0x44, 0x7b,
]);

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

    /// Starts a new game session for the player at a specific level.
    ///
    /// Validates:
    /// - Player has available runs > 0
    /// - Campaign level is within player's unlocked range (1 to highest_level_unlocked)
    /// - No existing session for this (player, level) pair
    ///
    /// Actions:
    /// - Creates session with snapshot of player's active_item_pool
    /// - Transfers SOL to burner wallet for gameplay transactions
    /// - Emits SessionStarted event
    pub fn start_session(
        ctx: Context<StartSession>,
        campaign_level: u8,
        burner_lamports: u64,
    ) -> Result<()> {
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
        let session = &mut ctx.accounts.game_session;
        let clock = Clock::get()?;

        // Increment counter and get new session ID
        counter.count = counter
            .count
            .checked_add(1)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;

        session.player = ctx.accounts.player.key();
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
        session.burner_wallet = ctx.accounts.burner_wallet.key();

        // Transfer SOL to burner wallet for gameplay fees
        if burner_lamports > 0 {
            system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    system_program::Transfer {
                        from: ctx.accounts.player.to_account_info(),
                        to: ctx.accounts.burner_wallet.to_account_info(),
                    },
                ),
                burner_lamports,
            )?;
        }

        emit!(SessionStarted {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            burner_wallet: session.burner_wallet,
            burner_lamports,
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

        // Account will be closed by Anchor (close = player constraint)
        Ok(())
    }

    /// Forces a session to close.
    /// Can be called by anyone to clean up abandoned sessions.
    pub fn force_close_session(ctx: Context<ForceCloseSession>, _campaign_level: u8) -> Result<()> {
        let session = &ctx.accounts.game_session;
        let clock = Clock::get()?;

        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            victory: false,
            final_state_hash: session.state_hash,
            timestamp: clock.unix_timestamp,
        });

        // Account will be closed by Anchor (close = recipient constraint)
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
    /// CHECK: We manually validate this is the correct PDA
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
}

#[derive(Accounts)]
#[instruction(campaign_level: u8)]
pub struct ForceCloseSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, session_owner.key().as_ref(), &[campaign_level]],
        bump = game_session.bump,
        close = recipient
    )]
    pub game_session: Account<'info, GameSession>,

    /// CHECK: The original session owner (for PDA derivation)
    pub session_owner: AccountInfo<'info>,

    /// CHECK: Account to receive the closed session's rent
    #[account(mut)]
    pub recipient: AccountInfo<'info>,
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
    pub burner_lamports: u64,
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
