use crate::constants::{BASE_DIG_COST, FLOOR_MOVE_COST, MIN_DIG_COST};
use crate::errors::GameplayStateError;
use crate::state::{GameState, Phase};
use anchor_lang::prelude::*;
use map_generator::constants::PACKED_TILES_SIZE;

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

/// VRF-based duel boss selection.
/// Uses VRF randomness with DUEL_BOSS domain for verifiable boss selection.
pub fn get_duel_boss_for_combat_vrf(
    vrf: (&[u8; 32], u64),
    week: u8,
) -> Result<combat_system::state::CombatantInput> {
    let boss_week = to_boss_week(week)?;
    let (randomness, nonce) = vrf;
    let mut rng = vrf_rng::GameRng::from_vrf(randomness, nonce, vrf_rng::domains::DUEL_BOSS);
    let boss = boss_system::select_duel_week_boss_vrf(&mut rng, boss_week)
        .ok_or(GameplayStateError::InvalidWeek)?;
    let scaled = boss_system::scale_boss(boss, 20, boss_week);
    Ok(boss_system::scaling::to_combatant_input(&scaled))
}

/// VRF-based duel boss ID lookup.
pub fn get_duel_boss_id_vrf(vrf: (&[u8; 32], u64), week: u8) -> Result<[u8; 12]> {
    let boss_week = to_boss_week(week)?;
    let (randomness, nonce) = vrf;
    let mut rng = vrf_rng::GameRng::from_vrf(randomness, nonce, vrf_rng::domains::DUEL_BOSS);
    let boss = boss_system::select_duel_week_boss_vrf(&mut rng, boss_week)
        .ok_or(GameplayStateError::InvalidWeek)?;
    Ok(boss.id)
}

/// Derive deterministic RNG from map seed for boss selection.
/// Both matched duel players share the same map seed, so they get the same bosses.
fn duel_boss_rng_from_seed(map_seed: u64, week: u8) -> vrf_rng::GameRng {
    let seed_bytes = map_seed.to_le_bytes();
    let mut randomness = [0u8; 32];
    randomness[..8].copy_from_slice(&seed_bytes);
    randomness[8..16].copy_from_slice(&seed_bytes);
    randomness[16..24].copy_from_slice(&seed_bytes);
    randomness[24..32].copy_from_slice(&seed_bytes);
    vrf_rng::GameRng::from_vrf(&randomness, week as u64, vrf_rng::domains::DUEL_BOSS)
}

/// Select boss ID from the shared map seed so both duel players get the same boss.
pub fn get_duel_boss_id_from_seed(map_seed: u64, week: u8) -> Result<[u8; 12]> {
    let boss_week = to_boss_week(week)?;
    let mut rng = duel_boss_rng_from_seed(map_seed, week);
    let boss = boss_system::select_duel_week_boss_vrf(&mut rng, boss_week)
        .ok_or(GameplayStateError::InvalidWeek)?;
    Ok(boss.id)
}

/// Select boss combatant from the shared map seed for combat resolution.
pub fn get_duel_boss_for_combat_from_seed(
    map_seed: u64,
    week: u8,
) -> Result<combat_system::state::CombatantInput> {
    let boss_week = to_boss_week(week)?;
    let mut rng = duel_boss_rng_from_seed(map_seed, week);
    let boss = boss_system::select_duel_week_boss_vrf(&mut rng, boss_week)
        .ok_or(GameplayStateError::InvalidWeek)?;
    let scaled = boss_system::scale_boss(boss, 20, boss_week);
    Ok(boss_system::scaling::to_combatant_input(&scaled))
}

/// Compute all enemies on discovered tiles for SessionDiscovery sync.
pub fn compute_visible_enemies(
    game_state: &GameState,
    discovered_tiles: &[u8; PACKED_TILES_SIZE],
    map_width: u8,
) -> Vec<map_generator::state::DiscoveredEnemy> {
    let mut result = Vec::new();
    for (idx, enemy) in game_state.enemies.iter().enumerate() {
        if enemy.defeated {
            continue;
        }
        let tile_index = (enemy.y as usize) * (map_width as usize) + (enemy.x as usize);
        let byte_idx = tile_index / 8;
        let bit_idx = tile_index % 8;
        if byte_idx < PACKED_TILES_SIZE && (discovered_tiles[byte_idx] >> bit_idx) & 1 == 1 {
            result.push(map_generator::state::DiscoveredEnemy {
                archetype_id: enemy.archetype_id,
                tier: enemy.tier,
                x: enemy.x,
                y: enemy.y,
                defeated: if enemy.defeated { 1 } else { 0 },
                map_enemies_index: idx as u8,
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EnemyInstance;
    use anchor_lang::prelude::Pubkey;

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

    #[test]
    fn test_compute_visible_enemies_excludes_defeated() {
        use crate::state::Phase;

        let mut discovered_tiles = [0u8; PACKED_TILES_SIZE];
        discovered_tiles[0] = 0b0000_0011;

        let game_state = GameState {
            player: Pubkey::default(),
            session_signer: Pubkey::default(),
            session: Pubkey::default(),
            position_x: 0,
            position_y: 0,
            map_width: 50,
            map_height: 50,
            hp: 100,
            gear_slots: 4,
            week: 1,
            phase: Phase::Day1,
            moves_remaining: 50,
            total_moves: 0,
            boss_fight_ready: false,
            gold: 0,
            bump: 0,
            campaign_level: 1,
            run_mode: crate::state::RunMode::Campaign,
            max_weeks: 3,
            is_dead: false,
            completed: false,
            gauntlet_epoch_id: 0,
            gauntlet_points_earned: 0,
            gauntlet_defender_credit: None,
            gauntlet_highest_week_won: 0,
            gauntlet_settled: false,
            duel_map_seed: 0,
            enemies: vec![
                EnemyInstance {
                    archetype_id: 0,
                    tier: 0,
                    x: 0,
                    y: 0,
                    defeated: false,
                },
                EnemyInstance {
                    archetype_id: 1,
                    tier: 1,
                    x: 1,
                    y: 0,
                    defeated: true,
                },
            ],
            enemy_count: 2,
            enemies_defeated: 0,
        };

        let visible = compute_visible_enemies(&game_state, &discovered_tiles, 50);
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].archetype_id, 0);
        assert_eq!(visible[0].map_enemies_index, 0);
    }
}
