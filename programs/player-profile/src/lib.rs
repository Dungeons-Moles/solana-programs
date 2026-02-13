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

    /// Updates the active item pool bitmask.
    /// The pool must be a subset of unlocked items and contain at least 40 entries.
    pub fn update_active_item_pool(
        ctx: Context<UpdateActiveItemPool>,
        active_item_pool: [u8; ITEM_BITMASK_SIZE],
    ) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;

        require!(
            bitmask::is_subset(active_item_pool, profile.unlocked_items),
            PlayerProfileError::ItemNotUnlocked
        );

        require!(
            bitmask::count_bits(active_item_pool) >= MIN_ACTIVE_POOL_SIZE,
            PlayerProfileError::ActivePoolTooSmall
        );

        profile.active_item_pool = active_item_pool;

        Ok(())
    }

    /// Records the result of a completed dungeon run.
    /// On first-time victory, unlocks the next level and a random item.
    /// Note: available_runs is NOT decremented here - it's already done by consume_run
    /// at session start via CPI from session-manager.
    pub fn record_run_result(
        ctx: Context<RecordRunResult>,
        level_completed: u8,
        victory: bool,
    ) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        let clock = Clock::get()?;

        // Note: available_runs already decremented by consume_run at session start

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

            // Unlock a random item from the locked pool (indices 40-92)
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

    /// Consumes one available run from the player's profile.
    /// Called via CPI from session-manager when starting a new session.
    pub fn consume_run(ctx: Context<ConsumeRun>) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;

        require!(
            profile.available_runs > 0,
            PlayerProfileError::NoAvailableRuns
        );

        profile.available_runs = profile
            .available_runs
            .checked_sub(1)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        emit!(RunConsumed {
            owner: profile.owner,
            available_runs: profile.available_runs,
        });

        Ok(())
    }

    /// Records the result of a completed dungeon run via CPI from session-manager.
    /// Uses session account for authorization instead of requiring player signature.
    /// This allows the burner wallet to trigger run result recording without user interaction.
    ///
    /// Authorization: The session account proves player ownership. We verify:
    /// 1. Session account is owned by the session-manager program
    /// 2. Session's player field matches the profile's owner
    /// 3. Burner wallet signer matches the session's stored burner_wallet
    pub fn record_run_result_cpi(
        ctx: Context<RecordRunResultCpi>,
        level_completed: u8,
        victory: bool,
    ) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        let session_info = &ctx.accounts.session;

        // Verify session account is owned by the session-manager program
        require!(
            *session_info.owner == Pubkey::new_from_array(SESSION_MANAGER_PROGRAM_ID),
            PlayerProfileError::InvalidSessionOwner
        );

        let session_data = session_info.try_borrow_data()?;
        let clock = Clock::get()?;

        // Verify session account has enough data to read through burner_wallet
        require!(
            session_data.len() >= SESSION_MIN_DATA_LEN,
            PlayerProfileError::InvalidSession
        );

        // Read player pubkey from session account (offset 8 for discriminator)
        let session_player = Pubkey::try_from(&session_data[8..40])
            .map_err(|_| PlayerProfileError::InvalidSession)?;

        // Verify session's player matches profile's owner
        require!(
            session_player == profile.owner,
            PlayerProfileError::Unauthorized
        );

        // Read burner_wallet from session account and verify it matches the signer
        let session_burner = Pubkey::try_from(
            &session_data[SESSION_BURNER_WALLET_OFFSET..SESSION_BURNER_WALLET_OFFSET + 32],
        )
        .map_err(|_| PlayerProfileError::InvalidSession)?;

        require!(
            session_burner == ctx.accounts.burner_wallet.key(),
            PlayerProfileError::InvalidBurnerWallet
        );

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

            // Unlock a random item from the locked pool (indices 40-92)
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

    /// Purchase additional runs and split payment between treasury and gauntlet pool.
    /// Each purchase adds 20 runs and costs 0.001 SOL.
    pub fn purchase_runs(ctx: Context<PurchaseRuns>) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        let clock = Clock::get()?;
        let half = RUN_PURCHASE_COST_LAMPORTS / 2;
        let treasury_amount = half;
        let gauntlet_amount = RUN_PURCHASE_COST_LAMPORTS
            .checked_sub(half)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        // Transfer treasury split.
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.owner.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            treasury_amount,
        )?;

        // Transfer gauntlet pool split.
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.owner.to_account_info(),
                    to: ctx.accounts.gauntlet_pool.to_account_info(),
                },
            ),
            gauntlet_amount,
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
pub struct UpdateActiveItemPool<'info> {
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

/// Context for recording run results via CPI from session-manager.
/// Uses session account for authorization instead of requiring player signature.
#[derive(Accounts)]
pub struct RecordRunResultCpi<'info> {
    #[account(mut)]
    pub player_profile: Account<'info, PlayerProfile>,

    /// CHECK: All three checks performed in record_run_result_cpi handler:
    /// 1. Account owner == session-manager program ID
    /// 2. session.player == player_profile.owner
    /// 3. session.burner_wallet == burner_wallet signer
    pub session: AccountInfo<'info>,

    /// Burner wallet signer - verified against session's stored burner_wallet field.
    pub burner_wallet: Signer<'info>,
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

    #[account(mut)]
    pub gauntlet_pool: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ConsumeRun<'info> {
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

/// Emitted when a new item is unlocked on first-time level completion
#[event]
pub struct ItemUnlocked {
    pub owner: Pubkey,
    pub item_index: u8,
    pub level_completed: u8,
    pub timestamp: i64,
}

/// Emitted when a run is consumed at session start
#[event]
pub struct RunConsumed {
    pub owner: Pubkey,
    pub available_runs: u32,
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
