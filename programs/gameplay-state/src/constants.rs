/// Moves allowed per day phase
pub const DAY_MOVES: u8 = 50;

/// Moves allowed per night phase
pub const NIGHT_MOVES: u8 = 30;

// =============================================================================
// Base Stats (derived from inventory at runtime)
// =============================================================================

/// Base HP before item bonuses
pub const BASE_HP: i16 = 10;

/// Base attack stat before item bonuses
pub const BASE_ATK: i16 = 0;

/// Base armor stat before item bonuses
pub const BASE_ARM: i16 = 0;

/// Base speed stat before item bonuses
pub const BASE_SPD: i16 = 0;

/// Base dig stat before item bonuses
pub const BASE_DIG: i16 = 1;

/// Initial gear slots at game start
pub const INITIAL_GEAR_SLOTS: u8 = 4;

/// Maximum gear slots (capped)
pub const MAX_GEAR_SLOTS: u8 = 8;

/// Base cost for digging through walls (before DIG stat reduction)
pub const BASE_DIG_COST: u8 = 6;

/// Minimum cost for digging (even with high DIG stat)
pub const MIN_DIG_COST: u8 = 2;

/// Cost for moving to a floor tile
pub const FLOOR_MOVE_COST: u8 = 1;

/// PDA seed prefix for GameState accounts
pub const GAME_STATE_SEED: &[u8] = b"game_state";
