use crate::{bosses::*, Biome, Week};

/// Campaign has 40 levels total (4 acts × 10 levels each)
pub const LEVELS_PER_ACT: u8 = 10;

/// Calculate which act (0-3) based on campaign level (1-40)
pub fn calculate_act(level: u8) -> u8 {
    (level - 1) / LEVELS_PER_ACT
}

/// Calculate level within the current act (1-10)
pub fn calculate_stage_in_act(level: u8) -> u8 {
    (level - 1) % LEVELS_PER_ACT + 1
}

/// Get biome for an act (even acts = A, odd acts = B)
#[allow(clippy::manual_is_multiple_of)]
pub fn get_biome_for_act(act: u8) -> Biome {
    if act % 2 == 0 {
        Biome::A
    } else {
        Biome::B
    }
}

/// Select Week 1 boss index based on level in act
/// Cycles through 0-4 for levels 1-10 (repeats at level 6)
pub fn select_week1_boss_index(level_in_act: u8) -> u8 {
    (level_in_act - 1) % 5
}

/// Select Week 2 boss index based on stage in act
/// Offset by 2 to avoid same pairing as Week 1
pub fn select_week2_boss_index(stage_in_act: u8) -> u8 {
    ((stage_in_act - 1) + 2) % 5
}

/// Select Week 3 boss index based on stage in act
/// Odd stages = Final 1, Even stages = Final 2
pub fn select_week3_boss_index(stage_in_act: u8) -> u8 {
    if stage_in_act % 2 == 1 {
        0 // Odd stages get Final 1
    } else {
        1 // Even stages get Final 2
    }
}

/// Main boss selection function
/// Returns the appropriate boss for a given stage and week
pub fn select_boss(stage: u8, week: Week) -> &'static BossDefinition {
    let act = calculate_act(stage);
    let stage_in_act = calculate_stage_in_act(stage);
    let biome = get_biome_for_act(act);

    let boss_index = match week {
        Week::One => select_week1_boss_index(stage_in_act),
        Week::Two => select_week2_boss_index(stage_in_act),
        Week::Three => select_week3_boss_index(stage_in_act),
    };

    // Get the boss from our registry
    // Safe to unwrap: all 24 bosses exist, inputs validated at instruction level
    get_boss_by_index(biome, week, boss_index).expect("Boss should exist for valid stage and week")
}

#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[inline]
fn duel_week1_pool(index: u8) -> &'static BossDefinition {
    match index {
        0 => &BROODMOTHER_A,
        1 => &OBSIDIAN_GOLEM_A,
        2 => &GAS_ANOMALY_A,
        3 => &MAD_MINER_A,
        4 => &SHARD_COLOSSUS_A,
        _ => unreachable!("duel week1 index must be in 0..=4"),
    }
}

#[inline]
fn duel_week2_pool(index: u8) -> &'static BossDefinition {
    match index {
        0 => &DRILL_SERGEANT_A,
        1 => &CRYSTAL_MIMIC_A,
        2 => &RUST_REGENT_A,
        3 => &POWDER_KEG_BARON_A,
        4 => &GREEDKEEPER_A,
        _ => unreachable!("duel week2 index must be in 0..=4"),
    }
}

/// Deterministically maps seed to one of the 25 week1×week2 duel boss combinations.
/// Week1 index = combo / 5, Week2 index = combo % 5.
pub fn select_duel_week12_indices(seed: u64) -> (u8, u8) {
    let combo = (splitmix64(seed) % 25) as u8;
    (combo / 5, combo % 5)
}

/// Returns duel week 1/2 boss by seed.
/// Week 3 intentionally has no weekly boss (opponent fight).
pub fn select_duel_week_boss(seed: u64, week: Week) -> Option<&'static BossDefinition> {
    let (w1, w2) = select_duel_week12_indices(seed);
    match week {
        Week::One => Some(duel_week1_pool(w1)),
        Week::Two => Some(duel_week2_pool(w2)),
        Week::Three => None,
    }
}

/// VRF-aware duel boss selection using GameRng.
///
/// Uses VRF-backed randomness (via GameRng) instead of the legacy splitmix64 path.
/// Produces the same combo-space (25 = 5x5) mapping as the legacy function.
pub fn select_duel_week12_indices_vrf(rng: &mut vrf_rng::GameRng) -> (u8, u8) {
    let combo = (rng.next_val() % 25) as u8;
    (combo / 5, combo % 5)
}

/// VRF-aware duel week boss selection.
pub fn select_duel_week_boss_vrf(
    rng: &mut vrf_rng::GameRng,
    week: Week,
) -> Option<&'static BossDefinition> {
    let (w1, w2) = select_duel_week12_indices_vrf(rng);
    match week {
        Week::One => Some(duel_week1_pool(w1)),
        Week::Two => Some(duel_week2_pool(w2)),
        Week::Three => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_act() {
        // 40 levels total: 4 acts × 10 levels
        assert_eq!(calculate_act(1), 0); // Level 1 = Act 0
        assert_eq!(calculate_act(10), 0); // Level 10 = Act 0
        assert_eq!(calculate_act(11), 1); // Level 11 = Act 1
        assert_eq!(calculate_act(20), 1); // Level 20 = Act 1
        assert_eq!(calculate_act(21), 2); // Level 21 = Act 2
        assert_eq!(calculate_act(30), 2); // Level 30 = Act 2
        assert_eq!(calculate_act(31), 3); // Level 31 = Act 3
        assert_eq!(calculate_act(40), 3); // Level 40 = Act 3
    }

    #[test]
    fn test_calculate_stage_in_act() {
        // 10 levels per act
        assert_eq!(calculate_stage_in_act(1), 1);
        assert_eq!(calculate_stage_in_act(10), 10);
        assert_eq!(calculate_stage_in_act(11), 1);
        assert_eq!(calculate_stage_in_act(20), 10);
        assert_eq!(calculate_stage_in_act(40), 10);
    }

    #[test]
    fn test_get_biome_for_act() {
        assert_eq!(get_biome_for_act(0), Biome::A); // Act 1 = Biome A
        assert_eq!(get_biome_for_act(1), Biome::B); // Act 2 = Biome B
        assert_eq!(get_biome_for_act(2), Biome::A); // Act 3 = Biome A+
        assert_eq!(get_biome_for_act(3), Biome::B); // Act 4 = Biome B+
    }

    #[test]
    fn test_week1_boss_cycling() {
        // Week 1 bosses cycle 0-4 repeatedly (5 bosses, 10 levels per act)
        assert_eq!(select_week1_boss_index(1), 0);
        assert_eq!(select_week1_boss_index(2), 1);
        assert_eq!(select_week1_boss_index(3), 2);
        assert_eq!(select_week1_boss_index(4), 3);
        assert_eq!(select_week1_boss_index(5), 4);
        assert_eq!(select_week1_boss_index(6), 0); // Cycle repeats
        assert_eq!(select_week1_boss_index(10), 4); // Level 10 = index 4
    }

    #[test]
    fn test_week2_boss_offset() {
        // Week 2 bosses are offset by 2 from Week 1
        assert_eq!(select_week2_boss_index(1), 2); // Stage 1: offset 2
        assert_eq!(select_week2_boss_index(2), 3);
        assert_eq!(select_week2_boss_index(3), 4);
        assert_eq!(select_week2_boss_index(4), 0); // Wraps around
        assert_eq!(select_week2_boss_index(5), 1);
    }

    #[test]
    fn test_week3_final_alternation() {
        // Week 3: odd levels = Final 1 (index 0), even levels = Final 2 (index 1)
        assert_eq!(select_week3_boss_index(1), 0); // Odd = Final 1
        assert_eq!(select_week3_boss_index(2), 1); // Even = Final 2
        assert_eq!(select_week3_boss_index(3), 0);
        assert_eq!(select_week3_boss_index(4), 1);
        assert_eq!(select_week3_boss_index(9), 0);
        assert_eq!(select_week3_boss_index(10), 1);
    }

    #[test]
    fn test_select_boss_act1_week1() {
        // Level 1, Week 1: Biome A, boss index 0 = Broodmother
        let boss = select_boss(1, Week::One);
        assert_eq!(boss.name, "The Broodmother");
        assert_eq!(boss.biome, Biome::A);
    }

    #[test]
    fn test_select_boss_act2_week1() {
        // Level 11, Week 1: Biome B, boss index 0 = Broodmother B
        let boss = select_boss(11, Week::One);
        assert_eq!(boss.name, "The Broodmother");
        assert_eq!(boss.biome, Biome::B);
        assert_eq!(boss.base_stats.spd, 3); // +1 SPD in Biome B
    }

    #[test]
    fn test_select_boss_week3_finals() {
        // Level 1, Week 3: Biome A, odd = Final 1 (Eldritch Mole)
        let boss = select_boss(1, Week::Three);
        assert_eq!(boss.name, "The Eldritch Mole");

        // Level 2, Week 3: Biome A, even = Final 2 (Gilded Devourer)
        let boss = select_boss(2, Week::Three);
        assert_eq!(boss.name, "The Gilded Devourer");

        // Level 11, Week 3: Biome B, odd = Final 1 (Frostbound Leviathan)
        let boss = select_boss(11, Week::Three);
        assert_eq!(boss.name, "The Frostbound Leviathan");

        // Level 12, Week 3: Biome B, even = Final 2 (Rusted Chronomancer)
        let boss = select_boss(12, Week::Three);
        assert_eq!(boss.name, "The Rusted Chronomancer");
    }

    #[test]
    fn test_duel_week12_indices_deterministic() {
        let a = select_duel_week12_indices(42);
        let b = select_duel_week12_indices(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_duel_week12_all_combinations_reachable() {
        let mut seen = [false; 25];
        for seed in 0..20_000u64 {
            let (w1, w2) = select_duel_week12_indices(seed);
            let idx = (w1 as usize) * 5 + (w2 as usize);
            seen[idx] = true;
        }
        assert!(seen.iter().all(|v| *v));
    }

    #[test]
    fn test_duel_week3_has_no_boss() {
        assert!(select_duel_week_boss(123, Week::Three).is_none());
    }

    // =========================================================================
    // VRF-aware Duel Selection Tests
    // =========================================================================

    #[test]
    fn test_duel_vrf_deterministic() {
        let mut rng1 = vrf_rng::GameRng::from_seed(42);
        let mut rng2 = vrf_rng::GameRng::from_seed(42);
        assert_eq!(
            select_duel_week12_indices_vrf(&mut rng1),
            select_duel_week12_indices_vrf(&mut rng2)
        );
    }

    #[test]
    fn test_duel_vrf_all_combinations_reachable() {
        let mut seen = [false; 25];
        for seed in 0..20_000u64 {
            let mut rng = vrf_rng::GameRng::from_seed(seed.max(1));
            let (w1, w2) = select_duel_week12_indices_vrf(&mut rng);
            let idx = (w1 as usize) * 5 + (w2 as usize);
            seen[idx] = true;
        }
        assert!(seen.iter().all(|v| *v));
    }

    #[test]
    fn test_duel_vrf_week3_no_boss() {
        let mut rng = vrf_rng::GameRng::from_seed(42);
        assert!(select_duel_week_boss_vrf(&mut rng, Week::Three).is_none());
    }
}
