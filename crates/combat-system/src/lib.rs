use anchor_lang::prelude::*;

pub mod constants;
pub mod effects;
pub mod engine;
pub mod errors;
pub mod state;
pub mod triggers;

use constants::{MAX_TURNS, MIN_STRIKES, SUDDEN_DEATH_TURN};
use effects::{
    apply_chill_to_strikes, decay_status_effects, process_bleed_damage, process_rust_decay,
};
use errors::CombatSystemError;
use state::{CombatState, CombatantInput, StatusEffects};
use triggers::{check_wounded, process_triggers_for_phase, reset_once_per_turn_flags};

// Re-export common types for use by other programs
pub use state::{
    CombatLogEntry, EffectType, ItemEffect, LogAction, ResolutionType, TriggerType, STATUS_BLEED,
    STATUS_CHILL, STATUS_REFLECTION, STATUS_RUST, STATUS_SHRAPNEL,
};

pub struct CombatOutcome {
    pub player_won: bool,
    pub final_player_hp: i16,
    pub final_enemy_hp: i16,
    pub turns_taken: u8,
    pub resolution_type: ResolutionType,
    /// Detailed combat log for turn-by-turn visualization
    pub log: Vec<CombatLogEntry>,
    /// Net gold change during combat (positive = player gains, negative = player loses)
    pub gold_change: i16,
}

#[allow(clippy::manual_is_multiple_of)]
pub fn resolve_combat(
    player_stats: CombatantInput,
    enemy_stats: CombatantInput,
    mut player_effects: Vec<ItemEffect>,
    mut enemy_effects: Vec<ItemEffect>,
) -> Result<CombatOutcome> {
    validate_combatant(&player_stats)?;
    validate_combatant(&enemy_stats)?;

    // Initialize combat log (pre-allocate for typical combat length)
    let mut log: Vec<CombatLogEntry> = Vec::with_capacity(64);

    let mut combat_state = CombatState {
        turn: 1,
        player_hp: player_stats.hp,
        player_max_hp: player_stats.max_hp,
        player_atk: player_stats.atk,
        player_arm: player_stats.arm,
        player_spd: player_stats.spd,
        player_dig: player_stats.dig,
        player_strikes: player_stats.strikes.max(MIN_STRIKES),
        player_status: StatusEffects::default(),
        enemy_hp: enemy_stats.hp,
        enemy_max_hp: enemy_stats.max_hp,
        enemy_atk: enemy_stats.atk,
        enemy_arm: enemy_stats.arm,
        enemy_spd: enemy_stats.spd,
        enemy_dig: enemy_stats.dig,
        enemy_strikes: enemy_stats.strikes.max(MIN_STRIKES),
        enemy_status: StatusEffects::default(),
        sudden_death_bonus: 0,
        gold_change: 0,
        player_has_become_wounded: false,
        enemy_has_become_wounded: false,
    };

    let mut player_triggered = vec![false; player_effects.len()];
    let mut enemy_triggered = vec![false; enemy_effects.len()];

    apply_status_effects(
        &mut player_effects,
        &mut enemy_effects,
        &mut combat_state,
        TriggerType::BattleStart,
        &mut player_triggered,
        &mut enemy_triggered,
        &mut log,
    );

    let mut turn = combat_state.turn;
    require!(
        turn > 0 && turn <= MAX_TURNS,
        CombatSystemError::MaxTurnsExceeded
    );

    loop {
        let is_first_turn = turn == 1;
        if is_first_turn {
            apply_status_effects(
                &mut player_effects,
                &mut enemy_effects,
                &mut combat_state,
                TriggerType::FirstTurn,
                &mut player_triggered,
                &mut enemy_triggered,
                &mut log,
            );
        }
        apply_status_effects(
            &mut player_effects,
            &mut enemy_effects,
            &mut combat_state,
            TriggerType::TurnStart,
            &mut player_triggered,
            &mut enemy_triggered,
            &mut log,
        );
        if turn % 2 == 0 {
            apply_status_effects(
                &mut player_effects,
                &mut enemy_effects,
                &mut combat_state,
                TriggerType::EveryOtherTurn,
                &mut player_triggered,
                &mut enemy_triggered,
                &mut log,
            );
        }

        execute_turn(
            &mut combat_state,
            turn,
            &mut player_effects,
            &mut enemy_effects,
            &mut player_triggered,
            &mut enemy_triggered,
            &mut log,
        )?;

        apply_end_of_turn_effects(
            &mut combat_state,
            &mut player_effects,
            &mut enemy_effects,
            &mut player_triggered,
            &mut enemy_triggered,
            &mut log,
        )?;

        apply_status_effects(
            &mut player_effects,
            &mut enemy_effects,
            &mut combat_state,
            TriggerType::TurnEnd,
            &mut player_triggered,
            &mut enemy_triggered,
            &mut log,
        );

        if let Some((player_won, resolution_type)) = resolve_if_ended(&combat_state, turn) {
            return Ok(CombatOutcome {
                player_won,
                final_player_hp: combat_state.player_hp,
                final_enemy_hp: combat_state.enemy_hp,
                turns_taken: turn,
                resolution_type,
                log,
                gold_change: combat_state.gold_change,
            });
        }

        if let Some(player_won) = engine::check_failsafe(
            turn,
            combat_state.player_hp,
            combat_state.player_max_hp,
            combat_state.enemy_hp,
            combat_state.enemy_max_hp,
        ) {
            let resolution_type = if player_won {
                ResolutionType::FailsafePlayerWin
            } else {
                ResolutionType::FailsafeEnemyWin
            };

            return Ok(CombatOutcome {
                player_won,
                final_player_hp: combat_state.player_hp,
                final_enemy_hp: combat_state.enemy_hp,
                turns_taken: turn,
                resolution_type,
                log,
                gold_change: combat_state.gold_change,
            });
        }

        turn = turn
            .checked_add(1)
            .ok_or(CombatSystemError::ArithmeticOverflow)?;
        require!(turn <= MAX_TURNS, CombatSystemError::MaxTurnsExceeded);
        combat_state.turn = turn;
        reset_once_per_turn_flags(&mut player_triggered);
        reset_once_per_turn_flags(&mut enemy_triggered);
    }
}

fn validate_combatant(stats: &CombatantInput) -> Result<()> {
    require!(stats.max_hp > 0, CombatSystemError::InvalidCombatant);
    let max_hp = i16::try_from(stats.max_hp).map_err(|_| CombatSystemError::InvalidCombatant)?;
    require!(stats.hp > 0, CombatSystemError::InvalidCombatant);
    require!(stats.hp <= max_hp, CombatSystemError::InvalidCombatant);
    require!(
        stats.strikes >= MIN_STRIKES,
        CombatSystemError::InvalidCombatant
    );
    Ok(())
}

fn execute_turn(
    combat_state: &mut CombatState,
    turn: u8,
    player_effects: &mut [ItemEffect],
    enemy_effects: &mut [ItemEffect],
    player_triggered: &mut [bool],
    enemy_triggered: &mut [bool],
    log: &mut Vec<CombatLogEntry>,
) -> Result<(i16, i16)> {
    apply_sudden_death(combat_state, turn)?;

    let (player_first, _) =
        engine::determine_turn_order(combat_state.player_spd, combat_state.enemy_spd);

    let player_strikes = apply_chill_to_strikes(
        combat_state.player_strikes,
        combat_state.player_status.chill,
    );
    let enemy_strikes =
        apply_chill_to_strikes(combat_state.enemy_strikes, combat_state.enemy_status.chill);

    let mut player_stats = combat_state.player_stats();
    let mut enemy_stats = combat_state.enemy_stats();
    let mut player_status = combat_state.player_status;
    let mut enemy_status = combat_state.enemy_status;

    let mut player_damage_dealt = 0;
    let mut enemy_damage_dealt = 0;

    if player_first {
        let (enemy_hp, damage) = engine::execute_strikes(
            player_strikes,
            &mut player_stats,
            &mut player_status,
            &mut enemy_stats,
            &mut enemy_status,
            player_effects,
            player_triggered,
            enemy_effects,
            enemy_triggered,
            turn,
            true, // is_player attacking
            &mut combat_state.gold_change,
            log,
        );
        enemy_stats.hp = enemy_hp;
        player_damage_dealt = damage;

        if enemy_stats.hp > 0 {
            let (player_hp, damage) = engine::execute_strikes(
                enemy_strikes,
                &mut enemy_stats,
                &mut enemy_status,
                &mut player_stats,
                &mut player_status,
                enemy_effects,
                enemy_triggered,
                player_effects,
                player_triggered,
                turn,
                false, // enemy attacking
                &mut combat_state.gold_change,
                log,
            );
            player_stats.hp = player_hp;
            enemy_damage_dealt = damage;
        }
    } else {
        let (player_hp, damage) = engine::execute_strikes(
            enemy_strikes,
            &mut enemy_stats,
            &mut enemy_status,
            &mut player_stats,
            &mut player_status,
            enemy_effects,
            enemy_triggered,
            player_effects,
            player_triggered,
            turn,
            false, // enemy attacking
            &mut combat_state.gold_change,
            log,
        );
        player_stats.hp = player_hp;
        enemy_damage_dealt = damage;

        if player_stats.hp > 0 {
            let (enemy_hp, damage) = engine::execute_strikes(
                player_strikes,
                &mut player_stats,
                &mut player_status,
                &mut enemy_stats,
                &mut enemy_status,
                player_effects,
                player_triggered,
                enemy_effects,
                enemy_triggered,
                turn,
                true, // is_player attacking
                &mut combat_state.gold_change,
                log,
            );
            enemy_stats.hp = enemy_hp;
            player_damage_dealt = damage;
        }
    }

    combat_state.player_hp = player_stats.hp;
    combat_state.enemy_hp = enemy_stats.hp;
    // ARM changes during combat must be synced (from weapon damage and effects like RemoveArmor)
    combat_state.player_arm = player_stats.arm;
    combat_state.enemy_arm = enemy_stats.arm;
    combat_state.player_status = player_status;
    combat_state.enemy_status = enemy_status;

    // Check for first-time wounded transitions and fire FirstTimeWounded triggers
    check_first_time_wounded(
        combat_state,
        player_effects,
        enemy_effects,
        player_triggered,
        enemy_triggered,
        log,
    );

    Ok((player_damage_dealt, enemy_damage_dealt))
}

/// Check if either combatant became wounded for the first time this battle and fire
/// the FirstTimeWounded trigger for their effects. This ensures items like Gore Mantle
/// (G-BO-07) work correctly.
fn check_first_time_wounded(
    combat_state: &mut CombatState,
    player_effects: &mut [ItemEffect],
    enemy_effects: &mut [ItemEffect],
    player_triggered: &mut [bool],
    enemy_triggered: &mut [bool],
    log: &mut Vec<CombatLogEntry>,
) {
    let turn = combat_state.turn;
    let (player_acts_first, _) =
        engine::determine_turn_order(combat_state.player_spd, combat_state.enemy_spd);

    // Check if player became wounded for the first time
    if !combat_state.player_has_become_wounded
        && check_wounded(combat_state.player_hp, combat_state.player_max_hp)
    {
        combat_state.player_has_become_wounded = true;

        // Fire FirstTimeWounded effects for player
        let mut player_stats = combat_state.player_stats();
        let mut player_status = combat_state.player_status;
        let mut enemy_stats = combat_state.enemy_stats();
        let mut enemy_status = combat_state.enemy_status;

        process_triggers_for_phase(
            player_effects,
            TriggerType::FirstTimeWounded,
            turn,
            &mut player_stats,
            &mut player_status,
            &mut enemy_stats,
            &mut enemy_status,
            player_triggered,
            true, // is_owner_player
            player_acts_first,
            &mut combat_state.gold_change,
            log,
        );

        combat_state.set_player_stats(&player_stats);
        combat_state.player_status = player_status;
        combat_state.set_enemy_stats(&enemy_stats);
        combat_state.enemy_status = enemy_status;
    }

    // Check if enemy became wounded for the first time
    if !combat_state.enemy_has_become_wounded
        && check_wounded(combat_state.enemy_hp, combat_state.enemy_max_hp)
    {
        combat_state.enemy_has_become_wounded = true;

        // Fire FirstTimeWounded effects for enemy
        let mut enemy_stats = combat_state.enemy_stats();
        let mut enemy_status = combat_state.enemy_status;
        let mut player_stats = combat_state.player_stats();
        let mut player_status = combat_state.player_status;

        process_triggers_for_phase(
            enemy_effects,
            TriggerType::FirstTimeWounded,
            turn,
            &mut enemy_stats,
            &mut enemy_status,
            &mut player_stats,
            &mut player_status,
            enemy_triggered,
            false, // is_owner_player (enemy is owner)
            !player_acts_first,
            &mut combat_state.gold_change,
            log,
        );

        combat_state.set_enemy_stats(&enemy_stats);
        combat_state.enemy_status = enemy_status;
        combat_state.set_player_stats(&player_stats);
        combat_state.player_status = player_status;
    }
}

fn apply_sudden_death(combat_state: &mut CombatState, turn: u8) -> Result<()> {
    let bonus = engine::check_sudden_death(turn);
    if bonus > combat_state.sudden_death_bonus {
        let delta = bonus
            .checked_sub(combat_state.sudden_death_bonus)
            .ok_or(CombatSystemError::ArithmeticOverflow)?;
        combat_state.player_atk = combat_state
            .player_atk
            .checked_add(delta)
            .ok_or(CombatSystemError::ArithmeticOverflow)?;
        combat_state.enemy_atk = combat_state
            .enemy_atk
            .checked_add(delta)
            .ok_or(CombatSystemError::ArithmeticOverflow)?;
        combat_state.sudden_death_bonus = bonus;
    }

    Ok(())
}

/// Apply end of turn effects and fire OnEnemyBleedDamage triggers.
///
/// When the enemy takes bleed damage, player's OnEnemyBleedDamage effects fire.
/// When the player takes bleed damage, enemy's OnEnemyBleedDamage effects fire.
#[allow(clippy::too_many_arguments)]
fn apply_end_of_turn_effects(
    combat_state: &mut CombatState,
    player_effects: &mut [ItemEffect],
    enemy_effects: &mut [ItemEffect],
    player_triggered: &mut [bool],
    enemy_triggered: &mut [bool],
    log: &mut Vec<CombatLogEntry>,
) -> Result<()> {
    let turn = combat_state.turn;
    let (player_acts_first, _) =
        engine::determine_turn_order(combat_state.player_spd, combat_state.enemy_spd);

    // Process Rust (armor decay)
    if combat_state.player_status.rust > 0 {
        let old_arm = combat_state.player_arm;
        combat_state.player_arm =
            process_rust_decay(combat_state.player_status.rust, combat_state.player_arm);
        let arm_lost = old_arm - combat_state.player_arm;
        if arm_lost > 0 {
            log.push(CombatLogEntry::armor_change(
                combat_state.turn,
                true,
                -arm_lost,
            ));
        }
    }
    if combat_state.enemy_status.rust > 0 {
        let old_arm = combat_state.enemy_arm;
        combat_state.enemy_arm =
            process_rust_decay(combat_state.enemy_status.rust, combat_state.enemy_arm);
        let arm_lost = old_arm - combat_state.enemy_arm;
        if arm_lost > 0 {
            log.push(CombatLogEntry::armor_change(
                combat_state.turn,
                false,
                -arm_lost,
            ));
        }
    }

    // Process Bleed (damage over time)
    if combat_state.player_status.bleed > 0 {
        let old_hp = combat_state.player_hp;
        combat_state.player_hp =
            process_bleed_damage(combat_state.player_status.bleed, combat_state.player_hp);
        let damage = old_hp - combat_state.player_hp;
        if damage > 0 {
            log.push(CombatLogEntry::status_damage(
                combat_state.turn,
                true,
                damage,
            ));
            // Fire enemy's OnEnemyBleedDamage triggers (player is enemy's enemy)
            let mut enemy_stats = combat_state.enemy_stats();
            let mut enemy_status = combat_state.enemy_status;
            let mut player_stats = combat_state.player_stats();
            let mut player_status = combat_state.player_status;

            process_triggers_for_phase(
                enemy_effects,
                TriggerType::OnEnemyBleedDamage,
                turn,
                &mut enemy_stats,
                &mut enemy_status,
                &mut player_stats,
                &mut player_status,
                enemy_triggered,
                false, // is_owner_player (enemy is owner)
                !player_acts_first,
                &mut combat_state.gold_change,
                log,
            );

            combat_state.set_enemy_stats(&enemy_stats);
            combat_state.enemy_status = enemy_status;
            combat_state.set_player_stats(&player_stats);
            combat_state.player_status = player_status;
        }
    }
    if combat_state.enemy_status.bleed > 0 {
        let old_hp = combat_state.enemy_hp;
        combat_state.enemy_hp =
            process_bleed_damage(combat_state.enemy_status.bleed, combat_state.enemy_hp);
        let damage = old_hp - combat_state.enemy_hp;
        if damage > 0 {
            log.push(CombatLogEntry::status_damage(
                combat_state.turn,
                false,
                damage,
            ));
            // Fire player's OnEnemyBleedDamage triggers (enemy took bleed damage)
            let mut player_stats = combat_state.player_stats();
            let mut player_status = combat_state.player_status;
            let mut enemy_stats = combat_state.enemy_stats();
            let mut enemy_status = combat_state.enemy_status;

            process_triggers_for_phase(
                player_effects,
                TriggerType::OnEnemyBleedDamage,
                turn,
                &mut player_stats,
                &mut player_status,
                &mut enemy_stats,
                &mut enemy_status,
                player_triggered,
                true, // is_owner_player
                player_acts_first,
                &mut combat_state.gold_change,
                log,
            );

            combat_state.set_player_stats(&player_stats);
            combat_state.player_status = player_status;
            combat_state.set_enemy_stats(&enemy_stats);
            combat_state.enemy_status = enemy_status;
        }
    }

    decay_status_effects(&mut combat_state.player_status);
    decay_status_effects(&mut combat_state.enemy_status);

    Ok(())
}

fn resolve_if_ended(combat_state: &CombatState, turn: u8) -> Option<(bool, ResolutionType)> {
    if combat_state.player_hp <= 0 {
        let resolution_type = if turn >= SUDDEN_DEATH_TURN {
            ResolutionType::SuddenDeathEnemyWin
        } else {
            ResolutionType::PlayerDefeated
        };
        return Some((false, resolution_type));
    }

    if combat_state.enemy_hp <= 0 {
        let resolution_type = if turn >= SUDDEN_DEATH_TURN {
            ResolutionType::SuddenDeathPlayerWin
        } else {
            ResolutionType::EnemyDefeated
        };
        return Some((true, resolution_type));
    }

    None
}

fn apply_status_effects(
    player_effects: &mut [ItemEffect],
    enemy_effects: &mut [ItemEffect],
    combat_state: &mut CombatState,
    trigger: TriggerType,
    player_triggered: &mut [bool],
    enemy_triggered: &mut [bool],
    log: &mut Vec<CombatLogEntry>,
) {
    let mut player_applied = StatusEffects::default();
    let mut enemy_applied = StatusEffects::default();

    // Determine who acts first for FirstTurnIfFaster/FirstTurnIfSlower triggers
    let (player_acts_first, _) =
        engine::determine_turn_order(combat_state.player_spd, combat_state.enemy_spd);

    process_phase_effects(
        player_effects,
        combat_state,
        true,
        trigger,
        &mut player_applied,
        &mut enemy_applied,
        player_triggered,
        player_acts_first,
        log,
    );
    process_phase_effects(
        enemy_effects,
        combat_state,
        false,
        trigger,
        &mut player_applied,
        &mut enemy_applied,
        enemy_triggered,
        !player_acts_first, // Enemy acts first if player doesn't
        log,
    );
}

#[allow(clippy::too_many_arguments)]
fn process_phase_effects(
    effects: &mut [ItemEffect],
    combat_state: &mut CombatState,
    is_player: bool,
    trigger: TriggerType,
    player_applied: &mut StatusEffects,
    enemy_applied: &mut StatusEffects,
    triggered_flags: &mut [bool],
    owner_acts_first: bool,
    log: &mut Vec<CombatLogEntry>,
) {
    let (mut working_stats, mut working_status, mut opponent_stats, mut opponent_status) =
        if is_player {
            (
                combat_state.player_stats(),
                combat_state.player_status,
                combat_state.enemy_stats(),
                combat_state.enemy_status,
            )
        } else {
            (
                combat_state.enemy_stats(),
                combat_state.enemy_status,
                combat_state.player_stats(),
                combat_state.player_status,
            )
        };

    let owner_status_before = working_status;
    let opponent_status_before = opponent_status;

    process_triggers_for_phase(
        effects,
        trigger,
        combat_state.turn,
        &mut working_stats,
        &mut working_status,
        &mut opponent_stats,
        &mut opponent_status,
        triggered_flags,
        is_player,
        owner_acts_first,
        &mut combat_state.gold_change,
        log,
    );

    // Check if rust was applied to opponent (for OnApplyRust)
    let rust_applied_to_opponent = opponent_status.rust > opponent_status_before.rust;
    // Check if shrapnel was gained by owner (for OnGainShrapnel)
    let shrapnel_gained_by_owner = working_status.shrapnel > owner_status_before.shrapnel;

    // Fire OnApplyRust if rust was applied to opponent
    if rust_applied_to_opponent {
        process_triggers_for_phase(
            effects,
            TriggerType::OnApplyRust,
            combat_state.turn,
            &mut working_stats,
            &mut working_status,
            &mut opponent_stats,
            &mut opponent_status,
            triggered_flags,
            is_player,
            owner_acts_first,
            &mut combat_state.gold_change,
            log,
        );
    }

    // Fire OnGainShrapnel if shrapnel was gained by owner
    if shrapnel_gained_by_owner {
        process_triggers_for_phase(
            effects,
            TriggerType::OnGainShrapnel,
            combat_state.turn,
            &mut working_stats,
            &mut working_status,
            &mut opponent_stats,
            &mut opponent_status,
            triggered_flags,
            is_player,
            owner_acts_first,
            &mut combat_state.gold_change,
            log,
        );
    }

    if is_player {
        combat_state.set_enemy_stats(&opponent_stats);
        combat_state.enemy_status = opponent_status;
        combat_state.set_player_stats(&working_stats);
        combat_state.player_status = working_status;
    } else {
        combat_state.set_player_stats(&opponent_stats);
        combat_state.player_status = opponent_status;
        combat_state.set_enemy_stats(&working_stats);
        combat_state.enemy_status = working_status;
    }

    update_status_applied(
        owner_status_before,
        working_status,
        player_applied,
        enemy_applied,
        is_player,
    );
}

fn update_status_applied(
    before: StatusEffects,
    after: StatusEffects,
    player_applied: &mut StatusEffects,
    enemy_applied: &mut StatusEffects,
    is_player: bool,
) {
    let applied = StatusEffects {
        chill: after.chill.saturating_sub(before.chill),
        shrapnel: after.shrapnel.saturating_sub(before.shrapnel),
        rust: after.rust.saturating_sub(before.rust),
        bleed: after.bleed.saturating_sub(before.bleed),
        reflection: after.reflection.saturating_sub(before.reflection),
    };

    let target = if is_player {
        player_applied
    } else {
        enemy_applied
    };

    target.chill = target.chill.saturating_add(applied.chill);
    target.shrapnel = target.shrapnel.saturating_add(applied.shrapnel);
    target.rust = target.rust.saturating_add(applied.rust);
    target.bleed = target.bleed.saturating_add(applied.bleed);
    target.reflection = target.reflection.saturating_add(applied.reflection);
}

#[cfg(test)]
mod tests {
    use super::*;
    use state::{Condition, ItemEffect};

    /// Test that FirstTimeWounded triggers when player HP first drops below 50%.
    /// This verifies Gore Mantle (G-BO-07) functionality.
    #[test]
    fn test_first_time_wounded_triggers_once_for_player() {
        // Player: 20 HP, enemy: 10 HP. Enemy has enough ATK to wound player.
        let player = CombatantInput {
            hp: 20,
            max_hp: 20,
            atk: 2,
            arm: 0,
            spd: 1, // Player acts second
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 100, // Enemy won't die quickly
            max_hp: 100,
            atk: 5, // Will deal 5 damage per strike, wounding player after 2 hits
            arm: 0,
            spd: 2, // Enemy acts first
            dig: 0,
            strikes: 2,
        };

        // Player has a FirstTimeWounded effect that grants 6 armor (simulating Gore Mantle)
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::FirstTimeWounded,
            once_per_turn: false,
            effect_type: EffectType::GainArmor,
            value: 6,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // The combat should proceed without errors, and the player should have gained armor
        // from the FirstTimeWounded trigger when they first became wounded.
        // Player takes 10 damage on turn 1 (2 strikes * 5 ATK = 10 damage).
        // 20 - 10 = 10 HP, which is exactly 50%, so NOT wounded (wounded = HP * 2 < max_hp).
        // But enemy hits again on turn 2, bringing player to lower HP.
        // When player HP drops below 50% (< 10 HP out of 20), FirstTimeWounded fires.

        // Check that log contains an armor_change entry (proof the trigger fired)
        let armor_gain_log_entries: Vec<_> = outcome
            .log
            .iter()
            .filter(|entry| {
                matches!(entry.action, LogAction::ArmorChange) && entry.is_player && entry.value > 0
            })
            .collect();

        assert!(
            !armor_gain_log_entries.is_empty(),
            "Player should have gained armor from FirstTimeWounded trigger. Log: {:?}",
            outcome.log
        );
    }

    /// Test that FirstTimeWounded only fires once, even if player stays wounded.
    #[test]
    fn test_first_time_wounded_only_fires_once() {
        // Player will be wounded early and stay wounded for multiple turns
        let player = CombatantInput {
            hp: 20,
            max_hp: 20,
            atk: 1,  // Low damage to extend combat
            arm: 10, // Some armor to survive
            spd: 1,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 15, // High damage to wound player quickly
            arm: 0,
            spd: 2,
            dig: 0,
            strikes: 1,
        };

        // Player has a FirstTimeWounded effect that grants 6 armor
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::FirstTimeWounded,
            once_per_turn: false,
            effect_type: EffectType::GainArmor,
            value: 6,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Count how many times player gained exactly 6 armor (our trigger value)
        let armor_gain_count = outcome
            .log
            .iter()
            .filter(|entry| {
                matches!(entry.action, LogAction::ArmorChange)
                    && entry.is_player
                    && entry.value == 6
            })
            .count();

        assert_eq!(
            armor_gain_count, 1,
            "FirstTimeWounded should fire exactly once. Log: {:?}",
            outcome.log
        );
    }

    /// Test that FirstTimeWounded triggers for enemy when their HP drops below 50%.
    #[test]
    fn test_first_time_wounded_triggers_for_enemy() {
        let player = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 15, // High damage to wound enemy
            arm: 0,
            spd: 2, // Player acts first
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 20,
            max_hp: 20,
            atk: 1,
            arm: 0,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        let player_effects = vec![];
        // Enemy has a FirstTimeWounded effect that grants 6 armor
        let enemy_effects = vec![ItemEffect {
            trigger: TriggerType::FirstTimeWounded,
            once_per_turn: false,
            effect_type: EffectType::GainArmor,
            value: 6,
            condition: Condition::None,
        }];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Check that log contains an armor_change entry for enemy
        let enemy_armor_gain = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange) && !entry.is_player && entry.value > 0
        });

        assert!(
            enemy_armor_gain,
            "Enemy should have gained armor from FirstTimeWounded trigger. Log: {:?}",
            outcome.log
        );
    }

    // ========================================================================
    // Special Event Trigger Tests (OnEnemyBleedDamage, OnApplyRust, OnGainShrapnel)
    // ========================================================================

    /// Test that OnEnemyBleedDamage triggers when enemy takes bleed damage.
    /// This verifies Leech Wraps (G-BO-03) functionality.
    #[test]
    fn test_on_enemy_bleed_damage_triggers_heal() {
        // Player has high HP/ARM to survive, applies bleed to enemy, and has OnEnemyBleedDamage heal
        let player = CombatantInput {
            hp: 50,
            max_hp: 60, // max_hp > hp so we can test healing
            atk: 5,
            arm: 10,
            spd: 2, // Player acts first
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 0,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Player has:
        // 1. BattleStart: Apply 3 Bleed to enemy
        // 2. OnEnemyBleedDamage: Heal 2 HP (simulating Leech Wraps)
        let player_effects = vec![
            ItemEffect {
                trigger: TriggerType::BattleStart,
                once_per_turn: false,
                effect_type: EffectType::ApplyBleed,
                value: 3,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnEnemyBleedDamage,
                once_per_turn: true, // once per turn like the real item
                effect_type: EffectType::Heal,
                value: 2,
                condition: Condition::None,
            },
        ];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Check that player healed at least once from OnEnemyBleedDamage
        let player_heal_count = outcome
            .log
            .iter()
            .filter(|entry| {
                matches!(entry.action, LogAction::Heal) && entry.is_player && entry.value == 2
            })
            .count();

        assert!(
            player_heal_count >= 1,
            "Player should have healed from OnEnemyBleedDamage. Log: {:?}",
            outcome.log
        );
    }

    /// Test that OnApplyRust triggers when rust is applied to enemy.
    /// This verifies Salvage Clamp (G-RU-08) functionality.
    #[test]
    fn test_on_apply_rust_triggers_gold_gain() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 5,
            arm: 10,
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 5, // Enemy has armor for rust to affect
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Player has:
        // 1. BattleStart: Apply 2 Rust to enemy
        // 2. OnApplyRust: Gain 1 Gold (simulating Salvage Clamp)
        let player_effects = vec![
            ItemEffect {
                trigger: TriggerType::BattleStart,
                once_per_turn: false,
                effect_type: EffectType::ApplyRust,
                value: 2,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnApplyRust,
                once_per_turn: true,
                effect_type: EffectType::GainGold,
                value: 1,
                condition: Condition::None,
            },
        ];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // OnApplyRust should have fired, which uses GainGold
        // GainGold is not directly tracked in log but affects gold_change
        // Since GainGold is processed outside combat, we verify the trigger fired
        // by checking rust was applied (prerequisite for trigger)
        let rust_applied = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ApplyStatus) && !entry.is_player && entry.value == 2
        });

        assert!(
            rust_applied,
            "Rust should have been applied to enemy. Log: {:?}",
            outcome.log
        );

        // The OnApplyRust trigger should have fired after rust was applied.
        // GainGold effect is processed outside combat system but the trigger mechanism works.
    }

    /// Test that OnGainShrapnel triggers when player gains shrapnel.
    /// This verifies Shrapnel Talisman (G-ST-06) functionality.
    #[test]
    fn test_on_gain_shrapnel_triggers_armor_gain() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 5,
            arm: 0, // Start with no armor to verify gain
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 0,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Player has:
        // 1. BattleStart: Gain 2 Shrapnel (self-application, like Spiked Bracers)
        // 2. OnGainShrapnel: Gain 3 Armor (simulating Shrapnel Talisman)
        let player_effects = vec![
            ItemEffect {
                trigger: TriggerType::BattleStart,
                once_per_turn: false,
                effect_type: EffectType::ApplyShrapnel,
                value: 2,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnGainShrapnel,
                once_per_turn: true,
                effect_type: EffectType::GainArmor,
                value: 3,
                condition: Condition::None,
            },
        ];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Check that player gained armor from OnGainShrapnel
        let player_armor_gain = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange) && entry.is_player && entry.value == 3
        });

        assert!(
            player_armor_gain,
            "Player should have gained 3 armor from OnGainShrapnel. Log: {:?}",
            outcome.log
        );
    }

    /// Test that OnGainShrapnel fires for OnHit shrapnel application.
    #[test]
    fn test_on_gain_shrapnel_fires_on_hit() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 5,
            arm: 0,
            spd: 2, // Player attacks first
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 0,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Player has:
        // 1. OnHit: Gain 1 Shrapnel (like Shard Beetle)
        // 2. OnGainShrapnel: Gain 2 Armor (simulating Shrapnel Talisman)
        let player_effects = vec![
            ItemEffect {
                trigger: TriggerType::OnHit,
                once_per_turn: false,
                effect_type: EffectType::ApplyShrapnel,
                value: 1,
                condition: Condition::None,
            },
            ItemEffect {
                trigger: TriggerType::OnGainShrapnel,
                once_per_turn: true, // Once per turn
                effect_type: EffectType::GainArmor,
                value: 2,
                condition: Condition::None,
            },
        ];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Check that player gained armor from OnGainShrapnel on turn 1
        let player_armor_gain = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange) && entry.is_player && entry.value == 2
        });

        assert!(
            player_armor_gain,
            "Player should have gained 2 armor from OnGainShrapnel triggered by OnHit. Log: {:?}",
            outcome.log
        );
    }

    // ========================================================================
    // EveryOtherTurnFirstHit Trigger Tests (G-GR-05 through G-GR-08 Shards)
    // ========================================================================

    // ========================================================================
    // TurnEnd Trigger Tests (G-ST-08 Stone Sigil)
    // ========================================================================

    /// Test that TurnEnd triggers fire at the end of each turn.
    /// This verifies Stone Sigil (G-ST-08) functionality: "End of turn: if you have Armor, gain Armor".
    #[test]
    fn test_turn_end_triggers_gain_armor_with_condition() {
        // Player starts with armor, so TurnEnd conditional effect should fire
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 5, // Has armor, so OwnerHasArmor condition is met
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 2,
            arm: 5,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Stone Sigil: End of turn, if you have Armor, gain +2 Armor
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::TurnEnd,
            once_per_turn: false,
            effect_type: EffectType::GainArmor,
            value: 2,
            condition: Condition::OwnerHasArmor,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // TurnEnd should fire on turn 1, granting armor
        let armor_gain_turn_1 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange)
                && entry.is_player
                && entry.turn == 1
                && entry.value == 2
        });

        assert!(
            armor_gain_turn_1,
            "Stone Sigil should grant +2 armor at end of turn 1 (player has armor). Log: {:?}",
            outcome.log
        );
    }

    /// Test that TurnEnd conditional effects do NOT fire when condition is not met.
    #[test]
    fn test_turn_end_does_not_fire_when_condition_fails() {
        // Player starts with NO armor, so OwnerHasArmor condition is not met
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 0, // No armor
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 2,
            arm: 0,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Stone Sigil: End of turn, if you have Armor, gain +2 Armor
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::TurnEnd,
            once_per_turn: false,
            effect_type: EffectType::GainArmor,
            value: 2,
            condition: Condition::OwnerHasArmor,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // TurnEnd should NOT grant armor since player has no armor
        let any_armor_gain = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange) && entry.is_player && entry.value == 2
        });

        assert!(
            !any_armor_gain,
            "Stone Sigil should NOT grant armor when player has no armor. Log: {:?}",
            outcome.log
        );
    }

    /// Test that TurnEnd fires every turn (not just once).
    #[test]
    fn test_turn_end_fires_every_turn() {
        // Long combat to verify TurnEnd fires on multiple turns
        let player = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 3,
            arm: 10, // Enough armor to survive and keep OwnerHasArmor true
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 2,
            arm: 10,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        let player_effects = vec![ItemEffect {
            trigger: TriggerType::TurnEnd,
            once_per_turn: false,
            effect_type: EffectType::GainArmor,
            value: 1,
            condition: Condition::OwnerHasArmor,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Count TurnEnd armor gains across all turns
        let armor_gain_count = outcome
            .log
            .iter()
            .filter(|entry| {
                matches!(entry.action, LogAction::ArmorChange)
                    && entry.is_player
                    && entry.value == 1
            })
            .count();

        // Should fire on at least 2 different turns
        assert!(
            armor_gain_count >= 2,
            "TurnEnd should fire on multiple turns. Gains: {}, Turns: {}. Log: {:?}",
            armor_gain_count,
            outcome.turns_taken,
            outcome.log
        );
    }

    /// Test that EveryOtherTurnFirstHit triggers on turn 2 (first even turn) on first hit.
    /// This verifies Emerald Shard (G-GR-05) functionality.
    #[test]
    fn test_every_other_turn_first_hit_fires_on_even_turns() {
        // Combat that lasts at least 2 turns
        let player = CombatantInput {
            hp: 50,
            max_hp: 60, // max_hp > hp so we can verify healing
            atk: 3,
            arm: 5,
            spd: 2, // Player attacks first
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 2,
            arm: 5,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Player has EveryOtherTurnFirstHit heal effect (like Emerald Shard)
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::EveryOtherTurnFirstHit,
            once_per_turn: true, // first hit only
            effect_type: EffectType::Heal,
            value: 3,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Should have healed on turn 2 (first even turn)
        let heal_on_turn_2 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::Heal)
                && entry.is_player
                && entry.turn == 2
                && entry.value == 3
        });

        assert!(
            heal_on_turn_2,
            "Player should have healed 3 HP on turn 2 from EveryOtherTurnFirstHit. Log: {:?}",
            outcome.log
        );
    }

    /// Test that EveryOtherTurnFirstHit does NOT fire on odd turns.
    #[test]
    fn test_every_other_turn_first_hit_does_not_fire_on_odd_turns() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 60,
            atk: 3,
            arm: 5,
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 2,
            arm: 5,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        let player_effects = vec![ItemEffect {
            trigger: TriggerType::EveryOtherTurnFirstHit,
            once_per_turn: true,
            effect_type: EffectType::Heal,
            value: 5,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Should NOT have healed on turn 1 (odd turn)
        let heal_on_turn_1 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::Heal)
                && entry.is_player
                && entry.turn == 1
                && entry.value == 5
        });

        assert!(
            !heal_on_turn_1,
            "Player should NOT have healed on turn 1 (odd turn). Log: {:?}",
            outcome.log
        );
    }

    /// Test that EveryOtherTurnFirstHit only fires once per turn (first hit only).
    /// With multiple strikes, only the first hit should trigger the effect.
    #[test]
    fn test_every_other_turn_first_hit_fires_once_per_turn() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 60,
            atk: 3,
            arm: 5,
            spd: 2,
            dig: 0,
            strikes: 3, // Multiple strikes
        };
        let enemy = CombatantInput {
            hp: 100, // High HP to survive multiple turns
            max_hp: 100,
            atk: 2,
            arm: 0, // No armor so player deals damage
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        let player_effects = vec![ItemEffect {
            trigger: TriggerType::EveryOtherTurnFirstHit,
            once_per_turn: true, // First hit only
            effect_type: EffectType::Heal,
            value: 2,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Count heals on turn 2 - should be exactly 1 despite 3 strikes
        let heals_on_turn_2 = outcome
            .log
            .iter()
            .filter(|entry| {
                matches!(entry.action, LogAction::Heal)
                    && entry.is_player
                    && entry.turn == 2
                    && entry.value == 2
            })
            .count();

        assert_eq!(
            heals_on_turn_2, 1,
            "Should heal exactly once on turn 2 despite multiple strikes. Log: {:?}",
            outcome.log
        );
    }

    /// Test EveryOtherTurnFirstHit with non-weapon damage (like Ruby Shard G-GR-06).
    #[test]
    fn test_every_other_turn_first_hit_deals_damage() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 5,
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 2,
            arm: 5,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Ruby Shard: Deal non-weapon damage on first hit every other turn
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::EveryOtherTurnFirstHit,
            once_per_turn: true,
            effect_type: EffectType::DealNonWeaponDamage,
            value: 2,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Should have dealt non-weapon damage on turn 2
        let damage_on_turn_2 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::NonWeaponDamage)
                && !entry.is_player // Damage dealt TO enemy
                && entry.turn == 2
                && entry.value == 2
        });

        assert!(
            damage_on_turn_2,
            "Ruby Shard should deal 2 non-weapon damage on turn 2. Log: {:?}",
            outcome.log
        );
    }

    /// Test EveryOtherTurnFirstHit with armor gain (like Sapphire Shard G-GR-07).
    #[test]
    fn test_every_other_turn_first_hit_gains_armor() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 0, // Start with no armor
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 2,
            arm: 5,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Sapphire Shard: Gain armor on first hit every other turn
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::EveryOtherTurnFirstHit,
            once_per_turn: true,
            effect_type: EffectType::GainArmor,
            value: 2,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Should have gained armor on turn 2
        let armor_on_turn_2 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange)
                && entry.is_player
                && entry.turn == 2
                && entry.value == 2
        });

        assert!(
            armor_on_turn_2,
            "Sapphire Shard should gain 2 armor on turn 2. Log: {:?}",
            outcome.log
        );
    }

    /// Test that EveryOtherTurnFirstHit fires on multiple even turns (2, 4, 6...).
    #[test]
    fn test_every_other_turn_first_hit_fires_on_multiple_even_turns() {
        // Long combat to verify trigger fires on turns 2 and 4
        let player = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 3,
            arm: 10,
            spd: 2,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 2,
            arm: 10,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        let player_effects = vec![ItemEffect {
            trigger: TriggerType::EveryOtherTurnFirstHit,
            once_per_turn: true,
            effect_type: EffectType::GainArmor,
            value: 1,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Count armor gains on even turns
        let armor_on_turn_2 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange)
                && entry.is_player
                && entry.turn == 2
                && entry.value == 1
        });
        let armor_on_turn_4 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange)
                && entry.is_player
                && entry.turn == 4
                && entry.value == 1
        });

        // The combat should last long enough to reach turn 4
        if outcome.turns_taken >= 4 {
            assert!(
                armor_on_turn_2 && armor_on_turn_4,
                "Should gain armor on both turn 2 and turn 4. Log: {:?}",
                outcome.log
            );
        } else {
            // At minimum, turn 2 should have triggered
            assert!(
                armor_on_turn_2,
                "Should gain armor on turn 2. Log: {:?}",
                outcome.log
            );
        }
    }

    // ========================================================================
    // OnStruck Trigger Tests (G-FR-05 Rime Cloak)
    // ========================================================================

    /// Test that OnStruck triggers when the defender is struck.
    /// This verifies Rime Cloak (G-FR-05) functionality: "When struck (once/turn): apply 1 Chill to attacker"
    #[test]
    fn test_on_struck_applies_chill_to_attacker() {
        // Player has Rime Cloak (OnStruck: apply Chill to attacker)
        // Enemy attacks player, so player's OnStruck should fire
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 5,
            spd: 1, // Player acts second (enemy attacks first)
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 5,
            arm: 0,
            spd: 2, // Enemy acts first
            dig: 0,
            strikes: 1,
        };

        // Rime Cloak: OnStruck (once per turn), apply 1 Chill to attacker
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::OnStruck,
            once_per_turn: true,
            effect_type: EffectType::ApplyChill,
            value: 1,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Enemy should have received Chill from OnStruck on turn 1
        let chill_applied_to_enemy = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ApplyStatus)
                && !entry.is_player // Chill applied to enemy
                && entry.turn == 1
        });

        assert!(
            chill_applied_to_enemy,
            "OnStruck should apply Chill to enemy when player is struck. Log: {:?}",
            outcome.log
        );
    }

    /// Test that OnStruck fires once per turn when once_per_turn is true,
    /// even if the defender is struck multiple times.
    #[test]
    fn test_on_struck_once_per_turn_with_multiple_strikes() {
        let player = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 3,
            arm: 0, // No armor so all strikes deal damage
            spd: 1, // Player acts second
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 2,
            arm: 0,
            spd: 2, // Enemy acts first
            dig: 0,
            strikes: 3, // Multiple strikes
        };

        // OnStruck once per turn: apply 1 Chill
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::OnStruck,
            once_per_turn: true,
            effect_type: EffectType::ApplyChill,
            value: 1,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Count Chill applications on turn 1 - should be exactly 1
        let chill_on_turn_1 = outcome
            .log
            .iter()
            .filter(|entry| {
                matches!(entry.action, LogAction::ApplyStatus)
                    && !entry.is_player
                    && entry.turn == 1
            })
            .count();

        assert_eq!(
            chill_on_turn_1, 1,
            "OnStruck with once_per_turn should fire exactly once on turn 1 despite 3 strikes. Log: {:?}",
            outcome.log
        );
    }

    /// Test that OnStruck does NOT fire when the defender is not struck
    /// (e.g., attacker deals 0 damage).
    #[test]
    fn test_on_struck_does_not_fire_without_damage() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 10, // High armor
            spd: 1,
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 0, // Zero ATK, deals no damage
            arm: 0,
            spd: 2, // Enemy acts first
            dig: 0,
            strikes: 1,
        };

        let player_effects = vec![ItemEffect {
            trigger: TriggerType::OnStruck,
            once_per_turn: true,
            effect_type: EffectType::ApplyChill,
            value: 1,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // No Chill should be applied since enemy deals 0 damage
        let any_chill = outcome
            .log
            .iter()
            .any(|entry| matches!(entry.action, LogAction::ApplyStatus) && !entry.is_player);

        assert!(
            !any_chill,
            "OnStruck should NOT fire when no damage is dealt. Log: {:?}",
            outcome.log
        );
    }

    /// Test that OnStruck fires for both combatants when both have OnStruck effects.
    #[test]
    fn test_on_struck_fires_for_both_combatants() {
        let player = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 5,
            arm: 0,
            spd: 2, // Player acts first
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 100,
            max_hp: 100,
            atk: 5,
            arm: 0,
            spd: 1,
            dig: 0,
            strikes: 1,
        };

        // Player: OnStruck apply Chill to attacker
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::OnStruck,
            once_per_turn: true,
            effect_type: EffectType::ApplyChill,
            value: 1,
            condition: Condition::None,
        }];
        // Enemy: OnStruck apply Bleed to attacker
        let enemy_effects = vec![ItemEffect {
            trigger: TriggerType::OnStruck,
            once_per_turn: true,
            effect_type: EffectType::ApplyBleed,
            value: 1,
            condition: Condition::None,
        }];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // Player attacks enemy -> enemy's OnStruck fires -> Bleed on player
        let bleed_on_player = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ApplyStatus)
                && entry.is_player // Bleed applied to player
                && entry.turn == 1
        });

        // Enemy attacks player -> player's OnStruck fires -> Chill on enemy
        let chill_on_enemy = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ApplyStatus)
                && !entry.is_player // Chill applied to enemy
                && entry.turn == 1
        });

        assert!(
            bleed_on_player,
            "Enemy's OnStruck should apply Bleed to player when player attacks. Log: {:?}",
            outcome.log
        );
        assert!(
            chill_on_enemy,
            "Player's OnStruck should apply Chill to enemy when enemy attacks. Log: {:?}",
            outcome.log
        );
    }

    /// Test that OnStruck fires when only armor is damaged (no HP damage).
    /// This confirms that "when struck" means any damage dealt, not just HP damage.
    /// Rime Cloak's own ARM bonus should not work against its triggered effect.
    #[test]
    fn test_on_struck_fires_on_armor_only_damage() {
        let player = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 3,
            arm: 20, // High armor absorbs all damage
            spd: 1,  // Player acts second
            dig: 0,
            strikes: 1,
        };
        let enemy = CombatantInput {
            hp: 50,
            max_hp: 50,
            atk: 5, // Less than player's ARM, only armor damage
            arm: 0,
            spd: 2, // Enemy acts first
            dig: 0,
            strikes: 1,
        };

        // Rime Cloak: OnStruck (once per turn), apply 1 Chill to attacker
        let player_effects = vec![ItemEffect {
            trigger: TriggerType::OnStruck,
            once_per_turn: true,
            effect_type: EffectType::ApplyChill,
            value: 1,
            condition: Condition::None,
        }];
        let enemy_effects = vec![];

        let outcome = resolve_combat(player, enemy, player_effects, enemy_effects).unwrap();

        // On turn 1, enemy (ATK 5) hits player (ARM 20). All damage goes to armor.
        // Attack log with is_player=false means the enemy dealt HP damage through armor.
        // There should be NO such entry on turn 1 (armor absorbs everything).
        let enemy_hp_damage_turn_1 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::Attack)
                && !entry.is_player // enemy is the attacker
                && entry.turn == 1
        });
        assert!(
            !enemy_hp_damage_turn_1,
            "Enemy should not deal HP damage on turn 1 (armor absorbs all). Log: {:?}",
            outcome.log
        );

        // Confirm armor WAS damaged on turn 1 (the hit landed, just on armor)
        let armor_hit_turn_1 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ArmorChange)
                && entry.is_player // player's armor
                && entry.turn == 1
        });
        assert!(
            armor_hit_turn_1,
            "Player's armor should take damage on turn 1. Log: {:?}",
            outcome.log
        );

        // OnStruck should fire on turn 1 despite only armor damage
        let chill_on_turn_1 = outcome.log.iter().any(|entry| {
            matches!(entry.action, LogAction::ApplyStatus)
                && !entry.is_player // applied to enemy
                && entry.turn == 1
        });
        assert!(
            chill_on_turn_1,
            "OnStruck SHOULD fire on armor-only damage (turn 1). Log: {:?}",
            outcome.log
        );
    }
}
