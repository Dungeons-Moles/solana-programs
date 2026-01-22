use crate::constants::{BASE_DIG_COST, FLOOR_MOVE_COST, MIN_DIG_COST};
use crate::errors::GameplayStateError;
use crate::state::Phase;
use anchor_lang::prelude::*;

/// Calculates Chebyshev distance (max of x/y difference) between two points.
/// Used for enemy detection radius (enemies within 3 tiles move during night).
pub fn chebyshev_distance(x1: u8, y1: u8, x2: u8, y2: u8) -> u8 {
    let dx = (x1 as i16 - x2 as i16).unsigned_abs() as u8;
    let dy = (y1 as i16 - y2 as i16).unsigned_abs() as u8;
    dx.max(dy)
}

/// Calculates the move cost for moving to a tile.
/// Floor tiles cost 1 move, wall tiles cost max(2, 6 - DIG).
pub fn calculate_move_cost(is_wall: bool, dig_stat: i8) -> u8 {
    if is_wall {
        let cost = (BASE_DIG_COST as i16 - dig_stat as i16).max(MIN_DIG_COST as i16);
        cost as u8
    } else {
        FLOOR_MOVE_COST
    }
}

/// Validates that the target position is adjacent (Manhattan distance = 1) to current position.
pub fn is_adjacent(from_x: u8, from_y: u8, to_x: u8, to_y: u8) -> bool {
    let dx = (to_x as i16 - from_x as i16).abs();
    let dy = (to_y as i16 - from_y as i16).abs();
    dx + dy == 1
}

/// Validates that the target position is within map bounds.
pub fn is_within_bounds(x: u8, y: u8, map_width: u8, map_height: u8) -> bool {
    x < map_width && y < map_height
}

/// Combat statistics for inline combat resolution
#[derive(Clone, Copy, Debug)]
pub struct InlineCombatStats {
    pub hp: i16,
    pub max_hp: u16,
    pub atk: i16,
    pub arm: i16,
    pub spd: i16,
    pub strikes: u8,
}

/// Result of inline combat resolution
#[derive(Clone, Copy, Debug)]
pub struct CombatResult {
    pub player_won: bool,
    pub final_player_hp: i16,
    pub final_enemy_hp: i16,
    pub turns_taken: u8,
    pub gold_earned: u16,
}

/// Resolves combat inline without CPI to avoid compute overhead.
/// Uses deterministic turn-based combat similar to combat-system program.
///
/// Combat rules:
/// 1. Higher SPD attacks first (player wins ties)
/// 2. Each combatant deals ATK - opponent_ARM damage (minimum 1)
/// 3. Combat continues until one combatant HP <= 0
/// 4. Sudden death: after turn 10, both gain +2 ATK per turn
pub fn resolve_combat_inline(
    player_stats: &mut InlineCombatStats,
    enemy_stats: &mut InlineCombatStats,
) -> CombatResult {
    let mut turn: u8 = 1;
    const MAX_TURNS: u8 = 50;
    const SUDDEN_DEATH_TURN: u8 = 10;

    let mut player_atk = player_stats.atk;
    let mut enemy_atk = enemy_stats.atk;

    while player_stats.hp > 0 && enemy_stats.hp > 0 && turn <= MAX_TURNS {
        // Apply sudden death bonus after turn 10
        if turn > SUDDEN_DEATH_TURN {
            let bonus = ((turn - SUDDEN_DEATH_TURN) * 2) as i16;
            player_atk = player_stats.atk + bonus;
            enemy_atk = enemy_stats.atk + bonus;
        }

        // Determine turn order - player wins ties
        let player_first = player_stats.spd >= enemy_stats.spd;

        if player_first {
            // Player attacks first
            let damage = (player_atk - enemy_stats.arm).max(1);
            enemy_stats.hp -= damage;

            // Enemy attacks if still alive
            if enemy_stats.hp > 0 {
                let damage = (enemy_atk - player_stats.arm).max(1);
                player_stats.hp -= damage;
            }
        } else {
            // Enemy attacks first
            let damage = (enemy_atk - player_stats.arm).max(1);
            player_stats.hp -= damage;

            // Player attacks if still alive
            if player_stats.hp > 0 {
                let damage = (player_atk - enemy_stats.arm).max(1);
                enemy_stats.hp -= damage;
            }
        }

        turn += 1;
    }

    let player_won = player_stats.hp > 0;

    CombatResult {
        player_won,
        final_player_hp: player_stats.hp,
        final_enemy_hp: enemy_stats.hp,
        turns_taken: turn.saturating_sub(1),
        gold_earned: 0, // Calculated separately based on enemy tier
    }
}

/// Moves an enemy one tile toward the player using simple pathfinding.
/// Returns the new (x, y) position for the enemy.
pub fn move_toward(enemy_x: u8, enemy_y: u8, player_x: u8, player_y: u8) -> (u8, u8) {
    let mut new_x = enemy_x;
    let mut new_y = enemy_y;

    // Move in the direction that reduces distance most
    // Prioritize X movement, then Y
    if enemy_x < player_x {
        new_x = enemy_x.saturating_add(1);
    } else if enemy_x > player_x {
        new_x = enemy_x.saturating_sub(1);
    }

    if enemy_y < player_y {
        new_y = enemy_y.saturating_add(1);
    } else if enemy_y > player_y {
        new_y = enemy_y.saturating_sub(1);
    }

    // If both X and Y would move, only move in the axis with greater distance
    // to avoid diagonal movement
    let dx = (enemy_x as i16 - player_x as i16).abs();
    let dy = (enemy_y as i16 - player_y as i16).abs();

    if dx > 0 && dy > 0 {
        if dx >= dy {
            // Move in X direction only
            new_y = enemy_y;
        } else {
            // Move in Y direction only
            new_x = enemy_x;
        }
    }

    (new_x, new_y)
}

/// Check if boss fight should trigger (end of week, moves exhausted, night 3 phase)
pub fn should_trigger_boss(phase: &Phase, moves_remaining: u8) -> bool {
    moves_remaining == 0 && phase.is_night3()
}

/// Result of boss combat resolution
#[derive(Clone, Copy, Debug)]
pub struct BossCombatResult {
    pub player_won: bool,
    pub final_player_hp: i16,
    pub final_boss_hp: i16,
    pub turns_taken: u8,
    /// true if this was the Week 3 boss (final boss of level)
    pub was_final_boss: bool,
}

/// Resolve boss combat inline.
/// Similar to resolve_combat_inline but adapted for boss encounters.
pub fn resolve_boss_combat_inline(
    player_stats: &mut InlineCombatStats,
    boss_stats: &mut InlineCombatStats,
) -> CombatResult {
    // Boss combat uses the same resolution as normal combat
    resolve_combat_inline(player_stats, boss_stats)
}

/// Convert boss week enum (1, 2, 3) to boss_system::Week
/// Returns error for invalid week values instead of silently defaulting
pub fn to_boss_week(week: u8) -> Result<boss_system::Week> {
    match week {
        1 => Ok(boss_system::Week::One),
        2 => Ok(boss_system::Week::Two),
        3 => Ok(boss_system::Week::Three),
        _ => Err(GameplayStateError::InvalidWeek.into()),
    }
}

/// Get boss stats for the current stage and week using boss_system's inline functions.
/// Returns scaled boss stats ready for combat.
pub fn get_boss_for_combat(stage: u8, week: u8) -> Result<InlineCombatStats> {
    let boss_week = to_boss_week(week)?;
    let boss = boss_system::select_boss(stage, boss_week);
    let scaled = boss_system::scale_boss(boss, stage, boss_week);

    Ok(InlineCombatStats {
        hp: scaled.hp as i16,
        max_hp: scaled.hp,
        atk: scaled.atk as i16,
        arm: scaled.arm as i16,
        spd: scaled.spd as i16,
        strikes: scaled.strikes,
    })
}

/// Get boss ID (12 bytes) for event emission
pub fn get_boss_id(stage: u8, week: u8) -> Result<[u8; 12]> {
    let boss_week = to_boss_week(week)?;
    let boss = boss_system::select_boss(stage, boss_week);
    Ok(boss.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chebyshev_distance_same_point() {
        assert_eq!(chebyshev_distance(5, 5, 5, 5), 0);
    }

    #[test]
    fn test_chebyshev_distance_horizontal() {
        assert_eq!(chebyshev_distance(0, 0, 3, 0), 3);
        assert_eq!(chebyshev_distance(5, 2, 2, 2), 3);
    }

    #[test]
    fn test_chebyshev_distance_vertical() {
        assert_eq!(chebyshev_distance(0, 0, 0, 4), 4);
        assert_eq!(chebyshev_distance(3, 7, 3, 2), 5);
    }

    #[test]
    fn test_chebyshev_distance_diagonal() {
        // Chebyshev distance for diagonal is max(dx, dy)
        assert_eq!(chebyshev_distance(0, 0, 3, 3), 3);
        assert_eq!(chebyshev_distance(0, 0, 2, 5), 5);
        assert_eq!(chebyshev_distance(5, 5, 2, 8), 3);
    }

    #[test]
    fn test_calculate_move_cost_floor() {
        assert_eq!(calculate_move_cost(false, 0), 1);
        assert_eq!(calculate_move_cost(false, 5), 1);
    }

    #[test]
    fn test_calculate_move_cost_wall() {
        // Wall cost = max(2, 6 - DIG)
        assert_eq!(calculate_move_cost(true, 0), 6); // 6 - 0 = 6
        assert_eq!(calculate_move_cost(true, 1), 5); // 6 - 1 = 5
        assert_eq!(calculate_move_cost(true, 2), 4); // 6 - 2 = 4
        assert_eq!(calculate_move_cost(true, 3), 3); // 6 - 3 = 3
        assert_eq!(calculate_move_cost(true, 4), 2); // 6 - 4 = 2
        assert_eq!(calculate_move_cost(true, 5), 2); // min is 2
        assert_eq!(calculate_move_cost(true, 10), 2); // still min 2
    }

    #[test]
    fn test_is_adjacent() {
        // Adjacent tiles
        assert!(is_adjacent(5, 5, 5, 6)); // up
        assert!(is_adjacent(5, 5, 5, 4)); // down
        assert!(is_adjacent(5, 5, 6, 5)); // right
        assert!(is_adjacent(5, 5, 4, 5)); // left

        // Not adjacent
        assert!(!is_adjacent(5, 5, 5, 5)); // same
        assert!(!is_adjacent(5, 5, 6, 6)); // diagonal
        assert!(!is_adjacent(5, 5, 5, 7)); // too far
        assert!(!is_adjacent(5, 5, 7, 5)); // too far
    }

    #[test]
    fn test_is_within_bounds() {
        assert!(is_within_bounds(0, 0, 10, 10));
        assert!(is_within_bounds(9, 9, 10, 10));
        assert!(!is_within_bounds(10, 0, 10, 10));
        assert!(!is_within_bounds(0, 10, 10, 10));
        assert!(!is_within_bounds(10, 10, 10, 10));
    }

    #[test]
    fn test_resolve_combat_inline_player_wins() {
        let mut player = InlineCombatStats {
            hp: 10,
            max_hp: 10,
            atk: 5,
            arm: 2,
            spd: 3,
            strikes: 1,
        };
        let mut enemy = InlineCombatStats {
            hp: 5,
            max_hp: 5,
            atk: 2,
            arm: 0,
            spd: 1,
            strikes: 1,
        };

        let result = resolve_combat_inline(&mut player, &mut enemy);

        assert!(result.player_won);
        assert!(result.final_player_hp > 0);
        assert!(result.final_enemy_hp <= 0);
    }

    #[test]
    fn test_resolve_combat_inline_enemy_wins() {
        let mut player = InlineCombatStats {
            hp: 3,
            max_hp: 3,
            atk: 1,
            arm: 0,
            spd: 1,
            strikes: 1,
        };
        let mut enemy = InlineCombatStats {
            hp: 20,
            max_hp: 20,
            atk: 5,
            arm: 2,
            spd: 5, // Higher speed, attacks first
            strikes: 1,
        };

        let result = resolve_combat_inline(&mut player, &mut enemy);

        assert!(!result.player_won);
        assert!(result.final_player_hp <= 0);
        assert!(result.final_enemy_hp > 0);
    }

    #[test]
    fn test_resolve_combat_inline_speed_tie_player_wins() {
        // When speed is equal, player should attack first
        let mut player = InlineCombatStats {
            hp: 5,
            max_hp: 5,
            atk: 10,
            arm: 0,
            spd: 3,
            strikes: 1,
        };
        let mut enemy = InlineCombatStats {
            hp: 5,
            max_hp: 5,
            atk: 10,
            arm: 0,
            spd: 3, // Same speed
            strikes: 1,
        };

        let result = resolve_combat_inline(&mut player, &mut enemy);

        // Player attacks first due to tie-breaker, so player wins
        assert!(result.player_won);
    }

    #[test]
    fn test_move_toward_horizontal() {
        // Enemy to the left of player
        let (x, y) = move_toward(2, 5, 5, 5);
        assert_eq!(x, 3);
        assert_eq!(y, 5);

        // Enemy to the right of player
        let (x, y) = move_toward(8, 5, 5, 5);
        assert_eq!(x, 7);
        assert_eq!(y, 5);
    }

    #[test]
    fn test_move_toward_vertical() {
        // Enemy above player
        let (x, y) = move_toward(5, 2, 5, 5);
        assert_eq!(x, 5);
        assert_eq!(y, 3);

        // Enemy below player
        let (x, y) = move_toward(5, 8, 5, 5);
        assert_eq!(x, 5);
        assert_eq!(y, 7);
    }

    #[test]
    fn test_move_toward_diagonal_x_priority() {
        // When X distance >= Y distance, move in X direction
        let (x, y) = move_toward(2, 3, 5, 5);
        // dx = 3, dy = 2, should move in X
        assert_eq!(x, 3);
        assert_eq!(y, 3);
    }

    #[test]
    fn test_move_toward_diagonal_y_priority() {
        // When Y distance > X distance, move in Y direction
        let (x, y) = move_toward(4, 2, 5, 6);
        // dx = 1, dy = 4, should move in Y
        assert_eq!(x, 4);
        assert_eq!(y, 3);
    }

    #[test]
    fn test_move_toward_same_position() {
        // Enemy at same position as player (shouldn't happen, but handle it)
        let (x, y) = move_toward(5, 5, 5, 5);
        assert_eq!(x, 5);
        assert_eq!(y, 5);
    }
}
