//! POI spawn configuration per act.
//!
//! Contains spawn counts and guarantees per act from GDD Section 12.

use crate::state::PlacementRule;

/// Spawn configuration for a single POI type
#[derive(Clone, Copy, Debug)]
pub struct PoiSpawnEntry {
    /// POI type (1-14)
    pub poi_type: u8,
    /// Number to spawn
    pub count: u8,
}

/// Guaranteed POI spawn with conditions
#[derive(Clone, Copy, Debug)]
pub struct SpawnGuarantee {
    /// POI type to place
    pub poi_type: u8,
    /// Number to place
    pub count: u8,
    /// Week restriction (None = any week)
    pub week: Option<u8>,
    /// 100 = guaranteed, <100 = probability
    pub chance_percent: u8,
    /// Placement strategy
    pub placement: PlacementRule,
}

/// Act spawn configuration
#[derive(Clone, Debug)]
pub struct ActSpawnConfig {
    /// Act number (1-4)
    pub act: u8,
    /// Baseline spawns (poi_type, count)
    pub baseline: &'static [PoiSpawnEntry],
    /// Guaranteed spawns
    pub guarantees: &'static [SpawnGuarantee],
}

// =============================================================================
// Act 1 Configuration
// =============================================================================

const ACT1_BASELINE: &[PoiSpawnEntry] = &[
    PoiSpawnEntry {
        poi_type: 2,
        count: 10,
    }, // L2: Supply Cache x10
    PoiSpawnEntry {
        poi_type: 3,
        count: 2,
    }, // L3: Tool Crate x2
    PoiSpawnEntry {
        poi_type: 4,
        count: 3,
    }, // L4: Tool Oil Rack x3
    PoiSpawnEntry {
        poi_type: 5,
        count: 4,
    }, // L5: Rest Alcove x4
    PoiSpawnEntry {
        poi_type: 6,
        count: 3,
    }, // L6: Survey Beacon x3
    PoiSpawnEntry {
        poi_type: 7,
        count: 2,
    }, // L7: Seismic Scanner x2
    PoiSpawnEntry {
        poi_type: 8,
        count: 2,
    }, // L8: Rail Waypoint x2
    PoiSpawnEntry {
        poi_type: 9,
        count: 1,
    }, // L9: Smuggler Hatch x1
    PoiSpawnEntry {
        poi_type: 10,
        count: 1,
    }, // L10: Rusty Anvil x1
    PoiSpawnEntry {
        poi_type: 14,
        count: 1,
    }, // L14: Scrap Chute x1
];

const ACT1_GUARANTEES: &[SpawnGuarantee] = &[
    SpawnGuarantee {
        poi_type: 1,
        count: 1,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::AdjacentToStart,
    },
    SpawnGuarantee {
        poi_type: 8,
        count: 2,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 13,
        count: 1,
        week: Some(2),
        chance_percent: 50,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 13,
        count: 1,
        week: Some(3),
        chance_percent: 75,
        placement: PlacementRule::Random,
    },
];

// =============================================================================
// Act 2 Configuration
// =============================================================================

const ACT2_BASELINE: &[PoiSpawnEntry] = &[
    PoiSpawnEntry {
        poi_type: 2,
        count: 12,
    }, // L2: Supply Cache x12
    PoiSpawnEntry {
        poi_type: 3,
        count: 3,
    }, // L3: Tool Crate x3
    PoiSpawnEntry {
        poi_type: 4,
        count: 4,
    }, // L4: Tool Oil Rack x4
    PoiSpawnEntry {
        poi_type: 5,
        count: 5,
    }, // L5: Rest Alcove x5
    PoiSpawnEntry {
        poi_type: 6,
        count: 3,
    }, // L6: Survey Beacon x3
    PoiSpawnEntry {
        poi_type: 7,
        count: 2,
    }, // L7: Seismic Scanner x2
    PoiSpawnEntry {
        poi_type: 8,
        count: 3,
    }, // L8: Rail Waypoint x3
    PoiSpawnEntry {
        poi_type: 9,
        count: 1,
    }, // L9: Smuggler Hatch x1
    PoiSpawnEntry {
        poi_type: 10,
        count: 2,
    }, // L10: Rusty Anvil x2
    PoiSpawnEntry {
        poi_type: 11,
        count: 1,
    }, // L11: Rune Kiln x1
    PoiSpawnEntry {
        poi_type: 14,
        count: 2,
    }, // L14: Scrap Chute x2
];

const ACT2_GUARANTEES: &[SpawnGuarantee] = &[
    SpawnGuarantee {
        poi_type: 1,
        count: 1,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::AdjacentToStart,
    },
    SpawnGuarantee {
        poi_type: 8,
        count: 3,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 12,
        count: 1,
        week: Some(3),
        chance_percent: 50,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 13,
        count: 1,
        week: Some(1),
        chance_percent: 50,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 13,
        count: 1,
        week: Some(2),
        chance_percent: 75,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 13,
        count: 1,
        week: Some(3),
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
];

// =============================================================================
// Act 3 Configuration
// =============================================================================

const ACT3_BASELINE: &[PoiSpawnEntry] = &[
    PoiSpawnEntry {
        poi_type: 2,
        count: 14,
    }, // L2: Supply Cache x14
    PoiSpawnEntry {
        poi_type: 3,
        count: 4,
    }, // L3: Tool Crate x4
    PoiSpawnEntry {
        poi_type: 4,
        count: 4,
    }, // L4: Tool Oil Rack x4
    PoiSpawnEntry {
        poi_type: 5,
        count: 5,
    }, // L5: Rest Alcove x5
    PoiSpawnEntry {
        poi_type: 6,
        count: 4,
    }, // L6: Survey Beacon x4
    PoiSpawnEntry {
        poi_type: 7,
        count: 3,
    }, // L7: Seismic Scanner x3
    PoiSpawnEntry {
        poi_type: 8,
        count: 4,
    }, // L8: Rail Waypoint x4
    PoiSpawnEntry {
        poi_type: 9,
        count: 2,
    }, // L9: Smuggler Hatch x2
    PoiSpawnEntry {
        poi_type: 10,
        count: 2,
    }, // L10: Rusty Anvil x2
    PoiSpawnEntry {
        poi_type: 11,
        count: 1,
    }, // L11: Rune Kiln x1
    PoiSpawnEntry {
        poi_type: 12,
        count: 1,
    }, // L12: Geode Vault x1
    PoiSpawnEntry {
        poi_type: 14,
        count: 2,
    }, // L14: Scrap Chute x2
];

const ACT3_GUARANTEES: &[SpawnGuarantee] = &[
    SpawnGuarantee {
        poi_type: 1,
        count: 1,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::AdjacentToStart,
    },
    SpawnGuarantee {
        poi_type: 8,
        count: 4,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 12,
        count: 1,
        week: Some(2),
        chance_percent: 50,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 12,
        count: 1,
        week: Some(3),
        chance_percent: 75,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 13,
        count: 1,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
];

// =============================================================================
// Act 4 Configuration
// =============================================================================

const ACT4_BASELINE: &[PoiSpawnEntry] = &[
    PoiSpawnEntry {
        poi_type: 2,
        count: 16,
    }, // L2: Supply Cache x16
    PoiSpawnEntry {
        poi_type: 3,
        count: 5,
    }, // L3: Tool Crate x5
    PoiSpawnEntry {
        poi_type: 4,
        count: 5,
    }, // L4: Tool Oil Rack x5
    PoiSpawnEntry {
        poi_type: 5,
        count: 6,
    }, // L5: Rest Alcove x6
    PoiSpawnEntry {
        poi_type: 6,
        count: 4,
    }, // L6: Survey Beacon x4
    PoiSpawnEntry {
        poi_type: 7,
        count: 3,
    }, // L7: Seismic Scanner x3
    PoiSpawnEntry {
        poi_type: 8,
        count: 5,
    }, // L8: Rail Waypoint x5
    PoiSpawnEntry {
        poi_type: 9,
        count: 2,
    }, // L9: Smuggler Hatch x2
    PoiSpawnEntry {
        poi_type: 10,
        count: 2,
    }, // L10: Rusty Anvil x2
    PoiSpawnEntry {
        poi_type: 11,
        count: 2,
    }, // L11: Rune Kiln x2
    PoiSpawnEntry {
        poi_type: 12,
        count: 2,
    }, // L12: Geode Vault x2
    PoiSpawnEntry {
        poi_type: 14,
        count: 3,
    }, // L14: Scrap Chute x3
];

const ACT4_GUARANTEES: &[SpawnGuarantee] = &[
    SpawnGuarantee {
        poi_type: 1,
        count: 1,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::AdjacentToStart,
    },
    SpawnGuarantee {
        poi_type: 8,
        count: 5,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 12,
        count: 1,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
    SpawnGuarantee {
        poi_type: 13,
        count: 2,
        week: None,
        chance_percent: 100,
        placement: PlacementRule::Random,
    },
];

// =============================================================================
// Spawn Configuration Registry
// =============================================================================

/// All act spawn configurations
pub const SPAWN_CONFIGS: &[ActSpawnConfig] = &[
    ActSpawnConfig {
        act: 1,
        baseline: ACT1_BASELINE,
        guarantees: ACT1_GUARANTEES,
    },
    ActSpawnConfig {
        act: 2,
        baseline: ACT2_BASELINE,
        guarantees: ACT2_GUARANTEES,
    },
    ActSpawnConfig {
        act: 3,
        baseline: ACT3_BASELINE,
        guarantees: ACT3_GUARANTEES,
    },
    ActSpawnConfig {
        act: 4,
        baseline: ACT4_BASELINE,
        guarantees: ACT4_GUARANTEES,
    },
];

// =============================================================================
// Query Functions
// =============================================================================

/// Get spawn configuration for an act (1-4)
pub fn get_spawn_config(act: u8) -> Option<&'static ActSpawnConfig> {
    if (1..=4).contains(&act) {
        Some(&SPAWN_CONFIGS[(act - 1) as usize])
    } else {
        None
    }
}

/// Get total baseline POI count for an act
pub fn get_baseline_poi_count(act: u8) -> u8 {
    get_spawn_config(act)
        .map(|config| config.baseline.iter().map(|e| e.count).sum())
        .unwrap_or(0)
}

/// Get guaranteed spawns for a specific week in an act
pub fn get_week_guarantees(act: u8, week: u8) -> Vec<&'static SpawnGuarantee> {
    get_spawn_config(act)
        .map(|config| {
            config
                .guarantees
                .iter()
                .filter(|g| g.week.is_none() || g.week == Some(week))
                .collect()
        })
        .unwrap_or_default()
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_spawn_config_valid() {
        for act in 1..=4 {
            let config = get_spawn_config(act);
            assert!(config.is_some(), "Act {} should have config", act);
            assert_eq!(config.unwrap().act, act);
        }
    }

    #[test]
    fn test_get_spawn_config_invalid() {
        assert!(get_spawn_config(0).is_none());
        assert!(get_spawn_config(5).is_none());
    }

    #[test]
    fn test_act1_baseline_spawns() {
        let config = get_spawn_config(1).unwrap();

        // Check Supply Cache count
        let supply_cache = config.baseline.iter().find(|e| e.poi_type == 2);
        assert!(supply_cache.is_some());
        assert_eq!(supply_cache.unwrap().count, 10);

        // Check Tool Crate count
        let tool_crate = config.baseline.iter().find(|e| e.poi_type == 3);
        assert!(tool_crate.is_some());
        assert_eq!(tool_crate.unwrap().count, 2);
    }

    #[test]
    fn test_act1_guarantees() {
        let config = get_spawn_config(1).unwrap();

        // Mole Den should be guaranteed adjacent to start
        let mole_den = config.guarantees.iter().find(|g| g.poi_type == 1);
        assert!(mole_den.is_some());
        let mole_den = mole_den.unwrap();
        assert_eq!(mole_den.chance_percent, 100);
        assert_eq!(mole_den.placement, PlacementRule::AdjacentToStart);
    }

    #[test]
    fn test_counter_cache_week_based_chances() {
        let config = get_spawn_config(1).unwrap();

        // Week 2: 50% chance
        let week2_counter = config
            .guarantees
            .iter()
            .find(|g| g.poi_type == 13 && g.week == Some(2));
        assert!(week2_counter.is_some());
        assert_eq!(week2_counter.unwrap().chance_percent, 50);

        // Week 3: 75% chance
        let week3_counter = config
            .guarantees
            .iter()
            .find(|g| g.poi_type == 13 && g.week == Some(3));
        assert!(week3_counter.is_some());
        assert_eq!(week3_counter.unwrap().chance_percent, 75);
    }

    #[test]
    fn test_all_acts_have_configs() {
        for act in 1..=4 {
            let config = get_spawn_config(act).unwrap();
            assert!(
                !config.baseline.is_empty(),
                "Act {} should have baseline spawns",
                act
            );
            assert!(
                !config.guarantees.is_empty(),
                "Act {} should have guarantees",
                act
            );
        }
    }

    #[test]
    fn test_baseline_poi_count_increases_with_act() {
        let act1_count = get_baseline_poi_count(1);
        let act2_count = get_baseline_poi_count(2);
        let act3_count = get_baseline_poi_count(3);
        let act4_count = get_baseline_poi_count(4);

        assert!(
            act2_count > act1_count,
            "Act 2 should have more POIs than Act 1"
        );
        assert!(
            act3_count > act2_count,
            "Act 3 should have more POIs than Act 2"
        );
        assert!(
            act4_count > act3_count,
            "Act 4 should have more POIs than Act 3"
        );
    }

    #[test]
    fn test_get_week_guarantees() {
        let guarantees = get_week_guarantees(1, 2);

        // Should include Mole Den (week: None) and Counter Cache (week: 2)
        assert!(
            guarantees.iter().any(|g| g.poi_type == 1),
            "Should include Mole Den"
        );
        assert!(
            guarantees.iter().any(|g| g.poi_type == 13),
            "Should include Counter Cache"
        );
    }

    #[test]
    fn test_rail_waypoint_count_increases() {
        let act1_config = get_spawn_config(1).unwrap();
        let act4_config = get_spawn_config(4).unwrap();

        let act1_waypoints = act1_config
            .guarantees
            .iter()
            .filter(|g| g.poi_type == 8)
            .map(|g| g.count)
            .sum::<u8>();
        let act4_waypoints = act4_config
            .guarantees
            .iter()
            .filter(|g| g.poi_type == 8)
            .map(|g| g.count)
            .sum::<u8>();

        assert!(
            act4_waypoints > act1_waypoints,
            "Act 4 should have more waypoints"
        );
    }
}
