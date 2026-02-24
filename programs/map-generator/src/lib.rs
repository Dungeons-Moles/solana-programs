use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{commit, delegate, ephemeral};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

pub mod constants;
pub mod errors;
pub mod maze;
pub mod rng;
pub mod state;

use constants::*;
use errors::MapGeneratorError;
use state::{GeneratedMap, MapConfig};

declare_id!("GCy5GqvnJN99rgGtV6fMn8NtL9E7RoAyHDGzQv8me65j");

/// Gameplay state program ID for authorized tile modifications (wall breaking)
pub const GAMEPLAY_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xa5, 0x69, 0x33, 0xc3, 0x32, 0x44, 0x5d, 0xb7, 0x52, 0x8d, 0x7a, 0x6b, 0xc3, 0x01, 0x56, 0x1e,
    0x68, 0x50, 0xaa, 0x96, 0x7a, 0x85, 0xea, 0x62, 0xb5, 0x79, 0xe3, 0x23, 0xe4, 0xa8, 0x88, 0x36,
]);

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x58, 0x20, 0x64, 0x87, 0xdf, 0xd8, 0x68, 0xf1, 0xa4, 0x79, 0x15, 0x8b, 0xb2, 0x8a, 0x56, 0x0c,
    0xa9, 0x4f, 0x56, 0x2e, 0x62, 0x85, 0x26, 0xb7, 0x4f, 0x8b, 0xa1, 0x4d, 0x08, 0x36, 0x20, 0x99,
]);
pub const LOCAL_ER_VALIDATOR: Pubkey = pubkey!("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev");

fn local_delegate_config() -> DelegateConfig {
    DelegateConfig {
        validator: Some(LOCAL_ER_VALIDATOR),
        ..DelegateConfig::default()
    }
}

#[ephemeral]
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

    /// Generates a map for a game session using an explicit seed.
    /// Used by Duels so seed selection is decoupled from campaign progression.
    pub fn generate_map_with_seed(
        ctx: Context<GenerateMap>,
        campaign_level: u8,
        seed: u64,
    ) -> Result<()> {
        require!(
            campaign_level > 0 && campaign_level <= MAX_LEVEL,
            MapGeneratorError::InvalidLevel
        );

        let generated_map = &mut ctx.accounts.generated_map;
        generated_map.session = ctx.accounts.session.key();
        generated_map.bump = ctx.bumps.generated_map;

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

    /// Closes the GeneratedMap account, returning rent to player.
    /// Used by session-manager CPI during end_session to clean up.
    ///
    /// Authorization: Reads session account to verify session_signer matches signer,
    /// then returns rent to session.player.
    pub fn close_generated_map(ctx: Context<CloseGeneratedMap>) -> Result<()> {
        /// Byte offset of `player` in GameSession account data.
        /// Must match session_manager::state::GameSession layout.
        const SESSION_PLAYER_OFFSET: usize = 8;
        /// Byte offset of `session_signer` in GameSession account data.
        /// Keep in sync with session_manager::state::GameSession::SESSION_SIGNER_OFFSET.
        const SESSION_SESSION_SIGNER_OFFSET: usize = 77;

        let session_data = ctx.accounts.session.try_borrow_data()?;
        require!(
            session_data.len() >= SESSION_SESSION_SIGNER_OFFSET + 32,
            MapGeneratorError::InvalidSession
        );

        let stored_session_signer = Pubkey::from(
            <[u8; 32]>::try_from(
                &session_data[SESSION_SESSION_SIGNER_OFFSET..SESSION_SESSION_SIGNER_OFFSET + 32],
            )
            .unwrap(),
        );
        require!(
            stored_session_signer == ctx.accounts.session_signer.key(),
            MapGeneratorError::Unauthorized
        );

        let stored_player = Pubkey::from(
            <[u8; 32]>::try_from(&session_data[SESSION_PLAYER_OFFSET..SESSION_PLAYER_OFFSET + 32])
                .unwrap(),
        );
        require!(
            stored_player == ctx.accounts.player.key(),
            MapGeneratorError::Unauthorized
        );

        drop(session_data);

        emit!(GeneratedMapClosed {
            session: ctx.accounts.generated_map.session,
        });

        Ok(())
    }

    /// Delegates generated-map PDA to MagicBlock from its owning program.
    pub fn delegate_generated_map(ctx: Context<DelegateGeneratedMap>) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected_generated_map, _) = Pubkey::find_program_address(
            &[GeneratedMap::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.generated_map.key(),
            expected_generated_map,
            MapGeneratorError::Unauthorized
        );
        let map_seeds: &[&[u8]] = &[GeneratedMap::SEED_PREFIX, session_key.as_ref()];
        ctx.accounts.delegate_generated_map(
            &ctx.accounts.player,
            map_seeds,
            local_delegate_config(),
        )?;
        Ok(())
    }

    /// Commits and undelegates generated-map PDA from ER back to base layer.
    pub fn undelegate_generated_map(ctx: Context<UndelegateGeneratedMap>) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected_generated_map, _) = Pubkey::find_program_address(
            &[GeneratedMap::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.generated_map.key(),
            expected_generated_map,
            MapGeneratorError::Unauthorized
        );
        let generated_map = read_generated_map(&ctx.accounts.generated_map)?;
        require_keys_eq!(
            generated_map.session,
            session_key,
            MapGeneratorError::Unauthorized
        );

        let generated_map_info = ctx.accounts.generated_map.to_account_info();
        commit_and_undelegate_accounts(
            &ctx.accounts.session_signer.to_account_info(),
            vec![&generated_map_info],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
        )?;
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
    /// CHECK: Ownership is validated by constraint; PDA relationship is enforced by seeds on generated_map.
    #[account(
        owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner
    )]
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

#[delegate]
#[derive(Accounts)]
pub struct DelegateGeneratedMap<'info> {
    #[account(mut, del)]
    /// CHECK: PDA is validated via explicit seed check in handler.
    pub generated_map: AccountInfo<'info>,
    /// CHECK: Session PDA owned by session-manager; used only for seed derivation.
    pub session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
pub struct UndelegateGeneratedMap<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated and deserialized in handler.
    pub generated_map: AccountInfo<'info>,
    /// CHECK: Session PDA used only for deterministic PDA validation.
    pub session: UncheckedAccount<'info>,
    pub session_signer: Signer<'info>,
}

fn read_generated_map(generated_map: &AccountInfo<'_>) -> Result<GeneratedMap> {
    let data = generated_map.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    GeneratedMap::try_deserialize(&mut slice).map_err(|_| MapGeneratorError::InvalidSession.into())
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

/// Context for closing GeneratedMap account via session key signer.
#[derive(Accounts)]
pub struct CloseGeneratedMap<'info> {
    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session,
        close = player,
    )]
    pub generated_map: Account<'info, GeneratedMap>,

    /// Game session PDA to verify session_signer authorization
    /// CHECK: Session account is validated manually in instruction
    pub session: UncheckedAccount<'info>,

    /// Player wallet receives the rent refund (not a signer)
    /// CHECK: Validated against session.player in instruction
    #[account(mut)]
    pub player: AccountInfo<'info>,

    /// Session key signer must sign to authorize closure
    pub session_signer: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct GeneratedMapClosed {
    pub session: Pubkey,
}
