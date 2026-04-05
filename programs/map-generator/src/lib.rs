use anchor_lang::prelude::*;
use er_compat::DelegateConfig;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;

pub mod constants;
pub mod errors;
pub mod maze;
pub mod rng;
pub mod state;

use constants::*;
use errors::MapGeneratorError;
use state::{GeneratedMap, MapConfig, MapVrfState, SessionDiscovery};
use vrf_rng::VrfStatus;

declare_id!("E6kc5Edg1s3AXVQQFRoYdAq4vPAFbkYbP7B5ujiuZwz4");

/// Gameplay state program ID for authorized tile modifications (wall breaking)
pub const GAMEPLAY_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x2a, 0x85, 0x94, 0xcf, 0xca, 0x5f, 0x00, 0x45, 0x30, 0xce, 0x64, 0xd1, 0x54, 0x94, 0x6b, 0x36,
    0xcb, 0xd4, 0x94, 0x56, 0x16, 0x97, 0xa1, 0x82, 0x0d, 0x72, 0x1b, 0x7e, 0x89, 0xb7, 0xbf, 0x7e,
]);

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xb0, 0x1c, 0x9d, 0x6a, 0x40, 0xc6, 0xe0, 0xa9, 0xe4, 0xaf, 0xa9, 0xa9, 0xd9, 0xad, 0x02, 0x15,
    0x89, 0xbd, 0xf1, 0x36, 0x79, 0x88, 0x02, 0x94, 0xc2, 0x24, 0x9f, 0xd9, 0xa4, 0x21, 0xd7, 0x39,
]);
fn local_delegate_config(validator: Option<Pubkey>) -> DelegateConfig {
    DelegateConfig {
        validator: validator.map(|v| unsafe { std::mem::transmute(v) }),
        ..DelegateConfig::default()
    }
}

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
        let success = maze::generate_map(generated_map, seed, campaign_level, false);
        require!(success, MapGeneratorError::MapGenerationFailed);
        generated_map.clear_discovery();
        let spawn_x = generated_map.spawn_x;
        let spawn_y = generated_map.spawn_y;
        generated_map.reveal_radius(spawn_x, spawn_y, 6);

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

        let success = maze::generate_map(generated_map, seed, campaign_level, false);
        require!(success, MapGeneratorError::MapGenerationFailed);
        generated_map.clear_discovery();
        let spawn_x = generated_map.spawn_x;
        let spawn_y = generated_map.spawn_y;
        generated_map.reveal_radius(spawn_x, spawn_y, 6);

        Ok(())
    }

    /// Allocates an empty GeneratedMap account without generating any maze content.
    /// Called via CPI from session-manager during session start so the account
    /// exists and can be delegated to ER. Actual maze generation happens on ER
    /// via fill_map_with_seed (PvE) or generate_map_with_vrf (Gauntlet/Duel).
    pub fn init_map_account(ctx: Context<InitMapAccount>, campaign_level: u8) -> Result<()> {
        require!(
            campaign_level > 0 && campaign_level <= MAX_LEVEL,
            MapGeneratorError::InvalidLevel
        );

        let generated_map = &mut ctx.accounts.generated_map;
        generated_map.session = ctx.accounts.session.key();
        generated_map.bump = ctx.bumps.generated_map;
        generated_map.clear_discovery();
        // All other fields (tiles, enemies, pois) remain zeroed by Anchor account init.
        // Width/height/spawn will be set by fill_map_with_seed or generate_map_with_vrf on ER.

        Ok(())
    }

    /// Fills the map with a deterministic seed. Used for PvE campaign sessions on ER.
    /// Must be called after init_map_account and delegation (runs on Ephemeral Rollup).
    pub fn fill_map_with_seed(
        ctx: Context<FillMapWithSeed>,
        seed: u64,
        campaign_level: u8,
    ) -> Result<()> {
        require!(
            campaign_level > 0 && campaign_level <= MAX_LEVEL,
            MapGeneratorError::InvalidLevel
        );

        let generated_map = &mut ctx.accounts.generated_map;
        require_generated_map_uninitialized(generated_map)?;
        let success = maze::generate_map(generated_map, seed, campaign_level, false);
        require!(success, MapGeneratorError::MapGenerationFailed);
        generated_map.clear_discovery();
        let spawn_x = generated_map.spawn_x;
        let spawn_y = generated_map.spawn_y;
        generated_map.reveal_radius(spawn_x, spawn_y, 6);

        // Populate SessionDiscovery with initial map metadata and spawn-area reveal
        if let Some(ref mut discovery) = ctx.accounts.session_discovery {
            discovery.spawn_x = generated_map.spawn_x;
            discovery.spawn_y = generated_map.spawn_y;
            discovery.mole_den_x = generated_map.mole_den_x;
            discovery.mole_den_y = generated_map.mole_den_y;
            discovery.map_width = generated_map.width;
            discovery.map_height = generated_map.height;
            discovery.sync_all_discovered(generated_map);
        }

        Ok(())
    }

    /// Fills the map with a deterministic seed, authorized by gameplay-state via CPI.
    /// Used for duel map generation where gameplay-state controls seed selection.
    pub fn fill_map_with_seed_authorized(
        ctx: Context<FillMapWithSeedAuthorized>,
        seed: u64,
        campaign_level: u8,
    ) -> Result<()> {
        require!(
            campaign_level > 0 && campaign_level <= MAX_LEVEL,
            MapGeneratorError::InvalidLevel
        );

        let generated_map = &mut ctx.accounts.generated_map;
        require_generated_map_uninitialized(generated_map)?;
        let success = maze::generate_map(generated_map, seed, campaign_level, false);
        require!(success, MapGeneratorError::MapGenerationFailed);
        generated_map.clear_discovery();
        let spawn_x = generated_map.spawn_x;
        let spawn_y = generated_map.spawn_y;
        generated_map.reveal_radius(spawn_x, spawn_y, 6);

        if let Some(ref mut discovery) = ctx.accounts.session_discovery {
            discovery.spawn_x = generated_map.spawn_x;
            discovery.spawn_y = generated_map.spawn_y;
            discovery.mole_den_x = generated_map.mole_den_x;
            discovery.mole_den_y = generated_map.mole_den_y;
            discovery.map_width = generated_map.width;
            discovery.map_height = generated_map.height;
            discovery.sync_all_discovered(generated_map);
        }

        Ok(())
    }

    /// Fills the map for a campaign level using the on-chain MapConfig seed.
    /// This keeps the campaign seed private from the client while still using
    /// deterministic generation on the Ephemeral Rollup.
    pub fn fill_map_for_campaign(
        ctx: Context<FillMapForCampaign>,
        campaign_level: u8,
    ) -> Result<()> {
        require!(
            campaign_level > 0 && campaign_level <= MAX_LEVEL,
            MapGeneratorError::InvalidLevel
        );

        let seed = ctx.accounts.map_config.seeds[(campaign_level - 1) as usize];
        let generated_map = &mut ctx.accounts.generated_map;
        require_generated_map_uninitialized(generated_map)?;
        let success = maze::generate_map(generated_map, seed, campaign_level, false);
        require!(success, MapGeneratorError::MapGenerationFailed);
        generated_map.clear_discovery();
        let spawn_x = generated_map.spawn_x;
        let spawn_y = generated_map.spawn_y;
        generated_map.reveal_radius(spawn_x, spawn_y, 6);

        if let Some(ref mut discovery) = ctx.accounts.session_discovery {
            discovery.spawn_x = generated_map.spawn_x;
            discovery.spawn_y = generated_map.spawn_y;
            discovery.mole_den_x = generated_map.mole_den_x;
            discovery.mole_den_y = generated_map.mole_den_y;
            discovery.map_width = generated_map.width;
            discovery.map_height = generated_map.height;
            discovery.sync_all_discovered(generated_map);
        }

        Ok(())
    }

    /// Marks a POI as used on the generated map.
    pub fn mark_poi_used(ctx: Context<MarkPoiUsed>, poi_index: u8) -> Result<()> {
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

        // If this tile is already discovered, update SessionDiscovery to reflect floor
        if let Some(ref mut discovery) = ctx.accounts.session_discovery {
            discovery.update_tile_type(generated_map.width, x, y, false);
        }

        Ok(())
    }

    /// Persists map discovery around the provided position.
    ///
    /// Authorization: session_signer must match the owning session signer.
    pub fn reveal_radius(
        ctx: Context<RevealRadius>,
        center_x: u8,
        center_y: u8,
        radius: u8,
    ) -> Result<()> {
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
        drop(session_data);

        let generated_map = &mut ctx.accounts.generated_map;

        require!(
            center_x < generated_map.width && center_y < generated_map.height,
            MapGeneratorError::TileOutOfBounds
        );

        generated_map.reveal_radius(center_x, center_y, radius);

        // Dual-write to SessionDiscovery
        if let Some(ref mut discovery) = ctx.accounts.session_discovery {
            discovery.sync_radius(generated_map, center_x, center_y, radius);
        }

        Ok(())
    }

    /// Persists discovery using a Manhattan-distance diamond.
    pub fn reveal_manhattan_radius(
        ctx: Context<RevealRadius>,
        center_x: u8,
        center_y: u8,
        radius: u8,
    ) -> Result<()> {
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
        drop(session_data);

        let generated_map = &mut ctx.accounts.generated_map;

        require!(
            center_x < generated_map.width && center_y < generated_map.height,
            MapGeneratorError::TileOutOfBounds
        );

        generated_map.reveal_manhattan_radius(center_x, center_y, radius);

        // Dual-write to SessionDiscovery
        if let Some(ref mut discovery) = ctx.accounts.session_discovery {
            discovery.sync_manhattan_radius(generated_map, center_x, center_y, radius);
        }

        Ok(())
    }

    // ========================================================================
    // SessionDiscovery Instructions
    // ========================================================================

    /// Allocates an empty SessionDiscovery account.
    /// Called via CPI from session-manager during session start.
    pub fn init_session_discovery(ctx: Context<InitSessionDiscovery>) -> Result<()> {
        let discovery = &mut ctx.accounts.session_discovery;
        discovery.session = ctx.accounts.session.key();
        discovery.bump = ctx.bumps.session_discovery;
        Ok(())
    }

    /// Closes the SessionDiscovery account, returning rent to player.
    pub fn close_session_discovery(ctx: Context<CloseSessionDiscovery>) -> Result<()> {

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

        emit!(SessionDiscoveryClosed {
            session: ctx.accounts.session_discovery.session,
        });

        Ok(())
    }

    /// Delegates SessionDiscovery PDA to MagicBlock from its owning program.
    pub fn delegate_session_discovery(
        ctx: Context<DelegateSessionDiscovery>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected, _) = Pubkey::find_program_address(
            &[SessionDiscovery::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.session_discovery.key(),
            expected,
            MapGeneratorError::Unauthorized
        );
        let seeds: &[&[u8]] = &[SessionDiscovery::SEED_PREFIX, session_key.as_ref()];
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.session_discovery,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_session_discovery,
            &ctx.accounts.delegation_record_session_discovery,
            &ctx.accounts.delegation_metadata_session_discovery,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Commits and undelegates SessionDiscovery PDA from ER back to base layer.
    pub fn undelegate_session_discovery(ctx: Context<UndelegateSessionDiscovery>) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected, _) = Pubkey::find_program_address(
            &[SessionDiscovery::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.session_discovery.key(),
            expected,
            MapGeneratorError::Unauthorized
        );

        let discovery_info = ctx.accounts.session_discovery.to_account_info();
        er_compat::commit_and_undelegate(
            ctx.accounts.session_signer.to_account_info(),
            ctx.accounts.magic_context.to_account_info(),
            ctx.accounts.magic_program.to_account_info(),
            &[discovery_info],
        )?;
        Ok(())
    }

    /// Records a discovered POI in SessionDiscovery.
    /// Called via CPI from poi-system when a POI is discovered.
    pub fn record_discovered_poi(
        ctx: Context<RecordDiscoveredPoi>,
        poi_type: u8,
        x: u8,
        y: u8,
        map_pois_index: u8,
    ) -> Result<()> {
        let discovery = &mut ctx.accounts.session_discovery;
        let count = discovery.discovered_poi_count as usize;
        require!(
            count < constants::MAX_DISCOVERED_POIS,
            MapGeneratorError::DiscoveredPoisFull
        );

        // Check for duplicate (same position)
        for i in 0..count {
            if discovery.discovered_pois[i].x == x && discovery.discovered_pois[i].y == y {
                return Ok(()); // Already recorded
            }
        }

        discovery.discovered_pois[count] = state::DiscoveredPoi {
            poi_type,
            x,
            y,
            used: 0,
            map_pois_index,
        };
        discovery.discovered_poi_count = (count + 1) as u8;
        Ok(())
    }

    /// Marks a discovered POI as used in SessionDiscovery.
    /// Called via CPI from poi-system after a one-time POI is consumed.
    pub fn mark_discovered_poi_used(
        ctx: Context<RecordDiscoveredPoi>,
        map_pois_index: u8,
    ) -> Result<()> {
        let discovery = &mut ctx.accounts.session_discovery;
        for i in 0..discovery.discovered_poi_count as usize {
            if discovery.discovered_pois[i].map_pois_index == map_pois_index {
                discovery.discovered_pois[i].used = 1;
                return Ok(());
            }
        }
        Ok(()) // POI not in discovery yet — no-op
    }

    /// Updates the active offer data in SessionDiscovery.
    /// Called via CPI from poi-system when an offer is generated, rerolled, or consumed.
    pub fn update_active_offer(
        ctx: Context<UpdateActiveOffer>,
        offer_type: u8,
        poi_index: u8,
        data: Vec<u8>,
    ) -> Result<()> {
        let discovery = &mut ctx.accounts.session_discovery;
        discovery.active_offer_type = offer_type;
        discovery.active_offer_poi_index = poi_index;

        match offer_type {
            0 => {
                // Clear offer — no data needed
            }
            1 => {
                match data.len() {
                    len if len >= 272 => {
                        // Backward-compatible decode for the expanded relic-aware payload:
                        // 6 * (8 item_id + 32 relic_asset + 1 is_relic + 1 tier + 2 price + 1 purchased)
                        // + 1 reroll + 1 active = 272 bytes
                        for i in 0..6 {
                            let offset = i * 45;
                            let mut item_id = [0u8; 8];
                            item_id.copy_from_slice(&data[offset..offset + 8]);
                            discovery.shop_offers[i] = state::DiscoveryShopOffer {
                                item_id,
                                tier: data[offset + 41],
                                price: u16::from_le_bytes([data[offset + 42], data[offset + 43]]),
                                purchased: data[offset + 44],
                            };
                        }
                        discovery.shop_reroll_count = data[270];
                        discovery.shop_active = data[271];
                    }
                    len if len >= 74 => {
                        // Compact payload:
                        // 6 * (8 item_id + 1 tier + 2 price + 1 purchased) + 1 reroll + 1 active = 74 bytes
                        for i in 0..6 {
                            let offset = i * 12;
                            let mut item_id = [0u8; 8];
                            item_id.copy_from_slice(&data[offset..offset + 8]);
                            discovery.shop_offers[i] = state::DiscoveryShopOffer {
                                item_id,
                                tier: data[offset + 8],
                                price: u16::from_le_bytes([data[offset + 9], data[offset + 10]]),
                                purchased: data[offset + 11],
                            };
                        }
                        discovery.shop_reroll_count = data[72];
                        discovery.shop_active = data[73];
                    }
                    _ => return Err(MapGeneratorError::InvalidOfferData.into()),
                }
            }
            2 => {
                match data.len() {
                    len if len >= 129 => {
                        // Backward-compatible decode for the expanded relic-aware payload:
                        // 3 * (8 item_id + 32 relic_asset + 1 is_relic + 1 rarity + 1 tier) = 129 bytes
                        for i in 0..3 {
                            let offset = i * 43;
                            let mut item_id = [0u8; 8];
                            item_id.copy_from_slice(&data[offset..offset + 8]);
                            discovery.cache_offer_items[i] = state::DiscoveryOfferItem {
                                item_id,
                                rarity: data[offset + 41],
                                tier: data[offset + 42],
                            };
                        }
                    }
                    len if len >= 30 => {
                        // Compact payload: 3 * (8 item_id + 1 rarity + 1 tier) = 30 bytes
                        for i in 0..3 {
                            let offset = i * 10;
                            let mut item_id = [0u8; 8];
                            item_id.copy_from_slice(&data[offset..offset + 8]);
                            discovery.cache_offer_items[i] = state::DiscoveryOfferItem {
                                item_id,
                                rarity: data[offset + 8],
                                tier: data[offset + 9],
                            };
                        }
                    }
                    _ => return Err(MapGeneratorError::InvalidOfferData.into()),
                }
            }
            3 => {
                // Oil: 3 oil flags
                require!(data.len() >= 3, MapGeneratorError::InvalidOfferData);
                discovery.oil_offer_oils.copy_from_slice(&data[..3]);
            }
            4 => {
                // Scanner: 1 count + 3 poi_types = 4 bytes
                require!(data.len() >= 4, MapGeneratorError::InvalidOfferData);
                discovery.scanner_offer_count = data[0];
                discovery.scanner_offer_types.copy_from_slice(&data[1..4]);
            }
            _ => return Err(MapGeneratorError::InvalidOfferData.into()),
        }

        Ok(())
    }

    /// Overwrites discovered enemies in SessionDiscovery.
    /// Called via CPI from gameplay-state after movement reveals new tiles.
    ///
    /// Authorization: Requires gameplay_authority PDA as signer.
    pub fn update_discovered_enemies(
        ctx: Context<UpdateDiscoveredEnemies>,
        enemies: Vec<state::DiscoveredEnemy>,
    ) -> Result<()> {
        let discovery = &mut ctx.accounts.session_discovery;
        let len = enemies.len().min(constants::MAX_ENEMIES);
        discovery.discovered_enemies[..len].copy_from_slice(&enemies[..len]);
        for i in len..constants::MAX_ENEMIES {
            discovery.discovered_enemies[i] = state::DiscoveredEnemy::default();
        }
        discovery.discovered_enemy_count = len as u8;
        Ok(())
    }

    /// Updates the current boss ID in SessionDiscovery.
    /// Called via CPI from gameplay-state at sync_map_enemies and week transitions.
    ///
    /// Authorization: Requires gameplay_authority PDA as signer.
    pub fn update_boss_id(ctx: Context<UpdateBossId>, boss_id: [u8; 12]) -> Result<()> {
        ctx.accounts.session_discovery.current_boss_id = boss_id;
        Ok(())
    }

    /// Updates the current gauntlet echo in SessionDiscovery.
    /// Called via CPI from gameplay-state at sync_map_enemies and week transitions.
    ///
    /// Authorization: Requires gameplay_authority PDA as signer.
    pub fn update_current_echo(
        ctx: Context<UpdateCurrentEcho>,
        echo_present: u8,
        echo_data: [u8; 179],
    ) -> Result<()> {
        let discovery = &mut ctx.accounts.session_discovery;
        discovery.current_echo_present = echo_present;
        discovery.current_echo_data = echo_data;
        Ok(())
    }

    /// Closes the GeneratedMap account, returning rent to player.
    /// Used by session-manager CPI during end_session to clean up.
    ///
    /// Authorization: Reads session account to verify session_signer matches signer,
    /// then returns rent to session.player.
    pub fn close_generated_map(ctx: Context<CloseGeneratedMap>) -> Result<()> {
        // Byte offset of `session_signer` in GameSession account data.
        // Keep in sync with session_manager::state::GameSession::SESSION_SIGNER_OFFSET.
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

    /// Close GeneratedMap when the session PDA no longer exists (orphaned).
    /// Validates session_signer and player via the GameState account instead.
    pub fn close_generated_map_orphaned(ctx: Context<CloseGeneratedMapOrphaned>) -> Result<()> {


        let gs_data = ctx.accounts.game_state.try_borrow_data()?;
        require!(
            gs_data.len() >= GAME_STATE_SESSION_OFFSET + 32,
            MapGeneratorError::InvalidSession
        );

        let stored_session = Pubkey::from(
            <[u8; 32]>::try_from(
                &gs_data[GAME_STATE_SESSION_OFFSET..GAME_STATE_SESSION_OFFSET + 32],
            )
            .unwrap(),
        );
        require!(
            stored_session == ctx.accounts.generated_map.session,
            MapGeneratorError::InvalidSession
        );

        let stored_session_signer = Pubkey::from(
            <[u8; 32]>::try_from(
                &gs_data[GAME_STATE_SESSION_SIGNER_OFFSET..GAME_STATE_SESSION_SIGNER_OFFSET + 32],
            )
            .unwrap(),
        );
        require!(
            stored_session_signer == ctx.accounts.session_signer.key(),
            MapGeneratorError::Unauthorized
        );

        let stored_player = Pubkey::from(
            <[u8; 32]>::try_from(&gs_data[GAME_STATE_PLAYER_OFFSET..GAME_STATE_PLAYER_OFFSET + 32])
                .unwrap(),
        );
        require!(
            stored_player == ctx.accounts.player.key(),
            MapGeneratorError::Unauthorized
        );

        drop(gs_data);

        emit!(GeneratedMapClosed {
            session: ctx.accounts.generated_map.session,
        });

        Ok(())
    }

    /// Close SessionDiscovery when the session PDA no longer exists (orphaned).
    /// Validates session_signer and player via the GameState account instead.
    pub fn close_session_discovery_orphaned(
        ctx: Context<CloseSessionDiscoveryOrphaned>,
    ) -> Result<()> {


        let gs_data = ctx.accounts.game_state.try_borrow_data()?;
        require!(
            gs_data.len() >= GAME_STATE_SESSION_OFFSET + 32,
            MapGeneratorError::InvalidSession
        );

        let stored_session = Pubkey::from(
            <[u8; 32]>::try_from(
                &gs_data[GAME_STATE_SESSION_OFFSET..GAME_STATE_SESSION_OFFSET + 32],
            )
            .unwrap(),
        );

        let stored_session_signer = Pubkey::from(
            <[u8; 32]>::try_from(
                &gs_data[GAME_STATE_SESSION_SIGNER_OFFSET..GAME_STATE_SESSION_SIGNER_OFFSET + 32],
            )
            .unwrap(),
        );
        require!(
            stored_session_signer == ctx.accounts.session_signer.key(),
            MapGeneratorError::Unauthorized
        );

        let stored_player = Pubkey::from(
            <[u8; 32]>::try_from(&gs_data[GAME_STATE_PLAYER_OFFSET..GAME_STATE_PLAYER_OFFSET + 32])
                .unwrap(),
        );
        require!(
            stored_player == ctx.accounts.player.key(),
            MapGeneratorError::Unauthorized
        );

        drop(gs_data);

        let expected_session_discovery = Pubkey::find_program_address(
            &[SessionDiscovery::SEED_PREFIX, stored_session.as_ref()],
            &crate::ID,
        )
        .0;
        require!(
            expected_session_discovery == ctx.accounts.session_discovery.key(),
            MapGeneratorError::InvalidSession
        );

        let discovery_info = ctx.accounts.session_discovery.to_account_info();
        let discovery_data = discovery_info.try_borrow_data()?;
        require!(
            discovery_data.len() >= SESSION_DISCOVERY_SESSION_OFFSET + 32,
            MapGeneratorError::InvalidSession
        );
        let discovery_session = Pubkey::from(
            <[u8; 32]>::try_from(
                &discovery_data
                    [SESSION_DISCOVERY_SESSION_OFFSET..SESSION_DISCOVERY_SESSION_OFFSET + 32],
            )
            .unwrap(),
        );
        require!(
            discovery_session == stored_session,
            MapGeneratorError::InvalidSession
        );
        drop(discovery_data);

        close_owned_account(&discovery_info, &ctx.accounts.player)?;

        emit!(SessionDiscoveryClosed {
            session: stored_session,
        });

        Ok(())
    }

    /// Delegates generated-map PDA to MagicBlock from its owning program.
    pub fn delegate_generated_map(
        ctx: Context<DelegateGeneratedMap>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
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
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.generated_map,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_generated_map,
            &ctx.accounts.delegation_record_generated_map,
            &ctx.accounts.delegation_metadata_generated_map,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            map_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    // ========================================================================
    // VRF Instructions
    // ========================================================================

    /// Pre-creates MapVrfState PDA on base chain without requesting randomness.
    /// Used to initialize the account before delegation to ER, so VRF requests
    /// on the Ephemeral Rollup don't need to create new accounts inline.
    pub fn init_map_vrf_state(ctx: Context<InitMapVrfState>) -> Result<()> {
        let vrf_state = &mut ctx.accounts.vrf_state;
        vrf_state.session = ctx.accounts.session.key();
        vrf_state.randomness = [0u8; 32];
        vrf_state.nonce = 1;
        vrf_state.status = VrfStatus::Requested;
        vrf_state.bump = ctx.bumps.vrf_state;
        Ok(())
    }

    /// Delegates MapVrfState PDA to MagicBlock from its owning program.
    pub fn delegate_map_vrf_state(
        ctx: Context<DelegateMapVrfState>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected_vrf_state, _) = Pubkey::find_program_address(
            &[MapVrfState::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.map_vrf_state.key(),
            expected_vrf_state,
            MapGeneratorError::Unauthorized
        );
        let vrf_seeds: &[&[u8]] = &[MapVrfState::SEED_PREFIX, session_key.as_ref()];
        er_compat::delegate_account(
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.map_vrf_state,
            &ctx.accounts.owner_program,
            &ctx.accounts.buffer_map_vrf_state,
            &ctx.accounts.delegation_record_map_vrf_state,
            &ctx.accounts.delegation_metadata_map_vrf_state,
            &ctx.accounts.delegation_program,
            &ctx.accounts.system_program.to_account_info(),
            vrf_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Commits and undelegates MapVrfState PDA from ER back to base layer.
    pub fn undelegate_map_vrf_state(ctx: Context<UndelegateMapVrfState>) -> Result<()> {
        let session_key = ctx.accounts.session.key();
        let (expected_vrf_state, _) = Pubkey::find_program_address(
            &[MapVrfState::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.map_vrf_state.key(),
            expected_vrf_state,
            MapGeneratorError::Unauthorized
        );

        let map_vrf_state_info = ctx.accounts.map_vrf_state.to_account_info();
        er_compat::commit_and_undelegate(
            ctx.accounts.session_signer.to_account_info(),
            ctx.accounts.magic_context.to_account_info(),
            ctx.accounts.magic_program.to_account_info(),
            &[map_vrf_state_info],
        )?;
        Ok(())
    }

    /// Requests VRF randomness for map generation.
    /// Initializes a MapVrfState account with status=Requested.
    #[allow(clippy::missing_transmute_annotations)]
    pub fn request_map_vrf(ctx: Context<RequestMapVrf>) -> Result<()> {
        let vrf_state = &mut ctx.accounts.vrf_state;
        require!(
            can_request_map_vrf(vrf_state.session, vrf_state.status),
            MapGeneratorError::VrfAlreadyFinalized
        );
        vrf_state.session = ctx.accounts.session.key();
        vrf_state.randomness = [0u8; 32];
        vrf_state.nonce = 1;
        vrf_state.status = VrfStatus::Requested;
        vrf_state.bump = ctx.bumps.vrf_state;

        let mut caller_seed = [0u8; 32];
        caller_seed.copy_from_slice(ctx.accounts.session.key().as_ref());
        caller_seed[..8].copy_from_slice(&vrf_state.nonce.to_le_bytes());

        // SAFETY: Pubkey layout is identical between versions (32 bytes).
        let ix = unsafe {
            create_request_randomness_ix(RequestRandomnessParams {
                payer: std::mem::transmute(ctx.accounts.payer.key()),
                oracle_queue: std::mem::transmute(ctx.accounts.oracle_queue.key()),
                callback_program_id: std::mem::transmute(crate::ID),
                callback_discriminator: instruction::FulfillMapVrf::DISCRIMINATOR.to_vec(),
                accounts_metas: Some(vec![SerializableAccountMeta {
                    pubkey: std::mem::transmute(ctx.accounts.vrf_state.key()),
                    is_signer: false,
                    is_writable: true,
                }]),
                caller_seed,
                ..Default::default()
            })
        };

        let (_, identity_bump) =
            Pubkey::find_program_address(&[er_compat::VRF_IDENTITY_SEED], &crate::ID);
        // SAFETY: Instruction layout is identical between versions.
        let ix_new: anchor_lang::solana_program::instruction::Instruction = unsafe { std::mem::transmute(ix) };
        anchor_lang::solana_program::program::invoke_signed(
            &ix_new,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.program_identity.to_account_info(),
                ctx.accounts.oracle_queue.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.slot_hashes.to_account_info(),
            ],
            &[&[er_compat::VRF_IDENTITY_SEED, &[identity_bump]]],
        )?;
        Ok(())
    }

    /// Oracle callback: receives VRF randomness and writes it to state.
    /// TODO: Verify vrf_program_identity signer when ephemeral-vrf-sdk is available.
    pub fn fulfill_map_vrf(ctx: Context<FulfillMapVrf>, randomness: [u8; 32]) -> Result<()> {
        let vrf_state = &mut ctx.accounts.vrf_state;
        require!(
            vrf_state.status == VrfStatus::Requested,
            MapGeneratorError::VrfNotRequested
        );
        vrf_state.randomness = randomness;
        vrf_state.status = VrfStatus::Fulfilled;
        Ok(())
    }

    /// Generates the map using VRF-derived randomness. Used for Gauntlet/Duel sessions on ER.
    /// Must be called after VRF fulfillment on the Ephemeral Rollup.
    pub fn generate_map_with_vrf(
        ctx: Context<GenerateMapWithVrf>,
        campaign_level: u8,
    ) -> Result<()> {
        require!(
            campaign_level > 0 && campaign_level <= MAX_LEVEL,
            MapGeneratorError::InvalidLevel
        );

        let vrf_state = &mut ctx.accounts.vrf_state;
        require!(
            vrf_state.status == VrfStatus::Fulfilled,
            MapGeneratorError::VrfNotFulfilled
        );

        let mut rng = vrf_rng::GameRng::from_vrf(
            &vrf_state.randomness,
            vrf_state.nonce,
            vrf_rng::domains::MAP_GENERATION,
        );
        let vrf_seed = rng.next_val();

        let generated_map = &mut ctx.accounts.generated_map;
        require_generated_map_uninitialized(generated_map)?;
        let success = maze::generate_map(generated_map, vrf_seed, campaign_level, true);
        require!(success, MapGeneratorError::MapGenerationFailed);
        generated_map.clear_discovery();
        let spawn_x = generated_map.spawn_x;
        let spawn_y = generated_map.spawn_y;
        generated_map.reveal_radius(spawn_x, spawn_y, 6);

        // Populate SessionDiscovery with initial map metadata and spawn-area reveal
        if let Some(ref mut discovery) = ctx.accounts.session_discovery {
            discovery.spawn_x = generated_map.spawn_x;
            discovery.spawn_y = generated_map.spawn_y;
            discovery.mole_den_x = generated_map.mole_den_x;
            discovery.mole_den_y = generated_map.mole_den_y;
            discovery.map_width = generated_map.width;
            discovery.map_height = generated_map.height;
            discovery.sync_all_discovered(generated_map);
        }

        vrf_state.status = VrfStatus::Consumed;
        Ok(())
    }

    /// Marks MapVrfState as consumed after VRF randomness has been used.
    /// Called via CPI from session-manager during PvP session start.
    pub fn consume_map_vrf(ctx: Context<ConsumeMapVrf>) -> Result<()> {
        let vrf_state = &mut ctx.accounts.vrf_state;
        require!(
            vrf_state.status == VrfStatus::Fulfilled,
            MapGeneratorError::VrfNotFulfilled
        );
        vrf_state.status = VrfStatus::Consumed;
        Ok(())
    }

    /// Closes MapVrfState account and returns rent to the player.
    /// Called via CPI from session-manager during end_session/abandon_session.
    pub fn close_map_vrf_state(ctx: Context<CloseMapVrfState>) -> Result<()> {

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
        er_compat::commit_and_undelegate(
            ctx.accounts.session_signer.to_account_info(),
            ctx.accounts.magic_context.to_account_info(),
            ctx.accounts.magic_program.to_account_info(),
            &[generated_map_info],
        )?;
        Ok(())
    }

    /// Processes undelegation (replaces #[ephemeral] macro output).
    pub fn process_undelegation(ctx: Context<InitializeAfterUndelegation>, account_seeds: Vec<Vec<u8>>) -> Result<()> {
        er_compat::undelegate_account(
            &ctx.accounts.base_account,
            &crate::id(),
            &ctx.accounts.buffer,
            &ctx.accounts.payer,
            &ctx.accounts.system_program,
            account_seeds,
        )
    }
}

/// Context for undelegation processing (replaces #[ephemeral] macro output).
#[derive(Accounts)]
pub struct InitializeAfterUndelegation<'info> {
    /// CHECK: Account being undelegated
    #[account(mut)]
    pub base_account: UncheckedAccount<'info>,
    /// CHECK: Delegation buffer
    pub buffer: UncheckedAccount<'info>,
    /// CHECK: Payer
    #[account(mut)]
    pub payer: UncheckedAccount<'info>,
    /// CHECK: System program
    pub system_program: UncheckedAccount<'info>,
}

fn close_owned_account(account: &AccountInfo, destination: &AccountInfo) -> Result<()> {
    let lamports = account.lamports();
    **destination.try_borrow_mut_lamports()? = destination
        .lamports()
        .checked_add(lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    **account.try_borrow_mut_lamports()? = 0;
    account.assign(&system_program::ID);
    account.resize(0)?;
    Ok(())
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

#[derive(Accounts)]
pub struct DelegateGeneratedMap<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated via explicit seed check in handler.
    pub generated_map: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for seed derivation.
    pub session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, generated_map.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_generated_map: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, generated_map.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_generated_map: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, generated_map.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_generated_map: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UndelegateGeneratedMap<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated and deserialized in handler.
    pub generated_map: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for seed derivation.
    pub session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
    /// CHECK: Magic program
    #[account(address = er_compat::MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
    /// CHECK: Magic context
    #[account(mut, address = er_compat::MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,
}

fn read_generated_map(generated_map: &AccountInfo<'_>) -> Result<GeneratedMap> {
    let data = generated_map.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    GeneratedMap::try_deserialize(&mut slice).map_err(|_| MapGeneratorError::InvalidSession.into())
}

fn require_generated_map_uninitialized(generated_map: &GeneratedMap) -> Result<()> {
    require!(
        generated_map.seed == 0 && generated_map.width == 0 && generated_map.height == 0,
        MapGeneratorError::MapAlreadyExists
    );
    Ok(())
}

fn can_request_map_vrf(session: Pubkey, status: VrfStatus) -> bool {
    session == Pubkey::default()
        || (status != VrfStatus::Fulfilled && status != VrfStatus::Consumed)
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
    /// CHECK: Session PDA owned by session-manager; validated via raw-byte reads in handler.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,
}

/// Context for allocating an empty GeneratedMap (no maze generation).
/// Called via CPI from session-manager; actual generation happens on ER.
#[derive(Accounts)]
pub struct InitMapAccount<'info> {
    /// Payer for rent
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Game session PDA reference
    /// CHECK: Ownership is validated by constraint; PDA relationship is enforced by seeds on generated_map.
    #[account(
        owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner
    )]
    pub session: UncheckedAccount<'info>,

    /// Generated map output (allocated empty, filled on ER)
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

/// Context for filling map with a deterministic seed (PvE campaign on ER).
#[derive(Accounts)]
pub struct FillMapWithSeed<'info> {
    pub session_signer: Signer<'info>,

    /// CHECK: Session PDA used for seed derivation.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session,
    )]
    pub generated_map: Box<Account<'info, GeneratedMap>>,

    /// Optional SessionDiscovery to populate with initial map data.
    #[account(mut)]
    pub session_discovery: Option<Box<Account<'info, SessionDiscovery>>>,
}

/// Context for filling a map with a seed, authorized by gameplay-state via CPI.
/// Uses gameplay_authority PDA from gameplay-state as signer.
#[derive(Accounts)]
pub struct FillMapWithSeedAuthorized<'info> {
    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session,
    )]
    pub generated_map: Box<Account<'info, GeneratedMap>>,

    /// CHECK: Session PDA used for PDA derivation of generated_map.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    /// Gameplay authority PDA from gameplay-state that must sign.
    /// This ensures only gameplay-state can call this instruction.
    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = GAMEPLAY_STATE_PROGRAM_ID,
    )]
    pub gameplay_authority: Signer<'info>,

    /// Optional SessionDiscovery to populate with initial map data.
    #[account(mut)]
    pub session_discovery: Option<Box<Account<'info, SessionDiscovery>>>,
}

/// Context for filling a campaign map using the private on-chain MapConfig seed.
#[derive(Accounts)]
pub struct FillMapForCampaign<'info> {
    pub session_signer: Signer<'info>,

    /// CHECK: Session PDA used for map ownership validation.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    #[account(
        seeds = [MapConfig::SEED_PREFIX],
        bump = map_config.bump
    )]
    pub map_config: Account<'info, MapConfig>,

    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session,
    )]
    pub generated_map: Box<Account<'info, GeneratedMap>>,

    /// Optional SessionDiscovery to populate with initial map data.
    #[account(mut)]
    pub session_discovery: Option<Box<Account<'info, SessionDiscovery>>>,
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
    pub generated_map: Box<Account<'info, GeneratedMap>>,

    /// Game session PDA reference (validated by owner + has_one)
    /// CHECK: Session PDA owned by session-manager; validated via has_one constraint.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    /// Gameplay authority PDA from gameplay-state that must sign
    /// This ensures only gameplay-state can call this instruction
    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = GAMEPLAY_STATE_PROGRAM_ID,
    )]
    pub gameplay_authority: Signer<'info>,

    /// Optional SessionDiscovery to update tile type when a discovered wall becomes floor.
    #[account(mut)]
    pub session_discovery: Option<Box<Account<'info, SessionDiscovery>>>,
}

/// Context for persisting discovered tiles via session key signer.
#[derive(Accounts)]
pub struct RevealRadius<'info> {
    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session,
    )]
    pub generated_map: Box<Account<'info, GeneratedMap>>,

    /// CHECK: Session PDA owned by session-manager; validated via raw-byte reads in handler.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    pub session_signer: Signer<'info>,

    /// Optional SessionDiscovery for dual-write of discovered tile types.
    #[account(mut)]
    pub session_discovery: Option<Box<Account<'info, SessionDiscovery>>>,
}

/// Context for closing GeneratedMap account via session key signer.
#[derive(Accounts)]
pub struct CloseGeneratedMap<'info> {
    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session,
        close = session_signer,
    )]
    pub generated_map: Account<'info, GeneratedMap>,

    /// Game session PDA to verify session_signer authorization
    /// CHECK: Session PDA owned by session-manager; validated via raw-byte reads in handler.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    /// CHECK: Validated against session.player in instruction
    pub player: UncheckedAccount<'info>,

    /// Session key signer must sign to authorize closure and receives rent refund
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

// ============================================================================
// VRF Account Contexts
// ============================================================================

#[derive(Accounts)]
pub struct RequestMapVrf<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Session PDA key used only for VRF PDA derivation.
    /// This may be called before session account initialization.
    pub session: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = MapVrfState::SPACE,
        seeds = [MapVrfState::SEED_PREFIX, session.key().as_ref()],
        bump
    )]
    pub vrf_state: Account<'info, MapVrfState>,

    /// CHECK: Program identity PDA used as callback signer.
    #[account(seeds = [er_compat::VRF_IDENTITY_SEED], bump)]
    pub program_identity: UncheckedAccount<'info>,

    /// CHECK: Oracle queue account — must be owned by the VRF program.
    #[account(mut, owner = er_compat::VRF_PROGRAM_ID)]
    pub oracle_queue: UncheckedAccount<'info>,

    /// CHECK: Slot hashes sysvar for VRF request validation.
    /// CHECK: SlotHashes sysvar - SysvarS1otHashes111111111111111111111111111
    #[account(address = Pubkey::new_from_array([
        0x06, 0xa7, 0xd5, 0x17, 0x19, 0x2f, 0x0a, 0xaf, 0xc6, 0xf2, 0x65, 0xe3, 0xfb, 0x77,
        0xcc, 0x7a, 0xda, 0x82, 0xc5, 0x29, 0xd0, 0xbe, 0x3b, 0x13, 0x6e, 0x2d, 0x00, 0x55,
        0x20, 0x00, 0x00, 0x00,
    ]))]
    pub slot_hashes: UncheckedAccount<'info>,

    /// CHECK: VRF program for CPI invocation.
    #[account(address = er_compat::VRF_PROGRAM_ID)]
    pub vrf_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FulfillMapVrf<'info> {
    /// Oracle identity signer.
    #[cfg_attr(
        not(feature = "mock-vrf"),
        account(address = er_compat::VRF_PROGRAM_IDENTITY)
    )]
    pub oracle: Signer<'info>,

    #[account(
        mut,
        seeds = [MapVrfState::SEED_PREFIX, vrf_state.session.as_ref()],
        bump = vrf_state.bump,
    )]
    pub vrf_state: Account<'info, MapVrfState>,
}

#[derive(Accounts)]
pub struct ConsumeMapVrf<'info> {
    pub session_signer: Signer<'info>,

    #[account(
        mut,
        seeds = [MapVrfState::SEED_PREFIX, vrf_state.session.as_ref()],
        bump = vrf_state.bump,
    )]
    pub vrf_state: Account<'info, MapVrfState>,
}

#[derive(Accounts)]
pub struct CloseMapVrfState<'info> {
    #[account(
        mut,
        seeds = [MapVrfState::SEED_PREFIX, vrf_state.session.as_ref()],
        bump = vrf_state.bump,
        close = session_signer,
    )]
    pub vrf_state: Account<'info, MapVrfState>,

    /// CHECK: Session PDA owned by session-manager; validated via raw-byte reads in handler.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    /// CHECK: Validated against session.player in instruction body.
    pub player: UncheckedAccount<'info>,

    #[account(mut)]
    pub session_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct GenerateMapWithVrf<'info> {
    pub session_signer: Signer<'info>,

    /// CHECK: Session PDA used for seed derivation.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session,
    )]
    pub generated_map: Box<Account<'info, GeneratedMap>>,

    #[account(
        mut,
        seeds = [MapVrfState::SEED_PREFIX, session.key().as_ref()],
        bump = vrf_state.bump,
        has_one = session,
    )]
    pub vrf_state: Account<'info, MapVrfState>,

    /// Optional SessionDiscovery to populate with initial map data.
    #[account(mut)]
    pub session_discovery: Option<Box<Account<'info, SessionDiscovery>>>,
}

/// Pre-creates MapVrfState on base chain (no VRF request).
#[derive(Accounts)]
pub struct InitMapVrfState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Session PDA owned by session-manager.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = MapVrfState::SPACE,
        seeds = [MapVrfState::SEED_PREFIX, session.key().as_ref()],
        bump,
    )]
    pub vrf_state: Account<'info, MapVrfState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DelegateMapVrfState<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated via explicit seed check in handler.
    pub map_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for seed derivation.
    pub session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, map_vrf_state.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_map_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, map_vrf_state.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_map_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, map_vrf_state.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_map_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UndelegateMapVrfState<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated in handler.
    pub map_vrf_state: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for deterministic PDA validation.
    pub session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
    /// CHECK: Magic program
    #[account(address = er_compat::MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
    /// CHECK: Magic context
    #[account(mut, address = er_compat::MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,
}

// ============================================================================
// Events
// ============================================================================

// ============================================================================
// SessionDiscovery Account Contexts
// ============================================================================

/// Context for allocating an empty SessionDiscovery.
/// Called via CPI from session-manager; populated on ER during map generation.
#[derive(Accounts)]
pub struct InitSessionDiscovery<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Ownership is validated by constraint; PDA relationship enforced by seeds.
    #[account(
        owner = SESSION_MANAGER_PROGRAM_ID @ MapGeneratorError::InvalidSessionOwner
    )]
    pub session: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = SessionDiscovery::SPACE,
        seeds = [SessionDiscovery::SEED_PREFIX, session.key().as_ref()],
        bump
    )]
    pub session_discovery: Account<'info, SessionDiscovery>,

    pub system_program: Program<'info, System>,
}

/// Context for closing SessionDiscovery account via session key signer.
#[derive(Accounts)]
pub struct CloseSessionDiscovery<'info> {
    #[account(
        mut,
        seeds = [SessionDiscovery::SEED_PREFIX, session.key().as_ref()],
        bump = session_discovery.bump,
        has_one = session,
        close = session_signer,
    )]
    pub session_discovery: Account<'info, SessionDiscovery>,

    /// CHECK: Session PDA owned by session-manager; validated via raw-byte reads in handler.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    /// CHECK: Validated against session.player in instruction body.
    pub player: UncheckedAccount<'info>,

    #[account(mut)]
    pub session_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct DelegateSessionDiscovery<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated via explicit seed check in handler.
    pub session_discovery: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for seed derivation.
    pub session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
    /// CHECK: Buffer for delegation
    #[account(mut, seeds = [er_compat::DELEGATE_BUFFER_TAG, session_discovery.key().as_ref()], bump, seeds::program = crate::id())]
    pub buffer_session_discovery: UncheckedAccount<'info>,
    /// CHECK: Delegation record
    #[account(mut, seeds = [er_compat::DELEGATION_RECORD_TAG, session_discovery.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_record_session_discovery: UncheckedAccount<'info>,
    /// CHECK: Delegation metadata
    #[account(mut, seeds = [er_compat::DELEGATION_METADATA_TAG, session_discovery.key().as_ref()], bump, seeds::program = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_metadata_session_discovery: UncheckedAccount<'info>,
    /// CHECK: Owner program
    #[account(address = crate::id())]
    pub owner_program: UncheckedAccount<'info>,
    /// CHECK: Delegation program
    #[account(address = er_compat::DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UndelegateSessionDiscovery<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated in handler.
    pub session_discovery: UncheckedAccount<'info>,
    /// CHECK: Session PDA used only for seed derivation.
    pub session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
    /// CHECK: Magic program
    #[account(address = er_compat::MAGIC_PROGRAM_ID)]
    pub magic_program: UncheckedAccount<'info>,
    /// CHECK: Magic context
    #[account(mut, address = er_compat::MAGIC_CONTEXT_ID)]
    pub magic_context: UncheckedAccount<'info>,
}

/// Context for recording a discovered POI into SessionDiscovery.
/// Called via CPI from poi-system when POIs are discovered.
#[derive(Accounts)]
pub struct RecordDiscoveredPoi<'info> {
    #[account(
        mut,
        seeds = [SessionDiscovery::SEED_PREFIX, session.key().as_ref()],
        bump = session_discovery.bump,
        has_one = session,
    )]
    pub session_discovery: Account<'info, SessionDiscovery>,

    /// CHECK: Session PDA owned by session-manager; validated via seeds on session_discovery.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    pub session_signer: Signer<'info>,
}

/// Context for writing active offer data into SessionDiscovery.
/// Called via CPI from poi-system when offers are generated, rerolled, or consumed.
#[derive(Accounts)]
pub struct UpdateActiveOffer<'info> {
    #[account(
        mut,
        seeds = [SessionDiscovery::SEED_PREFIX, session.key().as_ref()],
        bump = session_discovery.bump,
        has_one = session,
    )]
    pub session_discovery: Account<'info, SessionDiscovery>,

    /// CHECK: Session PDA owned by session-manager; validated via seeds on session_discovery.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    pub session_signer: Signer<'info>,
}

/// Context for updating discovered enemies, authorized by gameplay-state via CPI.
#[derive(Accounts)]
pub struct UpdateDiscoveredEnemies<'info> {
    #[account(
        mut,
        seeds = [SessionDiscovery::SEED_PREFIX, session.key().as_ref()],
        bump = session_discovery.bump,
        has_one = session,
    )]
    pub session_discovery: Account<'info, SessionDiscovery>,

    /// CHECK: Session PDA owned by session-manager; validated via seeds on session_discovery.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = GAMEPLAY_STATE_PROGRAM_ID,
    )]
    pub gameplay_authority: Signer<'info>,
}

/// Context for updating boss ID, authorized by gameplay-state via CPI.
#[derive(Accounts)]
pub struct UpdateBossId<'info> {
    #[account(
        mut,
        seeds = [SessionDiscovery::SEED_PREFIX, session.key().as_ref()],
        bump = session_discovery.bump,
        has_one = session,
    )]
    pub session_discovery: Account<'info, SessionDiscovery>,

    /// CHECK: Session PDA owned by session-manager; validated via seeds on session_discovery.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = GAMEPLAY_STATE_PROGRAM_ID,
    )]
    pub gameplay_authority: Signer<'info>,
}

/// Context for updating current echo, authorized by gameplay-state via CPI.
#[derive(Accounts)]
pub struct UpdateCurrentEcho<'info> {
    #[account(
        mut,
        seeds = [SessionDiscovery::SEED_PREFIX, session.key().as_ref()],
        bump = session_discovery.bump,
        has_one = session,
    )]
    pub session_discovery: Account<'info, SessionDiscovery>,

    /// CHECK: Session PDA owned by session-manager; validated via seeds on session_discovery.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID)]
    pub session: UncheckedAccount<'info>,

    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = GAMEPLAY_STATE_PROGRAM_ID,
    )]
    pub gameplay_authority: Signer<'info>,
}

// ============================================================================
// Orphaned Close Contexts
// ============================================================================

/// Close orphaned GeneratedMap (session PDA already closed).
/// Validates via GameState which stores session_signer and player.
#[derive(Accounts)]
pub struct CloseGeneratedMapOrphaned<'info> {
    #[account(
        mut,
        seeds = [GeneratedMap::SEED_PREFIX, generated_map.session.as_ref()],
        bump = generated_map.bump,
        close = player,
    )]
    pub generated_map: Account<'info, GeneratedMap>,

    /// GameState for auth — must belong to the same session.
    /// CHECK: Owner checked (gameplay-state program). Fields validated via raw-byte reads.
    #[account(owner = GAMEPLAY_STATE_PROGRAM_ID)]
    pub game_state: UncheckedAccount<'info>,

    /// Player wallet receives the rent refund.
    /// CHECK: Validated against game_state.player in handler.
    #[account(mut)]
    pub player: UncheckedAccount<'info>,

    /// Session key signer — validated against game_state.session_signer.
    pub session_signer: Signer<'info>,
}

/// Close orphaned SessionDiscovery (session PDA already closed).
/// Validates via GameState which stores session_signer and player.
#[derive(Accounts)]
pub struct CloseSessionDiscoveryOrphaned<'info> {
    #[account(mut, owner = crate::ID)]
    /// CHECK: PDA and stored session are validated in the handler. Closed manually.
    pub session_discovery: UncheckedAccount<'info>,

    /// GameState for auth — must belong to the same session.
    /// CHECK: Owner checked (gameplay-state program). Fields validated via raw-byte reads.
    #[account(owner = GAMEPLAY_STATE_PROGRAM_ID)]
    pub game_state: UncheckedAccount<'info>,

    /// Player wallet receives the rent refund.
    /// CHECK: Validated against game_state.player in handler.
    #[account(mut)]
    pub player: UncheckedAccount<'info>,

    /// Session key signer — validated against game_state.session_signer.
    pub session_signer: Signer<'info>,
}

// ============================================================================
// Events
// ============================================================================

#[event]
pub struct GeneratedMapClosed {
    pub session: Pubkey,
}

#[event]
pub struct SessionDiscoveryClosed {
    pub session: Pubkey,
}

#[cfg(test)]
mod guard_tests {
    use super::*;
    use crate::constants::PACKED_TILES_SIZE;
    use crate::state::{EnemySpawn, PoiSpawn};

    fn blank_generated_map() -> GeneratedMap {
        GeneratedMap {
            session: Pubkey::default(),
            width: 0,
            height: 0,
            seed: 0,
            spawn_x: 0,
            spawn_y: 0,
            mole_den_x: 0,
            mole_den_y: 0,
            walkable_count: 0,
            packed_tiles: [0; PACKED_TILES_SIZE],
            discovered_tiles: [0; PACKED_TILES_SIZE],
            enemy_count: 0,
            enemies: [EnemySpawn::default(); 48],
            poi_count: 0,
            pois: [PoiSpawn::default(); 64],
            bump: 0,
        }
    }

    #[test]
    fn generated_map_guard_allows_uninitialized_account() {
        let map = blank_generated_map();
        assert!(require_generated_map_uninitialized(&map).is_ok());
    }

    #[test]
    fn generated_map_guard_rejects_existing_seed() {
        let mut map = blank_generated_map();
        map.seed = 42;
        assert!(require_generated_map_uninitialized(&map).is_err());
    }

    #[test]
    fn generated_map_guard_rejects_existing_dimensions() {
        let mut map = blank_generated_map();
        map.width = 50;
        assert!(require_generated_map_uninitialized(&map).is_err());
    }

    #[test]
    fn map_vrf_request_guard_rejects_finalized_states_for_existing_session() {
        let session = Pubkey::new_unique();
        assert!(!can_request_map_vrf(session, VrfStatus::Fulfilled));
        assert!(!can_request_map_vrf(session, VrfStatus::Consumed));
    }

    #[test]
    fn map_vrf_request_guard_allows_uninitialized_or_pending_state() {
        let session = Pubkey::new_unique();
        assert!(can_request_map_vrf(Pubkey::default(), VrfStatus::Requested));
        assert!(can_request_map_vrf(session, VrfStatus::Requested));
    }
}
