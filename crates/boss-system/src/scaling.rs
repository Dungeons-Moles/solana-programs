use crate::{
    bosses::BossDefinition,
    selection::{calculate_act, calculate_stage_in_act},
    CombatantInput, ScaledBossStats, Week,
};

/// Calculate tier (0-3) based on stage within act
/// Tier 0: stages 1-5, Tier 1: stages 6-10, Tier 2: stages 11-15, Tier 3: stages 16-20
pub fn calculate_tier(stage_in_act: u8) -> u8 {
    (stage_in_act - 1) / 5
}

/// Scale Week 1 boss stats based on tier
/// +2 HP per tier, +1 ARM per tier
pub fn scale_week1_stats(base_hp: u16, base_arm: u8, tier: u8) -> (u16, u8, u8) {
    let hp_bonus = 2 * tier as u16;
    let arm_bonus = tier;
    let atk_bonus = 0u8;

    (base_hp + hp_bonus, base_arm + arm_bonus, atk_bonus)
}

/// Scale Week 2 boss stats based on tier
/// +3 HP per tier, +1 ARM per tier, +1 ATK at tier >= 2
pub fn scale_week2_stats(base_hp: u16, base_arm: u8, tier: u8) -> (u16, u8, u8) {
    let hp_bonus = 3 * tier as u16;
    let arm_bonus = tier;
    let atk_bonus = if tier >= 2 { 1u8 } else { 0u8 };

    (base_hp + hp_bonus, base_arm + arm_bonus, atk_bonus)
}

/// Scale Week 3 boss stats based on tier
/// +4 HP per tier, +1 ARM per tier, +1 ATK at tier >= 1
pub fn scale_week3_stats(base_hp: u16, base_arm: u8, tier: u8) -> (u16, u8, u8) {
    let hp_bonus = 4 * tier as u16;
    let arm_bonus = tier;
    let atk_bonus = if tier >= 1 { 1u8 } else { 0u8 };

    (base_hp + hp_bonus, base_arm + arm_bonus, atk_bonus)
}

/// Apply act baseline bonuses to stats
/// Act 3 (A+): +1 ATK (W1/W2), +2 ATK (W3)
/// Act 4 (B+): +1 ATK +1 SPD (W1/W2), +2 ATK +1 SPD (W3)
pub fn apply_act_baseline(atk: u8, spd: u8, act: u8, week: Week) -> (u8, u8) {
    let mut atk = atk;
    let mut spd = spd;

    match act {
        2 => {
            // Act 3 (A+)
            atk += if week == Week::Three { 2 } else { 1 };
        }
        3 => {
            // Act 4 (B+)
            atk += if week == Week::Three { 2 } else { 1 };
            spd += 1;
        }
        _ => {}
    }

    (atk, spd)
}

/// Main boss scaling function
/// Scales base stats based on stage, act, and tier
pub fn scale_boss(boss: &BossDefinition, stage: u8, week: Week) -> ScaledBossStats {
    let act = calculate_act(stage);
    let stage_in_act = calculate_stage_in_act(stage);
    let tier = calculate_tier(stage_in_act);

    let base = &boss.base_stats;

    // Get tier scaling based on week
    let (scaled_hp, scaled_arm, tier_atk_bonus) = match week {
        Week::One => scale_week1_stats(base.hp, base.arm, tier),
        Week::Two => scale_week2_stats(base.hp, base.arm, tier),
        Week::Three => scale_week3_stats(base.hp, base.arm, tier),
    };

    let scaled_atk = base.atk + tier_atk_bonus;

    // Apply act baseline bonuses
    let (final_atk, final_spd) = apply_act_baseline(scaled_atk, base.spd, act, week);

    ScaledBossStats {
        hp: scaled_hp,
        atk: final_atk,
        arm: scaled_arm,
        spd: final_spd,
        dig: base.dig,
        strikes: boss.get_base_strikes(),
    }
}

/// Convert scaled stats to CombatantInput for combat system
pub fn to_combatant_input(scaled: &ScaledBossStats) -> CombatantInput {
    CombatantInput {
        hp: scaled.hp as i16,
        max_hp: scaled.hp,
        atk: scaled.atk as i16,
        arm: scaled.arm as i16,
        spd: scaled.spd as i16,
        dig: scaled.dig as i16,
        strikes: scaled.strikes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::select_boss;

    #[test]
    fn test_calculate_tier() {
        assert_eq!(calculate_tier(1), 0); // Stage 1 = Tier 0
        assert_eq!(calculate_tier(5), 0); // Stage 5 = Tier 0
        assert_eq!(calculate_tier(6), 1); // Stage 6 = Tier 1
        assert_eq!(calculate_tier(10), 1); // Stage 10 = Tier 1
        assert_eq!(calculate_tier(11), 2); // Stage 11 = Tier 2
        assert_eq!(calculate_tier(15), 2); // Stage 15 = Tier 2
        assert_eq!(calculate_tier(16), 3); // Stage 16 = Tier 3
        assert_eq!(calculate_tier(20), 3); // Stage 20 = Tier 3
    }

    #[test]
    fn test_week1_scaling() {
        // Base HP = 32, ARM = 2 (Broodmother)
        let (hp, arm, atk) = scale_week1_stats(32, 2, 0);
        assert_eq!((hp, arm, atk), (32, 2, 0)); // Tier 0: no bonuses

        let (hp, arm, atk) = scale_week1_stats(32, 2, 1);
        assert_eq!((hp, arm, atk), (34, 3, 0)); // Tier 1: +2 HP, +1 ARM

        let (hp, arm, atk) = scale_week1_stats(32, 2, 2);
        assert_eq!((hp, arm, atk), (36, 4, 0)); // Tier 2: +4 HP, +2 ARM

        let (hp, arm, atk) = scale_week1_stats(32, 2, 3);
        assert_eq!((hp, arm, atk), (38, 5, 0)); // Tier 3: +6 HP, +3 ARM
    }

    #[test]
    fn test_week2_scaling() {
        // Base HP = 46, ARM = 10 (Drill Sergeant)
        let (hp, arm, atk) = scale_week2_stats(46, 10, 0);
        assert_eq!((hp, arm, atk), (46, 10, 0)); // Tier 0: no bonuses

        let (hp, arm, atk) = scale_week2_stats(46, 10, 1);
        assert_eq!((hp, arm, atk), (49, 11, 0)); // Tier 1: +3 HP, +1 ARM

        let (hp, arm, atk) = scale_week2_stats(46, 10, 2);
        assert_eq!((hp, arm, atk), (52, 12, 1)); // Tier 2: +6 HP, +2 ARM, +1 ATK

        let (hp, arm, atk) = scale_week2_stats(46, 10, 3);
        assert_eq!((hp, arm, atk), (55, 13, 1)); // Tier 3: +9 HP, +3 ARM, +1 ATK
    }

    #[test]
    fn test_week3_scaling() {
        // Base HP = 72, ARM = 12 (Eldritch Mole)
        let (hp, arm, atk) = scale_week3_stats(72, 12, 0);
        assert_eq!((hp, arm, atk), (72, 12, 0)); // Tier 0: no bonuses

        let (hp, arm, atk) = scale_week3_stats(72, 12, 1);
        assert_eq!((hp, arm, atk), (76, 13, 1)); // Tier 1: +4 HP, +1 ARM, +1 ATK

        let (hp, arm, atk) = scale_week3_stats(72, 12, 2);
        assert_eq!((hp, arm, atk), (80, 14, 1)); // Tier 2: +8 HP, +2 ARM, +1 ATK

        let (hp, arm, atk) = scale_week3_stats(72, 12, 3);
        assert_eq!((hp, arm, atk), (84, 15, 1)); // Tier 3: +12 HP, +3 ARM, +1 ATK
    }

    #[test]
    fn test_act_baseline_bonuses() {
        // Act 1 and 2: no bonuses
        let (atk, spd) = apply_act_baseline(3, 2, 0, Week::One);
        assert_eq!((atk, spd), (3, 2));

        let (atk, spd) = apply_act_baseline(3, 2, 1, Week::One);
        assert_eq!((atk, spd), (3, 2));

        // Act 3 (A+): +1 ATK for W1/W2, +2 ATK for W3
        let (atk, spd) = apply_act_baseline(3, 2, 2, Week::One);
        assert_eq!((atk, spd), (4, 2));

        let (atk, spd) = apply_act_baseline(3, 2, 2, Week::Three);
        assert_eq!((atk, spd), (5, 2));

        // Act 4 (B+): +1 ATK +1 SPD for W1/W2, +2 ATK +1 SPD for W3
        let (atk, spd) = apply_act_baseline(3, 2, 3, Week::One);
        assert_eq!((atk, spd), (4, 3));

        let (atk, spd) = apply_act_baseline(3, 2, 3, Week::Three);
        assert_eq!((atk, spd), (5, 3));
    }

    #[test]
    fn test_full_scaling_example() {
        // Stage 75 in Act 4 (B+), Week 3 Final
        // tier = (75-61)/5 = 2 (stages 71-75)
        // Boss: The Frostbound Leviathan (B-B-W3-01)
        // Base: HP=74, ATK=4, ARM=14, SPD=2, DIG=3

        let boss = select_boss(75, Week::Three);
        assert_eq!(boss.name, "The Frostbound Leviathan");

        let scaled = scale_boss(boss, 75, Week::Three);

        // Week 3 tier 2 scaling: +8 HP, +2 ARM, +1 ATK
        // Act 4 baseline: +2 ATK, +1 SPD
        assert_eq!(scaled.hp, 74 + 8); // 82
        assert_eq!(scaled.atk, 4 + 1 + 2); // 7
        assert_eq!(scaled.arm, 14 + 2); // 16
        assert_eq!(scaled.spd, 2 + 1); // 3
        assert_eq!(scaled.dig, 3); // unchanged
    }

    #[test]
    fn test_broodmother_strikes() {
        // Broodmother has +2 strikes at battle start, should report 3 total
        let boss = select_boss(1, Week::One);
        assert_eq!(boss.name, "The Broodmother");
        let scaled = scale_boss(boss, 1, Week::One);
        assert_eq!(scaled.strikes, 3); // 1 base + 2 from trait
    }
}
