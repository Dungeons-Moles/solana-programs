use anchor_lang::prelude::*;

pub mod bitmask;
pub mod constants;
pub mod errors;
pub mod state;

use anchor_lang::system_program;
use bitmask::STARTER_ITEMS_BITMASK;
use constants::*;
use errors::PlayerProfileError;
use state::PlayerProfile;

declare_id!("29DPbP1zuCCRg63PiShMjxAmZos97BR5TmhpijUYQzze");

#[program]
pub mod player_profile {
    use super::*;

    /// Creates a new player profile for the signer's wallet.
    /// Each wallet can only have one profile.
    /// Initializes with 20 runs, level 1 unlocked, and 40 starter items.
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
        profile.highest_level_unlocked = INITIAL_LEVEL;
        profile.available_runs = INITIAL_AVAILABLE_RUNS;
        profile.created_at = clock.unix_timestamp;
        profile.bump = ctx.bumps.player_profile;
        // Initialize with starter items (bits 0-39 set)
        profile.unlocked_items = STARTER_ITEMS_BITMASK;
        profile.active_item_pool = STARTER_ITEMS_BITMASK;

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
    /// On first-time victory, unlocks the next level and a random item.
    pub fn record_run_result(
        ctx: Context<RecordRunResult>,
        level_completed: u8,
        victory: bool,
    ) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        let clock = Clock::get()?;

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

        // On first-time victory (completing highest unlocked level), advance and unlock item
        if victory && level_completed == profile.highest_level_unlocked {
            // Increment highest level unlocked (cap at MAX_CAMPAIGN_LEVEL)
            if profile.highest_level_unlocked < MAX_CAMPAIGN_LEVEL {
                profile.highest_level_unlocked = profile
                    .highest_level_unlocked
                    .checked_add(1)
                    .ok_or(PlayerProfileError::ArithmeticOverflow)?;
            }

            // Unlock a random item from the locked pool (indices 40-79)
            if let Some(item_index) = bitmask::select_random_locked_item(
                profile.unlocked_items,
                &profile.owner,
                level_completed,
                clock.slot,
            ) {
                bitmask::set_bit(&mut profile.unlocked_items, item_index);
                bitmask::set_bit(&mut profile.active_item_pool, item_index);

                emit!(ItemUnlocked {
                    owner: profile.owner,
                    item_index,
                    level_completed,
                    timestamp: clock.unix_timestamp,
                });
            }
        }

        emit!(RunCompleted {
            owner: profile.owner,
            total_runs: profile.total_runs,
            available_runs: profile.available_runs,
            level_reached: level_completed,
            victory,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Purchase additional runs by paying SOL to the treasury.
    /// Each purchase adds 20 runs and costs 0.001 SOL.
    pub fn purchase_runs(ctx: Context<PurchaseRuns>) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        let clock = Clock::get()?;

        // Transfer SOL from player to treasury
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.owner.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            RUN_PURCHASE_COST_LAMPORTS,
        )?;

        // Add runs to profile
        profile.available_runs = profile
            .available_runs
            .checked_add(RUNS_PER_PURCHASE)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        emit!(RunsPurchased {
            owner: profile.owner,
            runs_added: RUNS_PER_PURCHASE,
            total_available_runs: profile.available_runs,
            cost_lamports: RUN_PURCHASE_COST_LAMPORTS,
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

#[derive(Accounts)]
pub struct PurchaseRuns<'info> {
    #[account(
        mut,
        seeds = [PlayerProfile::SEED_PREFIX, owner.key().as_ref()],
        bump = player_profile.bump,
        has_one = owner @ PlayerProfileError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub owner: Signer<'info>,

    /// The treasury account to receive payment.
    /// Validated to be the expected treasury pubkey.
    #[account(
        mut,
        address = Pubkey::new_from_array(TREASURY_PUBKEY) @ PlayerProfileError::InvalidTreasury
    )]
    pub treasury: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
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

/// Emitted when a new item is unlocked on first-time level completion
#[event]
pub struct ItemUnlocked {
    pub owner: Pubkey,
    pub item_index: u8,
    pub level_completed: u8,
    pub timestamp: i64,
}

/// Emitted when a player purchases additional runs
#[event]
pub struct RunsPurchased {
    pub owner: Pubkey,
    pub runs_added: u32,
    pub total_available_runs: u32,
    pub cost_lamports: u64,
    pub timestamp: i64,
}
