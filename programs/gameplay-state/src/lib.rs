use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod movement;
pub mod state;
pub mod stats;

use combat_system::state::CombatantInput;
use combat_system::{resolve_combat, CombatLogEntry, EffectType, ItemEffect};
use constants::{BASE_HP, DAY_MOVES, GAME_STATE_SEED, INITIAL_GEAR_SLOTS, MAX_GEAR_SLOTS};
use errors::GameplayStateError;

/// Seed for gameplay_authority PDA used for CPI calls to other programs
pub const GAMEPLAY_AUTHORITY_SEED: &[u8] = b"gameplay_authority";
use movement::{
    calculate_move_cost, chebyshev_distance, get_boss_for_combat, get_boss_id, is_adjacent,
    is_within_bounds,
};
use player_inventory::effects::generate_combat_effects;
use player_inventory::state::PlayerInventory;
use state::{GameState, MapEnemies, Phase};
use stats::{calculate_stats, PlayerStats};

declare_id!("5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n");

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    217, 18, 17, 128, 79, 140, 152, 73, 103, 95, 134, 179, 31, 109, 34, 82, 250, 167, 91, 67, 186,
    23, 209, 2, 80, 255, 118, 192, 175, 242, 222, 183,
]);

/// POI system program ID for authorized HP/Gold modifications
/// Derived from "6E27r1Cyo2CNPvtRsonn3uHUAdznS3cMXEBX4HRbfBQY"
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    77, 160, 63, 209, 182, 56, 149, 181, 2, 195, 173, 95, 65, 136, 88, 122, 235, 166, 235, 216,
    241, 107, 2, 35, 185, 14, 177, 21, 150, 103, 215, 77,
]);

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
        require_keys_eq!(
            *ctx.accounts.game_session.owner,
            SESSION_MANAGER_PROGRAM_ID,
            GameplayStateError::InvalidSessionOwner
        );

        require!(
            start_x < map_width && start_y < map_height,
            GameplayStateError::OutOfBounds
        );

        let game_state = &mut ctx.accounts.game_state;
        game_state.player = ctx.accounts.player.key();
        game_state.burner_wallet = ctx.accounts.burner_wallet.key();
        game_state.session = ctx.accounts.game_session.key();
        game_state.position_x = start_x;
        game_state.position_y = start_y;
        game_state.map_width = map_width;
        game_state.map_height = map_height;
        game_state.hp = BASE_HP;
        game_state.gear_slots = INITIAL_GEAR_SLOTS;
        game_state.week = 1;
        game_state.phase = Phase::Day1;
        game_state.moves_remaining = DAY_MOVES;
        game_state.total_moves = 0;
        game_state.boss_fight_ready = false;
        game_state.gold = 0;
        game_state.bump = ctx.bumps.game_state;
        game_state.campaign_level = campaign_level;
        game_state.is_dead = false;

        let map_enemies = &mut ctx.accounts.map_enemies;
        let generated_map = &ctx.accounts.generated_map;

        map_enemies.session = ctx.accounts.game_session.key();
        map_enemies.bump = ctx.bumps.map_enemies;
        map_enemies.enemies = Vec::with_capacity(generated_map.enemy_count as usize);

        for idx in 0..generated_map.enemy_count as usize {
            let enemy = generated_map.enemies[idx];
            map_enemies.enemies.push(state::EnemyInstance {
                archetype_id: enemy.archetype_id,
                tier: enemy.tier,
                x: enemy.x,
                y: enemy.y,
                defeated: false,
            });
        }

        map_enemies.count = map_enemies.enemies.len() as u8;

        emit!(GameStateInitialized {
            player: game_state.player,
            session: game_state.session,
            map_width,
            map_height,
        });

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

    /// Heals the player by a specified amount, authorized by poi-system.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used for rest POI healing.
    ///
    /// The max HP is derived from the player's inventory (equipped items).
    /// HP is capped at the derived max_hp value.
    pub fn heal_player(ctx: Context<HealPlayer>, amount: u16) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;
        let player_stats = calculate_stats(inventory);

        let old_hp = game_state.hp;
        let new_hp = (game_state.hp as i32)
            .checked_add(amount as i32)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        let capped_hp = new_hp.min(player_stats.max_hp as i32);
        require!(
            capped_hp <= i16::MAX as i32,
            GameplayStateError::StatOverflow
        );

        game_state.hp = capped_hp as i16;

        emit!(PlayerHealed {
            player: game_state.player,
            old_hp,
            new_hp: game_state.hp,
            amount,
            max_hp: player_stats.max_hp,
        });

        Ok(())
    }

    /// Skips to the next Day phase, authorized by poi-system.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used by rest POIs (L1 Mole Den, L5 Rest Alcove)
    /// to skip the night phase.
    ///
    /// Behavior:
    /// - Night1 → Day2 (reset moves to DAY_MOVES)
    /// - Night2 → Day3 (reset moves to DAY_MOVES)
    /// - Night3 → triggers boss fight (cannot skip end-of-week boss)
    ///
    /// Returns an error if called during a Day phase.
    pub fn skip_to_day(ctx: Context<SkipToDay>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;
        let inventory_info = &ctx.accounts.inventory.to_account_info();
        let player = &ctx.accounts.player;
        let player_inventory_program = &ctx.accounts.player_inventory_program;

        require!(!game_state.is_dead, GameplayStateError::PlayerDead);
        require!(
            !game_state.boss_fight_ready,
            GameplayStateError::BossFightAlreadyTriggered
        );
        require!(
            game_state.phase.is_night(),
            GameplayStateError::NotNightPhase
        );

        if game_state.phase.is_night3() {
            // Night3: Cannot skip the boss fight - trigger it instead
            game_state.boss_fight_ready = true;

            emit!(BossFightReady {
                player: game_state.player,
                week: game_state.week,
            });

            // Resolve boss fight inline (same as move_player does)
            let player_won = resolve_boss_fight(
                game_state,
                inventory,
                inventory_info,
                player,
                player_inventory_program,
            )?;

            if !player_won {
                return Ok(());
            }
        } else {
            // Night1 or Night2: Skip to the next Day phase
            let next_day = match game_state.phase {
                Phase::Night1 => Phase::Day2,
                Phase::Night2 => Phase::Day3,
                _ => unreachable!(), // Already validated is_night() and not is_night3()
            };

            game_state.phase = next_day;
            game_state.moves_remaining = DAY_MOVES;

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: next_day,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        }

        Ok(())
    }

    /// Syncs HP to reflect current inventory bonuses.
    ///
    /// This instruction should be called after equipping gear that provides +HP.
    /// It calculates the new max_hp from the inventory and adjusts current HP:
    /// - If player was at full base health (hp == BASE_HP), set hp to new max_hp
    /// - If player was damaged, add the HP bonus (max_hp - BASE_HP) to current hp
    /// - HP is always capped at the new max_hp
    ///
    /// This ensures that equipping +HP gear immediately grants that HP.
    pub fn sync_hp_from_inventory(ctx: Context<SyncHpFromInventory>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;
        let player_stats = calculate_stats(inventory);

        let old_hp = game_state.hp;
        let old_max = BASE_HP; // Base max HP without gear bonuses
        let new_max = player_stats.max_hp;
        let hp_bonus = new_max.saturating_sub(old_max);

        // Calculate new HP:
        // - If at full base health, set to new max
        // - If damaged, add the bonus (but cap at new max)
        let new_hp = if old_hp >= old_max {
            // Player was at or above base max (full health or already had bonuses)
            new_max
        } else {
            // Player was damaged, add the bonus
            old_hp.saturating_add(hp_bonus).min(new_max)
        };

        game_state.hp = new_hp;

        emit!(HpSynced {
            player: game_state.player,
            old_hp,
            new_hp: game_state.hp,
            hp_bonus,
            max_hp: new_max,
        });

        Ok(())
    }

    /// Modifies the player's gold by a delta value, authorized by poi-system.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used for shop purchases, rerolls,
    /// rusty anvil upgrades, and scrap chute costs.
    pub fn modify_gold_authorized(ctx: Context<ModifyGoldAuthorized>, delta: i16) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        let new_gold = (game_state.gold as i32)
            .checked_add(delta as i32)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        require!(new_gold >= 0, GameplayStateError::GoldUnderflow);
        require!(
            new_gold <= u16::MAX as i32,
            GameplayStateError::StatOverflow
        );

        let old_gold = game_state.gold;
        game_state.gold = new_gold as u16;

        emit!(GoldModifiedAuthorized {
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
    pub fn move_player(ctx: Context<Move>, target_x: u8, target_y: u8) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let map_enemies = &mut ctx.accounts.map_enemies;
        let generated_map = &ctx.accounts.generated_map;
        let inventory = &ctx.accounts.inventory;
        let inventory_info = &ctx.accounts.inventory.to_account_info();
        let player = &ctx.accounts.player;
        let player_inventory_program = &ctx.accounts.player_inventory_program;

        require!(!game_state.is_dead, GameplayStateError::PlayerDead);
        require!(
            !game_state.boss_fight_ready,
            GameplayStateError::BossFightAlreadyTriggered
        );
        require!(
            is_within_bounds(
                target_x,
                target_y,
                game_state.map_width,
                game_state.map_height
            ),
            GameplayStateError::OutOfBounds
        );
        require!(
            is_adjacent(
                game_state.position_x,
                game_state.position_y,
                target_x,
                target_y
            ),
            GameplayStateError::NotAdjacent
        );

        let is_wall = !generated_map.is_walkable(target_x, target_y);
        let player_stats = calculate_stats(inventory);
        let move_cost = calculate_move_cost(is_wall, player_stats.dig);

        // Check if move can be afforded in current phase or by spanning phases
        let needs_phase_span = game_state.moves_remaining < move_cost;
        let can_span_phases = !game_state.phase.is_night3() && game_state.phase.next().is_some();

        if needs_phase_span {
            if !can_span_phases {
                // Night3 or no next phase - cannot span
                return Err(GameplayStateError::InsufficientMoves.into());
            }
            // Check if we can afford by spanning to next phase
            let next_phase = game_state.phase.next().unwrap();
            let total_available =
                game_state.moves_remaining as u16 + next_phase.moves_allowed() as u16;
            require!(
                total_available >= move_cost as u16,
                GameplayStateError::InsufficientMoves
            );
        }

        let is_last_move_of_week =
            game_state.phase.is_night3() && game_state.moves_remaining == move_cost;
        let from_x = game_state.position_x;
        let from_y = game_state.position_y;

        let mut enemies_moved: u8 = 0;
        let mut combat_triggered = false;

        if map_enemies.enemies.iter().any(|enemy| enemy.defeated) {
            map_enemies.enemies.retain(|enemy| !enemy.defeated);
            map_enemies.count = map_enemies.enemies.len() as u8;
        }

        let map_width = generated_map.width as usize;
        let map_height = generated_map.height as usize;
        let mut occupied = vec![false; map_width.saturating_mul(map_height)];
        for enemy in map_enemies.enemies.iter() {
            let index = (enemy.y as usize) * map_width + (enemy.x as usize);
            if index < occupied.len() {
                occupied[index] = true;
            }
        }

        let mut player_tile_blocked = false;

        // Night phase: enemies within 3 tiles (Chebyshev distance) move toward player
        if game_state.phase.is_night() {
            let player_x = game_state.position_x;
            let player_y = game_state.position_y;
            let mut enemy_idx = 0usize;

            while enemy_idx < map_enemies.enemies.len() {
                let enemy = map_enemies.enemies[enemy_idx];
                let distance = chebyshev_distance(enemy.x, enemy.y, player_x, player_y);
                if distance > 0 && distance <= 3 {
                    let old_x = enemy.x;
                    let old_y = enemy.y;

                    if let Some((new_x, new_y)) = select_enemy_step(
                        enemy.x,
                        enemy.y,
                        player_x,
                        player_y,
                        generated_map,
                        &occupied,
                        map_width,
                        player_tile_blocked,
                    ) {
                        let old_index = (old_y as usize) * map_width + (old_x as usize);
                        if old_index < occupied.len() {
                            occupied[old_index] = false;
                        }

                        if new_x == player_x && new_y == player_y {
                            player_tile_blocked = true;
                        } else {
                            let new_index = (new_y as usize) * map_width + (new_x as usize);
                            if new_index < occupied.len() {
                                occupied[new_index] = true;
                            }
                        }

                        map_enemies.enemies[enemy_idx].x = new_x;
                        map_enemies.enemies[enemy_idx].y = new_y;
                        enemies_moved = enemies_moved.saturating_add(1);

                        emit!(EnemyMoved {
                            enemy_index: enemy_idx as u8,
                            from_x: old_x,
                            from_y: old_y,
                            to_x: new_x,
                            to_y: new_y,
                        });

                        if new_x == player_x && new_y == player_y {
                            combat_triggered = true;
                            let player_won = resolve_enemy_combat(
                                game_state,
                                inventory,
                                map_enemies,
                                enemy_idx,
                            )?;
                            if !player_won {
                                return Ok(());
                            }

                            if enemy_idx < map_enemies.enemies.len() {
                                continue;
                            }
                        }
                    }
                }

                enemy_idx = enemy_idx.saturating_add(1);
            }
        }

        // Convert wall to floor via CPI so the tile change persists on-chain
        // (map_generator owns the GeneratedMap account, so we must use CPI)
        if is_wall {
            set_tile_floor_cpi(
                &ctx.accounts.generated_map.to_account_info(),
                &ctx.accounts.game_session,
                &ctx.accounts.gameplay_authority,
                &ctx.accounts.map_generator_program.to_account_info(),
                ctx.bumps.gameplay_authority,
                target_x,
                target_y,
            )?;
        }

        game_state.position_x = target_x;
        game_state.position_y = target_y;

        // Handle move cost consumption, potentially spanning phases
        if needs_phase_span {
            // Consume all moves from current phase
            let moves_from_current = game_state.moves_remaining;
            let remaining_cost = move_cost - moves_from_current;

            // Advance to next phase
            let next_phase = game_state.phase.next().unwrap();
            game_state.phase = next_phase;
            game_state.moves_remaining = next_phase
                .moves_allowed()
                .checked_sub(remaining_cost)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: next_phase,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        } else {
            // Simple subtraction within same phase
            game_state.moves_remaining = game_state
                .moves_remaining
                .checked_sub(move_cost)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
        }

        game_state.total_moves = game_state
            .total_moves
            .checked_add(1)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        let target_enemy_idx = find_enemy_index(map_enemies, target_x, target_y);

        if !is_last_move_of_week {
            if let Some(enemy_idx) = target_enemy_idx {
                combat_triggered = true;
                let player_won =
                    resolve_enemy_combat(game_state, inventory, map_enemies, enemy_idx)?;
                if !player_won {
                    return Ok(());
                }
            }
        } else {
            combat_triggered = true;
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

        if game_state.moves_remaining == 0 {
            if game_state.phase.is_night3() {
                game_state.boss_fight_ready = true;

                emit!(BossFightReady {
                    player: game_state.player,
                    week: game_state.week,
                });

                let player_won = resolve_boss_fight(
                    game_state,
                    inventory,
                    inventory_info,
                    player,
                    player_inventory_program,
                )?;
                if !player_won {
                    return Ok(());
                }

                if let Some(enemy_idx) =
                    find_enemy_index(map_enemies, game_state.position_x, game_state.position_y)
                {
                    let player_won =
                        resolve_enemy_combat(game_state, inventory, map_enemies, enemy_idx)?;
                    if !player_won {
                        return Ok(());
                    }
                }
            } else {
                handle_phase_advancement(game_state)?;
            }
        }

        map_enemies.count = map_enemies.enemies.len() as u8;

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
    /// Must be called after move sets boss_fight_ready = true.
    pub fn trigger_boss_fight(ctx: Context<TriggerBossFight>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let map_enemies = &mut ctx.accounts.map_enemies;
        let inventory = &ctx.accounts.inventory;
        let inventory_info = &ctx.accounts.inventory.to_account_info();
        let player = &ctx.accounts.player;
        let player_inventory_program = &ctx.accounts.player_inventory_program;

        require!(!game_state.is_dead, GameplayStateError::PlayerDead);
        require!(
            game_state.boss_fight_ready,
            GameplayStateError::BossFightNotReady
        );

        let player_won = resolve_boss_fight(
            game_state,
            inventory,
            inventory_info,
            player,
            player_inventory_program,
        )?;
        if !player_won {
            return Ok(());
        }

        if let Some(enemy_idx) =
            find_enemy_index(map_enemies, game_state.position_x, game_state.position_y)
        {
            let player_won = resolve_enemy_combat(game_state, inventory, map_enemies, enemy_idx)?;
            if !player_won {
                return Ok(());
            }
        }

        map_enemies.count = map_enemies.enemies.len() as u8;

        Ok(())
    }

    /// TEST ONLY: Sets the game phase and moves remaining directly.
    /// This instruction is intended for testing purposes to avoid
    /// doing hundreds of move transactions to reach a specific phase.
    ///
    /// WARNING: This should only be used in test environments.
    #[allow(unused_variables)]
    pub fn set_phase_for_testing(
        ctx: Context<SetPhaseForTesting>,
        phase: Phase,
        moves_remaining: u8,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        game_state.phase = phase;
        game_state.moves_remaining = moves_remaining;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct SetPhaseForTesting<'info> {
    #[account(
        mut,
        has_one = burner_wallet,
    )]
    pub game_state: Account<'info, GameState>,
    pub burner_wallet: Signer<'info>,
}

fn find_enemy_index(map_enemies: &MapEnemies, x: u8, y: u8) -> Option<usize> {
    map_enemies
        .enemies
        .iter()
        .position(|enemy| !enemy.defeated && enemy.x == x && enemy.y == y)
}

fn remove_enemy(map_enemies: &mut MapEnemies, enemy_index: usize) {
    if enemy_index >= map_enemies.enemies.len() {
        return;
    }
    map_enemies.enemies.swap_remove(enemy_index);
    map_enemies.count = map_enemies.enemies.len() as u8;
}

fn build_player_combatant(
    current_hp: i16,
    stats: &PlayerStats,
    _player_effects: &[ItemEffect],
) -> CombatantInput {
    // NOTE: BattleStart Heal effects are already included in stats.max_hp
    // via calculate_stats(). They represent permanent max HP bonuses from items
    // (e.g., Health Ring), not temporary combat-only boosts.
    //
    // current_hp is clamped to stats.max_hp to prevent exceeding derived max.
    let combat_hp = current_hp.clamp(1, stats.max_hp);

    CombatantInput {
        hp: combat_hp,
        max_hp: stats.max_hp as u16,
        atk: stats.atk,
        arm: stats.arm,
        spd: stats.spd,
        dig: stats.dig,
        strikes: 1,
    }
}

/// Preprocess enemy effects to handle dynamic calculations.
///
/// Currently handles:
/// - Coin Slug (id=10): BattleStart GainArmor based on player gold (floor(gold/10), cap 3)
fn preprocess_enemy_effects(archetype_id: u8, player_gold: u16) -> Vec<ItemEffect> {
    let base_effects = field_enemies::traits::get_enemy_traits(archetype_id);

    // Coin Slug: armor = min(player_gold / 10, 3)
    if archetype_id == field_enemies::archetypes::ids::COIN_SLUG {
        let armor_from_gold = ((player_gold / 10) as i16).min(3);
        return base_effects
            .iter()
            .map(|effect| {
                if matches!(effect.effect_type, EffectType::GainArmor) {
                    ItemEffect {
                        value: armor_from_gold,
                        ..*effect
                    }
                } else {
                    *effect
                }
            })
            .collect();
    }

    base_effects.to_vec()
}

fn resolve_enemy_combat(
    game_state: &mut GameState,
    inventory: &PlayerInventory,
    map_enemies: &mut MapEnemies,
    enemy_index: usize,
) -> Result<bool> {
    let enemy = map_enemies.enemies[enemy_index];
    let enemy_input = match field_enemies::archetypes::get_enemy_combatant_input(
        enemy.archetype_id,
        enemy.tier,
    ) {
        Some(input) => input,
        None => return Ok(true),
    };

    let player_stats = calculate_stats(inventory);
    let player_effects = generate_combat_effects(inventory);
    let player_input = build_player_combatant(game_state.hp, &player_stats, &player_effects);
    let enemy_effects = preprocess_enemy_effects(enemy.archetype_id, game_state.gold);

    emit!(CombatStarted {
        player: game_state.player,
        player_hp: game_state.hp,
        player_atk: player_stats.atk,
        enemy_archetype: enemy.archetype_id,
        enemy_hp: enemy_input.hp,
        enemy_atk: enemy_input.atk,
    });

    let result = resolve_combat(player_input, enemy_input, player_effects, enemy_effects)?;

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

    emit!(CombatLog {
        player: game_state.player,
        entries: result.log,
    });

    // HP capped at max_hp (discarding temp combat bonuses)
    game_state.hp = result.final_player_hp.min(player_stats.max_hp);

    // Gold changes from two sources (applied in order):
    // 1. gold_change: From combat effects (e.g., Ore Tick's StealGold trait).
    //    Can be negative if enemy stole gold. Clamped to not go below 0.
    // 2. gold_reward: Tier-based victory reward (T1=5, T2=10, T3=20).
    //    Only awarded if player won.
    // Example: If enemy steals 5 gold and player wins T1 fight:
    //   final_gold = (initial - 5) + 5 = initial
    let new_gold = (game_state.gold as i32)
        .saturating_add(result.gold_change as i32)
        .max(0) as u16;
    game_state.gold = new_gold;

    if result.player_won {
        remove_enemy(map_enemies, enemy_index);
        game_state.gold = game_state.gold.checked_add(gold_reward).unwrap_or(u16::MAX);
        Ok(true)
    } else {
        game_state.is_dead = true;
        game_state.hp = 0;

        emit!(PlayerDefeated {
            player: game_state.player,
            killed_by: DeathCause::Enemy,
            final_hp: result.final_player_hp,
        });

        // Session cleanup is handled by the frontend calling end_session
        // with the main wallet after detecting death.
        Ok(false)
    }
}

fn resolve_boss_fight<'info>(
    game_state: &mut GameState,
    inventory: &PlayerInventory,
    inventory_info: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
) -> Result<bool> {
    let stage = game_state.campaign_level;
    let boss_input = get_boss_for_combat(stage, game_state.week)?;
    let boss_id = get_boss_id(stage, game_state.week)?;
    let boss_week = movement::to_boss_week(game_state.week)?;
    let boss_definition = boss_system::select_boss(stage, boss_week);
    let boss_effects = boss_system::get_boss_item_effects(boss_definition);

    let player_stats = calculate_stats(inventory);
    let player_effects = generate_combat_effects(inventory);
    let player_input = build_player_combatant(game_state.hp, &player_stats, &player_effects);

    emit!(BossCombatStarted {
        player: game_state.player,
        boss_id,
        boss_hp: boss_input.hp,
        week: game_state.week,
    });

    let result = resolve_combat(player_input, boss_input, player_effects, boss_effects)?;

    emit!(CombatEnded {
        player: game_state.player,
        player_won: result.player_won,
        final_player_hp: result.final_player_hp,
        final_enemy_hp: result.final_enemy_hp,
        gold_earned: 0,
        turns_taken: result.turns_taken,
    });

    emit!(CombatLog {
        player: game_state.player,
        entries: result.log,
    });

    // HP capped at max_hp (discarding temp combat bonuses)
    game_state.hp = result.final_player_hp.min(player_stats.max_hp);

    // Gold changes from combat effects only (bosses have no tier-based reward).
    // gold_change can be negative if boss has theft effects. Clamped to not go below 0.
    let new_gold = (game_state.gold as i32)
        .saturating_add(result.gold_change as i32)
        .max(0) as u16;
    game_state.gold = new_gold;

    if result.player_won {
        game_state.boss_fight_ready = false;

        if game_state.week >= 3 {
            emit!(LevelCompleted {
                player: game_state.player,
                level: stage,
                total_moves: game_state.total_moves,
                gold_earned: game_state.gold,
            });
        } else {
            game_state.week = game_state
                .week
                .checked_add(1)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
            game_state.phase = Phase::Day1;
            game_state.moves_remaining = DAY_MOVES;

            game_state.gear_slots = game_state
                .gear_slots
                .checked_add(2)
                .ok_or(GameplayStateError::ArithmeticOverflow)?
                .min(MAX_GEAR_SLOTS);

            expand_gear_slots_cpi(inventory_info, player, player_inventory_program)?;

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: Phase::Day1,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        }

        Ok(true)
    } else {
        game_state.is_dead = true;
        game_state.hp = 0;

        emit!(PlayerDefeated {
            player: game_state.player,
            killed_by: DeathCause::Boss,
            final_hp: result.final_player_hp,
        });

        // Session cleanup is handled by the frontend calling end_session
        // with the main wallet after detecting death.
        Ok(false)
    }
}

fn expand_gear_slots_cpi<'info>(
    inventory: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
) -> Result<()> {
    player_inventory::cpi::expand_gear_slots(CpiContext::new(
        player_inventory_program.clone(),
        player_inventory::cpi::accounts::ExpandGearSlots {
            inventory: inventory.clone(),
            player: player.clone(),
        },
    ))?;

    Ok(())
}

/// CPI call to map_generator::set_tile_floor to persist wall-to-floor conversion.
/// Uses gameplay_authority PDA as signer for authorization.
fn set_tile_floor_cpi<'info>(
    generated_map: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    map_generator_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    x: u8,
    y: u8,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    map_generator::cpi::set_tile_floor(
        CpiContext::new_with_signer(
            map_generator_program.clone(),
            map_generator::cpi::accounts::SetTileFloor {
                generated_map: generated_map.clone(),
                session: session.clone(),
                gameplay_authority: gameplay_authority.clone(),
            },
            signer_seeds,
        ),
        x,
        y,
    )?;

    Ok(())
}

fn select_enemy_step(
    enemy_x: u8,
    enemy_y: u8,
    player_x: u8,
    player_y: u8,
    generated_map: &map_generator::state::GeneratedMap,
    occupied: &[bool],
    map_width: usize,
    player_tile_blocked: bool,
) -> Option<(u8, u8)> {
    let dx = player_x as i16 - enemy_x as i16;
    let dy = player_y as i16 - enemy_y as i16;

    if dx == 0 && dy == 0 {
        return None;
    }

    let step_toward = |pos: u8, delta: i16| -> Option<u8> {
        if delta == 0 {
            return None;
        }
        Some(if delta > 0 {
            pos.saturating_add(1)
        } else {
            pos.saturating_sub(1)
        })
    };

    let x_step = step_toward(enemy_x, dx).map(|x| (x, enemy_y));
    let y_step = step_toward(enemy_y, dy).map(|y| (enemy_x, y));

    let candidates: [Option<(u8, u8)>; 2] = if dx.abs() >= dy.abs() {
        [x_step, y_step]
    } else {
        [y_step, x_step]
    };

    for candidate in candidates.into_iter().flatten() {
        let (cx, cy) = candidate;
        if cx >= generated_map.width || cy >= generated_map.height {
            continue;
        }
        if !generated_map.is_walkable(cx, cy) {
            continue;
        }
        if cx == player_x && cy == player_y {
            if player_tile_blocked {
                continue;
            }
            return Some(candidate);
        }
        let index = (cy as usize) * map_width + (cx as usize);
        if index < occupied.len() && occupied[index] {
            continue;
        }
        return Some(candidate);
    }

    None
}

fn handle_phase_advancement(game_state: &mut GameState) -> Result<()> {
    match game_state.phase.next() {
        Some(next_phase) => {
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
            // Night3 complete - boss fight triggers
            game_state.boss_fight_ready = true;

            emit!(BossFightReady {
                player: game_state.player,
                week: game_state.week,
            });
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

    /// Generated map for seeding enemies
    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_session.key().as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

    /// Enemy instances seeded from generated map
    #[account(
        init,
        payer = player,
        space = 8 + MapEnemies::INIT_SPACE,
        seeds = [MapEnemies::SEED_PREFIX, game_session.key().as_ref()],
        bump
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    #[account(mut)]
    pub player: Signer<'info>,

    /// CHECK: Burner wallet whose pubkey is stored in game_state.burner_wallet
    /// for authorizing gameplay transactions (move, boss fight).
    pub burner_wallet: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
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

/// Context for healing the player, authorized by poi-system CPI.
/// Requires poi_authority PDA from poi-system as signer.
/// Includes inventory for deriving max_hp.
#[derive(Accounts)]
pub struct HealPlayer<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    /// Player inventory for deriving max_hp (PDA derived from session)
    #[account(
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

/// Context for skipping to day, authorized by poi-system CPI.
/// Used by rest POIs (L1 Mole Den, L5 Rest Alcove) to skip night phases.
/// Includes accounts needed for boss fight resolution (Night3 triggers boss fight).
#[derive(Accounts)]
pub struct SkipToDay<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    /// Player inventory for stats calculation and boss fight resolution
    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,

    /// Player for CPI to player_inventory_program (expand gear slots on boss victory)
    /// CHECK: Used for CPI context, validated by player_inventory_program
    pub player: AccountInfo<'info>,

    /// Player inventory program for CPI (expand gear slots on boss victory)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,
}

/// Context for syncing HP from inventory after equipping +HP gear.
/// Can be called directly by the player (no CPI authorization needed).
#[derive(Accounts)]
pub struct SyncHpFromInventory<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player inventory for deriving max_hp (PDA derived from session)
    #[account(
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Player who owns this game state (main wallet)
    pub player: Signer<'info>,
}

/// Context for authorized gold modification via poi-system CPI.
/// Requires poi_authority PDA from poi-system as signer.
#[derive(Accounts)]
pub struct ModifyGoldAuthorized<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Move<'info> {
    #[account(
        mut,
        constraint = game_state.burner_wallet == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession
    )]
    /// CHECK: Validated by game_state.session match.
    pub game_session: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [MapEnemies::SEED_PREFIX, game_state.session.as_ref()],
        bump = map_enemies.bump,
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    #[account(
        mut,
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Gameplay authority PDA for signing CPI calls to map_generator
    /// CHECK: This is a PDA derived from gameplay_state program, validated by seeds
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (expand gear slots on boss victory)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Map generator program for CPI (set tile floor on wall break)
    pub map_generator_program: Program<'info, map_generator::program::MapGenerator>,

    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct TriggerBossFight<'info> {
    #[account(
        mut,
        constraint = game_state.burner_wallet == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession
    )]
    /// CHECK: Validated by game_state.session match.
    pub game_session: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [MapEnemies::SEED_PREFIX, game_state.session.as_ref()],
        bump = map_enemies.bump,
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Player inventory program for CPI (expand gear slots on boss victory)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

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
pub struct GameStateClosed {
    pub player: Pubkey,
    pub total_moves: u32,
    pub final_phase: Phase,
    pub final_week: u8,
}

/// Emitted when player is healed via authorized CPI from poi-system
#[event]
pub struct PlayerHealed {
    pub player: Pubkey,
    pub old_hp: i16,
    pub new_hp: i16,
    pub amount: u16,
    pub max_hp: i16,
}

/// Emitted when HP is synced from inventory after equipping +HP gear
#[event]
pub struct HpSynced {
    pub player: Pubkey,
    pub old_hp: i16,
    pub new_hp: i16,
    pub hp_bonus: i16,
    pub max_hp: i16,
}

/// Emitted when gold is modified via authorized CPI from poi-system
#[event]
pub struct GoldModifiedAuthorized {
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

/// Detailed combat log for turn-by-turn visualization.
/// Contains a serialized vector of CombatLogEntry for replay.
/// Note: Solana logs have ~30KB limit; this compact format allows ~300-400 actions per battle.
#[event]
pub struct CombatLog {
    pub player: Pubkey,
    /// Serialized Vec<CombatLogEntry> - each entry is ~5 bytes
    pub entries: Vec<CombatLogEntry>,
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

#[cfg(test)]
mod hp_logic_tests {
    use super::*;

    fn make_base_stats() -> PlayerStats {
        PlayerStats {
            max_hp: 10,
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 1,
        }
    }

    #[test]
    fn test_hp_capping_logic() {
        // Test that combat HP is capped at max_hp from derived stats.
        // BattleStart Heal bonuses are included in stats.max_hp via calculate_stats(),
        // not as separate temporary bonuses in build_player_combatant().
        let stats = PlayerStats {
            max_hp: 15, // Already includes +5 from BattleStart Heal item
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 1,
        };

        // Player at full HP
        let current_hp: i16 = 15;
        let effects = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 15, "Combat HP should match current HP");
        assert_eq!(input.max_hp, 15, "Combat max_hp should match derived stats");

        // Simulate combat: lose 3 HP
        let final_combat_hp: i16 = 12;

        // Post-combat capping
        let new_persistent_hp = final_combat_hp.min(stats.max_hp);
        assert_eq!(
            new_persistent_hp, 12,
            "HP should persist as 12 (below max 15)"
        );
    }

    #[test]
    fn test_hp_damage_persistence() {
        // Test that damage persists correctly after combat.
        let stats = PlayerStats {
            max_hp: 15, // Includes item bonuses from calculate_stats()
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 1,
        };

        let current_hp: i16 = 15;
        let effects = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 15);
        assert_eq!(input.max_hp, 15);

        // Player loses 7 HP, ending at 8
        let final_combat_hp: i16 = 8;

        let new_persistent_hp = final_combat_hp.min(stats.max_hp);
        assert_eq!(
            new_persistent_hp, 8,
            "HP should persist as 8 (lower than max 15)"
        );
    }

    #[test]
    fn test_mid_combat_healing() {
        // Scenario 3: 10 HP. Lose 3 (7). Heal 2 (9). End -> 9.
        // Note: Mid-combat healing affects the final_combat_hp result directly.
        // We simulate the result of combat being 9.
        let current_hp: i16 = 10;
        let stats = make_base_stats();

        let effects = vec![]; // No battle start bonus

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 10);
        assert_eq!(input.max_hp, 10);

        // Combat happens: 10 -> 7 -> 9
        let final_combat_hp: i16 = 9;

        let new_persistent_hp = final_combat_hp.min(stats.max_hp);
        assert_eq!(new_persistent_hp, 9, "HP should be 9");
    }

    #[test]
    fn test_derived_stats_in_combat() {
        // Test that derived stats (from inventory) are used correctly in combat
        let current_hp: i16 = 8;
        let stats = PlayerStats {
            max_hp: 15, // Increased from items
            atk: 5,     // From weapon
            arm: 3,     // From gear
            spd: 2,     // From gear
            dig: 3,     // From items
        };

        let effects = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 8);
        assert_eq!(input.max_hp, 15);
        assert_eq!(input.atk, 5);
        assert_eq!(input.arm, 3);
        assert_eq!(input.spd, 2);
        assert_eq!(input.dig, 3);
    }

    #[test]
    fn test_battlestart_heal_not_double_counted() {
        // Regression test: BattleStart Heal bonuses are included in stats.max_hp
        // via calculate_stats(). They should NOT be added again in build_player_combatant().
        //
        // If this test fails, it means BattleStart Heal is being double-counted:
        // - Once in calculate_stats() -> stats.max_hp
        // - Again in build_player_combatant() -> combat_max_hp
        //
        // The fix ensures build_player_combatant() uses stats.max_hp directly.

        use combat_system::{EffectType, TriggerType};

        // Simulate: stats.max_hp = 15 (base 10 + 5 from BattleStart Heal item)
        let stats = PlayerStats {
            max_hp: 15,
            atk: 0,
            arm: 0,
            spd: 0,
            dig: 1,
        };

        // The same Heal effect that calculate_stats() already processed
        let effects = vec![ItemEffect {
            effect_type: EffectType::Heal,
            trigger: TriggerType::BattleStart,
            value: 5,
            once_per_turn: false,
        }];

        let current_hp: i16 = 15;
        let input = build_player_combatant(current_hp, &stats, &effects);

        // CORRECT: combat_max_hp = 15 (from stats, Heal NOT added again)
        // BUG: combat_max_hp = 20 (15 + 5, Heal double-counted)
        assert_eq!(
            input.max_hp, 15,
            "BattleStart Heal should NOT be double-counted"
        );
        assert_eq!(input.hp, 15, "Combat HP should not exceed derived max_hp");
    }

    #[test]
    fn test_coin_slug_armor_from_gold() {
        // Coin Slug: Battle Start: gain Armor equal to floor(player Gold/10) (cap 3)
        // This tests the preprocess_enemy_effects function.

        use field_enemies::archetypes::ids::COIN_SLUG;

        // 0 gold = 0 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 0);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].value, 0);

        // 9 gold = 0 armor (floor(9/10) = 0)
        let effects = preprocess_enemy_effects(COIN_SLUG, 9);
        assert_eq!(effects[0].value, 0);

        // 10 gold = 1 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 10);
        assert_eq!(effects[0].value, 1);

        // 25 gold = 2 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 25);
        assert_eq!(effects[0].value, 2);

        // 30 gold = 3 armor (cap)
        let effects = preprocess_enemy_effects(COIN_SLUG, 30);
        assert_eq!(effects[0].value, 3);

        // 100 gold = 3 armor (capped at 3)
        let effects = preprocess_enemy_effects(COIN_SLUG, 100);
        assert_eq!(effects[0].value, 3, "Armor should be capped at 3");

        // Non-Coin Slug enemies should not be affected
        let effects = preprocess_enemy_effects(0, 100); // Tunnel Rat
        assert!(!effects
            .iter()
            .any(|e| { matches!(e.effect_type, EffectType::GainArmor) && e.value == 3 }));
    }
}
