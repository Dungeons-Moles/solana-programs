#![no_std]

#[cfg(test)]
extern crate std;

use quasar_lang::prelude::*;
use quasar_lang::sysvars::Sysvar as SysvarGet;

pub mod bitmask;
pub mod constants;
pub mod errors;
pub mod state;

use bitmask::STARTER_ITEMS_BITMASK;
use constants::*;
use errors::PlayerProfileError;

declare_id!("GSLNDrNoHeZXVxB7Yu7tUe8417PpZ5XV7JPYupPw9WQy");

const ZERO_ADDRESS: Address = Address::new_from_array([0; 32]);
const TREASURY_ADDRESS: Address = Address::new_from_array(TREASURY_PUBKEY);

fn is_player_queued_in_pit_draft(pit_draft_queue: &UncheckedAccount, player: Address) -> Result<bool, ProgramError> {
    let expected_queue =
        quasar_lang::pda::based_try_find_program_address(&[PIT_DRAFT_QUEUE_SEED], &GAMEPLAY_STATE_PROGRAM_ADDRESS)
            .map_err(|_| PlayerProfileError::InvalidPitDraftQueue)?
            .0;

    require_keys_eq!(
        pit_draft_queue.address(),
        &expected_queue,
        PlayerProfileError::InvalidPitDraftQueue
    );
    require_keys_eq!(
        pit_draft_queue.to_account_view().owner(),
        &GAMEPLAY_STATE_PROGRAM_ADDRESS,
        PlayerProfileError::InvalidPitDraftQueue
    );

    let data = pit_draft_queue.to_account_view().try_borrow()?;
    require!(data.len() >= 9, PlayerProfileError::InvalidPitDraftQueue);
    require!(
        data[..8] == PIT_DRAFT_QUEUE_DISCRIMINATOR,
        PlayerProfileError::InvalidPitDraftQueueDiscriminator
    );

    let mut cursor = 8usize;
    let waiting_tag = data[cursor];
    cursor += 1;

    let waiting_player = if waiting_tag == 1 {
        require!(data.len() >= cursor + 32, PlayerProfileError::InvalidPitDraftQueue);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&data[cursor..cursor + 32]);
        Some(Address::new_from_array(bytes))
    } else {
        None
    };

    Ok(waiting_player == Some(player))
}

fn checked_u32_add(value: u32, amount: u32) -> Result<u32, ProgramError> {
    value
        .checked_add(amount)
        .ok_or(PlayerProfileError::ArithmeticOverflow.into())
}

fn checked_u32_sub(value: u32, amount: u32) -> Result<u32, ProgramError> {
    value
        .checked_sub(amount)
        .ok_or(PlayerProfileError::ArithmeticOverflow.into())
}

fn initialize_relic_pool_if_needed(pool: &mut state::PlayerRelicPool, owner: Address, bump: u8) {
    if pool.owner == ZERO_ADDRESS {
        pool.set_inner(state::PlayerRelicPoolInner {
            owner,
            count: 0,
            bump,
            relic_entries: [0; state::RELIC_ENTRY_BYTES],
        });
    }
}

fn validate_marketplace_authority(authority: &Signer) -> Result<(), ProgramError> {
    let (expected, _) = quasar_lang::pda::based_try_find_program_address(
        &[b"mint_authority"],
        &NFT_MARKETPLACE_PROGRAM_ADDRESS,
    )
    .map_err(|_| PlayerProfileError::Unauthorized)?;
    require_keys_eq!(
        authority.address(),
        &expected,
        PlayerProfileError::Unauthorized
    );
    Ok(())
}

#[program(no_entrypoint)]
mod player_profile {
    use super::*;

    /// Creates a new player profile for the signer's wallet.
    #[instruction(discriminator = [32, 145, 77, 213, 58, 39, 251, 234])]
    pub fn initialize_profile(ctx: Ctx<InitializeProfile>, name: String<32>) -> Result<(), ProgramError> {
        require!(name.len() <= MAX_NAME_LENGTH, PlayerProfileError::NameTooLong);

        let clock = Clock::get()?;
        ctx.accounts.player_profile.set_inner(state::PlayerProfileInner {
            owner: *ctx.accounts.owner.address(),
            total_runs: INITIAL_TOTAL_RUNS,
            highest_level_unlocked: INITIAL_LEVEL,
            available_runs: INITIAL_AVAILABLE_RUNS,
            created_at: clock.unix_timestamp.get(),
            bump: ctx.bumps.player_profile,
            unlocked_items: STARTER_ITEMS_BITMASK,
            active_item_pool: STARTER_ITEMS_BITMASK,
            equipped_skin: None,
            gauntlet_boosters: 0,
            name: String::default(),
        });
        require!(
            ctx.accounts.player_profile.name.set(name),
            PlayerProfileError::NameTooLong
        );

        Ok(())
    }

    /// Updates the display name of an existing profile.
    #[instruction(discriminator = [96, 69, 10, 229, 192, 184, 200, 20])]
    pub fn update_profile_name(ctx: Ctx<UpdateProfileName>, name: String<32>) -> Result<(), ProgramError> {
        require!(name.len() <= MAX_NAME_LENGTH, PlayerProfileError::NameTooLong);
        require!(
            ctx.accounts.player_profile.name.set(name),
            PlayerProfileError::NameTooLong
        );
        Ok(())
    }

    /// Closes the player's profile account and refunds rent to the owner.
    #[instruction(discriminator = [167, 36, 181, 8, 136, 158, 46, 207])]
    pub fn close_profile(_ctx: Ctx<CloseProfile>) -> Result<(), ProgramError> {
        Ok(())
    }

    /// Updates the active item pool bitmask.
    #[instruction(discriminator = [39, 206, 212, 174, 10, 184, 19, 143])]
    pub fn update_active_item_pool(
        ctx: Ctx<UpdateActiveItemPool>,
        active_item_pool: [u8; ITEM_BITMASK_SIZE],
    ) -> Result<(), ProgramError> {
        require!(
            !is_player_queued_in_pit_draft(&ctx.accounts.pit_draft_queue, *ctx.accounts.owner.address())?,
            PlayerProfileError::PitDraftQueueLocked
        );
        require!(
            bitmask::is_subset(active_item_pool, ctx.accounts.player_profile.unlocked_items),
            PlayerProfileError::ItemNotUnlocked
        );
        require!(
            bitmask::count_bits(active_item_pool) >= MIN_ACTIVE_POOL_SIZE,
            PlayerProfileError::ActivePoolTooSmall
        );

        ctx.accounts.player_profile.active_item_pool = active_item_pool;
        Ok(())
    }

    /// Direct profile mutation remains disabled.
    #[instruction(discriminator = [135, 126, 78, 133, 237, 190, 21, 71])]
    pub fn record_run_result(
        _ctx: Ctx<RecordRunResult>,
        _level_completed: u8,
        _victory: bool,
    ) -> Result<(), ProgramError> {
        Err(PlayerProfileError::DirectMutationDisabled.into())
    }

    /// Consumes one available run from the player's profile.
    #[instruction(discriminator = [107, 101, 54, 82, 132, 156, 15, 34])]
    pub fn consume_run(ctx: Ctx<ConsumeRun>) -> Result<(), ProgramError> {
        let available_runs = ctx.accounts.player_profile.available_runs.get();
        require!(available_runs > 0, PlayerProfileError::NoAvailableRuns);
        ctx.accounts.player_profile.available_runs = checked_u32_sub(available_runs, 1)?.into();
        Ok(())
    }

    /// Records a completed dungeon run via CPI from session-manager.
    #[instruction(discriminator = [9, 175, 246, 9, 31, 98, 121, 69])]
    pub fn record_run_result_cpi(
        ctx: Ctx<RecordRunResultCpi>,
        level_completed: u8,
        victory: bool,
        unlock_randomness: [u8; 32],
    ) -> Result<(), ProgramError> {
        require_keys_eq!(
            ctx.accounts.session.to_account_view().owner(),
            &SESSION_MANAGER_PROGRAM_ADDRESS,
            PlayerProfileError::InvalidSessionOwner
        );
        let (expected_authority, _) = quasar_lang::pda::based_try_find_program_address(
            &[SESSION_MANAGER_AUTHORITY_SEED],
            &SESSION_MANAGER_PROGRAM_ADDRESS,
        )
        .map_err(|_| PlayerProfileError::InvalidSessionManagerAuthority)?;
        require_keys_eq!(
            ctx.accounts.session_manager_authority.address(),
            &expected_authority,
            PlayerProfileError::InvalidSessionManagerAuthority
        );

        let session_data = ctx.accounts.session.to_account_view().try_borrow()?;
        let clock = Clock::get()?;
        require!(
            session_data.len() >= SESSION_MIN_DATA_LEN,
            PlayerProfileError::SessionDataTooShort
        );

        let mut session_player = [0u8; 32];
        session_player.copy_from_slice(&session_data[SESSION_PLAYER_OFFSET..SESSION_PLAYER_OFFSET + 32]);
        require!(
            Address::new_from_array(session_player) == ctx.accounts.player_profile.owner,
            PlayerProfileError::Unauthorized
        );
        require!(
            session_data[SESSION_CAMPAIGN_LEVEL_OFFSET] == level_completed,
            PlayerProfileError::LevelNotUnlocked
        );

        let mut session_signer = [0u8; 32];
        session_signer.copy_from_slice(
            &session_data[SESSION_SESSION_SIGNER_OFFSET..SESSION_SESSION_SIGNER_OFFSET + 32],
        );
        require!(
            Address::new_from_array(session_signer) == *ctx.accounts.session_signer.address(),
            PlayerProfileError::InvalidSessionSigner
        );
        drop(session_data);

        let profile = &mut ctx.accounts.player_profile;
        let total_runs = checked_u32_add(profile.total_runs.get(), 1)?;
        profile.total_runs = total_runs.into();

        if victory && level_completed == profile.highest_level_unlocked {
            if profile.highest_level_unlocked < MAX_CAMPAIGN_LEVEL {
                profile.highest_level_unlocked = profile
                    .highest_level_unlocked
                    .checked_add(1)
                    .ok_or(PlayerProfileError::ArithmeticOverflow)?;
            }

            if let Some(item_index) = bitmask::select_random_locked_item(
                profile.unlocked_items,
                &profile.owner,
                level_completed,
                &unlock_randomness,
            ) {
                bitmask::set_bit(&mut profile.unlocked_items, item_index);
                bitmask::set_bit(&mut profile.active_item_pool, item_index);
                let _ = clock;
            }
        }

        Ok(())
    }

    /// Purchase additional runs and split payment between treasury and gauntlet pool.
    #[instruction(discriminator = [60, 148, 42, 27, 159, 188, 113, 143])]
    pub fn purchase_runs(ctx: Ctx<PurchaseRuns>) -> Result<(), ProgramError> {
        let (expected_gauntlet_pool, _) = quasar_lang::pda::based_try_find_program_address(
            &[GAUNTLET_POOL_VAULT_SEED],
            &GAMEPLAY_STATE_PROGRAM_ADDRESS,
        )
        .map_err(|_| PlayerProfileError::InvalidGauntletPool)?;
        require_keys_eq!(
            ctx.accounts.gauntlet_pool.address(),
            &expected_gauntlet_pool,
            PlayerProfileError::InvalidGauntletPool
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_pool.to_account_view().owner(),
            &GAMEPLAY_STATE_PROGRAM_ADDRESS,
            PlayerProfileError::InvalidGauntletPool
        );

        let half = RUN_PURCHASE_COST_LAMPORTS / 2;
        let treasury_amount = half;
        let gauntlet_amount = RUN_PURCHASE_COST_LAMPORTS
            .checked_sub(half)
            .ok_or(PlayerProfileError::ArithmeticOverflow)?;

        ctx.accounts
            .system_program
            .transfer(&ctx.accounts.owner, &ctx.accounts.treasury, treasury_amount)
            .invoke()?;
        ctx.accounts
            .system_program
            .transfer(&ctx.accounts.owner, &ctx.accounts.gauntlet_pool, gauntlet_amount)
            .invoke()?;

        let available_runs = checked_u32_add(
            ctx.accounts.player_profile.available_runs.get(),
            RUNS_PER_PURCHASE,
        )?;
        ctx.accounts.player_profile.available_runs = available_runs.into();
        Ok(())
    }

    /// Equips a Metaplex Core skin NFT on the player's profile.
    #[instruction(discriminator = [114, 85, 49, 172, 128, 188, 210, 76])]
    pub fn equip_skin(ctx: Ctx<EquipSkin>) -> Result<(), ProgramError> {
        require_keys_eq!(
            ctx.accounts.skin_asset.to_account_view().owner(),
            &MPL_CORE_PROGRAM_ID,
            PlayerProfileError::InvalidSkinAsset
        );

        const MPL_CORE_ASSET_V1_DISCRIMINATOR: u8 = 1;
        const MPL_CORE_OWNER_OFFSET: usize = 1;
        const MPL_CORE_MIN_DATA_LEN: usize = MPL_CORE_OWNER_OFFSET + 32;

        let data = ctx.accounts.skin_asset.to_account_view().try_borrow()?;
        require!(
            data.len() >= MPL_CORE_MIN_DATA_LEN,
            PlayerProfileError::InvalidSkinAsset
        );
        require!(
            data[0] == MPL_CORE_ASSET_V1_DISCRIMINATOR,
            PlayerProfileError::InvalidSkinAsset
        );

        let mut owner_bytes = [0u8; 32];
        owner_bytes.copy_from_slice(&data[MPL_CORE_OWNER_OFFSET..MPL_CORE_MIN_DATA_LEN]);
        require!(
            Address::new_from_array(owner_bytes) == *ctx.accounts.owner.address(),
            PlayerProfileError::SkinNotOwned
        );
        drop(data);

        ctx.accounts
            .player_profile
            .equipped_skin
            .set(Some(*ctx.accounts.skin_asset.address()));
        Ok(())
    }

    /// Unequips the currently equipped skin NFT.
    #[instruction(discriminator = [144, 147, 87, 145, 250, 113, 30, 135])]
    pub fn unequip_skin(ctx: Ctx<UnequipSkin>) -> Result<(), ProgramError> {
        ctx.accounts.player_profile.equipped_skin.set(None);
        Ok(())
    }

    /// Toggles whether an owned relic type can appear in future session item offers.
    #[instruction(discriminator = [59, 219, 164, 158, 201, 20, 227, 95])]
    pub fn set_relic_active(
        ctx: Ctx<SetRelicActive>,
        relic_item_id: [u8; 8],
        active: bool,
    ) -> Result<(), ProgramError> {
        let index = ctx
            .accounts
            .player_relic_pool
            .find_index_by_item_id(relic_item_id)
            .ok_or(PlayerProfileError::RelicNotFound)?;
        let mut entry = ctx.accounts.player_relic_pool.entry(index);
        require!(entry.owned_count > 0, PlayerProfileError::RelicNotOwned);
        entry.in_active_pool = active;
        ctx.accounts.player_relic_pool.set_entry(index, entry);
        Ok(())
    }

    /// Marketplace-authorized ownership increment used during minting and purchases.
    #[instruction(discriminator = [176, 52, 31, 132, 17, 73, 125, 192])]
    pub fn grant_relic_ownership(
        ctx: Ctx<GrantRelicOwnership>,
        relic_item_id: [u8; 8],
    ) -> Result<(), ProgramError> {
        validate_marketplace_authority(&ctx.accounts.marketplace_authority)?;
        let pool = &mut ctx.accounts.player_relic_pool;
        initialize_relic_pool_if_needed(pool, *ctx.accounts.owner.address(), ctx.bumps.player_relic_pool);

        if let Some(index) = pool.find_index_by_item_id(relic_item_id) {
            let mut entry = pool.entry(index);
            entry.owned_count = entry
                .owned_count
                .checked_add(1)
                .ok_or(PlayerProfileError::ArithmeticOverflow)?;
            pool.set_entry(index, entry);
        } else {
            pool.push_entry(state::RelicEntry::new(relic_item_id, 1, false))?;
        }
        Ok(())
    }

    /// Marketplace-authorized ownership decrement used during item trades.
    #[instruction(discriminator = [123, 199, 51, 123, 182, 0, 159, 177])]
    pub fn revoke_relic_ownership(
        ctx: Ctx<RevokeRelicOwnership>,
        relic_item_id: [u8; 8],
    ) -> Result<(), ProgramError> {
        validate_marketplace_authority(&ctx.accounts.marketplace_authority)?;
        let pool = &mut ctx.accounts.player_relic_pool;
        let index = pool
            .find_index_by_item_id(relic_item_id)
            .ok_or(PlayerProfileError::RelicNotFound)?;
        let mut entry = pool.entry(index);

        if entry.owned_count > 1 {
            entry.owned_count -= 1;
            pool.set_entry(index, entry);
        } else {
            pool.swap_remove(index);
        }
        Ok(())
    }

    /// Reconciles owned relic counts from externally supplied ownership proofs.
    #[instruction(discriminator = [35, 216, 49, 188, 212, 247, 12, 202])]
    pub fn sync_relic_ownership(
        ctx: Ctx<SyncRelicOwnership>,
        owned_relic_item_ids: Vec<[u8; 8], MAX_RELICS, 4>,
    ) -> Result<(), ProgramError> {
        let pool = &mut ctx.accounts.player_relic_pool;

        let mut proven_items = [[0u8; 8]; MAX_RELICS];
        let mut proven_counts = [0u16; MAX_RELICS];
        let mut proven_len = 0usize;

        for item_id in owned_relic_item_ids.iter() {
            let mut found = None;
            let mut index = 0usize;
            while index < proven_len {
                if proven_items[index] == *item_id {
                    found = Some(index);
                    break;
                }
                index += 1;
            }

            if let Some(existing_index) = found {
                proven_counts[existing_index] = proven_counts[existing_index]
                    .checked_add(1)
                    .ok_or(PlayerProfileError::ArithmeticOverflow)?;
            } else {
                require!(proven_len < MAX_RELICS, PlayerProfileError::RelicPoolFull);
                proven_items[proven_len] = *item_id;
                proven_counts[proven_len] = 1;
                proven_len += 1;
            }
        }

        let mut index = 0usize;
        while index < pool.count as usize {
            let item_id = pool.entry(index).item_id;
            let mut proven_index = None;
            let mut cursor = 0usize;
            while cursor < proven_len {
                if proven_items[cursor] == item_id {
                    proven_index = Some(cursor);
                    break;
                }
                cursor += 1;
            }

            if let Some(found_index) = proven_index {
                let mut entry = pool.entry(index);
                entry.owned_count = proven_counts[found_index];
                pool.set_entry(index, entry);
                index += 1;
            } else {
                pool.swap_remove(index);
            }
        }

        let mut cursor = 0usize;
        while cursor < proven_len {
            if let Some(existing_index) = pool.find_index_by_item_id(proven_items[cursor]) {
                let mut entry = pool.entry(existing_index);
                entry.owned_count = proven_counts[cursor];
                pool.set_entry(existing_index, entry);
            } else {
                pool.push_entry(state::RelicEntry::new(proven_items[cursor], proven_counts[cursor], false))?;
            }
            cursor += 1;
        }

        Ok(())
    }
}

#[cfg(all(
    not(feature = "no-entrypoint"),
    any(target_os = "solana", target_arch = "bpf")
))]
#[unsafe(no_mangle)]
#[allow(unexpected_cfgs)]
pub unsafe extern "C" fn entrypoint(ptr: *mut u8, instruction_data: *const u8) -> u64 {
    let instruction_data = unsafe {
        core::slice::from_raw_parts(
            instruction_data,
            *(instruction_data.sub(8) as *const u64) as usize,
        )
    };
    match player_profile::__dispatch(ptr, instruction_data) {
        Ok(_) => 0,
        Err(error) => error.into(),
    }
}

#[derive(Accounts)]
pub struct InitializeProfile {
    #[account(mut)]
    pub owner: Signer,
    #[account(
        mut,
        init,
        payer = owner,
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct UpdateProfileName {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
}

#[derive(Accounts)]
pub struct CloseProfile {
    #[account(mut)]
    pub owner: Signer,
    #[account(
        mut,
        close(dest = owner),
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
}

#[derive(Accounts)]
pub struct UpdateActiveItemPool {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
    pub pit_draft_queue: UncheckedAccount,
}

#[derive(Accounts)]
pub struct RecordRunResult {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
}

#[derive(Accounts)]
pub struct ConsumeRun {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
}

#[derive(Accounts)]
pub struct RecordRunResultCpi {
    #[account(mut)]
    pub player_profile: Account<state::PlayerProfile>,
    pub session: UncheckedAccount,
    pub session_signer: Signer,
    pub session_manager_authority: Signer,
}

#[derive(Accounts)]
pub struct PurchaseRuns {
    #[account(mut)]
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
    #[account(mut, address = TREASURY_ADDRESS)]
    pub treasury: UncheckedAccount,
    #[account(mut)]
    pub gauntlet_pool: UncheckedAccount,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct EquipSkin {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
    pub skin_asset: UncheckedAccount,
}

#[derive(Accounts)]
pub struct UnequipSkin {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerProfile::seeds(owner.address())
    )]
    pub player_profile: Account<state::PlayerProfile>,
}

#[derive(Accounts)]
pub struct SetRelicActive {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerRelicPool::seeds(owner.address())
    )]
    pub player_relic_pool: Account<state::PlayerRelicPool>,
}

#[derive(Accounts)]
pub struct GrantRelicOwnership {
    pub owner: UncheckedAccount,
    #[account(mut)]
    pub payer: Signer,
    #[account(
        mut,
        init(idempotent),
        payer = payer,
        address = state::PlayerRelicPool::seeds(owner.address())
    )]
    pub player_relic_pool: Account<state::PlayerRelicPool>,
    pub marketplace_authority: Signer,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct RevokeRelicOwnership {
    pub owner: UncheckedAccount,
    #[account(
        mut,
        address = state::PlayerRelicPool::seeds(owner.address())
    )]
    pub player_relic_pool: Account<state::PlayerRelicPool>,
    pub marketplace_authority: Signer,
}

#[derive(Accounts)]
pub struct SyncRelicOwnership {
    pub owner: Signer,
    #[account(
        mut,
        has_one(owner),
        address = state::PlayerRelicPool::seeds(owner.address())
    )]
    pub player_relic_pool: Account<state::PlayerRelicPool>,
}
