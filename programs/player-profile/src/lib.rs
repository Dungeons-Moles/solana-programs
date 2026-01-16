use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod state;

use constants::*;
use errors::PlayerProfileError;
use state::{PlayerProfile, Treasury};

declare_id!("3iAgDo4LeSkpqPo9G73ymaGwvdvPVKNYUjzoySVrGaYn");

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
        profile.unlocked_tier = INITIAL_TIER;
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

    /// Initializes the treasury account (admin only, one-time).
    pub fn initialize_treasury(ctx: Context<InitializeTreasury>) -> Result<()> {
        let treasury = &mut ctx.accounts.treasury;
        treasury.admin = ctx.accounts.admin.key();
        treasury.total_collected = 0;
        treasury.bump = ctx.bumps.treasury;

        Ok(())
    }

    /// Pays 0.05 SOL to unlock the next 40 campaign levels.
    pub fn unlock_campaign_tier(ctx: Context<UnlockCampaignTier>) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;

        // Calculate the tier boundary: 39, 79, 119, etc.
        let current_tier_max = profile
            .unlocked_tier
            .checked_add(1)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?
            .checked_mul(LEVELS_PER_TIER)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?
            .checked_sub(1)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        // Must be at the tier boundary to unlock
        require!(
            profile.current_level >= current_tier_max,
            PlayerProfileError::TierNotReached
        );

        // Store values for later use before borrowing treasury
        let profile_owner = profile.owner;
        let new_tier = profile
            .unlocked_tier
            .checked_add(1)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;
        profile.unlocked_tier = new_tier;

        // Get treasury key and account info before mutable borrow
        let treasury_key = ctx.accounts.treasury.key();

        // Transfer SOL to treasury
        let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.owner.key(),
            &treasury_key,
            TIER_UNLOCK_COST,
        );
        anchor_lang::solana_program::program::invoke(
            &transfer_ix,
            &[
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.treasury.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // Update treasury accounting
        let treasury = &mut ctx.accounts.treasury;
        treasury.total_collected = treasury
            .total_collected
            .checked_add(TIER_UNLOCK_COST)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        let clock = Clock::get()?;
        emit!(TierUnlocked {
            owner: profile_owner,
            tier: new_tier,
            amount_paid: TIER_UNLOCK_COST,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Withdraws SOL from the treasury (admin only).
    pub fn withdraw_treasury(ctx: Context<WithdrawTreasury>, amount: u64) -> Result<()> {
        // Transfer from treasury PDA using PDA signature
        **ctx.accounts.treasury.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.recipient.try_borrow_mut_lamports()? += amount;

        msg!("Withdrawn {} lamports from treasury", amount);

        Ok(())
    }

    /// Records the result of a completed dungeon run.
    pub fn record_run_result(
        ctx: Context<RecordRunResult>,
        level_reached: u8,
        victory: bool,
    ) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;

        // Increment total runs
        profile.total_runs = profile
            .total_runs
            .checked_add(1)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        // On victory, advance level if within unlocked tier
        if victory {
            let max_allowed_level = profile
                .unlocked_tier
                .checked_add(1)
                .ok_or(PlayerProfileError::ArithmeticOverflow)?
                .checked_mul(LEVELS_PER_TIER)
                .ok_or(PlayerProfileError::ArithmeticOverflow)?
                .checked_sub(1)
                .ok_or(PlayerProfileError::ArithmeticOverflow)?;

            if profile.current_level < max_allowed_level {
                profile.current_level = profile
                    .current_level
                    .checked_add(1)
                    .ok_or(PlayerProfileError::ArithmeticOverflow)?;
            }
        }

        let clock = Clock::get()?;
        emit!(RunCompleted {
            owner: profile.owner,
            total_runs: profile.total_runs,
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
pub struct InitializeTreasury<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + Treasury::INIT_SPACE,
        seeds = [Treasury::SEED_PREFIX],
        bump
    )]
    pub treasury: Account<'info, Treasury>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnlockCampaignTier<'info> {
    #[account(
        mut,
        seeds = [PlayerProfile::SEED_PREFIX, owner.key().as_ref()],
        bump = player_profile.bump,
        has_one = owner @ PlayerProfileError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [Treasury::SEED_PREFIX],
        bump = treasury.bump
    )]
    pub treasury: Account<'info, Treasury>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawTreasury<'info> {
    #[account(
        mut,
        seeds = [Treasury::SEED_PREFIX],
        bump = treasury.bump,
        has_one = admin @ PlayerProfileError::Unauthorized
    )]
    pub treasury: Account<'info, Treasury>,

    pub admin: Signer<'info>,

    /// CHECK: Recipient account to receive withdrawn funds
    #[account(mut)]
    pub recipient: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
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
pub struct TierUnlocked {
    pub owner: Pubkey,
    pub tier: u8,
    pub amount_paid: u64,
    pub timestamp: i64,
}

#[event]
pub struct RunCompleted {
    pub owner: Pubkey,
    pub total_runs: u32,
    pub level_reached: u8,
    pub victory: bool,
    pub timestamp: i64,
}
