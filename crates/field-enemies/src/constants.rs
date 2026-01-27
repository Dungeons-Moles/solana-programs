/// Number of enemy archetypes (12 field enemies per GDD)
pub const ARCHETYPE_COUNT: usize = 12;

/// Number of tiers per archetype
pub const TIER_COUNT: usize = 3;

/// Tier distribution thresholds per act
/// Sample 0-99: if < t1_threshold => T1, if < t2_threshold => T2, else T3
#[derive(Clone, Copy, Debug)]
pub struct TierDistribution {
    pub t1_threshold: u8,
    pub t2_threshold: u8,
}

/// Tier distributions by act (0-indexed: Act 1 = index 0)
/// Act 1: 70% T1, 25% T2, 5% T3
/// Act 2: 55% T1, 35% T2, 10% T3
/// Act 3: 45% T1, 40% T2, 15% T3
/// Act 4: 35% T1, 45% T2, 20% T3
pub const TIER_DISTRIBUTIONS: [TierDistribution; 4] = [
    TierDistribution {
        t1_threshold: 70,
        t2_threshold: 95,
    }, // Act 1
    TierDistribution {
        t1_threshold: 55,
        t2_threshold: 90,
    }, // Act 2
    TierDistribution {
        t1_threshold: 45,
        t2_threshold: 85,
    }, // Act 3
    TierDistribution {
        t1_threshold: 35,
        t2_threshold: 80,
    }, // Act 4
];

/// Enemy spawn counts per act
pub const SPAWN_COUNTS: [u8; 4] = [36, 40, 44, 48];

/// Biome A weights (Acts 1 & 3)
/// Emphasized: Tunnel Rat (0), Collapsed Miner (4), Shard Beetle (5), Coin Slug (10)
pub const BIOME_A_WEIGHTS: [u8; ARCHETYPE_COUNT] = [
    2, // 0: Tunnel Rat (emphasized)
    1, // 1: Cave Bat
    1, // 2: Spore Slime
    1, // 3: Rust Mite Swarm
    2, // 4: Collapsed Miner (emphasized)
    2, // 5: Shard Beetle (emphasized)
    1, // 6: Tunnel Warden
    1, // 7: Burrow Ambusher
    1, // 8: Frost Wisp
    1, // 9: Powder Tick
    2, // 10: Coin Slug (emphasized)
    1, // 11: Blood Mosquito
];

/// Biome B weights (Acts 2 & 4)
/// Emphasized: Rust Mite (3), Burrow Ambusher (7), Frost Wisp (8), Powder Tick (9), Blood Mosquito (11)
pub const BIOME_B_WEIGHTS: [u8; ARCHETYPE_COUNT] = [
    1, // 0: Tunnel Rat
    1, // 1: Cave Bat
    1, // 2: Spore Slime
    2, // 3: Rust Mite Swarm (emphasized)
    1, // 4: Collapsed Miner
    1, // 5: Shard Beetle
    1, // 6: Tunnel Warden
    2, // 7: Burrow Ambusher (emphasized)
    2, // 8: Frost Wisp (emphasized)
    2, // 9: Powder Tick (emphasized)
    1, // 10: Coin Slug
    2, // 11: Blood Mosquito (emphasized)
];

/// Get biome weights for an act (1-indexed)
pub fn get_biome_weights(act: u8) -> &'static [u8; ARCHETYPE_COUNT] {
    match act {
        1 | 3 => &BIOME_A_WEIGHTS,
        2 | 4 => &BIOME_B_WEIGHTS,
        _ => &BIOME_A_WEIGHTS, // fallback
    }
}

/// Get spawn count for an act (1-indexed)
pub fn get_spawn_count(act: u8) -> u8 {
    if act >= 1 && act <= 4 {
        SPAWN_COUNTS[(act - 1) as usize]
    } else {
        SPAWN_COUNTS[0] // fallback to Act 1
    }
}

/// Get tier distribution for an act (1-indexed)
pub fn get_tier_distribution(act: u8) -> &'static TierDistribution {
    if act >= 1 && act <= 4 {
        &TIER_DISTRIBUTIONS[(act - 1) as usize]
    } else {
        &TIER_DISTRIBUTIONS[0] // fallback to Act 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_counts() {
        assert_eq!(get_spawn_count(1), 36);
        assert_eq!(get_spawn_count(2), 40);
        assert_eq!(get_spawn_count(3), 44);
        assert_eq!(get_spawn_count(4), 48);
    }

    #[test]
    fn test_biome_weights() {
        // Biome A (Acts 1, 3) - check emphasized archetypes
        let biome_a = get_biome_weights(1);
        assert_eq!(biome_a[0], 2); // Tunnel Rat emphasized
        assert_eq!(biome_a[4], 2); // Collapsed Miner emphasized
        assert_eq!(biome_a[5], 2); // Shard Beetle emphasized
        assert_eq!(biome_a[10], 2); // Coin Slug emphasized

        // Biome B (Acts 2, 4) - check emphasized archetypes
        let biome_b = get_biome_weights(2);
        assert_eq!(biome_b[3], 2); // Rust Mite emphasized
        assert_eq!(biome_b[7], 2); // Burrow Ambusher emphasized
        assert_eq!(biome_b[8], 2); // Frost Wisp emphasized
        assert_eq!(biome_b[9], 2); // Powder Tick emphasized
        assert_eq!(biome_b[11], 2); // Blood Mosquito emphasized
    }
}
