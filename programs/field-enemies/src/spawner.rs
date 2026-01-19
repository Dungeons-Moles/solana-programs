// Spawner module - Enemy spawning logic
// Implementation in US2 (T030-T031) and US4 (T059-T060)

use crate::constants::{
    get_biome_weights, get_spawn_count, get_tier_distribution, ARCHETYPE_COUNT,
};
use crate::state::EnemyTier;

/// Sample a tier based on act distribution using a random value 0-99
pub fn sample_tier(random_value: u8, act: u8) -> EnemyTier {
    let dist = get_tier_distribution(act);
    let value = random_value % 100;

    if value < dist.t1_threshold {
        EnemyTier::T1
    } else if value < dist.t2_threshold {
        EnemyTier::T2
    } else {
        EnemyTier::T3
    }
}

/// Sample an archetype based on biome weights using a random value
pub fn sample_archetype(random_value: u8, act: u8) -> u8 {
    let weights = get_biome_weights(act);
    let total_weight: u16 = weights.iter().map(|&w| w as u16).sum();
    let mut threshold = (random_value as u16 * total_weight) / 256;

    for (id, &weight) in weights.iter().enumerate() {
        if threshold < weight as u16 {
            return id as u8;
        }
        threshold -= weight as u16;
    }

    // Fallback to last archetype
    (ARCHETYPE_COUNT - 1) as u8
}

/// Get the spawn count for an act
pub fn spawn_count_for_act(act: u8) -> u8 {
    get_spawn_count(act)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_tier_act1() {
        // Act 1: 70% T1, 25% T2, 5% T3
        assert_eq!(sample_tier(0, 1), EnemyTier::T1); // 0 < 70
        assert_eq!(sample_tier(69, 1), EnemyTier::T1); // 69 < 70
        assert_eq!(sample_tier(70, 1), EnemyTier::T2); // 70 >= 70, < 95
        assert_eq!(sample_tier(94, 1), EnemyTier::T2); // 94 < 95
        assert_eq!(sample_tier(95, 1), EnemyTier::T3); // 95 >= 95
        assert_eq!(sample_tier(99, 1), EnemyTier::T3); // 99 >= 95
    }

    #[test]
    fn test_sample_tier_act4() {
        // Act 4: 35% T1, 45% T2, 20% T3
        assert_eq!(sample_tier(0, 4), EnemyTier::T1); // 0 < 35
        assert_eq!(sample_tier(34, 4), EnemyTier::T1); // 34 < 35
        assert_eq!(sample_tier(35, 4), EnemyTier::T2); // 35 >= 35, < 80
        assert_eq!(sample_tier(79, 4), EnemyTier::T2); // 79 < 80
        assert_eq!(sample_tier(80, 4), EnemyTier::T3); // 80 >= 80
        assert_eq!(sample_tier(99, 4), EnemyTier::T3); // 99 >= 80
    }

    #[test]
    fn test_spawn_count() {
        assert_eq!(spawn_count_for_act(1), 36);
        assert_eq!(spawn_count_for_act(2), 40);
        assert_eq!(spawn_count_for_act(3), 44);
        assert_eq!(spawn_count_for_act(4), 48);
    }

    #[test]
    fn test_sample_archetype_returns_valid_id() {
        for act in 1..=4 {
            for random in 0..=255u8 {
                let archetype = sample_archetype(random, act);
                assert!(
                    archetype < ARCHETYPE_COUNT as u8,
                    "Invalid archetype {} for act {} random {}",
                    archetype,
                    act,
                    random
                );
            }
        }
    }

    #[test]
    fn test_all_archetypes_can_spawn() {
        // Verify all 12 archetypes can be selected in each biome
        let mut found_a = [false; ARCHETYPE_COUNT];
        let mut found_b = [false; ARCHETYPE_COUNT];

        for random in 0..=255u8 {
            let arch_a = sample_archetype(random, 1);
            let arch_b = sample_archetype(random, 2);
            found_a[arch_a as usize] = true;
            found_b[arch_b as usize] = true;
        }

        for id in 0..ARCHETYPE_COUNT {
            assert!(found_a[id], "Archetype {} not reachable in Biome A", id);
            assert!(found_b[id], "Archetype {} not reachable in Biome B", id);
        }
    }
}
