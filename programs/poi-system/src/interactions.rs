//! POI interaction logic.
//!
//! Contains the core logic for interacting with each POI type.

use crate::errors::PoiSystemError;
use crate::pois::{get_poi_definition, PoiDefinition};
use crate::state::{ActiveCondition, InteractionType, ItemOffer, PoiInstance, UseType};

/// Validates that the player is standing on the POI tile.
/// Returns an error if player position doesn't match POI position.
pub fn validate_player_on_poi(
    player_x: u8,
    player_y: u8,
    poi: &PoiInstance,
) -> Result<(), PoiSystemError> {
    if player_x != poi.x || player_y != poi.y {
        return Err(PoiSystemError::PlayerNotOnPoiTile);
    }
    Ok(())
}

/// Heal amounts for rest POIs
pub const REST_PARTIAL_HEAL: u8 = 10;

/// Survey Beacon reveal radius
pub const SURVEY_BEACON_RADIUS: u8 = 13;

/// Tool oil modification flags
pub const OIL_FLAG_ATK: u8 = 0x01;
pub const OIL_FLAG_SPD: u8 = 0x02;
pub const OIL_FLAG_DIG: u8 = 0x04;
pub const OIL_FLAG_ARM: u8 = 0x08;

/// Result of a rest interaction
#[derive(Clone, Debug)]
pub struct RestResult {
    /// Amount healed (u16 to support max_hp > 255)
    pub heal_amount: u16,
    /// Whether this was a full heal (Mole Den)
    pub full_heal: bool,
    /// Whether the POI is now marked as used
    pub mark_used: bool,
}

/// Result of picking an item from a POI
#[derive(Clone, Debug)]
pub struct PickItemResult {
    /// The item offer that was picked
    pub item: ItemOffer,
    /// Whether the POI is now marked as used (always true for pick POIs)
    pub mark_used: bool,
}

/// Check if a POI can be interacted with based on its use type and current state
pub fn can_interact(
    poi: &PoiInstance,
    is_night: bool,
) -> Result<&'static PoiDefinition, PoiSystemError> {
    let def = get_poi_definition(poi.poi_type).ok_or(PoiSystemError::InvalidPoiType)?;

    // Check active condition
    if def.active_condition == ActiveCondition::NightOnly && !is_night {
        return Err(PoiSystemError::NightOnlyPoi);
    }

    // Check if one-time POI already used
    if def.use_type == UseType::OneTime && poi.used {
        return Err(PoiSystemError::PoiAlreadyUsed);
    }

    Ok(def)
}

/// Execute rest interaction for L1 (Mole Den) or L5 (Rest Alcove).
///
/// - L1 (Mole Den): Full heal, repeatable, night-only
/// - L5 (Rest Alcove): Heal 10 HP, one-time, night-only
///
/// Returns: RestResult with heal amount and whether to mark POI as used
///
/// Note: HP values are i16 to match GameState.hp and PlayerStats.max_hp.
/// Negative current_hp is treated as 0 for healing calculations.
pub fn execute_rest_interaction(
    poi: &PoiInstance,
    current_hp: i16,
    max_hp: i16,
    is_night: bool,
) -> Result<RestResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    // Clamp current_hp to 0 if negative (shouldn't happen, but be safe)
    let effective_hp = current_hp.max(0) as u16;
    // Clamp max_hp to 0 if negative (shouldn't happen, but be safe)
    let effective_max_hp = max_hp.max(0) as u16;

    match def.interaction_type {
        InteractionType::RestFull => {
            // L1: Mole Den - full heal
            let heal_amount = effective_max_hp.saturating_sub(effective_hp);
            Ok(RestResult {
                heal_amount,
                full_heal: true,
                mark_used: false, // Mole Den is repeatable
            })
        }
        InteractionType::RestPartial => {
            // L5: Rest Alcove - heal 10 HP
            let missing_hp = effective_max_hp.saturating_sub(effective_hp);
            let heal_amount = (REST_PARTIAL_HEAL as u16).min(missing_hp);
            Ok(RestResult {
                heal_amount,
                full_heal: false,
                mark_used: true, // Rest Alcove is one-time
            })
        }
        _ => Err(PoiSystemError::InvalidInteraction),
    }
}

/// Execute pick item interaction for L2, L3, L12, L13.
///
/// - L2 (Supply Cache): Pick 1 of 3 Gear
/// - L3 (Tool Crate): Pick 1 of 3 Tools
/// - L12 (Geode Vault): Pick 1 of 3 Heroic+ items
/// - L13 (Counter Cache): Pick 1 of 3 weakness-tagged items
///
/// All pick-item POIs are one-time use.
///
/// Arguments:
/// - `poi`: The POI instance
/// - `offers`: Generated item offers for this POI
/// - `choice_index`: Which offer the player chooses (0-2)
/// - `is_night`: Whether it's currently night
pub fn execute_pick_item_interaction(
    poi: &PoiInstance,
    offers: &[ItemOffer],
    choice_index: u8,
    is_night: bool,
) -> Result<PickItemResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    // Validate interaction type
    if def.interaction_type != InteractionType::PickItem {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Validate choice index
    if choice_index as usize >= offers.len() {
        return Err(PoiSystemError::InvalidOfferIndex);
    }

    let item = offers[choice_index as usize];

    Ok(PickItemResult {
        item,
        mark_used: true, // All pick-item POIs are one-time
    })
}

/// Check if a specific tool oil has already been applied
pub fn is_oil_applied(tool_oil_flags: u8, modification: u8) -> bool {
    (tool_oil_flags & modification) != 0
}

/// Result of applying tool oil
#[derive(Clone, Debug)]
pub struct ToolOilResult {
    /// The modification applied (ATK, SPD, or DIG flag)
    pub modification: u8,
    /// New tool oil flags after applying
    pub new_flags: u8,
}

/// Execute tool oil interaction for L4 (Tool Oil Rack).
///
/// Tool Oil Rack is RepeatablePerTool - each oil type can only be applied once per tool.
/// The player can choose between +1 ATK, +1 SPD, or +1 DIG.
///
/// Arguments:
/// - `poi`: The POI instance
/// - `current_oil_flags`: Current tool oil flags (which oils have been applied)
/// - `modification`: The oil to apply (OIL_FLAG_ATK, OIL_FLAG_SPD, or OIL_FLAG_DIG)
/// - `is_night`: Whether it's currently night
pub fn execute_tool_oil_interaction(
    poi: &PoiInstance,
    current_oil_flags: u8,
    modification: u8,
    is_night: bool,
) -> Result<ToolOilResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    // Validate interaction type
    if def.interaction_type != InteractionType::ToolOil {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Validate modification is a valid oil flag
    if modification != OIL_FLAG_ATK
        && modification != OIL_FLAG_SPD
        && modification != OIL_FLAG_DIG
        && modification != OIL_FLAG_ARM
    {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Check if this oil has already been applied
    if is_oil_applied(current_oil_flags, modification) {
        return Err(PoiSystemError::OilAlreadyApplied);
    }

    // Apply the oil
    let new_flags = current_oil_flags | modification;

    Ok(ToolOilResult {
        modification,
        new_flags,
    })
}

// =============================================================================
// Shop Interactions (L9 Smuggler Hatch)
// =============================================================================

use crate::offers::{calculate_price, ItemRarity, ItemType as OfferItemType};
use crate::state::ShopState;

/// Validate that a POI can be used as a shop (L9 Smuggler Hatch)
pub fn validate_shop_poi(
    poi: &PoiInstance,
    is_night: bool,
) -> Result<&'static PoiDefinition, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    if def.interaction_type != InteractionType::Shop {
        return Err(PoiSystemError::InvalidInteraction);
    }

    Ok(def)
}

/// Get the price for an offer based on its item ID.
///
/// Looks up the authoritative rarity from the player-inventory item registry
/// to determine the correct price.
pub fn get_offer_price(item_id: &[u8; 8]) -> u16 {
    // Determine item type from ID prefix (T- = Tool, G- = Gear)
    let item_type = if item_id[0] == b'T' {
        OfferItemType::Tool
    } else {
        OfferItemType::Gear
    };

    // Look up rarity from the authoritative item definition
    let rarity_u8 = crate::offers::rarity_from_item_id(item_id);
    let rarity = match rarity_u8 {
        0 => ItemRarity::Common,
        1 => ItemRarity::Rare,
        2 => ItemRarity::Heroic,
        3 => ItemRarity::Mythic,
        _ => ItemRarity::Common,
    };

    calculate_price(item_type, rarity)
}

/// Validate and execute a shop purchase
pub fn execute_shop_purchase(
    shop_state: &ShopState,
    offer_index: u8,
    player_gold: u16,
) -> Result<(ItemOffer, u16), PoiSystemError> {
    // Validate shop is active
    if !shop_state.active {
        return Err(PoiSystemError::ShopNotActive);
    }

    // Validate offer index
    if offer_index as usize >= shop_state.offers.len() {
        return Err(PoiSystemError::InvalidOfferIndex);
    }

    let offer = &shop_state.offers[offer_index as usize];

    // Check if already purchased
    if offer.purchased {
        return Err(PoiSystemError::OfferAlreadyPurchased);
    }

    // Check gold
    if player_gold < offer.price {
        return Err(PoiSystemError::InsufficientGold);
    }

    Ok((*offer, offer.price))
}

/// Calculate reroll cost
pub fn calculate_shop_reroll_cost(shop_state: &ShopState) -> Result<u16, PoiSystemError> {
    if !shop_state.active {
        return Err(PoiSystemError::ShopNotActive);
    }

    Ok(shop_state.reroll_cost())
}

// =============================================================================
// Upgrade Interactions (L10 Rusty Anvil, L11 Rune Kiln)
// =============================================================================

use crate::offers::calculate_anvil_cost;

/// Result of anvil upgrade
#[derive(Clone, Debug)]
pub struct AnvilUpgradeResult {
    /// Item ID being upgraded
    pub item_id: [u8; 8],
    /// New tier after upgrade
    pub new_tier: u8,
    /// Gold cost paid
    pub cost: u16,
}

/// Execute Rusty Anvil upgrade (L10).
///
/// Upgrades a tool tier: I -> II costs 8 Gold, II -> III costs 16 Gold.
/// POI is one-time use.
pub fn execute_anvil_upgrade(
    poi: &PoiInstance,
    item_id: [u8; 8],
    current_tier: u8,
    player_gold: u16,
    is_night: bool,
) -> Result<AnvilUpgradeResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    if def.interaction_type != InteractionType::Upgrade {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Check tier is upgradeable
    if current_tier >= 3 {
        return Err(PoiSystemError::AlreadyMaxTier);
    }

    // Get upgrade cost
    let cost = calculate_anvil_cost(current_tier).ok_or(PoiSystemError::AlreadyMaxTier)?;

    // Check gold
    if player_gold < cost {
        return Err(PoiSystemError::InsufficientGold);
    }

    Ok(AnvilUpgradeResult {
        item_id,
        new_tier: current_tier + 1,
        cost,
    })
}

/// Result of kiln fusion
#[derive(Clone, Debug)]
pub struct KilnFusionResult {
    /// Item ID being fused
    pub item_id: [u8; 8],
    /// New tier after fusion
    pub result_tier: u8,
}

/// Execute Rune Kiln fusion (L11).
///
/// Fuses two identical items (same ID and tier) into a higher tier (free).
/// POI is repeatable.
pub fn execute_kiln_fusion(
    poi: &PoiInstance,
    item1_id: [u8; 8],
    item1_tier: u8,
    item2_id: [u8; 8],
    item2_tier: u8,
    is_night: bool,
) -> Result<KilnFusionResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    if def.interaction_type != InteractionType::Fuse {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Check items are identical
    if item1_id != item2_id || item1_tier != item2_tier {
        return Err(PoiSystemError::ItemsNotIdentical);
    }

    // Check tier is fuseable
    if item1_tier >= 3 {
        return Err(PoiSystemError::AlreadyMaxTier);
    }

    Ok(KilnFusionResult {
        item_id: item1_id,
        result_tier: item1_tier + 1,
    })
}

/// Calculate tiles revealed by Survey Beacon (radius 13)
pub fn calculate_revealed_tiles(
    center_x: u8,
    center_y: u8,
    map_width: u8,
    map_height: u8,
) -> Vec<(u8, u8)> {
    let mut tiles = Vec::new();
    let radius = SURVEY_BEACON_RADIUS as i16;

    for dx in -radius..=radius {
        for dy in -radius..=radius {
            // Check if within radius (Manhattan distance)
            if dx.abs() + dy.abs() <= radius {
                let x = center_x as i16 + dx;
                let y = center_y as i16 + dy;

                // Check bounds
                if x >= 0 && x < map_width as i16 && y >= 0 && y < map_height as i16 {
                    tiles.push((x as u8, y as u8));
                }
            }
        }
    }

    tiles
}

/// Find the nearest POI of a specific type from a position
pub fn find_nearest_poi_of_type(
    pois: &[PoiInstance],
    from_x: u8,
    from_y: u8,
    poi_type: u8,
    exclude_used: bool,
) -> Option<(usize, &PoiInstance)> {
    let mut nearest: Option<(usize, &PoiInstance, u16)> = None;

    for (idx, poi) in pois.iter().enumerate() {
        if poi.poi_type != poi_type {
            continue;
        }

        if exclude_used && poi.used {
            continue;
        }

        // Calculate Manhattan distance
        let dx = (poi.x as i16 - from_x as i16).unsigned_abs();
        let dy = (poi.y as i16 - from_y as i16).unsigned_abs();
        let distance = dx + dy;

        // Skip if same position
        if distance == 0 {
            continue;
        }

        match nearest {
            Some((_, _, d)) if distance < d => {
                nearest = Some((idx, poi, distance));
            }
            None => {
                nearest = Some((idx, poi, distance));
            }
            _ => {}
        }
    }

    nearest.map(|(idx, poi, _)| (idx, poi))
}

/// Get all discovered Rail Waypoints except the current one
pub fn get_available_waypoint_destinations(
    pois: &[PoiInstance],
    current_index: usize,
) -> Vec<(usize, &PoiInstance)> {
    pois.iter()
        .enumerate()
        .filter(|(idx, poi)| {
            poi.poi_type == 8 // Rail Waypoint
                && poi.discovered
                && *idx != current_index
        })
        .collect()
}

// =============================================================================
// Fast Travel Interactions (L8 Rail Waypoint)
// =============================================================================

/// Result of discovering a waypoint
#[derive(Clone, Debug)]
pub struct WaypointDiscoverResult {
    /// Whether the waypoint was newly discovered (false if already discovered)
    pub newly_discovered: bool,
    /// X coordinate of the waypoint
    pub x: u8,
    /// Y coordinate of the waypoint
    pub y: u8,
}

/// Result of fast travel
#[derive(Clone, Debug)]
pub struct FastTravelResult {
    /// Starting X coordinate
    pub from_x: u8,
    /// Starting Y coordinate
    pub from_y: u8,
    /// Destination X coordinate
    pub to_x: u8,
    /// Destination Y coordinate
    pub to_y: u8,
}

/// Validate a POI is a Rail Waypoint
pub fn validate_rail_waypoint_poi(
    poi: &PoiInstance,
    is_night: bool,
) -> Result<&'static PoiDefinition, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    if def.interaction_type != InteractionType::FastTravel {
        return Err(PoiSystemError::InvalidInteraction);
    }

    Ok(def)
}

/// Execute discovering a Rail Waypoint (L8).
///
/// On first visit, marks the waypoint as discovered for fast travel.
/// Returns whether this was a new discovery.
pub fn execute_waypoint_discover(
    poi: &PoiInstance,
    is_night: bool,
) -> Result<WaypointDiscoverResult, PoiSystemError> {
    let _def = validate_rail_waypoint_poi(poi, is_night)?;

    Ok(WaypointDiscoverResult {
        newly_discovered: !poi.discovered,
        x: poi.x,
        y: poi.y,
    })
}

/// Execute fast travel between Rail Waypoints (L8).
///
/// Player must be at a discovered waypoint and travel to another discovered waypoint.
pub fn execute_fast_travel(
    from_poi: &PoiInstance,
    to_poi: &PoiInstance,
    is_night: bool,
) -> Result<FastTravelResult, PoiSystemError> {
    // Validate source is a Rail Waypoint and player can interact
    let _def = validate_rail_waypoint_poi(from_poi, is_night)?;

    // Validate source is discovered
    if !from_poi.discovered {
        return Err(PoiSystemError::DestinationNotDiscovered);
    }

    // Validate destination is a Rail Waypoint
    if to_poi.poi_type != 8 {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Validate destination is discovered
    if !to_poi.discovered {
        return Err(PoiSystemError::DestinationNotDiscovered);
    }

    // Cannot travel to same location
    if from_poi.x == to_poi.x && from_poi.y == to_poi.y {
        return Err(PoiSystemError::InvalidInteraction);
    }

    Ok(FastTravelResult {
        from_x: from_poi.x,
        from_y: from_poi.y,
        to_x: to_poi.x,
        to_y: to_poi.y,
    })
}

// =============================================================================
// Map Reveal Interactions (L6 Survey Beacon, L7 Seismic Scanner)
// =============================================================================

use crate::state::PoiCategory;

/// Result of Survey Beacon activation
#[derive(Clone, Debug)]
pub struct SurveyBeaconResult {
    /// Tiles revealed (list of coordinates)
    pub tiles: Vec<(u8, u8)>,
    /// Center X coordinate (beacon position)
    pub center_x: u8,
    /// Center Y coordinate (beacon position)
    pub center_y: u8,
}

/// Result of Seismic Scanner activation
#[derive(Clone, Debug)]
pub struct SeismicScannerResult {
    /// The POI that was revealed (if found)
    pub revealed_poi: Option<(usize, u8, u8)>, // (index, x, y)
    /// The category that was scanned
    pub category: PoiCategory,
}

/// Execute Survey Beacon interaction (L6).
///
/// Reveals all tiles within radius 13 of the beacon position.
/// POI is one-time use.
pub fn execute_survey_beacon(
    poi: &PoiInstance,
    map_width: u8,
    map_height: u8,
    is_night: bool,
) -> Result<SurveyBeaconResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    if def.interaction_type != InteractionType::MapRevealRadius {
        return Err(PoiSystemError::InvalidInteraction);
    }

    let tiles = calculate_revealed_tiles(poi.x, poi.y, map_width, map_height);

    Ok(SurveyBeaconResult {
        tiles,
        center_x: poi.x,
        center_y: poi.y,
    })
}

/// Execute Seismic Scanner interaction (L7).
///
/// Reveals the nearest undiscovered POI of the selected category.
/// POI is one-time use.
pub fn execute_seismic_scanner(
    poi: &PoiInstance,
    pois: &[PoiInstance],
    poi_index: usize,
    category: PoiCategory,
    is_night: bool,
) -> Result<SeismicScannerResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    if def.interaction_type != InteractionType::MapRevealNearest {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Find POIs matching the category
    let matching_pois: Vec<(usize, &PoiInstance)> = pois
        .iter()
        .enumerate()
        .filter(|(idx, p)| {
            // Skip the scanner itself
            if *idx == poi_index {
                return false;
            }
            // Skip already discovered POIs
            if p.discovered {
                return false;
            }
            // Check category
            if let Some(p_def) = get_poi_definition(p.poi_type) {
                p_def.category == category
            } else {
                false
            }
        })
        .collect();

    if matching_pois.is_empty() {
        return Ok(SeismicScannerResult {
            revealed_poi: None,
            category,
        });
    }

    // Find the nearest one
    let mut nearest: Option<(usize, &PoiInstance, u16)> = None;
    for (idx, p) in matching_pois {
        let dx = (p.x as i16 - poi.x as i16).unsigned_abs();
        let dy = (p.y as i16 - poi.y as i16).unsigned_abs();
        let distance = dx + dy;

        match nearest {
            Some((_, _, d)) if distance < d => {
                nearest = Some((idx, p, distance));
            }
            None => {
                nearest = Some((idx, p, distance));
            }
            _ => {}
        }
    }

    let revealed_poi = nearest.map(|(idx, p, _)| (idx, p.x, p.y));

    Ok(SeismicScannerResult {
        revealed_poi,
        category,
    })
}

// =============================================================================
// Scrap Chute Interaction (L14)
// =============================================================================

use crate::offers::calculate_scrap_cost;

/// Result of scrapping gear at the Scrap Chute
#[derive(Clone, Debug)]
pub struct ScrapGearResult {
    /// Item ID that was scrapped
    pub item_id: [u8; 8],
    /// Gold cost paid
    pub cost: u16,
}

/// Execute Scrap Chute interaction (L14).
///
/// Destroys one gear item for a gold cost (8-12 depending on act).
/// POI is one-time use.
pub fn execute_scrap_gear(
    poi: &PoiInstance,
    item_id: [u8; 8],
    player_gold: u16,
    act: u8,
    is_night: bool,
) -> Result<ScrapGearResult, PoiSystemError> {
    let def = can_interact(poi, is_night)?;

    if def.interaction_type != InteractionType::Scrap {
        return Err(PoiSystemError::InvalidInteraction);
    }

    // Calculate scrap cost
    let cost = calculate_scrap_cost(act);

    // Check gold
    if player_gold < cost {
        return Err(PoiSystemError::InsufficientGold);
    }

    Ok(ScrapGearResult { item_id, cost })
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_poi(poi_type: u8, x: u8, y: u8, used: bool, discovered: bool) -> PoiInstance {
        PoiInstance {
            poi_type,
            x,
            y,
            used,
            discovered,
            week_spawned: 1,
        }
    }

    #[test]
    fn test_can_interact_mole_den_night() {
        let poi = create_test_poi(1, 5, 5, false, false);
        let result = can_interact(&poi, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_interact_mole_den_day_fails() {
        let poi = create_test_poi(1, 5, 5, false, false);
        let result = can_interact(&poi, false);
        assert!(matches!(result, Err(PoiSystemError::NightOnlyPoi)));
    }

    #[test]
    fn test_can_interact_rest_alcove_night() {
        let poi = create_test_poi(5, 5, 5, false, false);
        let result = can_interact(&poi, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_interact_rest_alcove_already_used() {
        let poi = create_test_poi(5, 5, 5, true, false);
        let result = can_interact(&poi, true);
        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }

    #[test]
    fn test_can_interact_supply_cache_anytime() {
        let poi = create_test_poi(2, 5, 5, false, false);

        // Day
        assert!(can_interact(&poi, false).is_ok());
        // Night
        assert!(can_interact(&poi, true).is_ok());
    }

    #[test]
    fn test_can_interact_one_time_poi_used() {
        let poi = create_test_poi(2, 5, 5, true, false);
        let result = can_interact(&poi, false);
        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }

    #[test]
    fn test_can_interact_repeatable_poi_used() {
        // Mole Den is repeatable
        let poi = create_test_poi(1, 5, 5, true, false);
        let result = can_interact(&poi, true);
        assert!(
            result.is_ok(),
            "Repeatable POI should allow interaction even when 'used'"
        );
    }

    #[test]
    fn test_oil_flag_checking() {
        assert!(!is_oil_applied(0, OIL_FLAG_ATK));
        assert!(is_oil_applied(OIL_FLAG_ATK, OIL_FLAG_ATK));
        assert!(!is_oil_applied(OIL_FLAG_ATK, OIL_FLAG_SPD));
        assert!(is_oil_applied(OIL_FLAG_ATK | OIL_FLAG_SPD, OIL_FLAG_ATK));
        assert!(is_oil_applied(OIL_FLAG_ATK | OIL_FLAG_SPD, OIL_FLAG_SPD));
    }

    #[test]
    fn test_calculate_revealed_tiles_center_of_map() {
        let tiles = calculate_revealed_tiles(16, 16, 32, 32);

        // Center should be included
        assert!(tiles.contains(&(16, 16)));

        // Tiles at radius 13 should be included
        assert!(tiles.contains(&(16 + 13, 16)));
        assert!(tiles.contains(&(16, 16 + 13)));

        // Tiles beyond radius should not be included
        assert!(!tiles.contains(&(16 + 14, 16)));

        // Should have significant number of tiles
        assert!(tiles.len() > 100, "Should reveal many tiles");
    }

    #[test]
    fn test_calculate_revealed_tiles_edge_of_map() {
        let tiles = calculate_revealed_tiles(0, 0, 32, 32);

        // Origin should be included
        assert!(tiles.contains(&(0, 0)));

        // Negative coordinates should be clipped
        for (x, y) in &tiles {
            assert!(*x < 32);
            assert!(*y < 32);
        }
    }

    #[test]
    fn test_find_nearest_poi_of_type() {
        let pois = vec![
            create_test_poi(2, 10, 10, false, false), // Supply Cache at (10, 10)
            create_test_poi(2, 20, 20, false, false), // Supply Cache at (20, 20)
            create_test_poi(3, 5, 5, false, false),   // Tool Crate at (5, 5)
        ];

        // From (0, 0), nearest Supply Cache should be at (10, 10)
        let result = find_nearest_poi_of_type(&pois, 0, 0, 2, false);
        assert!(result.is_some());
        let (idx, poi) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(poi.x, 10);
        assert_eq!(poi.y, 10);

        // From (25, 25), nearest Supply Cache should be at (20, 20)
        let result = find_nearest_poi_of_type(&pois, 25, 25, 2, false);
        assert!(result.is_some());
        let (idx, _) = result.unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn test_find_nearest_poi_excludes_used() {
        let pois = vec![
            create_test_poi(2, 5, 5, true, false),    // Used Supply Cache
            create_test_poi(2, 20, 20, false, false), // Unused Supply Cache
        ];

        let result = find_nearest_poi_of_type(&pois, 0, 0, 2, true);
        assert!(result.is_some());
        let (idx, _) = result.unwrap();
        assert_eq!(idx, 1, "Should skip used POI");
    }

    #[test]
    fn test_get_available_waypoint_destinations() {
        let pois = vec![
            create_test_poi(8, 5, 5, false, true), // Discovered waypoint (current)
            create_test_poi(8, 15, 15, false, true), // Discovered waypoint
            create_test_poi(8, 25, 25, false, false), // Undiscovered waypoint
            create_test_poi(2, 10, 10, false, false), // Not a waypoint
        ];

        let destinations = get_available_waypoint_destinations(&pois, 0);
        assert_eq!(
            destinations.len(),
            1,
            "Should only include other discovered waypoints"
        );
        assert_eq!(destinations[0].0, 1);
    }

    #[test]
    fn test_get_available_waypoint_destinations_none() {
        let pois = vec![
            create_test_poi(8, 5, 5, false, true), // Only waypoint (current)
        ];

        let destinations = get_available_waypoint_destinations(&pois, 0);
        assert_eq!(destinations.len(), 0, "No other waypoints available");
    }

    // =========================================================================
    // Rest Interaction Tests
    // =========================================================================

    #[test]
    fn test_execute_rest_mole_den_full_heal() {
        let poi = create_test_poi(1, 5, 5, false, false); // L1 Mole Den
        let result = execute_rest_interaction(&poi, 50i16, 100i16, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.heal_amount, 50, "Should heal to full");
        assert!(result.full_heal);
        assert!(!result.mark_used, "Mole Den is repeatable");
    }

    #[test]
    fn test_execute_rest_mole_den_already_full() {
        let poi = create_test_poi(1, 5, 5, false, false);
        let result = execute_rest_interaction(&poi, 100i16, 100i16, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.heal_amount, 0, "Already at full HP");
    }

    #[test]
    fn test_execute_rest_mole_den_day_fails() {
        let poi = create_test_poi(1, 5, 5, false, false);
        let result = execute_rest_interaction(&poi, 50i16, 100i16, false);

        assert!(matches!(result, Err(PoiSystemError::NightOnlyPoi)));
    }

    #[test]
    fn test_execute_rest_alcove_partial_heal() {
        let poi = create_test_poi(5, 5, 5, false, false); // L5 Rest Alcove
        let result = execute_rest_interaction(&poi, 50i16, 100i16, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.heal_amount, 10, "Should heal 10 HP");
        assert!(!result.full_heal);
        assert!(result.mark_used, "Rest Alcove is one-time");
    }

    #[test]
    fn test_execute_rest_alcove_caps_at_max() {
        let poi = create_test_poi(5, 5, 5, false, false);
        let result = execute_rest_interaction(&poi, 95i16, 100i16, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.heal_amount, 5, "Should cap at max HP");
    }

    #[test]
    fn test_execute_rest_alcove_already_used() {
        let poi = create_test_poi(5, 5, 5, true, false);
        let result = execute_rest_interaction(&poi, 50i16, 100i16, true);

        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }

    #[test]
    fn test_execute_rest_invalid_poi_type() {
        // L2 Supply Cache is not a rest POI
        let poi = create_test_poi(2, 5, 5, false, false);
        let result = execute_rest_interaction(&poi, 50i16, 100i16, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_rest_high_hp_values() {
        // Test with HP > 255 to ensure no truncation
        let poi = create_test_poi(1, 5, 5, false, false); // L1 Mole Den
        let result = execute_rest_interaction(&poi, 200i16, 500i16, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.heal_amount, 300, "Should heal 300 HP (500 - 200)");
    }

    #[test]
    fn test_execute_rest_negative_hp_treated_as_zero() {
        // Negative HP should be treated as 0 for healing calculations
        let poi = create_test_poi(1, 5, 5, false, false); // L1 Mole Den
        let result = execute_rest_interaction(&poi, -10i16, 100i16, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(
            result.heal_amount, 100,
            "Should heal full amount when HP is negative"
        );
    }

    // =========================================================================
    // Pick Item Interaction Tests
    // =========================================================================

    fn create_test_offers() -> Vec<ItemOffer> {
        vec![
            ItemOffer {
                item_id: *b"G-ST-01\0",
                tier: 1,
                price: 0,
                purchased: false,
            },
            ItemOffer {
                item_id: *b"G-ST-02\0",
                tier: 1,
                price: 0,
                purchased: false,
            },
            ItemOffer {
                item_id: *b"G-ST-03\0",
                tier: 1,
                price: 0,
                purchased: false,
            },
        ]
    }

    #[test]
    fn test_execute_pick_item_supply_cache() {
        let poi = create_test_poi(2, 5, 5, false, false); // L2 Supply Cache
        let offers = create_test_offers();
        let result = execute_pick_item_interaction(&poi, &offers, 0, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.item.item_id, *b"G-ST-01\0");
        assert!(result.mark_used);
    }

    #[test]
    fn test_execute_pick_item_tool_crate() {
        let poi = create_test_poi(3, 5, 5, false, false); // L3 Tool Crate
        let offers = create_test_offers();
        let result = execute_pick_item_interaction(&poi, &offers, 1, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.item.item_id, *b"G-ST-02\0");
    }

    #[test]
    fn test_execute_pick_item_geode_vault() {
        let poi = create_test_poi(12, 5, 5, false, false); // L12 Geode Vault
        let offers = create_test_offers();
        let result = execute_pick_item_interaction(&poi, &offers, 2, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.item.item_id, *b"G-ST-03\0");
    }

    #[test]
    fn test_execute_pick_item_counter_cache() {
        let poi = create_test_poi(13, 5, 5, false, false); // L13 Counter Cache
        let offers = create_test_offers();
        let result = execute_pick_item_interaction(&poi, &offers, 0, true);

        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_pick_item_already_used() {
        let poi = create_test_poi(2, 5, 5, true, false); // Already used
        let offers = create_test_offers();
        let result = execute_pick_item_interaction(&poi, &offers, 0, true);

        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }

    #[test]
    fn test_execute_pick_item_invalid_choice() {
        let poi = create_test_poi(2, 5, 5, false, false);
        let offers = create_test_offers();
        let result = execute_pick_item_interaction(&poi, &offers, 5, true); // Out of bounds

        assert!(matches!(result, Err(PoiSystemError::InvalidOfferIndex)));
    }

    #[test]
    fn test_execute_pick_item_wrong_poi_type() {
        let poi = create_test_poi(1, 5, 5, false, false); // L1 Mole Den - not a pick POI
        let offers = create_test_offers();
        let result = execute_pick_item_interaction(&poi, &offers, 0, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    // =========================================================================
    // Tool Oil Interaction Tests
    // =========================================================================

    #[test]
    fn test_execute_tool_oil_apply_atk() {
        let poi = create_test_poi(4, 5, 5, false, false); // L4 Tool Oil Rack
        let result = execute_tool_oil_interaction(&poi, 0, OIL_FLAG_ATK, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.modification, OIL_FLAG_ATK);
        assert_eq!(result.new_flags, OIL_FLAG_ATK);
    }

    #[test]
    fn test_execute_tool_oil_apply_spd() {
        let poi = create_test_poi(4, 5, 5, false, false);
        let result = execute_tool_oil_interaction(&poi, 0, OIL_FLAG_SPD, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.modification, OIL_FLAG_SPD);
        assert_eq!(result.new_flags, OIL_FLAG_SPD);
    }

    #[test]
    fn test_execute_tool_oil_apply_dig() {
        let poi = create_test_poi(4, 5, 5, false, false);
        let result = execute_tool_oil_interaction(&poi, 0, OIL_FLAG_DIG, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.modification, OIL_FLAG_DIG);
        assert_eq!(result.new_flags, OIL_FLAG_DIG);
    }

    #[test]
    fn test_execute_tool_oil_stacks() {
        let poi = create_test_poi(4, 5, 5, false, false);

        // Apply ATK first
        let result1 = execute_tool_oil_interaction(&poi, 0, OIL_FLAG_ATK, true).unwrap();
        assert_eq!(result1.new_flags, OIL_FLAG_ATK);

        // Apply SPD second
        let result2 =
            execute_tool_oil_interaction(&poi, result1.new_flags, OIL_FLAG_SPD, true).unwrap();
        assert_eq!(result2.new_flags, OIL_FLAG_ATK | OIL_FLAG_SPD);

        // Apply DIG third
        let result3 =
            execute_tool_oil_interaction(&poi, result2.new_flags, OIL_FLAG_DIG, true).unwrap();
        assert_eq!(
            result3.new_flags,
            OIL_FLAG_ATK | OIL_FLAG_SPD | OIL_FLAG_DIG
        );
    }

    #[test]
    fn test_execute_tool_oil_already_applied() {
        let poi = create_test_poi(4, 5, 5, false, false);
        let result = execute_tool_oil_interaction(&poi, OIL_FLAG_ATK, OIL_FLAG_ATK, true);

        assert!(matches!(result, Err(PoiSystemError::OilAlreadyApplied)));
    }

    #[test]
    fn test_execute_tool_oil_wrong_poi_type() {
        let poi = create_test_poi(2, 5, 5, false, false); // L2 Supply Cache - not tool oil
        let result = execute_tool_oil_interaction(&poi, 0, OIL_FLAG_ATK, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_tool_oil_invalid_modification() {
        let poi = create_test_poi(4, 5, 5, false, false);
        let result = execute_tool_oil_interaction(&poi, 0, 0x10, true); // Invalid flag (0x10 is not a valid oil)

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_tool_oil_apply_arm() {
        let poi = create_test_poi(4, 5, 5, false, false);
        let result = execute_tool_oil_interaction(&poi, 0, OIL_FLAG_ARM, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.modification, OIL_FLAG_ARM);
        assert_eq!(result.new_flags, OIL_FLAG_ARM);
    }

    #[test]
    fn test_execute_tool_oil_anytime() {
        let poi = create_test_poi(4, 5, 5, false, false);

        // Works during day
        assert!(execute_tool_oil_interaction(&poi, 0, OIL_FLAG_ATK, false).is_ok());

        // Works during night
        assert!(execute_tool_oil_interaction(&poi, 0, OIL_FLAG_SPD, true).is_ok());
    }

    // =========================================================================
    // Shop Interaction Tests
    // =========================================================================

    fn create_test_shop_state(active: bool) -> ShopState {
        let mut offers = [ItemOffer::default(); 6];

        // 1 Tool + 5 Gear
        offers[0] = ItemOffer {
            item_id: *b"T-ST-01\0",
            tier: 1,
            price: 10,
            purchased: false,
        };
        for i in 1..6 {
            offers[i] = ItemOffer {
                item_id: *b"G-ST-01\0",
                tier: 1,
                price: 8,
                purchased: false,
            };
        }

        ShopState {
            poi_index: 0,
            offers,
            reroll_count: 0,
            active,
            rng_state: 0,
        }
    }

    #[test]
    fn test_validate_shop_poi() {
        let poi = create_test_poi(9, 5, 5, false, false); // L9 Smuggler Hatch
        assert!(validate_shop_poi(&poi, true).is_ok());
        assert!(validate_shop_poi(&poi, false).is_ok()); // Anytime
    }

    #[test]
    fn test_validate_shop_poi_wrong_type() {
        let poi = create_test_poi(2, 5, 5, false, false); // L2 Supply Cache
        assert!(matches!(
            validate_shop_poi(&poi, true),
            Err(PoiSystemError::InvalidInteraction)
        ));
    }

    #[test]
    fn test_execute_shop_purchase_success() {
        let shop_state = create_test_shop_state(true);
        let result = execute_shop_purchase(&shop_state, 0, 100); // Plenty of gold

        assert!(result.is_ok());
        let (offer, price) = result.unwrap();
        assert_eq!(offer.item_id, *b"T-ST-01\0");
        assert_eq!(price, 10);
    }

    #[test]
    fn test_execute_shop_purchase_insufficient_gold() {
        let shop_state = create_test_shop_state(true);
        let result = execute_shop_purchase(&shop_state, 0, 5); // Not enough

        assert!(matches!(result, Err(PoiSystemError::InsufficientGold)));
    }

    #[test]
    fn test_execute_shop_purchase_already_purchased() {
        let mut shop_state = create_test_shop_state(true);
        shop_state.offers[0].purchased = true;

        let result = execute_shop_purchase(&shop_state, 0, 100);
        assert!(matches!(result, Err(PoiSystemError::OfferAlreadyPurchased)));
    }

    #[test]
    fn test_execute_shop_purchase_shop_not_active() {
        let shop_state = create_test_shop_state(false);
        let result = execute_shop_purchase(&shop_state, 0, 100);

        assert!(matches!(result, Err(PoiSystemError::ShopNotActive)));
    }

    #[test]
    fn test_execute_shop_purchase_invalid_index() {
        let shop_state = create_test_shop_state(true);
        let result = execute_shop_purchase(&shop_state, 10, 100); // Out of bounds

        assert!(matches!(result, Err(PoiSystemError::InvalidOfferIndex)));
    }

    #[test]
    fn test_calculate_shop_reroll_cost() {
        let mut shop_state = create_test_shop_state(true);

        // Initial cost: 4
        assert_eq!(calculate_shop_reroll_cost(&shop_state).unwrap(), 4);

        shop_state.reroll_count = 1;
        assert_eq!(calculate_shop_reroll_cost(&shop_state).unwrap(), 6);

        shop_state.reroll_count = 2;
        assert_eq!(calculate_shop_reroll_cost(&shop_state).unwrap(), 8);

        shop_state.reroll_count = 3;
        assert_eq!(calculate_shop_reroll_cost(&shop_state).unwrap(), 10);
    }

    #[test]
    fn test_calculate_shop_reroll_cost_not_active() {
        let shop_state = create_test_shop_state(false);
        assert!(matches!(
            calculate_shop_reroll_cost(&shop_state),
            Err(PoiSystemError::ShopNotActive)
        ));
    }

    #[test]
    fn test_get_offer_price() {
        // Tools
        assert_eq!(get_offer_price(b"T-ST-01\0"), 10); // Common
        assert_eq!(get_offer_price(b"T-FR-02\0"), 16); // Rare

        // Gear - prices based on authoritative rarity from item definitions
        assert_eq!(get_offer_price(b"G-ST-01\0"), 8); // Common
        assert_eq!(get_offer_price(b"G-ST-02\0"), 8); // Common
        assert_eq!(get_offer_price(b"G-ST-03\0"), 8); // Common (GDD override)
        assert_eq!(get_offer_price(b"G-ST-05\0"), 14); // Rare
        assert_eq!(get_offer_price(b"G-ST-06\0"), 14); // Rare (GDD override)
        assert_eq!(get_offer_price(b"G-ST-08\0"), 22); // Heroic (GDD override)
    }

    // =========================================================================
    // Anvil Upgrade Tests (L10 Rusty Anvil)
    // =========================================================================

    #[test]
    fn test_execute_anvil_upgrade_tier1_to_tier2() {
        let poi = create_test_poi(10, 5, 5, false, false); // L10 Rusty Anvil
        let item_id = *b"T-ST-01\0";
        let result = execute_anvil_upgrade(&poi, item_id, 1, 100, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.item_id, item_id);
        assert_eq!(result.new_tier, 2);
        assert_eq!(result.cost, 8); // Tier 1 -> 2 costs 8 gold
    }

    #[test]
    fn test_execute_anvil_upgrade_tier2_to_tier3() {
        let poi = create_test_poi(10, 5, 5, false, false);
        let item_id = *b"T-ST-01\0";
        let result = execute_anvil_upgrade(&poi, item_id, 2, 100, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.new_tier, 3);
        assert_eq!(result.cost, 16); // Tier 2 -> 3 costs 16 gold
    }

    #[test]
    fn test_execute_anvil_upgrade_insufficient_gold() {
        let poi = create_test_poi(10, 5, 5, false, false);
        let item_id = *b"T-ST-01\0";
        let result = execute_anvil_upgrade(&poi, item_id, 1, 5, true); // Only 5 gold, need 8

        assert!(matches!(result, Err(PoiSystemError::InsufficientGold)));
    }

    #[test]
    fn test_execute_anvil_upgrade_already_max_tier() {
        let poi = create_test_poi(10, 5, 5, false, false);
        let item_id = *b"T-ST-01\0";
        let result = execute_anvil_upgrade(&poi, item_id, 3, 100, true); // Already tier 3

        assert!(matches!(result, Err(PoiSystemError::AlreadyMaxTier)));
    }

    #[test]
    fn test_execute_anvil_upgrade_poi_already_used() {
        let poi = create_test_poi(10, 5, 5, true, false); // POI already used
        let item_id = *b"T-ST-01\0";
        let result = execute_anvil_upgrade(&poi, item_id, 1, 100, true);

        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }

    #[test]
    fn test_execute_anvil_upgrade_wrong_poi_type() {
        let poi = create_test_poi(2, 5, 5, false, false); // L2 Supply Cache - not an upgrade POI
        let item_id = *b"T-ST-01\0";
        let result = execute_anvil_upgrade(&poi, item_id, 1, 100, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_anvil_upgrade_anytime() {
        let poi = create_test_poi(10, 5, 5, false, false);
        let item_id = *b"T-ST-01\0";

        // Works during day
        assert!(execute_anvil_upgrade(&poi, item_id, 1, 100, false).is_ok());

        // Works during night
        let poi2 = create_test_poi(10, 5, 5, false, false);
        assert!(execute_anvil_upgrade(&poi2, item_id, 1, 100, true).is_ok());
    }

    // =========================================================================
    // Kiln Fusion Tests (L11 Rune Kiln)
    // =========================================================================

    #[test]
    fn test_execute_kiln_fusion_success() {
        let poi = create_test_poi(11, 5, 5, false, false); // L11 Rune Kiln
        let item_id = *b"G-ST-01\0";
        let result = execute_kiln_fusion(&poi, item_id, 1, item_id, 1, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.item_id, item_id);
        assert_eq!(result.result_tier, 2); // Tier 1 + 1 = Tier 2
    }

    #[test]
    fn test_execute_kiln_fusion_tier2_to_tier3() {
        let poi = create_test_poi(11, 5, 5, false, false);
        let item_id = *b"G-ST-01\0";
        let result = execute_kiln_fusion(&poi, item_id, 2, item_id, 2, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.result_tier, 3);
    }

    #[test]
    fn test_execute_kiln_fusion_items_not_identical_different_id() {
        let poi = create_test_poi(11, 5, 5, false, false);
        let item1_id = *b"G-ST-01\0";
        let item2_id = *b"G-ST-02\0"; // Different item
        let result = execute_kiln_fusion(&poi, item1_id, 1, item2_id, 1, true);

        assert!(matches!(result, Err(PoiSystemError::ItemsNotIdentical)));
    }

    #[test]
    fn test_execute_kiln_fusion_items_not_identical_different_tier() {
        let poi = create_test_poi(11, 5, 5, false, false);
        let item_id = *b"G-ST-01\0";
        let result = execute_kiln_fusion(&poi, item_id, 1, item_id, 2, true); // Different tiers

        assert!(matches!(result, Err(PoiSystemError::ItemsNotIdentical)));
    }

    #[test]
    fn test_execute_kiln_fusion_already_max_tier() {
        let poi = create_test_poi(11, 5, 5, false, false);
        let item_id = *b"G-ST-01\0";
        let result = execute_kiln_fusion(&poi, item_id, 3, item_id, 3, true); // Already tier 3

        assert!(matches!(result, Err(PoiSystemError::AlreadyMaxTier)));
    }

    #[test]
    fn test_execute_kiln_fusion_wrong_poi_type() {
        let poi = create_test_poi(2, 5, 5, false, false); // L2 Supply Cache - not a fuse POI
        let item_id = *b"G-ST-01\0";
        let result = execute_kiln_fusion(&poi, item_id, 1, item_id, 1, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_kiln_fusion_repeatable() {
        // Rune Kiln is Repeatable - can be used even if "used" flag is set
        let poi = create_test_poi(11, 5, 5, true, false); // "used" but still works
        let item_id = *b"G-ST-01\0";
        let result = execute_kiln_fusion(&poi, item_id, 1, item_id, 1, true);

        assert!(result.is_ok(), "Rune Kiln should be repeatable");
    }

    #[test]
    fn test_execute_kiln_fusion_anytime() {
        let poi = create_test_poi(11, 5, 5, false, false);
        let item_id = *b"G-ST-01\0";

        // Works during day
        assert!(execute_kiln_fusion(&poi, item_id, 1, item_id, 1, false).is_ok());

        // Works during night
        assert!(execute_kiln_fusion(&poi, item_id, 1, item_id, 1, true).is_ok());
    }

    // =========================================================================
    // Fast Travel Tests (L8 Rail Waypoint)
    // =========================================================================

    #[test]
    fn test_execute_waypoint_discover_new() {
        let poi = create_test_poi(8, 10, 20, false, false); // L8 Rail Waypoint, not discovered
        let result = execute_waypoint_discover(&poi, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.newly_discovered);
        assert_eq!(result.x, 10);
        assert_eq!(result.y, 20);
    }

    #[test]
    fn test_execute_waypoint_discover_already_discovered() {
        let poi = create_test_poi(8, 10, 20, false, true); // Already discovered
        let result = execute_waypoint_discover(&poi, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.newly_discovered);
    }

    #[test]
    fn test_execute_waypoint_discover_wrong_poi_type() {
        let poi = create_test_poi(2, 10, 20, false, false); // L2 Supply Cache
        let result = execute_waypoint_discover(&poi, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_fast_travel_success() {
        let from_poi = create_test_poi(8, 5, 5, false, true); // Discovered
        let to_poi = create_test_poi(8, 20, 20, false, true); // Discovered
        let result = execute_fast_travel(&from_poi, &to_poi, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.from_x, 5);
        assert_eq!(result.from_y, 5);
        assert_eq!(result.to_x, 20);
        assert_eq!(result.to_y, 20);
    }

    #[test]
    fn test_execute_fast_travel_source_not_discovered() {
        let from_poi = create_test_poi(8, 5, 5, false, false); // Not discovered
        let to_poi = create_test_poi(8, 20, 20, false, true);
        let result = execute_fast_travel(&from_poi, &to_poi, true);

        assert!(matches!(
            result,
            Err(PoiSystemError::DestinationNotDiscovered)
        ));
    }

    #[test]
    fn test_execute_fast_travel_destination_not_discovered() {
        let from_poi = create_test_poi(8, 5, 5, false, true);
        let to_poi = create_test_poi(8, 20, 20, false, false); // Not discovered
        let result = execute_fast_travel(&from_poi, &to_poi, true);

        assert!(matches!(
            result,
            Err(PoiSystemError::DestinationNotDiscovered)
        ));
    }

    #[test]
    fn test_execute_fast_travel_destination_wrong_type() {
        let from_poi = create_test_poi(8, 5, 5, false, true);
        let to_poi = create_test_poi(2, 20, 20, false, true); // L2 Supply Cache
        let result = execute_fast_travel(&from_poi, &to_poi, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_fast_travel_same_location() {
        let from_poi = create_test_poi(8, 5, 5, false, true);
        let to_poi = create_test_poi(8, 5, 5, false, true); // Same location
        let result = execute_fast_travel(&from_poi, &to_poi, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_fast_travel_anytime() {
        let from_poi = create_test_poi(8, 5, 5, false, true);
        let to_poi = create_test_poi(8, 20, 20, false, true);

        // Works during day
        assert!(execute_fast_travel(&from_poi, &to_poi, false).is_ok());

        // Works during night
        assert!(execute_fast_travel(&from_poi, &to_poi, true).is_ok());
    }

    // =========================================================================
    // Map Reveal Tests (L6 Survey Beacon, L7 Seismic Scanner)
    // =========================================================================

    #[test]
    fn test_execute_survey_beacon_success() {
        let poi = create_test_poi(6, 16, 16, false, false); // L6 Survey Beacon
        let result = execute_survey_beacon(&poi, 32, 32, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.center_x, 16);
        assert_eq!(result.center_y, 16);
        assert!(!result.tiles.is_empty());
        assert!(result.tiles.len() > 100); // Should reveal many tiles
    }

    #[test]
    fn test_execute_survey_beacon_wrong_poi_type() {
        let poi = create_test_poi(2, 16, 16, false, false); // L2 Supply Cache
        let result = execute_survey_beacon(&poi, 32, 32, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_survey_beacon_already_used() {
        let poi = create_test_poi(6, 16, 16, true, false); // Already used
        let result = execute_survey_beacon(&poi, 32, 32, true);

        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }

    #[test]
    fn test_execute_seismic_scanner_finds_poi() {
        let pois = vec![
            create_test_poi(7, 10, 10, false, false), // L7 Scanner at (10,10) - index 0
            create_test_poi(2, 15, 15, false, false), // L2 Supply Cache (Items) - index 1
            create_test_poi(2, 25, 25, false, false), // L2 Supply Cache (Items) - index 2
        ];

        let result = execute_seismic_scanner(&pois[0], &pois, 0, PoiCategory::Items, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.revealed_poi.is_some());
        let (idx, x, y) = result.revealed_poi.unwrap();
        assert_eq!(idx, 1); // Nearest Items POI
        assert_eq!(x, 15);
        assert_eq!(y, 15);
    }

    #[test]
    fn test_execute_seismic_scanner_no_matching_category() {
        let pois = vec![
            create_test_poi(7, 10, 10, false, false), // L7 Scanner (Utility)
            create_test_poi(2, 15, 15, false, false), // L2 Supply Cache (Items)
        ];

        // Search for Shop category - none exist
        let result = execute_seismic_scanner(&pois[0], &pois, 0, PoiCategory::Shop, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.revealed_poi.is_none());
    }

    #[test]
    fn test_execute_seismic_scanner_skips_discovered() {
        let pois = vec![
            create_test_poi(7, 10, 10, false, false), // L7 Scanner
            create_test_poi(2, 15, 15, false, true),  // Discovered (skip)
            create_test_poi(2, 25, 25, false, false), // Not discovered
        ];

        let result = execute_seismic_scanner(&pois[0], &pois, 0, PoiCategory::Items, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.revealed_poi.is_some());
        let (idx, _, _) = result.revealed_poi.unwrap();
        assert_eq!(idx, 2); // Skipped the discovered one
    }

    #[test]
    fn test_execute_seismic_scanner_wrong_poi_type() {
        let poi = create_test_poi(2, 10, 10, false, false); // L2 Supply Cache
        let pois = vec![poi.clone()];
        let result = execute_seismic_scanner(&poi, &pois, 0, PoiCategory::Items, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_seismic_scanner_already_used() {
        let poi = create_test_poi(7, 10, 10, true, false); // Already used
        let pois = vec![poi.clone()];
        let result = execute_seismic_scanner(&poi, &pois, 0, PoiCategory::Items, true);

        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }

    // =========================================================================
    // Scrap Chute Tests (L14)
    // =========================================================================

    #[test]
    fn test_execute_scrap_gear_success() {
        let poi = create_test_poi(14, 5, 5, false, false); // L14 Scrap Chute
        let item_id = *b"G-ST-01\0";
        let result = execute_scrap_gear(&poi, item_id, 100, 1, true);

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.item_id, item_id);
        assert_eq!(result.cost, 8); // Act 1 cost
    }

    #[test]
    fn test_execute_scrap_gear_act_costs() {
        let poi = create_test_poi(14, 5, 5, false, false);
        let item_id = *b"G-ST-01\0";

        // Act 1: 8 gold
        let r1 = execute_scrap_gear(&poi, item_id, 100, 1, true).unwrap();
        assert_eq!(r1.cost, 8);

        // Act 2: 8 gold
        let poi2 = create_test_poi(14, 5, 5, false, false);
        let r2 = execute_scrap_gear(&poi2, item_id, 100, 2, true).unwrap();
        assert_eq!(r2.cost, 8);

        // Act 3: 10 gold
        let poi3 = create_test_poi(14, 5, 5, false, false);
        let r3 = execute_scrap_gear(&poi3, item_id, 100, 3, true).unwrap();
        assert_eq!(r3.cost, 10);

        // Act 4: 12 gold
        let poi4 = create_test_poi(14, 5, 5, false, false);
        let r4 = execute_scrap_gear(&poi4, item_id, 100, 4, true).unwrap();
        assert_eq!(r4.cost, 12);
    }

    #[test]
    fn test_execute_scrap_gear_insufficient_gold() {
        let poi = create_test_poi(14, 5, 5, false, false);
        let item_id = *b"G-ST-01\0";
        let result = execute_scrap_gear(&poi, item_id, 5, 1, true); // Only 5 gold, need 8

        assert!(matches!(result, Err(PoiSystemError::InsufficientGold)));
    }

    #[test]
    fn test_execute_scrap_gear_wrong_poi_type() {
        let poi = create_test_poi(2, 5, 5, false, false); // L2 Supply Cache
        let item_id = *b"G-ST-01\0";
        let result = execute_scrap_gear(&poi, item_id, 100, 1, true);

        assert!(matches!(result, Err(PoiSystemError::InvalidInteraction)));
    }

    #[test]
    fn test_execute_scrap_gear_already_used() {
        let poi = create_test_poi(14, 5, 5, true, false); // Already used
        let item_id = *b"G-ST-01\0";
        let result = execute_scrap_gear(&poi, item_id, 100, 1, true);

        assert!(matches!(result, Err(PoiSystemError::PoiAlreadyUsed)));
    }
}
