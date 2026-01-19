#![allow(unexpected_cfgs)]

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
use state::{CombatState, CombatantInput, ItemEffect, ResolutionType, StatusEffects, TriggerType};
use triggers::{process_triggers_for_phase, reset_once_per_turn_flags, CombatantStats};

declare_id!("ArfGqYVEMwX2GzLip1rWxCef4Ewf4imtZGjzEyEhSf6r");

const COMBAT_STATE_SEED: &[u8] = b"combat_state";

#[program]
pub mod combat_system {
    use super::*;

    /// Initializes a new combat state account for the provided combatants.
    ///
    /// The combat state seeds are derived from `game_state` and the combatant
    /// stats are validated before storage. Emits `CombatInitialized`.
    pub fn initialize_combat(
        ctx: Context<InitializeCombat>,
        player_stats: CombatantInput,
        enemy_stats: CombatantInput,
    ) -> Result<()> {
        validate_combatant(&player_stats)?;
        validate_combatant(&enemy_stats)?;

        let combat_state = &mut ctx.accounts.combat_state;
        combat_state.game_state = ctx.accounts.game_state.key();
        combat_state.player = ctx.accounts.player.key();
        combat_state.turn = 1;
        combat_state.player_hp = player_stats.hp;
        combat_state.player_max_hp = player_stats.max_hp;
        combat_state.player_atk = player_stats.atk;
        combat_state.player_arm = player_stats.arm;
        combat_state.player_spd = player_stats.spd;
        combat_state.player_strikes = player_stats.strikes;
        combat_state.player_status = StatusEffects::default();
        combat_state.enemy_hp = enemy_stats.hp;
        combat_state.enemy_max_hp = enemy_stats.max_hp;
        combat_state.enemy_atk = enemy_stats.atk;
        combat_state.enemy_arm = enemy_stats.arm;
        combat_state.enemy_spd = enemy_stats.spd;
        combat_state.enemy_strikes = enemy_stats.strikes;
        combat_state.enemy_status = StatusEffects::default();
        combat_state.sudden_death_bonus = 0;
        combat_state.combat_ended = false;
        combat_state.player_won = false;
        combat_state.bump = ctx.bumps.combat_state;

        emit!(CombatInitialized {
            game_state: combat_state.game_state,
            player: combat_state.player,
            player_hp: combat_state.player_hp,
            enemy_hp: combat_state.enemy_hp,
        });

        Ok(())
    }

    /// Resolves combat deterministically until a winner is decided.
    ///
    /// Effects are applied in a fixed order (battle start, first turn, turn start,
    /// every other turn, strikes, end-of-turn decay). Emits `TurnExecuted` each
    /// turn and `CombatResult` when combat ends.
    #[allow(clippy::manual_is_multiple_of)]
    pub fn resolve_combat(
        ctx: Context<ResolveCombat>,
        player_effects: Vec<ItemEffect>,
        enemy_effects: Vec<ItemEffect>,
    ) -> Result<()> {
        let mut player_effects = player_effects;
        let mut enemy_effects = enemy_effects;
        let combat_state = &mut ctx.accounts.combat_state;

        require!(
            !combat_state.combat_ended,
            CombatSystemError::CombatAlreadyEnded
        );
        // Determinism guard: no randomness or clock-based logic allowed.
        debug_assert!(cfg!(not(feature = "rand")));

        let mut turn = combat_state.turn;
        require!(
            turn > 0 && turn <= MAX_TURNS,
            CombatSystemError::MaxTurnsExceeded
        );

        let mut player_triggered = vec![false; player_effects.len()];
        let mut enemy_triggered = vec![false; enemy_effects.len()];

        // Deterministic ordering: BattleStart -> (FirstTurn) -> TurnStart -> (EveryOtherTurn) -> strikes -> status decay.

        apply_status_effects(
            &mut player_effects,
            &mut enemy_effects,
            combat_state,
            TriggerType::BattleStart,
            &mut player_triggered,
            &mut enemy_triggered,
        );

        loop {
            let is_first_turn = turn == 1;
            if is_first_turn {
                apply_status_effects(
                    &mut player_effects,
                    &mut enemy_effects,
                    combat_state,
                    TriggerType::FirstTurn,
                    &mut player_triggered,
                    &mut enemy_triggered,
                );
            }
            apply_status_effects(
                &mut player_effects,
                &mut enemy_effects,
                combat_state,
                TriggerType::TurnStart,
                &mut player_triggered,
                &mut enemy_triggered,
            );
            if turn % 2 == 0 {
                apply_status_effects(
                    &mut player_effects,
                    &mut enemy_effects,
                    combat_state,
                    TriggerType::EveryOtherTurn,
                    &mut player_triggered,
                    &mut enemy_triggered,
                );
            }

            let (player_damage_dealt, enemy_damage_dealt) = execute_turn(
                combat_state,
                turn,
                &mut player_effects,
                &mut enemy_effects,
                &mut player_triggered,
                &mut enemy_triggered,
            )?;

            apply_end_of_turn_effects(combat_state)?;

            emit!(TurnExecuted {
                turn,
                player_hp: combat_state.player_hp,
                enemy_hp: combat_state.enemy_hp,
                player_damage_dealt,
                enemy_damage_dealt,
            });

            if let Some((player_won, resolution_type)) = resolve_if_ended(combat_state, turn) {
                combat_state.combat_ended = true;
                combat_state.player_won = player_won;

                emit!(CombatResult {
                    game_state: combat_state.game_state,
                    player: combat_state.player,
                    player_won,
                    final_turn: turn,
                    player_remaining_hp: combat_state.player_hp,
                    enemy_remaining_hp: combat_state.enemy_hp,
                    resolution_type,
                });

                break;
            }

            if let Some(player_won) = engine::check_failsafe(
                turn,
                combat_state.player_hp,
                combat_state.player_max_hp,
                combat_state.enemy_hp,
                combat_state.enemy_max_hp,
            ) {
                combat_state.combat_ended = true;
                combat_state.player_won = player_won;

                let resolution_type = if player_won {
                    ResolutionType::FailsafePlayerWin
                } else {
                    ResolutionType::FailsafeEnemyWin
                };

                emit!(CombatResult {
                    game_state: combat_state.game_state,
                    player: combat_state.player,
                    player_won,
                    final_turn: turn,
                    player_remaining_hp: combat_state.player_hp,
                    enemy_remaining_hp: combat_state.enemy_hp,
                    resolution_type,
                });

                break;
            }

            turn = turn
                .checked_add(1)
                .ok_or(CombatSystemError::ArithmeticOverflow)?;
            require!(turn <= MAX_TURNS, CombatSystemError::MaxTurnsExceeded);
            combat_state.turn = turn;
            reset_once_per_turn_flags(&mut player_triggered);
            reset_once_per_turn_flags(&mut enemy_triggered);
        }

        Ok(())
    }

    /// Closes a combat state account once combat has ended.
    pub fn close_combat(ctx: Context<CloseCombat>) -> Result<()> {
        let combat_state = &ctx.accounts.combat_state;
        require!(combat_state.combat_ended, CombatSystemError::CombatNotEnded);
        Ok(())
    }
}

#[event]
pub struct CombatInitialized {
    pub game_state: Pubkey,
    pub player: Pubkey,
    pub player_hp: i16,
    pub enemy_hp: i16,
}

#[event]
pub struct TurnExecuted {
    pub turn: u8,
    pub player_hp: i16,
    pub enemy_hp: i16,
    pub player_damage_dealt: i16,
    pub enemy_damage_dealt: i16,
}

#[event]
pub struct StatusApplied {
    pub target: String,
    pub effect_type: String,
    pub stacks: u8,
}

#[event]
pub struct CombatResult {
    pub game_state: Pubkey,
    pub player: Pubkey,
    pub player_won: bool,
    pub final_turn: u8,
    pub player_remaining_hp: i16,
    pub enemy_remaining_hp: i16,
    pub resolution_type: ResolutionType,
}

#[derive(Accounts)]
pub struct InitializeCombat<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + CombatState::INIT_SPACE,
        seeds = [COMBAT_STATE_SEED, game_state.key().as_ref()],
        bump
    )]
    pub combat_state: Account<'info, CombatState>,

    /// CHECK: Game state account owned by gameplay-state program.
    pub game_state: AccountInfo<'info>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveCombat<'info> {
    #[account(
        mut,
        has_one = player @ CombatSystemError::Unauthorized,
    )]
    pub combat_state: Account<'info, CombatState>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseCombat<'info> {
    #[account(
        mut,
        has_one = player @ CombatSystemError::Unauthorized,
        close = player,
    )]
    pub combat_state: Account<'info, CombatState>,

    #[account(mut)]
    pub player: Signer<'info>,
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

    let mut player_stats = CombatantStats {
        hp: combat_state.player_hp,
        max_hp: combat_state.player_max_hp,
        atk: combat_state.player_atk,
        arm: combat_state.player_arm,
        spd: combat_state.player_spd,
    };
    let mut enemy_stats = CombatantStats {
        hp: combat_state.enemy_hp,
        max_hp: combat_state.enemy_max_hp,
        atk: combat_state.enemy_atk,
        arm: combat_state.enemy_arm,
        spd: combat_state.enemy_spd,
    };
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

fn apply_end_of_turn_effects(combat_state: &mut CombatState) -> Result<()> {
    if combat_state.player_status.rust > 0 {
        combat_state.player_arm =
            process_rust_decay(combat_state.player_status.rust, combat_state.player_arm);
    }
    if combat_state.enemy_status.rust > 0 {
        combat_state.enemy_arm =
            process_rust_decay(combat_state.enemy_status.rust, combat_state.enemy_arm);
    }

    if combat_state.player_status.bleed > 0 {
        combat_state.player_hp =
            process_bleed_damage(combat_state.player_status.bleed, combat_state.player_hp);
    }
    if combat_state.enemy_status.bleed > 0 {
        combat_state.enemy_hp =
            process_bleed_damage(combat_state.enemy_status.bleed, combat_state.enemy_hp);
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
) {
    let mut player_applied = StatusEffects::default();
    let mut enemy_applied = StatusEffects::default();

    process_phase_effects(
        player_effects,
        combat_state,
        true,
        trigger,
        &mut player_applied,
        &mut enemy_applied,
        player_triggered,
    );
    process_phase_effects(
        enemy_effects,
        combat_state,
        false,
        trigger,
        &mut player_applied,
        &mut enemy_applied,
        enemy_triggered,
    );

    emit_status_events(&player_applied, &enemy_applied);
}

fn process_phase_effects(
    effects: &mut [ItemEffect],
    combat_state: &mut CombatState,
    is_player: bool,
    trigger: TriggerType,
    player_applied: &mut StatusEffects,
    enemy_applied: &mut StatusEffects,
    triggered_flags: &mut [bool],
) {
    let (stats, status) = if is_player {
        (
            CombatantStats {
                hp: combat_state.player_hp,
                max_hp: combat_state.player_max_hp,
                atk: combat_state.player_atk,
                arm: combat_state.player_arm,
                spd: combat_state.player_spd,
            },
            combat_state.player_status,
        )
    } else {
        (
            CombatantStats {
                hp: combat_state.enemy_hp,
                max_hp: combat_state.enemy_max_hp,
                atk: combat_state.enemy_atk,
                arm: combat_state.enemy_arm,
                spd: combat_state.enemy_spd,
            },
            combat_state.enemy_status,
        )
    };

    let mut working_stats = stats;
    let mut working_status = status;

    let mut opponent_stats = if is_player {
        CombatantStats {
            hp: combat_state.enemy_hp,
            max_hp: combat_state.enemy_max_hp,
            atk: combat_state.enemy_atk,
            arm: combat_state.enemy_arm,
            spd: combat_state.enemy_spd,
        }
    } else {
        CombatantStats {
            hp: combat_state.player_hp,
            max_hp: combat_state.player_max_hp,
            atk: combat_state.player_atk,
            arm: combat_state.player_arm,
            spd: combat_state.player_spd,
        }
    };
    let mut opponent_status = if is_player {
        combat_state.enemy_status
    } else {
        combat_state.player_status
    };

    process_triggers_for_phase(
        effects,
        trigger,
        combat_state.turn,
        &mut working_stats,
        &mut working_status,
        &mut opponent_stats,
        &mut opponent_status,
        triggered_flags,
    );

    if is_player {
        combat_state.enemy_hp = opponent_stats.hp;
        combat_state.enemy_atk = opponent_stats.atk;
        combat_state.enemy_arm = opponent_stats.arm;
        combat_state.enemy_spd = opponent_stats.spd;
        combat_state.enemy_status = opponent_status;
    } else {
        combat_state.player_hp = opponent_stats.hp;
        combat_state.player_atk = opponent_stats.atk;
        combat_state.player_arm = opponent_stats.arm;
        combat_state.player_spd = opponent_stats.spd;
        combat_state.player_status = opponent_status;
    }

    if is_player {
        combat_state.player_hp = working_stats.hp;
        combat_state.player_atk = working_stats.atk;
        combat_state.player_arm = working_stats.arm;
        combat_state.player_spd = working_stats.spd;
        combat_state.player_status = working_status;
    } else {
        combat_state.enemy_hp = working_stats.hp;
        combat_state.enemy_atk = working_stats.atk;
        combat_state.enemy_arm = working_stats.arm;
        combat_state.enemy_spd = working_stats.spd;
        combat_state.enemy_status = working_status;
    }

    update_status_applied(
        status,
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
}

fn emit_status_events(player_applied: &StatusEffects, enemy_applied: &StatusEffects) {
    if player_applied.chill > 0 {
        emit!(StatusApplied {
            target: "player".to_string(),
            effect_type: "chill".to_string(),
            stacks: player_applied.chill,
        });
    }
    if player_applied.shrapnel > 0 {
        emit!(StatusApplied {
            target: "player".to_string(),
            effect_type: "shrapnel".to_string(),
            stacks: player_applied.shrapnel,
        });
    }
    if player_applied.rust > 0 {
        emit!(StatusApplied {
            target: "player".to_string(),
            effect_type: "rust".to_string(),
            stacks: player_applied.rust,
        });
    }
    if player_applied.bleed > 0 {
        emit!(StatusApplied {
            target: "player".to_string(),
            effect_type: "bleed".to_string(),
            stacks: player_applied.bleed,
        });
    }

    if enemy_applied.chill > 0 {
        emit!(StatusApplied {
            target: "enemy".to_string(),
            effect_type: "chill".to_string(),
            stacks: enemy_applied.chill,
        });
    }
    if enemy_applied.shrapnel > 0 {
        emit!(StatusApplied {
            target: "enemy".to_string(),
            effect_type: "shrapnel".to_string(),
            stacks: enemy_applied.shrapnel,
        });
    }
    if enemy_applied.rust > 0 {
        emit!(StatusApplied {
            target: "enemy".to_string(),
            effect_type: "rust".to_string(),
            stacks: enemy_applied.rust,
        });
    }
    if enemy_applied.bleed > 0 {
        emit!(StatusApplied {
            target: "enemy".to_string(),
            effect_type: "bleed".to_string(),
            stacks: enemy_applied.bleed,
        });
    }
}
