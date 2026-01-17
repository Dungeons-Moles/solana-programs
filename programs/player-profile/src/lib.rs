use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod state;

use constants::*;
use errors::PlayerProfileError;
use state::PlayerProfile;

declare_id!("29DPbP1zuCCRg63PiShMjxAmZos97BR5TmhpijUYQzze");

#[program]
pub mod player_profile {
    use super::*;

    /// Creates a new player profile for the signer's wallet.
    /// Each wallet can only have one profile.
    pub fn initialize_profile(ctx: Context<InitializeProfile>, name: String) -> Result<()> {
        // Validate name length
        require!(
            name.len() <= MAX_NAME_LENGTH,
            PlayerProfileError::NameTooLong
        );

        let profile = &mut ctx.accounts.player_profile;
        let clock = Clock::get()?;

        profile.owner = ctx.accounts.owner.key();
        profile.name = name;
        profile.total_runs = INITIAL_TOTAL_RUNS;
        profile.current_level = INITIAL_LEVEL;
        profile.available_runs = INITIAL_AVAILABLE_RUNS;
        profile.created_at = clock.unix_timestamp;
        profile.bump = ctx.bumps.player_profile;

        emit!(ProfileCreated {
            owner: profile.owner,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Updates the display name of an existing profile.
    pub fn update_profile_name(ctx: Context<UpdateProfileName>, name: String) -> Result<()> {
        // Validate name length
        require!(
            name.len() <= MAX_NAME_LENGTH,
            PlayerProfileError::NameTooLong
        );

        let profile = &mut ctx.accounts.player_profile;
        profile.name = name;

        Ok(())
    }

    /// Records the result of a completed dungeon run.
    pub fn record_run_result(
        ctx: Context<RecordRunResult>,
        level_reached: u8,
        victory: bool,
    ) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;

        // Decrement available runs
        profile.available_runs = profile
            .available_runs
            .checked_sub(1)
            .ok_or(PlayerProfileError::NoAvailableRuns)?;

        // Increment total runs
        profile.total_runs = profile
            .total_runs
            .checked_add(1)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        // On victory, advance level
        if victory {
            profile.current_level = profile
                .current_level
                .checked_add(1)
                .ok_or(PlayerProfileError::ArithmeticOverflow)?;
        }

        let clock = Clock::get()?;
        emit!(RunCompleted {
            owner: profile.owner,
            total_runs: profile.total_runs,
            available_runs: profile.available_runs,
            level_reached,
            victory,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

// ============================================================================
// Account Contexts
// ============================================================================

#[derive(Accounts)]
pub struct InitializeProfile<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + PlayerProfile::INIT_SPACE,
        seeds = [PlayerProfile::SEED_PREFIX, owner.key().as_ref()],
        bump
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateProfileName<'info> {
    #[account(
        mut,
        seeds = [PlayerProfile::SEED_PREFIX, owner.key().as_ref()],
        bump = player_profile.bump,
        has_one = owner @ PlayerProfileError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RecordRunResult<'info> {
    #[account(
        mut,
        seeds = [PlayerProfile::SEED_PREFIX, owner.key().as_ref()],
        bump = player_profile.bump,
        has_one = owner @ PlayerProfileError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    pub owner: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct ProfileCreated {
    pub owner: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct RunCompleted {
    pub owner: Pubkey,
    pub total_runs: u32,
    pub available_runs: u32,
    pub level_reached: u8,
    pub victory: bool,
    pub timestamp: i64,
}
