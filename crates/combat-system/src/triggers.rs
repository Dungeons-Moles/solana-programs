use crate::state::{
    CombatLogEntry, EffectType, ItemEffect, StatusEffects, TriggerType, STATUS_BLEED, STATUS_CHILL,
    STATUS_REFLECTION, STATUS_RUST, STATUS_SHRAPNEL,
};

#[derive(Clone, Copy, Debug, Default)]
pub struct CombatantStats {
    pub hp: i16,
    pub max_hp: u16,
    pub atk: i16,
    pub arm: i16,
    pub spd: i16,
}

/// Check if a trigger should fire.
///
/// `acts_first` is needed for `FirstTurnIfFaster` and `FirstTurnIfSlower` triggers.
/// For enemies, `acts_first` means enemy SPD >= player SPD (tie goes to enemy).
/// For players, `acts_first` means player SPD > enemy SPD.
#[allow(clippy::manual_is_multiple_of)]
pub fn should_trigger(
    trigger_type: TriggerType,
    turn: u8,
    is_first_turn: bool,
    acts_first: bool,
) -> bool {
    match trigger_type {
        TriggerType::BattleStart => turn == 1,
        TriggerType::FirstTurn => is_first_turn,
        TriggerType::FirstTurnIfFaster => is_first_turn && acts_first,
        TriggerType::FirstTurnIfSlower => is_first_turn && !acts_first,
        TriggerType::TurnStart => true,
        TriggerType::EveryOtherTurn => turn % 2 == 0,
        TriggerType::OnHit => true,
        TriggerType::Exposed | TriggerType::Wounded => true,
        // Countdown fires every N turns (turn 2, 4, 6... for turns=2)
        TriggerType::Countdown { turns } => turns > 0 && turn > 0 && turn % turns == 0,
    }
}

pub fn check_exposed(arm: i16) -> bool {
    arm <= 0
}

pub fn check_wounded(hp: i16, max_hp: u16) -> bool {
    let hp_value = i32::from(hp);
    let max_value = i32::from(max_hp);
    hp_value * 2 < max_value
}

#[inline]
fn apply_status_effect(
    status_field: &mut u8,
    value: i16,
    turn: u8,
    is_target_player: bool,
    status_id: u8,
    log: &mut Vec<CombatLogEntry>,
) {
    *status_field = status_field.saturating_add(value as u8);
    if value > 0 {
        log.push(CombatLogEntry::apply_status(turn, is_target_player, status_id, value));
    }
}

/// Applies an effect and logs it.
/// `is_target_player` indicates whether the effect target is the player (for logging purposes).
/// `gold_change` tracks net gold changes during combat (positive = player gains).
pub fn apply_effect(
    effect_type: EffectType,
    value: i16,
    stats: &mut CombatantStats,
    status: &mut StatusEffects,
    turn: u8,
    is_target_player: bool,
    gold_change: &mut i16,
    log: &mut Vec<CombatLogEntry>,
) {
    let value = value.max(0);

    match effect_type {
        EffectType::DealDamage => {
            let damage = (value - stats.arm).max(0);
            stats.hp = stats.hp.checked_sub(damage).unwrap_or(i16::MIN);
            if damage > 0 {
                log.push(CombatLogEntry::attack(turn, !is_target_player, damage));
            }
        }
        EffectType::DealNonWeaponDamage => {
            stats.hp = stats.hp.checked_sub(value).unwrap_or(i16::MIN);
            if value > 0 {
                log.push(CombatLogEntry::non_weapon_damage(
                    turn,
                    is_target_player,
                    value,
                ));
            }
        }
        EffectType::Heal => {
            let max_hp = i16::try_from(stats.max_hp).unwrap_or(i16::MAX);
            let old_hp = stats.hp;
            let healed = stats.hp.checked_add(value).unwrap_or(i16::MAX);
            stats.hp = healed.min(max_hp);
            let actual_heal = stats.hp - old_hp;
            if actual_heal > 0 {
                log.push(CombatLogEntry::heal(turn, is_target_player, actual_heal));
            }
        }
        EffectType::GainArmor => {
            stats.arm = stats.arm.checked_add(value).unwrap_or(i16::MAX);
            if value > 0 {
                log.push(CombatLogEntry::armor_change(turn, is_target_player, value));
            }
        }
        EffectType::GainAtk => {
            stats.atk = stats.atk.checked_add(value).unwrap_or(i16::MAX);
            if value > 0 {
                log.push(CombatLogEntry::atk_change(turn, is_target_player, value));
            }
        }
        EffectType::GainSpd => {
            stats.spd = stats.spd.checked_add(value).unwrap_or(i16::MAX);
            if value > 0 {
                log.push(CombatLogEntry::spd_change(turn, is_target_player, value));
            }
        }
        EffectType::ApplyChill => {
            apply_status_effect(&mut status.chill, value, turn, is_target_player, STATUS_CHILL, log);
        }
        EffectType::ApplyShrapnel => {
            apply_status_effect(&mut status.shrapnel, value, turn, is_target_player, STATUS_SHRAPNEL, log);
        }
        EffectType::ApplyRust => {
            apply_status_effect(&mut status.rust, value, turn, is_target_player, STATUS_RUST, log);
        }
        EffectType::ApplyBleed => {
            apply_status_effect(&mut status.bleed, value, turn, is_target_player, STATUS_BLEED, log);
        }
        EffectType::RemoveArmor => {
            let old_arm = stats.arm;
            let reduced = stats.arm.checked_sub(value).unwrap_or(i16::MIN);
            stats.arm = reduced.max(0);
            let actual_reduction = old_arm - stats.arm;
            if actual_reduction > 0 {
                log.push(CombatLogEntry::armor_change(
                    turn,
                    is_target_player,
                    -actual_reduction,
                ));
            }
        }
        EffectType::StealGold => {
            if is_target_player {
                *gold_change = gold_change.saturating_sub(value);
                log.push(CombatLogEntry::gold_stolen(turn, false, -value));
            } else {
                *gold_change = gold_change.saturating_add(value);
                log.push(CombatLogEntry::gold_stolen(turn, true, value));
            }
        }
        EffectType::ApplyReflection => {
            apply_status_effect(&mut status.reflection, value, turn, is_target_player, STATUS_REFLECTION, log);
        }
        // These effects are processed outside the combat system
        EffectType::GainStrikes
        | EffectType::GoldToArmor
        | EffectType::GainDig
        | EffectType::GainGold
        | EffectType::ApplyBomb => {}
    }
}

#[allow(clippy::too_many_arguments)]
pub fn process_triggers_for_phase(
    effects: &mut [ItemEffect],
    phase: TriggerType,
    turn: u8,
    owner_stats: &mut CombatantStats,
    owner_status: &mut StatusEffects,
    opponent_stats: &mut CombatantStats,
    opponent_status: &mut StatusEffects,
    triggered_flags: &mut [bool],
    is_owner_player: bool,
    owner_acts_first: bool,
    gold_change: &mut i16,
    log: &mut Vec<CombatLogEntry>,
) {
    let is_first_turn = turn == 1;

    for (index, effect) in effects.iter_mut().enumerate() {
        if effect.trigger != phase {
            continue;
        }

        if effect.once_per_turn && triggered_flags.get(index).copied().unwrap_or(false) {
            continue;
        }

        let should_fire = match effect.trigger {
            TriggerType::Exposed => check_exposed(owner_stats.arm),
            TriggerType::Wounded => check_wounded(owner_stats.hp, owner_stats.max_hp),
            _ => should_trigger(effect.trigger, turn, is_first_turn, owner_acts_first),
        };

        if !should_fire {
            continue;
        }

        if targets_opponent(effect.effect_type) {
            // Check for reflection on status effects (excluding ApplyReflection itself)
            let is_reflectable_status = is_status_effect(effect.effect_type)
                && !matches!(effect.effect_type, EffectType::ApplyReflection);

            if is_reflectable_status && opponent_status.reflection > 0 {
                // Reflection: status is reflected back to the source (owner)
                opponent_status.reflection = opponent_status.reflection.saturating_sub(1);

                apply_effect(
                    effect.effect_type,
                    effect.value,
                    owner_stats,
                    owner_status,
                    turn,
                    is_owner_player, // Reflected back to owner
                    gold_change,
                    log,
                );
            } else {
                // Normal: effect targets the opponent
                apply_effect(
                    effect.effect_type,
                    effect.value,
                    opponent_stats,
                    opponent_status,
                    turn,
                    !is_owner_player, // Target is opponent
                    gold_change,
                    log,
                );
            }
        } else {
            // Effect targets self (owner)
            apply_effect(
                effect.effect_type,
                effect.value,
                owner_stats,
                owner_status,
                turn,
                is_owner_player, // Target is owner
                gold_change,
                log,
            );
        }

        if effect.once_per_turn {
            if let Some(flag) = triggered_flags.get_mut(index) {
                *flag = true;
            }
        }
    }
}

/// Returns true if the effect applies a status that can be reflected
fn is_status_effect(effect_type: EffectType) -> bool {
    matches!(
        effect_type,
        EffectType::ApplyChill
            | EffectType::ApplyShrapnel
            | EffectType::ApplyRust
            | EffectType::ApplyBleed
            | EffectType::ApplyReflection
    )
}

pub fn reset_once_per_turn_flags(flags: &mut [bool]) {
    for flag in flags.iter_mut() {
        *flag = false;
    }
}

fn targets_opponent(effect_type: EffectType) -> bool {
    matches!(
        effect_type,
        EffectType::DealDamage
            | EffectType::DealNonWeaponDamage
            | EffectType::ApplyChill
            | EffectType::ApplyShrapnel
            | EffectType::ApplyRust
            | EffectType::ApplyBleed
            | EffectType::RemoveArmor
            | EffectType::ApplyBomb
            | EffectType::StealGold
            | EffectType::ApplyReflection
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battle_start_trigger() {
        // acts_first doesn't matter for BattleStart
        assert!(should_trigger(TriggerType::BattleStart, 1, true, false));
        assert!(!should_trigger(TriggerType::BattleStart, 2, false, false));
    }

    #[test]
    fn test_first_turn_trigger() {
        // acts_first doesn't matter for unconditional FirstTurn
        assert!(should_trigger(TriggerType::FirstTurn, 1, true, false));
        assert!(!should_trigger(TriggerType::FirstTurn, 2, false, false));
    }

    #[test]
    fn test_first_turn_if_faster_trigger() {
        // Only fires on turn 1 AND if this combatant acts first
        assert!(should_trigger(TriggerType::FirstTurnIfFaster, 1, true, true));
        assert!(!should_trigger(TriggerType::FirstTurnIfFaster, 1, true, false));
        assert!(!should_trigger(TriggerType::FirstTurnIfFaster, 2, false, true));
    }

    #[test]
    fn test_first_turn_if_slower_trigger() {
        // Only fires on turn 1 AND if this combatant acts second
        assert!(should_trigger(TriggerType::FirstTurnIfSlower, 1, true, false));
        assert!(!should_trigger(TriggerType::FirstTurnIfSlower, 1, true, true));
        assert!(!should_trigger(TriggerType::FirstTurnIfSlower, 2, false, false));
    }

    #[test]
    fn test_first_turn_if_faster_effect_blocked_when_slower() {
        // Simulate Frost Wisp: "If it acts first on Turn 1: apply 2 Chill"
        // If player is faster (SPD 3 vs enemy SPD 1), enemy's FirstTurnIfFaster should NOT fire
        let mut owner_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1, // Enemy is slower
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 3, // Player is faster
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = vec![ItemEffect {
            trigger: TriggerType::FirstTurnIfFaster,
            once_per_turn: false,
            effect_type: EffectType::ApplyChill,
            value: 2,
        }];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        // Enemy is slower (acts_first = false), so FirstTurnIfFaster should NOT fire
        process_triggers_for_phase(
            &mut effects,
            TriggerType::FirstTurnIfFaster,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            false, // is_owner_player (enemy is owner)
            false, // owner_acts_first: enemy is slower
            &mut gold_change,
            &mut log,
        );

        // Chill should NOT be applied because enemy is slower
        assert_eq!(
            opponent_status.chill, 0,
            "Chill should NOT be applied when enemy is slower"
        );
        assert!(log.is_empty(), "No log entries should be created");
    }

    #[test]
    fn test_first_turn_if_faster_effect_fires_when_faster() {
        // Same scenario but enemy is faster this time
        let mut owner_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 3, // Enemy is faster
        };
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1, // Player is slower
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = vec![ItemEffect {
            trigger: TriggerType::FirstTurnIfFaster,
            once_per_turn: false,
            effect_type: EffectType::ApplyChill,
            value: 2,
        }];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        // Enemy is faster (acts_first = true), so FirstTurnIfFaster should fire
        process_triggers_for_phase(
            &mut effects,
            TriggerType::FirstTurnIfFaster,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            false, // is_owner_player (enemy is owner)
            true,  // owner_acts_first: enemy is faster
            &mut gold_change,
            &mut log,
        );

        // Chill SHOULD be applied because enemy acts first
        assert_eq!(
            opponent_status.chill, 2,
            "Chill should be applied when enemy is faster"
        );
        assert_eq!(log.len(), 1, "One log entry for ApplyChill");
    }

    #[test]
    fn test_turn_start_trigger() {
        assert!(should_trigger(TriggerType::TurnStart, 1, true, false));
        assert!(should_trigger(TriggerType::TurnStart, 5, false, false));
    }

    #[test]
    fn test_every_other_turn_trigger() {
        assert!(!should_trigger(TriggerType::EveryOtherTurn, 1, false, false));
        assert!(should_trigger(TriggerType::EveryOtherTurn, 2, false, false));
        assert!(!should_trigger(TriggerType::EveryOtherTurn, 3, false, false));
        assert!(should_trigger(TriggerType::EveryOtherTurn, 4, false, false));
    }

    #[test]
    fn test_exposed_condition() {
        assert!(check_exposed(0));
        assert!(check_exposed(-1));
        assert!(!check_exposed(2));
    }

    #[test]
    fn test_wounded_condition() {
        assert!(check_wounded(4, 10));
        assert!(!check_wounded(5, 10));
        assert!(check_wounded(0, 10));
    }

    #[test]
    fn test_deterministic_effect_ordering() {
        let mut stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 1,
            arm: 0,
            spd: 1,
        };
        let mut status = StatusEffects::default();
        let mut opponent_stats = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 1,
            arm: 0,
            spd: 1,
        };
        let mut opponent_status = StatusEffects::default();

        let mut effects = vec![
            ItemEffect {
                trigger: TriggerType::TurnStart,
                once_per_turn: false,
                effect_type: EffectType::GainAtk,
                value: 2,
            },
            ItemEffect {
                trigger: TriggerType::TurnStart,
                once_per_turn: false,
                effect_type: EffectType::GainAtk,
                value: 1,
            },
        ];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::TurnStart,
            1,
            &mut stats,
            &mut status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for TurnStart
            &mut gold_change,
            &mut log,
        );

        assert_eq!(stats.atk, 4);
        // Should have logged 2 ATK changes
        assert_eq!(log.len(), 2);
    }

    // ========================================================================
    // Reflection Status Effect Tests
    // ========================================================================

    fn make_test_combatant() -> (CombatantStats, StatusEffects) {
        (
            CombatantStats {
                hp: 10,
                max_hp: 10,
                atk: 3,
                arm: 0,
                spd: 1,
            },
            StatusEffects::default(),
        )
    }

    #[test]
    fn test_reflection_blocks_and_redirects_chill() {
        let (mut owner_stats, mut owner_status) = make_test_combatant();
        let (mut opponent_stats, mut opponent_status) = make_test_combatant();
        opponent_status.reflection = 1; // Opponent has 1 Reflection stack

        // Owner tries to apply Chill to opponent
        let mut effects = vec![ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyChill,
            value: 2,
        }];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut gold_change,
            &mut log,
        );

        // Chill should be reflected back to owner
        assert_eq!(owner_status.chill, 2, "Chill should be reflected to owner");
        assert_eq!(opponent_status.chill, 0, "Opponent should not have Chill");
        assert_eq!(
            opponent_status.reflection, 0,
            "Reflection stack should be consumed"
        );
    }

    #[test]
    fn test_reflection_blocks_and_redirects_bleed() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 2; // Opponent has 2 Reflection stacks

        let mut effects = vec![ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyBleed,
            value: 3,
        }];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut gold_change,
            &mut log,
        );

        assert_eq!(owner_status.bleed, 3, "Bleed should be reflected to owner");
        assert_eq!(opponent_status.bleed, 0, "Opponent should not have Bleed");
        assert_eq!(
            opponent_status.reflection, 1,
            "One Reflection stack should remain"
        );
    }

    #[test]
    fn test_reflection_does_not_block_apply_reflection() {
        // ApplyReflection itself should NOT be reflected (prevents infinite loops)
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 1;

        let mut effects = vec![ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyReflection,
            value: 2,
        }];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut gold_change,
            &mut log,
        );

        // ApplyReflection should go through to opponent, NOT be reflected
        assert_eq!(
            owner_status.reflection, 0,
            "Owner should not receive reflected Reflection"
        );
        assert_eq!(
            opponent_status.reflection, 3,
            "Opponent should have original 1 + applied 2 = 3 Reflection"
        );
    }

    #[test]
    fn test_reflection_zero_stacks_does_not_block() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 0; // No Reflection

        let mut effects = vec![ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::ApplyRust,
            value: 1,
        }];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut gold_change,
            &mut log,
        );

        // Without Reflection, Rust should apply to opponent normally
        assert_eq!(owner_status.rust, 0, "Owner should not have Rust");
        assert_eq!(opponent_status.rust, 1, "Opponent should have Rust applied");
    }

    #[test]
    fn test_reflection_does_not_affect_direct_damage() {
        let mut owner_stats = CombatantStats::default();
        owner_stats.hp = 20;
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        opponent_stats.hp = 20;
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 5; // Lots of Reflection

        // Direct damage effect (DealDamage targets opponent but is not a status effect)
        let mut effects = vec![ItemEffect {
            trigger: TriggerType::OnHit,
            once_per_turn: false,
            effect_type: EffectType::DealDamage,
            value: 5,
        }];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut gold_change,
            &mut log,
        );

        // Direct damage should NOT be reflected
        assert_eq!(owner_stats.hp, 20, "Owner HP should be unchanged");
        assert_eq!(opponent_stats.hp, 15, "Opponent should take 5 damage");
        assert_eq!(
            opponent_status.reflection, 5,
            "Reflection should not be consumed for non-status effects"
        );
    }

    #[test]
    fn test_reflection_multiple_status_effects_consumes_multiple_stacks() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 2; // 2 Reflection stacks

        // Two different status effects
        let mut effects = vec![
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyChill,
                value: 1,
            },
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyShrapnel,
                value: 2,
            },
        ];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut gold_change,
            &mut log,
        );

        // Both status effects should be reflected
        assert_eq!(owner_status.chill, 1, "Chill should be reflected to owner");
        assert_eq!(
            owner_status.shrapnel, 2,
            "Shrapnel should be reflected to owner"
        );
        assert_eq!(opponent_status.chill, 0, "Opponent should not have Chill");
        assert_eq!(
            opponent_status.shrapnel, 0,
            "Opponent should not have Shrapnel"
        );
        assert_eq!(
            opponent_status.reflection, 0,
            "Both Reflection stacks should be consumed"
        );
    }

    #[test]
    fn test_reflection_partial_block_when_stacks_run_out() {
        let mut owner_stats = CombatantStats::default();
        let mut owner_status = StatusEffects::default();
        let mut opponent_stats = CombatantStats::default();
        let mut opponent_status = StatusEffects::default();
        opponent_status.reflection = 1; // Only 1 Reflection stack

        // Three status effects, but only 1 Reflection
        let mut effects = vec![
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyChill,
                value: 1,
            },
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyBleed,
                value: 2,
            },
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyRust,
                value: 3,
            },
        ];
        let mut flags = vec![false; effects.len()];
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        process_triggers_for_phase(
            &mut effects,
            TriggerType::OnHit,
            1,
            &mut owner_stats,
            &mut owner_status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
            true,  // is_owner_player
            false, // owner_acts_first: unused for OnHit
            &mut gold_change,
            &mut log,
        );

        // First effect (Chill) should be reflected, rest should hit opponent
        assert_eq!(
            owner_status.chill, 1,
            "First effect (Chill) should be reflected to owner"
        );
        assert_eq!(owner_status.bleed, 0, "Bleed should NOT be on owner");
        assert_eq!(owner_status.rust, 0, "Rust should NOT be on owner");

        assert_eq!(opponent_status.chill, 0, "Chill should NOT be on opponent");
        assert_eq!(
            opponent_status.bleed, 2,
            "Bleed should hit opponent (no Reflection left)"
        );
        assert_eq!(
            opponent_status.rust, 3,
            "Rust should hit opponent (no Reflection left)"
        );
        assert_eq!(opponent_status.reflection, 0, "Reflection should be exhausted");
    }
}
