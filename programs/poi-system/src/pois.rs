//! POI definitions for the poi-system program.
//!
//! Contains all 14 POI definitions as compile-time constants.
//! Based on GDD Section 12 specifications.

use crate::state::{ActiveCondition, InteractionType, PoiCategory, PoiRarity, UseType};

/// Static POI definition (compile-time constant)
#[derive(Clone, Copy, Debug)]
pub struct PoiDefinition {
    /// POI type ID (1-14 mapping to L1-L14)
    pub id: u8,
    /// Display name (e.g., "Mole Den")
    pub name: &'static str,
    /// UTF-8 encoded emoji
    pub emoji: [u8; 4],
    /// POI rarity
    pub rarity: PoiRarity,
    /// Use type (one-time, repeatable, etc.)
    pub use_type: UseType,
    /// Active condition (anytime or night-only)
    pub active_condition: ActiveCondition,
    /// Interaction type
    pub interaction_type: InteractionType,
    /// Category for Seismic Scanner
    pub category: PoiCategory,
}

// =============================================================================
// POI Definitions (L1-L14)
// =============================================================================

/// L1: Mole Den - Night-only full heal, skip to day
pub const L1_MOLE_DEN: PoiDefinition = PoiDefinition {
    id: 1,
    name: "Mole Den",
    emoji: [0xF0, 0x9F, 0x95, 0xB3], // hole emoji
    rarity: PoiRarity::Fixed,
    use_type: UseType::Repeatable,
    active_condition: ActiveCondition::NightOnly,
    interaction_type: InteractionType::RestFull,
    category: PoiCategory::Utility,
};

/// L2: Supply Cache - Pick 1 of 3 Gear items
pub const L2_SUPPLY_CACHE: PoiDefinition = PoiDefinition {
    id: 2,
    name: "Supply Cache",
    emoji: [0xF0, 0x9F, 0x93, 0xA6], // package emoji
    rarity: PoiRarity::Common,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::PickItem,
    category: PoiCategory::Items,
};

/// L3: Tool Crate - Pick 1 of 3 Tools
pub const L3_TOOL_CRATE: PoiDefinition = PoiDefinition {
    id: 3,
    name: "Tool Crate",
    emoji: [0xF0, 0x9F, 0xA7, 0xB0], // toolbox emoji
    rarity: PoiRarity::Uncommon,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::PickItem,
    category: PoiCategory::Items,
};

/// L4: Tool Oil Rack - Modify tool stats (+1 ATK/SPD/DIG)
pub const L4_TOOL_OIL_RACK: PoiDefinition = PoiDefinition {
    id: 4,
    name: "Tool Oil Rack",
    emoji: [0xF0, 0x9F, 0x9B, 0xA2], // oil drum emoji
    rarity: PoiRarity::Common,
    use_type: UseType::RepeatablePerTool,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::ToolOil,
    category: PoiCategory::Upgrades,
};

/// L5: Rest Alcove - Night-only heal 10 HP, skip to day
pub const L5_REST_ALCOVE: PoiDefinition = PoiDefinition {
    id: 5,
    name: "Rest Alcove",
    emoji: [0xF0, 0x9F, 0x9B, 0x8F], // bed emoji
    rarity: PoiRarity::Common,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::NightOnly,
    interaction_type: InteractionType::RestPartial,
    category: PoiCategory::Utility,
};

/// L6: Survey Beacon - Reveal tiles in radius 13
pub const L6_SURVEY_BEACON: PoiDefinition = PoiDefinition {
    id: 6,
    name: "Survey Beacon",
    emoji: [0xF0, 0x9F, 0x93, 0xA1], // antenna emoji
    rarity: PoiRarity::Common,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::MapRevealRadius,
    category: PoiCategory::Utility,
};

/// L7: Seismic Scanner - Reveal nearest POI of category
pub const L7_SEISMIC_SCANNER: PoiDefinition = PoiDefinition {
    id: 7,
    name: "Seismic Scanner",
    emoji: [0xF0, 0x9F, 0x94, 0x8A], // loud sound emoji
    rarity: PoiRarity::Uncommon,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::MapRevealNearest,
    category: PoiCategory::Utility,
};

/// L8: Rail Waypoint - Fast travel between discovered waypoints
pub const L8_RAIL_WAYPOINT: PoiDefinition = PoiDefinition {
    id: 8,
    name: "Rail Waypoint",
    emoji: [0xF0, 0x9F, 0x9A, 0x82], // train emoji
    rarity: PoiRarity::Uncommon,
    use_type: UseType::Repeatable,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::FastTravel,
    category: PoiCategory::Utility,
};

/// L9: Smuggler Hatch - Shop: 1 Tool + 5 Gear, reroll for gold
pub const L9_SMUGGLER_HATCH: PoiDefinition = PoiDefinition {
    id: 9,
    name: "Smuggler Hatch",
    emoji: [0xF0, 0x9F, 0x92, 0xB0], // money bag emoji
    rarity: PoiRarity::Uncommon,
    use_type: UseType::Repeatable,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::Shop,
    category: PoiCategory::Shop,
};

/// L10: Rusty Anvil - Upgrade tool tier (8/16 Gold)
pub const L10_RUSTY_ANVIL: PoiDefinition = PoiDefinition {
    id: 10,
    name: "Rusty Anvil",
    emoji: [0xE2, 0x9A, 0x92, 0x00], // hammer emoji (3 bytes)
    rarity: PoiRarity::Uncommon,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::Upgrade,
    category: PoiCategory::Upgrades,
};

/// L11: Rune Kiln - Fuse 2 identical items (free)
pub const L11_RUNE_KILN: PoiDefinition = PoiDefinition {
    id: 11,
    name: "Rune Kiln",
    emoji: [0xF0, 0x9F, 0x94, 0xA5], // fire emoji
    rarity: PoiRarity::Rare,
    use_type: UseType::Repeatable,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::Fuse,
    category: PoiCategory::Upgrades,
};

/// L12: Geode Vault - Pick 1 of 3 Heroic items
pub const L12_GEODE_VAULT: PoiDefinition = PoiDefinition {
    id: 12,
    name: "Geode Vault",
    emoji: [0xF0, 0x9F, 0x92, 0x8E], // gem emoji
    rarity: PoiRarity::Rare,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::PickItem,
    category: PoiCategory::Items,
};

/// L13: Counter Cache - Pick 1 of 3 weakness-tagged items
pub const L13_COUNTER_CACHE: PoiDefinition = PoiDefinition {
    id: 13,
    name: "Counter Cache",
    emoji: [0xF0, 0x9F, 0x8E, 0xAF], // target emoji
    rarity: PoiRarity::Uncommon,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::PickItem,
    category: PoiCategory::Items,
};

/// L14: Scrap Chute - Destroy 1 Gear (8-12 Gold cost)
pub const L14_SCRAP_CHUTE: PoiDefinition = PoiDefinition {
    id: 14,
    name: "Scrap Chute",
    emoji: [0xE2, 0x99, 0xBB, 0x00], // recycle emoji (3 bytes)
    rarity: PoiRarity::Uncommon,
    use_type: UseType::OneTime,
    active_condition: ActiveCondition::Anytime,
    interaction_type: InteractionType::Scrap,
    category: PoiCategory::Utility,
};

// =============================================================================
// POI Registry
// =============================================================================

/// All 14 POI definitions as a compile-time constant array
pub const POIS: &[PoiDefinition] = &[
    L1_MOLE_DEN,
    L2_SUPPLY_CACHE,
    L3_TOOL_CRATE,
    L4_TOOL_OIL_RACK,
    L5_REST_ALCOVE,
    L6_SURVEY_BEACON,
    L7_SEISMIC_SCANNER,
    L8_RAIL_WAYPOINT,
    L9_SMUGGLER_HATCH,
    L10_RUSTY_ANVIL,
    L11_RUNE_KILN,
    L12_GEODE_VAULT,
    L13_COUNTER_CACHE,
    L14_SCRAP_CHUTE,
];

// =============================================================================
// Query Functions
// =============================================================================

/// Get a POI definition by its type ID (1-14)
pub fn get_poi_definition(poi_type: u8) -> Option<&'static PoiDefinition> {
    if (1..=14).contains(&poi_type) {
        Some(&POIS[(poi_type - 1) as usize])
    } else {
        None
    }
}

/// Get all POIs with a specific rarity
pub fn get_pois_by_rarity(rarity: PoiRarity) -> Vec<&'static PoiDefinition> {
    POIS.iter().filter(|p| p.rarity == rarity).collect()
}

/// Get all POIs in a category
pub fn get_pois_by_category(category: PoiCategory) -> Vec<&'static PoiDefinition> {
    POIS.iter().filter(|p| p.category == category).collect()
}

/// Get all item-giving POIs
pub fn get_item_pois() -> Vec<&'static PoiDefinition> {
    POIS.iter()
        .filter(|p| p.interaction_type == InteractionType::PickItem)
        .collect()
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_poi_count() {
        assert_eq!(POIS.len(), 14, "Should have exactly 14 POI types");
    }

    #[test]
    fn test_get_poi_definition_valid() {
        for id in 1..=14 {
            let poi = get_poi_definition(id);
            assert!(poi.is_some(), "POI {} should exist", id);
            assert_eq!(poi.unwrap().id, id, "POI ID should match");
        }
    }

    #[test]
    fn test_get_poi_definition_invalid() {
        assert!(get_poi_definition(0).is_none());
        assert!(get_poi_definition(15).is_none());
        assert!(get_poi_definition(255).is_none());
    }

    #[test]
    fn test_mole_den_properties() {
        let poi = get_poi_definition(1).unwrap();
        assert_eq!(poi.name, "Mole Den");
        assert_eq!(poi.rarity, PoiRarity::Fixed);
        assert_eq!(poi.use_type, UseType::Repeatable);
        assert_eq!(poi.active_condition, ActiveCondition::NightOnly);
        assert_eq!(poi.interaction_type, InteractionType::RestFull);
    }

    #[test]
    fn test_supply_cache_properties() {
        let poi = get_poi_definition(2).unwrap();
        assert_eq!(poi.name, "Supply Cache");
        assert_eq!(poi.rarity, PoiRarity::Common);
        assert_eq!(poi.use_type, UseType::OneTime);
        assert_eq!(poi.active_condition, ActiveCondition::Anytime);
        assert_eq!(poi.interaction_type, InteractionType::PickItem);
    }

    #[test]
    fn test_night_only_pois() {
        // L1 and L5 are night-only
        let l1 = get_poi_definition(1).unwrap();
        let l5 = get_poi_definition(5).unwrap();

        assert_eq!(l1.active_condition, ActiveCondition::NightOnly);
        assert_eq!(l5.active_condition, ActiveCondition::NightOnly);

        // All other POIs should be anytime
        for id in [2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14] {
            let poi = get_poi_definition(id).unwrap();
            assert_eq!(
                poi.active_condition,
                ActiveCondition::Anytime,
                "L{} should be Anytime",
                id
            );
        }
    }

    #[test]
    fn test_one_time_pois() {
        let one_time_ids = [2, 3, 5, 6, 7, 10, 12, 13, 14];
        for id in one_time_ids {
            let poi = get_poi_definition(id).unwrap();
            assert_eq!(poi.use_type, UseType::OneTime, "L{} should be OneTime", id);
        }
    }

    #[test]
    fn test_repeatable_pois() {
        let repeatable_ids = [1, 8, 9, 11];
        for id in repeatable_ids {
            let poi = get_poi_definition(id).unwrap();
            assert_eq!(
                poi.use_type,
                UseType::Repeatable,
                "L{} should be Repeatable",
                id
            );
        }
    }

    #[test]
    fn test_repeatable_per_tool_poi() {
        let poi = get_poi_definition(4).unwrap();
        assert_eq!(poi.use_type, UseType::RepeatablePerTool);
    }

    #[test]
    fn test_pois_by_rarity() {
        let fixed = get_pois_by_rarity(PoiRarity::Fixed);
        let common = get_pois_by_rarity(PoiRarity::Common);
        let uncommon = get_pois_by_rarity(PoiRarity::Uncommon);
        let rare = get_pois_by_rarity(PoiRarity::Rare);

        assert_eq!(fixed.len(), 1, "Should have 1 Fixed POI (L1)");
        assert_eq!(
            common.len(),
            4,
            "Should have 4 Common POIs (L2, L4, L5, L6)"
        );
        assert_eq!(uncommon.len(), 7, "Should have 7 Uncommon POIs");
        assert_eq!(rare.len(), 2, "Should have 2 Rare POIs (L11, L12)");
    }

    #[test]
    fn test_item_pois() {
        let item_pois = get_item_pois();
        assert_eq!(
            item_pois.len(),
            4,
            "Should have 4 item POIs (L2, L3, L12, L13)"
        );

        let ids: Vec<u8> = item_pois.iter().map(|p| p.id).collect();
        assert!(ids.contains(&2));
        assert!(ids.contains(&3));
        assert!(ids.contains(&12));
        assert!(ids.contains(&13));
    }
}
