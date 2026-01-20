use crate::{bosses::*, Biome, Week};

/// Calculate which act (0-3) based on stage number
pub fn calculate_act(stage: u8) -> u8 {
    (stage - 1) / 20
}

/// Calculate stage within the current act (1-20)
pub fn calculate_stage_in_act(stage: u8) -> u8 {
    (stage - 1) % 20 + 1
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

/// Select Week 1 boss index based on stage in act
/// Cycles through 0-4 for stages 1-20
pub fn select_week1_boss_index(stage_in_act: u8) -> u8 {
    (stage_in_act - 1) % 5
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
    get_boss_by_index(biome, week, boss_index).expect("Boss should exist for valid stage and week")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_act() {
        assert_eq!(calculate_act(1), 0); // Stage 1 = Act 0
        assert_eq!(calculate_act(20), 0); // Stage 20 = Act 0
        assert_eq!(calculate_act(21), 1); // Stage 21 = Act 1
        assert_eq!(calculate_act(40), 1); // Stage 40 = Act 1
        assert_eq!(calculate_act(41), 2); // Stage 41 = Act 2
        assert_eq!(calculate_act(60), 2); // Stage 60 = Act 2
        assert_eq!(calculate_act(61), 3); // Stage 61 = Act 3
        assert_eq!(calculate_act(80), 3); // Stage 80 = Act 3
    }

    #[test]
    fn test_calculate_stage_in_act() {
        assert_eq!(calculate_stage_in_act(1), 1);
        assert_eq!(calculate_stage_in_act(20), 20);
        assert_eq!(calculate_stage_in_act(21), 1);
        assert_eq!(calculate_stage_in_act(40), 20);
        assert_eq!(calculate_stage_in_act(80), 20);
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
        // Week 1 bosses cycle 0-4 repeatedly
        assert_eq!(select_week1_boss_index(1), 0);
        assert_eq!(select_week1_boss_index(2), 1);
        assert_eq!(select_week1_boss_index(3), 2);
        assert_eq!(select_week1_boss_index(4), 3);
        assert_eq!(select_week1_boss_index(5), 4);
        assert_eq!(select_week1_boss_index(6), 0); // Cycle repeats
        assert_eq!(select_week1_boss_index(20), 4); // Stage 20 = index 4
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
        // Week 3: odd stages = Final 1 (index 0), even stages = Final 2 (index 1)
        assert_eq!(select_week3_boss_index(1), 0); // Odd = Final 1
        assert_eq!(select_week3_boss_index(2), 1); // Even = Final 2
        assert_eq!(select_week3_boss_index(3), 0);
        assert_eq!(select_week3_boss_index(4), 1);
        assert_eq!(select_week3_boss_index(19), 0);
        assert_eq!(select_week3_boss_index(20), 1);
    }

    #[test]
    fn test_select_boss_act1_week1() {
        // Stage 1, Week 1: Biome A, boss index 0 = Broodmother
        let boss = select_boss(1, Week::One);
        assert_eq!(boss.name, "The Broodmother");
        assert_eq!(boss.biome, Biome::A);
    }

    #[test]
    fn test_select_boss_act2_week1() {
        // Stage 21, Week 1: Biome B, boss index 0 = Broodmother B
        let boss = select_boss(21, Week::One);
        assert_eq!(boss.name, "The Broodmother");
        assert_eq!(boss.biome, Biome::B);
        assert_eq!(boss.base_stats.spd, 4); // +1 SPD in Biome B
    }

    #[test]
    fn test_select_boss_week3_finals() {
        // Stage 1, Week 3: Biome A, odd = Final 1 (Eldritch Mole)
        let boss = select_boss(1, Week::Three);
        assert_eq!(boss.name, "The Eldritch Mole");

        // Stage 2, Week 3: Biome A, even = Final 2 (Gilded Devourer)
        let boss = select_boss(2, Week::Three);
        assert_eq!(boss.name, "The Gilded Devourer");

        // Stage 21, Week 3: Biome B, odd = Final 1 (Frostbound Leviathan)
        let boss = select_boss(21, Week::Three);
        assert_eq!(boss.name, "The Frostbound Leviathan");

        // Stage 22, Week 3: Biome B, even = Final 2 (Rusted Chronomancer)
        let boss = select_boss(22, Week::Three);
        assert_eq!(boss.name, "The Rusted Chronomancer");
    }
}
