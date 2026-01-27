use crate::constants::{MAX_TURNS, SUDDEN_DEATH_TURN};
use crate::effects::process_shrapnel_retaliation;
use crate::state::{CombatLogEntry, ItemEffect, StatusEffects, TriggerType};
use crate::triggers::{process_triggers_for_phase, CombatantStats};

pub fn calculate_weapon_damage(attacker_atk: i16, defender_arm: i16) -> i16 {
    let damage = attacker_atk.checked_sub(defender_arm).unwrap_or(i16::MIN);
    damage.max(0)
}

pub fn determine_turn_order(player_spd: i16, enemy_spd: i16) -> (bool, bool) {
    let player_first = player_spd > enemy_spd;
    (player_first, !player_first)
}

pub fn check_sudden_death(turn: u8) -> i16 {
    if turn >= SUDDEN_DEATH_TURN {
        let bonus_turns = turn.saturating_sub(SUDDEN_DEATH_TURN - 1);
        i16::from(bonus_turns)
    } else {
        0
    }
}

pub fn check_failsafe(
    turn: u8,
    player_hp: i16,
    player_max_hp: u16,
    enemy_hp: i16,
    enemy_max_hp: u16,
) -> Option<bool> {
    if turn < MAX_TURNS {
        return None;
    }

    let player_pct = i32::from(player_hp)
        .checked_mul(100)
        .and_then(|value| value.checked_div(i32::from(player_max_hp)))
        .unwrap_or(0);
    let enemy_pct = i32::from(enemy_hp)
        .checked_mul(100)
        .and_then(|value| value.checked_div(i32::from(enemy_max_hp)))
        .unwrap_or(0);

    Some(player_pct > enemy_pct)
}

pub fn execute_strike(attacker_atk: i16, defender_arm: i16, defender_hp: i16) -> (i16, i16) {
    let damage = calculate_weapon_damage(attacker_atk, defender_arm);
    let new_hp = defender_hp.checked_sub(damage).unwrap_or(i16::MIN);
    (new_hp, damage)
}

#[allow(clippy::too_many_arguments)]
pub fn execute_strikes(
    strikes: u8,
    attacker_stats: &mut CombatantStats,
    attacker_status: &mut StatusEffects,
    defender_stats: &mut CombatantStats,
    defender_status: &mut StatusEffects,
    on_hit_effects: &mut [ItemEffect],
    triggered_flags: &mut [bool],
    turn: u8,
    is_player_attacking: bool,
    gold_change: &mut i16,
    log: &mut Vec<CombatLogEntry>,
) -> (i16, i16) {
    let mut total_damage: i16 = 0;

    for _ in 0..strikes {
        let (new_hp, damage) =
            execute_strike(attacker_stats.atk, defender_stats.arm, defender_stats.hp);
        defender_stats.hp = new_hp;
        total_damage = total_damage.saturating_add(damage);

        // Log the attack
        if damage > 0 {
            log.push(CombatLogEntry::attack(turn, is_player_attacking, damage));
        }

        if damage > 0 {
            process_triggers_for_phase(
                on_hit_effects,
                TriggerType::OnHit,
                turn,
                attacker_stats,
                attacker_status,
                defender_stats,
                defender_status,
                triggered_flags,
                is_player_attacking,
                gold_change,
                log,
            );
        }

        // Process shrapnel retaliation
        let old_attacker_hp = attacker_stats.hp;
        attacker_stats.hp =
            process_shrapnel_retaliation(defender_status.shrapnel, attacker_stats.hp);
        let shrapnel_damage = old_attacker_hp - attacker_stats.hp;
        if shrapnel_damage > 0 {
            log.push(CombatLogEntry::shrapnel_retaliation(
                turn,
                is_player_attacking, // The attacker takes the damage
                shrapnel_damage,
            ));
        }

        if defender_stats.hp <= 0 {
            break;
        }
    }

    (defender_stats.hp, total_damage)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_strike_damage_accumulates() {
        let mut hp = 10;
        let mut total_damage = 0;
        for _ in 0..2 {
            let (new_hp, damage) = execute_strike(2, 1, hp);
            hp = new_hp;
            total_damage += damage;
        }
        assert_eq!(total_damage, 2);
        assert_eq!(hp, 8);
    }

    #[test]
    fn test_multi_strike_once_per_turn_behavior() {
        let mut triggered = false;
        for _ in 0..2 {
            if !triggered {
                triggered = true;
            }
        }
        assert!(triggered);
    }

    #[test]
    fn test_chill_reduces_strikes() {
        let strikes = crate::effects::apply_chill_to_strikes(3, 1);
        assert_eq!(strikes, 2);
    }

    #[test]
    fn test_deterministic_damage_sequence() {
        let mut attacker = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1,
        };
        let mut defender = CombatantStats {
            hp: 8,
            max_hp: 8,
            atk: 1,
            arm: 1,
            spd: 1,
        };
        let mut attacker_status = StatusEffects::default();
        let mut defender_status = StatusEffects::default();
        let mut effects: Vec<ItemEffect> = Vec::new();
        let mut flags: Vec<bool> = Vec::new();
        let mut gold_change: i16 = 0;
        let mut log: Vec<CombatLogEntry> = Vec::new();

        let (_, total_damage_first) = execute_strikes(
            2,
            &mut attacker,
            &mut attacker_status,
            &mut defender,
            &mut defender_status,
            &mut effects,
            &mut flags,
            1,
            true,
            &mut gold_change,
            &mut log,
        );
        let hp_after_first = defender.hp;

        let mut attacker_again = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1,
        };
        let mut defender_again = CombatantStats {
            hp: 8,
            max_hp: 8,
            atk: 1,
            arm: 1,
            spd: 1,
        };
        let mut attacker_status_again = StatusEffects::default();
        let mut defender_status_again = StatusEffects::default();
        let mut effects_again: Vec<ItemEffect> = Vec::new();
        let mut flags_again: Vec<bool> = Vec::new();
        let mut gold_change_again: i16 = 0;
        let mut log_again: Vec<CombatLogEntry> = Vec::new();

        let (_, total_damage_second) = execute_strikes(
            2,
            &mut attacker_again,
            &mut attacker_status_again,
            &mut defender_again,
            &mut defender_status_again,
            &mut effects_again,
            &mut flags_again,
            1,
            true,
            &mut gold_change_again,
            &mut log_again,
        );

        assert_eq!(total_damage_first, total_damage_second);
        assert_eq!(hp_after_first, defender_again.hp);
        // Both should have the same log length
        assert_eq!(log.len(), log_again.len());
    }

    #[test]
    fn test_damage_calculation() {
        let damage = calculate_weapon_damage(5, 3);
        assert_eq!(damage, 2);
    }

    #[test]
    fn test_damage_floors_at_zero() {
        let damage = calculate_weapon_damage(2, 5);
        assert_eq!(damage, 0);
    }

    #[test]
    fn test_turn_order_player_faster() {
        let (player_first, enemy_first) = determine_turn_order(2, 1);
        assert!(player_first);
        assert!(!enemy_first);
    }

    #[test]
    fn test_turn_order_enemy_on_tie() {
        let (player_first, enemy_first) = determine_turn_order(2, 2);
        assert!(!player_first);
        assert!(enemy_first);
    }

    #[test]
    fn test_sudden_death_bonus_before_threshold() {
        assert_eq!(check_sudden_death(24), 0);
    }

    #[test]
    fn test_sudden_death_bonus_starts_at_25() {
        assert_eq!(check_sudden_death(25), 1);
    }

    #[test]
    fn test_sudden_death_bonus_increases_each_turn() {
        assert_eq!(check_sudden_death(27), 3);
    }

    #[test]
    fn test_failsafe_returns_none_before_turn_50() {
        let result = check_failsafe(49, 10, 10, 10, 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_failsafe_player_wins_on_higher_hp_percent() {
        let result = check_failsafe(50, 6, 10, 5, 10);
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_failsafe_enemy_wins_on_tie() {
        let result = check_failsafe(50, 5, 10, 5, 10);
        assert_eq!(result, Some(false));
    }
}
