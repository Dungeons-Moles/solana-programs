//! Corridor maze generation using iterative recursive backtracker algorithm.
//!
//! This module generates corridor mazes, producing deterministic maps based on a seed value.
//!
//! Optimized for Solana BPF compute units - uses fixed-size arrays and integer math.

use crate::constants::{
    CELL_GRID_HEIGHT, CELL_GRID_SIZE, CELL_GRID_WIDTH, CELL_SPACING, MAP_HEIGHT, MAP_WIDTH,
    MAX_ENEMIES, MAX_POIS, TOTAL_TILES,
};
use crate::rng::SeededRNG;
use crate::state::{EnemySpawn, GeneratedMap, PoiSpawn, Position};

/// Direction for maze carving
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Direction {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
}

impl Direction {
    /// Get the opposite direction
    #[inline]
    fn opposite(self) -> Self {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East,
        }
    }

    /// Get delta (dx, dy) for this direction
    #[inline]
    fn delta(self) -> (i8, i8) {
        match self {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }

    /// Get direction from index
    #[inline]
    fn from_index(idx: u8) -> Self {
        match idx & 3 {
            0 => Direction::North,
            1 => Direction::South,
            2 => Direction::East,
            _ => Direction::West,
        }
    }
}

/// Cell in the maze grid (5 bits: 1 visited + 4 connections)
/// Packed into a single byte for efficiency
#[derive(Clone, Copy, Default)]
struct Cell {
    /// Bit 0: visited, Bits 1-4: connections (N, S, E, W)
    flags: u8,
}

impl Cell {
    const VISITED_BIT: u8 = 1;

    #[inline]
    fn is_visited(&self) -> bool {
        self.flags & Self::VISITED_BIT != 0
    }

    #[inline]
    fn set_visited(&mut self) {
        self.flags |= Self::VISITED_BIT;
    }

    #[inline]
    fn has_connection(&self, dir: Direction) -> bool {
        self.flags & (2 << (dir as u8)) != 0
    }

    #[inline]
    fn set_connection(&mut self, dir: Direction) {
        self.flags |= 2 << (dir as u8);
    }
}

// ============================================================================
// POI Placement Configuration
// ============================================================================

const POI_MIN_SPACING: u8 = 10;

// Baseline POI counts per act [Act1, Act2, Act3, Act4].
// Higher acts = fewer POIs (harder scarcity).
const BASELINE_L2_SUPPLY_CACHE: [u8; 4] = [16, 14, 14, 10];
const BASELINE_L3_TOOL_CRATE: [u8; 4] = [5, 4, 4, 2];
const BASELINE_L4_TOOL_OIL: [u8; 4] = [5, 4, 4, 3];
const BASELINE_L5_REST_ALCOVE: [u8; 4] = [6, 5, 5, 4];
const BASELINE_L6_SURVEY_BEACON: [u8; 4] = [4, 4, 4, 3];
const BASELINE_L7_SEISMIC_SCANNER: [u8; 4] = [3, 3, 3, 2];
const BASELINE_L8_RAIL_WAYPOINT: [u8; 4] = [5, 4, 4, 2];
const BASELINE_L9_SMUGGLER_HATCH: [u8; 4] = [2, 2, 2, 1];
const BASELINE_L10_RUSTY_ANVIL: [u8; 4] = [2, 2, 2, 1];
const BASELINE_L11_RUNE_KILN: [u8; 4] = [2, 1, 1, 1];
const BASELINE_L12_GEODE_VAULT: [u8; 4] = [2, 1, 1, 1];
const BASELINE_L13_COUNTER_CACHE: [u8; 4] = [2, 2, 2, 2];
const BASELINE_L14_SCRAP_CHUTE: [u8; 4] = [3, 2, 2, 1];

// ==========================================================================
// Enemy Placement Configuration
// ==========================================================================

const ACT_ENEMY_TARGETS: [u8; 4] = [36, 40, 44, 48];

/// Minimum Chebyshev distance from spawn where enemies can be placed.
/// Night phase chases enemies within 3 tiles, so 5 gives a safe buffer.
const SPAWN_PROTECTION_RADIUS: u8 = 5;

/// Difficulty pools for safe-start and ramping rules.
const EASY_POOL: [u8; 5] = [0, 1, 8, 10, 11]; // Rat, Bat, Wisp, Coin Slug, Blood Mosquito
const MEDIUM_POOL: [u8; 4] = [2, 3, 5, 9]; // Spore, Rust Mite, Shard Beetle, Powder Tick
const HARD_POOL: [u8; 3] = [4, 6, 7]; // Collapsed Miner, Tunnel Warden, Burrow Ambusher

/// Enemy pool weights by distance band: [easy%, medium%, hard%].
/// Near spawn = easier enemies, far from spawn = harder enemies.
const DISTANCE_POOL_WEIGHTS: [[u8; 3]; 3] = [
    [60, 30, 10], // Near spawn (0-33% distance)
    [40, 40, 20], // Mid map (34-66% distance)
    [30, 40, 30], // Far map (67-100% distance)
];

/// Tier distribution near spawn: 80% T1 / 15% T2 / 5% T3
const NEAR_TIER_WEIGHTS: [u8; 3] = [80, 15, 5];
/// Tier distribution far from spawn: 50% T1 / 35% T2 / 15% T3
const FAR_TIER_WEIGHTS: [u8; 3] = [50, 35, 15];
/// Mid-map act defaults by act 1-4 as [T1, T2, T3] percentages.
const ACT_DEFAULT_TIER_WEIGHTS: [[u8; 3]; 4] =
    [[70, 25, 5], [55, 35, 10], [45, 40, 15], [35, 45, 20]];

// Biome weights for enemy archetypes from GDD:
// Biome A emphasizes: Tunnel Rat, Collapsed Miner, Shard Beetle, Coin Slug
// Biome B emphasizes: Rust Mite Swarm, Frost Wisp, Blood Mosquito, Burrow Ambusher, Powder Tick
// Format: [biome_a_weight, biome_b_weight] for each archetype ID (0-11)
const ENEMY_BIOME_WEIGHTS: [[u8; 2]; 12] = [
    [2, 1], // 0: Tunnel Rat (Biome A)
    [1, 1], // 1: Cave Bat (neutral)
    [1, 1], // 2: Spore Slime (neutral)
    [1, 2], // 3: Rust Mite Swarm (Biome B)
    [2, 1], // 4: Collapsed Miner (Biome A)
    [2, 1], // 5: Shard Beetle (Biome A)
    [1, 1], // 6: Tunnel Warden (neutral)
    [1, 2], // 7: Burrow Ambusher (Biome B)
    [1, 2], // 8: Frost Wisp (Biome B)
    [1, 2], // 9: Powder Tick (Biome B)
    [2, 1], // 10: Coin Slug (Biome A)
    [1, 2], // 11: Blood Mosquito (Biome B)
];

/// Returns true if the campaign level is in Biome A (Acts 1 & 3), false for Biome B (Acts 2 & 4)
#[inline]
fn is_biome_a(campaign_level: u8) -> bool {
    // 40 levels total: Acts 1 (1-10), 2 (11-20), 3 (21-30), 4 (31-40)
    // Biome A = Acts 1 & 3, Biome B = Acts 2 & 4
    let act = ((campaign_level.saturating_sub(1)) / 10) + 1;
    act == 1 || act == 3
}

#[inline]
fn act_index_from_campaign_level(campaign_level: u8) -> usize {
    ((campaign_level.saturating_sub(1)) / 10).clamp(0, 3) as usize
}

fn poi_target_counts_for_act(act_index: usize, rng: &mut SeededRNG) -> [u8; 15] {
    let mut targets = [0u8; 15];

    targets[1] = 1; // L1 Mole Den (fixed)
    targets[2] = BASELINE_L2_SUPPLY_CACHE[act_index];
    targets[3] = BASELINE_L3_TOOL_CRATE[act_index];
    targets[4] = BASELINE_L4_TOOL_OIL[act_index];
    targets[5] = BASELINE_L5_REST_ALCOVE[act_index];
    targets[6] = BASELINE_L6_SURVEY_BEACON[act_index];
    targets[7] = BASELINE_L7_SEISMIC_SCANNER[act_index];
    targets[8] = BASELINE_L8_RAIL_WAYPOINT[act_index];
    targets[9] = BASELINE_L9_SMUGGLER_HATCH[act_index];
    targets[10] = BASELINE_L10_RUSTY_ANVIL[act_index];
    targets[11] = BASELINE_L11_RUNE_KILN[act_index];
    targets[12] = BASELINE_L12_GEODE_VAULT[act_index];
    targets[13] = BASELINE_L13_COUNTER_CACHE[act_index];
    targets[14] = BASELINE_L14_SCRAP_CHUTE[act_index];

    targets
}

/// Select an enemy archetype from a specific pool using biome-weighted random selection.
fn select_weighted_archetype_from_pool(rng: &mut SeededRNG, campaign_level: u8, pool: &[u8]) -> u8 {
    let biome_index = if is_biome_a(campaign_level) { 0 } else { 1 };

    // Calculate total weight
    let mut total_weight: u16 = 0;
    for &archetype_id in pool {
        total_weight += ENEMY_BIOME_WEIGHTS[archetype_id as usize][biome_index] as u16;
    }

    // Roll a random value in range [0, total_weight)
    let roll = (rng.next_val() % total_weight as u64) as u16;

    // Select archetype based on cumulative weights
    let mut cumulative: u16 = 0;
    for &archetype_id in pool {
        cumulative += ENEMY_BIOME_WEIGHTS[archetype_id as usize][biome_index] as u16;
        if roll < cumulative {
            return archetype_id;
        }
    }

    // Fallback (should never happen).
    pool[0]
}

#[inline]
fn distance_percent_from_spawn(position: Position, spawn_position: Position) -> u8 {
    // Max manhattan distance in 50x50 map = (49 + 49) = 98.
    const MAX_DISTANCE: u16 = (MAP_WIDTH as u16 - 1) + (MAP_HEIGHT as u16 - 1);
    let distance = manhattan_distance(position, spawn_position).min(MAX_DISTANCE);
    ((distance * 100) / MAX_DISTANCE) as u8
}

#[inline]
fn distance_band(distance_pct: u8) -> usize {
    if distance_pct <= 33 {
        0
    } else if distance_pct <= 66 {
        1
    } else {
        2
    }
}

fn select_pool_for_distance(distance_pct: u8, rng: &mut SeededRNG) -> &'static [u8] {
    let band = distance_band(distance_pct);
    let weights = DISTANCE_POOL_WEIGHTS[band];
    let roll = (rng.next_val() % 100) as u8;

    if roll < weights[0] {
        &EASY_POOL
    } else if roll < weights[0] + weights[1] {
        &MEDIUM_POOL
    } else {
        &HARD_POOL
    }
}

fn select_tier_for_distance(distance_pct: u8, campaign_level: u8, rng: &mut SeededRNG) -> u8 {
    let weights = match distance_band(distance_pct) {
        0 => NEAR_TIER_WEIGHTS,
        1 => {
            let act = ((campaign_level.saturating_sub(1)) / 10).clamp(0, 3);
            ACT_DEFAULT_TIER_WEIGHTS[act as usize]
        }
        _ => FAR_TIER_WEIGHTS,
    };

    let roll = (rng.next_val() % 100) as u8;
    if roll < weights[0] {
        1
    } else if roll < weights[0] + weights[1] {
        2
    } else {
        3
    }
}

#[inline]
fn is_easy_archetype(archetype_id: u8) -> bool {
    EASY_POOL.contains(&archetype_id)
}

fn enforce_safe_start_easy_pool(map: &mut GeneratedMap, rng: &mut SeededRNG, campaign_level: u8) {
    if map.enemy_count == 0 {
        return;
    }

    let spawn_position = Position {
        x: map.spawn_x,
        y: map.spawn_y,
    };

    // Track three closest enemies to spawn.
    let mut closest: [(u16, usize); 3] = [(u16::MAX, usize::MAX); 3];
    for idx in 0..map.enemy_count as usize {
        let enemy = map.enemies[idx];
        let dist = manhattan_distance(
            Position {
                x: enemy.x,
                y: enemy.y,
            },
            spawn_position,
        );

        for slot in 0..3 {
            if dist < closest[slot].0 {
                for shift in (slot + 1..3).rev() {
                    closest[shift] = closest[shift - 1];
                }
                closest[slot] = (dist, idx);
                break;
            }
        }
    }

    for (_, idx) in closest {
        if idx == usize::MAX {
            continue;
        }
        let enemy = &mut map.enemies[idx];
        if !is_easy_archetype(enemy.archetype_id) {
            enemy.archetype_id =
                select_weighted_archetype_from_pool(rng, campaign_level, &EASY_POOL);
        }
    }
}

/// Maze generator state with fixed-size arrays (no heap allocations)
pub struct MazeGenerator {
    cells: [Cell; CELL_GRID_SIZE],
    /// Stack for backtracking - stores packed (cx << 4 | cy) values
    stack: [u8; CELL_GRID_SIZE],
    stack_len: usize,
    rng: SeededRNG,
}

impl MazeGenerator {
    /// Create a new maze generator with the given seed
    pub fn new(seed: u64) -> Self {
        Self {
            cells: [Cell::default(); CELL_GRID_SIZE],
            stack: [0u8; CELL_GRID_SIZE],
            stack_len: 0,
            rng: SeededRNG::new(seed),
        }
    }

    /// Get cell index from grid coordinates
    #[inline]
    fn cell_index(cx: u8, cy: u8) -> usize {
        (cy as usize) * (CELL_GRID_WIDTH as usize) + (cx as usize)
    }

    /// Pack coordinates into a single byte
    #[inline]
    fn pack_coords(cx: u8, cy: u8) -> u8 {
        (cx << 4) | cy
    }

    /// Unpack coordinates from a single byte
    #[inline]
    fn unpack_coords(packed: u8) -> (u8, u8) {
        (packed >> 4, packed & 0x0F)
    }

    /// Count unvisited neighbors and get a random one
    /// Returns None if no unvisited neighbors, Some((nx, ny, dir)) otherwise
    #[inline]
    fn get_random_unvisited_neighbor(&mut self, cx: u8, cy: u8) -> Option<(u8, u8, Direction)> {
        // Count unvisited neighbors first
        let mut count: u8 = 0;
        let mut neighbors: [u8; 4] = [0; 4]; // packed: dir << 4 | valid

        // Check all 4 directions
        for dir_idx in 0..4u8 {
            let dir = Direction::from_index(dir_idx);
            let (dx, dy) = dir.delta();
            let nx = (cx as i8) + dx;
            let ny = (cy as i8) + dy;

            // Check bounds
            if nx >= 0 && ny >= 0 && nx < CELL_GRID_WIDTH as i8 && ny < CELL_GRID_HEIGHT as i8 {
                let idx = Self::cell_index(nx as u8, ny as u8);
                if !self.cells[idx].is_visited() {
                    neighbors[count as usize] = dir_idx;
                    count += 1;
                }
            }
        }

        if count == 0 {
            return None;
        }

        // Pick random neighbor
        let choice = (self.rng.next_val() % (count as u64)) as usize;
        let dir = Direction::from_index(neighbors[choice]);
        let (dx, dy) = dir.delta();

        Some((((cx as i8) + dx) as u8, ((cy as i8) + dy) as u8, dir))
    }

    /// Generate the maze using iterative recursive backtracker
    pub fn generate(&mut self) {
        // Start at random cell
        let start_cx = (self.rng.next_val() % (CELL_GRID_WIDTH as u64)) as u8;
        let start_cy = (self.rng.next_val() % (CELL_GRID_HEIGHT as u64)) as u8;

        // Push start to stack
        self.stack[0] = Self::pack_coords(start_cx, start_cy);
        self.stack_len = 1;

        // Mark start as visited
        let start_idx = Self::cell_index(start_cx, start_cy);
        self.cells[start_idx].set_visited();

        // Iterative backtracking
        while self.stack_len > 0 {
            let (cx, cy) = Self::unpack_coords(self.stack[self.stack_len - 1]);

            match self.get_random_unvisited_neighbor(cx, cy) {
                None => {
                    // Backtrack
                    self.stack_len -= 1;
                }
                Some((nx, ny, dir)) => {
                    // Connect current cell to neighbor
                    let current_idx = Self::cell_index(cx, cy);
                    self.cells[current_idx].set_connection(dir);

                    // Connect neighbor back to current and mark visited
                    let neighbor_idx = Self::cell_index(nx, ny);
                    self.cells[neighbor_idx].set_visited();
                    self.cells[neighbor_idx].set_connection(dir.opposite());

                    // Push neighbor to stack
                    self.stack[self.stack_len] = Self::pack_coords(nx, ny);
                    self.stack_len += 1;
                }
            }
        }

        // Add extra connections for loops (15% of cells = 21 connections)
        // Using integer math: 144 * 15 / 100 = 21
        let extra_count = (CELL_GRID_SIZE * 15) / 100;
        for _ in 0..extra_count {
            let cx = (self.rng.next_val() % (CELL_GRID_WIDTH as u64)) as u8;
            let cy = (self.rng.next_val() % (CELL_GRID_HEIGHT as u64)) as u8;
            let dir = Direction::from_index((self.rng.next_val() % 4) as u8);
            let (dx, dy) = dir.delta();

            let nx = (cx as i8) + dx;
            let ny = (cy as i8) + dy;

            // If neighbor is valid and not already connected, add connection
            if nx >= 0 && ny >= 0 && nx < CELL_GRID_WIDTH as i8 && ny < CELL_GRID_HEIGHT as i8 {
                let current_idx = Self::cell_index(cx, cy);
                if !self.cells[current_idx].has_connection(dir) {
                    self.cells[current_idx].set_connection(dir);
                    let neighbor_idx = Self::cell_index(nx as u8, ny as u8);
                    self.cells[neighbor_idx].set_connection(dir.opposite());
                }
            }
        }
    }

    /// Carve the maze corridors into the map
    pub fn carve_into_map(&self, map: &mut GeneratedMap) {
        let spacing = CELL_SPACING as usize;

        for cy in 0..CELL_GRID_HEIGHT {
            for cx in 0..CELL_GRID_WIDTH {
                let idx = Self::cell_index(cx, cy);
                let cell = &self.cells[idx];

                // Cell center position in map coordinates
                let map_x = (cx as usize) * spacing;
                let map_y = (cy as usize) * spacing;

                // Carve the cell center
                map.set_floor(map_x as u8, map_y as u8);

                // Carve corridors to connected neighbors (only east and south to avoid double-carving)
                if cell.has_connection(Direction::East) && cx < CELL_GRID_WIDTH - 1 {
                    for i in 1..=spacing {
                        map.set_floor((map_x + i) as u8, map_y as u8);
                    }
                }

                if cell.has_connection(Direction::South) && cy < CELL_GRID_HEIGHT - 1 {
                    for i in 1..=spacing {
                        map.set_floor(map_x as u8, (map_y + i) as u8);
                    }
                }
            }
        }
    }

    /// Find a valid spawn point using reservoir sampling (O(1) extra memory)
    /// Returns (x, y) of a floor tile with a wall above
    pub fn find_spawn_point(&mut self, map: &GeneratedMap) -> Option<(u8, u8)> {
        let mut result: Option<(u8, u8)> = None;
        let mut count: u32 = 0;

        // Iterate through all valid spawn positions using reservoir sampling
        for y in 1..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                // Must be a floor tile with wall above
                if map.is_walkable(x, y) && !map.is_walkable(x, y - 1) {
                    count += 1;
                    // Reservoir sampling: keep with probability 1/count
                    #[allow(clippy::manual_is_multiple_of)]
                    if self.rng.next_val() % (count as u64) == 0 {
                        result = Some((x, y));
                    }
                }
            }
        }

        result
    }
}

// ============================================================================
// POI Placement Helpers
// ============================================================================

#[inline]
fn manhattan_distance(a: Position, b: Position) -> u16 {
    (a.x.abs_diff(b.x) as u16) + (a.y.abs_diff(b.y) as u16)
}

fn is_position_used(map: &GeneratedMap, pos: Position) -> bool {
    if pos.x == map.spawn_x && pos.y == map.spawn_y {
        return true;
    }

    if pos.x == map.mole_den_x && pos.y == map.mole_den_y {
        return true;
    }

    for idx in 0..map.poi_count as usize {
        let existing = map.pois[idx];
        if pos.x == existing.x && pos.y == existing.y {
            return true;
        }
    }

    false
}

fn is_same_type_spacing_valid(map: &GeneratedMap, pos: Position, poi_type: u8) -> bool {
    for idx in 0..map.poi_count as usize {
        let existing = map.pois[idx];
        if existing.poi_type == poi_type {
            let existing_pos = Position {
                x: existing.x,
                y: existing.y,
            };
            if manhattan_distance(pos, existing_pos) < POI_MIN_SPACING as u16 {
                return false;
            }
        }
    }

    true
}

fn find_valid_poi_position(
    walkable_tiles: &mut [Position],
    map: &GeneratedMap,
    poi_type: u8,
    rng: &mut SeededRNG,
) -> Option<Position> {
    rng.shuffle(walkable_tiles);

    for &pos in walkable_tiles.iter() {
        if is_position_used(map, pos) {
            continue;
        }

        if !is_same_type_spacing_valid(map, pos, poi_type) {
            continue;
        }

        return Some(pos);
    }

    None
}

fn find_reachable_poi_position(
    walkable_tiles: &mut [Position],
    map: &GeneratedMap,
    rng: &mut SeededRNG,
    poi_type: u8,
    max_distance: u8,
) -> Option<Position> {
    let spawn = Position {
        x: map.spawn_x,
        y: map.spawn_y,
    };

    rng.shuffle(walkable_tiles);

    for &pos in walkable_tiles.iter() {
        if is_position_used(map, pos) {
            continue;
        }

        if manhattan_distance(pos, spawn) > max_distance as u16 {
            continue;
        }

        if !is_same_type_spacing_valid(map, pos, poi_type) {
            continue;
        }

        return Some(pos);
    }

    None
}

fn place_poi_type_count(
    map: &mut GeneratedMap,
    rng: &mut SeededRNG,
    walkable_tiles: &mut [Position],
    poi_type: u8,
    count: usize,
) {
    for _ in 0..count {
        if map.poi_count as usize >= MAX_POIS {
            return;
        }

        if let Some(position) = find_valid_poi_position(walkable_tiles, map, poi_type, rng) {
            let index = map.poi_count as usize;
            map.pois[index] = PoiSpawn {
                poi_type,
                is_used: false,
                x: position.x,
                y: position.y,
            };
            map.poi_count += 1;
        }
    }
}

fn place_pois(map: &mut GeneratedMap, rng: &mut SeededRNG, campaign_level: u8) {
    map.poi_count = 0;

    let mut walkable_tiles: Vec<Position> = Vec::with_capacity(map.walkable_count as usize);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            if map.is_walkable(x, y) {
                walkable_tiles.push(Position { x, y });
            }
        }
    }

    if map.poi_count as usize >= MAX_POIS {
        return;
    }

    map.pois[0] = PoiSpawn {
        poi_type: 1,
        is_used: false,
        x: map.mole_den_x,
        y: map.mole_den_y,
    };
    map.poi_count = 1;

    if walkable_tiles.is_empty() {
        return;
    }

    let act_index = act_index_from_campaign_level(campaign_level);
    let mut targets = poi_target_counts_for_act(act_index, rng);
    if targets[1] > 0 {
        targets[1] = targets[1].saturating_sub(1);
    }

    // Ensure one reachable Counter Cache near spawn when available.
    if targets[13] > 0 && (map.poi_count as usize) < MAX_POIS {
        if let Some(position) = find_reachable_poi_position(&mut walkable_tiles, map, rng, 13, 30)
        {
            let index = map.poi_count as usize;
            map.pois[index] = PoiSpawn {
                poi_type: 13,
                is_used: false,
                x: position.x,
                y: position.y,
            };
            map.poi_count += 1;
            targets[13] = targets[13].saturating_sub(1);
        }
    }

    // Ensure one reachable Tool Crate near spawn when available.
    if targets[3] > 0 && (map.poi_count as usize) < MAX_POIS {
        if let Some(position) = find_reachable_poi_position(&mut walkable_tiles, map, rng, 3, 12)
        {
            let index = map.poi_count as usize;
            map.pois[index] = PoiSpawn {
                poi_type: 3,
                is_used: false,
                x: position.x,
                y: position.y,
            };
            map.poi_count += 1;
            targets[3] = targets[3].saturating_sub(1);
        }
    }

    // Ensure one reachable Supply Cache near spawn when available.
    if targets[2] > 0 && (map.poi_count as usize) < MAX_POIS {
        if let Some(position) = find_reachable_poi_position(&mut walkable_tiles, map, rng, 2, 12)
        {
            let index = map.poi_count as usize;
            map.pois[index] = PoiSpawn {
                poi_type: 2,
                is_used: false,
                x: position.x,
                y: position.y,
            };
            map.poi_count += 1;
            targets[2] = targets[2].saturating_sub(1);
        }
    }

    // Ensure one reachable Tool Oil Rack near spawn when available.
    if targets[4] > 0 && (map.poi_count as usize) < MAX_POIS {
        if let Some(position) = find_reachable_poi_position(&mut walkable_tiles, map, rng, 4, 12)
        {
            let index = map.poi_count as usize;
            map.pois[index] = PoiSpawn {
                poi_type: 4,
                is_used: false,
                x: position.x,
                y: position.y,
            };
            map.poi_count += 1;
            targets[4] = targets[4].saturating_sub(1);
        }
    }

    // Place specific POI types according to per-act targets.
    // Order prioritizes mandatory utilities before high-volume baseline spawns.
    const PLACEMENT_ORDER: [u8; 13] = [12, 11, 10, 9, 8, 7, 14, 13, 3, 6, 5, 4, 2];
    for poi_type in PLACEMENT_ORDER {
        let count = targets[poi_type as usize] as usize;
        place_poi_type_count(map, rng, &mut walkable_tiles, poi_type, count);
    }
}

// ============================================================================
// Enemy Placement Helpers
// ============================================================================

fn place_enemies(map: &mut GeneratedMap, rng: &mut SeededRNG, campaign_level: u8) {
    map.enemy_count = 0;

    if map.walkable_count == 0 {
        return;
    }

    let mut walkable_tiles: Vec<Position> = Vec::with_capacity(map.walkable_count as usize);
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            if map.is_walkable(x, y) {
                walkable_tiles.push(Position { x, y });
            }
        }
    }

    if walkable_tiles.is_empty() {
        return;
    }

    let mut occupied = [false; TOTAL_TILES];
    let mut mark_occupied = |pos: Position| {
        let index = (pos.y as usize) * (MAP_WIDTH as usize) + (pos.x as usize);
        occupied[index] = true;
    };

    mark_occupied(Position {
        x: map.spawn_x,
        y: map.spawn_y,
    });
    mark_occupied(Position {
        x: map.mole_den_x,
        y: map.mole_den_y,
    });

    for idx in 0..map.poi_count as usize {
        let poi = map.pois[idx];
        mark_occupied(Position { x: poi.x, y: poi.y });
    }

    let mut target_count = ACT_ENEMY_TARGETS[act_index_from_campaign_level(campaign_level)] as u16;
    if target_count as usize > MAX_ENEMIES {
        target_count = MAX_ENEMIES as u16;
    }

    rng.shuffle(&mut walkable_tiles);

    let spawn_position = Position {
        x: map.spawn_x,
        y: map.spawn_y,
    };

    for pos in walkable_tiles {
        if map.enemy_count as u16 >= target_count {
            break;
        }

        // Keep a safe zone around spawn — no enemies within SPAWN_PROTECTION_RADIUS
        let chebyshev = pos
            .x
            .abs_diff(spawn_position.x)
            .max(pos.y.abs_diff(spawn_position.y));
        if chebyshev <= SPAWN_PROTECTION_RADIUS {
            continue;
        }

        let index = (pos.y as usize) * (MAP_WIDTH as usize) + (pos.x as usize);
        if occupied[index] {
            continue;
        }

        let distance_pct = distance_percent_from_spawn(pos, spawn_position);
        let tier = select_tier_for_distance(distance_pct, campaign_level, rng);
        let tier_index = tier.saturating_sub(1);
        let pool = select_pool_for_distance(distance_pct, rng);
        let archetype_id = select_weighted_archetype_from_pool(rng, campaign_level, pool);

        let enemy_index = map.enemy_count as usize;
        map.enemies[enemy_index] = EnemySpawn {
            archetype_id,
            tier: tier_index,
            x: pos.x,
            y: pos.y,
        };
        map.enemy_count += 1;
        occupied[index] = true;
    }

    enforce_safe_start_easy_pool(map, rng, campaign_level);
}

/// Generate a complete map with the given seed and campaign level
/// Campaign level determines biome weighting for enemy spawning
pub fn generate_map(map: &mut GeneratedMap, seed: u64, campaign_level: u8) -> bool {
    // Initialize map dimensions
    map.width = MAP_WIDTH;
    map.height = MAP_HEIGHT;
    map.seed = seed;

    // Initialize all tiles as walls
    map.init_all_walls();

    // Generate the maze
    let mut generator = MazeGenerator::new(seed);
    generator.generate();

    // Carve corridors into the map
    generator.carve_into_map(map);

    // Count walkable tiles while finding spawn point
    let mut walkable_count: u16 = 0;
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            if map.is_walkable(x, y) {
                walkable_count += 1;
            }
        }
    }
    map.walkable_count = walkable_count;

    // Find spawn point using reservoir sampling
    match generator.find_spawn_point(map) {
        Some((x, y)) => {
            map.spawn_x = x;
            map.spawn_y = y;
            // Mole den is directly above spawn — convert wall to floor so
            // the den replaces the wall tile visually and is walkable.
            map.mole_den_x = x;
            map.mole_den_y = y - 1;
            map.set_floor(x, y - 1);
            map.walkable_count += 1;
        }
        None => {
            // Failed to find valid spawn point
            return false;
        }
    }

    place_pois(map, &mut generator.rng, campaign_level);

    place_enemies(map, &mut generator.rng, campaign_level);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::PACKED_TILES_SIZE;
    use std::collections::HashSet;

    fn create_test_map() -> GeneratedMap {
        GeneratedMap {
            session: anchor_lang::prelude::Pubkey::default(),
            width: MAP_WIDTH,
            height: MAP_HEIGHT,
            seed: 0,
            spawn_x: 0,
            spawn_y: 0,
            mole_den_x: 0,
            mole_den_y: 0,
            walkable_count: 0,
            packed_tiles: [0xFF; PACKED_TILES_SIZE],
            enemy_count: 0,
            enemies: [crate::state::EnemySpawn::default(); 48],
            poi_count: 0,
            pois: [crate::state::PoiSpawn::default(); MAX_POIS],
            bump: 0,
        }
    }

    fn count_pois_by_type(map: &GeneratedMap, poi_type: u8) -> usize {
        (0..map.poi_count as usize)
            .filter(|&idx| map.pois[idx].poi_type == poi_type)
            .count()
    }

    #[test]
    fn test_maze_generation_deterministic() {
        let mut map1 = create_test_map();
        let mut map2 = create_test_map();

        assert!(generate_map(&mut map1, 12345, 1));
        assert!(generate_map(&mut map2, 12345, 1));

        // Same seed should produce same map
        assert_eq!(map1.packed_tiles, map2.packed_tiles);
        assert_eq!(map1.spawn_x, map2.spawn_x);
        assert_eq!(map1.spawn_y, map2.spawn_y);
        assert_eq!(map1.walkable_count, map2.walkable_count);
    }

    #[test]
    fn test_maze_generation_different_seeds() {
        let mut map1 = create_test_map();
        let mut map2 = create_test_map();

        assert!(generate_map(&mut map1, 12345, 1));
        assert!(generate_map(&mut map2, 54321, 1));

        // Different seeds should produce different maps
        assert_ne!(map1.packed_tiles, map2.packed_tiles);
    }

    #[test]
    fn test_spawn_point_valid() {
        let mut map = create_test_map();
        assert!(generate_map(&mut map, 42, 1));

        // Spawn should be on a floor tile
        assert!(map.is_walkable(map.spawn_x, map.spawn_y));

        // Mole den should be directly above spawn (converted to floor tile)
        assert_eq!(map.mole_den_x, map.spawn_x);
        assert_eq!(map.mole_den_y, map.spawn_y - 1);
        assert!(map.is_walkable(map.mole_den_x, map.mole_den_y));
    }

    #[test]
    fn test_walkable_count_positive() {
        let mut map = create_test_map();
        assert!(generate_map(&mut map, 42, 1));

        // Should have some walkable tiles
        assert!(map.walkable_count > 0);

        // Verify count matches actual walkable tiles
        let mut count = 0;
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if map.is_walkable(x, y) {
                    count += 1;
                }
            }
        }
        assert_eq!(map.walkable_count, count);
    }

    #[test]
    fn test_tile_bit_packing() {
        let mut map = create_test_map();
        map.init_all_walls();

        // Initially all walls
        assert!(!map.is_walkable(0, 0));
        assert!(!map.is_walkable(25, 25));

        // Set some floors
        map.set_floor(0, 0);
        map.set_floor(25, 25);

        assert!(map.is_walkable(0, 0));
        assert!(map.is_walkable(25, 25));

        // Set back to wall
        map.set_wall(0, 0);
        assert!(!map.is_walkable(0, 0));
        assert!(map.is_walkable(25, 25)); // 25,25 should still be floor
    }

    #[test]
    fn test_direction_opposite() {
        assert_eq!(Direction::North.opposite(), Direction::South);
        assert_eq!(Direction::South.opposite(), Direction::North);
        assert_eq!(Direction::East.opposite(), Direction::West);
        assert_eq!(Direction::West.opposite(), Direction::East);
    }

    #[test]
    fn test_cell_grid_constants() {
        // Verify our constants are correct
        assert_eq!(CELL_GRID_WIDTH, 12);
        assert_eq!(CELL_GRID_HEIGHT, 12);
        assert_eq!(CELL_GRID_SIZE, 144);
        assert_eq!(CELL_SPACING, 4);
    }

    #[test]
    fn test_pack_unpack_coords() {
        for cx in 0..12u8 {
            for cy in 0..12u8 {
                let packed = MazeGenerator::pack_coords(cx, cy);
                let (ux, uy) = MazeGenerator::unpack_coords(packed);
                assert_eq!(cx, ux);
                assert_eq!(cy, uy);
            }
        }
    }

    #[test]
    fn test_poi_placement_invariants() {
        let mut map = create_test_map();
        assert!(generate_map(&mut map, 4242, 1));

        assert!(map.poi_count > 0);
        assert!((map.poi_count as usize) <= MAX_POIS);

        let first_poi = map.pois[0];
        assert_eq!(first_poi.poi_type, 1);
        assert_eq!(first_poi.x, map.mole_den_x);
        assert_eq!(first_poi.y, map.mole_den_y);

        let mut seen_positions = HashSet::new();
        for idx in 0..map.poi_count as usize {
            let poi = map.pois[idx];
            assert!(poi.x < MAP_WIDTH);
            assert!(poi.y < MAP_HEIGHT);
            assert!(!poi.is_used);

            let key = (poi.x, poi.y);
            assert!(
                seen_positions.insert(key),
                "Duplicate POI position at {key:?}"
            );

            // All POIs (including mole den) should be on floor tiles
            assert!(map.is_walkable(poi.x, poi.y));
            if idx > 0 {
                assert!(poi.x != map.spawn_x || poi.y != map.spawn_y);
            }
        }

        for i in 0..map.poi_count as usize {
            let poi_a = map.pois[i];
            for j in (i + 1)..map.poi_count as usize {
                let poi_b = map.pois[j];
                if poi_a.poi_type == poi_b.poi_type {
                    let distance = manhattan_distance(
                        Position {
                            x: poi_a.x,
                            y: poi_a.y,
                        },
                        Position {
                            x: poi_b.x,
                            y: poi_b.y,
                        },
                    );
                    assert!(
                        distance >= POI_MIN_SPACING as u16,
                        "POI type {} placed too close: {distance}",
                        poi_a.poi_type
                    );
                }
            }
        }
    }

    #[test]
    fn test_poi_placement_deterministic() {
        let mut map1 = create_test_map();
        let mut map2 = create_test_map();

        assert!(generate_map(&mut map1, 9876, 1));
        assert!(generate_map(&mut map2, 9876, 1));

        assert_eq!(map1.poi_count, map2.poi_count);

        for idx in 0..map1.poi_count as usize {
            let poi_a = map1.pois[idx];
            let poi_b = map2.pois[idx];
            assert_eq!(poi_a.poi_type, poi_b.poi_type);
            assert_eq!(poi_a.x, poi_b.x);
            assert_eq!(poi_a.y, poi_b.y);
        }
    }

    #[test]
    fn test_enemy_placement_invariants() {
        let mut map = create_test_map();
        assert!(generate_map(&mut map, 8888, 1));

        assert!(map.enemy_count > 0);
        assert!((map.enemy_count as usize) <= MAX_ENEMIES);

        let mut seen_positions = HashSet::new();
        for idx in 0..map.enemy_count as usize {
            let enemy = map.enemies[idx];
            assert!(enemy.x < MAP_WIDTH);
            assert!(enemy.y < MAP_HEIGHT);
            assert!(enemy.archetype_id < 12); // 12 field enemies per GDD
            assert!(enemy.tier <= 2);
            assert!(map.is_walkable(enemy.x, enemy.y));

            let key = (enemy.x, enemy.y);
            assert!(
                seen_positions.insert(key),
                "Duplicate enemy position at {key:?}"
            );
            assert!(enemy.x != map.spawn_x || enemy.y != map.spawn_y);
            assert!(enemy.x != map.mole_den_x || enemy.y != map.mole_den_y);

            for poi_index in 0..map.poi_count as usize {
                let poi = map.pois[poi_index];
                assert!(enemy.x != poi.x || enemy.y != poi.y);
            }
        }
    }

    #[test]
    fn test_enemy_placement_deterministic() {
        let mut map1 = create_test_map();
        let mut map2 = create_test_map();

        assert!(generate_map(&mut map1, 1234, 1));
        assert!(generate_map(&mut map2, 1234, 1));

        assert_eq!(map1.enemy_count, map2.enemy_count);
        for idx in 0..map1.enemy_count as usize {
            let enemy_a = map1.enemies[idx];
            let enemy_b = map2.enemies[idx];
            assert_eq!(enemy_a.archetype_id, enemy_b.archetype_id);
            assert_eq!(enemy_a.tier, enemy_b.tier);
            assert_eq!(enemy_a.x, enemy_b.x);
            assert_eq!(enemy_a.y, enemy_b.y);
        }
    }

    #[test]
    fn test_enemy_targets_match_gdd_per_act() {
        let expected = [(1u8, 36u8), (11u8, 40u8), (21u8, 44u8), (31u8, 48u8)];

        for (campaign_level, expected_count) in expected {
            let mut map = create_test_map();
            assert!(generate_map(
                &mut map,
                9000 + campaign_level as u64,
                campaign_level
            ));
            assert_eq!(
                map.enemy_count, expected_count,
                "Enemy count mismatch for campaign level {}",
                campaign_level
            );
        }
    }

    #[test]
    fn test_poi_targets_match_gdd_per_act() {
        let levels = [1u8, 11u8, 21u8, 31u8];
        // Higher acts = fewer POIs (harder scarcity)
        let expected_l2 = [16usize, 14, 14, 10];
        let expected_l3 = [5usize, 4, 4, 2];
        let expected_l4 = [5usize, 4, 4, 3];
        let expected_l5 = [6usize, 5, 5, 4];
        let expected_l6 = [4usize, 4, 4, 3];
        let expected_l7 = [3usize, 3, 3, 2];
        let expected_l8 = [5usize, 4, 4, 2];
        let expected_l9 = [2usize, 2, 2, 1];
        let expected_l10 = [2usize, 2, 2, 1];
        let expected_l11 = [2usize, 1, 1, 1];
        let expected_l12 = [2usize, 1, 1, 1];
        let expected_l14 = [3usize, 2, 2, 1];

        for (act_idx, campaign_level) in levels.iter().copied().enumerate() {
            let mut map = create_test_map();
            assert!(generate_map(
                &mut map,
                12000 + campaign_level as u64,
                campaign_level
            ));

            assert_eq!(count_pois_by_type(&map, 1), 1, "L1 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 2), expected_l2[act_idx], "L2 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 3), expected_l3[act_idx], "L3 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 4), expected_l4[act_idx], "L4 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 5), expected_l5[act_idx], "L5 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 6), expected_l6[act_idx], "L6 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 7), expected_l7[act_idx], "L7 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 8), expected_l8[act_idx], "L8 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 9), expected_l9[act_idx], "L9 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 10), expected_l10[act_idx], "L10 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 11), expected_l11[act_idx], "L11 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 12), expected_l12[act_idx], "L12 count mismatch act {}", act_idx + 1);
            assert_eq!(count_pois_by_type(&map, 14), expected_l14[act_idx], "L14 count mismatch act {}", act_idx + 1);

            assert_eq!(count_pois_by_type(&map, 13), 2, "L13 count mismatch act {}", act_idx + 1);
        }
    }

    #[test]
    fn test_safe_start_first_three_enemies_are_easy_pool() {
        let mut map = create_test_map();
        assert!(generate_map(&mut map, 2222, 1));

        let spawn = Position {
            x: map.spawn_x,
            y: map.spawn_y,
        };
        let mut enemies: Vec<(u16, u8)> = (0..map.enemy_count as usize)
            .map(|idx| {
                let enemy = map.enemies[idx];
                (
                    manhattan_distance(
                        Position {
                            x: enemy.x,
                            y: enemy.y,
                        },
                        spawn,
                    ),
                    enemy.archetype_id,
                )
            })
            .collect();

        enemies.sort_by_key(|(dist, _)| *dist);
        for (_, archetype_id) in enemies.into_iter().take(3) {
            assert!(
                EASY_POOL.contains(&archetype_id),
                "Expected easy-pool archetype in first three encounters, got {}",
                archetype_id
            );
        }
    }

    #[test]
    fn test_counter_cache_reachable_within_30_moves() {
        let mut map = create_test_map();
        assert!(generate_map(&mut map, 3333, 1));

        let spawn = Position {
            x: map.spawn_x,
            y: map.spawn_y,
        };
        let has_reachable_counter_cache = (0..map.poi_count as usize).any(|idx| {
            let poi = map.pois[idx];
            poi.poi_type == 13
                && manhattan_distance(
                    Position { x: poi.x, y: poi.y },
                    Position {
                        x: spawn.x,
                        y: spawn.y,
                    },
                ) <= 30
        });

        assert!(
            has_reachable_counter_cache,
            "Expected at least one Counter Cache (type 13) within 30 moves"
        );
    }

    #[test]
    fn test_biome_a_weighting() {
        // 40 levels total: 4 acts × 10 levels each
        // Act 1 (levels 1-10) should be Biome A
        assert!(is_biome_a(1));
        assert!(is_biome_a(5));
        assert!(is_biome_a(10));
        // Act 2 (levels 11-20) should be Biome B
        assert!(!is_biome_a(11));
        assert!(!is_biome_a(15));
        assert!(!is_biome_a(20));
        // Act 3 (levels 21-30) should be Biome A
        assert!(is_biome_a(21));
        assert!(is_biome_a(25));
        assert!(is_biome_a(30));
        // Act 4 (levels 31-40) should be Biome B
        assert!(!is_biome_a(31));
        assert!(!is_biome_a(35));
        assert!(!is_biome_a(40));
    }

    #[test]
    fn test_biome_weighted_enemy_distribution() {
        // Generate many enemies in Biome A and check that biome A enemies are more common
        let mut biome_a_enemy_count = [0u32; 12];
        let mut biome_b_enemy_count = [0u32; 12];

        for seed in 0..100u64 {
            let mut map_a = create_test_map();
            let mut map_b = create_test_map();

            assert!(generate_map(&mut map_a, seed, 1)); // Biome A (Act 1)
            assert!(generate_map(&mut map_b, seed, 11)); // Biome B (Act 2)

            for idx in 0..map_a.enemy_count as usize {
                biome_a_enemy_count[map_a.enemies[idx].archetype_id as usize] += 1;
            }
            for idx in 0..map_b.enemy_count as usize {
                biome_b_enemy_count[map_b.enemies[idx].archetype_id as usize] += 1;
            }
        }

        // Biome A should have more Tunnel Rats (0), Collapsed Miners (4), Shard Beetles (5), Coin Slugs (10)
        // than Biome B
        let biome_a_emphasized = [0, 4, 5, 10];
        for &id in &biome_a_emphasized {
            assert!(
                biome_a_enemy_count[id] > biome_b_enemy_count[id],
                "Archetype {} should be more common in Biome A ({} vs {})",
                id,
                biome_a_enemy_count[id],
                biome_b_enemy_count[id]
            );
        }

        // Biome B should have more Rust Mites (3), Burrow Ambushers (7), Frost Wisps (8),
        // Powder Ticks (9), Blood Mosquitos (11)
        let biome_b_emphasized = [3, 7, 8, 9, 11];
        for &id in &biome_b_emphasized {
            assert!(
                biome_b_enemy_count[id] > biome_a_enemy_count[id],
                "Archetype {} should be more common in Biome B ({} vs {})",
                id,
                biome_b_enemy_count[id],
                biome_a_enemy_count[id]
            );
        }
    }

    #[test]
    fn test_guaranteed_pois_near_spawn_act1() {
        // Verify that Tool Crate (L3), Supply Cache (L2), and Tool Oil Rack (L4)
        // each have at least one instance within their guaranteed distance of spawn.
        let max_dist_l3 = 12u16;
        let max_dist_l2 = 12u16;
        let max_dist_l4 = 12u16;

        for seed in 1000u64..1200u64 {
            let mut map = create_test_map();
            assert!(generate_map(&mut map, seed, 1)); // Campaign level 1 = Act 1

            let spawn = Position {
                x: map.spawn_x,
                y: map.spawn_y,
            };

            let mut closest_l2 = u16::MAX;
            let mut closest_l3 = u16::MAX;
            let mut closest_l4 = u16::MAX;

            for idx in 0..map.poi_count as usize {
                let poi = map.pois[idx];
                let poi_pos = Position {
                    x: poi.x,
                    y: poi.y,
                };
                let dist = manhattan_distance(poi_pos, spawn);

                match poi.poi_type {
                    2 => closest_l2 = closest_l2.min(dist),
                    3 => closest_l3 = closest_l3.min(dist),
                    4 => closest_l4 = closest_l4.min(dist),
                    _ => {}
                }
            }

            assert!(
                closest_l3 <= max_dist_l3,
                "Seed {}: closest Tool Crate (L3) is {} tiles from spawn (max {})",
                seed, closest_l3, max_dist_l3
            );
            assert!(
                closest_l2 <= max_dist_l2,
                "Seed {}: closest Supply Cache (L2) is {} tiles from spawn (max {})",
                seed, closest_l2, max_dist_l2
            );
            assert!(
                closest_l4 <= max_dist_l4,
                "Seed {}: closest Tool Oil Rack (L4) is {} tiles from spawn (max {})",
                seed, closest_l4, max_dist_l4
            );
        }
    }
}
