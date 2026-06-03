#![no_std]

#[cfg(test)]
extern crate std;

use {
    constants::*,
    errors::MarketplaceError,
    mpl_core_cpi::*,
    quasar_lang::{
        cpi::{CpiDynamic, CpiSignerSeeds, Seed},
        prelude::*,
        sysvars::Sysvar as SysvarGet,
    },
    state::*,
};

pub mod constants;
pub mod errors;
pub mod mpl_core_cpi;
pub mod state;

declare_id!("GLKxBpZ8hc7qzvD9VHAVsJEjHSu2JVp1HaPrGH4fpTci");

#[inline(always)]
fn require_unchecked_address(
    account: &UncheckedAccount,
    expected: &Address,
    error: MarketplaceError,
) -> Result<(), ProgramError> {
    require_keys_eq!(account.address(), expected, error);
    Ok(())
}

#[inline(always)]
fn validate_mint_authority(account: &UncheckedAccount) -> Result<u8, ProgramError> {
    let (expected, bump) = quasar_lang::pda::based_try_find_program_address(
        &[MINT_AUTHORITY_SEED],
        &MARKETPLACE_PROGRAM_ADDRESS,
    )
    .map_err(|_| MarketplaceError::InvalidMintAuthority)?;
    require_keys_eq!(
        account.address(),
        &expected,
        MarketplaceError::InvalidMintAuthority
    );
    Ok(bump)
}

#[inline(always)]
fn validate_gauntlet_pool(account: &UncheckedAccount) -> Result<(), ProgramError> {
    let (expected, _) = quasar_lang::pda::based_try_find_program_address(
        &[GAUNTLET_POOL_VAULT_SEED],
        &GAMEPLAY_STATE_PROGRAM_ADDRESS,
    )
    .map_err(|_| MarketplaceError::InvalidGauntletPool)?;
    require_keys_eq!(
        account.address(),
        &expected,
        MarketplaceError::InvalidGauntletPool
    );
    Ok(())
}

#[inline(always)]
fn validate_external_programs(
    mpl_core_program: &UncheckedAccount,
    player_profile_program: Option<&UncheckedAccount>,
) -> Result<(), ProgramError> {
    require_unchecked_address(
        mpl_core_program,
        &MPL_CORE_PROGRAM_ADDRESS,
        MarketplaceError::InvalidAsset,
    )?;
    if let Some(program) = player_profile_program {
        require_unchecked_address(
            program,
            &PLAYER_PROFILE_PROGRAM_ADDRESS,
            MarketplaceError::Unauthorized,
        )?;
    }
    Ok(())
}

#[inline(always)]
fn validate_collection(
    config: &MarketplaceConfig,
    collection: &UncheckedAccount,
) -> Result<(), ProgramError> {
    require!(
        *collection.address() == config.skins_collection
            || *collection.address() == config.items_collection,
        MarketplaceError::InvalidCollection
    );
    Ok(())
}

#[inline(always)]
fn validate_player_profile_pda(
    profile: &UncheckedAccount,
    owner: &Address,
) -> Result<(), ProgramError> {
    let (expected, _) = quasar_lang::pda::based_try_find_program_address(
        &[PLAYER_PROFILE_SEED, owner.as_ref()],
        &PLAYER_PROFILE_PROGRAM_ADDRESS,
    )
    .map_err(|_| MarketplaceError::InvalidAccountData)?;
    require_keys_eq!(
        profile.address(),
        &expected,
        MarketplaceError::InvalidAccountData
    );
    Ok(())
}

#[inline(always)]
fn validate_asset_owner(asset: &UncheckedAccount, seller: &Signer) -> Result<(), ProgramError> {
    require_keys_eq!(
        asset.to_account_view().owner(),
        &MPL_CORE_PROGRAM_ADDRESS,
        MarketplaceError::InvalidAsset
    );

    let asset_data = asset.to_account_view().try_borrow()?;
    require!(
        asset_data.len() >= MPL_CORE_MIN_DATA_LEN,
        MarketplaceError::InvalidAsset
    );
    require!(
        asset_data[0] == MPL_CORE_ASSET_V1_DISCRIMINATOR,
        MarketplaceError::InvalidAsset
    );

    let mut owner_bytes = [0u8; 32];
    owner_bytes.copy_from_slice(&asset_data[MPL_CORE_OWNER_OFFSET..MPL_CORE_MIN_DATA_LEN]);
    require!(
        Address::new_from_array(owner_bytes) == *seller.address(),
        MarketplaceError::NotOwner
    );
    Ok(())
}

#[inline(always)]
fn require_skin_not_equipped(
    profile: &UncheckedAccount,
    seller: &Address,
    asset: &Address,
) -> Result<(), ProgramError> {
    validate_player_profile_pda(profile, seller)?;
    if profile.to_account_view().owner() != &PLAYER_PROFILE_PROGRAM_ADDRESS {
        return Ok(());
    }

    let profile_data = profile.to_account_view().try_borrow()?;
    let min_len = PROFILE_EQUIPPED_SKIN_OFFSET + 33;
    if profile_data.len() < min_len {
        return Ok(());
    }
    if profile_data[..8] != PLAYER_PROFILE_DISCRIMINATOR {
        return Ok(());
    }
    if profile_data[PROFILE_EQUIPPED_SKIN_OFFSET] == 1 {
        let mut skin_bytes = [0u8; 32];
        skin_bytes.copy_from_slice(
            &profile_data[PROFILE_EQUIPPED_SKIN_OFFSET + 1..PROFILE_EQUIPPED_SKIN_OFFSET + 33],
        );
        require!(
            Address::new_from_array(skin_bytes) != *asset,
            MarketplaceError::SkinCurrentlyEquipped
        );
    }
    Ok(())
}

#[inline(always)]
fn checked_fee(price: u64, bps: u16) -> Result<u64, ProgramError> {
    price
        .checked_mul(bps as u64)
        .ok_or(MarketplaceError::ArithmeticOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(MarketplaceError::ArithmeticOverflow.into())
}

#[inline(always)]
fn checked_seller_amount(
    price: u64,
    company_fee: u64,
    gauntlet_fee: u64,
) -> Result<u64, ProgramError> {
    price
        .checked_sub(company_fee)
        .ok_or(MarketplaceError::ArithmeticOverflow)?
        .checked_sub(gauntlet_fee)
        .ok_or(MarketplaceError::ArithmeticOverflow.into())
}

fn grant_relic_to_owner<S: CpiSignerSeeds + ?Sized>(
    player_profile_program: &AccountView,
    owner: &AccountView,
    payer: &AccountView,
    player_relic_pool: &AccountView,
    marketplace_authority: &AccountView,
    rent: &AccountView,
    system_program: &AccountView,
    relic_item_id: [u8; 8],
    signer_seeds: &S,
) -> Result<(), ProgramError> {
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&[176, 52, 31, 132, 17, 73, 125, 192]);
    data[8..16].copy_from_slice(&relic_item_id);

    let mut cpi = CpiDynamic::<6, 16>::new(player_profile_program.address());
    cpi.push_account(owner, false, false)?;
    cpi.push_account(payer, true, true)?;
    cpi.push_account(player_relic_pool, false, true)?;
    cpi.push_account(marketplace_authority, true, false)?;
    cpi.push_account(rent, false, false)?;
    cpi.push_account(system_program, false, false)?;
    cpi.set_data(&data)?;
    cpi.invoke_signed(signer_seeds)
}

fn revoke_relic_from_owner<S: CpiSignerSeeds + ?Sized>(
    player_profile_program: &AccountView,
    owner: &AccountView,
    player_relic_pool: &AccountView,
    marketplace_authority: &AccountView,
    relic_item_id: [u8; 8],
    signer_seeds: &S,
) -> Result<(), ProgramError> {
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&[123, 199, 51, 123, 182, 0, 159, 177]);
    data[8..16].copy_from_slice(&relic_item_id);

    let mut cpi = CpiDynamic::<3, 16>::new(player_profile_program.address());
    cpi.push_account(owner, false, false)?;
    cpi.push_account(player_relic_pool, false, true)?;
    cpi.push_account(marketplace_authority, true, false)?;
    cpi.set_data(&data)?;
    cpi.invoke_signed(signer_seeds)
}

#[program(no_entrypoint)]
mod nft_marketplace {
    use super::*;

    #[instruction(discriminator = [47, 81, 64, 0, 96, 56, 105, 7])]
    pub fn initialize_marketplace(
        ctx: Ctx<InitializeMarketplace>,
        skins_collection: Address,
        items_collection: Address,
    ) -> Result<(), ProgramError> {
        validate_gauntlet_pool(&ctx.accounts.gauntlet_pool)?;

        let total_fee_bps = (DEFAULT_COMPANY_FEE_BPS as u64)
            .checked_add(DEFAULT_GAUNTLET_FEE_BPS as u64)
            .ok_or(MarketplaceError::ArithmeticOverflow)?;
        require!(
            total_fee_bps < BPS_DENOMINATOR,
            MarketplaceError::FeeTooHigh
        );

        ctx.accounts
            .marketplace_config
            .set_inner(state::MarketplaceConfigInner {
                authority: *ctx.accounts.authority.address(),
                skins_collection,
                items_collection,
                company_treasury: COMPANY_TREASURY,
                gauntlet_pool: *ctx.accounts.gauntlet_pool.address(),
                company_fee_bps: DEFAULT_COMPANY_FEE_BPS,
                gauntlet_fee_bps: DEFAULT_GAUNTLET_FEE_BPS,
                bump: ctx.bumps.marketplace_config,
            });
        Ok(())
    }

    #[instruction(discriminator = [142, 213, 165, 190, 25, 244, 82, 176])]
    pub fn mint_skin(
        ctx: Ctx<MintSkin>,
        _skin_id: u16,
        _season: u8,
        _rarity: u8,
        name: String<64>,
        uri: String<200>,
    ) -> Result<(), ProgramError> {
        let bump = validate_mint_authority(&ctx.accounts.mint_authority)?;
        validate_external_programs(&ctx.accounts.mpl_core_program, None)?;
        require_unchecked_address(
            &ctx.accounts.log_wrapper,
            &SPL_NOOP_PROGRAM_ADDRESS,
            MarketplaceError::InvalidAsset,
        )?;
        require_keys_eq!(
            ctx.accounts.collection.address(),
            &ctx.accounts.marketplace_config.skins_collection,
            MarketplaceError::InvalidCollection
        );
        require_keys_eq!(
            ctx.accounts.payer.address(),
            &ctx.accounts.marketplace_config.authority,
            MarketplaceError::Unauthorized
        );

        let bump_bytes = [bump];
        let signer_seeds = [
            Seed::from(MINT_AUTHORITY_SEED),
            Seed::from(bump_bytes.as_ref()),
        ];
        let mpl_core_program = ctx.accounts.mpl_core_program.to_account_view();
        let asset = ctx.accounts.asset.to_account_view();
        let collection = ctx.accounts.collection.to_account_view();
        let mint_authority = ctx.accounts.mint_authority.to_account_view();
        let payer = ctx.accounts.payer.to_account_view();
        let owner = ctx.accounts.owner.to_account_view();
        let system_program = ctx.accounts.system_program.to_account_view();
        let log_wrapper = ctx.accounts.log_wrapper.to_account_view();

        create_v1(
            CreateV1Accounts {
                program: &mpl_core_program,
                asset: &asset,
                collection: Some(&collection),
                authority: Some(&mint_authority),
                payer: &payer,
                owner: Some(&owner),
                update_authority: None,
                system_program: &system_program,
                log_wrapper: Some(&log_wrapper),
            },
            name,
            uri,
            &signer_seeds,
        )
    }

    #[instruction(discriminator = [225, 105, 7, 236, 107, 78, 104, 144])]
    pub fn mint_nft_item(
        ctx: Ctx<MintNftItem>,
        nft_item_id: [u8; 8],
        name: String<64>,
        uri: String<200>,
    ) -> Result<(), ProgramError> {
        let bump = validate_mint_authority(&ctx.accounts.mint_authority)?;
        validate_external_programs(
            &ctx.accounts.mpl_core_program,
            Some(&ctx.accounts.player_profile_program),
        )?;
        require_unchecked_address(
            &ctx.accounts.log_wrapper,
            &SPL_NOOP_PROGRAM_ADDRESS,
            MarketplaceError::InvalidAsset,
        )?;
        require_keys_eq!(
            ctx.accounts.collection.address(),
            &ctx.accounts.marketplace_config.items_collection,
            MarketplaceError::InvalidCollection
        );
        require_keys_eq!(
            ctx.accounts.payer.address(),
            &ctx.accounts.marketplace_config.authority,
            MarketplaceError::Unauthorized
        );

        let bump_bytes = [bump];
        let signer_seeds = [
            Seed::from(MINT_AUTHORITY_SEED),
            Seed::from(bump_bytes.as_ref()),
        ];
        let mpl_core_program = ctx.accounts.mpl_core_program.to_account_view();
        let asset = ctx.accounts.asset.to_account_view();
        let collection = ctx.accounts.collection.to_account_view();
        let mint_authority = ctx.accounts.mint_authority.to_account_view();
        let payer = ctx.accounts.payer.to_account_view();
        let owner = ctx.accounts.owner.to_account_view();
        let system_program = ctx.accounts.system_program.to_account_view();
        let log_wrapper = ctx.accounts.log_wrapper.to_account_view();

        create_v1(
            CreateV1Accounts {
                program: &mpl_core_program,
                asset: &asset,
                collection: Some(&collection),
                authority: Some(&mint_authority),
                payer: &payer,
                owner: Some(&owner),
                update_authority: None,
                system_program: &system_program,
                log_wrapper: Some(&log_wrapper),
            },
            name,
            uri,
            &signer_seeds,
        )?;

        ctx.accounts.relic_asset.set_inner(state::RelicAssetInner {
            asset: *ctx.accounts.asset.address(),
            item_id: nft_item_id,
            bump: ctx.bumps.relic_asset,
        });

        let player_profile_program = ctx.accounts.player_profile_program.to_account_view();
        let player_relic_pool = ctx.accounts.player_relic_pool.to_account_view();
        let rent = ctx.accounts.rent.to_account_view();

        grant_relic_to_owner(
            &player_profile_program,
            &owner,
            &payer,
            &player_relic_pool,
            &mint_authority,
            &rent,
            &system_program,
            nft_item_id,
            &signer_seeds,
        )
    }

    #[instruction(discriminator = [88, 221, 93, 166, 63, 220, 106, 232])]
    pub fn list_nft(ctx: Ctx<ListNft>, price_lamports: u64) -> Result<(), ProgramError> {
        require!(price_lamports > 0, MarketplaceError::InvalidPrice);
        require!(
            price_lamports >= MIN_LISTING_PRICE,
            MarketplaceError::PriceTooLow
        );

        let bump = validate_mint_authority(&ctx.accounts.mint_authority)?;
        validate_external_programs(&ctx.accounts.mpl_core_program, None)?;
        validate_collection(&ctx.accounts.marketplace_config, &ctx.accounts.collection)?;
        validate_asset_owner(&ctx.accounts.asset, &ctx.accounts.seller)?;

        if *ctx.accounts.collection.address() == ctx.accounts.marketplace_config.skins_collection {
            require_skin_not_equipped(
                &ctx.accounts.player_profile,
                ctx.accounts.seller.address(),
                ctx.accounts.asset.address(),
            )?;
        }

        let clock = Clock::get()?;
        ctx.accounts.listing.set_inner(state::ListingInner {
            seller: *ctx.accounts.seller.address(),
            asset: *ctx.accounts.asset.address(),
            collection: *ctx.accounts.collection.address(),
            price_lamports,
            created_at: clock.unix_timestamp.get(),
            bump: ctx.bumps.listing,
        });

        let mpl_core_program = ctx.accounts.mpl_core_program.to_account_view();
        let asset = ctx.accounts.asset.to_account_view();
        let collection = ctx.accounts.collection.to_account_view();
        let seller = ctx.accounts.seller.to_account_view();
        let system_program = ctx.accounts.system_program.to_account_view();

        add_transfer_delegate_plugin(AddPluginV1Accounts {
            program: &mpl_core_program,
            asset: &asset,
            collection: Some(&collection),
            payer: &seller,
            authority: Some(&seller),
            system_program: &system_program,
        })?;

        let _ = bump;
        approve_transfer_delegate_authority(
            ApprovePluginAuthorityV1Accounts {
                program: &mpl_core_program,
                asset: &asset,
                collection: Some(&collection),
                payer: &seller,
                authority: Some(&seller),
                system_program: &system_program,
            },
            ctx.accounts.mint_authority.address(),
        )
    }

    #[instruction(discriminator = [41, 183, 50, 232, 230, 233, 157, 70])]
    pub fn cancel_listing(ctx: Ctx<CancelListing>) -> Result<(), ProgramError> {
        validate_external_programs(&ctx.accounts.mpl_core_program, None)?;
        require_keys_eq!(
            ctx.accounts.listing.seller,
            *ctx.accounts.seller.address(),
            MarketplaceError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.listing.asset,
            *ctx.accounts.asset.address(),
            MarketplaceError::InvalidAsset
        );

        let mpl_core_program = ctx.accounts.mpl_core_program.to_account_view();
        let asset = ctx.accounts.asset.to_account_view();
        let collection = ctx.accounts.collection.to_account_view();
        let seller = ctx.accounts.seller.to_account_view();
        let system_program = ctx.accounts.system_program.to_account_view();

        remove_transfer_delegate_plugin(RemovePluginV1Accounts {
            program: &mpl_core_program,
            asset: &asset,
            collection: Some(&collection),
            payer: &seller,
            authority: Some(&seller),
            system_program: &system_program,
        })
    }

    #[instruction(discriminator = [96, 0, 28, 190, 49, 107, 83, 222])]
    pub fn buy_nft(ctx: Ctx<BuyNft>) -> Result<(), ProgramError> {
        let seller = ctx.accounts.listing.seller;
        let asset_address = ctx.accounts.listing.asset;
        let collection = ctx.accounts.listing.collection;
        let price = ctx.accounts.listing.price_lamports.get();

        validate_external_programs(
            &ctx.accounts.mpl_core_program,
            Some(&ctx.accounts.player_profile_program),
        )?;
        require_keys_eq!(
            asset_address,
            *ctx.accounts.asset.address(),
            MarketplaceError::InvalidAsset
        );
        require_keys_eq!(
            seller,
            *ctx.accounts.seller.address(),
            MarketplaceError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.company_treasury.address(),
            &ctx.accounts.marketplace_config.company_treasury,
            MarketplaceError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_pool.address(),
            &ctx.accounts.marketplace_config.gauntlet_pool,
            MarketplaceError::InvalidGauntletPool
        );
        require!(
            *ctx.accounts.buyer.address() != seller,
            MarketplaceError::CannotBuySelf
        );

        let bump = validate_mint_authority(&ctx.accounts.mint_authority)?;
        let company_fee =
            checked_fee(price, ctx.accounts.marketplace_config.company_fee_bps.get())?;
        let gauntlet_fee = checked_fee(
            price,
            ctx.accounts.marketplace_config.gauntlet_fee_bps.get(),
        )?;
        let seller_amount = checked_seller_amount(price, company_fee, gauntlet_fee)?;

        ctx.accounts
            .system_program
            .transfer(&ctx.accounts.buyer, &ctx.accounts.seller, seller_amount)
            .invoke()?;
        if company_fee > 0 {
            ctx.accounts
                .system_program
                .transfer(
                    &ctx.accounts.buyer,
                    &ctx.accounts.company_treasury,
                    company_fee,
                )
                .invoke()?;
        }
        if gauntlet_fee > 0 {
            ctx.accounts
                .system_program
                .transfer(
                    &ctx.accounts.buyer,
                    &ctx.accounts.gauntlet_pool,
                    gauntlet_fee,
                )
                .invoke()?;
        }

        let bump_bytes = [bump];
        let signer_seeds = [
            Seed::from(MINT_AUTHORITY_SEED),
            Seed::from(bump_bytes.as_ref()),
        ];
        let mpl_core_program = ctx.accounts.mpl_core_program.to_account_view();
        let asset = ctx.accounts.asset.to_account_view();
        let collection_view = ctx.accounts.collection.to_account_view();
        let buyer = ctx.accounts.buyer.to_account_view();
        let seller_view = ctx.accounts.seller.to_account_view();
        let mint_authority = ctx.accounts.mint_authority.to_account_view();
        let system_program = ctx.accounts.system_program.to_account_view();

        transfer_v1(
            TransferV1Accounts {
                program: &mpl_core_program,
                asset: &asset,
                collection: Some(&collection_view),
                payer: &buyer,
                authority: Some(&mint_authority),
                new_owner: &buyer,
            },
            &signer_seeds,
        )?;

        remove_transfer_delegate_plugin(RemovePluginV1Accounts {
            program: &mpl_core_program,
            asset: &asset,
            collection: Some(&collection_view),
            payer: &buyer,
            authority: Some(&buyer),
            system_program: &system_program,
        })?;

        if collection == ctx.accounts.marketplace_config.items_collection {
            let relic_asset_record = ctx
                .accounts
                .relic_asset_record
                .as_ref()
                .ok_or(MarketplaceError::InvalidAsset)?;
            require_keys_eq!(
                relic_asset_record.asset,
                *ctx.accounts.asset.address(),
                MarketplaceError::InvalidAsset
            );

            let player_profile_program = ctx.accounts.player_profile_program.to_account_view();
            if let Some(seller_player_relic_pool) = ctx.accounts.seller_player_relic_pool.as_ref() {
                let seller_relic_pool = seller_player_relic_pool.to_account_view();
                revoke_relic_from_owner(
                    &player_profile_program,
                    &seller_view,
                    &seller_relic_pool,
                    &mint_authority,
                    relic_asset_record.item_id,
                    &signer_seeds,
                )?;
            }

            let buyer_relic_pool = ctx.accounts.buyer_player_relic_pool.to_account_view();
            let rent = ctx.accounts.rent.to_account_view();
            grant_relic_to_owner(
                &player_profile_program,
                &buyer,
                &buyer,
                &buyer_relic_pool,
                &mint_authority,
                &rent,
                &system_program,
                relic_asset_record.item_id,
                &signer_seeds,
            )?;
        }

        Ok(())
    }

    #[instruction(discriminator = [112, 49, 32, 224, 255, 173, 5, 7])]
    #[allow(clippy::too_many_arguments)]
    pub fn create_quest(
        ctx: Ctx<CreateQuest>,
        quest_id: u16,
        quest_type: QuestType,
        objective_type: ObjectiveType,
        objective_count: u16,
        reward_type: RewardType,
        reward_data: [u8; 32],
        season: u8,
    ) -> Result<(), ProgramError> {
        require_keys_eq!(
            ctx.accounts.authority.address(),
            &ctx.accounts.marketplace_config.authority,
            MarketplaceError::Unauthorized
        );
        ctx.accounts
            .quest_definition
            .set_inner(state::QuestDefinitionInner {
                quest_id,
                quest_type,
                objective_type,
                objective_count,
                reward_type,
                reward_data,
                season,
                active: true,
                bump: ctx.bumps.quest_definition,
            });
        Ok(())
    }

    #[instruction(discriminator = [227, 152, 182, 25, 142, 11, 231, 72])]
    pub fn accept_quest(ctx: Ctx<AcceptQuest>, quest_id: u16) -> Result<(), ProgramError> {
        require!(
            ctx.accounts.quest_definition.quest_id.get() == quest_id,
            MarketplaceError::InvalidQuestType
        );
        require!(
            ctx.accounts.quest_definition.active.get(),
            MarketplaceError::QuestNotActive
        );
        let clock = Clock::get()?;
        ctx.accounts
            .quest_progress
            .set_inner(state::QuestProgressInner {
                player: *ctx.accounts.player.address(),
                quest_id: ctx.accounts.quest_definition.quest_id.get(),
                progress: 0,
                completed: false,
                claimed: false,
                last_reset: clock.unix_timestamp.get(),
                bump: ctx.bumps.quest_progress,
            });
        Ok(())
    }

    #[instruction(discriminator = [167, 204, 80, 200, 137, 62, 63, 207])]
    pub fn update_quest_progress(
        ctx: Ctx<UpdateQuestProgress>,
        quest_id: u16,
        increment: u16,
    ) -> Result<(), ProgramError> {
        require!(
            ctx.accounts.quest_definition.quest_id.get() == quest_id,
            MarketplaceError::InvalidQuestType
        );
        require_keys_eq!(
            ctx.accounts.authority.address(),
            &ctx.accounts.marketplace_config.authority,
            MarketplaceError::Unauthorized
        );
        require_keys_eq!(
            ctx.accounts.quest_progress.player,
            *ctx.accounts.player.address(),
            MarketplaceError::Unauthorized
        );
        require!(
            !ctx.accounts.quest_progress.completed.get(),
            MarketplaceError::QuestAlreadyCompleted
        );

        let progress = ctx
            .accounts
            .quest_progress
            .progress
            .get()
            .saturating_add(increment);
        ctx.accounts.quest_progress.progress = progress.into();
        if progress >= ctx.accounts.quest_definition.objective_count.get() {
            ctx.accounts.quest_progress.completed = true.into();
        }
        Ok(())
    }

    #[instruction(discriminator = [73, 123, 191, 206, 63, 127, 247, 12])]
    pub fn claim_quest_reward(
        ctx: Ctx<ClaimQuestReward>,
        quest_id: u16,
    ) -> Result<(), ProgramError> {
        require!(
            ctx.accounts.quest_definition.quest_id.get() == quest_id,
            MarketplaceError::InvalidQuestType
        );
        require_keys_eq!(
            ctx.accounts.quest_progress.player,
            *ctx.accounts.player.address(),
            MarketplaceError::Unauthorized
        );
        require!(
            ctx.accounts.quest_progress.completed.get(),
            MarketplaceError::QuestNotCompleted
        );
        require!(
            !ctx.accounts.quest_progress.claimed.get(),
            MarketplaceError::QuestRewardAlreadyClaimed
        );
        ctx.accounts.quest_progress.claimed = true.into();
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
    match nft_marketplace::__dispatch(ptr, instruction_data) {
        Ok(_) => 0,
        Err(error) => error.into(),
    }
}

#[derive(Accounts)]
pub struct InitializeMarketplace {
    #[account(mut)]
    pub authority: Signer,
    #[account(
        mut,
        init,
        payer = authority,
        address = MarketplaceConfig::seeds()
    )]
    pub marketplace_config: Account<MarketplaceConfig>,
    pub gauntlet_pool: UncheckedAccount,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct MintSkin {
    #[account(mut)]
    pub asset: Signer,
    #[account(mut)]
    pub collection: UncheckedAccount,
    pub marketplace_config: Account<MarketplaceConfig>,
    pub mint_authority: UncheckedAccount,
    #[account(mut)]
    pub payer: Signer,
    pub owner: UncheckedAccount,
    pub mpl_core_program: UncheckedAccount,
    pub log_wrapper: UncheckedAccount,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct MintNftItem {
    #[account(mut)]
    pub asset: Signer,
    #[account(mut)]
    pub collection: UncheckedAccount,
    pub marketplace_config: Account<MarketplaceConfig>,
    pub mint_authority: UncheckedAccount,
    #[account(mut)]
    pub payer: Signer,
    #[account(
        mut,
        init,
        payer = payer,
        address = RelicAsset::seeds(asset.address())
    )]
    pub relic_asset: Account<RelicAsset>,
    #[account(mut)]
    pub player_relic_pool: UncheckedAccount,
    pub owner: UncheckedAccount,
    pub mpl_core_program: UncheckedAccount,
    pub player_profile_program: UncheckedAccount,
    pub log_wrapper: UncheckedAccount,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct ListNft {
    #[account(
        mut,
        init,
        payer = seller,
        address = Listing::seeds(asset.address())
    )]
    pub listing: Account<Listing>,
    pub marketplace_config: Account<MarketplaceConfig>,
    pub mint_authority: UncheckedAccount,
    #[account(mut)]
    pub asset: UncheckedAccount,
    #[account(mut)]
    pub collection: UncheckedAccount,
    #[account(mut)]
    pub seller: Signer,
    pub player_profile: UncheckedAccount,
    pub mpl_core_program: UncheckedAccount,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct CancelListing {
    #[account(
        mut,
        close(dest = seller),
        address = Listing::seeds(asset.address())
    )]
    pub listing: Account<Listing>,
    #[account(mut)]
    pub asset: UncheckedAccount,
    #[account(mut)]
    pub collection: UncheckedAccount,
    #[account(mut)]
    pub seller: Signer,
    pub mpl_core_program: UncheckedAccount,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
pub struct BuyNft {
    #[account(
        mut,
        close(dest = seller),
        address = Listing::seeds(asset.address())
    )]
    pub listing: Account<Listing>,
    pub marketplace_config: Account<MarketplaceConfig>,
    pub mint_authority: UncheckedAccount,
    #[account(mut)]
    pub asset: UncheckedAccount,
    pub relic_asset_record: Option<Account<RelicAsset>>,
    #[account(mut)]
    pub collection: UncheckedAccount,
    #[account(mut)]
    pub buyer: Signer,
    #[account(mut)]
    pub seller: UncheckedAccount,
    #[account(mut)]
    pub seller_player_relic_pool: Option<UncheckedAccount>,
    #[account(mut)]
    pub buyer_player_relic_pool: UncheckedAccount,
    #[account(mut)]
    pub company_treasury: UncheckedAccount,
    #[account(mut)]
    pub gauntlet_pool: UncheckedAccount,
    pub mpl_core_program: UncheckedAccount,
    pub player_profile_program: UncheckedAccount,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct CreateQuest {
    #[account(
        mut,
        init,
        payer = authority,
        address = QuestDefinition::seeds(quest_id)
    )]
    pub quest_definition: Account<QuestDefinition>,
    pub marketplace_config: Account<MarketplaceConfig>,
    #[account(mut)]
    pub authority: Signer,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct AcceptQuest {
    #[account(address = QuestDefinition::seeds(quest_id))]
    pub quest_definition: Account<QuestDefinition>,
    #[account(
        mut,
        init,
        payer = player,
        address = QuestProgress::seeds(player.address(), quest_id)
    )]
    pub quest_progress: Account<QuestProgress>,
    #[account(mut)]
    pub player: Signer,
    pub rent: Sysvar<Rent>,
    pub system_program: Program<SystemProgram>,
}

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct UpdateQuestProgress {
    #[account(address = QuestDefinition::seeds(quest_id))]
    pub quest_definition: Account<QuestDefinition>,
    #[account(
        mut,
        address = QuestProgress::seeds(player.address(), quest_id)
    )]
    pub quest_progress: Account<QuestProgress>,
    pub marketplace_config: Account<MarketplaceConfig>,
    pub authority: Signer,
    pub player: UncheckedAccount,
}

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct ClaimQuestReward {
    #[account(address = QuestDefinition::seeds(quest_id))]
    pub quest_definition: Account<QuestDefinition>,
    #[account(
        mut,
        address = QuestProgress::seeds(player.address(), quest_id)
    )]
    pub quest_progress: Account<QuestProgress>,
    pub player: Signer,
}
