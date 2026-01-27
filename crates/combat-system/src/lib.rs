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
use triggers::{process_triggers_for_phase, reset_once_per_turn_flags};

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
        player_strikes: player_stats.strikes.max(MIN_STRIKES),
        player_status: StatusEffects::default(),
        enemy_hp: enemy_stats.hp,
        enemy_max_hp: enemy_stats.max_hp,
        enemy_atk: enemy_stats.atk,
        enemy_arm: enemy_stats.arm,
        enemy_spd: enemy_stats.spd,
        enemy_strikes: enemy_stats.strikes.max(MIN_STRIKES),
        enemy_status: StatusEffects::default(),
        sudden_death_bonus: 0,
        gold_change: 0,
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

        apply_end_of_turn_effects(&mut combat_state, &mut log)?;

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
    combat_state.player_status = player_status;
    combat_state.enemy_status = enemy_status;

    Ok((player_damage_dealt, enemy_damage_dealt))
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

fn apply_end_of_turn_effects(
    combat_state: &mut CombatState,
    log: &mut Vec<CombatLogEntry>,
) -> Result<()> {
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

    let status_before = working_status;

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
        status_before,
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
