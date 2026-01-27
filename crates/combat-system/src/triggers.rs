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

#[allow(clippy::manual_is_multiple_of)]
pub fn should_trigger(trigger_type: TriggerType, turn: u8, is_first_turn: bool) -> bool {
    match trigger_type {
        TriggerType::BattleStart => turn == 1,
        TriggerType::FirstTurn => is_first_turn,
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
            status.chill = status.chill.saturating_add(value as u8);
            if value > 0 {
                log.push(CombatLogEntry::apply_status(
                    turn,
                    is_target_player,
                    STATUS_CHILL,
                    value,
                ));
            }
        }
        EffectType::ApplyShrapnel => {
            status.shrapnel = status.shrapnel.saturating_add(value as u8);
            if value > 0 {
                log.push(CombatLogEntry::apply_status(
                    turn,
                    is_target_player,
                    STATUS_SHRAPNEL,
                    value,
                ));
            }
        }
        EffectType::ApplyRust => {
            status.rust = status.rust.saturating_add(value as u8);
            if value > 0 {
                log.push(CombatLogEntry::apply_status(
                    turn,
                    is_target_player,
                    STATUS_RUST,
                    value,
                ));
            }
        }
        EffectType::ApplyBleed => {
            status.bleed = status.bleed.saturating_add(value as u8);
            if value > 0 {
                log.push(CombatLogEntry::apply_status(
                    turn,
                    is_target_player,
                    STATUS_BLEED,
                    value,
                ));
            }
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
            // Enemy steals gold from player (target is player) or player steals from enemy
            // Positive value = amount to steal
            // If target is player, player loses gold (gold_change decreases)
            // If target is enemy (player is stealing), player gains gold (gold_change increases)
            if is_target_player {
                // Enemy is stealing from player
                *gold_change = gold_change.saturating_sub(value);
                log.push(CombatLogEntry::gold_stolen(turn, false, -value));
            } else {
                // Player is stealing from enemy
                *gold_change = gold_change.saturating_add(value);
                log.push(CombatLogEntry::gold_stolen(turn, true, value));
            }
        }
        EffectType::ApplyReflection => {
            status.reflection = status.reflection.saturating_add(value as u8);
            if value > 0 {
                log.push(CombatLogEntry::apply_status(
                    turn,
                    is_target_player,
                    STATUS_REFLECTION,
                    value,
                ));
            }
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
            _ => should_trigger(effect.trigger, turn, is_first_turn),
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
        assert!(should_trigger(TriggerType::BattleStart, 1, true));
        assert!(!should_trigger(TriggerType::BattleStart, 2, false));
    }

    #[test]
    fn test_first_turn_trigger() {
        assert!(should_trigger(TriggerType::FirstTurn, 1, true));
        assert!(!should_trigger(TriggerType::FirstTurn, 2, false));
    }

    #[test]
    fn test_turn_start_trigger() {
        assert!(should_trigger(TriggerType::TurnStart, 1, true));
        assert!(should_trigger(TriggerType::TurnStart, 5, false));
    }

    #[test]
    fn test_every_other_turn_trigger() {
        assert!(!should_trigger(TriggerType::EveryOtherTurn, 1, false));
        assert!(should_trigger(TriggerType::EveryOtherTurn, 2, false));
        assert!(!should_trigger(TriggerType::EveryOtherTurn, 3, false));
        assert!(should_trigger(TriggerType::EveryOtherTurn, 4, false));
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
            true, // is_owner_player
            &mut gold_change,
            &mut log,
        );

        assert_eq!(stats.atk, 4);
        // Should have logged 2 ATK changes
        assert_eq!(log.len(), 2);
    }
}
