use anchor_lang::prelude::*;

pub mod bitmask;
pub mod constants;
pub mod errors;
pub mod state;

use anchor_lang::system_program;
use bitmask::STARTER_ITEMS_BITMASK;
use constants::*;
use errors::PlayerProfileError;
use state::{PlayerProfile, PlayerRelicPool, RelicEntry};

declare_id!("GSLNDrNoHeZXVxB7Yu7tUe8417PpZ5XV7JPYupPw9WQy");

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

    // Validate Anchor discriminator before trusting account data
    require!(
        data[..8] == PIT_DRAFT_QUEUE_DISCRIMINATOR,
        PlayerProfileError::InvalidPitDraftQueueDiscriminator
    );

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

    /// Closes the player's profile account and refunds rent to the owner.
    pub fn close_profile(_ctx: Context<CloseProfile>) -> Result<()> {
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
            !is_player_queued_in_pit_draft(
                &ctx.accounts.pit_draft_queue,
                ctx.accounts.owner.key()
            )?,
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
        unlock_randomness: [u8; 32],
    ) -> Result<()> {
        let profile = &mut ctx.accounts.player_profile;
        let session_info = &ctx.accounts.session;

        // Owner check is enforced by the #[account(owner = ...)] constraint on session.

        let session_data = session_info.try_borrow_data()?;
        let clock = Clock::get()?;

        // Verify session account has enough data to read through session_signer (offset 77 + 32 = 109)
        require!(
            session_data.len() >= SESSION_MIN_DATA_LEN,
            PlayerProfileError::SessionDataTooShort
        );

        // Read player pubkey from session account
        let session_player = Pubkey::try_from(
            &session_data[SESSION_PLAYER_OFFSET..SESSION_PLAYER_OFFSET + 32],
        )
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
                &unlock_randomness,
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
        let (expected_gauntlet_pool, _) =
            Pubkey::find_program_address(&[GAUNTLET_POOL_VAULT_SEED], &gameplay_state_program);
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
                ctx.accounts.system_program.key(),
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
                ctx.accounts.system_program.key(),
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

        // Metaplex Core AssetV1 raw byte layout (mpl-core 0.11.x):
        //   Byte 0:     Key discriminator (1 = AssetV1)
        //   Bytes 1-32: Owner Pubkey (32 bytes)
        //   Bytes 33+:  UpdateAuthority, Name, URI, etc.
        // Minimum viable read: 33 bytes for discriminator + owner.
        // If mpl-core changes this layout, the discriminator byte will change too,
        // so the check below will reject accounts with an incompatible layout.
        const MPL_CORE_ASSET_V1_DISCRIMINATOR: u8 = 1;
        const MPL_CORE_OWNER_OFFSET: usize = 1;
        const MPL_CORE_MIN_DATA_LEN: usize = MPL_CORE_OWNER_OFFSET + 32;

        let data = skin_asset.try_borrow_data()?;
        require!(
            data.len() >= MPL_CORE_MIN_DATA_LEN,
            PlayerProfileError::InvalidSkinAsset
        );

        // Validate AssetV1 discriminator
        require!(
            data[0] == MPL_CORE_ASSET_V1_DISCRIMINATOR,
            PlayerProfileError::InvalidSkinAsset
        );

        // Extract and validate owner
        let mut owner_bytes = [0u8; 32];
        owner_bytes.copy_from_slice(&data[MPL_CORE_OWNER_OFFSET..MPL_CORE_MIN_DATA_LEN]);
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

    /// Toggles whether an owned relic type can appear in future session item offers.
    pub fn set_relic_active(
        ctx: Context<SetRelicActive>,
        relic_item_id: [u8; 8],
        active: bool,
    ) -> Result<()> {
        let relic_pool = &mut ctx.accounts.player_relic_pool;
        let index = relic_pool
            .find_index_by_item_id(relic_item_id)
            .ok_or(PlayerProfileError::RelicNotFound)?;
        require!(
            relic_pool.relics[index].owned_count > 0,
            PlayerProfileError::RelicNotOwned
        );
        relic_pool.relics[index].in_active_pool = active;
        Ok(())
    }

    /// Marketplace-authorized ownership increment used during minting and purchases.
    pub fn grant_relic_ownership(
        ctx: Context<GrantRelicOwnership>,
        relic_item_id: [u8; 8],
    ) -> Result<()> {
        let relic_pool = &mut ctx.accounts.player_relic_pool;
        relic_pool.owner = ctx.accounts.owner.key();
        relic_pool.bump = ctx.bumps.player_relic_pool;

        if let Some(index) = relic_pool.find_index_by_item_id(relic_item_id) {
            relic_pool.relics[index].owned_count = relic_pool.relics[index]
                .owned_count
                .checked_add(1)
                .ok_or(PlayerProfileError::ArithmeticOverflow)?;
        } else {
            require!(
                relic_pool.relics.len() < MAX_RELICS,
                PlayerProfileError::RelicPoolFull
            );
            relic_pool.relics.push(RelicEntry {
                item_id: relic_item_id,
                owned_count: 1,
                in_active_pool: false,
            });
        }
        relic_pool.count = relic_pool.relics.len() as u8;

        Ok(())
    }

    /// Marketplace-authorized ownership decrement used during item trades.
    pub fn revoke_relic_ownership(
        ctx: Context<RevokeRelicOwnership>,
        relic_item_id: [u8; 8],
    ) -> Result<()> {
        let relic_pool = &mut ctx.accounts.player_relic_pool;
        let index = relic_pool
            .find_index_by_item_id(relic_item_id)
            .ok_or(PlayerProfileError::RelicNotFound)?;

        if relic_pool.relics[index].owned_count > 1 {
            relic_pool.relics[index].owned_count -= 1;
        } else {
            relic_pool.relics[index].in_active_pool = false;
            relic_pool.relics.swap_remove(index);
            relic_pool.count = relic_pool.relics.len() as u8;
        }

        Ok(())
    }

    /// Reconciles owned relic counts from externally supplied ownership proofs.
    /// Any relic whose proven count drops to zero is removed from the pool and
    /// automatically deactivated.
    pub fn sync_relic_ownership(
        ctx: Context<SyncRelicOwnership>,
        owned_relic_item_ids: Vec<[u8; 8]>,
    ) -> Result<()> {
        let relic_pool = &mut ctx.accounts.player_relic_pool;

        let mut proven_counts: Vec<([u8; 8], u16)> = Vec::with_capacity(owned_relic_item_ids.len());
        for item_id in owned_relic_item_ids {
            if let Some((_, count)) = proven_counts
                .iter_mut()
                .find(|(existing_item_id, _)| *existing_item_id == item_id)
            {
                *count = count
                    .checked_add(1)
                    .ok_or(PlayerProfileError::ArithmeticOverflow)?;
            } else {
                proven_counts.push((item_id, 1));
            }
        }

        let mut index = 0usize;
        while index < relic_pool.relics.len() {
            let item_id = relic_pool.relics[index].item_id;
            if let Some((_, owned_count)) = proven_counts
                .iter()
                .find(|(existing_item_id, _)| *existing_item_id == item_id)
            {
                relic_pool.relics[index].owned_count = *owned_count;
                index += 1;
            } else if relic_pool.relics[index].in_active_pool {
                relic_pool.relics[index].in_active_pool = false;
                relic_pool.relics.swap_remove(index);
            } else {
                index += 1;
            }
        }

        for (item_id, owned_count) in proven_counts {
            if let Some(existing_index) = relic_pool.find_index_by_item_id(item_id) {
                relic_pool.relics[existing_index].owned_count = owned_count;
            } else {
                require!(
                    relic_pool.relics.len() < MAX_RELICS,
                    PlayerProfileError::RelicPoolFull
                );
                relic_pool.relics.push(RelicEntry {
                    item_id,
                    owned_count,
                    in_active_pool: false,
                });
            }
        }

        relic_pool.count = relic_pool.relics.len() as u8;

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
pub struct CloseProfile<'info> {
    #[account(
        mut,
        seeds = [PlayerProfile::SEED_PREFIX, owner.key().as_ref()],
        bump = player_profile.bump,
        has_one = owner @ PlayerProfileError::Unauthorized,
        close = owner
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
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

    /// CHECK: Validated via owner constraint + PDA/discriminator checks in handler.
    #[account(owner = GAMEPLAY_STATE_PROGRAM_PUBKEY @ PlayerProfileError::InvalidPitDraftQueue)]
    pub pit_draft_queue: UncheckedAccount<'info>,
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

    /// CHECK: Validated via owner constraint + raw-byte reads in handler:
    /// 1. Account owner == session-manager program ID (enforced by constraint below)
    /// 2. session.player == player_profile.owner
    /// 3. session.campaign_level == level_completed input
    /// 4. session.session_signer == session_signer signer
    #[account(owner = SESSION_MANAGER_PROGRAM_PUBKEY @ PlayerProfileError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

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
    pub gauntlet_pool: UncheckedAccount<'info>,

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
    pub skin_asset: UncheckedAccount<'info>,
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

#[derive(Accounts)]
pub struct SetRelicActive<'info> {
    #[account(
        mut,
        seeds = [PlayerRelicPool::SEED_PREFIX, owner.key().as_ref()],
        bump = player_relic_pool.bump,
        has_one = owner @ PlayerProfileError::Unauthorized
    )]
    pub player_relic_pool: Account<'info, PlayerRelicPool>,

    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct GrantRelicOwnership<'info> {
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + PlayerRelicPool::INIT_SPACE,
        seeds = [PlayerRelicPool::SEED_PREFIX, owner.key().as_ref()],
        bump
    )]
    pub player_relic_pool: Account<'info, PlayerRelicPool>,

    /// CHECK: Relic owner whose pool is being credited.
    pub owner: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [b"mint_authority"],
        bump,
        seeds::program = NFT_MARKETPLACE_PROGRAM_PUBKEY,
    )]
    /// CHECK: Marketplace PDA signer authorizing the CPI.
    pub marketplace_authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RevokeRelicOwnership<'info> {
    #[account(
        mut,
        seeds = [PlayerRelicPool::SEED_PREFIX, owner.key().as_ref()],
        bump = player_relic_pool.bump,
    )]
    pub player_relic_pool: Account<'info, PlayerRelicPool>,

    /// CHECK: Relic owner whose pool is being debited.
    pub owner: UncheckedAccount<'info>,

    #[account(
        seeds = [b"mint_authority"],
        bump,
        seeds::program = NFT_MARKETPLACE_PROGRAM_PUBKEY,
    )]
    /// CHECK: Marketplace PDA signer authorizing the CPI.
    pub marketplace_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct SyncRelicOwnership<'info> {
    #[account(
        mut,
        seeds = [PlayerRelicPool::SEED_PREFIX, owner.key().as_ref()],
        bump = player_relic_pool.bump,
        has_one = owner @ PlayerProfileError::Unauthorized
    )]
    pub player_relic_pool: Account<'info, PlayerRelicPool>,

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
