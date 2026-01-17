use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod rng;
pub mod state;

use constants::*;
use errors::MapGeneratorError;
use state::{MapConfig, SeedUpdate};

declare_id!("BYdGuEGf8NqtLnHpSRuZFrPGEgvdxMfGfTt71QVBxYHa");

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

    /// Updates the seed for a single level (admin only).
    pub fn update_map_config(ctx: Context<UpdateMapConfig>, level: u8, seed: u64) -> Result<()> {
        require!(level <= MAX_LEVEL, MapGeneratorError::InvalidLevel);

        let config = &mut ctx.accounts.map_config;
        let old_seed = config.seeds[level as usize];
        config.seeds[level as usize] = seed;

        let clock = Clock::get()?;
        emit!(MapConfigUpdated {
            admin: config.admin,
            level,
            old_seed,
            new_seed: seed,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Batch updates multiple seed mappings (admin only).
    pub fn batch_update_map_config(
        ctx: Context<UpdateMapConfig>,
        updates: Vec<SeedUpdate>,
    ) -> Result<()> {
        require!(
            updates.len() <= MAX_BATCH_UPDATES,
            MapGeneratorError::TooManyUpdates
        );

        let config = &mut ctx.accounts.map_config;
        let clock = Clock::get()?;

        for update in updates {
            require!(update.level <= MAX_LEVEL, MapGeneratorError::InvalidLevel);

            let old_seed = config.seeds[update.level as usize];
            config.seeds[update.level as usize] = update.seed;

            emit!(MapConfigUpdated {
                admin: config.admin,
                level: update.level,
                old_seed,
                new_seed: update.seed,
                timestamp: clock.unix_timestamp,
            });
        }

        Ok(())
    }

    /// Returns the seed for a given level.
    /// This is a view function - state is not modified.
    pub fn get_map_seed(ctx: Context<GetMapSeed>, level: u8) -> Result<u64> {
        require!(level <= MAX_LEVEL, MapGeneratorError::InvalidLevel);

        let config = &ctx.accounts.map_config;
        let seed = config.seeds[level as usize];

        msg!("Level {} seed: {}", level, seed);
        Ok(seed)
    }

    /// Verifies that a submitted map hash matches the expected hash.
    /// Used to validate off-chain map generation.
    pub fn verify_map_hash(
        ctx: Context<VerifyMapHash>,
        level: u8,
        submitted_hash: [u8; 32],
    ) -> Result<bool> {
        require!(level <= MAX_LEVEL, MapGeneratorError::InvalidLevel);

        let config = &ctx.accounts.map_config;
        let seed = config.seeds[level as usize];

        // Create expected hash from seed using simple hash
        // In production, this would use the same hash algorithm as client
        let expected_hash = compute_map_hash(seed);

        let matches = submitted_hash == expected_hash;
        msg!(
            "Map hash verification for level {}: {}",
            level,
            if matches { "PASS" } else { "FAIL" }
        );

        if !matches {
            return Err(MapGeneratorError::InvalidMapHash.into());
        }

        Ok(true)
    }

    /// Transfers admin authority to a new address.
    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.map_config;
        let old_admin = config.admin;
        config.admin = new_admin;

        emit!(AdminTransferred {
            old_admin,
            new_admin,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Computes a deterministic hash from a seed value.
/// Uses the same XorShift RNG as map generation for consistency.
fn compute_map_hash(seed: u64) -> [u8; 32] {
    use crate::rng::SeededRNG;

    let mut rng = SeededRNG::new(seed);
    let mut hash = [0u8; 32];

    // Generate 32 bytes from RNG (4 u64 values = 32 bytes)
    for i in 0..4 {
        let value = rng.next();
        let bytes = value.to_le_bytes();
        hash[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
    }

    hash
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
pub struct UpdateMapConfig<'info> {
    #[account(
        mut,
        seeds = [MapConfig::SEED_PREFIX],
        bump = map_config.bump,
        has_one = admin @ MapGeneratorError::Unauthorized
    )]
    pub map_config: Account<'info, MapConfig>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct GetMapSeed<'info> {
    #[account(
        seeds = [MapConfig::SEED_PREFIX],
        bump = map_config.bump
    )]
    pub map_config: Account<'info, MapConfig>,
}

#[derive(Accounts)]
pub struct VerifyMapHash<'info> {
    #[account(
        seeds = [MapConfig::SEED_PREFIX],
        bump = map_config.bump
    )]
    pub map_config: Account<'info, MapConfig>,
}

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    #[account(
        mut,
        seeds = [MapConfig::SEED_PREFIX],
        bump = map_config.bump,
        has_one = admin @ MapGeneratorError::Unauthorized
    )]
    pub map_config: Account<'info, MapConfig>,

    pub admin: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct MapConfigUpdated {
    pub admin: Pubkey,
    pub level: u8,
    pub old_seed: u64,
    pub new_seed: u64,
    pub timestamp: i64,
}

#[event]
pub struct AdminTransferred {
    pub old_admin: Pubkey,
    pub new_admin: Pubkey,
    pub timestamp: i64,
}
