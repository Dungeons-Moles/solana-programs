use anchor_lang::prelude::*;

pub mod errors;
pub mod interactions;
pub mod offers;
pub mod pois;
pub mod spawn;
pub mod state;

use anchor_lang::context::CpiContext;
use ephemeral_rollups_sdk::anchor::{commit, delegate, ephemeral};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;
use errors::PoiSystemError;
use gameplay_state::state::{GameState, RunMode};
pub use pois::PoiDefinition;
use state::{ActiveCondition, MapPois, PoiVrfState, ShopState, UseType, MAP_POIS_SEED};
use vrf_rng::VrfStatus;

declare_id!("KiT25b86BSAF8yErcWwyuuWNaoXMpNf859NjH41TpSj");

/// Seed for POI authority PDA used to sign CPI calls to gameplay-state
pub const POI_AUTHORITY_SEED: &[u8] = b"poi_authority";
pub const NIGHT_VISION_RADIUS: u8 = 2;
pub const DAY_VISION_RADIUS: u8 = 4;
pub const SPAWN_VISION_RADIUS: u8 = 6;

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x58, 0x20, 0x64, 0x87, 0xdf, 0xd8, 0x68, 0xf1, 0xa4, 0x79, 0x15, 0x8b, 0xb2, 0x8a, 0x56, 0x0c,
    0xa9, 0x4f, 0x56, 0x2e, 0x62, 0x85, 0x26, 0xb7, 0x4f, 0x8b, 0xa1, 0x4d, 0x08, 0x36, 0x20, 0x99,
]);

/// Map generator program ID for reading GeneratedMap account
/// Must match the declare_id! in map-generator/src/lib.rs
/// GCy5GqvnJN99rgGtV6fMn8NtL9E7RoAyHDGzQv8me65j
pub const MAP_GENERATOR_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xe1, 0xf0, 0x18, 0x72, 0xcf, 0x4e, 0x1d, 0xea, 0xe0, 0x2f, 0x0a, 0xb0, 0xe8, 0xbf, 0x4b, 0x0c,
    0xf5, 0xb2, 0x05, 0xc5, 0x47, 0x61, 0x12, 0x2d, 0x49, 0xda, 0x54, 0xc1, 0xf5, 0xd0, 0xac, 0x6e,
]);
fn local_delegate_config(validator: Option<Pubkey>) -> DelegateConfig {
    DelegateConfig {
        validator,
        ..DelegateConfig::default()
    }
}

/// Validates and extracts VRF randomness from an optional PoiVrfState account.
///
/// Returns `Some((randomness, nonce))` if VRF is provided and valid.
/// Returns `None` if no VRF account is provided (campaign mode).
/// Errors if VRF is provided but invalid (wrong PDA, wrong status, etc.).
///
/// For PvP modes (Duel/Gauntlet), callers must require the result is `Some`.
fn extract_poi_vrf<'info>(
    poi_vrf_account: &Option<UncheckedAccount<'info>>,
    session_key: &Pubkey,
) -> Result<Option<([u8; 32], u64)>> {
    let account = match poi_vrf_account {
        Some(a) => a,
        None => return Ok(None),
    };

    // Validate PDA derivation
    let (expected_pda, _) = Pubkey::find_program_address(
        &[PoiVrfState::SEED_PREFIX, session_key.as_ref()],
        &crate::ID,
    );
    require_keys_eq!(
        account.key(),
        expected_pda,
        PoiSystemError::InvalidSession
    );

    // Validate owner
    require!(
        account.owner == &crate::ID,
        PoiSystemError::InvalidSession
    );

    // Deserialize and validate
    let data = account.try_borrow_data()?;
    let mut data_slice: &[u8] = &data;
    let vrf_state = PoiVrfState::try_deserialize(&mut data_slice)
        .map_err(|_| PoiSystemError::VrfNotFulfilled)?;

    require!(
        vrf_state.session == *session_key,
        PoiSystemError::InvalidSession
    );
    require!(
        vrf_state.status == VrfStatus::Fulfilled || vrf_state.status == VrfStatus::Consumed,
        PoiSystemError::VrfNotFulfilled
    );

    Ok(Some((vrf_state.randomness, vrf_state.nonce)))
}

/// Validates POI index, retrieves POI and definition, and validates interaction.
/// Use `skip_usage_check` for Repeatable/RepeatablePerTool POIs.
fn get_and_validate_poi<'a>(
    map_pois: &'a MapPois,
    game_state: &GameState,
    poi_index: u8,
    skip_usage_check: bool,
) -> Result<(&'a state::PoiInstance, pois::PoiDefinition)> {
    require!(
        (poi_index as usize) < map_pois.pois.len(),
        PoiSystemError::InvalidPoiIndex
    );

    let poi = &map_pois.pois[poi_index as usize];
    let poi_def = *pois::get_poi_definition(poi.poi_type).ok_or(PoiSystemError::InvalidPoiType)?;

    // Position check
    require!(
        game_state.position_x == poi.x && game_state.position_y == poi.y,
        PoiSystemError::PlayerNotOnPoiTile
    );

    // Usage check (unless skipped for repeatable POIs)
    if !skip_usage_check && poi_def.use_type == UseType::OneTime {
        require!(!poi.used, PoiSystemError::PoiAlreadyUsed);
    }

    // Time check
    if poi_def.active_condition == ActiveCondition::NightOnly {
        require!(game_state.phase.is_night(), PoiSystemError::NightOnlyPoi);
    }

    Ok((poi, poi_def))
}

fn require_player_owns_game_state(game_state: &GameState, player: &Signer<'_>) -> Result<()> {
    require_keys_eq!(
        game_state.session_signer,
        player.key(),
        PoiSystemError::Unauthorized
    );
    Ok(())
}

fn is_within_visibility_radius(center_x: u8, center_y: u8, x: u8, y: u8, radius: u8) -> bool {
    let dx = center_x as i16 - x as i16;
    let dy = center_y as i16 - y as i16;
    let distance_sq = dx * dx + dy * dy;
    let r = radius as i16;
    distance_sq <= r * (r + 1)
}

fn tier_from_u8(tier: u8) -> Result<player_inventory::state::Tier> {
    match tier {
        1 => Ok(player_inventory::state::Tier::I),
        2 => Ok(player_inventory::state::Tier::II),
        3 => Ok(player_inventory::state::Tier::III),
        _ => Err(PoiSystemError::InvalidInteraction.into()),
    }
}

fn oil_modification_from_flag(
    modification: u8,
) -> Result<player_inventory::state::ToolOilModification> {
    match modification {
        interactions::OIL_FLAG_ATK => Ok(player_inventory::state::ToolOilModification::PlusAtk),
        interactions::OIL_FLAG_SPD => Ok(player_inventory::state::ToolOilModification::PlusSpd),
        interactions::OIL_FLAG_DIG => Ok(player_inventory::state::ToolOilModification::PlusDig),
        interactions::OIL_FLAG_ARM => Ok(player_inventory::state::ToolOilModification::PlusArm),
        _ => Err(PoiSystemError::InvalidInteraction.into()),
    }
}

fn find_matching_gear_slots(
    inventory: &player_inventory::state::PlayerInventory,
    item_id: [u8; 8],
    tier: player_inventory::state::Tier,
) -> Option<(u8, u8)> {
    let mut first: Option<u8> = None;
    for slot_idx in 0..inventory.gear_slot_capacity as usize {
        if let Some(item) = inventory.gear[slot_idx] {
            if item.item_id == item_id && item.tier == tier {
                if let Some(first_idx) = first {
                    return Some((first_idx, slot_idx as u8));
                }
                first = Some(slot_idx as u8);
            }
        }
    }
    None
}

/// Converts game_state.week (1-3) to boss_system::Week enum.
fn to_boss_week(week: u8) -> Result<boss_system::Week> {
    match week {
        1 => Ok(boss_system::Week::One),
        2 => Ok(boss_system::Week::Two),
        3 => Ok(boss_system::Week::Three),
        _ => Err(PoiSystemError::InvalidWeek.into()),
    }
}

/// CPI call to player-inventory::equip_gear_authorized
#[allow(clippy::too_many_arguments)]
fn equip_gear_authorized_cpi<'info>(
    inventory: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    inventory_authority: &AccountInfo<'info>,
    poi_authority: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
    gameplay_state_program: &AccountInfo<'info>,
    poi_authority_bump: u8,
    item_id: [u8; 8],
    tier: player_inventory::state::Tier,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[POI_AUTHORITY_SEED, &[poi_authority_bump]]];

    player_inventory::cpi::equip_gear_authorized(
        CpiContext::new_with_signer(
            player_inventory_program.clone(),
            player_inventory::cpi::accounts::EquipGearAuthorized {
                inventory: inventory.clone(),
                game_state: game_state.clone(),
                inventory_authority: inventory_authority.clone(),
                poi_authority: poi_authority.clone(),
                gameplay_state_program: gameplay_state_program.clone(),
            },
            signer_seeds,
        ),
        item_id,
        tier,
    )?;

    Ok(())
}

/// CPI call to player-inventory::equip_tool_authorized
#[allow(clippy::too_many_arguments)]
fn equip_tool_authorized_cpi<'info>(
    inventory: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    inventory_authority: &AccountInfo<'info>,
    poi_authority: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
    gameplay_state_program: &AccountInfo<'info>,
    poi_authority_bump: u8,
    item_id: [u8; 8],
    tier: player_inventory::state::Tier,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[POI_AUTHORITY_SEED, &[poi_authority_bump]]];

    player_inventory::cpi::equip_tool_authorized(
        CpiContext::new_with_signer(
            player_inventory_program.clone(),
            player_inventory::cpi::accounts::EquipToolAuthorized {
                inventory: inventory.clone(),
                game_state: game_state.clone(),
                inventory_authority: inventory_authority.clone(),
                poi_authority: poi_authority.clone(),
                gameplay_state_program: gameplay_state_program.clone(),
            },
            signer_seeds,
        ),
        item_id,
        tier,
    )?;

    Ok(())
}

fn offer_tier_to_inventory_tier(tier: u8) -> player_inventory::state::Tier {
    match tier {
        0 => player_inventory::state::Tier::I,
        1 => player_inventory::state::Tier::II,
        2 => player_inventory::state::Tier::III,
        _ => player_inventory::state::Tier::I,
    }
}

#[allow(clippy::too_many_arguments)]
fn equip_item_authorized_cpi<'info>(
    inventory: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    inventory_authority: &AccountInfo<'info>,
    poi_authority: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
    gameplay_state_program: &AccountInfo<'info>,
    poi_authority_bump: u8,
    item_id: [u8; 8],
    item_tier: u8,
) -> Result<()> {
    let tier = offer_tier_to_inventory_tier(item_tier);
    if item_id[0] == b'T' {
        equip_tool_authorized_cpi(
            inventory,
            game_state,
            inventory_authority,
            poi_authority,
            player_inventory_program,
            gameplay_state_program,
            poi_authority_bump,
            item_id,
            tier,
        )
    } else {
        equip_gear_authorized_cpi(
            inventory,
            game_state,
            inventory_authority,
            poi_authority,
            player_inventory_program,
            gameplay_state_program,
            poi_authority_bump,
            item_id,
            tier,
        )
    }
}

fn copy_shop_offers(shop_state: &mut ShopState, filtered: &[state::ItemOffer], reset_first: bool) {
    if reset_first {
        shop_state.offers = [state::ItemOffer::default(); state::SHOP_OFFER_COUNT];
    }
    for (dst, src) in shop_state.offers.iter_mut().zip(filtered.iter()) {
        *dst = *src;
    }
}

#[ephemeral]
#[program]
pub mod poi_system {
    use super::*;

    /// Initializes POI state for a session by copying POIs from the generated map.
    ///
    /// The generated_map account contains POIs placed during map generation.
    /// This instruction copies them to the MapPois account for runtime management.
    pub fn initialize_map_pois(
        ctx: Context<InitializeMapPois>,
        act: u8,
        week: u8,
        seed: u64,
    ) -> Result<()> {
        require_keys_eq!(
            *ctx.accounts.session.owner,
            SESSION_MANAGER_PROGRAM_ID,
            PoiSystemError::InvalidSessionOwner
        );
        require!((1..=4).contains(&act), PoiSystemError::InvalidAct);

        // Read POI data from the generated map account
        let generated_map_info = &ctx.accounts.generated_map;
        let generated_map_data = generated_map_info.try_borrow_data()?;

        // Validate minimum size: 8 (discriminator) + 32 (session) + basic fields
        require!(
            generated_map_data.len() > 8 + 32 + 1 + 1 + 8 + 1 + 1 + 1 + 1 + 2 + 313 + 1 + (48 * 4),
            PoiSystemError::InvalidGeneratedMap
        );

        // Parse generated map fields:
        // Offset: 8 (discriminator) + 32 (session) + 1 (width) + 1 (height) + 8 (seed)
        //       + 1 (spawn_x) + 1 (spawn_y) + 1 (mole_den_x) + 1 (mole_den_y)
        //       + 2 (walkable_count) + 313 (packed_tiles) + 1 (enemy_count)
        //       + 192 (enemies: 48 * 4) = 562
        // poi_count is at offset 562
        let poi_count_offset = 8 + 32 + 1 + 1 + 8 + 1 + 1 + 1 + 1 + 2 + 313 + 1 + (48 * 4);
        let poi_count = generated_map_data[poi_count_offset] as usize;

        // POIs start at offset 563, each POI is 4 bytes: (poi_type, is_used, x, y)
        let pois_offset = poi_count_offset + 1;

        // Initialize the MapPois account
        let map_pois = &mut ctx.accounts.map_pois;
        map_pois.session = ctx.accounts.session.key();
        map_pois.bump = ctx.bumps.map_pois;
        map_pois.act = act;
        map_pois.week = week;
        map_pois.seed = seed;
        map_pois.shop_state = ShopState::default();

        // ABI NOTE:
        // `generated_map_data` stores raw POI type IDs encoded by map-generator (1..=14).
        // The ID mapping is a cross-program ABI and must remain stable between map-generator
        // placement logic and poi-system POI definitions (`pois::L*_*.id` constants).
        //
        // Counter Cache (L13) is boss-prep content and must not appear in PvP run modes.
        let exclude_counter_cache = matches!(
            ctx.accounts.game_state.run_mode,
            RunMode::Duel | RunMode::Gauntlet
        );

        // Copy POIs from generated map to MapPois
        let mut pois = Vec::with_capacity(poi_count);
        for i in 0..poi_count {
            let poi_start = pois_offset + (i * 4);
            if poi_start + 4 > generated_map_data.len() {
                break;
            }

            let poi_type = generated_map_data[poi_start];
            if exclude_counter_cache && poi_type == pois::L13_COUNTER_CACHE.id {
                continue;
            }
            let is_used = generated_map_data[poi_start + 1] != 0;
            let x = generated_map_data[poi_start + 2];
            let y = generated_map_data[poi_start + 3];

            pois.push(state::PoiInstance {
                poi_type,
                x,
                y,
                used: is_used,
                discovered: poi_type == pois::L1_MOLE_DEN.id, // Mole Den (L1) is always discovered
                week_spawned: week,
            });
        }

        map_pois.count = pois.len() as u8;
        map_pois.pois = pois;

        emit!(PoisInitialized {
            session: map_pois.session,
            count: map_pois.count,
            act,
        });

        Ok(())
    }

    /// Delegates map-pois PDA to MagicBlock from poi-system (its owner program).
    pub fn delegate_map_pois(
        ctx: Context<DelegateMapPois>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let session_key = ctx.accounts.game_session.key();
        let (expected_map_pois, _) =
            Pubkey::find_program_address(&[MAP_POIS_SEED, session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.map_pois.key(),
            expected_map_pois,
            PoiSystemError::Unauthorized
        );
        let map_pois_seeds: &[&[u8]] = &[MAP_POIS_SEED, session_key.as_ref()];
        ctx.accounts.delegate_map_pois(
            &ctx.accounts.player,
            map_pois_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Commits and undelegates map-pois PDA from ER back to base layer.
    pub fn undelegate_map_pois(ctx: Context<UndelegateMapPois>) -> Result<()> {
        use session_manager::state::GameSession;

        let session_key = ctx.accounts.game_session.key();
        let (expected_map_pois, _) =
            Pubkey::find_program_address(&[MAP_POIS_SEED, session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.map_pois.key(),
            expected_map_pois,
            PoiSystemError::Unauthorized
        );
        let map_pois = read_map_pois(&ctx.accounts.map_pois)?;
        require_keys_eq!(
            map_pois.session,
            session_key,
            PoiSystemError::Unauthorized
        );

        let session_signer_end = GameSession::SESSION_SIGNER_OFFSET + 32;
        let session_data = ctx.accounts.game_session.try_borrow_data()?;
        require!(
            session_data.len() >= session_signer_end,
            PoiSystemError::InvalidSession
        );
        let stored_session_signer = Pubkey::from(
            <[u8; 32]>::try_from(
                &session_data[GameSession::SESSION_SIGNER_OFFSET..session_signer_end],
            )
            .unwrap(),
        );
        require_keys_eq!(
            stored_session_signer,
            ctx.accounts.session_signer.key(),
            PoiSystemError::Unauthorized
        );
        drop(session_data);

        let map_pois_info = ctx.accounts.map_pois.to_account_info();
        commit_and_undelegate_accounts(
            &ctx.accounts.session_signer.to_account_info(),
            vec![&map_pois_info],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
        )?;
        Ok(())
    }

    /// Close MapPois account, returning rent to the session owner.
    pub fn close_map_pois(ctx: Context<CloseMapPois>) -> Result<()> {
        use session_manager::state::GameSession;
        let session_data = ctx.accounts.game_session.try_borrow_data()?;
        require!(session_data.len() >= 40, PoiSystemError::Unauthorized);
        let player_end = GameSession::PLAYER_OFFSET + 32;
        let stored_player = Pubkey::from(
            <[u8; 32]>::try_from(&session_data[GameSession::PLAYER_OFFSET..player_end]).unwrap(),
        );
        require!(
            stored_player == ctx.accounts.player.key(),
            PoiSystemError::Unauthorized
        );
        drop(session_data);

        emit!(PoisClosed {
            session: ctx.accounts.map_pois.session,
        });
        Ok(())
    }

    /// Close MapPois account via session key signer authorization.
    /// Used by session-manager CPI during end_session to clean up.
    /// Rent is returned to the player wallet.
    pub fn close_map_pois_via_session_signer(
        ctx: Context<CloseMapPoisViaSessionSigner>,
    ) -> Result<()> {
        use session_manager::state::GameSession;
        let player_end = GameSession::PLAYER_OFFSET + 32;
        let session_signer_end = GameSession::SESSION_SIGNER_OFFSET + 32;

        let session_data = ctx.accounts.game_session.try_borrow_data()?;
        require!(
            session_data.len() >= session_signer_end,
            PoiSystemError::InvalidSession
        );

        let stored_player = Pubkey::from(
            <[u8; 32]>::try_from(&session_data[GameSession::PLAYER_OFFSET..player_end]).unwrap(),
        );
        let stored_session_signer = Pubkey::from(
            <[u8; 32]>::try_from(
                &session_data[GameSession::SESSION_SIGNER_OFFSET..session_signer_end],
            )
            .unwrap(),
        );

        require!(
            stored_session_signer == ctx.accounts.session_signer.key(),
            PoiSystemError::Unauthorized
        );
        require!(
            stored_player == ctx.accounts.player.key(),
            PoiSystemError::Unauthorized
        );
        drop(session_data);

        emit!(PoisClosed {
            session: ctx.accounts.map_pois.session,
        });
        Ok(())
    }

    /// Close MapPois account when the session PDA no longer exists (orphaned).
    /// Validates session_signer and player via the GameState account instead.
    /// Used by frontend orphaned-account cleanup when force_close_session
    /// already closed the session PDA but some children were still delegated.
    pub fn close_map_pois_orphaned(ctx: Context<CloseMapPoisOrphaned>) -> Result<()> {
        emit!(PoisClosed {
            session: ctx.accounts.map_pois.session,
        });
        Ok(())
    }

    /// Get POI definition by type ID (view function).
    /// Returns POI properties for UI/client consumption.
    pub fn get_poi_definition(_ctx: Context<GetPoiDefinition>, poi_type: u8) -> Result<()> {
        let def = pois::get_poi_definition(poi_type).ok_or(PoiSystemError::InvalidPoiType)?;

        emit!(PoiDefinitionQueried {
            poi_type: def.id,
            name: def.name.to_string(),
            rarity: def.rarity as u8,
            use_type: def.use_type as u8,
            active_condition: def.active_condition as u8,
            interaction_type: def.interaction_type as u8,
            category: def.category as u8,
        });

        Ok(())
    }

    /// Interact with a rest POI (L1 Mole Den or L5 Rest Alcove).
    ///
    /// - L1: Full heal, repeatable, night-only, skip to day
    /// - L5: Heal 10 HP, one-time, night-only, skip to day
    ///
    /// This instruction validates the interaction, marks the POI as used (if applicable),
    /// heals the player, and skips to the next day phase via CPI to gameplay-state.
    /// If used during Night3, triggers the boss fight (cannot skip end-of-week boss).
    pub fn interact_rest(ctx: Context<InteractRest>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;
        let inventory = &ctx.accounts.inventory;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;
        let player_stats = gameplay_state::stats::calculate_stats(
            inventory,
            game_state.campaign_level,
            game_state.run_mode,
        );

        // Get values for rest interaction (i16 to handle HP > 255 or negative values)
        let current_hp = game_state.hp;
        let max_hp = player_stats.max_hp;
        let is_night = game_state.phase.is_night();

        // Execute rest interaction
        let result = interactions::execute_rest_interaction(poi, current_hp, max_hp, is_night)?;

        // Mark POI as used if needed
        if result.mark_used {
            map_pois.pois[poi_index as usize].used = true;
        }

        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];

        // CPI to gameplay-state to heal player atomically
        if result.heal_amount > 0 {
            gameplay_state::cpi::heal_player(
                CpiContext::new_with_signer(
                    ctx.accounts.gameplay_state_program.to_account_info(),
                    gameplay_state::cpi::accounts::HealPlayer {
                        game_state: ctx.accounts.game_state.to_account_info(),
                        inventory: ctx.accounts.inventory.to_account_info(),
                        poi_authority: ctx.accounts.poi_authority.to_account_info(),
                    },
                    &[&seeds[..]],
                ),
                result.heal_amount,
            )?;
        }

        // CPI to gameplay-state to skip to day (or trigger boss fight if Night3)
        gameplay_state::cpi::skip_to_day(CpiContext::new_with_signer(
            ctx.accounts.gameplay_state_program.to_account_info(),
            gameplay_state::cpi::accounts::SkipToDay {
                game_state: ctx.accounts.game_state.to_account_info(),
                inventory: ctx.accounts.inventory.to_account_info(),
                generated_map: ctx.accounts.generated_map.to_account_info(),
                poi_authority: ctx.accounts.poi_authority.to_account_info(),
                gameplay_authority: ctx.accounts.gameplay_authority.to_account_info(),
                player_inventory_program: ctx.accounts.player_inventory_program.to_account_info(),
                gameplay_vrf_state: ctx.accounts.gameplay_vrf_state.as_ref().map(|a| a.to_account_info()),
            },
            &[&seeds[..]],
        ))?;

        let poi = &map_pois.pois[poi_index as usize];
        emit!(RestCompleted {
            session: map_pois.session,
            poi_type: poi.poi_type,
            x: poi.x,
            y: poi.y,
            heal_amount: result.heal_amount,
            full_heal: result.full_heal,
        });

        Ok(())
    }

    /// Generate and store cache offers for a pick-item POI (L2, L3, L12, L13).
    ///
    /// This instruction generates offers using an on-chain derived seed (Clock)
    /// and stores them in `MapPois.cache_offers` so the frontend can read them.
    /// The user then calls `interact_pick_item` with their chosen index.
    ///
    /// Offers persist for the entire session — revisiting a POI returns the
    /// same offer instead of regenerating (VRF-ready).
    pub fn generate_cache_offer(ctx: Context<InteractPickItem>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        // Extract VRF data if provided; require for PvP modes
        let session_key = map_pois.session;
        let vrf_data = extract_poi_vrf(&ctx.accounts.poi_vrf_state, &session_key)?;
        require!(vrf_data.is_some(), PoiSystemError::VrfRequired);

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;
        let act = map_pois.act;

        // Reject if an offer already exists for this exact POI (prevents rerolls).
        require!(
            !map_pois
                .cache_offers
                .iter()
                .any(|o| o.poi_index == poi_index),
            PoiSystemError::OfferAlreadyGenerated
        );

        // When VRF is available, derive seed from VRF with domain separation.
        // Otherwise, use legacy deterministic seed.
        let seed = if let Some((ref randomness, nonce)) = vrf_data {
            let offer_ctx = offers::OfferContext::new(act, game_state.week, 0, poi_index);
            let vrf_ref: Option<(&[u8; 32], u64)> = Some((randomness, nonce));
            let mut rng = offer_ctx.create_rng(vrf_ref);
            rng.next_val()
        } else {
            map_pois.seed
                ^ ((poi_index as u64) << 8)
                ^ ((act as u64) << 16)
                ^ ((game_state.total_moves as u64) << 32)
        };

        // Fetch boss weaknesses on-chain
        let week = to_boss_week(game_state.week)?;
        let weaknesses = boss_system::get_boss_weaknesses(game_state.campaign_level, week)
            .map_err(|_| PoiSystemError::InvalidBossWeek)?;
        let w1 = offers::WeaknessTag::try_from(weaknesses[0] as u8)
            .unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weaknesses[1] as u8)
            .unwrap_or(offers::WeaknessTag::Frost);

        let pool = &ctx.accounts.game_session.active_item_pool;

        // Retry generation with different seeds until we have 3 pool-valid items.
        // Each attempt generates 3 candidates; we keep ones that are in the pool
        // and not yet collected. This prevents empty slots when the pool filters
        // out some generated items.
        let mut items = [state::OfferItem::default(); 3];
        let mut count = 0usize;
        let mut used_ids = [[0u8; 8]; 3];

        for attempt in 0..10u64 {
            let attempt_seed = seed ^ attempt.wrapping_mul(0x9e3779b97f4a7c15);
            let generated = offers::generate_poi_offers(poi_type, act, w1, w2, attempt_seed)
                .ok_or(PoiSystemError::InvalidInteraction)?;

            for offer in &generated.offers {
                // Skip duplicates
                if used_ids[..count].contains(&offer.item_id) {
                    continue;
                }
                // Check pool membership
                if let Some(index) = offers::item_id_to_pool_index(&offer.item_id) {
                    if offers::is_item_in_pool(pool, index) {
                        used_ids[count] = offer.item_id;
                        items[count] = state::OfferItem {
                            item_id: offer.item_id,
                            rarity: offers::rarity_from_item_id(&offer.item_id),
                            tier: offer.tier,
                        };
                        count += 1;
                        if count >= 3 {
                            break;
                        }
                    }
                }
            }

            if count >= 3 {
                break;
            }
        }

        // Deterministic fallback: if normal generation+pool filtering could not fill all 3
        // slots (e.g., very restrictive active item pool), backfill from the active pool
        // with POI-compatible items so frontend always receives 3 visible options.
        if count < 3 {
            let matches_weakness = |item_id: &[u8; 8], weakness: offers::WeaknessTag| -> bool {
                match weakness {
                    offers::WeaknessTag::Stone => item_id[2] == b'S' && item_id[3] == b'T',
                    offers::WeaknessTag::Scout => item_id[2] == b'S' && item_id[3] == b'C',
                    offers::WeaknessTag::Greed => item_id[2] == b'G' && item_id[3] == b'R',
                    offers::WeaknessTag::Blast => item_id[2] == b'B' && item_id[3] == b'L',
                    offers::WeaknessTag::Frost => item_id[2] == b'F' && item_id[3] == b'R',
                    offers::WeaknessTag::Rust => item_id[2] == b'R' && item_id[3] == b'U',
                    offers::WeaknessTag::Blood => item_id[2] == b'B' && item_id[3] == b'O',
                    offers::WeaknessTag::Tempo => item_id[2] == b'T' && item_id[3] == b'E',
                }
            };

            let accepts_for_poi = |item_id: &[u8; 8]| -> bool {
                match poi_type {
                    2 => item_id[0] == b'G', // Supply Cache: gear
                    3 => item_id[0] == b'T', // Tool Crate: tools
                    12 => item_id[0] == b'G' && offers::rarity_from_item_id(item_id) >= 2, // Geode: Heroic+
                    13 => {
                        item_id[0] == b'G'
                            && (matches_weakness(item_id, w1) || matches_weakness(item_id, w2))
                    } // Counter: weakness-tagged gear
                    _ => false,
                }
            };

            let mut fallback_candidates: Vec<[u8; 8]> = Vec::new();
            for (index, item) in player_inventory::items::ITEMS.iter().enumerate() {
                let pool_index = index as u8;
                if !offers::is_item_in_pool(pool, pool_index) {
                    continue;
                }
                let item_id = *item.id;
                if !accepts_for_poi(&item_id) {
                    continue;
                }
                if used_ids[..count].contains(&item_id) {
                    continue;
                }
                fallback_candidates.push(item_id);
            }

            if !fallback_candidates.is_empty() {
                let mut cursor = (seed as usize) % fallback_candidates.len();
                let mut scanned = 0usize;
                while count < 3 && scanned < fallback_candidates.len() {
                    let item_id = fallback_candidates[cursor];
                    if !used_ids[..count].contains(&item_id) {
                        used_ids[count] = item_id;
                        items[count] = state::OfferItem {
                            item_id,
                            rarity: offers::rarity_from_item_id(&item_id),
                            tier: 0,
                        };
                        count += 1;
                    }
                    cursor = (cursor + 1) % fallback_candidates.len();
                    scanned += 1;
                }
            }

            // Absolute last resort: duplicate a valid offer so UI still gets 3 choices.
            if count < 3 {
                let fallback_id = if count > 0 {
                    used_ids[0]
                } else if poi_type == 3 {
                    *b"T-ST-01\0"
                } else {
                    *b"G-ST-01\0"
                };

                while count < 3 {
                    used_ids[count] = fallback_id;
                    items[count] = state::OfferItem {
                        item_id: fallback_id,
                        rarity: offers::rarity_from_item_id(&fallback_id),
                        tier: 0,
                    };
                    count += 1;
                }
            }
        }

        map_pois.cache_offers.push(state::CacheOffer {
            poi_index,
            items,
            generated_at_seed: seed,
        });

        emit!(CacheOfferGenerated {
            session: map_pois.session,
            poi_index,
            poi_type,
            item0: items[0].item_id,
            item1: items[1].item_id,
            item2: items[2].item_id,
        });

        Ok(())
    }

    /// Interact with a pick-item POI (L2, L3, L12, L13).
    ///
    /// - L2 (Supply Cache): Pick 1 of 3 Gear
    /// - L3 (Tool Crate): Pick 1 of 3 Tools
    /// - L12 (Geode Vault): Pick 1 of 3 Heroic+ items
    /// - L13 (Counter Cache): Pick 1 of 3 weakness-tagged items
    ///
    /// Requires `generate_cache_offer` to have been called first.
    /// Reads the stored offer from `cache_offers` and applies the user's choice.
    ///
    /// After validating the pick, this instruction calls player-inventory to
    /// equip the item via CPI (equip_gear_authorized or equip_tool_authorized).
    /// This ensures items can only be equipped through authorized POI interactions.
    pub fn interact_pick_item(
        ctx: Context<InteractPickItem>,
        poi_index: u8,
        choice_index: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;
        let x = poi.x;
        let y = poi.y;
        let is_night = game_state.phase.is_night();

        // Find the saved offer for this POI
        let offer_pos = map_pois
            .cache_offers
            .iter()
            .position(|o| o.poi_index == poi_index)
            .ok_or(PoiSystemError::NoActiveInteraction)?;
        let cached = map_pois.cache_offers[offer_pos];

        // Convert cached OfferItems to ItemOffers for pick interaction
        let offers: Vec<state::ItemOffer> = cached
            .items
            .iter()
            .map(|item| state::ItemOffer {
                item_id: item.item_id,
                tier: item.tier,
                price: 0,
                purchased: false,
            })
            .collect();

        // Execute pick interaction
        let poi = &map_pois.pois[poi_index as usize];
        let result =
            interactions::execute_pick_item_interaction(poi, &offers, choice_index, is_night)?;

        equip_item_authorized_cpi(
            &ctx.accounts.inventory.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.inventory_authority,
            &ctx.accounts.poi_authority,
            &ctx.accounts.player_inventory_program.to_account_info(),
            &ctx.accounts.gameplay_state_program.to_account_info(),
            ctx.bumps.poi_authority,
            result.item.item_id,
            result.item.tier,
        )?;

        // Mark POI as used
        if result.mark_used {
            map_pois.pois[poi_index as usize].used = true;
        }

        // Remove the consumed offer from saved offers
        map_pois.cache_offers.swap_remove(offer_pos);

        emit!(ItemPicked {
            session: map_pois.session,
            poi_type,
            item_id: result.item.item_id,
            tier: result.item.tier,
        });

        emit!(PoiInteracted {
            session: map_pois.session,
            poi_type,
            x,
            y,
            interaction: "pick_item".to_string(),
        });

        Ok(())
    }

    /// Generate and store oil offers for a Tool Oil Rack (L4).
    ///
    /// This instruction generates 3 of 4 possible oils using an on-chain derived seed
    /// and stores them in `MapPois.oil_offers` so the frontend can read them.
    /// The user then calls `interact_tool_oil` with their chosen oil.
    ///
    /// Offers persist for the entire session — revisiting a POI returns the
    /// same offer instead of regenerating (VRF-ready).
    pub fn generate_oil_offer(ctx: Context<InteractToolOil>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        // Extract VRF data if provided; require for PvP modes
        let session_key = map_pois.session;
        let vrf_data = extract_poi_vrf(&ctx.accounts.poi_vrf_state, &session_key)?;
        require!(vrf_data.is_some(), PoiSystemError::VrfRequired);

        // Validate POI (don't skip usage check - Tool Oil is one-time use)
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;

        // Validate it's a Tool Oil Rack (L4 = poi_type 4)
        require!(poi_type == 4, PoiSystemError::InvalidInteraction);

        // Reject if an offer already exists for this exact POI (prevents rerolls).
        require!(
            !map_pois.oil_offers.iter().any(|o| o.poi_index == poi_index),
            PoiSystemError::OfferAlreadyGenerated
        );

        // When VRF is available, derive seed from VRF with tool oil domain.
        // Otherwise, use legacy deterministic seed.
        let seed = if let Some((ref randomness, nonce)) = vrf_data {
            let mut rng = vrf_rng::GameRng::from_vrf(
                randomness,
                nonce,
                vrf_rng::domains::POI_TOOL_OIL ^ ((poi_index as u64) << 16),
            );
            rng.next_val()
        } else {
            map_pois.seed
                ^ ((poi_index as u64) << 8)
                ^ ((game_state.total_moves as u64) << 32)
                ^ 0x4f494c_u64 // "OIL" domain separator
        };

        // Generate oil offer
        let oil_offer = offers::create_oil_offer(poi_index, seed);

        // Persist in MapPois
        map_pois.oil_offers.push(oil_offer);

        emit!(OilOfferGenerated {
            session: map_pois.session,
            poi_index,
            oils: oil_offer.oils,
        });

        Ok(())
    }

    /// Interact with a Tool Oil Rack (L4).
    ///
    /// Applies +1 to ATK, SPD, DIG, or ARM on the player's current tool.
    /// This is a one-time use POI - user picks one oil and the POI is consumed.
    ///
    /// Requires `generate_oil_offer` to have been called first.
    /// Validates the selected oil is one of the 3 generated offers.
    ///
    /// Arguments:
    /// - `poi_index`: Index of the POI in map_pois.pois
    /// - `current_oil_flags`: Current tool oil flags (unused, kept for API compat)
    /// - `modification`: Oil type (1=ATK, 2=SPD, 4=DIG, 8=ARM)
    pub fn interact_tool_oil(
        ctx: Context<InteractToolOil>,
        poi_index: u8,
        _current_oil_flags: u8,
        modification: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        // Validate POI (don't skip usage check - Tool Oil is one-time use)
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let poi_type = poi.poi_type;
        let x = poi.x;
        let y = poi.y;
        let is_night = game_state.phase.is_night();

        // Find the saved oil offer for this POI
        let offer_pos = map_pois
            .oil_offers
            .iter()
            .position(|o| o.poi_index == poi_index)
            .ok_or(PoiSystemError::NoActiveInteraction)?;
        let oil_offer = map_pois.oil_offers[offer_pos];
        require!(
            offers::validate_oil_selection(&oil_offer, modification),
            PoiSystemError::InvalidOilSelection
        );

        let current_oil_flags = ctx
            .accounts
            .inventory
            .tool
            .ok_or(PoiSystemError::NoToolEquipped)?
            .tool_oil_flags;

        // Tool oil is free (no gold cost) per balance-fixes-revised spec
        let result = interactions::execute_tool_oil_interaction(
            poi,
            current_oil_flags,
            modification,
            0, // act - unused, tool oil is free
            0, // player_gold - unused, tool oil is free
            is_night,
        )?;

        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        player_inventory::cpi::apply_tool_oil_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.player_inventory_program.to_account_info(),
                player_inventory::cpi::accounts::ApplyToolOilAuthorized {
                    inventory: ctx.accounts.inventory.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            oil_modification_from_flag(result.modification)?,
        )?;

        // Mark POI as used (one-time use)
        map_pois.pois[poi_index as usize].used = true;

        // Remove the consumed offer from saved offers
        map_pois.oil_offers.swap_remove(offer_pos);

        emit!(ToolOilApplied {
            session: map_pois.session,
            modification: result.modification,
        });

        emit!(PoiInteracted {
            session: map_pois.session,
            poi_type,
            x,
            y,
            interaction: "tool_oil".to_string(),
        });

        Ok(())
    }

    // =========================================================================
    // Shop Instructions (L9 Smuggler Hatch)
    // =========================================================================

    /// Enter the Smuggler Hatch shop (L9).
    ///
    /// Generates 6 offers (1 Tool + 5 Gear) and starts a shopping session.
    /// Only one shop session can be active at a time.
    pub fn enter_shop(ctx: Context<EnterShop>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        // Extract VRF data if provided; require for PvP modes
        let session_key = map_pois.session;
        let vrf_data = extract_poi_vrf(&ctx.accounts.poi_vrf_state, &session_key)?;
        require!(vrf_data.is_some(), PoiSystemError::VrfRequired);

        require!(
            !map_pois.shop_state.active,
            PoiSystemError::ShopAlreadyActive
        );

        // L9 is Repeatable, so skip usage check
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, true)?;

        let act = map_pois.act;
        let is_night = game_state.phase.is_night();

        interactions::validate_shop_poi(poi, is_night)?;

        // When VRF is available, derive seed from VRF with smuggler hatch domain.
        // Otherwise, use legacy deterministic seed.
        let seed = if let Some((ref randomness, nonce)) = vrf_data {
            let mut rng = vrf_rng::GameRng::from_vrf(
                randomness,
                nonce,
                vrf_rng::domains::POI_SMUGGLER_HATCH ^ ((poi_index as u64) << 16),
            );
            rng.next_val()
        } else {
            map_pois.seed
                ^ ((poi_index as u64) << 8)
                ^ ((act as u64) << 16)
                ^ ((game_state.total_moves as u64) << 32)
        };

        // Fetch boss weaknesses on-chain
        let week = to_boss_week(game_state.week)?;
        let weaknesses = boss_system::get_boss_weaknesses(game_state.campaign_level, week)
            .map_err(|_| PoiSystemError::InvalidBossWeek)?;
        let w1 = offers::WeaknessTag::try_from(weaknesses[0] as u8)
            .unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weaknesses[1] as u8)
            .unwrap_or(offers::WeaknessTag::Frost);

        let pool = &ctx.accounts.game_session.active_item_pool;
        let generated = offers::generate_smuggler_hatch_offers(act, w1, w2, seed, pool);

        // Initialize shop state
        map_pois.shop_state.active = true;
        map_pois.shop_state.poi_index = poi_index;
        map_pois.shop_state.reroll_count = 0;
        map_pois.shop_state.rng_state = seed;

        copy_shop_offers(&mut map_pois.shop_state, &generated.offers, false);

        emit!(ShopEntered {
            session: map_pois.session,
            poi_index,
        });

        Ok(())
    }

    /// Purchase an item from the active shop.
    ///
    /// Validates player has enough gold, marks the offer as purchased,
    /// deducts gold via CPI to gameplay-state, and equips the item via CPI
    /// to player-inventory.
    pub fn shop_purchase(ctx: Context<ShopPurchase>, offer_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;
        let player_gold = game_state.gold;

        // Execute purchase validation
        let (offer, price) =
            interactions::execute_shop_purchase(&map_pois.shop_state, offer_index, player_gold)?;

        // Mark offer as purchased
        map_pois.shop_state.offers[offer_index as usize].purchased = true;

        // CPI to gameplay-state to deduct gold atomically
        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            -(price as i16),
        )?;

        equip_item_authorized_cpi(
            &ctx.accounts.inventory.to_account_info(),
            &ctx.accounts.game_state.to_account_info(),
            &ctx.accounts.inventory_authority,
            &ctx.accounts.poi_authority,
            &ctx.accounts.player_inventory_program.to_account_info(),
            &ctx.accounts.gameplay_state_program.to_account_info(),
            ctx.bumps.poi_authority,
            offer.item_id,
            offer.tier,
        )?;

        emit!(ItemPurchased {
            session: map_pois.session,
            item_id: offer.item_id,
            price,
        });

        Ok(())
    }

    /// Reroll the shop offers for a gold cost.
    ///
    /// Cost increases with each reroll: 4, 6, 8, ...
    /// Gold is deducted atomically via CPI to gameplay-state.
    pub fn shop_reroll(ctx: Context<ShopReroll>) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;
        let player_gold = game_state.gold;

        require!(
            map_pois.shop_state.reroll_count < 3,
            PoiSystemError::ShopRerollLimitReached
        );

        // Calculate and validate reroll cost
        let cost = interactions::calculate_shop_reroll_cost(&map_pois.shop_state)?;

        require!(player_gold >= cost, PoiSystemError::InsufficientGold);

        // CPI to gameplay-state to deduct gold atomically
        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            -(cost as i16),
        )?;

        // Increment reroll count
        map_pois.shop_state.reroll_count = map_pois.shop_state.reroll_count.saturating_add(1);

        // Deterministic reroll sequence anchored to entry seed.
        let seed = map_pois.shop_state.rng_state
            ^ ((map_pois.shop_state.reroll_count as u64) << 8)
            ^ ((game_state.total_moves as u64) << 32);
        map_pois.shop_state.rng_state = seed.rotate_left(13) ^ 0x9e37_79b9_7f4a_7c15_u64;

        // Fetch boss weaknesses on-chain
        let week = to_boss_week(game_state.week)?;
        let weaknesses = boss_system::get_boss_weaknesses(game_state.campaign_level, week)
            .map_err(|_| PoiSystemError::InvalidBossWeek)?;
        let w1 = offers::WeaknessTag::try_from(weaknesses[0] as u8)
            .unwrap_or(offers::WeaknessTag::Stone);
        let w2 = offers::WeaknessTag::try_from(weaknesses[1] as u8)
            .unwrap_or(offers::WeaknessTag::Frost);

        // Generate new offers (pool-filtered during generation)
        let act = map_pois.act;
        let pool = &ctx.accounts.game_session.active_item_pool;

        let generated = offers::generate_smuggler_hatch_offers(act, w1, w2, seed, pool);

        copy_shop_offers(&mut map_pois.shop_state, &generated.offers, true);

        emit!(ShopRerolled {
            session: map_pois.session,
            cost,
            reroll_count: map_pois.shop_state.reroll_count,
        });

        Ok(())
    }

    /// Exit the shop without purchasing.
    pub fn leave_shop(ctx: Context<LeaveShop>) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        require_keys_eq!(
            ctx.accounts.game_session.player,
            ctx.accounts.player.key(),
            PoiSystemError::Unauthorized
        );

        require!(map_pois.shop_state.active, PoiSystemError::ShopNotActive);

        // Reset shop state
        map_pois.shop_state.active = false;
        map_pois.shop_state.reroll_count = 0;

        emit!(ShopExited {
            session: map_pois.session,
        });

        Ok(())
    }

    // =========================================================================
    // Upgrade Instructions (L10 Rusty Anvil, L11 Rune Kiln)
    // =========================================================================

    /// Upgrade a tool at the Rusty Anvil (L10).
    ///
    /// Tier I -> II costs 10 Gold, II -> III costs 20 Gold.
    /// POI is repeatable. Gold is deducted atomically via CPI.
    pub fn interact_rusty_anvil(
        ctx: Context<InteractRustyAnvil>,
        poi_index: u8,
        item_id: [u8; 8],
        current_tier: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        // L10 is Repeatable, so skip usage check
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, true)?;

        let player_gold = game_state.gold;
        let is_night = game_state.phase.is_night();

        let expected_tier = tier_from_u8(current_tier)?;
        let tool = ctx
            .accounts
            .inventory
            .tool
            .ok_or(PoiSystemError::ItemNotInInventory)?;
        require!(tool.item_id == item_id, PoiSystemError::ItemNotInInventory);
        require!(
            tool.tier == expected_tier,
            PoiSystemError::ItemNotInInventory
        );

        let result =
            interactions::execute_anvil_upgrade(poi, item_id, current_tier, player_gold, is_night)?;

        // CPI to gameplay-state to deduct gold atomically
        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            -(result.cost as i16),
        )?;

        player_inventory::cpi::upgrade_tool_tier_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.player_inventory_program.to_account_info(),
                player_inventory::cpi::accounts::UpgradeToolTierAuthorized {
                    inventory: ctx.accounts.inventory.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            item_id,
            expected_tier,
        )?;

        emit!(ToolUpgraded {
            session: map_pois.session,
            item_id: result.item_id,
            old_tier: current_tier,
            new_tier: result.new_tier,
            cost: result.cost,
        });

        Ok(())
    }

    /// Fuse two identical items at the Rune Kiln (L11).
    ///
    /// Items must have the same ID and tier. Free to use.
    /// POI is repeatable.
    pub fn interact_rune_kiln(
        ctx: Context<InteractRuneKiln>,
        poi_index: u8,
        item1_id: [u8; 8],
        item1_tier: u8,
        item2_id: [u8; 8],
        item2_tier: u8,
    ) -> Result<()> {
        let map_pois = &ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        // L11 is Repeatable, so skip usage check
        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, true)?;

        let is_night = game_state.phase.is_night();

        let tier = tier_from_u8(item1_tier)?;

        let result = interactions::execute_kiln_fusion(
            poi, item1_id, item1_tier, item2_id, item2_tier, is_night,
        )?;

        let (slot_a, slot_b) = find_matching_gear_slots(&ctx.accounts.inventory, item1_id, tier)
            .ok_or(PoiSystemError::ItemNotInInventory)?;

        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        player_inventory::cpi::fuse_items_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.player_inventory_program.to_account_info(),
                player_inventory::cpi::accounts::FuseItemsAuthorized {
                    inventory: ctx.accounts.inventory.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            slot_a,
            slot_b,
        )?;

        emit!(ItemsFused {
            session: map_pois.session,
            item_id: result.item_id,
            result_tier: result.result_tier,
        });

        Ok(())
    }

    // =========================================================================
    // Fast Travel Instructions (L8 Rail Waypoint)
    // =========================================================================

    /// Discover visible Rail Waypoints (L8) based on current player vision radius.
    ///
    /// This is called automatically on spawn and movement.
    pub fn discover_visible_waypoints(
        ctx: Context<DiscoverVisibleWaypoints>,
        visibility_radius: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;
        if visibility_radius == SPAWN_VISION_RADIUS {
            require!(
                game_state.total_moves == 0,
                PoiSystemError::InvalidVisionRadius
            );
        } else if game_state.phase.is_night() {
            require!(
                visibility_radius == NIGHT_VISION_RADIUS,
                PoiSystemError::InvalidVisionRadius
            );
        } else {
            require!(
                visibility_radius == DAY_VISION_RADIUS,
                PoiSystemError::InvalidVisionRadius
            );
        }

        let player_x = game_state.position_x;
        let player_y = game_state.position_y;
        let session = map_pois.session;

        for poi in map_pois.pois.iter_mut() {
            if poi.poi_type != 8 || poi.discovered {
                continue;
            }

            if is_within_visibility_radius(player_x, player_y, poi.x, poi.y, visibility_radius) {
                poi.discovered = true;

                emit!(WaypointDiscovered {
                    session,
                    x: poi.x,
                    y: poi.y,
                });
            }
        }

        Ok(())
    }

    /// Fast travel between Rail Waypoints (L8).
    ///
    /// Player must be at a discovered waypoint and select another discovered waypoint.
    pub fn fast_travel(
        ctx: Context<FastTravel>,
        from_poi_index: u8,
        to_poi_index: u8,
    ) -> Result<()> {
        let map_pois = &ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        // Validate POI indices
        require!(
            (from_poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );
        require!(
            (to_poi_index as usize) < map_pois.pois.len(),
            PoiSystemError::InvalidPoiIndex
        );

        let from_poi = &map_pois.pois[from_poi_index as usize];
        let to_poi = &map_pois.pois[to_poi_index as usize];

        // Validate player is at the from_poi
        require!(
            game_state.position_x == from_poi.x && game_state.position_y == from_poi.y,
            PoiSystemError::PlayerNotOnPoiTile
        );

        let is_night = game_state.phase.is_night();

        // Execute fast travel
        let result = interactions::execute_fast_travel(from_poi, to_poi, is_night)?;

        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        gameplay_state::cpi::set_position_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::SetPositionAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            result.to_x,
            result.to_y,
        )?;

        emit!(FastTravelCompleted {
            session: map_pois.session,
            from_x: result.from_x,
            from_y: result.from_y,
            to_x: result.to_x,
            to_y: result.to_y,
        });

        Ok(())
    }

    // =========================================================================
    // Map Reveal Instructions (L6 Survey Beacon, L7 Seismic Scanner)
    // =========================================================================

    /// Activate a Survey Beacon (L6).
    ///
    /// Reveals all tiles within radius 13 of the beacon.
    /// POI is one-time use.
    pub fn interact_survey_beacon(ctx: Context<InteractSurveyBeacon>, poi_index: u8) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let map_width = game_state.map_width;
        let map_height = game_state.map_height;
        let is_night = game_state.phase.is_night();

        let result = interactions::execute_survey_beacon(poi, map_width, map_height, is_night)?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

        emit!(TilesRevealed {
            session: map_pois.session,
            count: result.tiles.len() as u16,
            center_x: result.center_x,
            center_y: result.center_y,
        });

        Ok(())
    }

    /// Activate a Seismic Scanner (L7).
    ///
    /// Reveals the nearest undiscovered POI of the selected category.
    /// POI is one-time use.
    pub fn interact_seismic_scanner(
        ctx: Context<InteractSeismicScanner>,
        poi_index: u8,
        category: u8,
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let cat = match category {
            0 => state::PoiCategory::Items,
            1 => state::PoiCategory::Upgrades,
            2 => state::PoiCategory::Utility,
            3 => state::PoiCategory::Shop,
            _ => return Err(PoiSystemError::InvalidInteraction.into()),
        };

        let pois_snapshot: Vec<_> = map_pois.pois.clone();
        let is_night = game_state.phase.is_night();

        // Execute seismic scanner
        let result = interactions::execute_seismic_scanner(
            poi,
            &pois_snapshot,
            poi_index as usize,
            cat,
            is_night,
        )?;

        // Mark POI as used (one-time)
        map_pois.pois[poi_index as usize].used = true;

        // Mark revealed POI as discovered
        if let Some((revealed_idx, x, y)) = result.revealed_poi {
            map_pois.pois[revealed_idx].discovered = true;

            emit!(PoiRevealed {
                session: map_pois.session,
                poi_type: map_pois.pois[revealed_idx].poi_type,
                x,
                y,
            });
        }

        Ok(())
    }

    // =========================================================================
    // Scrap Chute Instruction (L14)
    // =========================================================================

    /// Scrap a gear item at the Scrap Chute (L14).
    ///
    /// Destroys one equipped gear item for a flat 4 gold cost and rarity refund.
    /// POI is repeatable and item removal + gold change are atomic.
    pub fn interact_scrap_chute(
        ctx: Context<InteractScrapChute>,
        poi_index: u8,
        item_id: [u8; 8],
    ) -> Result<()> {
        let map_pois = &mut ctx.accounts.map_pois;
        let game_state = &ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;

        require_player_owns_game_state(game_state, &ctx.accounts.player)?;
        let act = map_pois.act;

        let (poi, _) = get_and_validate_poi(map_pois, game_state, poi_index, false)?;

        let player_gold = game_state.gold;
        let is_night = game_state.phase.is_night();

        let slot_index = inventory
            .gear
            .iter()
            .take(inventory.gear_slot_capacity as usize)
            .position(|slot| slot.as_ref().is_some_and(|item| item.item_id == item_id))
            .ok_or(PoiSystemError::ItemNotInInventory)? as u8;

        let result = interactions::execute_scrap_gear(poi, item_id, player_gold, act, is_night)?;

        let seeds = &[POI_AUTHORITY_SEED, &[ctx.bumps.poi_authority]];
        player_inventory::cpi::unequip_gear_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.player_inventory_program.to_account_info(),
                player_inventory::cpi::accounts::UnequipGearAuthorized {
                    inventory: ctx.accounts.inventory.to_account_info(),
                    game_state: ctx.accounts.game_state.to_account_info(),
                    inventory_authority: ctx.accounts.inventory_authority.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                    gameplay_state_program: ctx.accounts.gameplay_state_program.to_account_info(),
                },
                &[&seeds[..]],
            ),
            slot_index,
        )?;

        // CPI to gameplay-state to deduct gold atomically
        let gold_delta =
            i16::try_from(result.refund).unwrap_or(0) - i16::try_from(result.cost).unwrap_or(0);
        gameplay_state::cpi::modify_gold_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.gameplay_state_program.to_account_info(),
                gameplay_state::cpi::accounts::ModifyGoldAuthorized {
                    game_state: ctx.accounts.game_state.to_account_info(),
                    poi_authority: ctx.accounts.poi_authority.to_account_info(),
                },
                &[&seeds[..]],
            ),
            gold_delta,
        )?;

        emit!(GearScrapped {
            session: map_pois.session,
            item_id: result.item_id,
            cost: result.cost,
        });

        Ok(())
    }

    /// Close a corrupted/empty MapPois account (0-byte data).
    /// After an ER reset, force-undelegation can leave accounts with no data.
    /// Only works on accounts owned by this program with 0 bytes of data.
    pub fn close_empty_map_pois(ctx: Context<CloseEmptyMapPois>) -> Result<()> {
        let info = ctx.accounts.map_pois.to_account_info();
        let dest = ctx.accounts.destination.to_account_info();
        **dest.try_borrow_mut_lamports()? += info.lamports();
        **info.try_borrow_mut_lamports()? = 0;
        info.assign(&anchor_lang::system_program::ID);
        info.realloc(0, false)?;
        Ok(())
    }

    // ========================================================================
    // VRF Instructions
    // ========================================================================

    /// Requests VRF randomness for POI offer generation.
    /// Initializes a PoiVrfState account with status=Requested.
    pub fn request_poi_vrf(ctx: Context<RequestPoiVrf>) -> Result<()> {
        let vrf_state = &mut ctx.accounts.vrf_state;
        vrf_state.session = ctx.accounts.session.key();
        vrf_state.randomness = [0u8; 32];
        vrf_state.nonce = 1;
        vrf_state.status = VrfStatus::Requested;
        vrf_state.bump = ctx.bumps.vrf_state;
        // TODO: CPI to oracle via ephemeral-vrf-sdk
        Ok(())
    }

    /// Oracle callback: receives VRF randomness for POI offers.
    pub fn fulfill_poi_vrf(ctx: Context<FulfillPoiVrf>, randomness: [u8; 32]) -> Result<()> {
        let vrf_state = &mut ctx.accounts.vrf_state;
        require!(
            vrf_state.status == VrfStatus::Requested,
            PoiSystemError::VrfNotRequested
        );
        vrf_state.randomness = randomness;
        vrf_state.status = VrfStatus::Fulfilled;
        Ok(())
    }

    /// Closes PoiVrfState account and returns rent to the player.
    /// Called via CPI from session-manager during end_session/abandon_session.
    pub fn close_poi_vrf_state(ctx: Context<ClosePoiVrfState>) -> Result<()> {
        use session_manager::state::GameSession;
        let player_end = GameSession::PLAYER_OFFSET + 32;
        let session_signer_end = GameSession::SESSION_SIGNER_OFFSET + 32;

        let session_data = ctx.accounts.game_session.try_borrow_data()?;
        require!(
            session_data.len() >= session_signer_end,
            PoiSystemError::InvalidSession
        );

        let stored_player = Pubkey::from(
            <[u8; 32]>::try_from(&session_data[GameSession::PLAYER_OFFSET..player_end]).unwrap(),
        );
        let stored_session_signer = Pubkey::from(
            <[u8; 32]>::try_from(&session_data[GameSession::SESSION_SIGNER_OFFSET..session_signer_end])
                .unwrap(),
        );

        require!(
            stored_session_signer == ctx.accounts.session_signer.key(),
            PoiSystemError::Unauthorized
        );
        require!(
            stored_player == ctx.accounts.player.key(),
            PoiSystemError::Unauthorized
        );
        Ok(())
    }
}

// =============================================================================
// Account Contexts
// =============================================================================

#[delegate]
#[derive(Accounts)]
pub struct DelegateMapPois<'info> {
    #[account(mut, del)]
    /// CHECK: PDA is validated in handler.
    pub map_pois: AccountInfo<'info>,
    /// CHECK: Session PDA owned by session-manager; used only for seed derivation.
    pub game_session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
pub struct UndelegateMapPois<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated and deserialized in handler.
    pub map_pois: AccountInfo<'info>,
    /// CHECK: Session account is read for session signer authorization.
    pub game_session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

fn read_map_pois(map_pois: &AccountInfo<'_>) -> Result<MapPois> {
    let data = map_pois.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    MapPois::try_deserialize(&mut slice).map_err(|_| PoiSystemError::InvalidSession.into())
}

#[derive(Accounts)]
pub struct InitializeMapPois<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + MapPois::INIT_SPACE,
        seeds = [MAP_POIS_SEED, session.key().as_ref()],
        bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// The GameSession PDA (must exist)
    /// CHECK: We only verify this account exists as validation of the session
    pub session: AccountInfo<'info>,

    /// Generated map containing POIs to copy
    /// CHECK: Validated by owner check (must be owned by map-generator program)
    /// and PDA derivation (seeds = ["generated_map", session])
    #[account(
        owner = MAP_GENERATOR_PROGRAM_ID @ PoiSystemError::InvalidGeneratedMap
    )]
    pub generated_map: AccountInfo<'info>,

    #[account(
        constraint = game_state.session == session.key() @ PoiSystemError::InvalidSession
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseMapPois<'info> {
    #[account(
        mut,
        close = player,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// The session that owns this MapPois.
    /// CHECK: Validated by constraint (address match) and owner check (session-manager program).
    /// Player field is extracted from account data in the instruction body.
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::Unauthorized,
        owner = SESSION_MANAGER_PROGRAM_ID @ PoiSystemError::InvalidSessionOwner,
    )]
    pub game_session: AccountInfo<'info>,

    /// Session owner — must match GameSession.player (validated in instruction body).
    #[account(mut)]
    pub player: Signer<'info>,
}

/// Context for closing MapPois via session key signer (for session-manager CPI)
#[derive(Accounts)]
pub struct CloseMapPoisViaSessionSigner<'info> {
    #[account(
        mut,
        close = player,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// The session that owns this MapPois.
    /// CHECK: Validated by constraint (address match) and owner check (session-manager program).
    /// Session key signer and player fields are extracted in instruction body.
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::Unauthorized,
        owner = SESSION_MANAGER_PROGRAM_ID @ PoiSystemError::InvalidSessionOwner,
    )]
    pub game_session: AccountInfo<'info>,

    /// Player wallet receives the rent refund (not a signer).
    /// CHECK: Validated against session.player in instruction body.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    /// Session key signer must sign to authorize closure.
    pub session_signer: Signer<'info>,
}

/// Context for closing orphaned MapPois (session PDA already closed).
/// Validates via GameState which stores session_signer and player.
#[derive(Accounts)]
pub struct CloseMapPoisOrphaned<'info> {
    #[account(
        mut,
        close = player,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// GameState for auth — must belong to the same session as map_pois.
    #[account(
        constraint = game_state.session == map_pois.session @ PoiSystemError::InvalidSession,
        has_one = session_signer @ PoiSystemError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player wallet receives the rent refund.
    /// CHECK: Validated via game_state.player.
    #[account(mut, address = game_state.player @ PoiSystemError::Unauthorized)]
    pub player: AccountInfo<'info>,

    /// Session key signer — validated via game_state.session_signer.
    pub session_signer: Signer<'info>,
}

/// Close a corrupted/empty MapPois account (0-byte data after ER reset + force-undelegate).
/// Only works on accounts owned by this program with exactly 0 bytes of data.
#[derive(Accounts)]
pub struct CloseEmptyMapPois<'info> {
    #[account(
        mut,
        constraint = map_pois.data_is_empty() @ PoiSystemError::InvalidPoiType,
        constraint = *map_pois.owner == crate::ID @ PoiSystemError::Unauthorized,
    )]
    /// CHECK: Validated via owner check + empty data constraint.
    pub map_pois: UncheckedAccount<'info>,

    /// Receives the lamports from the closed account.
    #[account(mut)]
    /// CHECK: Any destination is fine since the account is corrupted/empty.
    pub destination: AccountInfo<'info>,

    pub payer: Signer<'info>,
}

/// Context for querying POI definition (view instruction)
#[derive(Accounts)]
pub struct GetPoiDefinition {}

/// Context for interacting with a rest POI (L1 or L5)
#[derive(Accounts)]
pub struct InteractRest<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Box<Account<'info, MapPois>>,

    /// Player's GameState for position/time validation (mut for CPI)
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player's inventory for deriving max_hp and boss fight resolution (mut for gear slot expansion)
    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Box<Account<'info, player_inventory::state::PlayerInventory>>,

    /// Generated map account (provides seed for duel week 1/2 boss selection in skip_to_day).
    #[account(
        seeds = [b"generated_map", game_state.session.as_ref()],
        bump,
        seeds::program = MAP_GENERATOR_PROGRAM_ID,
    )]
    /// CHECK: PDA validated by seeds against map-generator program.
    pub generated_map: AccountInfo<'info>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay authority PDA from gameplay-state for gear slot expansion CPI
    /// CHECK: PDA derived from gameplay-state program
    #[account(
        seeds = [b"gameplay_authority"],
        bump,
        seeds::program = gameplay_state::ID,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI (heal_player and skip_to_day)
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player inventory program for CPI (expand gear slots on boss victory via skip_to_day)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Optional GameplayVrfState for VRF-backed boss selection in skip_to_day CPI.
    /// CHECK: Passed through to gameplay-state CPI; validated there.
    pub gameplay_vrf_state: Option<AccountInfo<'info>>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for interacting with a pick-item POI (L2, L3, L12, L13)
/// Now includes accounts for CPI to player-inventory to equip the picked item.
#[derive(Accounts)]
pub struct InteractPickItem<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation and HP modification via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player's inventory for equipping items
    #[account(
        mut,
        seeds = [b"inventory", map_pois.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Box<Account<'info, player_inventory::state::PlayerInventory>>,

    /// Inventory authority PDA from player-inventory for HP modification CPI
    /// CHECK: PDA derived from player-inventory program
    #[account(
        seeds = [b"inventory_authority"],
        bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory_authority: AccountInfo<'info>,

    /// POI authority PDA for signing CPI calls to player-inventory
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (equip_gear_authorized, equip_tool_authorized)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Gameplay state program for HP modification CPI chain
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Game session for active_item_pool filtering
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::InvalidSession,
    )]
    pub game_session: Account<'info, session_manager::state::GameSession>,

    /// Optional VRF state for PvP offer generation.
    /// Required when RunMode is Duel or Gauntlet; ignored for Campaign.
    /// CHECK: Validated via PDA derivation and manual deserialization in handler.
    pub poi_vrf_state: Option<UncheckedAccount<'info>>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for interacting with a Tool Oil Rack (L4)
#[derive(Accounts)]
pub struct InteractToolOil<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player inventory where tool-oil mutations are applied
    #[account(
        mut,
        seeds = [b"inventory", map_pois.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
        has_one = player @ PoiSystemError::Unauthorized,
    )]
    pub inventory: Box<Account<'info, player_inventory::state::PlayerInventory>>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (apply_tool_oil)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Optional VRF state for PvP offer generation.
    /// CHECK: Validated via PDA derivation and manual deserialization in handler.
    pub poi_vrf_state: Option<UncheckedAccount<'info>>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for entering the Smuggler Hatch shop (L9)
#[derive(Accounts)]
pub struct EnterShop<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Game session for active_item_pool filtering
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::InvalidSession,
    )]
    pub game_session: Account<'info, session_manager::state::GameSession>,

    /// Optional VRF state for PvP offer generation.
    /// CHECK: Validated via PDA derivation and manual deserialization in handler.
    pub poi_vrf_state: Option<UncheckedAccount<'info>>,

    /// Player initiating the interaction
    pub player: Signer<'info>,
}

/// Context for purchasing from the shop
/// Includes accounts for both gold deduction and item equipping via CPI.
#[derive(Accounts)]
pub struct ShopPurchase<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player's inventory for equipping purchased items
    #[account(
        mut,
        seeds = [b"inventory", map_pois.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Box<Account<'info, player_inventory::state::PlayerInventory>>,

    /// Inventory authority PDA from player-inventory for HP modification CPI
    /// CHECK: PDA derived from player-inventory program
    #[account(
        seeds = [b"inventory_authority"],
        bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory_authority: AccountInfo<'info>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (equip_gear_authorized, equip_tool_authorized)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player making the purchase
    pub player: Signer<'info>,
}

/// Context for rerolling shop offers
#[derive(Accounts)]
pub struct ShopReroll<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Game session for active_item_pool filtering
    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::InvalidSession,
    )]
    pub game_session: Account<'info, session_manager::state::GameSession>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player rerolling
    pub player: Signer<'info>,
}

/// Context for leaving the shop
#[derive(Accounts)]
pub struct LeaveShop<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    #[account(
        constraint = game_session.key() == map_pois.session @ PoiSystemError::InvalidSession,
    )]
    pub game_session: Account<'info, session_manager::state::GameSession>,

    /// Player leaving
    pub player: Signer<'info>,
}

/// Context for upgrading at the Rusty Anvil (L10)
#[derive(Accounts)]
pub struct InteractRustyAnvil<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation and gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player inventory where tool tier is upgraded
    #[account(
        mut,
        seeds = [b"inventory", map_pois.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
        has_one = player @ PoiSystemError::Unauthorized,
    )]
    pub inventory: Box<Account<'info, player_inventory::state::PlayerInventory>>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player inventory program for CPI (upgrade_tool_tier)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Player upgrading
    pub player: Signer<'info>,
}

/// Context for fusing at the Rune Kiln (L11)
#[derive(Accounts)]
pub struct InteractRuneKiln<'info> {
    #[account(
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player inventory where gear fusion is applied
    #[account(
        mut,
        seeds = [b"inventory", map_pois.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
        has_one = player @ PoiSystemError::Unauthorized,
    )]
    pub inventory: Box<Account<'info, player_inventory::state::PlayerInventory>>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (fuse_items)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Player fusing
    pub player: Signer<'info>,
}

/// Context for discovering visible Rail Waypoints (L8)
#[derive(Accounts)]
pub struct DiscoverVisibleWaypoints<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player discovering
    pub player: Signer<'info>,
}

/// Context for fast travel between Rail Waypoints (L8)
#[derive(Accounts)]
pub struct FastTravel<'info> {
    #[account(
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI (set_position_authorized)
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player traveling
    pub player: Signer<'info>,
}

/// Context for activating a Survey Beacon (L6)
#[derive(Accounts)]
pub struct InteractSurveyBeacon<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player activating
    pub player: Signer<'info>,
}

/// Context for activating a Seismic Scanner (L7)
#[derive(Accounts)]
pub struct InteractSeismicScanner<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation
    #[account(
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player activating
    pub player: Signer<'info>,
}

/// Context for scrapping gear at the Scrap Chute (L14)
#[derive(Accounts)]
pub struct InteractScrapChute<'info> {
    #[account(
        mut,
        seeds = [MAP_POIS_SEED, map_pois.session.as_ref()],
        bump = map_pois.bump
    )]
    pub map_pois: Account<'info, MapPois>,

    /// Player's GameState for position/time validation and gold deduction via CPI
    #[account(
        mut,
        seeds = [b"game_state", map_pois.session.as_ref()],
        bump = game_state.bump,
        seeds::program = gameplay_state::ID,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player inventory must belong to the same session and player
    #[account(
        mut,
        seeds = [b"inventory", map_pois.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
        has_one = player @ PoiSystemError::Unauthorized,
    )]
    pub inventory: Box<Account<'info, player_inventory::state::PlayerInventory>>,

    /// Inventory authority PDA from player-inventory for CPI calls
    /// CHECK: PDA derived from player-inventory program
    #[account(
        seeds = [b"inventory_authority"],
        bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory_authority: AccountInfo<'info>,

    /// POI authority PDA for signing CPI calls
    /// CHECK: PDA derived from this program, used as signer in CPI
    #[account(
        seeds = [POI_AUTHORITY_SEED],
        bump,
    )]
    pub poi_authority: AccountInfo<'info>,

    /// Gameplay state program for CPI
    pub gameplay_state_program: Program<'info, gameplay_state::program::GameplayState>,

    /// Player inventory program for CPI item removal
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Player scrapping
    pub player: Signer<'info>,
}

// =============================================================================
// VRF Account Contexts
// =============================================================================

#[derive(Accounts)]
pub struct RequestPoiVrf<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Session PDA owned by session-manager.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ PoiSystemError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = PoiVrfState::SPACE,
        seeds = [PoiVrfState::SEED_PREFIX, session.key().as_ref()],
        bump
    )]
    pub vrf_state: Account<'info, PoiVrfState>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FulfillPoiVrf<'info> {
    /// Oracle identity signer.
    pub oracle: Signer<'info>,

    #[account(
        mut,
        seeds = [PoiVrfState::SEED_PREFIX, vrf_state.session.as_ref()],
        bump = vrf_state.bump,
    )]
    pub vrf_state: Account<'info, PoiVrfState>,
}

#[derive(Accounts)]
pub struct ClosePoiVrfState<'info> {
    #[account(
        mut,
        seeds = [PoiVrfState::SEED_PREFIX, vrf_state.session.as_ref()],
        bump = vrf_state.bump,
        close = player,
    )]
    pub vrf_state: Account<'info, PoiVrfState>,

    /// CHECK: Session account for signer validation. Read as raw bytes.
    #[account(
        constraint = game_session.key() == vrf_state.session @ PoiSystemError::Unauthorized,
        owner = SESSION_MANAGER_PROGRAM_ID @ PoiSystemError::InvalidSessionOwner,
    )]
    pub game_session: AccountInfo<'info>,

    /// CHECK: Validated against session.player in instruction body.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    pub session_signer: Signer<'info>,
}

// =============================================================================
// Events
// =============================================================================

#[event]
pub struct PoisInitialized {
    pub session: Pubkey,
    pub count: u8,
    pub act: u8,
}

#[event]
pub struct PoisClosed {
    pub session: Pubkey,
}

#[event]
pub struct PoiInteracted {
    pub session: Pubkey,
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
    pub interaction: String,
}

#[event]
pub struct ItemPicked {
    pub session: Pubkey,
    pub poi_type: u8,
    pub item_id: [u8; 8],
    pub tier: u8,
}

#[event]
pub struct ToolOilApplied {
    pub session: Pubkey,
    pub modification: u8,
}

#[event]
pub struct TilesRevealed {
    pub session: Pubkey,
    pub count: u16,
    pub center_x: u8,
    pub center_y: u8,
}

#[event]
pub struct PoiRevealed {
    pub session: Pubkey,
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
}

#[event]
pub struct WaypointDiscovered {
    pub session: Pubkey,
    pub x: u8,
    pub y: u8,
}

#[event]
pub struct FastTravelCompleted {
    pub session: Pubkey,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
}

#[event]
pub struct ShopEntered {
    pub session: Pubkey,
    pub poi_index: u8,
}

#[event]
pub struct ItemPurchased {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub price: u16,
}

#[event]
pub struct ShopRerolled {
    pub session: Pubkey,
    pub cost: u16,
    pub reroll_count: u8,
}

#[event]
pub struct ShopExited {
    pub session: Pubkey,
}

#[event]
pub struct ToolUpgraded {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub old_tier: u8,
    pub new_tier: u8,
    pub cost: u16,
}

#[event]
pub struct ItemsFused {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub result_tier: u8,
}

#[event]
pub struct GearScrapped {
    pub session: Pubkey,
    pub item_id: [u8; 8],
    pub cost: u16,
}

#[event]
pub struct PoiDefinitionQueried {
    pub poi_type: u8,
    pub name: String,
    pub rarity: u8,
    pub use_type: u8,
    pub active_condition: u8,
    pub interaction_type: u8,
    pub category: u8,
}

#[event]
pub struct RestCompleted {
    pub session: Pubkey,
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
    /// Heal amount (u16 to support max_hp > 255)
    pub heal_amount: u16,
    pub full_heal: bool,
}

/// Emitted when a cache offer is generated for pick-item POIs.
#[event]
pub struct CacheOfferGenerated {
    pub session: Pubkey,
    pub poi_index: u8,
    pub poi_type: u8,
    pub item0: [u8; 8],
    pub item1: [u8; 8],
    pub item2: [u8; 8],
}

/// Emitted when oil offers are generated for Tool Oil Rack (L4).
#[event]
pub struct OilOfferGenerated {
    pub session: Pubkey,
    pub poi_index: u8,
    /// The 3 oil flags offered (from OIL_FLAG_ATK=1, SPD=2, DIG=4, ARM=8)
    pub oils: [u8; 3],
}

/// The discriminator for initialize_map_pois instruction.
/// This is exported so other programs can validate their manual CPI discriminators.
/// Computed as sha256("global:initialize_map_pois")[..8].
///
/// IMPORTANT: If you rename the `initialize_map_pois` instruction, you must:
/// 1. Update this constant
/// 2. Update session-manager's INITIALIZE_MAP_POIS_DISCRIMINATOR constant
pub const INITIALIZE_MAP_POIS_DISCRIMINATOR: [u8; 8] =
    [0xa8, 0xec, 0xff, 0x37, 0xee, 0xd2, 0x19, 0xfb];

#[cfg(test)]
mod discriminator_tests {
    use super::*;

    /// Validates that INITIALIZE_MAP_POIS_DISCRIMINATOR matches sha256("global:initialize_map_pois")[..8].
    /// Computes the hash at test time so a rename is caught immediately.
    #[test]
    fn test_initialize_map_pois_discriminator() {
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(b"global:initialize_map_pois");
        let expected: [u8; 8] = hash[..8].try_into().unwrap();
        assert_eq!(
            INITIALIZE_MAP_POIS_DISCRIMINATOR, expected,
            "INITIALIZE_MAP_POIS_DISCRIMINATOR doesn't match sha256(\"global:initialize_map_pois\")[..8]"
        );
    }
}
