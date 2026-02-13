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
pub fn calculate_move_cost(is_wall: bool, dig_stat: i16) -> u8 {
    if is_wall {
        let cost = (BASE_DIG_COST as i16 - dig_stat).max(MIN_DIG_COST as i16);
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

/// Check if boss fight should trigger (end of week, moves exhausted, night 3 phase)
pub fn should_trigger_boss(phase: &Phase, moves_remaining: u8) -> bool {
    moves_remaining == 0 && phase.is_night3()
}

/// Returns true when night enemy movement should run for this move action.
/// If the player is moving onto an occupied enemy tile, direct combat takes precedence.
pub fn should_process_night_enemy_movement(phase: &Phase, target_has_enemy: bool) -> bool {
    phase.is_night() && !target_has_enemy
}

/// Returns true when a target-tile enemy combat should be resolved after movement.
/// Ensures at most one combat is resolved during a single move transaction.
pub fn should_process_target_enemy_combat(
    combat_already_triggered: bool,
    is_last_move_of_week: bool,
    target_enemy_exists: bool,
) -> bool {
    !combat_already_triggered && !is_last_move_of_week && target_enemy_exists
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

/// Get boss combat input for the current stage and week.
/// Returns scaled boss stats ready for combat.
pub fn get_boss_for_combat(stage: u8, week: u8) -> Result<combat_system::state::CombatantInput> {
    let boss_week = to_boss_week(week)?;
    let boss = boss_system::select_boss(stage, boss_week);
    let scaled = boss_system::scale_boss(boss, stage, boss_week);
    Ok(boss_system::scaling::to_combatant_input(&scaled))
}

/// Get boss ID (12 bytes) for event emission
pub fn get_boss_id(stage: u8, week: u8) -> Result<[u8; 12]> {
    let boss_week = to_boss_week(week)?;
    let boss = boss_system::select_boss(stage, boss_week);
    Ok(boss.id)
}

/// Get duel week 1/2 boss combat input based on map seed.
/// Week 3 has no weekly boss in duel mode and returns InvalidWeek.
pub fn get_duel_boss_for_combat(seed: u64, week: u8) -> Result<combat_system::state::CombatantInput> {
    let boss_week = to_boss_week(week)?;
    let boss = boss_system::select_duel_week_boss(seed, boss_week).ok_or(GameplayStateError::InvalidWeek)?;
    let scaled = boss_system::scale_boss(boss, 20, boss_week);
    Ok(boss_system::scaling::to_combatant_input(&scaled))
}

/// Get duel week 1/2 boss ID (12 bytes) based on map seed.
/// Week 3 has no weekly boss in duel mode and returns InvalidWeek.
pub fn get_duel_boss_id(seed: u64, week: u8) -> Result<[u8; 12]> {
    let boss_week = to_boss_week(week)?;
    let boss = boss_system::select_duel_week_boss(seed, boss_week).ok_or(GameplayStateError::InvalidWeek)?;
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
    fn test_should_process_night_enemy_movement() {
        assert!(should_process_night_enemy_movement(&Phase::Night1, false));
        assert!(!should_process_night_enemy_movement(&Phase::Night2, true));
        assert!(!should_process_night_enemy_movement(&Phase::Day1, false));
    }

    #[test]
    fn test_should_process_target_enemy_combat() {
        assert!(should_process_target_enemy_combat(false, false, true));
        assert!(!should_process_target_enemy_combat(true, false, true));
        assert!(!should_process_target_enemy_combat(false, true, true));
        assert!(!should_process_target_enemy_combat(false, false, false));
    }
}
