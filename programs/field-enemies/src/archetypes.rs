// Archetypes module - Enemy archetype definitions
// Implementation in US1 (T017-T019) and US5 (T066-T079)

use crate::state::EnemyStats;
use combat_system::state::CombatantInput;

/// Number of enemy archetypes
pub const ARCHETYPE_COUNT: usize = 12;

/// Enemy archetype IDs
pub mod ids {
    pub const TUNNEL_RAT: u8 = 0;
    pub const CAVE_BAT: u8 = 1;
    pub const SPORE_SLIME: u8 = 2;
    pub const RUST_MITE_SWARM: u8 = 3;
    pub const COLLAPSED_MINER: u8 = 4;
    pub const SHARD_BEETLE: u8 = 5;
    pub const TUNNEL_WARDEN: u8 = 6;
    pub const BURROW_AMBUSHER: u8 = 7;
    pub const FROST_WISP: u8 = 8;
    pub const POWDER_TICK: u8 = 9;
    pub const COIN_SLUG: u8 = 10;
    pub const BLOOD_MOSQUITO: u8 = 11;
}

/// Enemy archetype metadata
#[derive(Clone, Copy, Debug)]
pub struct EnemyArchetype {
    pub id: u8,
    pub name: &'static str,
    pub emoji: &'static str,
    pub biome_a_weight: u8,
    pub biome_b_weight: u8,
}

/// Static array of all enemy archetypes
pub static ENEMY_ARCHETYPES: [EnemyArchetype; ARCHETYPE_COUNT] = [
    EnemyArchetype {
        id: 0,
        name: "Tunnel Rat",
        emoji: "🐀",
        biome_a_weight: 2,
        biome_b_weight: 1,
    },
    EnemyArchetype {
        id: 1,
        name: "Cave Bat",
        emoji: "🦇",
        biome_a_weight: 1,
        biome_b_weight: 1,
    },
    EnemyArchetype {
        id: 2,
        name: "Spore Slime",
        emoji: "🟢",
        biome_a_weight: 1,
        biome_b_weight: 1,
    },
    EnemyArchetype {
        id: 3,
        name: "Rust Mite Swarm",
        emoji: "🐜",
        biome_a_weight: 1,
        biome_b_weight: 2,
    },
    EnemyArchetype {
        id: 4,
        name: "Collapsed Miner",
        emoji: "🧟",
        biome_a_weight: 2,
        biome_b_weight: 1,
    },
    EnemyArchetype {
        id: 5,
        name: "Shard Beetle",
        emoji: "🪲",
        biome_a_weight: 2,
        biome_b_weight: 1,
    },
    EnemyArchetype {
        id: 6,
        name: "Tunnel Warden",
        emoji: "🦀",
        biome_a_weight: 1,
        biome_b_weight: 1,
    },
    EnemyArchetype {
        id: 7,
        name: "Burrow Ambusher",
        emoji: "🦂",
        biome_a_weight: 1,
        biome_b_weight: 2,
    },
    EnemyArchetype {
        id: 8,
        name: "Frost Wisp",
        emoji: "🧊",
        biome_a_weight: 1,
        biome_b_weight: 2,
    },
    EnemyArchetype {
        id: 9,
        name: "Powder Tick",
        emoji: "🧨",
        biome_a_weight: 1,
        biome_b_weight: 2,
    },
    EnemyArchetype {
        id: 10,
        name: "Coin Slug",
        emoji: "🐌",
        biome_a_weight: 2,
        biome_b_weight: 1,
    },
    EnemyArchetype {
        id: 11,
        name: "Blood Mosquito",
        emoji: "🦟",
        biome_a_weight: 1,
        biome_b_weight: 2,
    },
];

/// Stats for each enemy archetype at each tier
/// Indexed by [archetype_id][tier]
/// Stats format: HP/ATK/ARM/SPD/DIG
pub static ENEMY_STATS: [[EnemyStats; 3]; ARCHETYPE_COUNT] = [
    // 0: Tunnel Rat - 5/1/0/3/1, 7/2/0/4/1, 9/3/1/5/2
    [
        EnemyStats {
            hp: 5,
            atk: 1,
            arm: 0,
            spd: 3,
            dig: 1,
        },
        EnemyStats {
            hp: 7,
            atk: 2,
            arm: 0,
            spd: 4,
            dig: 1,
        },
        EnemyStats {
            hp: 9,
            atk: 3,
            arm: 1,
            spd: 5,
            dig: 2,
        },
    ],
    // 1: Cave Bat - 6/1/0/3/1, 8/2/0/4/1, 10/3/0/5/2
    [
        EnemyStats {
            hp: 6,
            atk: 1,
            arm: 0,
            spd: 3,
            dig: 1,
        },
        EnemyStats {
            hp: 8,
            atk: 2,
            arm: 0,
            spd: 4,
            dig: 1,
        },
        EnemyStats {
            hp: 10,
            atk: 3,
            arm: 0,
            spd: 5,
            dig: 2,
        },
    ],
    // 2: Spore Slime - 8/1/2/0/1, 11/2/3/0/1, 14/3/4/0/2
    [
        EnemyStats {
            hp: 8,
            atk: 1,
            arm: 2,
            spd: 0,
            dig: 1,
        },
        EnemyStats {
            hp: 11,
            atk: 2,
            arm: 3,
            spd: 0,
            dig: 1,
        },
        EnemyStats {
            hp: 14,
            atk: 3,
            arm: 4,
            spd: 0,
            dig: 2,
        },
    ],
    // 3: Rust Mite Swarm - 6/1/0/3/2, 9/2/0/4/2, 12/3/0/5/3
    [
        EnemyStats {
            hp: 6,
            atk: 1,
            arm: 0,
            spd: 3,
            dig: 2,
        },
        EnemyStats {
            hp: 9,
            atk: 2,
            arm: 0,
            spd: 4,
            dig: 2,
        },
        EnemyStats {
            hp: 12,
            atk: 3,
            arm: 0,
            spd: 5,
            dig: 3,
        },
    ],
    // 4: Collapsed Miner - 10/2/0/1/3, 14/3/0/2/3, 18/4/1/3/4
    [
        EnemyStats {
            hp: 10,
            atk: 2,
            arm: 0,
            spd: 1,
            dig: 3,
        },
        EnemyStats {
            hp: 14,
            atk: 3,
            arm: 0,
            spd: 2,
            dig: 3,
        },
        EnemyStats {
            hp: 18,
            atk: 4,
            arm: 1,
            spd: 3,
            dig: 4,
        },
    ],
    // 5: Shard Beetle - 9/1/3/1/2, 12/2/4/1/2, 15/3/5/2/3
    [
        EnemyStats {
            hp: 9,
            atk: 1,
            arm: 3,
            spd: 1,
            dig: 2,
        },
        EnemyStats {
            hp: 12,
            atk: 2,
            arm: 4,
            spd: 1,
            dig: 2,
        },
        EnemyStats {
            hp: 15,
            atk: 3,
            arm: 5,
            spd: 2,
            dig: 3,
        },
    ],
    // 6: Tunnel Warden - 8/2/4/2/2, 11/3/6/3/2, 14/4/8/4/3
    [
        EnemyStats {
            hp: 8,
            atk: 2,
            arm: 4,
            spd: 2,
            dig: 2,
        },
        EnemyStats {
            hp: 11,
            atk: 3,
            arm: 6,
            spd: 3,
            dig: 2,
        },
        EnemyStats {
            hp: 14,
            atk: 4,
            arm: 8,
            spd: 4,
            dig: 3,
        },
    ],
    // 7: Burrow Ambusher - 6/3/0/4/2, 9/4/0/5/2, 12/5/0/6/3
    [
        EnemyStats {
            hp: 6,
            atk: 3,
            arm: 0,
            spd: 4,
            dig: 2,
        },
        EnemyStats {
            hp: 9,
            atk: 4,
            arm: 0,
            spd: 5,
            dig: 2,
        },
        EnemyStats {
            hp: 12,
            atk: 5,
            arm: 0,
            spd: 6,
            dig: 3,
        },
    ],
    // 8: Frost Wisp - 7/1/0/4/1, 10/2/0/5/1, 13/3/0/6/2
    [
        EnemyStats {
            hp: 7,
            atk: 1,
            arm: 0,
            spd: 4,
            dig: 1,
        },
        EnemyStats {
            hp: 10,
            atk: 2,
            arm: 0,
            spd: 5,
            dig: 1,
        },
        EnemyStats {
            hp: 13,
            atk: 3,
            arm: 0,
            spd: 6,
            dig: 2,
        },
    ],
    // 9: Powder Tick - 7/1/0/2/1, 10/2/0/3/1, 13/3/0/4/2
    [
        EnemyStats {
            hp: 7,
            atk: 1,
            arm: 0,
            spd: 2,
            dig: 1,
        },
        EnemyStats {
            hp: 10,
            atk: 2,
            arm: 0,
            spd: 3,
            dig: 1,
        },
        EnemyStats {
            hp: 13,
            atk: 3,
            arm: 0,
            spd: 4,
            dig: 2,
        },
    ],
    // 10: Coin Slug - 7/1/2/1/1, 10/2/3/1/1, 13/3/4/2/2
    [
        EnemyStats {
            hp: 7,
            atk: 1,
            arm: 2,
            spd: 1,
            dig: 1,
        },
        EnemyStats {
            hp: 10,
            atk: 2,
            arm: 3,
            spd: 1,
            dig: 1,
        },
        EnemyStats {
            hp: 13,
            atk: 3,
            arm: 4,
            spd: 2,
            dig: 2,
        },
    ],
    // 11: Blood Mosquito - 6/1/0/3/1, 9/2/0/4/1, 12/3/0/5/2
    [
        EnemyStats {
            hp: 6,
            atk: 1,
            arm: 0,
            spd: 3,
            dig: 1,
        },
        EnemyStats {
            hp: 9,
            atk: 2,
            arm: 0,
            spd: 4,
            dig: 1,
        },
        EnemyStats {
            hp: 12,
            atk: 3,
            arm: 0,
            spd: 5,
            dig: 2,
        },
    ],
];

/// Get stats for an enemy by archetype ID and tier
pub fn get_enemy_stats(archetype_id: u8, tier: u8) -> Option<&'static EnemyStats> {
    if archetype_id >= ARCHETYPE_COUNT as u8 || tier > 2 {
        return None;
    }
    Some(&ENEMY_STATS[archetype_id as usize][tier as usize])
}

/// Get combatant input for combat system
pub fn get_enemy_combatant_input(archetype_id: u8, tier: u8) -> Option<CombatantInput> {
    let stats = get_enemy_stats(archetype_id, tier)?;
    Some(CombatantInput {
        hp: stats.hp as i16,
        max_hp: stats.hp,
        atk: stats.atk as i16,
        arm: stats.arm as i16,
        spd: stats.spd as i16,
        dig: stats.dig as i16,
        strikes: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_rat_t1_stats() {
        let stats = get_enemy_stats(ids::TUNNEL_RAT, 0).unwrap();
        assert_eq!(stats.hp, 5);
        assert_eq!(stats.atk, 1);
        assert_eq!(stats.arm, 0);
        assert_eq!(stats.spd, 3);
        assert_eq!(stats.dig, 1);
    }

    #[test]
    fn test_tunnel_warden_t3_stats() {
        let stats = get_enemy_stats(ids::TUNNEL_WARDEN, 2).unwrap();
        assert_eq!(stats.hp, 14);
        assert_eq!(stats.atk, 4);
        assert_eq!(stats.arm, 8);
        assert_eq!(stats.spd, 4);
        assert_eq!(stats.dig, 3);
    }

    #[test]
    fn test_get_enemy_combatant_input() {
        let input = get_enemy_combatant_input(ids::TUNNEL_RAT, 0).unwrap();
        assert_eq!(input.hp, 5);
        assert_eq!(input.max_hp, 5);
        assert_eq!(input.atk, 1);
        assert_eq!(input.arm, 0);
        assert_eq!(input.spd, 3);
        assert_eq!(input.dig, 1);
        assert_eq!(input.strikes, 1);
    }

    #[test]
    fn test_invalid_archetype_id() {
        assert!(get_enemy_stats(12, 0).is_none());
        assert!(get_enemy_stats(255, 0).is_none());
    }

    #[test]
    fn test_invalid_tier() {
        assert!(get_enemy_stats(0, 3).is_none());
        assert!(get_enemy_stats(0, 255).is_none());
    }

    #[test]
    fn test_all_36_configurations() {
        // Verify all 12 archetypes × 3 tiers are valid
        for archetype_id in 0..12u8 {
            for tier in 0..3u8 {
                let stats = get_enemy_stats(archetype_id, tier);
                assert!(
                    stats.is_some(),
                    "Missing stats for archetype {} tier {}",
                    archetype_id,
                    tier
                );
                assert!(
                    stats.unwrap().hp > 0,
                    "HP should be > 0 for archetype {} tier {}",
                    archetype_id,
                    tier
                );
            }
        }
    }
}
