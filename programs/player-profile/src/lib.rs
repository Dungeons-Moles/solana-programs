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

declare_id!("Ch3bbL1oQk2z5rX1jiun3KuSWZqnXZ1MnrfrtKj4MKun");

fn is_player_queued_in_pit_draft(
    pit_draft_queue: &AccountInfo<'_>,
    player: Pubkey,
) -> Result<bool> {
    let gameplay_state_program = Pubkey::new_from_array(GAMEPLAY_STATE_PROGRAM_ID);
    let (expected_queue, _) =
        Pubkey::find_program_address(&[PIT_DRAFT_QUEUE_SEED], &gameplay_state_program);

    require_keys_eq!(
        pit_draft_queue.key(),
        expected_queue,
        PlayerProfileError::InvalidPitDraftQueue
    );
    require_keys_eq!(
        *pit_draft_queue.owner,
        gameplay_state_program,
        PlayerProfileError::InvalidPitDraftQueue
    );

    let data = pit_draft_queue.try_borrow_data()?;
    require!(data.len() >= 9, PlayerProfileError::InvalidPitDraftQueue);
    let mut cursor = 8usize; // skip discriminator

    let waiting_tag = data[cursor];
    cursor += 1;

    let waiting_player = if waiting_tag == 1 {
        require!(
            data.len() >= cursor + 32,
            PlayerProfileError::InvalidPitDraftQueue
        );
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[cursor..cursor + 32]);
        Some(Pubkey::new_from_array(bytes))
    } else {
        None
    };

    Ok(waiting_player == Some(player))
}

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
        profile.equipped_skin = None;
        profile.gauntlet_boosters = 0;

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
            !is_player_queued_in_pit_draft(&ctx.accounts.pit_draft_queue, ctx.accounts.owner.key())?,
            PlayerProfileError::PitDraftQueueLocked
        );

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
        _ctx: Context<RecordRunResult>,
        _level_completed: u8,
        _victory: bool,
    ) -> Result<()> {
        err!(PlayerProfileError::DirectMutationDisabled)
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
    /// This allows the session key signer to trigger run result recording without user interaction.
    ///
    /// Authorization: The session account proves player ownership. We verify:
    /// 1. Session account is owned by the session-manager program
    /// 2. Session's player field matches the profile's owner
    /// 3. Session key signer signer matches the session's stored session_signer
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

        // Verify session account has enough data to read through session_signer
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

        // Verify level_completed matches the session's campaign_level.
        require!(
            session_data[SESSION_CAMPAIGN_LEVEL_OFFSET] == level_completed,
            PlayerProfileError::LevelNotUnlocked
        );

        // Read session_signer from session account and verify it matches the signer
        let session_signer_key = Pubkey::try_from(
            &session_data[SESSION_SESSION_SIGNER_OFFSET..SESSION_SESSION_SIGNER_OFFSET + 32],
        )
        .map_err(|_| PlayerProfileError::InvalidSession)?;

        require!(
            session_signer_key == ctx.accounts.session_signer.key(),
            PlayerProfileError::InvalidSessionSigner
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
    /// Each purchase adds 20 runs and costs 0.005 SOL.
    pub fn purchase_runs(ctx: Context<PurchaseRuns>) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        let clock = Clock::get()?;
        let gameplay_state_program = Pubkey::new_from_array(GAMEPLAY_STATE_PROGRAM_ID);
        let (expected_gauntlet_pool, _) = Pubkey::find_program_address(
            &[GAUNTLET_POOL_VAULT_SEED],
            &gameplay_state_program,
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_pool.key(),
            expected_gauntlet_pool,
            PlayerProfileError::InvalidGauntletPool
        );
        require_keys_eq!(
            *ctx.accounts.gauntlet_pool.owner,
            gameplay_state_program,
            PlayerProfileError::InvalidGauntletPool
        );

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

    /// Equips a Metaplex Core skin NFT on the player's profile.
    /// Validates that the NFT is owned by the player and is a valid Metaplex Core asset.
    pub fn equip_skin(ctx: Context<EquipSkin>) -> Result<()> {
        let skin_asset = &ctx.accounts.skin_asset;

        // Validate the account is owned by Metaplex Core program
        require!(
            *skin_asset.owner == MPL_CORE_PROGRAM_ID,
            PlayerProfileError::InvalidSkinAsset
        );

        // Read raw bytes to validate ownership
        let data = skin_asset.try_borrow_data()?;
        require!(data.len() >= 33, PlayerProfileError::InvalidSkinAsset);

        // Byte 0: Key discriminator (1 = AssetV1)
        require!(data[0] == 1, PlayerProfileError::InvalidSkinAsset);

        // Bytes 1..33: Owner pubkey
        let mut owner_bytes = [0u8; 32];
        owner_bytes.copy_from_slice(&data[1..33]);
        let asset_owner = Pubkey::new_from_array(owner_bytes);
        require!(
            asset_owner == ctx.accounts.owner.key(),
            PlayerProfileError::SkinNotOwned
        );
        drop(data);

        let profile = &mut ctx.accounts.player_profile;
        profile.equipped_skin = Some(skin_asset.key());

        Ok(())
    }

    /// Unequips the currently equipped skin NFT.
    pub fn unequip_skin(ctx: Context<UnequipSkin>) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        profile.equipped_skin = None;
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

    /// CHECK: Validated in update_active_item_pool against gameplay-state PDA/owner.
    pub pit_draft_queue: AccountInfo<'info>,
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

    /// CHECK: All checks performed in record_run_result_cpi handler:
    /// 1. Account owner == session-manager program ID
    /// 2. session.player == player_profile.owner
    /// 3. session.campaign_level == level_completed input
    /// 4. session.session_signer == session_signer signer
    pub session: AccountInfo<'info>,

    /// Session key signer signer - verified against session's stored session_signer field.
    pub session_signer: Signer<'info>,

    #[account(
        seeds = [SESSION_MANAGER_AUTHORITY_SEED],
        bump,
        seeds::program = Pubkey::new_from_array(SESSION_MANAGER_PROGRAM_ID),
    )]
    /// CHECK: PDA signer proving CPI originates from session-manager program.
    pub session_manager_authority: Signer<'info>,
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

    /// CHECK: Validated in instruction to be the canonical gameplay-state
    /// gauntlet pool vault PDA and owned by gameplay-state program.
    #[account(mut)]
    pub gauntlet_pool: AccountInfo<'info>,

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

#[derive(Accounts)]
pub struct EquipSkin<'info> {
    #[account(
        mut,
        seeds = [PlayerProfile::SEED_PREFIX, owner.key().as_ref()],
        bump = player_profile.bump,
        has_one = owner @ PlayerProfileError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    pub owner: Signer<'info>,

    /// CHECK: Metaplex Core asset account. Validated in equip_skin handler:
    /// 1. Account owner == Metaplex Core program ID
    /// 2. Asset discriminator == 1 (AssetV1)
    /// 3. Asset owner field == player wallet
    pub skin_asset: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UnequipSkin<'info> {
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
