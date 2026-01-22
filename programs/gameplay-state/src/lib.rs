use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod movement;
pub mod state;

use constants::*;
use errors::GameplayStateError;
use movement::{
    calculate_move_cost, chebyshev_distance, get_boss_for_combat, get_boss_id, is_adjacent,
    is_within_bounds, move_toward, resolve_combat_inline, InlineCombatStats,
};
use state::{GameState, Phase, StatType};

declare_id!("5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n");

#[program]
pub mod gameplay_state {
    use super::*;

    /// Initializes a new GameState account linked to an active GameSession.
    pub fn initialize_game_state(
        ctx: Context<InitializeGameState>,
        campaign_level: u8,
        map_width: u8,
        map_height: u8,
        start_x: u8,
        start_y: u8,
    ) -> Result<()> {
        // Validate starting position is within bounds
        require!(
            start_x < map_width && start_y < map_height,
            GameplayStateError::OutOfBounds
        );

        // Validate campaign level is valid (1-40)
        require!(
            campaign_level >= 1 && campaign_level <= 40,
            GameplayStateError::OutOfBounds
        );

        let game_state = &mut ctx.accounts.game_state;

        // Initialize core fields
        game_state.player = ctx.accounts.player.key();
        game_state.session = ctx.accounts.game_session.key();
        game_state.position_x = start_x;
        game_state.position_y = start_y;
        game_state.map_width = map_width;
        game_state.map_height = map_height;

        // Initialize default stats
        game_state.hp = DEFAULT_HP;
        game_state.max_hp = DEFAULT_MAX_HP;
        game_state.atk = DEFAULT_ATK;
        game_state.arm = DEFAULT_ARM;
        game_state.spd = DEFAULT_SPD;
        game_state.dig = DEFAULT_DIG;

        // Initialize game progression
        game_state.gear_slots = INITIAL_GEAR_SLOTS;
        game_state.week = 1;
        game_state.phase = Phase::Day1;
        game_state.moves_remaining = DAY_MOVES;
        game_state.total_moves = 0;
        game_state.boss_fight_ready = false;
        game_state.gold = 0;
        game_state.bump = ctx.bumps.game_state;

        // Store campaign level for boss fight validation
        game_state.campaign_level = campaign_level;

        // Player starts alive
        game_state.is_dead = false;

        emit!(GameStateInitialized {
            player: game_state.player,
            session: game_state.session,
            map_width,
            map_height,
        });

        Ok(())
    }

    /// Moves the player to an adjacent tile, deducting move cost.
    pub fn move_player(
        ctx: Context<MovePlayer>,
        target_x: u8,
        target_y: u8,
        is_wall: bool,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        // Check if boss fight already triggered
        require!(
            !game_state.boss_fight_ready,
            GameplayStateError::BossFightAlreadyTriggered
        );

        // Validate target is within bounds
        require!(
            target_x < game_state.map_width && target_y < game_state.map_height,
            GameplayStateError::OutOfBounds
        );

        // Validate target is adjacent (Manhattan distance = 1)
        let dx = (target_x as i16 - game_state.position_x as i16).abs();
        let dy = (target_y as i16 - game_state.position_y as i16).abs();
        require!(dx + dy == 1, GameplayStateError::NotAdjacent);

        // Calculate move cost
        let move_cost = if is_wall {
            // Wall dig cost: max(2, 6 - DIG)
            let dig_stat = game_state.dig as i16;
            (BASE_DIG_COST as i16 - dig_stat).max(MIN_DIG_COST as i16) as u8
        } else {
            FLOOR_MOVE_COST
        };

        // Check sufficient moves
        require!(
            game_state.moves_remaining >= move_cost,
            GameplayStateError::InsufficientMoves
        );

        // Store old position for event
        let from_x = game_state.position_x;
        let from_y = game_state.position_y;

        // Update position and moves
        game_state.position_x = target_x;
        game_state.position_y = target_y;
        game_state.moves_remaining = game_state
            .moves_remaining
            .checked_sub(move_cost)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        game_state.total_moves = game_state
            .total_moves
            .checked_add(1)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        emit!(PlayerMoved {
            player: game_state.player,
            from_x,
            from_y,
            to_x: target_x,
            to_y: target_y,
            moves_remaining: game_state.moves_remaining,
            is_dig: is_wall,
            combat_triggered: false,
            enemies_moved: 0,
        });

        // Handle phase advancement if moves exhausted
        if game_state.moves_remaining == 0 {
            handle_phase_advancement(game_state)?;
        }

        Ok(())
    }

    /// Modifies a player stat by a delta value.
    pub fn modify_stat(ctx: Context<ModifyStat>, stat: StatType, delta: i8) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        match stat {
            StatType::Hp => {
                let new_hp = (game_state.hp as i16)
                    .checked_add(delta as i16)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;

                // HP cannot go below 0
                require!(new_hp >= 0, GameplayStateError::HpUnderflow);

                // HP cannot exceed max_hp
                let clamped = new_hp.min(game_state.max_hp as i16) as i8;
                let old_value = game_state.hp;
                game_state.hp = clamped;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: clamped,
                });
            }
            StatType::MaxHp => {
                let new_max_hp = (game_state.max_hp as i16)
                    .checked_add(delta as i16)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;

                require!(
                    new_max_hp >= 0 && new_max_hp <= u8::MAX as i16,
                    GameplayStateError::StatOverflow
                );

                let old_value = game_state.max_hp as i8;
                game_state.max_hp = new_max_hp as u8;

                // Clamp current HP if it exceeds new max
                if game_state.hp > game_state.max_hp as i8 {
                    game_state.hp = game_state.max_hp as i8;
                }

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_max_hp as i8,
                });
            }
            StatType::Atk => {
                let new_atk = game_state
                    .atk
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.atk;
                game_state.atk = new_atk;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_atk,
                });
            }
            StatType::Arm => {
                let new_arm = game_state
                    .arm
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.arm;
                game_state.arm = new_arm;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_arm,
                });
            }
            StatType::Spd => {
                let new_spd = game_state
                    .spd
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.spd;
                game_state.spd = new_spd;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_spd,
                });
            }
            StatType::Dig => {
                let new_dig = game_state
                    .dig
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.dig;
                game_state.dig = new_dig;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_dig,
                });
            }
        }

        Ok(())
    }

    /// Closes the GameState account, returning rent to player.
    pub fn close_game_state(ctx: Context<CloseGameState>) -> Result<()> {
        let game_state = &ctx.accounts.game_state;

        emit!(GameStateClosed {
            player: game_state.player,
            total_moves: game_state.total_moves,
            final_phase: game_state.phase,
            final_week: game_state.week,
        });

        Ok(())
    }

    /// Modifies the player's gold by a delta value.
    pub fn modify_gold(ctx: Context<ModifyGold>, delta: i16) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        let new_gold = (game_state.gold as i32)
            .checked_add(delta as i32)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        // Gold cannot go below 0
        require!(new_gold >= 0, GameplayStateError::GoldUnderflow);

        // Gold cannot exceed u16::MAX
        require!(
            new_gold <= u16::MAX as i32,
            GameplayStateError::StatOverflow
        );

        let old_gold = game_state.gold;
        game_state.gold = new_gold as u16;

        emit!(GoldModified {
            player: game_state.player,
            old_gold,
            new_gold: game_state.gold,
            delta,
        });

        Ok(())
    }

    /// Moves the player to an adjacent tile with automatic combat resolution.
    ///
    /// This instruction handles:
    /// 1. Movement validation (bounds, adjacency, move cost)
    /// 2. Night phase enemy movement (enemies within 3 tiles move toward player)
    /// 3. Combat triggered by enemy moving into player's tile
    /// 4. Combat triggered by player moving into enemy's tile
    /// 5. Phase advancement when moves are exhausted
    ///
    /// Combat is resolved inline without CPI for compute efficiency.
    pub fn move_with_combat(
        ctx: Context<MoveWithCombat>,
        target_x: u8,
        target_y: u8,
        is_wall: bool,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let map_enemies = &mut ctx.accounts.map_enemies;

        // Check if player is dead
        require!(!game_state.is_dead, GameplayStateError::PlayerDead);

        // Check if boss fight already triggered
        require!(
            !game_state.boss_fight_ready,
            GameplayStateError::BossFightAlreadyTriggered
        );

        // Validate target is within bounds
        require!(
            is_within_bounds(
                target_x,
                target_y,
                game_state.map_width,
                game_state.map_height
            ),
            GameplayStateError::OutOfBounds
        );

        // Validate target is adjacent (Manhattan distance = 1)
        require!(
            is_adjacent(
                game_state.position_x,
                game_state.position_y,
                target_x,
                target_y
            ),
            GameplayStateError::NotAdjacent
        );

        // Calculate move cost
        let move_cost = calculate_move_cost(is_wall, game_state.dig);

        // Check sufficient moves
        require!(
            game_state.moves_remaining >= move_cost,
            GameplayStateError::InsufficientMoves
        );

        // Store old position for event
        let from_x = game_state.position_x;
        let from_y = game_state.position_y;

        let mut enemies_moved: u8 = 0;
        let mut combat_triggered = false;

        // **Night Phase Enemy Movement**
        // During night phases, enemies within 3 tiles (Chebyshev distance) move toward the player
        if game_state.phase.is_night() {
            let player_x = game_state.position_x;
            let player_y = game_state.position_y;

            for (enemy_idx, enemy) in map_enemies.enemies.iter_mut().enumerate() {
                if enemy.defeated {
                    continue;
                }

                let distance = chebyshev_distance(enemy.x, enemy.y, player_x, player_y);
                if distance <= 3 && distance > 0 {
                    let old_x = enemy.x;
                    let old_y = enemy.y;

                    let (new_x, new_y) = move_toward(enemy.x, enemy.y, player_x, player_y);
                    enemy.x = new_x;
                    enemy.y = new_y;
                    enemies_moved += 1;

                    emit!(EnemyMoved {
                        enemy_index: enemy_idx as u8,
                        from_x: old_x,
                        from_y: old_y,
                        to_x: new_x,
                        to_y: new_y,
                    });

                    // Check if enemy moved into player's position
                    if new_x == player_x && new_y == player_y {
                        // Trigger combat - enemy attacks player
                        let enemy_stats = field_enemies::archetypes::get_enemy_stats(
                            enemy.archetype_id,
                            enemy.tier,
                        );

                        if let Some(stats) = enemy_stats {
                            emit!(CombatStarted {
                                player: game_state.player,
                                player_hp: game_state.hp as i16,
                                player_atk: game_state.atk as i16,
                                enemy_archetype: enemy.archetype_id,
                                enemy_hp: stats.hp as i16,
                                enemy_atk: stats.atk as i16,
                            });

                            let mut player_combat = InlineCombatStats {
                                hp: game_state.hp as i16,
                                max_hp: game_state.max_hp as u16,
                                atk: game_state.atk as i16,
                                arm: game_state.arm as i16,
                                spd: game_state.spd as i16,
                                strikes: 1,
                            };
                            let mut enemy_combat = InlineCombatStats {
                                hp: stats.hp as i16,
                                max_hp: stats.hp,
                                atk: stats.atk as i16,
                                arm: stats.arm as i16,
                                spd: stats.spd as i16,
                                strikes: 1,
                            };

                            let result =
                                resolve_combat_inline(&mut player_combat, &mut enemy_combat);
                            combat_triggered = true;

                            // Get gold reward based on enemy tier
                            let tier_enum = field_enemies::state::EnemyTier::from_u8(enemy.tier);
                            require!(tier_enum.is_some(), GameplayStateError::InvalidEnemyTier);
                            let gold_reward = tier_enum.unwrap().gold_reward() as u16;

                            emit!(CombatEnded {
                                player: game_state.player,
                                player_won: result.player_won,
                                final_player_hp: result.final_player_hp,
                                final_enemy_hp: result.final_enemy_hp,
                                gold_earned: if result.player_won { gold_reward } else { 0 },
                                turns_taken: result.turns_taken,
                            });

                            // Update player HP
                            game_state.hp = result.final_player_hp as i8;

                            if result.player_won {
                                // Mark enemy as defeated
                                enemy.defeated = true;

                                // Award gold
                                game_state.gold =
                                    game_state.gold.checked_add(gold_reward).unwrap_or(u16::MAX);
                            } else {
                                // Player defeated - persist death state and return Ok
                                game_state.is_dead = true;
                                game_state.hp = 0;

                                emit!(PlayerDefeated {
                                    player: game_state.player,
                                    killed_by: DeathCause::Enemy,
                                    final_hp: result.final_player_hp,
                                });

                                // Return Ok to persist the death state
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }

        // **Move Player**
        game_state.position_x = target_x;
        game_state.position_y = target_y;
        game_state.moves_remaining = game_state
            .moves_remaining
            .checked_sub(move_cost)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        game_state.total_moves = game_state
            .total_moves
            .checked_add(1)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        // **Check for enemy at target position**
        // We need to check all enemies since we can't use the helper after borrowing mutably
        let mut target_enemy_idx: Option<usize> = None;
        for (idx, enemy) in map_enemies.enemies.iter().enumerate() {
            if !enemy.defeated && enemy.x == target_x && enemy.y == target_y {
                target_enemy_idx = Some(idx);
                break;
            }
        }

        if let Some(enemy_idx) = target_enemy_idx {
            let enemy = &mut map_enemies.enemies[enemy_idx];

            let enemy_stats =
                field_enemies::archetypes::get_enemy_stats(enemy.archetype_id, enemy.tier);

            if let Some(stats) = enemy_stats {
                emit!(CombatStarted {
                    player: game_state.player,
                    player_hp: game_state.hp as i16,
                    player_atk: game_state.atk as i16,
                    enemy_archetype: enemy.archetype_id,
                    enemy_hp: stats.hp as i16,
                    enemy_atk: stats.atk as i16,
                });

                let mut player_combat = InlineCombatStats {
                    hp: game_state.hp as i16,
                    max_hp: game_state.max_hp as u16,
                    atk: game_state.atk as i16,
                    arm: game_state.arm as i16,
                    spd: game_state.spd as i16,
                    strikes: 1,
                };
                let mut enemy_combat = InlineCombatStats {
                    hp: stats.hp as i16,
                    max_hp: stats.hp,
                    atk: stats.atk as i16,
                    arm: stats.arm as i16,
                    spd: stats.spd as i16,
                    strikes: 1,
                };

                let result = resolve_combat_inline(&mut player_combat, &mut enemy_combat);
                combat_triggered = true;

                // Get gold reward based on enemy tier
                let tier_enum = field_enemies::state::EnemyTier::from_u8(enemy.tier);
                require!(tier_enum.is_some(), GameplayStateError::InvalidEnemyTier);
                let gold_reward = tier_enum.unwrap().gold_reward() as u16;

                emit!(CombatEnded {
                    player: game_state.player,
                    player_won: result.player_won,
                    final_player_hp: result.final_player_hp,
                    final_enemy_hp: result.final_enemy_hp,
                    gold_earned: if result.player_won { gold_reward } else { 0 },
                    turns_taken: result.turns_taken,
                });

                // Update player HP
                game_state.hp = result.final_player_hp as i8;

                if result.player_won {
                    // Mark enemy as defeated
                    enemy.defeated = true;

                    // Award gold
                    game_state.gold = game_state.gold.checked_add(gold_reward).unwrap_or(u16::MAX);
                } else {
                    // Player defeated - persist death state and return Ok
                    game_state.is_dead = true;
                    game_state.hp = 0;

                    emit!(PlayerDefeated {
                        player: game_state.player,
                        killed_by: DeathCause::Enemy,
                        final_hp: result.final_player_hp,
                    });

                    // Return Ok to persist the death state
                    return Ok(());
                }
            }
        }

        emit!(PlayerMoved {
            player: game_state.player,
            from_x,
            from_y,
            to_x: target_x,
            to_y: target_y,
            moves_remaining: game_state.moves_remaining,
            is_dig: is_wall,
            combat_triggered,
            enemies_moved,
        });

        // Handle phase advancement if moves exhausted
        if game_state.moves_remaining == 0 {
            handle_phase_advancement(game_state)?;
        }

        Ok(())
    }

    /// Triggers and resolves the boss fight when conditions are met.
    ///
    /// This instruction handles:
    /// 1. Validation that boss fight is ready (boss_fight_ready flag set)
    /// 2. Boss selection based on stored campaign_level and week
    /// 3. Combat resolution inline
    /// 4. Victory handling: week advancement or level completion
    /// 5. Defeat handling: player death persisted in state
    ///
    /// Must be called after move_with_combat sets boss_fight_ready = true.
    pub fn trigger_boss_fight(ctx: Context<TriggerBossFight>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        // Check if player is dead
        require!(!game_state.is_dead, GameplayStateError::PlayerDead);

        // Validate boss fight is ready
        require!(
            game_state.boss_fight_ready,
            GameplayStateError::BossFightNotReady
        );

        // Use the stored campaign_level instead of user-provided stage
        let stage = game_state.campaign_level;

        // Get boss stats for this stage and week
        let mut boss_stats = get_boss_for_combat(stage, game_state.week)?;
        let boss_id = get_boss_id(stage, game_state.week)?;

        emit!(BossCombatStarted {
            player: game_state.player,
            boss_id,
            boss_hp: boss_stats.hp,
            week: game_state.week,
        });

        // Setup player combat stats
        let mut player_combat = InlineCombatStats {
            hp: game_state.hp as i16,
            max_hp: game_state.max_hp as u16,
            atk: game_state.atk as i16,
            arm: game_state.arm as i16,
            spd: game_state.spd as i16,
            strikes: 1,
        };

        // Resolve combat
        let result = resolve_combat_inline(&mut player_combat, &mut boss_stats);

        emit!(CombatEnded {
            player: game_state.player,
            player_won: result.player_won,
            final_player_hp: result.final_player_hp,
            final_enemy_hp: result.final_enemy_hp,
            gold_earned: 0, // Boss doesn't give gold directly
            turns_taken: result.turns_taken,
        });

        // Update player HP
        game_state.hp = result.final_player_hp as i8;

        if result.player_won {
            // Boss defeated - handle week/level progression
            game_state.boss_fight_ready = false;

            if game_state.week >= 3 {
                // Week 3 boss defeated - level complete!
                emit!(LevelCompleted {
                    player: game_state.player,
                    level: stage,
                    total_moves: game_state.total_moves,
                    gold_earned: game_state.gold,
                });

                // Mark session as complete (the client should call end_session)
                // For now, just emit the event and keep the state
            } else {
                // Week 1 or 2 boss defeated - advance to next week
                game_state.week = game_state
                    .week
                    .checked_add(1)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;
                game_state.phase = Phase::Day1;
                game_state.moves_remaining = DAY_MOVES;

                // Increase gear slots (capped at MAX_GEAR_SLOTS)
                game_state.gear_slots = game_state
                    .gear_slots
                    .checked_add(2)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?
                    .min(MAX_GEAR_SLOTS);

                emit!(PhaseAdvanced {
                    player: game_state.player,
                    new_phase: Phase::Day1,
                    new_week: game_state.week,
                    moves_remaining: game_state.moves_remaining,
                });
            }

            Ok(())
        } else {
            // Player defeated by boss - persist death state and return Ok
            game_state.is_dead = true;
            game_state.hp = 0;

            emit!(PlayerDefeated {
                player: game_state.player,
                killed_by: DeathCause::Boss,
                final_hp: result.final_player_hp,
            });

            Ok(())
        }
    }
}

/// Helper function to handle phase advancement when moves are exhausted
fn handle_phase_advancement(game_state: &mut GameState) -> Result<()> {
    match game_state.phase.next() {
        Some(next_phase) => {
            // Advance to next phase
            game_state.phase = next_phase;
            game_state.moves_remaining = next_phase.moves_allowed();

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: next_phase,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        }
        None => {
            // Night3 complete - handle week transition or boss fight
            if game_state.week >= 3 {
                // Week 3 Night 3 complete - trigger boss fight
                game_state.boss_fight_ready = true;

                emit!(BossFightReady {
                    player: game_state.player,
                    week: game_state.week,
                });
            } else {
                // Advance to next week
                game_state.week = game_state
                    .week
                    .checked_add(1)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;
                game_state.phase = Phase::Day1;
                game_state.moves_remaining = DAY_MOVES;

                // Increase gear slots (capped at MAX_GEAR_SLOTS)
                game_state.gear_slots = game_state
                    .gear_slots
                    .checked_add(2)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?
                    .min(MAX_GEAR_SLOTS);

                emit!(PhaseAdvanced {
                    player: game_state.player,
                    new_phase: Phase::Day1,
                    new_week: game_state.week,
                    moves_remaining: game_state.moves_remaining,
                });
            }
        }
    }

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeGameState<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + GameState::INIT_SPACE,
        seeds = [GAME_STATE_SEED, game_session.key().as_ref()],
        bump
    )]
    pub game_state: Account<'info, GameState>,

    /// The linked GameSession PDA (must exist)
    /// CHECK: We only verify this account exists as validation of the session
    pub game_session: AccountInfo<'info>,

    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MovePlayer<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct ModifyStat<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseGameState<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
        close = player,
    )]
    pub game_state: Account<'info, GameState>,

    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct ModifyGold<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct MoveWithCombat<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        seeds = [field_enemies::state::MapEnemies::SEED_PREFIX, game_state.session.as_ref()],
        bump = map_enemies.bump,
        seeds::program = field_enemies::ID,
    )]
    pub map_enemies: Account<'info, field_enemies::state::MapEnemies>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct TriggerBossFight<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    pub player: Signer<'info>,
}

// Events

#[event]
pub struct GameStateInitialized {
    pub player: Pubkey,
    pub session: Pubkey,
    pub map_width: u8,
    pub map_height: u8,
}

#[event]
pub struct PlayerMoved {
    pub player: Pubkey,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
    pub moves_remaining: u8,
    pub is_dig: bool,
    pub combat_triggered: bool,
    pub enemies_moved: u8,
}

#[event]
pub struct PhaseAdvanced {
    pub player: Pubkey,
    pub new_phase: Phase,
    pub new_week: u8,
    pub moves_remaining: u8,
}

#[event]
pub struct BossFightReady {
    pub player: Pubkey,
    pub week: u8,
}

#[event]
pub struct StatModified {
    pub player: Pubkey,
    pub stat: StatType,
    pub old_value: i8,
    pub new_value: i8,
}

#[event]
pub struct GameStateClosed {
    pub player: Pubkey,
    pub total_moves: u32,
    pub final_phase: Phase,
    pub final_week: u8,
}

#[event]
pub struct GoldModified {
    pub player: Pubkey,
    pub old_gold: u16,
    pub new_gold: u16,
    pub delta: i16,
}

/// Emitted when combat starts (either player walked into enemy or enemy walked into player)
#[event]
pub struct CombatStarted {
    pub player: Pubkey,
    pub player_hp: i16,
    pub player_atk: i16,
    pub enemy_archetype: u8,
    pub enemy_hp: i16,
    pub enemy_atk: i16,
}

/// Emitted when combat ends
#[event]
pub struct CombatEnded {
    pub player: Pubkey,
    pub player_won: bool,
    pub final_player_hp: i16,
    pub final_enemy_hp: i16,
    pub gold_earned: u16,
    pub turns_taken: u8,
}

/// Emitted when an enemy moves during night phase
#[event]
pub struct EnemyMoved {
    pub enemy_index: u8,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
}

/// Emitted when boss combat starts
#[event]
pub struct BossCombatStarted {
    pub player: Pubkey,
    pub boss_id: [u8; 12],
    pub boss_hp: i16,
    pub week: u8,
}

/// Emitted when the player is defeated (HP <= 0)
#[event]
pub struct PlayerDefeated {
    pub player: Pubkey,
    pub killed_by: DeathCause,
    pub final_hp: i16,
}

/// Cause of player death - uses enum instead of String for efficiency
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeathCause {
    /// Killed by a field enemy
    Enemy = 0,
    /// Killed by a boss
    Boss = 1,
}

/// Emitted when a level is completed (Week 3 boss defeated)
#[event]
pub struct LevelCompleted {
    pub player: Pubkey,
    pub level: u8,
    pub total_moves: u32,
    pub gold_earned: u16,
}
