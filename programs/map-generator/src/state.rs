use crate::constants::{
    ECHO_DATA_SIZE, LEVEL_COUNT, MAX_DISCOVERED_POIS, MAX_ENEMIES, MAX_POIS, PACKED_TILES_SIZE,
};
use anchor_lang::prelude::*;
use vrf_rng::VrfStatus;

// =============================================================================
// VRF State
// =============================================================================

/// Per-session VRF randomness for map generation.
/// PDA seeds: ["map_vrf", session.key()]
#[account]
pub struct MapVrfState {
    /// Session PDA this VRF is tied to
    pub session: Pubkey,
    /// Oracle-provided randomness (filled on fulfill)
    pub randomness: [u8; 32],
    /// Anti-replay counter (incremented on each request)
    pub nonce: u64,
    /// Lifecycle stage
    pub status: VrfStatus,
    /// PDA bump
    pub bump: u8,
}

impl MapVrfState {
    pub const SEED_PREFIX: &'static [u8] = b"map_vrf";
    /// 8 (disc) + 32 + 32 + 8 + 1 + 1 = 82
    pub const SPACE: usize = vrf_rng::VRF_STATE_SPACE;
}

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
    /// Bit-packed discovered tiles (1 = discovered/revealed)
    pub discovered_tiles: [u8; PACKED_TILES_SIZE], // 313 bytes
    /// Number of enemies placed
    pub enemy_count: u8, // 1 byte
    /// Enemy spawn positions (max 48)
    pub enemies: [EnemySpawn; MAX_ENEMIES], // 192 bytes (48 * 4)
    /// Number of POIs placed
    pub poi_count: u8, // 1 byte
    /// POI spawn positions (max 64)
    pub pois: [PoiSpawn; MAX_POIS], // 256 bytes (64 * 4)
    /// PDA bump seed
    pub bump: u8, // 1 byte
}

impl GeneratedMap {
    /// PDA seed prefix for generated map
    pub const SEED_PREFIX: &'static [u8] = b"generated_map";

    /// Account space: 8 (discriminator) + 812 (data) = 820 bytes
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

    /// Check if a tile at (x, y) has been discovered.
    pub fn is_discovered(&self, x: u8, y: u8) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        let byte_index = index / 8;
        let bit_index = index % 8;
        (self.discovered_tiles[byte_index] >> bit_index) & 1 == 1
    }

    /// Mark a tile at (x, y) as discovered.
    pub fn reveal_tile(&mut self, x: u8, y: u8) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        let byte_index = index / 8;
        let bit_index = index % 8;
        let was_discovered = (self.discovered_tiles[byte_index] >> bit_index) & 1 == 1;
        self.discovered_tiles[byte_index] |= 1 << bit_index;
        !was_discovered
    }

    /// Reveal all tiles in the gameplay visibility radius shape.
    pub fn reveal_radius(&mut self, center_x: u8, center_y: u8, radius: u8) -> u16 {
        let mut revealed = 0u16;
        let radius_i = radius as i16;
        let center_x_i = center_x as i16;
        let center_y_i = center_y as i16;

        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                let x = center_x_i + dx;
                let y = center_y_i + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as u8;
                let y = y as u8;
                if x >= self.width || y >= self.height {
                    continue;
                }

                let distance_sq = dx * dx + dy * dy;
                if distance_sq <= radius_i * (radius_i + 1) && self.reveal_tile(x, y) {
                    revealed = revealed.saturating_add(1);
                }
            }
        }

        revealed
    }

    /// Reveal all tiles within a Manhattan-distance diamond.
    pub fn reveal_manhattan_radius(&mut self, center_x: u8, center_y: u8, radius: u8) -> u16 {
        let mut revealed = 0u16;
        let radius_i = radius as i16;
        let center_x_i = center_x as i16;
        let center_y_i = center_y as i16;

        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                if dx.abs() + dy.abs() > radius_i {
                    continue;
                }

                let x = center_x_i + dx;
                let y = center_y_i + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as u8;
                let y = y as u8;
                if x >= self.width || y >= self.height {
                    continue;
                }

                if self.reveal_tile(x, y) {
                    revealed = revealed.saturating_add(1);
                }
            }
        }

        revealed
    }

    /// Initialize all tiles as walls (all bits set to 1)
    pub fn init_all_walls(&mut self) {
        for byte in self.packed_tiles.iter_mut() {
            *byte = 0xFF;
        }
    }

    /// Reset discovered-state to fully hidden.
    pub fn clear_discovery(&mut self) {
        for byte in self.discovered_tiles.iter_mut() {
            *byte = 0;
        }
    }
}

// =============================================================================
// Session Discovery (player-readable fog-of-war account)
// =============================================================================

/// A shop offer entry stored in SessionDiscovery (12 bytes).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug)]
pub struct DiscoveryShopOffer {
    pub item_id: [u8; 8],
    pub tier: u8,
    pub price: u16,
    pub purchased: u8,
}

/// An item offer entry stored in SessionDiscovery (10 bytes).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, Debug)]
pub struct DiscoveryOfferItem {
    pub item_id: [u8; 8],
    pub rarity: u8,
    pub tier: u8,
}

/// A discovered POI entry visible to the player (5 bytes).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct DiscoveredPoi {
    /// POI definition ID (1-14)
    pub poi_type: u8,
    /// X position on map
    pub x: u8,
    /// Y position on map
    pub y: u8,
    /// Whether the POI has been used
    pub used: u8,
    /// Index into MapPois.pois[] array (needed for interaction transactions)
    pub map_pois_index: u8,
}

/// A discovered enemy entry visible to the player (6 bytes).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct DiscoveredEnemy {
    /// Enemy archetype ID (0-11)
    pub archetype_id: u8,
    /// Enemy tier: 0=T1, 1=T2, 2=T3
    pub tier: u8,
    /// X position on map
    pub x: u8,
    /// Y position on map
    pub y: u8,
    /// 0=alive, 1=defeated
    pub defeated: u8,
    /// Index into MapEnemies.enemies[] array
    pub map_enemies_index: u8,
}

/// Player-readable account containing only discovered map data.
/// Private ER accounts (GeneratedMap, MapPois, MapEnemies) are unreadable via RPC;
/// the client reads this account for fog-of-war rendering.
///
/// PDA seeds: ["session_discovery", session.key()]
#[account]
pub struct SessionDiscovery {
    /// Session this discovery belongs to
    pub session: Pubkey, // 32
    /// Bit-packed discovered tiles (1 = discovered)
    pub discovered_tiles: [u8; PACKED_TILES_SIZE], // 313
    /// Bit-packed tile types for discovered tiles (0=floor, 1=wall)
    pub revealed_tile_types: [u8; PACKED_TILES_SIZE], // 313
    /// Player spawn X
    pub spawn_x: u8, // 1
    /// Player spawn Y
    pub spawn_y: u8, // 1
    /// Mole den X (objective)
    pub mole_den_x: u8, // 1
    /// Mole den Y
    pub mole_den_y: u8, // 1
    /// Map width in tiles
    pub map_width: u8, // 1
    /// Map height in tiles
    pub map_height: u8, // 1
    /// Number of discovered POIs
    pub discovered_poi_count: u8, // 1
    /// Discovered POI entries (populated as player discovers them)
    pub discovered_pois: [DiscoveredPoi; MAX_DISCOVERED_POIS], // 320 (64 * 5)
    /// PDA bump seed
    pub bump: u8, // 1
    /// Number of enemies in discovered tiles
    pub discovered_enemy_count: u8, // 1
    /// Enemies visible to the player (max 48, 6 bytes each)
    pub discovered_enemies: [DiscoveredEnemy; MAX_ENEMIES], // 288
    /// Boss ID for current week only (12-byte boss_system ID)
    pub current_boss_id: [u8; 12], // 12
    /// 0=no echo, 1=echo present
    pub current_echo_present: u8, // 1
    /// Raw serialized GauntletEchoSnapshot bytes
    pub current_echo_data: [u8; ECHO_DATA_SIZE], // 179
    /// Active offer type: 0=none, 1=shop, 2=cache, 3=oil, 4=scanner
    pub active_offer_type: u8, // 1
    /// MapPois index of the POI with the active offer
    pub active_offer_poi_index: u8, // 1
    /// Shop offers (valid when active_offer_type == 1)
    pub shop_offers: [DiscoveryShopOffer; 6], // 72
    /// Shop reroll count
    pub shop_reroll_count: u8, // 1
    /// Shop active flag
    pub shop_active: u8, // 1
    /// Cache offer items (valid when active_offer_type == 2)
    pub cache_offer_items: [DiscoveryOfferItem; 3], // 30
    /// Oil offer flags (valid when active_offer_type == 3)
    pub oil_offer_oils: [u8; 3], // 3
    /// Scanner offer count (valid when active_offer_type == 4)
    pub scanner_offer_count: u8, // 1
    /// Scanner offer POI types
    pub scanner_offer_types: [u8; 3], // 3
}

impl SessionDiscovery {
    pub const SEED_PREFIX: &'static [u8] = b"session_discovery";

    /// Account space: 8 (discriminator) + 1580 (data) = 1588 bytes
    pub const SPACE: usize = 8
        + 32                      // session
        + PACKED_TILES_SIZE       // discovered_tiles
        + PACKED_TILES_SIZE       // revealed_tile_types
        + 1                       // spawn_x
        + 1                       // spawn_y
        + 1                       // mole_den_x
        + 1                       // mole_den_y
        + 1                       // map_width
        + 1                       // map_height
        + 1                       // discovered_poi_count
        + (MAX_DISCOVERED_POIS * 5) // discovered_pois
        + 1                       // bump
        + 1                       // discovered_enemy_count
        + (MAX_ENEMIES * 6)       // discovered_enemies
        + 12                      // current_boss_id
        + 1                       // current_echo_present
        + ECHO_DATA_SIZE          // current_echo_data
        + 1                       // active_offer_type
        + 1                       // active_offer_poi_index
        + (6 * 12)                // shop_offers (6 * DiscoveryShopOffer)
        + 1                       // shop_reroll_count
        + 1                       // shop_active
        + (3 * 10)                // cache_offer_items (3 * DiscoveryOfferItem)
        + 3                       // oil_offer_oils
        + 1                       // scanner_offer_count
        + 3; // scanner_offer_types

    /// Sync a single tile's discovery state from GeneratedMap to SessionDiscovery.
    /// Returns true if the tile was newly discovered.
    pub fn sync_tile(&mut self, map: &GeneratedMap, x: u8, y: u8) -> bool {
        if x >= map.width || y >= map.height {
            return false;
        }
        let index = (y as usize) * (map.width as usize) + (x as usize);
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        let mask = 1u8 << bit_idx;

        let was_discovered = (self.discovered_tiles[byte_idx] & mask) != 0;
        self.discovered_tiles[byte_idx] |= mask;

        // Copy tile type (0=floor, 1=wall) from packed_tiles
        let is_wall = (map.packed_tiles[byte_idx] >> bit_idx) & 1 == 1;
        if is_wall {
            self.revealed_tile_types[byte_idx] |= mask;
        } else {
            self.revealed_tile_types[byte_idx] &= !mask;
        }

        !was_discovered
    }

    /// Bulk sync all currently discovered tiles from GeneratedMap.
    /// Used after initial map generation to populate the spawn-area reveal.
    pub fn sync_all_discovered(&mut self, map: &GeneratedMap) {
        for byte_idx in 0..PACKED_TILES_SIZE {
            let discovered = map.discovered_tiles[byte_idx];
            if discovered == 0 {
                continue;
            }
            self.discovered_tiles[byte_idx] |= discovered;
            // For each discovered bit, copy the tile type
            for bit in 0..8u8 {
                if (discovered >> bit) & 1 == 1 {
                    let mask = 1u8 << bit;
                    if (map.packed_tiles[byte_idx] >> bit) & 1 == 1 {
                        self.revealed_tile_types[byte_idx] |= mask;
                    } else {
                        self.revealed_tile_types[byte_idx] &= !mask;
                    }
                }
            }
        }
    }

    /// Sync discovery for all tiles in a circular radius (matches GeneratedMap::reveal_radius).
    pub fn sync_radius(&mut self, map: &GeneratedMap, center_x: u8, center_y: u8, radius: u8) {
        let radius_i = radius as i16;
        let cx = center_x as i16;
        let cy = center_y as i16;

        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                let x = cx + dx;
                let y = cy + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as u8;
                let y = y as u8;
                if x >= map.width || y >= map.height {
                    continue;
                }
                let distance_sq = dx * dx + dy * dy;
                if distance_sq <= radius_i * (radius_i + 1) {
                    self.sync_tile(map, x, y);
                }
            }
        }
    }

    /// Sync discovery for all tiles in a Manhattan-distance diamond.
    pub fn sync_manhattan_radius(
        &mut self,
        map: &GeneratedMap,
        center_x: u8,
        center_y: u8,
        radius: u8,
    ) {
        let radius_i = radius as i16;
        let cx = center_x as i16;
        let cy = center_y as i16;

        for dy in -radius_i..=radius_i {
            for dx in -radius_i..=radius_i {
                if dx.abs() + dy.abs() > radius_i {
                    continue;
                }
                let x = cx + dx;
                let y = cy + dy;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as u8;
                let y = y as u8;
                if x >= map.width || y >= map.height {
                    continue;
                }
                self.sync_tile(map, x, y);
            }
        }
    }

    /// Update a single tile's type in revealed_tile_types (e.g., after wall→floor conversion).
    /// Only updates if the tile is already discovered.
    pub fn update_tile_type(&mut self, width: u8, x: u8, y: u8, is_wall: bool) {
        let index = (y as usize) * (width as usize) + (x as usize);
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        let mask = 1u8 << bit_idx;

        // Only update if tile is discovered
        if (self.discovered_tiles[byte_idx] & mask) != 0 {
            if is_wall {
                self.revealed_tile_types[byte_idx] |= mask;
            } else {
                self.revealed_tile_types[byte_idx] &= !mask;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PACKED_TILES_SIZE;

    fn create_test_map() -> GeneratedMap {
        GeneratedMap {
            session: Pubkey::default(),
            width: 50,
            height: 50,
            seed: 42,
            spawn_x: 10,
            spawn_y: 10,
            mole_den_x: 10,
            mole_den_y: 9,
            walkable_count: 0,
            packed_tiles: [0xFF; PACKED_TILES_SIZE], // all walls
            discovered_tiles: [0; PACKED_TILES_SIZE],
            enemy_count: 0,
            enemies: [EnemySpawn::default(); 48],
            poi_count: 0,
            pois: [PoiSpawn::default(); 64],
            bump: 0,
        }
    }

    fn create_test_discovery() -> SessionDiscovery {
        SessionDiscovery {
            session: Pubkey::default(),
            discovered_tiles: [0; PACKED_TILES_SIZE],
            revealed_tile_types: [0; PACKED_TILES_SIZE],
            spawn_x: 0,
            spawn_y: 0,
            mole_den_x: 0,
            mole_den_y: 0,
            map_width: 50,
            map_height: 50,
            discovered_poi_count: 0,
            discovered_pois: [DiscoveredPoi::default(); MAX_DISCOVERED_POIS],
            bump: 0,
            discovered_enemy_count: 0,
            discovered_enemies: [DiscoveredEnemy::default(); MAX_ENEMIES],
            current_boss_id: [0u8; 12],
            current_echo_present: 0,
            current_echo_data: [0u8; ECHO_DATA_SIZE],
            active_offer_type: 0,
            active_offer_poi_index: 0,
            shop_offers: [DiscoveryShopOffer::default(); 6],
            shop_reroll_count: 0,
            shop_active: 0,
            cache_offer_items: [DiscoveryOfferItem::default(); 3],
            oil_offer_oils: [0u8; 3],
            scanner_offer_count: 0,
            scanner_offer_types: [0u8; 3],
        }
    }

    // =========================================================================
    // SessionDiscovery space calculation
    // =========================================================================

    #[test]
    fn test_session_discovery_space() {
        let expected = 8
            + 32
            + 313
            + 313
            + 1
            + 1
            + 1
            + 1
            + 1
            + 1
            + 1
            + (64 * 5)
            + 1
            + 1
            + (48 * 6)
            + 12
            + 1
            + 179
            + 1
            + 1
            + (6 * 12)
            + 1
            + 1
            + (3 * 10)
            + 3
            + 1
            + 3;
        assert_eq!(SessionDiscovery::SPACE, expected);
        assert_eq!(SessionDiscovery::SPACE, 1588);
    }

    // =========================================================================
    // sync_tile: basic operation
    // =========================================================================

    #[test]
    fn test_sync_tile_wall() {
        let map = create_test_map(); // all walls
        let mut discovery = create_test_discovery();

        let newly = discovery.sync_tile(&map, 5, 5);
        assert!(newly);

        // Tile should be marked discovered
        let index = 5 * 50 + 5; // y=5, x=5
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        assert!((discovery.discovered_tiles[byte_idx] >> bit_idx) & 1 == 1);
        // Tile type should be wall (bit set)
        assert!((discovery.revealed_tile_types[byte_idx] >> bit_idx) & 1 == 1);
    }

    #[test]
    fn test_sync_tile_floor() {
        let mut map = create_test_map();
        map.set_floor(5, 5); // make it a floor tile
        let mut discovery = create_test_discovery();

        discovery.sync_tile(&map, 5, 5);

        let index = 5 * 50 + 5;
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        assert!((discovery.discovered_tiles[byte_idx] >> bit_idx) & 1 == 1);
        // Tile type should be floor (bit clear)
        assert!((discovery.revealed_tile_types[byte_idx] >> bit_idx) & 1 == 0);
    }

    #[test]
    fn test_sync_tile_idempotent() {
        let map = create_test_map();
        let mut discovery = create_test_discovery();

        let first = discovery.sync_tile(&map, 5, 5);
        assert!(first); // newly discovered

        let second = discovery.sync_tile(&map, 5, 5);
        assert!(!second); // already discovered
    }

    #[test]
    fn test_sync_tile_out_of_bounds() {
        let map = create_test_map();
        let mut discovery = create_test_discovery();

        assert!(!discovery.sync_tile(&map, 50, 0));
        assert!(!discovery.sync_tile(&map, 0, 50));
        assert!(!discovery.sync_tile(&map, 255, 255));
    }

    // =========================================================================
    // sync_all_discovered: bulk sync
    // =========================================================================

    #[test]
    fn test_sync_all_discovered_matches_map() {
        let mut map = create_test_map();
        // Set some floors and reveal them
        map.set_floor(10, 10);
        map.set_floor(11, 10);
        map.set_floor(12, 10);
        map.reveal_radius(10, 10, 2);

        let mut discovery = create_test_discovery();
        discovery.sync_all_discovered(&map);

        // All discovered tiles in map should be discovered in discovery
        for byte_idx in 0..PACKED_TILES_SIZE {
            assert_eq!(
                discovery.discovered_tiles[byte_idx] & map.discovered_tiles[byte_idx],
                map.discovered_tiles[byte_idx],
                "Discovery missing tiles at byte {byte_idx}"
            );
        }

        // Verify tile types match for discovered tiles
        assert!(map.is_discovered(10, 10));
        let index = 10 * 50 + 10;
        let byte_idx = index / 8;
        let bit_idx = index % 8;
        // (10,10) is floor
        assert!((discovery.revealed_tile_types[byte_idx] >> bit_idx) & 1 == 0);

        // (10,9) should be wall (all tiles start as walls, we only set_floor for 10-12,10)
        if map.is_discovered(10, 9) {
            let index2 = 9 * 50 + 10;
            let byte_idx2 = index2 / 8;
            let bit_idx2 = index2 % 8;
            assert!((discovery.revealed_tile_types[byte_idx2] >> bit_idx2) & 1 == 1);
        }
    }

    #[test]
    fn test_sync_all_discovered_empty_map() {
        let map = create_test_map(); // nothing discovered
        let mut discovery = create_test_discovery();
        discovery.sync_all_discovered(&map);

        // Discovery should remain empty
        for byte in discovery.discovered_tiles.iter() {
            assert_eq!(*byte, 0);
        }
    }

    // =========================================================================
    // sync_radius: dual-write consistency
    // =========================================================================

    #[test]
    fn test_sync_radius_matches_reveal_radius() {
        let mut map = create_test_map();
        map.set_floor(25, 25);
        map.set_floor(26, 25);
        map.set_floor(24, 25);
        map.set_floor(25, 26);
        map.set_floor(25, 24);

        // Reveal on map
        map.reveal_radius(25, 25, 3);

        // Sync to discovery
        let mut discovery = create_test_discovery();
        discovery.sync_radius(&map, 25, 25, 3);

        // Every tile discovered in map should be discovered in discovery
        for y in 0..50u8 {
            for x in 0..50u8 {
                assert_eq!(
                    map.is_discovered(x, y),
                    {
                        let idx = (y as usize) * 50 + (x as usize);
                        (discovery.discovered_tiles[idx / 8] >> (idx % 8)) & 1 == 1
                    },
                    "Mismatch at ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn test_sync_radius_tile_types_correct() {
        let mut map = create_test_map();
        // Create a mix of floor and wall
        map.set_floor(25, 25);
        map.set_floor(26, 25);
        // (24, 25) stays wall

        map.reveal_radius(25, 25, 2);
        let mut discovery = create_test_discovery();
        discovery.sync_radius(&map, 25, 25, 2);

        // Floor tile
        let idx_floor = 25 * 50 + 25;
        assert!((discovery.revealed_tile_types[idx_floor / 8] >> (idx_floor % 8)) & 1 == 0);

        // Wall tile
        let idx_wall = 25 * 50 + 24;
        assert!((discovery.revealed_tile_types[idx_wall / 8] >> (idx_wall % 8)) & 1 == 1);
    }

    #[test]
    fn test_sync_radius_at_map_edge() {
        let mut map = create_test_map();
        map.set_floor(0, 0);
        map.reveal_radius(0, 0, 4);

        let mut discovery = create_test_discovery();
        discovery.sync_radius(&map, 0, 0, 4);

        // Should not panic, corner tile should be discovered
        let idx = 0;
        assert!((discovery.discovered_tiles[idx / 8] >> (idx % 8)) & 1 == 1);
    }

    // =========================================================================
    // sync_manhattan_radius
    // =========================================================================

    #[test]
    fn test_sync_manhattan_radius_matches_map() {
        let mut map = create_test_map();
        map.reveal_manhattan_radius(25, 25, 5);

        let mut discovery = create_test_discovery();
        discovery.sync_manhattan_radius(&map, 25, 25, 5);

        for y in 0..50u8 {
            for x in 0..50u8 {
                let idx = (y as usize) * 50 + (x as usize);
                let map_disc = map.is_discovered(x, y);
                let disc_disc = (discovery.discovered_tiles[idx / 8] >> (idx % 8)) & 1 == 1;
                assert_eq!(map_disc, disc_disc, "Mismatch at ({x}, {y})");
            }
        }
    }

    // =========================================================================
    // update_tile_type: wall-to-floor conversion
    // =========================================================================

    #[test]
    fn test_update_tile_type_discovered_wall_to_floor() {
        let mut map = create_test_map(); // all walls
        let mut discovery = create_test_discovery();

        // Discover a wall tile
        discovery.sync_tile(&map, 5, 5);
        let idx = 5 * 50 + 5;
        assert!((discovery.revealed_tile_types[idx / 8] >> (idx % 8)) & 1 == 1); // wall

        // Convert to floor
        discovery.update_tile_type(50, 5, 5, false);
        assert!((discovery.revealed_tile_types[idx / 8] >> (idx % 8)) & 1 == 0);
        // now floor
    }

    #[test]
    fn test_update_tile_type_undiscovered_noop() {
        let mut discovery = create_test_discovery();

        // Try to update an undiscovered tile — should be a no-op
        discovery.update_tile_type(50, 5, 5, false);

        let idx = 5 * 50 + 5;
        // Still not discovered
        assert!((discovery.discovered_tiles[idx / 8] >> (idx % 8)) & 1 == 0);
        // Tile type unchanged (0)
        assert!((discovery.revealed_tile_types[idx / 8] >> (idx % 8)) & 1 == 0);
    }

    // =========================================================================
    // DiscoveredPoi tracking
    // =========================================================================

    #[test]
    fn test_discovered_poi_defaults() {
        let poi = DiscoveredPoi::default();
        assert_eq!(poi.poi_type, 0);
        assert_eq!(poi.x, 0);
        assert_eq!(poi.y, 0);
        assert_eq!(poi.used, 0);
    }

    #[test]
    fn test_discovered_poi_count_tracking() {
        let mut discovery = create_test_discovery();
        assert_eq!(discovery.discovered_poi_count, 0);

        discovery.discovered_pois[0] = DiscoveredPoi {
            poi_type: 8,
            x: 20,
            y: 30,
            used: 0,
            map_pois_index: 0,
        };
        discovery.discovered_poi_count = 1;

        assert_eq!(discovery.discovered_poi_count, 1);
        assert_eq!(discovery.discovered_pois[0].poi_type, 8);
        assert_eq!(discovery.discovered_pois[0].x, 20);
        assert_eq!(discovery.discovered_pois[0].y, 30);
    }

    // =========================================================================
    // Full generation + discovery sync (integration-style)
    // =========================================================================

    #[test]
    fn test_full_map_generation_discovery_sync() {
        let mut map = create_test_map();
        let success = crate::maze::generate_map(&mut map, 12345, 1);
        assert!(success);

        map.clear_discovery();
        let spawn_x = map.spawn_x;
        let spawn_y = map.spawn_y;
        map.reveal_radius(spawn_x, spawn_y, 6);

        let mut discovery = create_test_discovery();
        discovery.spawn_x = map.spawn_x;
        discovery.spawn_y = map.spawn_y;
        discovery.mole_den_x = map.mole_den_x;
        discovery.mole_den_y = map.mole_den_y;
        discovery.map_width = map.width;
        discovery.map_height = map.height;
        discovery.sync_all_discovered(&map);

        // Verify metadata
        assert_eq!(discovery.spawn_x, map.spawn_x);
        assert_eq!(discovery.spawn_y, map.spawn_y);
        assert_eq!(discovery.mole_den_x, map.mole_den_x);
        assert_eq!(discovery.mole_den_y, map.mole_den_y);
        assert_eq!(discovery.map_width, 50);
        assert_eq!(discovery.map_height, 50);

        // Spawn tile should be discovered and floor
        let spawn_idx = (spawn_y as usize) * 50 + (spawn_x as usize);
        assert!((discovery.discovered_tiles[spawn_idx / 8] >> (spawn_idx % 8)) & 1 == 1);
        assert!((discovery.revealed_tile_types[spawn_idx / 8] >> (spawn_idx % 8)) & 1 == 0);

        // Count discovered tiles — should match map
        let mut map_count = 0u32;
        let mut disc_count = 0u32;
        for byte_idx in 0..PACKED_TILES_SIZE {
            map_count += map.discovered_tiles[byte_idx].count_ones();
            disc_count += discovery.discovered_tiles[byte_idx].count_ones();
        }
        assert_eq!(map_count, disc_count);
        assert!(disc_count > 0);
    }

    #[test]
    fn test_incremental_reveal_consistency() {
        let mut map = create_test_map();
        let success = crate::maze::generate_map(&mut map, 99999, 5);
        assert!(success);
        map.clear_discovery();

        let mut discovery = create_test_discovery();
        discovery.map_width = map.width;
        discovery.map_height = map.height;

        // Simulate movement: reveal at spawn, then move and reveal at new position
        let sx = map.spawn_x;
        let sy = map.spawn_y;
        map.reveal_radius(sx, sy, 4);
        discovery.sync_radius(&map, sx, sy, 4);

        // Move right (if possible)
        let nx = sx + 1;
        let ny = sy;
        map.reveal_radius(nx, ny, 4);
        discovery.sync_radius(&map, nx, ny, 4);

        // All discovered tiles should match
        for byte_idx in 0..PACKED_TILES_SIZE {
            assert_eq!(
                discovery.discovered_tiles[byte_idx], map.discovered_tiles[byte_idx],
                "Discovery mismatch at byte {byte_idx}"
            );
        }

        // Tile types should be consistent
        for byte_idx in 0..PACKED_TILES_SIZE {
            let discovered = discovery.discovered_tiles[byte_idx];
            for bit in 0..8u8 {
                if (discovered >> bit) & 1 == 1 {
                    let is_wall_in_map = (map.packed_tiles[byte_idx] >> bit) & 1 == 1;
                    let is_wall_in_disc = (discovery.revealed_tile_types[byte_idx] >> bit) & 1 == 1;
                    assert_eq!(
                        is_wall_in_map, is_wall_in_disc,
                        "Tile type mismatch at byte {byte_idx} bit {bit}"
                    );
                }
            }
        }
    }
}
