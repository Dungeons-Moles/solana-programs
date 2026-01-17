use crate::constants::LEVEL_COUNT;
use anchor_lang::prelude::*;

/// Configuration for map generation, storing seed values for each level.
#[account]
#[derive(InitSpace)]
pub struct MapConfig {
    /// Authority that can update seed mappings
    pub admin: Pubkey,
    /// Seed values for campaign levels 0-80
    #[max_len(81)]
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
    /// Level to update (0-80)
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
    Gravel = 2,
    Water = 3,
}

/// Position on the map grid
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct Position {
    pub x: u8,
    pub y: u8,
}
