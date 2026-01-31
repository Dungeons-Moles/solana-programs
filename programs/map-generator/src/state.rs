use crate::constants::{LEVEL_COUNT, MAX_ENEMIES, MAX_POIS, PACKED_TILES_SIZE};
use anchor_lang::prelude::*;

/// Configuration for map generation, storing seed values for each level.
#[account]
#[derive(InitSpace)]
pub struct MapConfig {
    /// Authority that can update seed mappings
    pub admin: Pubkey,
    /// Seed values for campaign levels 1-40 (index = level - 1)
    #[max_len(40)]
    pub seeds: [u64; LEVEL_COUNT],
    /// Config version for migrations
    pub version: u8,
    /// PDA bump seed
    pub bump: u8,
}

impl MapConfig {
    /// PDA seed prefix for map config
    pub const SEED_PREFIX: &'static [u8] = b"map_config";
}

/// Represents a single seed update for batch operations
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct SeedUpdate {
    /// Level to update (1-40)
    pub level: u8,
    /// New seed value
    pub seed: u64,
}

/// Tile types for map generation (off-chain structure)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TileType {
    Floor = 0,
    Wall = 1,
}

/// Position on the map grid
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}

// ============================================================================
// Generated Map Structures
// ============================================================================

/// Initial enemy spawn position (4 bytes)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct EnemySpawn {
    /// Enemy archetype ID (0-11)
    pub archetype_id: u8,
    /// Enemy tier: 0=T1, 1=T2, 2=T3
    pub tier: u8,
    /// X position on map
    pub x: u8,
    /// Y position on map
    pub y: u8,
}

/// Initial POI spawn position (4 bytes)
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct PoiSpawn {
    /// POI definition ID (1-14)
    pub poi_type: u8,
    /// True if POI has been used
    pub is_used: bool,
    /// X position on map
    pub x: u8,
    /// Y position on map
    pub y: u8,
}

/// Generated map account storing tiles, enemies, and POI positions.
/// PDA seeds: ["generated_map", session.key()]
///
/// **Persistence invariant:** This account is session-scoped and its layout
/// (including `packed_tiles`) persists unchanged across week transitions.
/// Week advancement in gameplay-state only modifies `GameState.week` —
/// `packed_tiles` is never reset. Wall-to-floor conversions via `set_floor()`
/// write directly to `packed_tiles` and persist for the entire session.
#[account]
pub struct GeneratedMap {
    /// Session this map belongs to
    pub session: Pubkey, // 32 bytes
    /// Map width in tiles
    pub width: u8, // 1 byte
    /// Map height in tiles
    pub height: u8, // 1 byte
    /// Seed used for generation
    pub seed: u64, // 8 bytes
    /// Player spawn X position
    pub spawn_x: u8, // 1 byte
    /// Player spawn Y position
    pub spawn_y: u8, // 1 byte
    /// Mole den X position (above spawn)
    pub mole_den_x: u8, // 1 byte
    /// Mole den Y position
    pub mole_den_y: u8, // 1 byte
    /// Number of walkable (floor) tiles
    pub walkable_count: u16, // 2 bytes
    /// Bit-packed tiles (0=floor, 1=wall), 313 bytes for 2500 tiles
    pub packed_tiles: [u8; PACKED_TILES_SIZE], // 313 bytes
    /// Number of enemies placed
    pub enemy_count: u8, // 1 byte
    /// Enemy spawn positions (max 48)
    pub enemies: [EnemySpawn; MAX_ENEMIES], // 192 bytes (48 * 4)
    /// Number of POIs placed
    pub poi_count: u8, // 1 byte
    /// POI spawn positions (max 50)
    pub pois: [PoiSpawn; MAX_POIS], // 200 bytes (50 * 4)
    /// PDA bump seed
    pub bump: u8, // 1 byte
}

impl GeneratedMap {
    /// PDA seed prefix for generated map
    pub const SEED_PREFIX: &'static [u8] = b"generated_map";

    /// Account space: 8 (discriminator) + 756 (data) = 764 bytes
    pub const SPACE: usize = 8
        + 32
        + 1
        + 1
        + 8
        + 1
        + 1
        + 1
        + 1
        + 2
        + PACKED_TILES_SIZE
        + 1
        + (MAX_ENEMIES * 4)
        + 1
        + (MAX_POIS * 4)
        + 1;

    /// Check if a tile at (x, y) is walkable (floor)
    pub fn is_walkable(&self, x: u8, y: u8) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        let byte_index = index / 8;
        let bit_index = index % 8;
        // 0 = floor (walkable), 1 = wall
        (self.packed_tiles[byte_index] >> bit_index) & 1 == 0
    }

    /// Set a tile at (x, y) as floor (walkable)
    pub fn set_floor(&mut self, x: u8, y: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        let byte_index = index / 8;
        let bit_index = index % 8;
        // Clear the bit (0 = floor)
        self.packed_tiles[byte_index] &= !(1 << bit_index);
    }

    /// Set a tile at (x, y) as wall
    pub fn set_wall(&mut self, x: u8, y: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        let byte_index = index / 8;
        let bit_index = index % 8;
        // Set the bit (1 = wall)
        self.packed_tiles[byte_index] |= 1 << bit_index;
    }

    /// Initialize all tiles as walls (all bits set to 1)
    pub fn init_all_walls(&mut self) {
        for byte in self.packed_tiles.iter_mut() {
            *byte = 0xFF;
        }
    }
}
