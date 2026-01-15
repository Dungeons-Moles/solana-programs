use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod state;

use constants::*;
use errors::SessionManagerError;
use state::{GameSession, SessionCounter};

declare_id!("H6Z6herhFbGR8Cnc4hypyMyCTncxbfFArmGsNSkvd2yQ");

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

    /// Starts a new game session for the player.
    /// Validates that the campaign level is within the player's unlocked tier.
    pub fn start_session(ctx: Context<StartSession>, campaign_level: u8) -> Result<()> {
        // Note: In production, we would validate campaign_level against player's unlocked tier
        // by reading the player-profile program. For now, we just validate it's reasonable.
        require!(
            campaign_level < 81, // Max 80 levels (0-80)
            SessionManagerError::InvalidCampaignLevel
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

        emit!(SessionStarted {
            player: session.player,
            session_id: session.session_id,
            campaign_level: session.campaign_level,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Delegates the session to the MagicBlock ephemeral rollup.
    /// NOTE: MagicBlock SDK is blocked due to Rust edition 2024 requirement.
    /// This is a stub that just sets the is_delegated flag.
    pub fn delegate_session(ctx: Context<DelegateSession>) -> Result<()> {
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
    pub fn commit_session(ctx: Context<CommitSession>, state_hash: [u8; 32]) -> Result<()> {
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
    pub fn end_session(ctx: Context<EndSession>) -> Result<()> {
        let session = &ctx.accounts.game_session;
        let clock = Clock::get()?;

        // In production: Call ephemeral_rollups_sdk::cpi::commit_and_undelegate_accounts

        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
            final_state_hash: session.state_hash,
            timestamp: clock.unix_timestamp,
        });

        // Account will be closed by Anchor (close = player constraint)
        Ok(())
    }

    /// Forces a session to close after timeout (1 hour inactivity).
    /// Can be called by anyone to clean up abandoned sessions.
    pub fn force_close_session(ctx: Context<ForceCloseSession>) -> Result<()> {
        let session = &ctx.accounts.game_session;
        let clock = Clock::get()?;

        // Check if session has timed out (1 hour)
        let elapsed = clock
            .unix_timestamp
            .checked_sub(session.last_activity)
            .ok_or(SessionManagerError::ArithmeticOverflow)?;

        require!(
            elapsed >= SESSION_TIMEOUT,
            SessionManagerError::SessionNotTimedOut
        );

        emit!(SessionEnded {
            player: session.player,
            session_id: session.session_id,
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

#[derive(Accounts)]
pub struct StartSession<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + GameSession::INIT_SPACE,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref()],
        bump
    )]
    pub game_session: Account<'info, GameSession>,

    #[account(
        mut,
        seeds = [SessionCounter::SEED_PREFIX],
        bump = session_counter.bump
    )]
    pub session_counter: Account<'info, SessionCounter>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DelegateSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref()],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized
    )]
    pub game_session: Account<'info, GameSession>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct CommitSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref()],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized
    )]
    pub game_session: Account<'info, GameSession>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct EndSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, player.key().as_ref()],
        bump = game_session.bump,
        has_one = player @ SessionManagerError::Unauthorized,
        close = player
    )]
    pub game_session: Account<'info, GameSession>,

    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct ForceCloseSession<'info> {
    #[account(
        mut,
        seeds = [GameSession::SEED_PREFIX, session_owner.key().as_ref()],
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
    pub final_state_hash: [u8; 32],
    pub timestamp: i64,
}
