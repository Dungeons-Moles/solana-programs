use anchor_lang::prelude::*;
use anchor_lang::system_program;
use player_profile::program::PlayerProfile;

pub mod constants;
pub mod errors;
pub mod mpl_core_cpi;
pub mod state;

use constants::*;
use errors::MarketplaceError;
use mpl_core_cpi::*;
use state::*;

declare_id!("GLKxBpZ8hc7qzvD9VHAVsJEjHSu2JVp1HaPrGH4fpTci");

const SPL_NOOP_PROGRAM_ID: Pubkey = pubkey!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

fn grant_relic_to_owner<'info>(
    player_profile_program: &Program<'info, PlayerProfile>,
    player_relic_pool: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    payer: &Signer<'info>,
    marketplace_authority: &AccountInfo<'info>,
    system_program: &Program<'info, System>,
    relic_item_id: [u8; 8],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    player_profile::cpi::grant_relic_ownership(
        CpiContext::new_with_signer(
            player_profile_program.to_account_info(),
            player_profile::cpi::accounts::GrantRelicOwnership {
                player_relic_pool: player_relic_pool.clone(),
                owner: owner.clone(),
                payer: payer.to_account_info(),
                marketplace_authority: marketplace_authority.clone(),
                system_program: system_program.to_account_info(),
            },
            signer_seeds,
        ),
        relic_item_id,
    )
}

fn revoke_relic_from_owner<'info>(
    player_profile_program: &Program<'info, PlayerProfile>,
    player_relic_pool: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    marketplace_authority: &AccountInfo<'info>,
    relic_item_id: [u8; 8],
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    player_profile::cpi::revoke_relic_ownership(
        CpiContext::new_with_signer(
            player_profile_program.to_account_info(),
            player_profile::cpi::accounts::RevokeRelicOwnership {
                player_relic_pool: player_relic_pool.clone(),
                owner: owner.clone(),
                marketplace_authority: marketplace_authority.clone(),
            },
            signer_seeds,
        ),
        relic_item_id,
    )
}

#[program]
pub mod nft_marketplace {
    use super::*;

    /// One-time marketplace configuration setup.
    pub fn initialize_marketplace(
        ctx: Context<InitializeMarketplace>,
        skins_collection: Pubkey,
        items_collection: Pubkey,
    ) -> Result<()> {
        // Validate gauntlet_pool is the canonical gameplay-state PDA
        let (expected_pool, _) =
            Pubkey::find_program_address(&[GAUNTLET_POOL_VAULT_SEED], &GAMEPLAY_STATE_PROGRAM_ID);
        require_keys_eq!(
            ctx.accounts.gauntlet_pool.key(),
            expected_pool,
            MarketplaceError::InvalidGauntletPool
        );

        let config = &mut ctx.accounts.marketplace_config;
        config.authority = ctx.accounts.authority.key();
        config.skins_collection = skins_collection;
        config.items_collection = items_collection;
        config.company_treasury = COMPANY_TREASURY;
        config.gauntlet_pool = ctx.accounts.gauntlet_pool.key();
        config.company_fee_bps = DEFAULT_COMPANY_FEE_BPS;
        config.gauntlet_fee_bps = DEFAULT_GAUNTLET_FEE_BPS;

        // Validate total fees do not exceed 100%
        require!(
            (config.company_fee_bps as u64)
                .checked_add(config.gauntlet_fee_bps as u64)
                .ok_or(MarketplaceError::ArithmeticOverflow)?
                < BPS_DENOMINATOR,
            MarketplaceError::FeeTooHigh
        );

        config.bump = ctx.bumps.marketplace_config;
        Ok(())
    }

    /// Mint a skin NFT via CPI to Metaplex Core.
    /// Only callable by the mint_authority PDA (used by admin scripts or quest rewards).
    pub fn mint_skin(
        ctx: Context<MintSkin>,
        name: String,
        uri: String,
        _skin_id: u16,
        _season: u8,
        _rarity: u8,
    ) -> Result<()> {
        let bump = ctx.bumps.mint_authority;
        let signer_seeds: &[&[u8]] = &[b"mint_authority", &[bump]];

        create_v1(
            CreateV1Accounts {
                program: &ctx.accounts.mpl_core_program,
                asset: &ctx.accounts.asset,
                collection: Some(&ctx.accounts.collection),
                authority: Some(&ctx.accounts.mint_authority),
                payer: &ctx.accounts.payer,
                owner: Some(&ctx.accounts.owner),
                update_authority: None,
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: Some(&ctx.accounts.log_wrapper),
            },
            name,
            uri,
            &[signer_seeds],
        )?;

        Ok(())
    }

    /// Mint an NFT item via CPI to Metaplex Core.
    pub fn mint_nft_item(
        ctx: Context<MintNftItem>,
        name: String,
        uri: String,
        nft_item_id: [u8; 8],
    ) -> Result<()> {
        let bump = ctx.bumps.mint_authority;
        let signer_seeds: &[&[u8]] = &[b"mint_authority", &[bump]];

        create_v1(
            CreateV1Accounts {
                program: &ctx.accounts.mpl_core_program,
                asset: &ctx.accounts.asset,
                collection: Some(&ctx.accounts.collection),
                authority: Some(&ctx.accounts.mint_authority),
                payer: &ctx.accounts.payer,
                owner: Some(&ctx.accounts.owner),
                update_authority: None,
                system_program: &ctx.accounts.system_program.to_account_info(),
                log_wrapper: Some(&ctx.accounts.log_wrapper),
            },
            name,
            uri,
            &[signer_seeds],
        )?;

        let relic_asset = &mut ctx.accounts.relic_asset;
        relic_asset.asset = ctx.accounts.asset.key();
        relic_asset.item_id = nft_item_id;
        relic_asset.bump = ctx.bumps.relic_asset;

        grant_relic_to_owner(
            &ctx.accounts.player_profile_program,
            &ctx.accounts.player_relic_pool,
            &ctx.accounts.owner,
            &ctx.accounts.payer,
            &ctx.accounts.mint_authority,
            &ctx.accounts.system_program,
            nft_item_id,
            &[signer_seeds],
        )?;

        Ok(())
    }

    /// List an NFT for sale. Adds Transfer Delegate plugin so marketplace PDA can transfer.
    pub fn list_nft(ctx: Context<ListNft>, price_lamports: u64) -> Result<()> {
        require!(price_lamports > 0, MarketplaceError::InvalidPrice);
        require!(
            price_lamports >= MIN_LISTING_PRICE,
            MarketplaceError::PriceTooLow
        );

        // Validate asset is owned by seller by reading raw bytes.
        // Metaplex Core asset: byte 0 = Key discriminator (1 = AssetV1), bytes 1..33 = owner.
        let asset_data = ctx.accounts.asset.try_borrow_data()?;
        require!(asset_data.len() >= 33, MarketplaceError::InvalidAsset);
        require!(asset_data[0] == 1, MarketplaceError::InvalidAsset); // AssetV1

        let mut owner_bytes = [0u8; 32];
        owner_bytes.copy_from_slice(&asset_data[1..33]);
        let asset_owner = Pubkey::new_from_array(owner_bytes);
        require!(
            asset_owner == ctx.accounts.seller.key(),
            MarketplaceError::NotOwner
        );
        drop(asset_data);

        // Validate the asset belongs to one of our collections.
        let config = &ctx.accounts.marketplace_config;
        let collection_key = ctx.accounts.collection.key();
        require!(
            collection_key == config.skins_collection || collection_key == config.items_collection,
            MarketplaceError::InvalidCollection
        );

        // Block listing skins that are currently equipped on the player profile.
        // PlayerProfile layout (Borsh): 8 (disc) + 32 (owner) + 4+N (name String) +
        // 4 (total_runs) + 1 (highest_level) + 4 (available_runs) + 8 (created_at) +
        // 1 (bump) + 10 (unlocked_items) + 10 (active_item_pool) + 1+32 (equipped_skin)
        if collection_key == config.skins_collection {
            let profile_data = ctx.accounts.player_profile.try_borrow_data()?;
            if profile_data.len() > 44 {
                let name_len =
                    u32::from_le_bytes(profile_data[40..44].try_into().unwrap()) as usize;
                require!(name_len <= 32, MarketplaceError::InvalidAccountData);
                let equipped_offset = 44usize
                    .checked_add(name_len)
                    .and_then(|v| v.checked_add(4 + 1 + 4 + 8 + 1 + 10 + 10))
                    .ok_or(MarketplaceError::ArithmeticOverflow)?;
                if profile_data.len() > equipped_offset + 33 && profile_data[equipped_offset] == 1 {
                    let mut skin_bytes = [0u8; 32];
                    skin_bytes
                        .copy_from_slice(&profile_data[equipped_offset + 1..equipped_offset + 33]);
                    let equipped_pubkey = Pubkey::new_from_array(skin_bytes);
                    require!(
                        equipped_pubkey != ctx.accounts.asset.key(),
                        MarketplaceError::SkinCurrentlyEquipped
                    );
                }
            }
            drop(profile_data);
        }

        let clock = Clock::get()?;

        // Set up listing.
        let listing = &mut ctx.accounts.listing;
        listing.seller = ctx.accounts.seller.key();
        listing.asset = ctx.accounts.asset.key();
        listing.collection = collection_key;
        listing.price_lamports = price_lamports;
        listing.created_at = clock.unix_timestamp;
        listing.bump = ctx.bumps.listing;

        // Add Transfer Delegate plugin on the asset (seller as authority).
        add_transfer_delegate_plugin(AddPluginV1Accounts {
            program: &ctx.accounts.mpl_core_program,
            asset: &ctx.accounts.asset,
            collection: Some(&ctx.accounts.collection),
            payer: &ctx.accounts.seller,
            authority: Some(&ctx.accounts.seller),
            system_program: &ctx.accounts.system_program.to_account_info(),
        })?;

        // Approve the mint_authority PDA as the delegate so it can transfer during buy_nft.
        approve_plugin_authority(
            ApprovePluginAuthorityV1Accounts {
                program: &ctx.accounts.mpl_core_program,
                asset: &ctx.accounts.asset,
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.seller,
                authority: Some(&ctx.accounts.seller),
                system_program: &ctx.accounts.system_program.to_account_info(),
            },
            PluginType::TransferDelegate,
            PluginAuthority::Address {
                address: ctx.accounts.mint_authority.key(),
            },
        )?;

        Ok(())
    }

    /// Cancel a listing. Removes Transfer Delegate and closes listing account.
    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        // Remove Transfer Delegate plugin.
        remove_plugin(
            RemovePluginV1Accounts {
                program: &ctx.accounts.mpl_core_program,
                asset: &ctx.accounts.asset,
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.seller,
                authority: Some(&ctx.accounts.seller),
                system_program: &ctx.accounts.system_program.to_account_info(),
            },
            PluginType::TransferDelegate,
        )?;

        // Listing account is closed via the `close = seller` constraint.

        Ok(())
    }

    /// Buy a listed NFT. Transfers SOL (with fee split) and NFT.
    pub fn buy_nft(ctx: Context<BuyNft>) -> Result<()> {
        let listing = &ctx.accounts.listing;
        let config = &ctx.accounts.marketplace_config;

        require!(
            ctx.accounts.buyer.key() != listing.seller,
            MarketplaceError::CannotBuySelf
        );

        let price = listing.price_lamports;

        // Calculate fee split.
        let company_fee = price
            .checked_mul(config.company_fee_bps as u64)
            .ok_or(MarketplaceError::ArithmeticOverflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(MarketplaceError::ArithmeticOverflow)?;

        let gauntlet_fee = price
            .checked_mul(config.gauntlet_fee_bps as u64)
            .ok_or(MarketplaceError::ArithmeticOverflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(MarketplaceError::ArithmeticOverflow)?;

        let seller_amount = price
            .checked_sub(company_fee)
            .ok_or(MarketplaceError::ArithmeticOverflow)?
            .checked_sub(gauntlet_fee)
            .ok_or(MarketplaceError::ArithmeticOverflow)?;

        // Transfer SOL to seller.
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.seller.to_account_info(),
                },
            ),
            seller_amount,
        )?;

        // Transfer SOL to company treasury.
        if company_fee > 0 {
            system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    system_program::Transfer {
                        from: ctx.accounts.buyer.to_account_info(),
                        to: ctx.accounts.company_treasury.to_account_info(),
                    },
                ),
                company_fee,
            )?;
        }

        // Transfer SOL to gauntlet pool.
        if gauntlet_fee > 0 {
            system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    system_program::Transfer {
                        from: ctx.accounts.buyer.to_account_info(),
                        to: ctx.accounts.gauntlet_pool.to_account_info(),
                    },
                ),
                gauntlet_fee,
            )?;
        }

        // Transfer NFT from seller to buyer via mint_authority PDA (collection update authority).
        let bump = ctx.bumps.mint_authority;
        let signer_seeds: &[&[u8]] = &[b"mint_authority", &[bump]];

        transfer_v1(
            TransferV1Accounts {
                program: &ctx.accounts.mpl_core_program,
                asset: &ctx.accounts.asset,
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.buyer,
                authority: Some(&ctx.accounts.mint_authority),
                new_owner: &ctx.accounts.buyer.to_account_info(),
            },
            &[signer_seeds],
        )?;

        // Clean up TransferDelegate plugin so the buyer can re-list later.
        // After transfer, buyer is the new owner and can remove plugins.
        remove_plugin(
            RemovePluginV1Accounts {
                program: &ctx.accounts.mpl_core_program,
                asset: &ctx.accounts.asset,
                collection: Some(&ctx.accounts.collection),
                payer: &ctx.accounts.buyer,
                authority: Some(&ctx.accounts.buyer),
                system_program: &ctx.accounts.system_program.to_account_info(),
            },
            PluginType::TransferDelegate,
        )?;

        if listing.collection == config.items_collection {
            let relic_asset_record = ctx
                .accounts
                .relic_asset_record
                .as_ref()
                .ok_or(MarketplaceError::InvalidAsset)?;
            require_keys_eq!(
                relic_asset_record.asset,
                ctx.accounts.asset.key(),
                MarketplaceError::InvalidAsset
            );

            if let Some(seller_player_relic_pool) = ctx.accounts.seller_player_relic_pool.as_ref() {
                revoke_relic_from_owner(
                    &ctx.accounts.player_profile_program,
                    seller_player_relic_pool,
                    &ctx.accounts.seller,
                    &ctx.accounts.mint_authority,
                    relic_asset_record.item_id,
                    &[signer_seeds],
                )?;
            }

            grant_relic_to_owner(
                &ctx.accounts.player_profile_program,
                &ctx.accounts.buyer_player_relic_pool,
                &ctx.accounts.buyer.to_account_info(),
                &ctx.accounts.buyer,
                &ctx.accounts.mint_authority,
                &ctx.accounts.system_program,
                relic_asset_record.item_id,
                &[signer_seeds],
            )?;
        }

        // Listing account closed via `close = seller` constraint in BuyNft.

        Ok(())
    }

    // ========================================================================
    // Quest System Instructions
    // ========================================================================

    /// Admin creates a quest definition.
    #[allow(clippy::too_many_arguments)]
    pub fn create_quest(
        ctx: Context<CreateQuest>,
        quest_id: u16,
        quest_type: QuestType,
        objective_type: ObjectiveType,
        objective_count: u16,
        reward_type: RewardType,
        reward_data: [u8; 32],
        season: u8,
    ) -> Result<()> {
        let quest = &mut ctx.accounts.quest_definition;
        quest.quest_id = quest_id;
        quest.quest_type = quest_type;
        quest.objective_type = objective_type;
        quest.objective_count = objective_count;
        quest.reward_type = reward_type;
        quest.reward_data = reward_data;
        quest.season = season;
        quest.active = true;
        quest.bump = ctx.bumps.quest_definition;
        Ok(())
    }

    /// Player accepts a quest, creating their progress account.
    pub fn accept_quest(ctx: Context<AcceptQuest>, _quest_id: u16) -> Result<()> {
        let quest_def = &ctx.accounts.quest_definition;
        require!(quest_def.active, MarketplaceError::QuestNotActive);

        let progress = &mut ctx.accounts.quest_progress;
        let clock = Clock::get()?;
        progress.player = ctx.accounts.player.key();
        progress.quest_id = quest_def.quest_id;
        progress.progress = 0;
        progress.completed = false;
        progress.claimed = false;
        progress.last_reset = clock.unix_timestamp;
        progress.bump = ctx.bumps.quest_progress;
        Ok(())
    }

    /// Update quest progress. Only callable by the marketplace authority.
    pub fn update_quest_progress(
        ctx: Context<UpdateQuestProgress>,
        _quest_id: u16,
        increment: u16,
    ) -> Result<()> {
        let quest_def = &ctx.accounts.quest_definition;
        let progress = &mut ctx.accounts.quest_progress;

        require!(!progress.completed, MarketplaceError::QuestAlreadyCompleted);

        progress.progress = progress.progress.saturating_add(increment);

        if progress.progress >= quest_def.objective_count {
            progress.completed = true;
        }

        Ok(())
    }

    /// Player claims a completed quest reward.
    /// For hackathon: minting happens via separate mint_skin/mint_nft_item calls.
    /// This instruction just marks the quest as claimed.
    pub fn claim_quest_reward(ctx: Context<ClaimQuestReward>, _quest_id: u16) -> Result<()> {
        let progress = &mut ctx.accounts.quest_progress;
        require!(progress.completed, MarketplaceError::QuestNotCompleted);
        require!(
            !progress.claimed,
            MarketplaceError::QuestRewardAlreadyClaimed
        );

        progress.claimed = true;
        Ok(())
    }
}

// ============================================================================
// Account Contexts
// ============================================================================

#[derive(Accounts)]
pub struct InitializeMarketplace<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + MarketplaceConfig::INIT_SPACE,
        seeds = [MarketplaceConfig::SEED_PREFIX],
        bump
    )]
    pub marketplace_config: Account<'info, MarketplaceConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: Validated in handler against gameplay-state gauntlet pool vault PDA.
    pub gauntlet_pool: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintSkin<'info> {
    /// CHECK: New asset account, will be initialized by Metaplex Core.
    #[account(mut)]
    pub asset: Signer<'info>,

    /// CHECK: Validated against marketplace_config.skins_collection.
    #[account(
        mut,
        address = marketplace_config.skins_collection @ MarketplaceError::InvalidCollection
    )]
    pub collection: AccountInfo<'info>,

    #[account(
        seeds = [MarketplaceConfig::SEED_PREFIX],
        bump = marketplace_config.bump,
    )]
    pub marketplace_config: Account<'info, MarketplaceConfig>,

    /// Mint authority PDA -- update authority on both collections.
    /// CHECK: PDA derived from seeds.
    #[account(
        seeds = [b"mint_authority"],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,

    /// Admin who triggers the mint.
    #[account(
        mut,
        address = marketplace_config.authority @ MarketplaceError::Unauthorized
    )]
    pub payer: Signer<'info>,

    /// CHECK: The wallet that will own the minted NFT.
    pub owner: AccountInfo<'info>,

    /// CHECK: Metaplex Core program.
    #[account(address = MPL_CORE_PROGRAM_ID)]
    pub mpl_core_program: AccountInfo<'info>,

    /// CHECK: SPL Noop program used by mpl-core create.
    #[account(address = SPL_NOOP_PROGRAM_ID)]
    pub log_wrapper: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNftItem<'info> {
    /// CHECK: New asset account, will be initialized by Metaplex Core.
    #[account(mut)]
    pub asset: Signer<'info>,

    /// CHECK: Validated against marketplace_config.items_collection.
    #[account(
        mut,
        address = marketplace_config.items_collection @ MarketplaceError::InvalidCollection
    )]
    pub collection: AccountInfo<'info>,

    #[account(
        seeds = [MarketplaceConfig::SEED_PREFIX],
        bump = marketplace_config.bump,
    )]
    pub marketplace_config: Account<'info, MarketplaceConfig>,

    /// CHECK: PDA derived from seeds.
    #[account(
        seeds = [b"mint_authority"],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,

    #[account(
        mut,
        address = marketplace_config.authority @ MarketplaceError::Unauthorized
    )]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + RelicAsset::INIT_SPACE,
        seeds = [RelicAsset::SEED_PREFIX, asset.key().as_ref()],
        bump
    )]
    pub relic_asset: Account<'info, RelicAsset>,

    /// CHECK: Player relic pool PDA credited automatically on mint.
    #[account(mut)]
    pub player_relic_pool: AccountInfo<'info>,

    /// CHECK: The wallet that will own the minted NFT.
    pub owner: AccountInfo<'info>,

    /// CHECK: Metaplex Core program.
    #[account(address = MPL_CORE_PROGRAM_ID)]
    pub mpl_core_program: AccountInfo<'info>,

    pub player_profile_program: Program<'info, PlayerProfile>,

    /// CHECK: SPL Noop program used by mpl-core create.
    #[account(address = SPL_NOOP_PROGRAM_ID)]
    pub log_wrapper: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ListNft<'info> {
    #[account(
        init,
        payer = seller,
        space = 8 + Listing::INIT_SPACE,
        seeds = [Listing::SEED_PREFIX, asset.key().as_ref()],
        bump
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        seeds = [MarketplaceConfig::SEED_PREFIX],
        bump = marketplace_config.bump,
    )]
    pub marketplace_config: Account<'info, MarketplaceConfig>,

    /// CHECK: PDA used as delegate.
    #[account(
        seeds = [b"mint_authority"],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,

    /// CHECK: Metaplex Core asset account. Validated in handler.
    #[account(mut)]
    pub asset: AccountInfo<'info>,

    /// CHECK: Collection the asset belongs to. Validated against config.
    #[account(mut)]
    pub collection: AccountInfo<'info>,

    #[account(mut)]
    pub seller: Signer<'info>,

    /// CHECK: Player profile PDA. Read to check equipped_skin.
    #[account(
        seeds = [PLAYER_PROFILE_SEED, seller.key().as_ref()],
        bump,
        seeds::program = PLAYER_PROFILE_PROGRAM_ID,
    )]
    pub player_profile: AccountInfo<'info>,

    /// CHECK: Metaplex Core program.
    #[account(address = MPL_CORE_PROGRAM_ID)]
    pub mpl_core_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelListing<'info> {
    #[account(
        mut,
        seeds = [Listing::SEED_PREFIX, asset.key().as_ref()],
        bump = listing.bump,
        has_one = seller @ MarketplaceError::Unauthorized,
        has_one = asset,
        close = seller
    )]
    pub listing: Account<'info, Listing>,

    /// CHECK: Metaplex Core asset account.
    #[account(mut)]
    pub asset: AccountInfo<'info>,

    /// CHECK: Collection for the asset.
    #[account(mut)]
    pub collection: AccountInfo<'info>,

    #[account(mut)]
    pub seller: Signer<'info>,

    /// CHECK: Metaplex Core program.
    #[account(address = MPL_CORE_PROGRAM_ID)]
    pub mpl_core_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyNft<'info> {
    #[account(
        mut,
        seeds = [Listing::SEED_PREFIX, asset.key().as_ref()],
        bump = listing.bump,
        has_one = seller @ MarketplaceError::Unauthorized,
        has_one = asset,
        close = seller
    )]
    pub listing: Account<'info, Listing>,

    #[account(
        seeds = [MarketplaceConfig::SEED_PREFIX],
        bump = marketplace_config.bump,
    )]
    pub marketplace_config: Account<'info, MarketplaceConfig>,

    /// CHECK: PDA for transfer delegate signing.
    #[account(
        seeds = [b"mint_authority"],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,

    /// CHECK: Metaplex Core asset account.
    #[account(mut)]
    pub asset: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [RelicAsset::SEED_PREFIX, asset.key().as_ref()],
        bump,
    )]
    pub relic_asset_record: Option<Account<'info, RelicAsset>>,

    /// CHECK: Collection for the asset.
    #[account(mut)]
    pub collection: AccountInfo<'info>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: Seller wallet, receives payment.
    #[account(mut, address = listing.seller @ MarketplaceError::Unauthorized)]
    pub seller: AccountInfo<'info>,

    /// CHECK: Seller relic pool PDA, present for relic trades.
    #[account(mut)]
    pub seller_player_relic_pool: Option<AccountInfo<'info>>,

    /// CHECK: Buyer relic pool PDA, created on demand when buying a relic.
    #[account(mut)]
    pub buyer_player_relic_pool: AccountInfo<'info>,

    /// CHECK: Company treasury, receives fee.
    #[account(mut, address = marketplace_config.company_treasury)]
    pub company_treasury: AccountInfo<'info>,

    /// CHECK: Gauntlet pool, receives fee.
    #[account(mut, address = marketplace_config.gauntlet_pool)]
    pub gauntlet_pool: AccountInfo<'info>,

    /// CHECK: Metaplex Core program.
    #[account(address = MPL_CORE_PROGRAM_ID)]
    pub mpl_core_program: AccountInfo<'info>,

    pub player_profile_program: Program<'info, PlayerProfile>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Quest Account Contexts
// ============================================================================

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct CreateQuest<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + QuestDefinition::INIT_SPACE,
        seeds = [QuestDefinition::SEED_PREFIX, &quest_id.to_le_bytes()],
        bump
    )]
    pub quest_definition: Account<'info, QuestDefinition>,

    #[account(
        seeds = [MarketplaceConfig::SEED_PREFIX],
        bump = marketplace_config.bump,
    )]
    pub marketplace_config: Account<'info, MarketplaceConfig>,

    #[account(
        mut,
        address = marketplace_config.authority @ MarketplaceError::Unauthorized
    )]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct AcceptQuest<'info> {
    #[account(
        seeds = [QuestDefinition::SEED_PREFIX, &quest_id.to_le_bytes()],
        bump = quest_definition.bump,
    )]
    pub quest_definition: Account<'info, QuestDefinition>,

    #[account(
        init,
        payer = player,
        space = 8 + QuestProgress::INIT_SPACE,
        seeds = [QuestProgress::SEED_PREFIX, player.key().as_ref(), &quest_id.to_le_bytes()],
        bump
    )]
    pub quest_progress: Account<'info, QuestProgress>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct UpdateQuestProgress<'info> {
    #[account(
        seeds = [QuestDefinition::SEED_PREFIX, &quest_id.to_le_bytes()],
        bump = quest_definition.bump,
    )]
    pub quest_definition: Account<'info, QuestDefinition>,

    #[account(
        mut,
        seeds = [QuestProgress::SEED_PREFIX, player.key().as_ref(), &quest_id.to_le_bytes()],
        bump = quest_progress.bump,
        has_one = player,
    )]
    pub quest_progress: Account<'info, QuestProgress>,

    #[account(
        seeds = [MarketplaceConfig::SEED_PREFIX],
        bump = marketplace_config.bump,
    )]
    pub marketplace_config: Account<'info, MarketplaceConfig>,

    /// Authority that must match marketplace_config.authority.
    #[account(address = marketplace_config.authority @ MarketplaceError::Unauthorized)]
    pub authority: Signer<'info>,

    /// CHECK: Player whose quest progress is being updated. Not a signer —
    /// only the authority can advance quest progress.
    pub player: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(quest_id: u16)]
pub struct ClaimQuestReward<'info> {
    #[account(
        seeds = [QuestDefinition::SEED_PREFIX, &quest_id.to_le_bytes()],
        bump = quest_definition.bump,
    )]
    pub quest_definition: Account<'info, QuestDefinition>,

    #[account(
        mut,
        seeds = [QuestProgress::SEED_PREFIX, player.key().as_ref(), &quest_id.to_le_bytes()],
        bump = quest_progress.bump,
        has_one = player,
    )]
    pub quest_progress: Account<'info, QuestProgress>,

    pub player: Signer<'info>,
}
