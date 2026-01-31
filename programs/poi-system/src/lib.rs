use anchor_lang::prelude::*;

pub mod errors;
pub mod interactions;
pub mod offers;
pub mod pois;
pub mod spawn;
pub mod state;

use anchor_lang::context::CpiContext;
use errors::PoiSystemError;
use gameplay_state::state::GameState;
pub use pois::PoiDefinition;
use state::{ActiveCondition, MapPois, ShopState, UseType, MAP_POIS_SEED};

declare_id!("6E27r1Cyo2CNPvtRsonn3uHUAdznS3cMXEBX4HRbfBQY");

/// Seed for POI authority PDA used to sign CPI calls to gameplay-state
pub const POI_AUTHORITY_SEED: &[u8] = b"poi_authority";

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    217, 18, 17, 128, 79, 140, 152, 73, 103, 95, 134, 179, 31, 109, 34, 82, 250, 167, 91, 67, 186,
    23, 209, 2, 80, 255, 118, 192, 175, 242, 222, 183,
]);

/// Map generator program ID for reading GeneratedMap account
/// Must match the declare_id! in map-generator/src/lib.rs
/// BYdGuEGf8NqtLnHpSRuZFrPGEgvdxMfGfTt71QVBxYHa
pub const MAP_GENERATOR_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    156, 174, 227, 192, 77, 77, 237, 57, 57, 229, 227, 42, 100, 51, 52, 5,
    241, 68, 44, 141, 222, 59, 35, 223, 249, 8, 30, 121, 140, 38, 69, 149,
]);

/// Validates POI index, retrieves POI and definition, and validates interaction.
/// Use `skip_usage_check` for Repeatable/RepeatablePerTool POIs.
fn get_and_validate_poi<'a>(
    map_pois: &'a MapPois,
    game_state: &GameState,
    poi_index: u8,
    skip_usage_check: bool,
) -> Result<(&'a state::PoiInstance, pois::PoiDefinition)> {
    require!(
        (poi_index as usize) < map_pois.pois.len(),
        PoiSystemError::InvalidPoiIndex
    );

    let poi = &map_pois.pois[poi_index as usize];
    let poi_def = *pois::get_poi_definition(poi.poi_type).ok_or(PoiSystemError::InvalidPoiType)?;

    // Position check
    require!(
        game_state.position_x == poi.x && game_state.position_y == poi.y,
        PoiSystemError::PlayerNotOnPoiTile
    );

    // Usage check (unless skipped for repeatable POIs)
    if !skip_usage_check && poi_def.use_type == UseType::OneTime {
        require!(!poi.used, PoiSystemError::PoiAlreadyUsed);
    }

    // Time check
    if poi_def.active_condition == ActiveCondition::NightOnly {
        require!(game_state.phase.is_night(), PoiSystemError::NightOnlyPoi);
    }

    Ok((poi, poi_def))
}

/// Converts game_state.week (1-3) to boss_system::Week enum.
fn to_boss_week(week: u8) -> Result<boss_system::Week> {
    match week {
        1 => Ok(boss_system::Week::One),
        2 => Ok(boss_system::Week::Two),
        3 => Ok(boss_system::Week::Three),
        _ => Err(PoiSystemError::InvalidWeek.into()),
    }
}

#[program]
pub mod poi_system {
    use super::*;

    /// Initializes POI state for a session by copying POIs from the generated map.
    ///
    /// The generated_map account contains POIs placed during map generation.
    /// This instruction copies them to the MapPois account for runtime management.
    pub fn initialize_map_pois(
        ctx: Context<InitializeMapPois>,
        act: u8,
        week: u8,
        seed: u64,
    ) -> Result<()> {
        require_keys_eq!(
            *ctx.accounts.session.owner,
            SESSION_MANAGER_PROGRAM_ID,
            PoiSystemError::InvalidSessionOwner
        );
        require!((1..=4).contains(&act), PoiSystemError::InvalidAct);

        // Read POI data from the generated map account
        let generated_map_info = &ctx.accounts.generated_map;
        let generated_map_data = generated_map_info.try_borrow_data()?;

        // Validate minimum size: 8 (discriminator) + 32 (session) + basic fields
        require!(
            generated_map_data.len() >= 8 + 32 + 1 + 1 + 8 + 1 + 1 + 1 + 1 + 2 + 313 + 1 + (48 * 4) + 1,
            PoiSystemError::InvalidGeneratedMap
        );

        // Parse generated map fields:
        // Offset: 8 (discriminator) + 32 (session) + 1 (width) + 1 (height) + 8 (seed)
        //       + 1 (spawn_x) + 1 (spawn_y) + 1 (mole_den_x) + 1 (mole_den_y)
        //       + 2 (walkable_count) + 313 (packed_tiles) + 1 (enemy_count)
        //       + 192 (enemies: 48 * 4) = 562
        // poi_count is at offset 562
        let poi_count_offset = 8 + 32 + 1 + 1 + 8 + 1 + 1 + 1 + 1 + 2 + 313 + 1 + (48 * 4);
        let poi_count = generated_map_data[poi_count_offset] as usize;

        // POIs start at offset 563, each POI is 4 bytes: (poi_type, is_used, x, y)
        let pois_offset = poi_count_offset + 1;

        // Initialize the MapPois account
        let map_pois = &mut ctx.accounts.map_pois;
        map_pois.session = ctx.accounts.session.key();
        map_pois.bump = ctx.bumps.map_pois;
        map_pois.act = act;
        map_pois.week = week;
        map_pois.seed = seed;
        map_pois.shop_state = ShopState::default();

        // Copy POIs from generated map to MapPois
        let mut pois = Vec::with_capacity(poi_count);
        for i in 0..poi_count {
            let poi_start = pois_offset + (i * 4);
            if poi_start + 4 > generated_map_data.len() {
                break;
            }

            let poi_type = generated_map_data[poi_start];
            let is_used = generated_map_data[poi_start + 1] != 0;
            let x = generated_map_data[poi_start + 2];
            let y = generated_map_data[poi_start + 3];

            pois.push(state::PoiInstance {
                poi_type,
                x,
                y,
                used: is_used,
                discovered: poi_type == 1, // Mole Den (L1) is always discovered
                week_spawned: week,
            });
        }

        map_pois.count = pois.len() as u8;
        map_pois.pois = pois;

        emit!(PoisInitialized {
            session: map_pois.session,
            count: map_pois.count,
            act,
        });

        Ok(())
    }

    /// Close MapPois account, returning rent to the session owner.
    pub fn close_map_pois(ctx: Context<CloseMapPois>) -> Result<()> {
        // Verify the signer is the session owner by reading GameSession.player
        // (first 32 bytes after the 8-byte Anchor discriminator).
        let session_data = ctx.accounts.game_session.try_borrow_data()?;
        require!(session_data.len() >= 40, PoiSystemError::Unauthorized);
        let stored_player = Pubkey::from(
            <[u8; 32]>::try_from(&session_data[8..40]).unwrap(),
        );
        require!(
            stored_player == ctx.accounts.player.key(),
            PoiSystemError::Unauthorized
        );
        drop(session_data);

        emit!(PoisClosed {
            session: ctx.accounts.map_pois.session,
        });
        Ok(())
    }

    /// Get POI definition by type ID (view function).
    /// Returns POI properties for UI/client consumption.
    pub fn get_poi_definition(_ctx: Context<GetPoiDefinition>, poi_type: u8) -> Result<()> {
        let def = pois::get_poi_definition(poi_type).ok_or(PoiSystemError::InvalidPoiType)?;

        emit!(PoiDefinitionQueried {
            poi_type: def.id,
            name: def.name.to_string(),
            rarity: def.rarity as u8,
            use_type: def.use_type as u8,
            active_condition: def.active_condition as u8,
            interaction_type: def.interaction_type as u8,
            category: def.category as u8,
        });

        Ok(())
    }

    /// Interact with a rest POI (L1 Mole Den or L5 Rest Alcove).
    ///
    /// - L1: Full heal, repeatable, night-only, skip to day
    /// - L5: Heal 10 HP, one-time, night-only, skip to day
    ///
    /// This instruction validates the interaction, marks the POI as used (if applicable),
    /// heals the player, and skips to the next day phase via CPI to gameplay-state.
    /// If used during Night3, triggers the boss fight (cannot skip end-of-week boss).
    pub fn interact_rest(ctx: Context<InteractRest>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;
        let player_stats = gameplay_state::stats::calculate_stats(inventory);

        // Get values for rest interaction (i16 to handle HP > 255 or negative values)
        let current_hp = game_state.hp;
        let max_hp = player_stats.max_hp;
        let is_night = game_state.phase.is_night();

        // Execute rest interaction
        let result = interactions::execute_rest_interaction(poi, current_hp, max_hp, is_night)?;

        // Mark POI as used if needed
        if result.mark_used {
            map_pois.pois[poi_index as usize].used = true;
        }

        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];

        // CPI to gameplay-state to heal player atomically
        if result.heal_amount > 0 {
            gameplay_state::cpi::heal_player(
                CpiContext::new_with_signer(
                    ctx.accounts.gameplay_state_program.to_account_info(),
                    gameplay_state::cpi::accounts::HealPlayer {
                        game_state: ctx.accounts.game_state.to_account_info(),
                        inventory: ctx.accounts.inventory.to_account_info(),
                        poi_authority: ctx.accounts.poi_authority.to_account_info(),
                    },
                    &[&seeds[..]],
                ),
                result.heal_amount,
            )?;
        }

        // CPI to gameplay-state to skip to day (or trigger boss fight if Night3)
        gameplay_state::cpi::skip_to_day(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::SkipToDay {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    inventory: ctx.accounts.inventory.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                    player: ctx.accounts.player.to_account_info(),
                    player_inventory_program: ctx.accounts.player_inventory_program.to_account_info(),
                },
                &[&seeds[..]],
            ),
        )?;

        let poi = &map_pois.pois[poi_index as usize];
        emit!(RestCompleted {
            session: map_pois.session,
            poi_type: poi.poi_type,
            x: poi.x,
            y: poi.y,
            heal_amount: result.heal_amount,
            full_heal: result.full_heal,
        });

        Ok(())
    }

    /// Generate and store cache offers for a pick-item POI (L2, L3, L12, L13).
    ///
    /// This instruction generates offers using an on-chain derived seed (Clock)
    /// and stores them in `MapPois.current_offer` so the frontend can read them.
    /// The user then calls `interact_pick_item` with their chosen index.
    ///
    /// Must be called before `interact_pick_item` for pick-item POIs.
    pub fn generate_cache_offer(
        ctx: Context<InteractPickItem>,
        poi_index: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;
        let act = map_pois.act;

        // Derive seed from on-chain Clock to prevent client manipulation
        let clock = Clock::get()?;
        let session_bytes = map_pois.session.to_bytes();
        let seed = (clock.unix_timestamp as u64)
            ^ (poi_index as u64)
            ^ (act as u64) << 8
            ^ (session_bytes[0] as u64) << 16
            ^ (session_bytes[1] as u64) << 24;

        // Fetch boss weaknesses on-chain
        let week = to_boss_week(game_state.week)?;
        let weaknesses = boss_system::get_boss_weaknesses(game_state.campaign_level, week)
            .map_err(|_| PoiSystemError::InvalidBossWeek)?;
        let w1 = offers::WeaknessTag::try_from(weaknesses[0] as u8)
            .unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weaknesses[1] as u8)
            .unwrap_or(offers::WeaknessTag::Frost);

        let generated = offers::generate_poi_offers(poi_type, act, w1, w2, seed)
            .ok_or(PoiSystemError::InvalidInteraction)?;

        // Filter offers by active item pool from session
        let filtered = offers::filter_offers_by_pool(
            &generated.offers,
            &ctx.accounts.game_session.active_item_pool,
        );

        // Convert filtered offers to OfferItems for storage
        let mut items = [state::OfferItem::default(); 3];
        for (i, offer) in filtered.iter().take(3).enumerate() {
            items[i] = state::OfferItem {
                item_id: offer.item_id,
                rarity: offer.tier,
            };
        }

        map_pois.current_offer = Some(state::CacheOffer {
            poi_index,
            items,
            generated_at_seed: seed,
        });

        emit!(CacheOfferGenerated {
            session: map_pois.session,
            poi_index,
            poi_type,
            item0: items[0].item_id,
            item1: items[1].item_id,
            item2: items[2].item_id,
        });

        Ok(())
    }

    /// Interact with a pick-item POI (L2, L3, L12, L13).
    ///
    /// - L2 (Supply Cache): Pick 1 of 3 Gear
    /// - L3 (Tool Crate): Pick 1 of 3 Tools
    /// - L12 (Geode Vault): Pick 1 of 3 Heroic+ items
    /// - L13 (Counter Cache): Pick 1 of 3 weakness-tagged items
    ///
    /// Requires `generate_cache_offer` to have been called first.
    /// Reads the stored offers from `current_offer` and applies the user's choice.
    pub fn interact_pick_item(
        ctx: Context<InteractPickItem>,
        poi_index: u8,
        choice_index: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;
        let x = poi.x;
        let y = poi.y;
        let is_night = game_state.phase.is_night();

        // Read cached offers instead of regenerating
        let cached = map_pois.current_offer.ok_or(PoiSystemError::NoActiveInteraction)?;
        require!(cached.poi_index == poi_index, PoiSystemError::InvalidPoiIndex);

        // Convert cached OfferItems to ItemOffers for pick interaction
        let offers: Vec<state::ItemOffer> = cached
            .items
            .iter()
            .map(|item| state::ItemOffer {
                item_id: item.item_id,
                tier: item.rarity,
                price: 0,
                purchased: false,
            })
            .collect();

        // Execute pick interaction
        let poi = &map_pois.pois[poi_index as usize];
        let result = interactions::execute_pick_item_interaction(
            poi,
            &offers,
            choice_index,
            is_night,
        )?;

        // Mark POI as used
        if result.mark_used {
            map_pois.pois[poi_index as usize].used = true;
        }

        // Clear current offer after pick
        map_pois.current_offer = None;

        emit!(ItemPicked {
            session: map_pois.session,
            poi_type,
            item_id: result.item.item_id,
            tier: result.item.tier,
        });

        emit!(PoiInteracted {
            session: map_pois.session,
            poi_type,
            x,
            y,
            interaction: "pick_item".to_string(),
        });

        Ok(())
    }

    /// Generate and store oil offers for a Tool Oil Rack (L4).
    ///
    /// This instruction generates 3 of 4 possible oils using an on-chain derived seed
    /// and stores them in `MapPois.current_oil_offer` so the frontend can read them.
    /// The user then calls `interact_tool_oil` with their chosen oil.
    ///
    /// Must be called before `interact_tool_oil` for Tool Oil Rack POIs.
    pub fn generate_oil_offer(
        ctx: Context<InteractToolOil>,
        poi_index: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        // Validate POI (don't skip usage check - Tool Oil is one-time use)
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;

        // Validate it's a Tool Oil Rack (L4 = poi_type 4)
        require!(poi_type == 4, PoiSystemError::InvalidInteraction);

        // Derive seed from on-chain Clock to prevent client manipulation
        let clock = Clock::get()?;
        let session_bytes = map_pois.session.to_bytes();
        let seed = (clock.unix_timestamp as u64)
            ^ (poi_index as u64)
            ^ (session_bytes[0] as u64) << 16
            ^ (session_bytes[1] as u64) << 24;

        // Generate oil offer
        let oil_offer = offers::create_oil_offer(poi_index, seed);

        // Store in MapPois
        map_pois.current_oil_offer = Some(oil_offer);

        emit!(OilOfferGenerated {
            session: map_pois.session,
            poi_index,
            oils: oil_offer.oils,
        });

        Ok(())
    }

    /// Interact with a Tool Oil Rack (L4).
    ///
    /// Applies +1 to ATK, SPD, DIG, or ARM on the player's current tool.
    /// This is a one-time use POI - user picks one oil and the POI is consumed.
    ///
    /// Requires `generate_oil_offer` to have been called first.
    /// Validates the selected oil is one of the 3 generated offers.
    ///
    /// Arguments:
    /// - `poi_index`: Index of the POI in map_pois.pois
    /// - `current_oil_flags`: Current tool oil flags (unused, kept for API compat)
    /// - `modification`: Oil type (1=ATK, 2=SPD, 4=DIG, 8=ARM)
    pub fn interact_tool_oil(
        ctx: Context<InteractToolOil>,
        poi_index: u8,
        _current_oil_flags: u8,
        modification: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        // Validate POI (don't skip usage check - Tool Oil is one-time use)
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;
        let x = poi.x;
        let y = poi.y;
        let is_night = game_state.phase.is_night();

        // Read cached oil offer and validate selection
        let oil_offer = map_pois.current_oil_offer.ok_or(PoiSystemError::NoActiveInteraction)?;
        require!(oil_offer.poi_index == poi_index, PoiSystemError::InvalidPoiIndex);
        require!(
            offers::validate_oil_selection(&oil_offer, modification),
            PoiSystemError::InvalidOilSelection
        );

        // We don't check current_oil_flags here - player-inventory tracks that
        let result = interactions::execute_tool_oil_interaction(
            poi,
            0, // Not used for validation anymore
            modification,
            is_night,
        )?;

        // Mark POI as used (one-time use)
        map_pois.pois[poi_index as usize].used = true;

        // Clear oil offer after selection
        map_pois.current_oil_offer = None;

        emit!(ToolOilApplied {
            session: map_pois.session,
            modification: result.modification,
        });

        emit!(PoiInteracted {
            session: map_pois.session,
            poi_type,
            x,
            y,
            interaction: "tool_oil".to_string(),
        });

        Ok(())
    }

    // =========================================================================
    // Shop Instructions (L9 Smuggler Hatch)
    // =========================================================================

    /// Enter the Smuggler Hatch shop (L9).
    ///
    /// Generates 6 offers (1 Tool + 5 Gear) and starts a shopping session.
    /// Only one shop session can be active at a time.
    pub fn enter_shop(
        ctx: Context<EnterShop>,
        poi_index: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require!(
            !map_pois.shop_state.active,
            PoiSystemError::ShopAlreadyActive
        );

        // L9 is Repeatable, so skip usage check
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, true)?;

        let act = map_pois.act;
        let is_night = game_state.phase.is_night();

        interactions::validate_shop_poi(poi, is_night)?;

        // Derive seed from on-chain Clock to prevent client manipulation
        let clock = Clock::get()?;
        let session_bytes = map_pois.session.to_bytes();
        let seed = (clock.unix_timestamp as u64)
            ^ (poi_index as u64)
            ^ (act as u64) << 8
            ^ (session_bytes[0] as u64) << 16
            ^ (session_bytes[1] as u64) << 24;

        // Fetch boss weaknesses on-chain
        let week = to_boss_week(game_state.week)?;
        let weaknesses = boss_system::get_boss_weaknesses(game_state.campaign_level, week)
            .map_err(|_| PoiSystemError::InvalidBossWeek)?;
        let w1 = offers::WeaknessTag::try_from(weaknesses[0] as u8)
            .unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weaknesses[1] as u8)
            .unwrap_or(offers::WeaknessTag::Frost);

        let generated = offers::generate_smuggler_hatch_offers(act, w1, w2, seed);

        // Filter offers by active item pool from session
        let filtered = offers::filter_offers_by_pool(
            &generated.offers,
            &ctx.accounts.game_session.active_item_pool,
        );

        // Initialize shop state
        map_pois.shop_state.active = true;
        map_pois.shop_state.poi_index = poi_index;
        map_pois.shop_state.reroll_count = 0;

        // Copy filtered offers to shop state
        for (i, offer) in filtered.iter().enumerate() {
            if i < state::SHOP_OFFER_COUNT {
                map_pois.shop_state.offers[i] = *offer;
            }
        }

        emit!(ShopEntered {
            session: map_pois.session,
            poi_index,
        });

        Ok(())
    }

    /// Purchase an item from the active shop.
    ///
    /// Validates player has enough gold, marks the offer as purchased,
    /// and atomically deducts gold via CPI to gameplay-state.
    pub fn shop_purchase(ctx: Context<ShopPurchase>, offer_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;
        let player_gold = game_state.gold;

        // Execute purchase validation
        let (offer, price) =
            interactions::execute_shop_purchase(&map_pois.shop_state, offer_index, player_gold)?;

        // Mark offer as purchased
        map_pois.shop_state.offers[offer_index as usize].purchased = true;

        // CPI to gameplay-state to deduct gold atomically
        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            -(price as i16),
        )?;

        emit!(ItemPurchased {
            session: map_pois.session,
            item_id: offer.item_id,
            price,
        });

        Ok(())
    }

    /// Reroll the shop offers for a gold cost.
    ///
    /// Cost increases with each reroll: 4, 6, 8, 10, ...
    /// Gold is deducted atomically via CPI to gameplay-state.
    pub fn shop_reroll(ctx: Context<ShopReroll>) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;
        let player_gold = game_state.gold;

        // Calculate and validate reroll cost
        let cost = interactions::calculate_shop_reroll_cost(&map_pois.shop_state)?;

        require!(player_gold >= cost, PoiSystemError::InsufficientGold);

        // CPI to gameplay-state to deduct gold atomically
        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            -(cost as i16),
        )?;

        // Increment reroll count
        map_pois.shop_state.reroll_count = map_pois.shop_state.reroll_count.saturating_add(1);

        // Derive seed from on-chain Clock + reroll_count to prevent manipulation
        let clock = Clock::get()?;
        let session_bytes = map_pois.session.to_bytes();
        let seed = (clock.unix_timestamp as u64)
            ^ (map_pois.shop_state.reroll_count as u64)
            ^ (session_bytes[0] as u64) << 16
            ^ (session_bytes[1] as u64) << 24;

        // Fetch boss weaknesses on-chain
        let week = to_boss_week(game_state.week)?;
        let weaknesses = boss_system::get_boss_weaknesses(game_state.campaign_level, week)
            .map_err(|_| PoiSystemError::InvalidBossWeek)?;
        let w1 = offers::WeaknessTag::try_from(weaknesses[0] as u8)
            .unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weaknesses[1] as u8)
            .unwrap_or(offers::WeaknessTag::Frost);

        // Generate new offers
        let act = map_pois.act;

        let generated = offers::generate_smuggler_hatch_offers(act, w1, w2, seed);

        // Filter offers by active item pool from session
        let filtered = offers::filter_offers_by_pool(
            &generated.offers,
            &ctx.accounts.game_session.active_item_pool,
        );

        // Replace offers with filtered results
        // Reset all slots first, then fill with filtered offers
        map_pois.shop_state.offers = [state::ItemOffer::default(); state::SHOP_OFFER_COUNT];
        for (i, offer) in filtered.iter().enumerate() {
            if i < state::SHOP_OFFER_COUNT {
                map_pois.shop_state.offers[i] = *offer;
            }
        }

        emit!(ShopRerolled {
            session: map_pois.session,
            cost,
            reroll_count: map_pois.shop_state.reroll_count,
        });

        Ok(())
    }

    /// Exit the shop without purchasing.
    pub fn leave_shop(ctx: Context<LeaveShop>) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        require!(map_pois.shop_state.active, PoiSystemError::ShopNotActive);

        // Reset shop state
        map_pois.shop_state.active = false;
        map_pois.shop_state.reroll_count = 0;

        emit!(ShopExited {
            session: map_pois.session,
        });

        Ok(())
    }

    // =========================================================================
    // Upgrade Instructions (L10 Rusty Anvil, L11 Rune Kiln)
    // =========================================================================

    /// Upgrade a tool at the Rusty Anvil (L10).
    ///
    /// Tier I -> II costs 8 Gold, II -> III costs 16 Gold.
    /// POI is one-time use. Gold is deducted atomically via CPI.
    pub fn interact_rusty_anvil(
        ctx: Context<InteractRustyAnvil>,
        poi_index: u8,
        item_id: [u8; 8],
        current_tier: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let player_gold = game_state.gold;
        let is_night = game_state.phase.is_night();

        let result =
            interactions::execute_anvil_upgrade(poi, item_id, current_tier, player_gold, is_night)?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

        // CPI to gameplay-state to deduct gold atomically
        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            -(result.cost as i16),
        )?;

        emit!(ToolUpgraded {
            session: map_pois.session,
            item_id: result.item_id,
            old_tier: current_tier,
            new_tier: result.new_tier,
            cost: result.cost,
        });

        Ok(())
    }

    /// Fuse two identical items at the Rune Kiln (L11).
    ///
    /// Items must have the same ID and tier. Free to use.
    /// POI is repeatable.
    pub fn interact_rune_kiln(
        ctx: Context<InteractRuneKiln>,
        poi_index: u8,
        item1_id: [u8; 8],
        item1_tier: u8,
        item2_id: [u8; 8],
        item2_tier: u8,
    ) -> Result<()> {
        let map_pois = &ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        // L11 is Repeatable, so skip usage check
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, true)?;

        let is_night = game_state.phase.is_night();

        let result = interactions::execute_kiln_fusion(
            poi, item1_id, item1_tier, item2_id, item2_tier, is_night,
        )?;

        emit!(ItemsFused {
            session: map_pois.session,
            item_id: result.item_id,
            result_tier: result.result_tier,
        });

        Ok(())
    }

    // =========================================================================
    // Fast Travel Instructions (L8 Rail Waypoint)
    // =========================================================================

    /// Discover a Rail Waypoint (L8).
    ///
    /// On first visit, marks the waypoint as discovered for fast travel.
    /// Must be called when player first reaches a waypoint.
    pub fn discover_waypoint(ctx: Context<DiscoverWaypoint>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        // L8 is Repeatable, so skip usage check
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, true)?;

        let is_night = game_state.phase.is_night();
        let result = interactions::execute_waypoint_discover(poi, is_night)?;

        // Mark as discovered if new
        if result.newly_discovered {
            map_pois.pois[poi_index as usize].discovered = true;

            emit!(WaypointDiscovered {
                session: map_pois.session,
                x: result.x,
                y: result.y,
            });
        }

        Ok(())
    }

    /// Fast travel between Rail Waypoints (L8).
    ///
    /// Player must be at a discovered waypoint and select another discovered waypoint.
    /// The caller is responsible for updating player position via CPI.
    pub fn fast_travel(
        ctx: Context<FastTravel>,
        from_poi_index: u8,
        to_poi_index: u8,
    ) -> Result<()> {
        let map_pois = &ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        // Validate POI indices
        require!(
            (from_poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );
        require!(
            (to_poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let from_poi = &map_pois.pois[from_poi_index as usize];
        let to_poi = &map_pois.pois[to_poi_index as usize];

        // Validate player is at the from_poi
        require!(
            game_state.position_x == from_poi.x && game_state.position_y == from_poi.y,
            PoiSystemError::PlayerNotOnPoiTile
        );

        let is_night = game_state.phase.is_night();

        // Execute fast travel
        let result = interactions::execute_fast_travel(from_poi, to_poi, is_night)?;

        emit!(FastTravelCompleted {
            session: map_pois.session,
            from_x: result.from_x,
            from_y: result.from_y,
            to_x: result.to_x,
            to_y: result.to_y,
        });

        Ok(())
    }

    // =========================================================================
    // Map Reveal Instructions (L6 Survey Beacon, L7 Seismic Scanner)
    // =========================================================================

    /// Activate a Survey Beacon (L6).
    ///
    /// Reveals all tiles within radius 13 of the beacon.
    /// POI is one-time use.
    pub fn interact_survey_beacon(ctx: Context<InteractSurveyBeacon>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let map_width = game_state.map_width;
        let map_height = game_state.map_height;
        let is_night = game_state.phase.is_night();

        let result = interactions::execute_survey_beacon(poi, map_width, map_height, is_night)?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

        emit!(TilesRevealed {
            session: map_pois.session,
            count: result.tiles.len() as u16,
            center_x: result.center_x,
            center_y: result.center_y,
        });

        Ok(())
    }

    /// Activate a Seismic Scanner (L7).
    ///
    /// Reveals the nearest undiscovered POI of the selected category.
    /// POI is one-time use.
    pub fn interact_seismic_scanner(
        ctx: Context<InteractSeismicScanner>,
        poi_index: u8,
        category: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let cat = match category {
            0 => state::PoiCategory::Items,
            1 => state::PoiCategory::Upgrades,
            2 => state::PoiCategory::Utility,
            3 => state::PoiCategory::Shop,
            _ => return Err(PoiSystemError::InvalidInteraction.into()),
        };

        let pois_snapshot: Vec<_> = map_pois.pois.clone();
        let is_night = game_state.phase.is_night();

        // Execute seismic scanner
        let result = interactions::execute_seismic_scanner(
            poi,
            &pois_snapshot,
            poi_index as usize,
            cat,
            is_night,
        )?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

        // Mark revealed POI as discovered
        if let Some((revealed_idx, x, y)) = result.revealed_poi {
            map_pois.pois[revealed_idx].discovered = true;

            emit!(PoiRevealed {
                session: map_pois.session,
                poi_type: map_pois.pois[revealed_idx].poi_type,
                x,
                y,
            });
        }

        Ok(())
    }

    // =========================================================================
    // Scrap Chute Instruction (L14)
    // =========================================================================

    /// Scrap a gear item at the Scrap Chute (L14).
    ///
    /// Destroys one gear item for a gold cost (8-12 depending on act).
    /// POI is one-time use. Gold is deducted atomically via CPI.
    /// The caller is responsible for removing the item from inventory via CPI.
    pub fn interact_scrap_chute(
        ctx: Context<InteractScrapChute>,
        poi_index: u8,
        item_id: [u8; 8],
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;
        let act = map_pois.act;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let player_gold = game_state.gold;
        let is_night = game_state.phase.is_night();

        let result = interactions::execute_scrap_gear(poi, item_id, player_gold, act, is_night)?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

        // CPI to gameplay-state to deduct gold atomically
        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            -(result.cost as i16),
        )?;

        emit!(GearScrapped {
            session: map_pois.session,
            item_id: result.item_id,
            cost: result.cost,
        });

        Ok(())
    }
}

// =============================================================================
// Account Contexts
// =============================================================================

#[derive(Accounts)]
pub struct InitializeMapPois<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + MapPois::INIT_SPACE,
        seeds = [MAP_POIS_SEED, session.key().as_ref()],
        bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// The GameSession PDA (must exist)
    /// CHECK: We only verify this account exists as validation of the session
    pub session: AccountInfo<'info>,

    /// Generated map containing POIs to copy
    /// CHECK: Validated by owner check (must be owned by map-generator program)
    /// and PDA derivation (seeds = ["generated_map", session])
    #[account(
        owner = MAP_GENERATOR_PROGRAM_ID @ PoiSystemError::InvalidGeneratedMap
    )]
    pub generated_map: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseMapPois<'info> {
    #[account(
        mut,
        close = player,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// The session that owns this MapPois.
    /// CHECK: Validated by constraint (address match) and owner check (session-manager program).
    /// Player field is extracted from account data in the instruction body.
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::Unauthorized,
        owner = SESSION_MANAGER_PROGRAM_ID @ PoiSystemError::InvalidSessionOwner,
    )]
    pub game_session: AccountInfo<'info>,

    /// Session owner — must match GameSession.player (validated in instruction body).
    #[account(mut)]
    pub player: Signer<'info>,
}

/// Context for querying POI definition (view instruction)
#[derive(Accounts)]
pub struct GetPoiDefinition {}

/// Context for interacting with a rest POI (L1 or L5)
#[derive(Accounts)]
pub struct InteractRest<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation (mut for CPI)
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player's inventory for deriving max_hp and boss fight resolution (mut for gear slot expansion)
    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, player_inventory::state::PlayerInventory>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI (heal_player and skip_to_day)
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player inventory program for CPI (expand gear slots on boss victory via skip_to_day)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for interacting with a pick-item POI (L2, L3, L12, L13)
#[derive(Accounts)]
pub struct InteractPickItem<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Game session for active_item_pool filtering
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::InvalidSession,
    )]
    pub game_session: Account<'info, session_manager::state::GameSession>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for interacting with a Tool Oil Rack (L4)
#[derive(Accounts)]
pub struct InteractToolOil<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for entering the Smuggler Hatch shop (L9)
#[derive(Accounts)]
pub struct EnterShop<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Game session for active_item_pool filtering
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::InvalidSession,
    )]
    pub game_session: Account<'info, session_manager::state::GameSession>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for purchasing from the shop
#[derive(Accounts)]
pub struct ShopPurchase<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player making the purchase
    pub player: Signer<'info>,
}

/// Context for rerolling shop offers
#[derive(Accounts)]
pub struct ShopReroll<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Game session for active_item_pool filtering
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::InvalidSession,
    )]
    pub game_session: Account<'info, session_manager::state::GameSession>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player rerolling
    pub player: Signer<'info>,
}

/// Context for leaving the shop
#[derive(Accounts)]
pub struct LeaveShop<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player leaving
    pub player: Signer<'info>,
}

/// Context for upgrading at the Rusty Anvil (L10)
#[derive(Accounts)]
pub struct InteractRustyAnvil<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation and gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player upgrading
    pub player: Signer<'info>,
}

/// Context for fusing at the Rune Kiln (L11)
#[derive(Accounts)]
pub struct InteractRuneKiln<'info> {
    #[account(
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player fusing
    pub player: Signer<'info>,
}

/// Context for discovering a Rail Waypoint (L8)
#[derive(Accounts)]
pub struct DiscoverWaypoint<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player discovering
    pub player: Signer<'info>,
}

/// Context for fast travel between Rail Waypoints (L8)
#[derive(Accounts)]
pub struct FastTravel<'info> {
    #[account(
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player traveling
    pub player: Signer<'info>,
}

/// Context for activating a Survey Beacon (L6)
#[derive(Accounts)]
pub struct InteractSurveyBeacon<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player activating
    pub player: Signer<'info>,
}

/// Context for activating a Seismic Scanner (L7)
#[derive(Accounts)]
pub struct InteractSeismicScanner<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player activating
    pub player: Signer<'info>,
}

/// Context for scrapping gear at the Scrap Chute (L14)
#[derive(Accounts)]
pub struct InteractScrapChute<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation and gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Account<'info, GameState>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player scrapping
    pub player: Signer<'info>,
}

// =============================================================================
// Events
// =============================================================================

#[event]
pub struct PoisInitialized {
    pub session: Pubkey,
    pub count: u8,
    pub act: u8,
}

#[event]
pub struct PoisClosed {
    pub session: Pubkey,
}

#[event]
pub struct PoiInteracted {
    pub session: Pubkey,
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
    pub interaction: String,
}

#[event]
pub struct ItemPicked {
    pub session: Pubkey,
    pub poi_type: u8,
    pub item_id: [u8; 8],
    pub tier: u8,
}

#[event]
pub struct ToolOilApplied {
    pub session: Pubkey,
    pub modification: u8,
}

#[event]
pub struct TilesRevealed {
    pub session: Pubkey,
    pub count: u16,
    pub center_x: u8,
    pub center_y: u8,
}

#[event]
pub struct PoiRevealed {
    pub session: Pubkey,
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
}

#[event]
pub struct WaypointDiscovered {
    pub session: Pubkey,
    pub x: u8,
    pub y: u8,
}

#[event]
pub struct FastTravelCompleted {
    pub session: Pubkey,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
}

#[event]
pub struct ShopEntered {
    pub session: Pubkey,
    pub poi_index: u8,
}

#[event]
pub struct ItemPurchased {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub price: u16,
}

#[event]
pub struct ShopRerolled {
    pub session: Pubkey,
    pub cost: u16,
    pub reroll_count: u8,
}

#[event]
pub struct ShopExited {
    pub session: Pubkey,
}

#[event]
pub struct ToolUpgraded {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub old_tier: u8,
    pub new_tier: u8,
    pub cost: u16,
}

#[event]
pub struct ItemsFused {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub result_tier: u8,
}

#[event]
pub struct GearScrapped {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub cost: u16,
}

#[event]
pub struct PoiDefinitionQueried {
    pub poi_type: u8,
    pub name: String,
    pub rarity: u8,
    pub use_type: u8,
    pub active_condition: u8,
    pub interaction_type: u8,
    pub category: u8,
}

#[event]
pub struct RestCompleted {
    pub session: Pubkey,
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
    /// Heal amount (u16 to support max_hp > 255)
    pub heal_amount: u16,
    pub full_heal: bool,
}

/// Emitted when a cache offer is generated for pick-item POIs.
#[event]
pub struct CacheOfferGenerated {
    pub session: Pubkey,
    pub poi_index: u8,
    pub poi_type: u8,
    pub item0: [u8; 8],
    pub item1: [u8; 8],
    pub item2: [u8; 8],
}

/// Emitted when oil offers are generated for Tool Oil Rack (L4).
#[event]
pub struct OilOfferGenerated {
    pub session: Pubkey,
    pub poi_index: u8,
    /// The 3 oil flags offered (from OIL_FLAG_ATK=1, SPD=2, DIG=4, ARM=8)
    pub oils: [u8; 3],
}

/// The discriminator for initialize_map_pois instruction.
/// This is exported so other programs can validate their manual CPI discriminators.
/// Computed as sha256("global:initialize_map_pois")[..8].
///
/// IMPORTANT: If you rename the `initialize_map_pois` instruction, you must:
/// 1. Update this constant
/// 2. Update session-manager's INITIALIZE_MAP_POIS_DISCRIMINATOR constant
pub const INITIALIZE_MAP_POIS_DISCRIMINATOR: [u8; 8] =
    [0xa8, 0xec, 0xff, 0x37, 0xee, 0xd2, 0x19, 0xfb];

#[cfg(test)]
mod discriminator_tests {
    use super::*;

    /// Validates that INITIALIZE_MAP_POIS_DISCRIMINATOR matches sha256("global:initialize_map_pois")[..8].
    /// Computes the hash at test time so a rename is caught immediately.
    #[test]
    fn test_initialize_map_pois_discriminator() {
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(b"global:initialize_map_pois");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            INITIALIZE_MAP_POIS_DISCRIMINATOR, expected,
            "INITIALIZE_MAP_POIS_DISCRIMINATOR doesn't match sha256(\"global:initialize_map_pois\")[..8]"
        );
    }
}
