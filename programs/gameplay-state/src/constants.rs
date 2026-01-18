/// Moves allowed per day phase
pub const DAY_MOVES: u8 = 50;

/// Moves allowed per night phase
pub const NIGHT_MOVES: u8 = 30;

/// Default starting HP
pub const DEFAULT_HP: i8 = 10;

/// Default maximum HP
pub const DEFAULT_MAX_HP: u8 = 10;

/// Default attack stat
pub const DEFAULT_ATK: i8 = 1;

/// Default armor stat
pub const DEFAULT_ARM: i8 = 0;

/// Default speed stat
pub const DEFAULT_SPD: i8 = 0;

/// Default dig stat
pub const DEFAULT_DIG: i8 = 1;

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
