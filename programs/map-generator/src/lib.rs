use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod maze;
pub mod rng;
pub mod state;

use constants::*;
use errors::MapGeneratorError;
use state::{GeneratedMap, MapConfig};

declare_id!("BYdGuEGf8NqtLnHpSRuZFrPGEgvdxMfGfTt71QVBxYHa");

/// Gameplay state program ID for authorized tile modifications (wall breaking)
/// Derived from "5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n"
pub const GAMEPLAY_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    66, 165, 213, 208, 125, 103, 44, 88, 115, 217, 192, 197, 1, 117, 7, 170, 78, 32, 208, 143, 119,
    94, 47, 124, 229, 196, 47, 149, 235, 227, 237, 31,
]);

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    217, 18, 17, 128, 79, 140, 152, 73, 103, 95, 134, 179, 31, 109, 34, 82, 250, 167, 91, 67, 186,
    23, 209, 2, 80, 255, 118, 192, 175, 242, 222, 183,
]);

#[program]
pub mod map_generator {
    use super::*;

    /// Initializes the map configuration with default seed mappings.
    /// Each level i gets seed value i as default.
    pub fn initialize_map_config(ctx: Context<InitializeMapConfig>) -> Result<()> {
        let config = &mut ctx.accounts.map_config;

        config.admin = ctx.accounts.admin.key();
        config.seeds = DEFAULT_SEEDS;
        config.version = 1;
        config.bump = ctx.bumps.map_config;

        Ok(())
    }

    /// Generates a map for a game session based on the campaign level.
    /// Uses the seed from map_config for the given level.
    pub fn generate_map(ctx: Context<GenerateMap>, campaign_level: u8) -> Result<()> {
        // Validate campaign level
        require!(
            campaign_level > 0 && campaign_level <= MAX_LEVEL,
            MapGeneratorError::InvalidLevel
        );

        let map_config = &ctx.accounts.map_config;
        let generated_map = &mut ctx.accounts.generated_map;

        // Get seed for this level (1-indexed, array is 0-indexed)
        let seed = map_config.seeds[(campaign_level - 1) as usize];

        // Set session reference
        generated_map.session = ctx.accounts.session.key();
        generated_map.bump = ctx.bumps.generated_map;

        // Generate the maze with biome-weighted enemy spawning
        let success = maze::generate_map(generated_map, seed, campaign_level);
        require!(success, MapGeneratorError::MapGenerationFailed);

        Ok(())
    }

    /// Marks a POI as used on the generated map.
    pub fn mark_poi_used(ctx: Context<MarkPoiUsed>, poi_index: u8) -> Result<()> {
        require_keys_eq!(
            *ctx.accounts.session.owner,
            SESSION_MANAGER_PROGRAM_ID,
            MapGeneratorError::InvalidSessionOwner
        );

        let generated_map = &mut ctx.accounts.generated_map;

        require!(
            poi_index < generated_map.poi_count,
            MapGeneratorError::InvalidPoiIndex
        );

        let index = poi_index as usize;
        generated_map.pois[index].is_used = true;

        Ok(())
    }

    /// Converts a wall tile to a floor tile, authorized by gameplay-state.
    ///
    /// This instruction is called via CPI from gameplay-state when a player
    /// breaks through a wall. The change persists for the entire session,
    /// so future movement to this tile costs only 1 move (floor cost).
    ///
    /// Authorization: Requires gameplay_authority PDA as signer.
    pub fn set_tile_floor(ctx: Context<SetTileFloor>, x: u8, y: u8) -> Result<()> {
        let generated_map = &mut ctx.accounts.generated_map;

        require!(
            x < generated_map.width && y < generated_map.height,
            MapGeneratorError::TileOutOfBounds
        );

        generated_map.set_floor(x, y);

        Ok(())
    }
}

// ============================================================================
// Account Contexts
// ============================================================================

#[derive(Accounts)]
pub struct InitializeMapConfig<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + MapConfig::INIT_SPACE,
        seeds = [MapConfig::SEED_PREFIX],
        bump
    )]
    pub map_config: Account<'info, MapConfig>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GenerateMap<'info> {
    /// Payer for rent
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Game session PDA reference (validated externally)
    /// CHECK: Session account is validated by the caller
    pub session: UncheckedAccount<'info>,

    /// Map configuration with seeds
    #[account(
        seeds = [MapConfig::SEED_PREFIX],
        bump = map_config.bump
    )]
    pub map_config: Account<'info, MapConfig>,

    /// Generated map output
    #[account(
        init,
        payer = payer,
        space = GeneratedMap::SPACE,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump
    )]
    pub generated_map: Account<'info, GeneratedMap>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MarkPoiUsed<'info> {
    /// Generated map output
    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session
    )]
    pub generated_map: Account<'info, GeneratedMap>,

    /// Game session PDA reference (validated by owner + has_one)
    /// CHECK: Session account is validated by owner check
    pub session: UncheckedAccount<'info>,
}

/// Context for setting a tile as floor, authorized by gameplay-state via CPI.
/// Uses gameplay_authority PDA from gameplay-state as signer.
#[derive(Accounts)]
pub struct SetTileFloor<'info> {
    /// Generated map to modify
    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session
    )]
    pub generated_map: Account<'info, GeneratedMap>,

    /// Game session PDA reference (validated by has_one)
    /// CHECK: Session account is validated by has_one constraint
    pub session: UncheckedAccount<'info>,

    /// Gameplay authority PDA from gameplay-state that must sign
    /// This ensures only gameplay-state can call this instruction
    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = GAMEPLAY_STATE_PROGRAM_ID,
    )]
    pub gameplay_authority: Signer<'info>,
}
