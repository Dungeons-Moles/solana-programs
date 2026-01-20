use crate::state::{EffectType, ItemEffect, StatusEffects, TriggerType};

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
        TriggerType::Countdown { turns } => turn >= turns,
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

pub fn apply_effect(
    effect_type: EffectType,
    value: i16,
    stats: &mut CombatantStats,
    status: &mut StatusEffects,
) {
    let value = value.max(0);

    match effect_type {
        EffectType::DealDamage => {
            let damage = (value - stats.arm).max(0);
            stats.hp = stats.hp.checked_sub(damage).unwrap_or(i16::MIN);
        }
        EffectType::DealNonWeaponDamage => {
            stats.hp = stats.hp.checked_sub(value).unwrap_or(i16::MIN);
        }
        EffectType::Heal => {
            let max_hp = i16::try_from(stats.max_hp).unwrap_or(i16::MAX);
            let healed = stats.hp.checked_add(value).unwrap_or(i16::MAX);
            stats.hp = healed.min(max_hp);
        }
        EffectType::GainArmor => {
            stats.arm = stats.arm.checked_add(value).unwrap_or(i16::MAX);
        }
        EffectType::GainAtk => {
            stats.atk = stats.atk.checked_add(value).unwrap_or(i16::MAX);
        }
        EffectType::GainSpd => {
            stats.spd = stats.spd.checked_add(value).unwrap_or(i16::MAX);
        }
        EffectType::ApplyChill => {
            status.chill = status.chill.saturating_add(value as u8);
        }
        EffectType::ApplyShrapnel => {
            status.shrapnel = status.shrapnel.saturating_add(value as u8);
        }
        EffectType::ApplyRust => {
            status.rust = status.rust.saturating_add(value as u8);
        }
        EffectType::ApplyBleed => {
            status.bleed = status.bleed.saturating_add(value as u8);
        }
        EffectType::RemoveArmor => {
            let reduced = stats.arm.checked_sub(value).unwrap_or(i16::MIN);
            stats.arm = reduced.max(0);
        }
        // Boss-specific effects (handled by boss-system or gameplay-state)
        EffectType::GainStrikes | EffectType::StealGold | EffectType::GoldToArmor => {
            // These effects are processed outside the combat system
        }
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
            apply_effect(
                effect.effect_type,
                effect.value,
                opponent_stats,
                opponent_status,
            );
        } else {
            apply_effect(effect.effect_type, effect.value, owner_stats, owner_status);
        }

        if effect.once_per_turn {
            if let Some(flag) = triggered_flags.get_mut(index) {
                *flag = true;
            }
        }
    }
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

        process_triggers_for_phase(
            &mut effects,
            TriggerType::TurnStart,
            1,
            &mut stats,
            &mut status,
            &mut opponent_stats,
            &mut opponent_status,
            &mut flags,
        );

        assert_eq!(stats.atk, 4);
    }
}
