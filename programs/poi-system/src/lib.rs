use anchor_lang::prelude::*;

pub mod errors;
pub mod interactions;
pub mod offers;
pub mod pois;
pub mod spawn;
pub mod state;

use errors::PoiSystemError;
pub use pois::PoiDefinition;
use state::{MapPois, ShopState, MAP_POIS_SEED};

declare_id!("FJVnZE45hxcd7BJeci27BiTx23XD6inN4paiM2EkMaoB");

#[program]
pub mod poi_system {
    use super::*;

    /// Initializes POI state for a session.
    pub fn initialize_map_pois(
        ctx: Context<InitializeMapPois>,
        act: u8,
        week: u8,
        _seed: u64,
    ) -> Result<()> {
        require!((1..=4).contains(&act), PoiSystemError::InvalidAct);

        let map_pois = &mut ctx.accounts.map_pois;
        map_pois.session = ctx.accounts.session.key();
        map_pois.bump = ctx.bumps.map_pois;
        map_pois.count = 0;
        map_pois.act = act;
        map_pois.week = week;
        map_pois.pois = Vec::new();
        map_pois.shop_state = ShopState::default();

        emit!(PoisInitialized {
            session: map_pois.session,
            count: 0,
            act,
        });

        Ok(())
    }

    /// Close MapPois account, returning rent to payer.
    pub fn close_map_pois(_ctx: Context<CloseMapPois>) -> Result<()> {
        emit!(PoisClosed {
            session: _ctx.accounts.map_pois.session,
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
    /// - L1: Full heal, repeatable, night-only
    /// - L5: Heal 10 HP, one-time, night-only
    ///
    /// The instruction validates the interaction and emits a RestCompleted event.
    /// The caller is responsible for updating GameState HP via CPI.
    pub fn interact_rest(
        ctx: Context<InteractRest>,
        poi_index: u8,
        current_hp: u8,
        max_hp: u8,
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];

        // Execute rest interaction
        let result = interactions::execute_rest_interaction(poi, current_hp, max_hp, is_night)?;

        // Mark POI as used if needed
        if result.mark_used {
            map_pois.pois[poi_index as usize].used = true;
        }

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

    /// Interact with a pick-item POI (L2, L3, L12, L13).
    ///
    /// - L2 (Supply Cache): Pick 1 of 3 Gear
    /// - L3 (Tool Crate): Pick 1 of 3 Tools
    /// - L12 (Geode Vault): Pick 1 of 3 Heroic+ items
    /// - L13 (Counter Cache): Pick 1 of 3 weakness-tagged items
    ///
    /// The offers are generated deterministically from the seed.
    /// The instruction validates the interaction and emits an ItemPicked event.
    pub fn interact_pick_item(
        ctx: Context<InteractPickItem>,
        poi_index: u8,
        choice_index: u8,
        is_night: bool,
        weakness1: u8, // Boss weakness tag 1 (0-7)
        weakness2: u8, // Boss weakness tag 2 (0-7)
        seed: u64,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];
        let poi_type = poi.poi_type;
        let x = poi.x;
        let y = poi.y;
        let act = map_pois.act;

        // Generate offers for this POI
        let w1 = offers::WeaknessTag::try_from(weakness1).unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weakness2).unwrap_or(offers::WeaknessTag::Frost);

        let generated = offers::generate_poi_offers(poi_type, act, w1, w2, seed)
            .ok_or(PoiSystemError::InvalidInteraction)?;

        // Execute pick interaction
        let poi = &map_pois.pois[poi_index as usize];
        let result = interactions::execute_pick_item_interaction(
            poi,
            &generated.offers,
            choice_index,
            is_night,
        )?;

        // Mark POI as used
        if result.mark_used {
            map_pois.pois[poi_index as usize].used = true;
        }

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

    /// Interact with a Tool Oil Rack (L4).
    ///
    /// Applies +1 to ATK, SPD, or DIG on the player's current tool.
    /// Each oil type can only be applied once per tool (RepeatablePerTool).
    ///
    /// Arguments:
    /// - `poi_index`: Index of the POI in map_pois.pois
    /// - `current_oil_flags`: Current tool oil flags (tracked by client/gameplay-state)
    /// - `modification`: Oil type (1=ATK, 2=SPD, 4=DIG)
    /// - `is_night`: Whether it's currently night
    pub fn interact_tool_oil(
        ctx: Context<InteractToolOil>,
        poi_index: u8,
        current_oil_flags: u8,
        modification: u8,
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];
        let poi_type = poi.poi_type;
        let x = poi.x;
        let y = poi.y;

        // Execute tool oil interaction
        let result = interactions::execute_tool_oil_interaction(
            poi,
            current_oil_flags,
            modification,
            is_night,
        )?;

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
        is_night: bool,
        weakness1: u8,
        weakness2: u8,
        seed: u64,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Check no shop is already active
        require!(
            !map_pois.shop_state.active,
            PoiSystemError::ShopAlreadyActive
        );

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];
        let act = map_pois.act;

        // Validate this is a shop POI
        interactions::validate_shop_poi(poi, is_night)?;

        // Generate offers
        let w1 = offers::WeaknessTag::try_from(weakness1).unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weakness2).unwrap_or(offers::WeaknessTag::Frost);

        let generated = offers::generate_smuggler_hatch_offers(act, w1, w2, seed);

        // Initialize shop state
        map_pois.shop_state.active = true;
        map_pois.shop_state.poi_index = poi_index;
        map_pois.shop_state.reroll_count = 0;

        // Copy offers to shop state
        for (i, offer) in generated.offers.iter().enumerate() {
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
    /// Validates player has enough gold and marks the offer as purchased.
    /// The caller is responsible for updating player gold and inventory via CPI.
    pub fn shop_purchase(
        ctx: Context<ShopPurchase>,
        offer_index: u8,
        player_gold: u16,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Execute purchase validation
        let (offer, price) =
            interactions::execute_shop_purchase(&map_pois.shop_state, offer_index, player_gold)?;

        // Mark offer as purchased
        map_pois.shop_state.offers[offer_index as usize].purchased = true;

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
    /// The caller is responsible for deducting gold via CPI.
    pub fn shop_reroll(
        ctx: Context<ShopReroll>,
        player_gold: u16,
        weakness1: u8,
        weakness2: u8,
        seed: u64,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Calculate and validate reroll cost
        let cost = interactions::calculate_shop_reroll_cost(&map_pois.shop_state)?;

        require!(player_gold >= cost, PoiSystemError::InsufficientGold);

        // Increment reroll count
        map_pois.shop_state.reroll_count = map_pois.shop_state.reroll_count.saturating_add(1);

        // Generate new offers
        let act = map_pois.act;
        let w1 = offers::WeaknessTag::try_from(weakness1).unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weakness2).unwrap_or(offers::WeaknessTag::Frost);

        let generated = offers::generate_smuggler_hatch_offers(act, w1, w2, seed);

        // Replace offers
        for (i, offer) in generated.offers.iter().enumerate() {
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
    /// POI is one-time use.
    pub fn interact_rusty_anvil(
        ctx: Context<InteractRustyAnvil>,
        poi_index: u8,
        item_id: [u8; 8],
        current_tier: u8,
        player_gold: u16,
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];

        // Execute upgrade
        let result =
            interactions::execute_anvil_upgrade(poi, item_id, current_tier, player_gold, is_night)?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

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
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];

        // Execute fusion
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
    pub fn discover_waypoint(
        ctx: Context<DiscoverWaypoint>,
        poi_index: u8,
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];

        // Execute discover
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
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &ctx.accounts.map_pois;

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
    pub fn interact_survey_beacon(
        ctx: Context<InteractSurveyBeacon>,
        poi_index: u8,
        map_width: u8,
        map_height: u8,
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];

        // Execute survey beacon
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
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        // Parse category
        let cat = match category {
            0 => state::PoiCategory::Items,
            1 => state::PoiCategory::Upgrades,
            2 => state::PoiCategory::Utility,
            3 => state::PoiCategory::Shop,
            _ => return Err(PoiSystemError::InvalidInteraction.into()),
        };

        let poi = &map_pois.pois[poi_index as usize];
        let pois_snapshot: Vec<_> = map_pois.pois.clone();

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
    /// POI is one-time use.
    /// The caller is responsible for removing the item from inventory via CPI.
    pub fn interact_scrap_chute(
        ctx: Context<InteractScrapChute>,
        poi_index: u8,
        item_id: [u8; 8],
        player_gold: u16,
        is_night: bool,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let act = map_pois.act;

        // Validate POI index
        require!(
            (poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let poi = &map_pois.pois[poi_index as usize];

        // Execute scrap
        let result = interactions::execute_scrap_gear(poi, item_id, player_gold, act, is_night)?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

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

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseMapPois<'info> {
    #[account(
        mut,
        close = rent_destination,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    pub authority: Signer<'info>,

    /// CHECK: Rent destination can be any account
    #[account(mut)]
    pub rent_destination: AccountInfo<'info>,
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

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for interacting with a Tool Oil Rack (L4)
#[derive(Accounts)]
pub struct InteractToolOil<'info> {
    #[account(
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

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
    pub heal_amount: u8,
    pub full_heal: bool,
}
