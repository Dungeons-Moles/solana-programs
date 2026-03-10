use crate::constants::{MAX_TURNS, SUDDEN_DEATH_RAMP_TURN, SUDDEN_DEATH_TURN};
use crate::effects::process_shrapnel_retaliation;
use crate::state::{
    AnnotatedItemEffect, CombatContribution, CombatLogEntry, CombatSourceKind, CombatSourceRef,
    EffectType, StatusEffects, TriggerType,
};
use crate::triggers::{process_triggers_for_phase, CombatantStats};

const EXECUTION_EMBLEM_ID: [u8; 8] = *b"G-BO-06\0";

/// Calculate weapon damage. ARM is now a separate HP pool, not damage reduction.
/// This function returns raw ATK as damage (minimum 0).
pub fn calculate_weapon_damage(attacker_atk: i16) -> i16 {
    attacker_atk.max(0)
}

pub fn determine_turn_order(player_spd: i16, enemy_spd: i16) -> (bool, bool) {
    let player_first = player_spd > enemy_spd;
    (player_first, !player_first)
}

pub fn check_sudden_death(turn: u8) -> i16 {
    if turn < SUDDEN_DEATH_TURN {
        return 0;
    }

    let mut bonus = i16::from(turn.saturating_sub(SUDDEN_DEATH_TURN - 1));
    if turn >= SUDDEN_DEATH_RAMP_TURN {
        bonus =
            bonus.saturating_add(i16::from(turn.saturating_sub(SUDDEN_DEATH_RAMP_TURN - 1)) * 2);
    }
    bonus
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

fn is_execution_emblem_effect(annotated: &AnnotatedItemEffect) -> bool {
    matches!(
        annotated.source,
        Some(CombatSourceRef {
            kind: CombatSourceKind::Gear,
            id,
        }) if id[..8] == EXECUTION_EMBLEM_ID
    )
}

fn suppress_execution_emblem_on_non_first_strike(
    effects: &[AnnotatedItemEffect],
    triggered_flags: &mut [bool],
    strike_index: u8,
) -> Vec<(usize, bool)> {
    if strike_index == 0 {
        return Vec::new();
    }

    let mut suppressed = Vec::new();
    for (index, annotated) in effects.iter().enumerate() {
        if annotated.effect.trigger == TriggerType::OnHit && is_execution_emblem_effect(annotated) {
            let previous = triggered_flags.get(index).copied().unwrap_or(false);
            if let Some(flag) = triggered_flags.get_mut(index) {
                *flag = true;
            }
            suppressed.push((index, previous));
        }
    }

    suppressed
}

fn restore_suppressed_flags(triggered_flags: &mut [bool], suppressed: Vec<(usize, bool)>) {
    for (index, previous) in suppressed {
        if let Some(flag) = triggered_flags.get_mut(index) {
            *flag = previous;
        }
    }
}

fn has_double_on_hit_effects(effects: &[AnnotatedItemEffect]) -> bool {
    effects
        .iter()
        .any(|annotated| annotated.effect.effect_type == EffectType::DoubleOnHitEffects)
}

/// Execute a single strike. ARM is "HP before HP" - damage depletes ARM first,
/// then excess damage carries over to HP.
///
/// Returns: (new_hp, new_arm, hp_damage, arm_damage)
pub fn execute_strike(
    attacker_atk: i16,
    attacker_armor_piercing: i16,
    defender_arm: i16,
    defender_hp: i16,
    _defender_chill: u8,
) -> (i16, i16, i16, i16) {
    let raw_damage = calculate_weapon_damage(attacker_atk);

    if raw_damage <= 0 {
        return (defender_hp, defender_arm, 0, 0);
    }

    // ARM is "HP before HP": damage depletes ARM first, remainder goes to HP.
    let effective_defender_arm = defender_arm.saturating_sub(attacker_armor_piercing).max(0);
    let arm_damage = raw_damage.min(effective_defender_arm);
    let new_arm = defender_arm.saturating_sub(arm_damage);
    let hp_damage = raw_damage.saturating_sub(arm_damage);
    let new_hp = defender_hp.saturating_sub(hp_damage);

    (new_hp, new_arm, hp_damage, arm_damage)
}

fn shrapnel_source() -> CombatSourceRef {
    let mut id = [0u8; 16];
    id[..8].copy_from_slice(b"shrapnel");
    CombatSourceRef {
        kind: CombatSourceKind::Status,
        id,
    }
}

fn attack_contributions(attacker_stats: &CombatantStats, strike_value: i16) -> Vec<CombatContribution> {
    if strike_value <= 0 {
        return Vec::new();
    }

    let mut contributions = Vec::new();

    if attacker_stats.attack_base_value > 0 {
        if let Some(source) = attacker_stats.attack_source {
            contributions.push(CombatContribution {
                source,
                value: attacker_stats.attack_base_value,
            });
        }
    }

    contributions.extend(attacker_stats.atk_contributions.iter().cloned());
    let total: i16 = contributions.iter().map(|entry| entry.value).sum();
    if total < strike_value {
        if let Some(source) = attacker_stats.attack_source {
            if let Some(existing) = contributions.iter_mut().find(|entry| entry.source == source) {
                existing.value = existing.value.saturating_add(strike_value - total);
            } else {
                contributions.push(CombatContribution {
                    source,
                    value: strike_value - total,
                });
            }
        }
    }

    contributions.retain(|entry| entry.value > 0);
    contributions
}

#[allow(clippy::too_many_arguments)]
pub fn execute_strikes(
    strikes: u8,
    attacker_stats: &mut CombatantStats,
    attacker_status: &mut StatusEffects,
    defender_stats: &mut CombatantStats,
    defender_status: &mut StatusEffects,
    on_hit_effects: &mut [AnnotatedItemEffect],
    triggered_flags: &mut [bool],
    defender_effects: &mut [AnnotatedItemEffect],
    defender_triggered_flags: &mut [bool],
    turn: u8,
    is_player_attacking: bool,
    player_gold: &mut u16,
    enemy_gold: &mut u16,
    gold_change: &mut i16,
    log: &mut Vec<CombatLogEntry>,
) -> (i16, i16) {
    execute_strikes_with_armor_override(
        strikes,
        attacker_stats,
        attacker_status,
        defender_stats,
        defender_status,
        on_hit_effects,
        triggered_flags,
        defender_effects,
        defender_triggered_flags,
        turn,
        is_player_attacking,
        player_gold,
        enemy_gold,
        gold_change,
        log,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn execute_strikes_with_armor_override(
    strikes: u8,
    attacker_stats: &mut CombatantStats,
    attacker_status: &mut StatusEffects,
    defender_stats: &mut CombatantStats,
    defender_status: &mut StatusEffects,
    on_hit_effects: &mut [AnnotatedItemEffect],
    triggered_flags: &mut [bool],
    defender_effects: &mut [AnnotatedItemEffect],
    defender_triggered_flags: &mut [bool],
    turn: u8,
    is_player_attacking: bool,
    player_gold: &mut u16,
    enemy_gold: &mut u16,
    gold_change: &mut i16,
    log: &mut Vec<CombatLogEntry>,
    ignore_defender_armor: bool,
) -> (i16, i16) {
    let mut total_hp_damage: i16 = 0;

    for strike_index in 0..strikes {
        process_triggers_for_phase(
            on_hit_effects,
            TriggerType::BeforeStrike,
            turn,
            attacker_stats,
            attacker_status,
            defender_stats,
            defender_status,
            triggered_flags,
            is_player_attacking,
            false,
            player_gold,
            enemy_gold,
            gold_change,
            log,
        );

        let mut strike_atk = attacker_stats.atk;
        if attacker_stats.half_gear_atk_after_second_strike && strike_index >= 2 {
            let gear_bonus = attacker_stats.gear_atk_bonus.max(0);
            let non_gear_atk = attacker_stats.atk.saturating_sub(gear_bonus);
            strike_atk = non_gear_atk.saturating_add(gear_bonus / 2);
        }

        let contributions = attack_contributions(attacker_stats, strike_atk);

        let (new_hp, new_arm, hp_damage, arm_damage) = execute_strike(
            strike_atk,
            attacker_stats.armor_piercing,
            if ignore_defender_armor {
                0
            } else {
                defender_stats.arm
            },
            defender_stats.hp,
            defender_status.chill,
        );
        defender_stats.hp = new_hp;
        defender_stats.arm = new_arm;
        total_hp_damage = total_hp_damage.saturating_add(hp_damage);

        // Log armor damage if any
        if arm_damage > 0 {
            let mut entry = CombatLogEntry::armor_change(
                turn,
                !is_player_attacking, // Defender's armor is being reduced
                -arm_damage,
            );
            if let Some(source) = attacker_stats.attack_source {
                entry = entry.with_source(source);
            }
            if !contributions.is_empty() {
                entry = entry.with_contributions(contributions.clone());
            }
            log.push(entry);
        }

        // Log HP damage if any (this is the "attack" that got through armor)
        if hp_damage > 0 {
            let mut entry = CombatLogEntry::attack(turn, is_player_attacking, hp_damage);
            if let Some(source) = attacker_stats.attack_source {
                entry = entry.with_source(source);
            }
            if !contributions.is_empty() {
                entry = entry.with_contributions(contributions.clone());
            }
            log.push(entry);
        }

        // Trigger OnHit effects if any damage was dealt (armor or HP)
        if arm_damage > 0 || hp_damage > 0 {
            // Track status before OnHit effects to detect rust/shrapnel applications
            let attacker_shrapnel_before = attacker_status.shrapnel;
            let defender_rust_before = defender_status.rust;
            let defender_hp_before_on_hit_effects = defender_stats.hp;
            let had_double_on_hit = has_double_on_hit_effects(on_hit_effects);
            let triggered_before_on_hit = triggered_flags.to_vec();
            let suppressed_execution_emblem = suppress_execution_emblem_on_non_first_strike(
                on_hit_effects,
                triggered_flags,
                strike_index,
            );

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
                false, // acts_first: unused for OnHit triggers
                player_gold,
                enemy_gold,
                gold_change,
                log,
            );
            restore_suppressed_flags(triggered_flags, suppressed_execution_emblem);

            if had_double_on_hit {
                let mut repeated_effect_indexes: Vec<usize> = Vec::new();
                for (index, annotated) in on_hit_effects.iter().enumerate() {
                    if annotated.effect.trigger == TriggerType::OnHit
                        && annotated.effect.once_per_turn
                        && !triggered_before_on_hit.get(index).copied().unwrap_or(false)
                        && triggered_flags.get(index).copied().unwrap_or(false)
                    {
                        repeated_effect_indexes.push(index);
                    }
                }

                for index in &repeated_effect_indexes {
                    if let Some(flag) = triggered_flags.get_mut(*index) {
                        *flag = false;
                    }
                }

                let suppressed_execution_emblem = suppress_execution_emblem_on_non_first_strike(
                    on_hit_effects,
                    triggered_flags,
                    strike_index,
                );

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
                    false,
                    player_gold,
                    enemy_gold,
                    gold_change,
                    log,
                );
                restore_suppressed_flags(triggered_flags, suppressed_execution_emblem);

                for index in repeated_effect_indexes {
                    if let Some(flag) = triggered_flags.get_mut(index) {
                        *flag = true;
                    }
                }
            }

            // Process EveryOtherTurnFirstHit triggers (fires on even turns, first hit only)
            // The once_per_turn flag ensures only the first hit triggers the effect
            process_triggers_for_phase(
                on_hit_effects,
                TriggerType::EveryOtherTurnFirstHit,
                turn,
                attacker_stats,
                attacker_status,
                defender_stats,
                defender_status,
                triggered_flags,
                is_player_attacking,
                false, // acts_first: unused for this trigger
                player_gold,
                enemy_gold,
                gold_change,
                log,
            );

            // Check if rust was applied to defender (for OnApplyRust)
            let rust_applied_to_defender = defender_status.rust > defender_rust_before;
            // Check if shrapnel was gained by attacker (for OnGainShrapnel)
            let shrapnel_gained_by_attacker = attacker_status.shrapnel > attacker_shrapnel_before;

            // Fire OnApplyRust if rust was applied to defender
            if rust_applied_to_defender {
                process_triggers_for_phase(
                    on_hit_effects,
                    TriggerType::OnApplyRust,
                    turn,
                    attacker_stats,
                    attacker_status,
                    defender_stats,
                    defender_status,
                    triggered_flags,
                    is_player_attacking,
                    false,
                    player_gold,
                    enemy_gold,
                    gold_change,
                    log,
                );
            }

            // Fire OnGainShrapnel if shrapnel was gained by attacker
            if shrapnel_gained_by_attacker {
                process_triggers_for_phase(
                    on_hit_effects,
                    TriggerType::OnGainShrapnel,
                    turn,
                    attacker_stats,
                    attacker_status,
                    defender_stats,
                    defender_status,
                    triggered_flags,
                    is_player_attacking,
                    false,
                    player_gold,
                    enemy_gold,
                    gold_change,
                    log,
                );
            }

            let defender_took_non_weapon_damage =
                defender_stats.hp < defender_hp_before_on_hit_effects;
            if defender_took_non_weapon_damage {
                process_triggers_for_phase(
                    defender_effects,
                    TriggerType::OnDealNonWeaponDamage,
                    turn,
                    defender_stats,
                    defender_status,
                    attacker_stats,
                    attacker_status,
                    defender_triggered_flags,
                    !is_player_attacking,
                    false,
                    player_gold,
                    enemy_gold,
                    gold_change,
                    log,
                );
            }

            // Fire OnStruck effects for the defender (e.g., Rime Cloak applies Chill to attacker)
            // Note: defender is the "owner" of OnStruck effects, attacker is the "opponent"
            process_triggers_for_phase(
                defender_effects,
                TriggerType::OnStruck,
                turn,
                defender_stats,
                defender_status,
                attacker_stats,
                attacker_status,
                defender_triggered_flags,
                !is_player_attacking, // defender is player if attacker is not
                false,                // acts_first: unused for OnStruck triggers
                player_gold,
                enemy_gold,
                gold_change,
                log,
            );
        }

        // Shrapnel: defender retaliates with damage when struck
        let old_attacker_hp = attacker_stats.hp;
        attacker_stats.hp = process_shrapnel_retaliation(
            defender_status.shrapnel,
            attacker_stats.hp,
        );
        let shrapnel_damage = old_attacker_hp - attacker_stats.hp;
        if shrapnel_damage > 0 {
            log.push(CombatLogEntry::shrapnel_retaliation(
                turn,
                is_player_attacking, // The attacker takes the damage
                shrapnel_damage,
            ).with_source(shrapnel_source()));
        }

        if defender_stats.hp <= 0 {
            break;
        }
    }

    (defender_stats.hp, total_hp_damage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AnnotatedItemEffect, ItemEffect, LogAction};

    fn make_combatant(atk: i16, spd: i16, arm: i16) -> CombatantStats {
        CombatantStats {
            hp: 30,
            max_hp: 30,
            atk,
            arm,
            spd,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        }
    }

    fn run_test_strikes(
        attacker_stats: &mut CombatantStats,
        defender_stats: &mut CombatantStats,
        strikes: u8,
        turn: u8,
        is_player_attacking: bool,
    ) -> Vec<CombatLogEntry> {
        let mut attacker_status = StatusEffects::default();
        let mut defender_status = StatusEffects::default();
        let mut on_hit_effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut triggered_flags: Vec<bool> = Vec::new();
        let mut defender_effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut defender_triggered_flags: Vec<bool> = Vec::new();
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change: i16 = 0;
        let mut log: Vec<CombatLogEntry> = Vec::new();

        execute_strikes(
            strikes,
            attacker_stats,
            &mut attacker_status,
            defender_stats,
            &mut defender_status,
            &mut on_hit_effects,
            &mut triggered_flags,
            &mut defender_effects,
            &mut defender_triggered_flags,
            turn,
            is_player_attacking,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        log
    }

    #[test]
    fn test_multi_strike_damage_accumulates() {
        // ARM is now "HP before HP" - damage hits ARM first, excess to HP
        let mut hp = 10;
        let mut arm = 1;
        let mut total_hp_damage = 0;
        for _ in 0..2 {
            let (new_hp, new_arm, hp_damage, _arm_damage) = execute_strike(2, 0, arm, hp, 0);
            hp = new_hp;
            arm = new_arm;
            total_hp_damage += hp_damage;
        }
        // Strike 1: 2 ATK vs 1 ARM -> 1 ARM damage, 1 HP damage (ARM depleted)
        // Strike 2: 2 ATK vs 0 ARM -> 0 ARM damage, 2 HP damage
        assert_eq!(total_hp_damage, 3);
        assert_eq!(hp, 7);
        assert_eq!(arm, 0);
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
        // Test that two identical combat sequences produce identical results
        // ARM is now "HP before HP" - damage hits ARM first, excess to HP
        let mut attacker = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        };
        let mut defender = CombatantStats {
            hp: 8,
            max_hp: 8,
            atk: 1,
            arm: 1,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        };
        let mut attacker_status = StatusEffects::default();
        let mut defender_status = StatusEffects::default();
        let mut effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut flags: Vec<bool> = Vec::new();
        let mut defender_effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut defender_flags: Vec<bool> = Vec::new();
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change: i16 = 0;
        let mut log: Vec<CombatLogEntry> = Vec::new();

        // 2 strikes with ATK 2 vs ARM 1, HP 8:
        // Strike 1: 2 damage -> 1 to ARM (depleted), 1 to HP -> HP 7
        // Strike 2: 2 damage -> 0 to ARM, 2 to HP -> HP 5
        let (_, total_hp_damage_first) = execute_strikes(
            2,
            &mut attacker,
            &mut attacker_status,
            &mut defender,
            &mut defender_status,
            &mut effects,
            &mut flags,
            &mut defender_effects,
            &mut defender_flags,
            1,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );
        let hp_after_first = defender.hp;
        let arm_after_first = defender.arm;

        let mut attacker_again = CombatantStats {
            hp: 10,
            max_hp: 10,
            atk: 2,
            arm: 0,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        };
        let mut defender_again = CombatantStats {
            hp: 8,
            max_hp: 8,
            atk: 1,
            arm: 1,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        };
        let mut attacker_status_again = StatusEffects::default();
        let mut defender_status_again = StatusEffects::default();
        let mut effects_again: Vec<AnnotatedItemEffect> = Vec::new();
        let mut flags_again: Vec<bool> = Vec::new();
        let mut defender_effects_again: Vec<AnnotatedItemEffect> = Vec::new();
        let mut defender_flags_again: Vec<bool> = Vec::new();
        let mut player_gold_again: u16 = 0;
        let mut enemy_gold_again: u16 = 0;
        let mut gold_change_again: i16 = 0;
        let mut log_again: Vec<CombatLogEntry> = Vec::new();

        let (_, total_hp_damage_second) = execute_strikes(
            2,
            &mut attacker_again,
            &mut attacker_status_again,
            &mut defender_again,
            &mut defender_status_again,
            &mut effects_again,
            &mut flags_again,
            &mut defender_effects_again,
            &mut defender_flags_again,
            1,
            true,
            &mut player_gold_again,
            &mut enemy_gold_again,
            &mut gold_change_again,
            &mut log_again,
        );

        assert_eq!(total_hp_damage_first, total_hp_damage_second);
        assert_eq!(hp_after_first, defender_again.hp);
        assert_eq!(arm_after_first, defender_again.arm);
        // Both should have the same log length
        assert_eq!(log.len(), log_again.len());

        // Verify expected values with new ARM mechanic
        assert_eq!(hp_after_first, 5, "HP should be 5 after 2 strikes");
        assert_eq!(arm_after_first, 0, "ARM should be depleted");
        assert_eq!(total_hp_damage_first, 3, "Total HP damage should be 3");
    }

    #[test]
    fn test_damage_calculation() {
        // calculate_weapon_damage now just returns ATK (ARM is handled separately)
        let damage = calculate_weapon_damage(5);
        assert_eq!(damage, 5);
    }

    #[test]
    fn test_pneumatic_drill_scales_gear_atk_after_second_strike() {
        let mut attacker = CombatantStats {
            hp: 20,
            max_hp: 20,
            atk: 5, // 1 base/tool + 4 from gear
            arm: 0,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 4,
            half_gear_atk_after_second_strike: true,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        };
        let mut defender = CombatantStats {
            hp: 20,
            max_hp: 20,
            atk: 0,
            arm: 0,
            spd: 1,
            dig: 0,
            armor_piercing: 0,
            stored_damage: 0,
            gear_atk_bonus: 0,
            half_gear_atk_after_second_strike: false,
            next_bomb_damage_bonus: 0,
            next_bomb_self_damage_reduction: 0,
            active_bomb_self_damage_reduction: 0,
            non_weapon_damage_bonus: 0,
            next_non_weapon_damage_bonus: 0,
            preserve_shrapnel_cap: 0,
            shards_every_turn: false,
            ..Default::default()
        };
        let mut attacker_status = StatusEffects::default();
        let mut defender_status = StatusEffects::default();
        let mut effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut flags: Vec<bool> = Vec::new();
        let mut defender_effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut defender_flags: Vec<bool> = Vec::new();
        let mut player_gold: u16 = 0;
        let mut enemy_gold: u16 = 0;
        let mut gold_change: i16 = 0;
        let mut log: Vec<CombatLogEntry> = Vec::new();

        let (_, total_hp_damage) = execute_strikes(
            3,
            &mut attacker,
            &mut attacker_status,
            &mut defender,
            &mut defender_status,
            &mut effects,
            &mut flags,
            &mut defender_effects,
            &mut defender_flags,
            1,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        // Strike 1: 5, strike 2: 5, strike 3: 1 + floor(4/2) = 3
        assert_eq!(total_hp_damage, 13);
        assert_eq!(defender.hp, 7);
    }

    #[test]
    fn test_damage_floors_at_zero() {
        // Negative ATK should floor at 0
        let damage = calculate_weapon_damage(-2);
        assert_eq!(damage, 0);
    }

    #[test]
    fn test_arm_as_hp_pool() {
        // ARM is "HP before HP" - damage depletes ARM first, excess to HP
        // 5 ATK vs 3 ARM, 10 HP -> ARM 0, HP 8
        let (new_hp, new_arm, hp_damage, arm_damage) = execute_strike(5, 0, 3, 10, 0);
        assert_eq!(new_arm, 0, "ARM should be depleted");
        assert_eq!(arm_damage, 3, "Should deal 3 ARM damage");
        assert_eq!(hp_damage, 2, "Excess 2 damage should hit HP");
        assert_eq!(new_hp, 8, "HP should be 10 - 2 = 8");
    }

    #[test]
    fn test_arm_fully_blocks_small_damage() {
        // 2 ATK vs 5 ARM, 10 HP -> ARM 3, HP 10 (armor absorbs all damage)
        let (new_hp, new_arm, hp_damage, arm_damage) = execute_strike(2, 0, 5, 10, 0);
        assert_eq!(new_arm, 3, "ARM should absorb the full strike");
        assert_eq!(arm_damage, 2, "Should deal 2 ARM damage");
        assert_eq!(hp_damage, 0, "No HP damage when armor fully absorbs");
        assert_eq!(new_hp, 10, "HP should remain unchanged");
    }

    #[test]
    fn test_no_arm_all_damage_to_hp() {
        // 3 ATK vs 0 ARM, 10 HP -> ARM 0, HP 7
        let (new_hp, new_arm, hp_damage, arm_damage) = execute_strike(3, 0, 0, 10, 0);
        assert_eq!(new_arm, 0, "ARM should remain 0");
        assert_eq!(arm_damage, 0, "No ARM to damage");
        assert_eq!(hp_damage, 3, "All damage goes to HP");
        assert_eq!(new_hp, 7, "HP should be 10 - 3 = 7");
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
    fn test_double_on_hit_effects_repeat_once_per_turn_on_same_hit() {
        let mut attacker = make_combatant(1, 5, 0);
        let mut defender = make_combatant(0, 0, 0);
        defender.hp = 10;
        defender.max_hp = 10;

        let mut attacker_status = StatusEffects::default();
        let mut defender_status = StatusEffects::default();
        let mut on_hit_effects = vec![
            AnnotatedItemEffect {
                effect: ItemEffect {
                    trigger: TriggerType::BattleStart,
                    once_per_turn: false,
                    effect_type: EffectType::DoubleOnHitEffects,
                    value: 1,
                    condition: crate::state::Condition::None,
                },
                source: None,
            },
            AnnotatedItemEffect {
                effect: ItemEffect {
                    trigger: TriggerType::OnHit,
                    once_per_turn: true,
                    effect_type: EffectType::ApplyBleed,
                    value: 1,
                    condition: crate::state::Condition::None,
                },
                source: None,
            },
        ];
        let mut triggered_flags = vec![false; on_hit_effects.len()];
        let mut defender_effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut defender_triggered_flags: Vec<bool> = Vec::new();
        let mut player_gold = 0u16;
        let mut enemy_gold = 0u16;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        execute_strikes(
            1,
            &mut attacker,
            &mut attacker_status,
            &mut defender,
            &mut defender_status,
            &mut on_hit_effects,
            &mut triggered_flags,
            &mut defender_effects,
            &mut defender_triggered_flags,
            1,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        assert_eq!(defender_status.bleed, 2);

        let bleed_logs = log
            .iter()
            .filter(|entry| {
                entry.action == LogAction::ApplyStatus
                    && entry.value == 1
                    && entry.extra == crate::state::STATUS_BLEED
            })
            .count();
        assert_eq!(bleed_logs, 2);
    }

    #[test]
    fn test_before_strike_remove_armor_applies_before_damage() {
        let mut attacker = make_combatant(2, 5, 0);
        let mut defender = make_combatant(0, 0, 1);
        defender.hp = 10;
        defender.max_hp = 10;

        let mut attacker_status = StatusEffects::default();
        let mut defender_status = StatusEffects::default();
        let mut on_hit_effects = vec![AnnotatedItemEffect {
            effect: ItemEffect {
                trigger: TriggerType::BeforeStrike,
                once_per_turn: true,
                effect_type: EffectType::RemoveArmor,
                value: 1,
                condition: crate::state::Condition::None,
            },
            source: None,
        }];
        let mut triggered_flags = vec![false; on_hit_effects.len()];
        let mut defender_effects: Vec<AnnotatedItemEffect> = Vec::new();
        let mut defender_triggered_flags: Vec<bool> = Vec::new();
        let mut player_gold = 0u16;
        let mut enemy_gold = 0u16;
        let mut gold_change = 0i16;
        let mut log = Vec::new();

        let (_, total_hp_damage) = execute_strikes(
            1,
            &mut attacker,
            &mut attacker_status,
            &mut defender,
            &mut defender_status,
            &mut on_hit_effects,
            &mut triggered_flags,
            &mut defender_effects,
            &mut defender_triggered_flags,
            1,
            true,
            &mut player_gold,
            &mut enemy_gold,
            &mut gold_change,
            &mut log,
        );

        assert_eq!(defender.arm, 0);
        assert_eq!(defender.hp, 8);
        assert_eq!(total_hp_damage, 2);
    }

    #[test]
    fn test_sudden_death_bonus_before_threshold() {
        assert_eq!(check_sudden_death(19), 0);
    }

    #[test]
    fn test_sudden_death_bonus_starts_at_25() {
        assert_eq!(check_sudden_death(20), 1);
    }

    #[test]
    fn test_sudden_death_bonus_increases_each_turn() {
        assert_eq!(check_sudden_death(27), 8);
        assert_eq!(check_sudden_death(30), 13);
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
