/// Maximum campaign level (1-40 = 40 levels)
pub const MAX_LEVEL: u8 = 40;

/// Number of levels in the campaign
pub const LEVEL_COUNT: usize = 40;

/// Map width in tiles
pub const MAP_WIDTH: u8 = 50;

/// Map height in tiles
pub const MAP_HEIGHT: u8 = 50;

/// Default seeds for campaign levels (level number as seed)
pub const DEFAULT_SEEDS: [u64; 40] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
];

// ============================================================================
// Maze Generation Constants
// ============================================================================

/// Spacing between cells in the maze grid (corridor length)
pub const CELL_SPACING: u8 = 4;

/// Extra connections percentage (15% of cells = 21 connections)
/// Calculated as integer math: CELL_GRID_SIZE * 15 / 100
pub const EXTRA_CONNECTION_PERCENT: usize = 15;

/// Width of the cell grid for maze generation ((50-1) / 4 = 12)
pub const CELL_GRID_WIDTH: u8 = (MAP_WIDTH - 1) / CELL_SPACING;

/// Height of the cell grid for maze generation ((50-1) / 4 = 12)
pub const CELL_GRID_HEIGHT: u8 = (MAP_HEIGHT - 1) / CELL_SPACING;

/// Total cells in the grid (12 * 12 = 144)
pub const CELL_GRID_SIZE: usize = (CELL_GRID_WIDTH as usize) * (CELL_GRID_HEIGHT as usize);

/// Size of packed tiles array in bytes (ceil(2500/8) = 313)
pub const PACKED_TILES_SIZE: usize = 313;

/// Maximum number of enemies that can be spawned on a map
pub const MAX_ENEMIES: usize = 48;

/// Maximum number of POIs that can be spawned on a map
pub const MAX_POIS: usize = 64;

/// Maximum number of discovered POIs tracked in SessionDiscovery
pub const MAX_DISCOVERED_POIS: usize = 64;

/// Total tiles in the map (50 * 50 = 2500)
pub const TOTAL_TILES: usize = (MAP_WIDTH as usize) * (MAP_HEIGHT as usize);

/// Size of serialized GauntletEchoSnapshot in bytes
pub const ECHO_DATA_SIZE: usize = 179;

// =============================================================================
// Cross-Program Raw Byte Offsets
//
// These offsets are used to read fields from accounts owned by other programs
// without taking a crate dependency. Keep in sync with the canonical structs:
//   - GameSession: session-manager/src/state.rs (PLAYER_OFFSET, SESSION_SIGNER_OFFSET)
//   - GameState:   gameplay-state/src/state.rs
//   - SessionDiscovery: this crate's state.rs
// =============================================================================

/// GameSession.player byte offset (8-byte discriminator + 0)
pub const SESSION_PLAYER_OFFSET: usize = 8;

/// GameSession.session_signer byte offset
/// Layout: 8 (disc) + 32 (player) + 8 (session_id) + 1 (campaign_level) +
/// 8 (started_at) + 8 (last_activity) + 1 (is_delegated) + 1 (bump) +
/// 10 (active_item_pool) + 1 (active_relic_count) + 640 (active_relics) = 718
pub const SESSION_SESSION_SIGNER_OFFSET: usize = 718;

/// GameState.player byte offset (8-byte discriminator + 0)
pub const GAME_STATE_PLAYER_OFFSET: usize = 8;

/// GameState.session_signer byte offset (8 + 32 = 40)
pub const GAME_STATE_SESSION_SIGNER_OFFSET: usize = 40;

/// GameState.session byte offset (8 + 32 + 32 = 72)
pub const GAME_STATE_SESSION_OFFSET: usize = 72;

/// SessionDiscovery.session byte offset (8-byte discriminator + 0)
pub const SESSION_DISCOVERY_SESSION_OFFSET: usize = 8;
