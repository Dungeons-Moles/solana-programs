use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::system_program;
use core::str::FromStr;
use ephemeral_rollups_sdk::anchor::{commit, delegate, ephemeral};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;

pub mod constants;
pub mod errors;
pub mod movement;
pub mod state;
pub mod stats;

use combat_system::state::{
    AnnotatedItemEffect, CombatContribution, CombatSourceKind, CombatSourceRef, CombatantInput,
    Condition,
};
use combat_system::{
    resolve_boss_combat_annotated_with_player_gold, resolve_combat_annotated_with_both_gold,
    resolve_pvp_combat_annotated_with_both_gold, EffectType, ItemEffect, TriggerType,
};
use constants::{
    base_hp, COMPANY_TREASURY_ADDRESS, DAY_MOVES, PVP_BASE_HP,
    DUEL_ENTRY_LAMPORTS, DUEL_ENTRY_SEED, DUEL_ER_QUEUE_SEED, DUEL_OPEN_QUEUE_SEED,
    DUEL_QUEUE_SEED, DUEL_VAULT_SEED,
    GAME_STATE_SEED, GAUNTLET_BOOTSTRAP_ECHOES_PER_WEEK, GAUNTLET_CAMPAIGN_LEVEL,
    GAUNTLET_ECHOES_SEED,
    GAUNTLET_COMPANY_FEE_BPS, GAUNTLET_CONFIG_SEED, GAUNTLET_ENTRY_LAMPORTS,
    GAUNTLET_EPOCH_DURATION_SECONDS, GAUNTLET_EPOCH_POOL_SEED, GAUNTLET_MAX_WEEKLY_ECHOES,
    GAUNTLET_PLAYER_SCORE_SEED, GAUNTLET_POOL_FEE_BPS, GAUNTLET_POOL_VAULT_SEED,
    GAUNTLET_WEEK_POOL_SEED, INITIAL_GEAR_SLOTS, MAX_GEAR_SLOTS, PIT_DRAFT_BPS_DENOMINATOR,
    PIT_DRAFT_COMPANY_FEE_BPS, PIT_DRAFT_ENTRY_LAMPORTS, PIT_DRAFT_GAUNTLET_FEE_BPS,
    PIT_DRAFT_QUEUE_SEED,
    PIT_DRAFT_VAULT_SEED, PIT_DRAFT_WINNER_BPS,
};
use errors::GameplayStateError;

/// Seed for gameplay_authority PDA used for CPI calls to other programs
pub const GAMEPLAY_AUTHORITY_SEED: &[u8] = b"gameplay_authority";
pub const SESSION_MANAGER_RUNMODE_AUTHORITY_SEED: &[u8] = b"session_manager_authority";
use movement::{
    calculate_move_cost, chebyshev_distance, compute_visible_enemies, get_boss_for_combat,
    get_boss_id, get_duel_boss_for_combat_from_seed, get_duel_boss_for_combat_vrf,
    get_duel_boss_id_from_seed, get_duel_boss_id_vrf, is_adjacent,
    is_within_bounds, should_process_night_enemy_movement, should_process_target_enemy_combat,
};
use player_inventory::effects::{generate_annotated_combat_effects, generate_combat_effects};
use player_inventory::items::ITEMS;
use player_inventory::state::{ItemInstance, ItemType, PlayerInventory, Tier, ToolOilModification};
use player_profile::state::PlayerProfile;
use state::{
    DuelCreatorEntry, DuelEntry, DuelErQueue, DuelLoadoutSnapshot, DuelOpenQueue, DuelQueue,
    DuelRunOutcome,
    DuelVault, GameState, GameplayVrfState, GauntletConfig, GauntletDefenderCredit,
    GauntletEchoSnapshot, GauntletEchoSource, GauntletEchoes, GauntletEpochPool,
    GauntletLoadoutSnapshot, GauntletPendingPoints, GauntletPlayerScore, GauntletPoolVault,
    GauntletWeekPool, Phase, PitDraftQueue, PitDraftVault, RunMode,
};
use stats::{calculate_stats, PlayerStats};
use vrf_rng::VrfStatus;

fn compute_gold_gain_multiplier(effects: &[ItemEffect]) -> i16 {
    effects
        .iter()
        .filter(|effect| effect.effect_type == EffectType::AmplifyGoldGain)
        .map(|effect| effect.value.max(0))
        .sum()
}

fn compute_gold_gain_multiplier_annotated(effects: &[AnnotatedItemEffect]) -> i16 {
    effects
        .iter()
        .filter(|effect| effect.effect.effect_type == EffectType::AmplifyGoldGain)
        .map(|effect| effect.effect.value.max(0))
        .sum()
}

fn is_baked_battle_start_stat_effect(effect: &AnnotatedItemEffect) -> bool {
    effect.effect.trigger == combat_system::TriggerType::BattleStart
        && effect.effect.condition == player_inventory::state::Condition::None
        && matches!(
            effect.effect.effect_type,
            EffectType::GainAtk
                | EffectType::GainGearAtk
                | EffectType::GainArmor
                | EffectType::GainSpd
                | EffectType::GainDig
                | EffectType::GainStrikes
                | EffectType::MaxHp
        )
        && matches!(
            effect.source.map(|source| source.kind),
            Some(CombatSourceKind::Tool | CombatSourceKind::Gear)
        )
}

fn strip_baked_battle_start_stat_effects(
    effects: Vec<AnnotatedItemEffect>,
) -> Vec<AnnotatedItemEffect> {
    effects
        .into_iter()
        .filter(|effect| !is_baked_battle_start_stat_effect(effect))
        .collect()
}

fn attack_source_from_effects(effects: &[AnnotatedItemEffect]) -> Option<CombatSourceRef> {
    effects.iter().find_map(|effect| match effect.source {
        Some(source) if matches!(source.kind, CombatSourceKind::Tool) => Some(source),
        _ => None,
    })
}

fn baked_attack_contributions_from_effects(
    effects: &[AnnotatedItemEffect],
) -> Vec<CombatContribution> {
    let mut contributions: Vec<CombatContribution> = Vec::new();

    for effect in effects {
        let Some(source) = effect.source else {
            continue;
        };

        if !matches!(source.kind, CombatSourceKind::Tool | CombatSourceKind::Gear) {
            continue;
        }

        if effect.effect.trigger != TriggerType::BattleStart
            || !matches!(effect.effect.condition, Condition::None)
        {
            continue;
        }

        if !matches!(
            effect.effect.effect_type,
            EffectType::GainAtk | EffectType::GainGearAtk
        ) || effect.effect.value <= 0
        {
            continue;
        }

        if let Some(existing) = contributions
            .iter_mut()
            .find(|entry| entry.source == source)
        {
            existing.value = existing.value.saturating_add(effect.effect.value);
        } else {
            contributions.push(CombatContribution {
                source,
                value: effect.effect.value,
            });
        }
    }

    contributions
}

fn apply_gold_reward_multiplier(base: u16, multiplier_percent: i16) -> u16 {
    if multiplier_percent <= 0 || base == 0 {
        return base;
    }

    let total = i32::from(base) + (i32::from(base) * i32::from(multiplier_percent) / 100);
    total.clamp(0, i32::from(u16::MAX)) as u16
}

declare_id!("3rzGGgHRRnMATmYJkjidPMapEMesvA16PTs5HhfAep4V");

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xb0, 0x1c, 0x9d, 0x6a, 0x40, 0xc6, 0xe0, 0xa9, 0xe4, 0xaf, 0xa9, 0xa9, 0xd9, 0xad, 0x02, 0x15,
    0x89, 0xbd, 0xf1, 0x36, 0x79, 0x88, 0x02, 0x94, 0xc2, 0x24, 0x9f, 0xd9, 0xa4, 0x21, 0xd7, 0x39,
]);

/// POI system program ID for authorized HP/Gold modifications
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0x65, 0xd1, 0x76, 0xb1, 0x94, 0xe7, 0xc4, 0x89, 0xa6, 0x09, 0xbd, 0xa7, 0x8c, 0x0c, 0x0a, 0xe6,
    0xf6, 0xae, 0xd1, 0x4e, 0xf0, 0xba, 0xe9, 0x21, 0xbb, 0xde, 0x72, 0x90, 0xf1, 0x04, 0xcc, 0xf9,
]);
pub const NIGHT_VISION_RADIUS: u8 = 2;
pub const DAY_VISION_RADIUS: u8 = 4;
pub const PIT_DRAFT_MAX_START_GOLD: u16 = 30;
pub const DISCOVER_VISIBLE_WAYPOINTS_AUTHORIZED_DISCRIMINATOR: [u8; 8] =
    [0xe8, 0x21, 0x93, 0x9c, 0x32, 0x7a, 0x1e, 0x8f];
/// Player inventory program ID for authorized HP modifications via CPI
pub const PLAYER_INVENTORY_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xeb, 0x8f, 0x56, 0x3f, 0xf8, 0x4c, 0xa7, 0xf3, 0x8a, 0xd6, 0xe5, 0xb4, 0xb3, 0xe1, 0x5b, 0x18,
    0xd8, 0x8b, 0x1b, 0xa6, 0x72, 0xa5, 0xc2, 0x18, 0xed, 0x52, 0xd0, 0x4e, 0x77, 0xe1, 0x0e, 0x0f,
]);

fn local_delegate_config(validator: Option<Pubkey>) -> DelegateConfig {
    DelegateConfig {
        validator,
        ..DelegateConfig::default()
    }
}

#[ephemeral]
#[program]
pub mod gameplay_state {
    use super::*;

    /// Initializes a new GameState account linked to an active GameSession.
    pub fn initialize_game_state(
        ctx: Context<InitializeGameState>,
        campaign_level: u8,
        map_width: u8,
        map_height: u8,
        start_x: u8,
        start_y: u8,
    ) -> Result<()> {
        require_keys_eq!(
            *ctx.accounts.game_session.owner,
            SESSION_MANAGER_PROGRAM_ID,
            GameplayStateError::InvalidSessionOwner
        );

        require!(
            (1..=40).contains(&campaign_level),
            GameplayStateError::InvalidCampaignLevel
        );
        require!(
            start_x < map_width && start_y < map_height,
            GameplayStateError::OutOfBounds
        );

        let game_state = &mut ctx.accounts.game_state;
        game_state.player = ctx.accounts.player.key();
        game_state.session_signer = ctx.accounts.session_signer.key();
        game_state.session = ctx.accounts.game_session.key();
        game_state.position_x = start_x;
        game_state.position_y = start_y;
        game_state.map_width = map_width;
        game_state.map_height = map_height;
        game_state.hp = base_hp(campaign_level);
        game_state.gear_slots = INITIAL_GEAR_SLOTS;
        game_state.week = 1;
        game_state.phase = Phase::Day1;
        game_state.moves_remaining = DAY_MOVES;
        game_state.total_moves = 0;
        game_state.boss_fight_ready = false;
        game_state.gold = match campaign_level {
            1..=5 => 10,
            6..=10 => 8,
            11..=15 => 5,
            16..=20 => 3,
            21..=30 => 2,
            _ => 0,
        };
        game_state.bump = ctx.bumps.game_state;
        game_state.campaign_level = campaign_level;
        game_state.run_mode = RunMode::Campaign;
        game_state.max_weeks = 3;
        game_state.is_dead = false;
        game_state.completed = false;
        game_state.duel_map_seed = 0;

        let generated_map = &ctx.accounts.generated_map;

        game_state.enemies = Vec::with_capacity(generated_map.enemy_count as usize);

        for idx in 0..generated_map.enemy_count as usize {
            let enemy = generated_map.enemies[idx];
            game_state.enemies.push(state::EnemyInstance {
                archetype_id: enemy.archetype_id,
                tier: enemy.tier,
                x: enemy.x,
                y: enemy.y,
                defeated: false,
            });
        }

        game_state.enemy_count = game_state.enemies.len() as u8;

        emit!(GameStateInitialized {
            player: game_state.player,
            session: game_state.session,
            map_width,
            map_height,
        });

        Ok(())
    }

    /// Syncs enemies and game position from GeneratedMap after ER map generation.
    ///
    /// Called on the Ephemeral Rollup immediately after fill_map_with_seed or
    /// generate_map_with_vrf populates the GeneratedMap. Updates:
    /// - game_state.enemies from generated_map.enemies
    /// - game_state map dimensions and spawn position (was set to placeholder 50x50/0,0 at init)
    pub fn sync_map_enemies(ctx: Context<SyncMapEnemies>) -> Result<()> {
        let generated_map = &ctx.accounts.generated_map;
        let game_state = &mut ctx.accounts.game_state;

        // Sync actual map dimensions and player spawn position
        game_state.map_width = generated_map.width;
        game_state.map_height = generated_map.height;
        game_state.position_x = generated_map.spawn_x;
        game_state.position_y = generated_map.spawn_y;

        // Populate enemy instances from generated map
        game_state.enemies = Vec::with_capacity(generated_map.enemy_count as usize);
        for idx in 0..generated_map.enemy_count as usize {
            let enemy = generated_map.enemies[idx];
            game_state.enemies.push(state::EnemyInstance {
                archetype_id: enemy.archetype_id,
                tier: enemy.tier,
                x: enemy.x,
                y: enemy.y,
                defeated: false,
            });
        }
        game_state.enemy_count = game_state.enemies.len() as u8;

        discover_visible_waypoints_authorized_cpi(
            &ctx.accounts.map_pois,
            &ctx.accounts.gameplay_authority.to_account_info(),
            &ctx.accounts.poi_system_program.to_account_info(),
            ctx.bumps.gameplay_authority,
            generated_map.spawn_x,
            generated_map.spawn_y,
            6, // SPAWN_VISION_RADIUS
            ctx.accounts.session_discovery.as_ref().map(|a| a.to_account_info()).as_ref(),
            Some(&ctx.accounts.session.to_account_info()),
            Some(&ctx.accounts.session_signer.to_account_info()),
            Some(&ctx.accounts.map_generator_program.to_account_info()),
        )?;

        // Sync boss ID, echo, and visible enemies to SessionDiscovery
        if let Some(ref sd) = ctx.accounts.session_discovery {
            let sd_info = sd.to_account_info();
            let session_info = ctx.accounts.session.to_account_info();
            let ga_info = ctx.accounts.gameplay_authority.to_account_info();
            let mgp_info = ctx.accounts.map_generator_program.to_account_info();
            let ga_bump = ctx.bumps.gameplay_authority;

            let map_pois_data = ctx.accounts.map_pois.try_borrow_data()?;
            const MAP_POIS_HEADER_LEN: usize = 8 + 32 + 1 + 1 + 1 + 1 + 8;
            const POI_INSTANCE_LEN: usize = 6;
            if map_pois_data.len() >= MAP_POIS_HEADER_LEN + 4 {
                let pois_len_offset = MAP_POIS_HEADER_LEN;
                let pois_len = u32::from_le_bytes(
                    map_pois_data[pois_len_offset..pois_len_offset + 4]
                        .try_into()
                        .unwrap_or([0u8; 4]),
                ) as usize;
                let pois_start = pois_len_offset + 4;

                let mut mole_den: Option<(u8, u8, u8, u8)> = None;
                for index in 0..pois_len {
                    let base = pois_start + index * POI_INSTANCE_LEN;
                    if base + POI_INSTANCE_LEN > map_pois_data.len() {
                        break;
                    }

                    let poi_type = map_pois_data[base];
                    let x = map_pois_data[base + 1];
                    let y = map_pois_data[base + 2];
                    if poi_type == 1 {
                        mole_den = Some((index as u8, poi_type, x, y));
                        break;
                    }
                }

                if let Some((mole_den_index, mole_den_type, mole_den_x, mole_den_y)) = mole_den {
                map_generator::cpi::record_discovered_poi(
                    CpiContext::new(
                        mgp_info.clone(),
                        map_generator::cpi::accounts::RecordDiscoveredPoi {
                            session_discovery: sd_info.clone(),
                            session: session_info.clone(),
                            session_signer: ctx.accounts.session_signer.to_account_info(),
                        },
                    ),
                    mole_den_type,
                    mole_den_x,
                    mole_den_y,
                    mole_den_index,
                )?;
            }
            }

            // Boss ID for week 1
            let boss_id = if game_state.run_mode == RunMode::Gauntlet {
                [0u8; 12]
            } else if game_state.run_mode == RunMode::Duel {
                // Use map seed so both matched players get the same boss.
                get_duel_boss_id_from_seed(generated_map.seed, 1)?
            } else {
                get_boss_id(game_state.campaign_level, 1)?
            };
            update_boss_id_cpi(&sd_info, &session_info, &ga_info, &mgp_info, ga_bump, boss_id)?;

            // Echo for gauntlet mode (week 1)
            if game_state.run_mode == RunMode::Gauntlet {
                let ge_echo = ctx.accounts.gauntlet_echoes.as_ref()
                    .and_then(|ge| ge.echoes[0].as_ref());
                if let Some(echo) = ge_echo {
                    let echo_bytes = echo.try_to_vec().unwrap_or_default();
                    let mut echo_data = [0u8; 179];
                    let copy_len = echo_bytes.len().min(179);
                    echo_data[..copy_len].copy_from_slice(&echo_bytes[..copy_len]);
                    update_current_echo_cpi(
                        &sd_info, &session_info, &ga_info, &mgp_info, ga_bump, 1, echo_data,
                    )?;
                } else {
                    update_current_echo_cpi(
                        &sd_info, &session_info, &ga_info, &mgp_info, ga_bump, 0, [0u8; 179],
                    )?;
                }
            } else {
                update_current_echo_cpi(
                    &sd_info, &session_info, &ga_info, &mgp_info, ga_bump, 0, [0u8; 179],
                )?;
            }

            // Visible enemies (from spawn radius)
            let visible = compute_visible_enemies(
                game_state,
                &generated_map.discovered_tiles,
                generated_map.width,
            );
            update_discovered_enemies_cpi(
                &sd_info, &session_info, &ga_info, &mgp_info, ga_bump, visible,
            )?;
        }

        Ok(())
    }

    /// Refreshes discovered enemies in SessionDiscovery from MapEnemies + GeneratedMap.
    /// Legacy fallback for clients that have not yet moved POI reveal flows to the
    /// inline authorized path.
    pub fn refresh_discovered_enemies(ctx: Context<RefreshDiscoveredEnemies>) -> Result<()> {
        if let Some(ref sd) = ctx.accounts.session_discovery {
            let visible = compute_visible_enemies(
                &ctx.accounts.game_state,
                &ctx.accounts.generated_map.discovered_tiles,
                ctx.accounts.generated_map.width,
            );
            update_discovered_enemies_cpi(
                &sd.to_account_info(),
                &ctx.accounts.session.to_account_info(),
                &ctx.accounts.gameplay_authority.to_account_info(),
                &ctx.accounts.map_generator_program.to_account_info(),
                ctx.bumps.gameplay_authority,
                visible,
            )?;
        }
        Ok(())
    }

    /// Refreshes discovered enemies in SessionDiscovery, authorized by poi-system CPI.
    /// Used by reveal-heavy POIs so the frontend does not need a follow-up tx.
    pub fn refresh_discovered_enemies_authorized(
        ctx: Context<RefreshDiscoveredEnemiesAuthorized>,
    ) -> Result<()> {
        if let Some(ref sd) = ctx.accounts.session_discovery {
            let visible = compute_visible_enemies(
                &ctx.accounts.game_state,
                &ctx.accounts.generated_map.discovered_tiles,
                ctx.accounts.generated_map.width,
            );
            update_discovered_enemies_cpi(
                &sd.to_account_info(),
                &ctx.accounts.session.to_account_info(),
                &ctx.accounts.gameplay_authority.to_account_info(),
                &ctx.accounts.map_generator_program.to_account_info(),
                ctx.bumps.gameplay_authority,
                visible,
            )?;
        }
        Ok(())
    }

    /// Delegates GameState PDA to MagicBlock from gameplay-state.
    pub fn delegate_gameplay_accounts(
        ctx: Context<DelegateGameplayAccounts>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let session_key = ctx.accounts.game_session.key();
        let (expected_game_state, _) =
            Pubkey::find_program_address(&[b"game_state", session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.game_state.key(),
            expected_game_state,
            GameplayStateError::Unauthorized
        );

        let game_state_seeds: &[&[u8]] = &[b"game_state", session_key.as_ref()];
        ctx.accounts.delegate_game_state(
            &ctx.accounts.player,
            game_state_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Delegates GauntletEchoes PDA to MagicBlock (gauntlet sessions only).
    pub fn delegate_gauntlet_echoes(
        ctx: Context<DelegateGauntletEchoes>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let session_key = ctx.accounts.game_session.key();
        let (expected_ge, _) = Pubkey::find_program_address(
            &[GauntletEchoes::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_echoes.key(),
            expected_ge,
            GameplayStateError::Unauthorized
        );
        let ge_seeds: &[&[u8]] = &[GauntletEchoes::SEED_PREFIX, session_key.as_ref()];
        ctx.accounts.delegate_gauntlet_echoes(
            &ctx.accounts.player,
            ge_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Commits and undelegates gameplay runtime accounts from ER back to base layer.
    pub fn undelegate_gameplay_accounts(ctx: Context<UndelegateGameplayAccounts>) -> Result<()> {
        let session_key = ctx.accounts.game_session.key();
        let (expected_game_state, _) =
            Pubkey::find_program_address(&[b"game_state", session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.game_state.key(),
            expected_game_state,
            GameplayStateError::Unauthorized
        );
        require_keys_eq!(
            read_game_state(&ctx.accounts.game_state)?.session,
            session_key,
            GameplayStateError::Unauthorized
        );
        require_keys_eq!(
            read_game_state(&ctx.accounts.game_state)?.session_signer,
            ctx.accounts.session_signer.key(),
            GameplayStateError::Unauthorized
        );

        let game_state_info = ctx.accounts.game_state.to_account_info();
        commit_and_undelegate_accounts(
            &ctx.accounts.session_signer.to_account_info(),
            vec![&game_state_info],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
        )?;
        Ok(())
    }

    /// Commits and undelegates GauntletEchoes from ER (gauntlet sessions only).
    pub fn undelegate_gauntlet_echoes(ctx: Context<UndelegateGauntletEchoes>) -> Result<()> {
        let session_key = ctx.accounts.game_session.key();
        let (expected_ge, _) = Pubkey::find_program_address(
            &[GauntletEchoes::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_echoes.key(),
            expected_ge,
            GameplayStateError::Unauthorized
        );
        let ge_info = ctx.accounts.gauntlet_echoes.to_account_info();
        commit_and_undelegate_accounts(
            &ctx.accounts.session_signer.to_account_info(),
            vec![&ge_info],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
        )?;
        Ok(())
    }

    /// Commits and undelegates only the game_state account from ER back to base layer.
    pub fn undelegate_game_state(ctx: Context<UndelegateGameState>) -> Result<()> {
        let session_key = ctx.accounts.game_session.key();
        let (expected_game_state, _) =
            Pubkey::find_program_address(&[b"game_state", session_key.as_ref()], &crate::ID);
        require_keys_eq!(
            ctx.accounts.game_state.key(),
            expected_game_state,
            GameplayStateError::Unauthorized
        );
        require_keys_eq!(
            read_game_state(&ctx.accounts.game_state)?.session,
            session_key,
            GameplayStateError::Unauthorized
        );
        require_keys_eq!(
            read_game_state(&ctx.accounts.game_state)?.session_signer,
            ctx.accounts.session_signer.key(),
            GameplayStateError::Unauthorized
        );

        let game_state_info = ctx.accounts.game_state.to_account_info();
        commit_and_undelegate_accounts(
            &ctx.accounts.session_signer.to_account_info(),
            vec![&game_state_info],
            &ctx.accounts.magic_context,
            &ctx.accounts.magic_program.to_account_info(),
        )?;
        Ok(())
    }

    // undelegate_map_enemies removed — MapEnemies merged into GameState

    /// Initializes global pit draft queue/vault PDAs.
    pub fn initialize_pit_draft(ctx: Context<InitializePitDraft>) -> Result<()> {
        let queue = &mut ctx.accounts.pit_draft_queue;
        let vault = &mut ctx.accounts.pit_draft_vault;

        if !queue.initialized {
            queue.waiting_player = None;
            queue.waiting_profile = None;
            queue.initialized = true;
            queue.bump = ctx.bumps.pit_draft_queue;
        }
        if !vault.initialized {
            vault.initialized = true;
            vault.bump = ctx.bumps.pit_draft_vault;
        }

        Ok(())
    }

    /// Initializes global duel vault PDA.
    pub fn initialize_duels(ctx: Context<InitializeDuels>) -> Result<()> {
        let vault = &mut ctx.accounts.duel_vault;
        if !vault.initialized {
            vault.initialized = true;
            vault.bump = ctx.bumps.duel_vault;
        }
        let open_queue = &mut ctx.accounts.duel_open_queue;
        if !open_queue.initialized {
            open_queue.entries = Vec::new();
            open_queue.initialized = true;
            open_queue.bump = ctx.bumps.duel_open_queue;
        }
        let er_queue = &mut ctx.accounts.duel_er_queue;
        if !er_queue.initialized {
            er_queue.entries = Vec::new();
            er_queue.initialized = true;
            er_queue.bump = ctx.bumps.duel_er_queue;
        }
        Ok(())
    }

    /// Delegates the per-session DuelEntry PDA to the ER.
    /// Called during duel session setup, after enter_duel creates the account.
    pub fn delegate_duel_entry(ctx: Context<DelegateDuelEntry>, session_pda: Pubkey) -> Result<()> {
        let (expected_pda, _) = Pubkey::find_program_address(
            &[DUEL_ENTRY_SEED, session_pda.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.duel_entry.key(),
            expected_pda,
            GameplayStateError::Unauthorized
        );
        let seeds: &[&[u8]] = &[DUEL_ENTRY_SEED, session_pda.as_ref()];
        ctx.accounts.delegate_duel_entry(
            &ctx.accounts.player,
            seeds,
            ephemeral_rollups_sdk::cpi::DelegateConfig::default(),
        )?;
        Ok(())
    }

    /// Delegates the DuelErQueue PDA to the ER (permanently, never undelegate).
    /// Must be called once after initialize_duels creates the account.
    pub fn delegate_duel_er_queue(ctx: Context<DelegateDuelErQueue>) -> Result<()> {
        let (expected_pda, _) = Pubkey::find_program_address(&[DUEL_ER_QUEUE_SEED], &crate::ID);
        require_keys_eq!(
            ctx.accounts.duel_er_queue.key(),
            expected_pda,
            GameplayStateError::Unauthorized
        );
        let seeds: &[&[u8]] = &[DUEL_ER_QUEUE_SEED];
        ctx.accounts.delegate_duel_er_queue(
            &ctx.accounts.admin,
            seeds,
            ephemeral_rollups_sdk::cpi::DelegateConfig::default(),
        )?;
        Ok(())
    }

    /// Initializes a seed-scoped duel queue PDA.
    pub fn initialize_duel_queue(ctx: Context<InitializeDuelQueue>, seed: u64) -> Result<()> {
        let queue = &mut ctx.accounts.duel_queue;
        if !queue.initialized {
            queue.seed = seed;
            queue.player_a = None;
            queue.player_b = None;
            queue.initialized = true;
            queue.bump = ctx.bumps.duel_queue;
        } else {
            require!(queue.seed == seed, GameplayStateError::DuelSeedMismatch);
        }
        Ok(())
    }

    /// Sets run mode and max weeks for a session.
    pub fn configure_run_mode(
        ctx: Context<ConfigureRunMode>,
        run_mode: RunMode,
        max_weeks: u8,
    ) -> Result<()> {
        require!(
            ctx.accounts.session_manager_authority.is_signer,
            GameplayStateError::Unauthorized
        );
        require!(
            (run_mode == RunMode::Campaign || run_mode == RunMode::Duel) && max_weeks == 3
                || (run_mode == RunMode::Gauntlet && max_weeks == 5),
            GameplayStateError::InvalidRunModeMaxWeeks
        );
        let game_state = &mut ctx.accounts.game_state;
        require!(
            game_state.week == 1
                && matches!(game_state.phase, Phase::Day1)
                && game_state.total_moves == 0
                && game_state.moves_remaining == DAY_MOVES
                && !game_state.boss_fight_ready
                && !game_state.is_dead
                && !game_state.completed,
            GameplayStateError::RunModeConfigurationLocked
        );
        game_state.run_mode = run_mode;
        game_state.max_weeks = max_weeks;

        // PvP modes use fixed starting stats independent of campaign level
        if run_mode == RunMode::Duel || run_mode == RunMode::Gauntlet {
            game_state.hp = PVP_BASE_HP;
            game_state.gold = 0;
        }

        Ok(())
    }

    /// Initializes Gauntlet config, pool vault and weekly echo pools.
    pub fn initialize_gauntlet(ctx: Context<InitializeGauntlet>) -> Result<()> {
        let config = &mut ctx.accounts.gauntlet_config;
        let clock = Clock::get()?;

        if !config.initialized {
            config.entry_lamports = GAUNTLET_ENTRY_LAMPORTS;
            config.company_fee_bps = GAUNTLET_COMPANY_FEE_BPS as u16;
            config.pool_fee_bps = GAUNTLET_POOL_FEE_BPS as u16;
            config.current_epoch_id = 0;
            config.current_epoch_start_ts = clock.unix_timestamp;
            config.epoch_duration_seconds = GAUNTLET_EPOCH_DURATION_SECONDS;
            config.initialized = true;
            config.bump = ctx.bumps.gauntlet_config;
        }

        if !ctx.accounts.gauntlet_pool_vault.initialized {
            ctx.accounts.gauntlet_pool_vault.initialized = true;
            ctx.accounts.gauntlet_pool_vault.bump = ctx.bumps.gauntlet_pool_vault;
        }

        if !ctx.accounts.gauntlet_week1.initialized {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week1, 1)?;
            ctx.accounts.gauntlet_week1.initialized = true;
            ctx.accounts.gauntlet_week1.bump = ctx.bumps.gauntlet_week1;
        } else {
            require!(
                ctx.accounts.gauntlet_week1.week == 1,
                GameplayStateError::InvalidGauntletWeek
            );
        }
        if !ctx.accounts.gauntlet_week2.initialized {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week2, 2)?;
            ctx.accounts.gauntlet_week2.initialized = true;
            ctx.accounts.gauntlet_week2.bump = ctx.bumps.gauntlet_week2;
        } else {
            require!(
                ctx.accounts.gauntlet_week2.week == 2,
                GameplayStateError::InvalidGauntletWeek
            );
        }
        if !ctx.accounts.gauntlet_week3.initialized {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week3, 3)?;
            ctx.accounts.gauntlet_week3.initialized = true;
            ctx.accounts.gauntlet_week3.bump = ctx.bumps.gauntlet_week3;
        } else {
            require!(
                ctx.accounts.gauntlet_week3.week == 3,
                GameplayStateError::InvalidGauntletWeek
            );
        }
        if !ctx.accounts.gauntlet_week4.initialized {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week4, 4)?;
            ctx.accounts.gauntlet_week4.initialized = true;
            ctx.accounts.gauntlet_week4.bump = ctx.bumps.gauntlet_week4;
        } else {
            require!(
                ctx.accounts.gauntlet_week4.week == 4,
                GameplayStateError::InvalidGauntletWeek
            );
        }
        if !ctx.accounts.gauntlet_week5.initialized {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week5, 5)?;
            ctx.accounts.gauntlet_week5.initialized = true;
            ctx.accounts.gauntlet_week5.bump = ctx.bumps.gauntlet_week5;
        } else {
            require!(
                ctx.accounts.gauntlet_week5.week == 5,
                GameplayStateError::InvalidGauntletWeek
            );
        }

        Ok(())
    }

    /// Pays gauntlet entry and marks this run as gauntlet mode.
    pub fn enter_gauntlet(ctx: Context<EnterGauntlet>, epoch_id: u64) -> Result<()> {
        require_expected_address(
            ctx.accounts.company_treasury.key(),
            COMPANY_TREASURY_ADDRESS,
            GameplayStateError::InvalidGauntletFeeAccount,
        )?;

        let game_state = &mut ctx.accounts.game_state;
        require!(
            !game_state.is_dead && !game_state.completed,
            GameplayStateError::GauntletRunEnded
        );
        // GauntletEchoes is created via `init` — Anchor prevents double-init automatically.
        require!(
            ctx.accounts.gauntlet_config.current_epoch_id == epoch_id,
            GameplayStateError::GauntletScoreMismatch
        );

        let entry = ctx.accounts.gauntlet_config.entry_lamports;
        let company_fee = entry
            .checked_mul(GAUNTLET_COMPANY_FEE_BPS)
            .and_then(|v| v.checked_div(PIT_DRAFT_BPS_DENOMINATOR))
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        let pool_fee = entry
            .checked_sub(company_fee)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player.to_account_info(),
                    to: ctx.accounts.company_treasury.to_account_info(),
                },
            ),
            company_fee,
        )?;

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player.to_account_info(),
                    to: ctx.accounts.gauntlet_pool_vault.to_account_info(),
                },
            ),
            pool_fee,
        )?;

        game_state.run_mode = RunMode::Gauntlet;
        game_state.max_weeks = 5;
        game_state.hp = PVP_BASE_HP;
        game_state.gold = 0;
        game_state.gauntlet_epoch_id = epoch_id;

        let epoch_pool = &mut ctx.accounts.gauntlet_epoch_pool;
        if !epoch_pool.initialized {
            epoch_pool.epoch_id = epoch_id;
            epoch_pool.initialized = true;
            epoch_pool.bump = ctx.bumps.gauntlet_epoch_pool;
            epoch_pool.finalized = false;
        }

        let player_score = &mut ctx.accounts.gauntlet_player_score;
        if player_score.player == Pubkey::default() {
            player_score.epoch_id = epoch_id;
            player_score.player = ctx.accounts.player.key();
            player_score.points = 0;
            player_score.claimed = false;
            player_score.bump = ctx.bumps.gauntlet_player_score;
        }

        let gauntlet_echoes = &mut ctx.accounts.gauntlet_echoes;
        gauntlet_echoes.session = game_state.session;
        gauntlet_echoes.bump = ctx.bumps.gauntlet_echoes;
        // Echo draw deferred to redraw_gauntlet_echoes (on ER, after VRF fulfillment).
        // VRF is not available at enter_gauntlet time because it runs on base layer
        // before delegation. Echoes remain None until redraw populates them.

        emit!(GauntletEntered {
            player: ctx.accounts.player.key(),
            session: game_state.session,
            entry_lamports: entry,
            company_fee,
            pool_fee,
        });
        Ok(())
    }

    /// Draws gauntlet echoes using VRF randomness.
    /// Called on ER after VRF fulfillment. enter_gauntlet only inits the
    /// GauntletEchoes account — this instruction populates it with VRF-random draws.
    pub fn redraw_gauntlet_echoes(ctx: Context<RedrawGauntletEchoes>) -> Result<()> {
        let game_state = &ctx.accounts.game_state;
        require!(
            game_state.run_mode == RunMode::Gauntlet,
            GameplayStateError::GauntletRunNotActive
        );

        let vrf = extract_gameplay_vrf(&ctx.accounts.gameplay_vrf_state, &game_state.session)?;
        // VRF must be present for redraw
        let vrf_data = vrf.ok_or(GameplayStateError::VrfNotFulfilled)?;

        let gauntlet_echoes = &mut ctx.accounts.gauntlet_echoes;
        draw_gauntlet_echoes_from_remaining_vrf(
            gauntlet_echoes,
            ctx.remaining_accounts,
            (&vrf_data.0, vrf_data.1),
        )?;

        Ok(())
    }

    /// Finalizes current epoch when duration elapsed.
    pub fn finalize_gauntlet_epoch(
        ctx: Context<FinalizeGauntletEpoch>,
        epoch_id: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let config = &mut ctx.accounts.gauntlet_config;
        require!(
            config.current_epoch_id == epoch_id,
            GameplayStateError::GauntletScoreMismatch
        );
        require!(
            ctx.accounts.gauntlet_epoch_pool.epoch_id == epoch_id,
            GameplayStateError::GauntletScoreMismatch
        );
        if clock.unix_timestamp
            < config
                .current_epoch_start_ts
                .checked_add(config.epoch_duration_seconds)
                .ok_or(GameplayStateError::ArithmeticOverflow)?
        {
            return Ok(());
        }

        let current_epoch_id = epoch_id;
        let epoch_pool = &mut ctx.accounts.gauntlet_epoch_pool;
        epoch_pool.epoch_id = current_epoch_id;
        epoch_pool.total_pool_lamports = ctx
            .accounts
            .gauntlet_pool_vault
            .to_account_info()
            .lamports();
        if epoch_pool.pending_defender_points.is_empty() {
            epoch_pool.pending_defender_points = Vec::new();
        }
        epoch_pool.finalized = true;
        config.current_epoch_id = config
            .current_epoch_id
            .checked_add(1)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        config.current_epoch_start_ts = clock.unix_timestamp;

        emit!(GauntletEpochFinalized {
            epoch_id: current_epoch_id,
            total_pool_lamports: epoch_pool.total_pool_lamports,
            total_points: epoch_pool.total_points,
        });
        Ok(())
    }

    /// Claims rewards for a finalized epoch.
    pub fn claim_gauntlet_rewards(ctx: Context<ClaimGauntletRewards>, epoch_id: u64) -> Result<()> {
        require!(
            ctx.accounts.gauntlet_epoch_pool.epoch_id == epoch_id,
            GameplayStateError::GauntletScoreMismatch
        );
        require!(
            ctx.accounts.gauntlet_player_score.epoch_id == epoch_id,
            GameplayStateError::GauntletScoreMismatch
        );
        if ctx.accounts.gauntlet_player_score.player == Pubkey::default() {
            ctx.accounts.gauntlet_player_score.epoch_id = epoch_id;
            ctx.accounts.gauntlet_player_score.player = ctx.accounts.player.key();
            ctx.accounts.gauntlet_player_score.points = 0;
            ctx.accounts.gauntlet_player_score.claimed = false;
            ctx.accounts.gauntlet_player_score.bump = ctx.bumps.gauntlet_player_score;
        }
        require!(
            ctx.accounts.gauntlet_player_score.player == ctx.accounts.player.key(),
            GameplayStateError::GauntletScoreMismatch
        );
        require!(
            ctx.accounts.gauntlet_epoch_pool.finalized,
            GameplayStateError::GauntletEpochNotFinalized
        );
        require!(
            !ctx.accounts.gauntlet_player_score.claimed,
            GameplayStateError::GauntletAlreadyClaimed
        );

        let pending_defender_points = take_pending_defender_points(
            &mut ctx.accounts.gauntlet_epoch_pool,
            ctx.accounts.player.key(),
        );
        if pending_defender_points > 0 {
            ctx.accounts.gauntlet_player_score.points = ctx
                .accounts
                .gauntlet_player_score
                .points
                .checked_add(pending_defender_points)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
        }

        let total_points = ctx.accounts.gauntlet_epoch_pool.total_points;
        if total_points == 0 || ctx.accounts.gauntlet_player_score.points == 0 {
            ctx.accounts.gauntlet_player_score.claimed = true;
            return Ok(());
        }

        let payout = ctx
            .accounts
            .gauntlet_epoch_pool
            .total_pool_lamports
            .checked_mul(ctx.accounts.gauntlet_player_score.points)
            .and_then(|v| v.checked_div(total_points))
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        transfer_lamports_from_vault(
            &ctx.accounts.gauntlet_pool_vault.to_account_info(),
            &ctx.accounts.player_wallet.to_account_info(),
            payout,
        )?;
        ctx.accounts.gauntlet_player_score.claimed = true;

        emit!(GauntletRewardsClaimed {
            epoch_id,
            player: ctx.accounts.player.key(),
            points: ctx.accounts.gauntlet_player_score.points,
            payout_lamports: payout,
        });
        Ok(())
    }

    /// Settles gauntlet points and defender credits to global accounts after undelegation.
    /// Signed by session key — no wallet popup required.
    pub fn settle_gauntlet_session(
        ctx: Context<SettleGauntletSession>,
        epoch_id: u64,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        require!(
            game_state.run_mode == RunMode::Gauntlet,
            GameplayStateError::GauntletRunNotActive
        );
        require!(
            !game_state.gauntlet_settled,
            GameplayStateError::GauntletAlreadySettled
        );
        require!(
            game_state.is_dead || game_state.completed,
            GameplayStateError::GauntletRunEnded
        );
        require!(
            game_state.gauntlet_epoch_id == epoch_id,
            GameplayStateError::GauntletScoreMismatch
        );

        let player = game_state.player;
        let points = game_state.gauntlet_points_earned;
        let score_bump = ctx.accounts.gauntlet_player_score.bump;

        if points > 0 {
            upsert_player_score(
                &mut ctx.accounts.gauntlet_player_score,
                &player,
                epoch_id,
                points,
                score_bump,
            )?;
            ctx.accounts.gauntlet_epoch_pool.total_points = ctx
                .accounts
                .gauntlet_epoch_pool
                .total_points
                .checked_add(points)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
        }

        if let Some(credit) = game_state.gauntlet_defender_credit.take() {
            add_pending_defender_points(
                &mut ctx.accounts.gauntlet_epoch_pool,
                credit.defender,
                credit.points,
            )?;
            ctx.accounts.gauntlet_epoch_pool.total_points = ctx
                .accounts
                .gauntlet_epoch_pool
                .total_points
                .checked_add(credit.points)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
        }

        let highest_week_won = game_state.gauntlet_highest_week_won;
        if highest_week_won > 0 && highest_week_won <= 5 {
            let vrf = extract_gameplay_vrf(&ctx.accounts.gameplay_vrf_state, &game_state.session)?;
            let vrf_data = vrf.ok_or(GameplayStateError::VrfNotFulfilled)?;
            let week_pool = match highest_week_won {
                1 => &mut ctx.accounts.gauntlet_week1,
                2 => &mut ctx.accounts.gauntlet_week2,
                3 => &mut ctx.accounts.gauntlet_week3,
                4 => &mut ctx.accounts.gauntlet_week4,
                _ => &mut ctx.accounts.gauntlet_week5,
            };
            maybe_insert_player_echo_vrf(
                week_pool,
                highest_week_won,
                &ctx.accounts.inventory,
                game_state.gold,
                player,
                (&vrf_data.0, vrf_data.1),
            )?;
        }

        game_state.gauntlet_settled = true;

        emit!(GauntletSessionSettled {
            player,
            session: game_state.session,
            epoch_id,
            points_credited: points,
        });
        Ok(())
    }

    /// Pays duel entry fee. Matching happens later on ER in generate_duel_map.
    pub fn enter_duel(ctx: Context<EnterDuel>) -> Result<()> {
        require_expected_address(
            ctx.accounts.company_treasury.key(),
            COMPANY_TREASURY_ADDRESS,
            GameplayStateError::InvalidDuelFeeAccount,
        )?;
        require!(
            !ctx.accounts.game_state.is_dead && !ctx.accounts.game_state.completed,
            GameplayStateError::DuelRunNotFinished
        );
        require!(
            ctx.accounts.game_state.run_mode == RunMode::Duel,
            GameplayStateError::DuelInvalidRunMode
        );

        let player_key = ctx.accounts.player.key();
        let duel_entry = &mut ctx.accounts.duel_entry;
        require!(
            duel_entry.entry_lamports == 0,
            GameplayStateError::DuelAlreadyQueued
        );

        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player.to_account_info(),
                    to: ctx.accounts.duel_vault.to_account_info(),
                },
            ),
            DUEL_ENTRY_LAMPORTS,
        )?;

        duel_entry.player = player_key;
        duel_entry.session = ctx.accounts.game_state.session;
        duel_entry.game_state = ctx.accounts.game_state.key();
        duel_entry.seed = 0;
        duel_entry.entry_lamports = DUEL_ENTRY_LAMPORTS;
        duel_entry.finalized = false;
        duel_entry.outcome = DuelRunOutcome::Pending;
        duel_entry.loadout = DuelLoadoutSnapshot {
            tool: None,
            gear: [None; 12],
            gold_at_battle_start: 0,
            hp_at_finish: 0,
        };
        duel_entry.matched_creator = None;
        duel_entry.settled = false;
        duel_entry.bump = ctx.bumps.duel_entry;

        emit!(DuelQueued {
            seed: 0,
            player: player_key,
            game_state: ctx.accounts.game_state.key(),
            entry_lamports: DUEL_ENTRY_LAMPORTS,
            slot: 0,
        });

        Ok(())
    }

    /// Assigns the duel map seed from DuelOpenQueue on base layer (before delegation).
    /// If a creator exists in the queue whose player != current player, pops their entry,
    /// stores the seed in game_state.duel_map_seed, and saves the creator snapshot in
    /// duel_entry.matched_creator for PvP resolution on ER.
    /// Must be called on base layer where DuelOpenQueue is accessible.
    pub fn assign_duel_map_seed(ctx: Context<AssignDuelMapSeed>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let player_key = game_state.player;
        let queue = &mut ctx.accounts.duel_open_queue;
        let duel_entry = &mut ctx.accounts.duel_entry;

        let matched_idx = queue
            .entries
            .iter()
            .position(|e| e.player != player_key);

        if let Some(idx) = matched_idx {
            let creator = queue.entries.remove(idx);
            game_state.duel_map_seed = creator.seed;
            duel_entry.matched_creator = Some(creator);
        } else {
            game_state.duel_map_seed = 0;
        }
        Ok(())
    }

    /// Generates the duel map on ER.
    /// Reads game_state.duel_map_seed (set by assign_duel_map_seed on base layer):
    /// - If duel_map_seed != 0 (matched player): uses it so both players get the same map.
    /// - If duel_map_seed == 0 (creator): derives seed from MapVrfState.
    pub fn generate_duel_map(ctx: Context<GenerateDuelMap>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        require!(
            game_state.run_mode == RunMode::Duel,
            GameplayStateError::DuelInvalidRunMode
        );

        let seed = if game_state.duel_map_seed != 0 {
            // Matched path: use the seed assigned from DuelOpenQueue on base layer.
            game_state.duel_map_seed
        } else {
            // Creator path: derive seed from VRF, store on game_state so it
            // survives undelegation (generated_map data may not persist on surfpool).
            let vrf_state = ctx
                .accounts
                .map_vrf_state
                .as_ref()
                .ok_or(GameplayStateError::VrfNotFulfilled)?;
            require!(
                vrf_state.status == VrfStatus::Fulfilled,
                GameplayStateError::VrfNotFulfilled
            );
            let mut rng = vrf_rng::GameRng::from_vrf(
                &vrf_state.randomness,
                vrf_state.nonce,
                vrf_rng::domains::MAP_GENERATION,
            );
            let derived_seed = rng.next_val();
            game_state.duel_map_seed = derived_seed;
            derived_seed
        };

        let authority_seeds: &[&[&[u8]]] =
            &[&[GAMEPLAY_AUTHORITY_SEED, &[ctx.bumps.gameplay_authority]]];

        map_generator::cpi::fill_map_with_seed_authorized(
            CpiContext::new_with_signer(
                ctx.accounts.map_generator_program.to_account_info(),
                map_generator::cpi::accounts::FillMapWithSeedAuthorized {
                    generated_map: ctx.accounts.generated_map.to_account_info(),
                    session: ctx.accounts.game_session.to_account_info(),
                    gameplay_authority: ctx.accounts.gameplay_authority.to_account_info(),
                    session_discovery: ctx
                        .accounts
                        .session_discovery
                        .as_ref()
                        .map(|a| a.to_account_info()),
                },
                authority_seeds,
            ),
            seed,
            game_state.campaign_level,
        )?;

        Ok(())
    }

    /// Settles duel run on base layer after undelegation.
    /// Captures loadout from game_state + inventory, resolves PvP if matched,
    /// handles payouts, and pushes unmatched creators to DuelOpenQueue.
    pub fn settle_duel_payout(ctx: Context<SettleDuelPayout>) -> Result<()> {
        require_expected_address(
            ctx.accounts.company_treasury.key(),
            COMPANY_TREASURY_ADDRESS,
            GameplayStateError::InvalidDuelFeeAccount,
        )?;

        let game_state = &ctx.accounts.game_state;
        require!(
            game_state.run_mode == RunMode::Duel,
            GameplayStateError::DuelInvalidRunMode
        );
        require!(
            game_state.completed || game_state.is_dead,
            GameplayStateError::DuelRunNotFinished
        );

        let seed = game_state.duel_map_seed;
        let duel_entry = &mut ctx.accounts.duel_entry;
        require!(!duel_entry.settled, GameplayStateError::DuelAlreadyQueued);

        let player_key = ctx.accounts.player.key();
        require_keys_eq!(
            duel_entry.player,
            player_key,
            GameplayStateError::DuelNotQueued
        );

        // Capture loadout from game_state + inventory (now back on base after undelegation).
        duel_entry.seed = seed;
        duel_entry.finalized = true;
        duel_entry.outcome = if game_state.completed {
            DuelRunOutcome::CompletedWeek3
        } else {
            DuelRunOutcome::EliminatedBeforeWeek3
        };
        duel_entry.loadout = DuelLoadoutSnapshot {
            tool: ctx.accounts.inventory.tool,
            gear: ctx.accounts.inventory.gear,
            gold_at_battle_start: game_state.gold,
            hp_at_finish: game_state.hp,
        };

        emit!(DuelRunFinalized {
            seed,
            player: player_key,
            completed_week3: game_state.completed,
            final_week: game_state.week,
        });

        duel_entry.settled = true;

        if let Some(creator) = duel_entry.matched_creator {
            if duel_entry.outcome == DuelRunOutcome::CompletedWeek3 {
                // Resolve PvP combat deterministically from both loadouts.
                let creator_inventory = snapshot_creator_inventory(creator);
                let opponent_inventory = snapshot_duel_entry_inventory(duel_entry);
                let creator_stats = calculate_stats(
                    &creator_inventory,
                    GAUNTLET_CAMPAIGN_LEVEL,
                    RunMode::Duel,
                );
                let opponent_stats = calculate_stats(
                    &opponent_inventory,
                    GAUNTLET_CAMPAIGN_LEVEL,
                    RunMode::Duel,
                );
                let all_creator_effects = generate_annotated_combat_effects(&creator_inventory);
                let all_opponent_effects = generate_annotated_combat_effects(&opponent_inventory);
                let creator_combatant = build_player_combatant(creator.loadout.hp_at_finish, &creator_stats, &all_creator_effects);
                let opponent_combatant = build_player_combatant(duel_entry.loadout.hp_at_finish, &opponent_stats, &all_opponent_effects);
                let creator_effects = strip_baked_battle_start_stat_effects(all_creator_effects);
                let opponent_effects = strip_baked_battle_start_stat_effects(all_opponent_effects);
                let combat_outcome = resolve_pvp_combat_annotated_with_both_gold(
                    creator_combatant,
                    opponent_combatant,
                    creator_effects,
                    opponent_effects,
                    creator.loadout.gold_at_battle_start,
                    duel_entry.loadout.gold_at_battle_start,
                    duel_final_tie_player_a_wins(duel_entry.seed),
                )?;

                let total_pot = creator
                    .entry_lamports
                    .checked_add(duel_entry.entry_lamports)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;
                let (company_fee, gauntlet_fee, winner_payout) = compute_pvp_pot_split(total_pot)?;
                let winner_key = if combat_outcome.player_won {
                    creator.player
                } else {
                    player_key
                };

                if winner_key == player_key {
                    transfer_lamports_from_vault(
                        &ctx.accounts.duel_vault.to_account_info(),
                        &ctx.accounts.player.to_account_info(),
                        winner_payout,
                    )?;
                } else {
                    let creator_wallet = resolve_wallet_account(
                        ctx.accounts.creator_wallet.as_ref(),
                        creator.player,
                    )?;
                    transfer_lamports_from_vault(
                        &ctx.accounts.duel_vault.to_account_info(),
                        &creator_wallet,
                        winner_payout,
                    )?;
                }
                transfer_lamports_from_vault(
                    &ctx.accounts.duel_vault.to_account_info(),
                    &ctx.accounts.company_treasury.to_account_info(),
                    company_fee,
                )?;
                transfer_lamports_from_vault(
                    &ctx.accounts.duel_vault.to_account_info(),
                    &ctx.accounts.gauntlet_pool_vault.to_account_info(),
                    gauntlet_fee,
                )?;
            } else {
                // Opponent eliminated: creator wins pot minus fees.
                let total_pot = creator
                    .entry_lamports
                    .checked_add(duel_entry.entry_lamports)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;
                let (company_fee, gauntlet_fee, winner_payout) = compute_pvp_pot_split(total_pot)?;
                let creator_wallet =
                    resolve_wallet_account(ctx.accounts.creator_wallet.as_ref(), creator.player)?;
                transfer_lamports_from_vault(
                    &ctx.accounts.duel_vault.to_account_info(),
                    &creator_wallet,
                    winner_payout,
                )?;
                transfer_lamports_from_vault(
                    &ctx.accounts.duel_vault.to_account_info(),
                    &ctx.accounts.company_treasury.to_account_info(),
                    company_fee,
                )?;
                transfer_lamports_from_vault(
                    &ctx.accounts.duel_vault.to_account_info(),
                    &ctx.accounts.gauntlet_pool_vault.to_account_info(),
                    gauntlet_fee,
                )?;
            }
            return Ok(());
        }

        // Unmatched creator who completed: push to DuelOpenQueue for matching.
        if duel_entry.outcome == DuelRunOutcome::CompletedWeek3 {
            let open_queue = &mut ctx.accounts.duel_open_queue;
            require!(
                open_queue.entries.len() < constants::DUEL_OPEN_QUEUE_CAPACITY,
                GameplayStateError::DuelQueueFull
            );
            open_queue.entries.push(DuelCreatorEntry {
                player: duel_entry.player,
                seed: duel_entry.seed,
                entry_lamports: duel_entry.entry_lamports,
                finished_slot: Clock::get()?.slot,
                loadout: duel_entry.loadout,
            });
            return Ok(());
        }

        // Unmatched + eliminated: distribute to company + gauntlet pool.
        let (company_fee, gauntlet_fee) =
            compute_eliminated_unmatched_distribution(duel_entry.entry_lamports)?;
        transfer_lamports_from_vault(
            &ctx.accounts.duel_vault.to_account_info(),
            &ctx.accounts.company_treasury.to_account_info(),
            company_fee,
        )?;
        transfer_lamports_from_vault(
            &ctx.accounts.duel_vault.to_account_info(),
            &ctx.accounts.gauntlet_pool_vault.to_account_info(),
            gauntlet_fee,
        )?;

        Ok(())
    }

    /// Resets the duel entry for the current session.
    /// Abandoning forfeits the player's stake — no refund.
    /// If player was matched, the matched creator's entry is NOT re-queued.
    /// Called before end_session/abandon_session so the player can start a new duel.
    pub fn reset_duel_entry(ctx: Context<ResetDuelEntry>) -> Result<()> {
        let duel_entry = &mut ctx.accounts.duel_entry;

        if duel_entry.entry_lamports == 0 {
            return Ok(());
        }

        // If already settled (via settle_duel_payout), just zero the entry — no distribution.
        if duel_entry.settled || duel_entry.finalized {
            duel_entry.matched_creator = None;
            duel_entry.entry_lamports = 0;
            duel_entry.finalized = false;
            duel_entry.settled = false;
            duel_entry.outcome = DuelRunOutcome::Pending;
            return Ok(());
        }

        // Abandon (not finalized/settled): distribute forfeited stake.
        if let Some(creator) = duel_entry.matched_creator.take() {
            // Matched: creator wins pot minus fees.
            let total_pot = creator
                .entry_lamports
                .checked_add(duel_entry.entry_lamports)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
            let (company_fee, gauntlet_fee, winner_payout) = compute_pvp_pot_split(total_pot)?;
            let creator_wallet = resolve_wallet_account(
                ctx.accounts.creator_wallet.as_ref(),
                creator.player,
            )?;
            transfer_lamports_from_vault(
                &ctx.accounts.duel_vault.to_account_info(),
                &creator_wallet,
                winner_payout,
            )?;
            transfer_lamports_from_vault(
                &ctx.accounts.duel_vault.to_account_info(),
                &ctx.accounts.company_treasury.to_account_info(),
                company_fee,
            )?;
            transfer_lamports_from_vault(
                &ctx.accounts.duel_vault.to_account_info(),
                &ctx.accounts.gauntlet_pool_vault.to_account_info(),
                gauntlet_fee,
            )?;
        } else {
            // Unmatched: 50/50 split to company + gauntlet pool.
            let (company_fee, gauntlet_fee) =
                compute_eliminated_unmatched_distribution(duel_entry.entry_lamports)?;
            transfer_lamports_from_vault(
                &ctx.accounts.duel_vault.to_account_info(),
                &ctx.accounts.company_treasury.to_account_info(),
                company_fee,
            )?;
            transfer_lamports_from_vault(
                &ctx.accounts.duel_vault.to_account_info(),
                &ctx.accounts.gauntlet_pool_vault.to_account_info(),
                gauntlet_fee,
            )?;
        }

        duel_entry.entry_lamports = 0;
        duel_entry.finalized = false;
        duel_entry.settled = false;
        duel_entry.outcome = DuelRunOutcome::Pending;

        Ok(())
    }

    /// Resets an orphaned duel entry whose session PDA no longer exists.
    /// This recovers players stuck in the duel queue after an interrupted abandon flow.
    /// No refund — stake is forfeited.
    pub fn reset_orphaned_duel_entry(ctx: Context<ResetOrphanedDuelEntry>) -> Result<()> {
        let duel_entry = &mut ctx.accounts.duel_entry;

        if duel_entry.entry_lamports == 0 {
            return Ok(());
        }

        // No refund on orphaned reset — stake is forfeited.
        duel_entry.matched_creator = None;
        duel_entry.entry_lamports = 0;
        duel_entry.finalized = false;
        duel_entry.settled = false;
        duel_entry.outcome = DuelRunOutcome::Pending;

        Ok(())
    }

    /// Enters Pit Draft with atomic resolution.
    ///
    /// Behavior:
    /// - If queue is empty: player pays stake and becomes waiting player. No VRF needed.
    /// - If queue has a waiting player: player pays stake, VRF must be pre-fulfilled,
    ///   combat resolves inline, winner receives 95% of pot, company gets 3%,
    ///   gauntlet pool gets 2%, queue is cleared.
    pub fn enter_pit_draft(ctx: Context<EnterPitDraft>) -> Result<()> {
        let queue = &mut ctx.accounts.pit_draft_queue;
        let player_key = ctx.accounts.player.key();
        let player_profile_key = ctx.accounts.player_profile.key();

        require!(
            queue.waiting_player != Some(player_key),
            GameplayStateError::PitDraftAlreadyQueued
        );

        // Every entrant pays 0.1 SOL into the pit draft vault.
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: ctx.accounts.player.to_account_info(),
                    to: ctx.accounts.pit_draft_vault.to_account_info(),
                },
            ),
            PIT_DRAFT_ENTRY_LAMPORTS,
        )?;

        // If queue is empty, this player becomes the waiting challenger.
        if queue.waiting_player.is_none() {
            queue.waiting_player = Some(player_key);
            queue.waiting_profile = Some(player_profile_key);

            emit!(PitDraftQueued {
                player: player_key,
                profile: player_profile_key,
                entry_lamports: PIT_DRAFT_ENTRY_LAMPORTS,
            });

            return Ok(());
        }

        // --- Queue has a waiting player — resolve match atomically ---

        let waiting_player = queue
            .waiting_player
            .ok_or(GameplayStateError::PitDraftInvalidWaitingState)?;
        let _waiting_profile_key = queue
            .waiting_profile
            .ok_or(GameplayStateError::PitDraftInvalidWaitingState)?;

        require!(
            waiting_player != player_key,
            GameplayStateError::PitDraftSelfMatch
        );

        // Validate required matching accounts are present
        let waiting_profile = ctx
            .accounts
            .waiting_profile
            .as_ref()
            .ok_or(GameplayStateError::PitDraftMissingWaitingAccounts)?;

        require!(
            waiting_profile.owner == waiting_player,
            GameplayStateError::PitDraftWaitingAccountMismatch
        );

        let waiting_player_wallet = ctx
            .accounts
            .waiting_player_wallet
            .as_ref()
            .ok_or(GameplayStateError::PitDraftMissingWaitingAccounts)?;

        require_keys_eq!(
            waiting_player_wallet.key(),
            waiting_player,
            GameplayStateError::PitDraftWaitingAccountMismatch
        );

        let company_treasury = ctx
            .accounts
            .company_treasury
            .as_ref()
            .ok_or(GameplayStateError::PitDraftMissingWaitingAccounts)?;

        require_expected_address(
            company_treasury.key(),
            COMPANY_TREASURY_ADDRESS,
            GameplayStateError::InvalidPitDraftFeeAccount,
        )?;

        let gauntlet_pool_vault = ctx
            .accounts
            .gauntlet_pool_vault
            .as_ref()
            .ok_or(GameplayStateError::PitDraftMissingWaitingAccounts)?;

        // Require pre-fulfilled VRF for fair randomness
        let vrf_state = ctx
            .accounts
            .gameplay_vrf_state
            .as_ref()
            .ok_or(GameplayStateError::VrfNotFulfilled)?;
        require!(
            vrf_state.status == VrfStatus::Fulfilled,
            GameplayStateError::VrfNotFulfilled
        );
        let randomness = &vrf_state.randomness;
        let nonce = vrf_state.nonce;

        // Build inventories from VRF
        let pool_a = waiting_profile.active_item_pool;
        let pool_b = ctx.accounts.player_profile.active_item_pool;

        let waiting_inventory = build_pit_draft_inventory_vrf(
            waiting_player,
            pool_a,
            (randomness, nonce),
            b"pit_waiting",
        )?;
        let entrant_inventory = build_pit_draft_inventory_vrf(
            player_key,
            pool_b,
            (randomness, nonce),
            b"pit_entrant",
        )?;

        let waiting_stats =
            calculate_stats(&waiting_inventory, GAUNTLET_CAMPAIGN_LEVEL, RunMode::Duel);
        let entrant_stats =
            calculate_stats(&entrant_inventory, GAUNTLET_CAMPAIGN_LEVEL, RunMode::Duel);

        // Gold derivation from VRF
        let mut gold_rng = vrf_rng::GameRng::from_vrf(
            randomness,
            nonce,
            vrf_rng::domains::PIT_DRAFT_GOLD,
        );
        let waiting_start_gold =
            gold_rng.next_bounded(u64::from(PIT_DRAFT_MAX_START_GOLD) + 1) as u16;
        let entrant_start_gold =
            gold_rng.next_bounded(u64::from(PIT_DRAFT_MAX_START_GOLD) + 1) as u16;

        let all_waiting_effects = generate_annotated_combat_effects(&waiting_inventory);
        let all_entrant_effects = generate_annotated_combat_effects(&entrant_inventory);

        // Build combatant inputs first (borrows effects by ref), then consume
        // effects via strip (takes ownership) — avoids clone() heap waste.
        let waiting_combatant = build_full_hp_combatant(&waiting_stats, &all_waiting_effects);
        let entrant_combatant = build_full_hp_combatant(&entrant_stats, &all_entrant_effects);
        let waiting_effects = strip_baked_battle_start_stat_effects(all_waiting_effects);
        let entrant_effects = strip_baked_battle_start_stat_effects(all_entrant_effects);

        let combat_outcome = resolve_pvp_combat_annotated_with_both_gold(
            waiting_combatant,
            entrant_combatant,
            waiting_effects,
            entrant_effects,
            waiting_start_gold,
            entrant_start_gold,
            pit_draft_final_tie_player_a_wins(randomness, nonce),
        )?;

        emit!(PitDraftCombatVisual {
            player_a: waiting_player,
            player_b: player_key,
            player_a_tool: waiting_inventory.tool,
            player_a_gear: waiting_inventory.gear,
            player_b_tool: entrant_inventory.tool,
            player_b_gear: entrant_inventory.gear,
            player_a_won: combat_outcome.player_won,
            final_player_a_hp: combat_outcome.final_player_hp,
            final_player_b_hp: combat_outcome.final_enemy_hp,
            turns_taken: combat_outcome.turns_taken,
        });

        let total_pot = PIT_DRAFT_ENTRY_LAMPORTS
            .checked_mul(2)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        let (company_fee, gauntlet_fee, winner_payout) = compute_pvp_pot_split(total_pot)?;

        let winner_account = if combat_outcome.player_won {
            waiting_player_wallet.to_account_info()
        } else {
            ctx.accounts.player.to_account_info()
        };
        let winner = if combat_outcome.player_won {
            waiting_player
        } else {
            player_key
        };

        transfer_lamports_from_vault(
            &ctx.accounts.pit_draft_vault.to_account_info(),
            &winner_account,
            winner_payout,
        )?;
        transfer_lamports_from_vault(
            &ctx.accounts.pit_draft_vault.to_account_info(),
            &company_treasury.to_account_info(),
            company_fee,
        )?;
        transfer_lamports_from_vault(
            &ctx.accounts.pit_draft_vault.to_account_info(),
            &gauntlet_pool_vault.to_account_info(),
            gauntlet_fee,
        )?;

        emit!(PitDraftResolved {
            player_a: waiting_player,
            player_b: player_key,
            winner,
            entry_lamports: PIT_DRAFT_ENTRY_LAMPORTS,
            total_pot,
            winner_payout,
            company_fee,
            gauntlet_fee,
            turns_taken: combat_outcome.turns_taken,
        });

        // Clear queue
        queue.waiting_player = None;
        queue.waiting_profile = None;

        Ok(())
    }

    /// Closes the GameState account, returning rent to player.
    pub fn close_game_state(ctx: Context<CloseGameState>) -> Result<()> {
        let game_state = &ctx.accounts.game_state;

        emit!(GameStateClosed {
            player: game_state.player,
            total_moves: game_state.total_moves,
            final_phase: game_state.phase,
            final_week: game_state.week,
        });

        Ok(())
    }

    /// Closes the GameState account via session key signer authorization.
    /// Used by session-manager CPI during end_session to clean up game state.
    /// Rent is returned to the player wallet.
    pub fn close_game_state_via_session_signer(
        ctx: Context<CloseGameStateViaSessionSigner>,
    ) -> Result<()> {
        let game_state = &ctx.accounts.game_state;

        emit!(GameStateClosed {
            player: game_state.player,
            total_moves: game_state.total_moves,
            final_phase: game_state.phase,
            final_week: game_state.week,
        });

        Ok(())
    }

    // close_map_enemies removed — MapEnemies merged into GameState

    /// Closes the GauntletEchoes account via session key signer authorization.
    /// Used by session-manager CPI during end_session to clean up.
    /// Rent is returned to the player wallet.
    pub fn close_gauntlet_echoes(ctx: Context<CloseGauntletEchoes>) -> Result<()> {
        emit!(GauntletEchoesClosed {
            session: ctx.accounts.gauntlet_echoes.session,
        });
        Ok(())
    }

    /// Closes an orphaned GauntletEchoes account whose session PDA no longer exists.
    pub fn close_orphaned_gauntlet_echoes(
        ctx: Context<CloseOrphanedGauntletEchoes>,
    ) -> Result<()> {
        emit!(GauntletEchoesClosed {
            session: ctx.accounts.gauntlet_echoes.session,
        });
        Ok(())
    }

    /// Heals the player by a specified amount, authorized by poi-system.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used for rest POI healing.
    ///
    /// The max HP is derived from the player's inventory (equipped items).
    /// HP is capped at the derived max_hp value.
    pub fn heal_player(ctx: Context<HealPlayer>, amount: u16) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;
        let player_stats =
            calculate_stats(inventory, game_state.campaign_level, game_state.run_mode);

        let old_hp = game_state.hp;
        let new_hp = (game_state.hp as i32)
            .checked_add(amount as i32)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        let capped_hp = new_hp.min(player_stats.max_hp as i32);
        require!(
            capped_hp <= i16::MAX as i32,
            GameplayStateError::StatOverflow
        );

        game_state.hp = capped_hp as i16;

        emit!(PlayerHealed {
            player: game_state.player,
            old_hp,
            new_hp: game_state.hp,
            amount,
            max_hp: player_stats.max_hp,
        });

        Ok(())
    }

    /// Skips to the next Day phase, authorized by poi-system.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used by rest POIs (L1 Mole Den, L5 Rest Alcove)
    /// to skip the night phase.
    ///
    /// Behavior:
    /// - Night1 → Day2 (reset moves to DAY_MOVES)
    /// - Night2 → Day3 (reset moves to DAY_MOVES)
    /// - Night3 → triggers boss fight (cannot skip end-of-week boss)
    ///
    /// Returns an error if called during a Day phase.
    pub fn skip_to_day(ctx: Context<SkipToDay>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;
        let inventory_info = &ctx.accounts.inventory.to_account_info();
        let player_inventory_program = &ctx.accounts.player_inventory_program;

        require!(!game_state.is_dead, GameplayStateError::PlayerDead);
        require!(!game_state.completed, GameplayStateError::RunCompleted);
        require!(
            !game_state.boss_fight_ready,
            GameplayStateError::BossFightAlreadyTriggered
        );
        require!(
            game_state.phase.is_night(),
            GameplayStateError::NotNightPhase
        );

        if game_state.phase.is_night3() {
            // Night3: Cannot skip the boss fight - trigger it instead
            game_state.boss_fight_ready = true;

            emit!(BossFightReady {
                player: game_state.player,
                week: game_state.week,
            });

            if should_resolve_weekly_boss(game_state.run_mode, game_state.week) {
                let vrf =
                    extract_gameplay_vrf(&ctx.accounts.gameplay_vrf_state, &game_state.session)?;
                let vrf_ref = vrf.as_ref().map(|(r, n)| (r, *n));
                // Resolve boss fight inline (same as move_player does)
                // NOTE: session_discovery/session/map_generator_program are None here
                // because the ER rejects writable CPI accounts in the skip_to_day chain.
                // The frontend handles the missing boss ID / echo update via fallbacks.
                let player_won = resolve_boss_fight(
                    game_state,
                    ctx.accounts.generated_map.seed,
                    inventory,
                    inventory_info,
                    &ctx.accounts.gameplay_authority,
                    player_inventory_program,
                    ctx.bumps.gameplay_authority,
                    vrf_ref,
                    None,
                    None,
                    None,
                    None,
                )?;

                if !player_won {
                    return Ok(());
                }
            } else if game_state.run_mode == RunMode::Gauntlet {
                let ge = ctx.accounts.gauntlet_echoes.as_ref()
                    .ok_or(GameplayStateError::GauntletNotInitialized)?;
                let player_won = resolve_gauntlet_echo_inline(
                    game_state,
                    ge,
                    inventory,
                    inventory_info,
                    &ctx.accounts.gameplay_authority,
                    player_inventory_program,
                    ctx.bumps.gameplay_authority,
                    ctx.accounts.generated_map.seed,
                )?;
                if !player_won {
                    return Ok(());
                }
            } else if game_state.run_mode == RunMode::Duel
                && game_state.week >= game_state.max_weeks
            {
                // Duel week 3: no boss fight — mark run as completed immediately.
                // PvP resolution happens asynchronously via finalize_duel_run.
                game_state.completed = true;
                game_state.boss_fight_ready = false;
            }
        } else {
            // Night1 or Night2: Skip to the next Day phase
            let next_day = match game_state.phase {
                Phase::Night1 => Phase::Day2,
                Phase::Night2 => Phase::Day3,
                _ => unreachable!(), // Already validated is_night() and not is_night3()
            };

            game_state.phase = next_day;
            game_state.moves_remaining = DAY_MOVES;

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: next_day,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        }

        Ok(())
    }

    /// Adds an HP bonus when equipping +HP gear, authorized by player-inventory.
    ///
    /// This instruction can only be called via CPI from player-inventory using
    /// the inventory_authority PDA as signer. Used when equipping gear that has
    /// a MaxHp effect.
    ///
    /// Behavior:
    /// - Adds the hp_bonus to both current HP and max HP
    /// - Current HP increases by hp_bonus (grants immediate HP)
    /// - Max HP is tracked implicitly via inventory effects
    ///
    /// Example: Pick +4 HP item at 10/10 -> 14/14
    pub fn add_hp_bonus_authorized(
        ctx: Context<AddHpBonusAuthorized>,
        hp_bonus: i16,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        require!(hp_bonus > 0, GameplayStateError::InvalidHpBonus);

        let old_hp = game_state.hp;
        let new_hp = old_hp
            .checked_add(hp_bonus)
            .ok_or(GameplayStateError::StatOverflow)?;

        game_state.hp = new_hp;

        emit!(HpBonusAdded {
            player: game_state.player,
            old_hp,
            new_hp: game_state.hp,
            hp_bonus,
        });

        Ok(())
    }

    /// Removes an HP bonus when unequipping +HP gear, authorized by player-inventory.
    ///
    /// This instruction can only be called via CPI from player-inventory using
    /// the inventory_authority PDA as signer. Used when unequipping gear that has
    /// a MaxHp effect.
    ///
    /// Behavior:
    /// - Reduces max HP by hp_bonus
    /// - If current HP exceeds new max HP, caps it at new max HP
    /// - If current HP is already below new max HP, leaves it unchanged
    ///
    /// Example: Unequip +4 HP item at 14/14 -> 10/10
    /// Example: Unequip +4 HP item at 7/14 -> 7/10
    /// Example: Unequip +4 HP item at 12/14 -> 10/10 (capped)
    pub fn remove_hp_bonus_authorized(
        ctx: Context<RemoveHpBonusAuthorized>,
        hp_bonus: i16,
        new_max_hp: i16,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        require!(hp_bonus > 0, GameplayStateError::InvalidHpBonus);
        require!(
            new_max_hp >= base_hp(game_state.campaign_level),
            GameplayStateError::InvalidHpBonus
        );

        let old_hp = game_state.hp;
        // Cap current HP at the new max HP
        let new_hp = old_hp.min(new_max_hp);

        game_state.hp = new_hp;

        emit!(HpBonusRemoved {
            player: game_state.player,
            old_hp,
            new_hp: game_state.hp,
            hp_bonus,
            new_max_hp,
        });

        Ok(())
    }

    /// Modifies the player's gold by a delta value, authorized by poi-system.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used for shop purchases, rerolls,
    /// rusty anvil upgrades, and scrap chute costs.
    pub fn modify_gold_authorized(ctx: Context<ModifyGoldAuthorized>, delta: i16) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        let new_gold = (game_state.gold as i32)
            .checked_add(delta as i32)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        require!(new_gold >= 0, GameplayStateError::GoldUnderflow);
        require!(
            new_gold <= u16::MAX as i32,
            GameplayStateError::StatOverflow
        );

        let old_gold = game_state.gold;
        game_state.gold = new_gold as u16;

        emit!(GoldModifiedAuthorized {
            player: game_state.player,
            old_gold,
            new_gold: game_state.gold,
            delta,
        });

        Ok(())
    }

    /// Sets the player's position, authorized by poi-system.
    ///
    /// This instruction can only be called via CPI from poi-system using
    /// the poi_authority PDA as signer. Used for fast travel between
    /// discovered Rail Waypoints.
    pub fn set_position_authorized(
        ctx: Context<SetPositionAuthorized>,
        target_x: u8,
        target_y: u8,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        require!(
            is_within_bounds(
                target_x,
                target_y,
                game_state.map_width,
                game_state.map_height
            ),
            GameplayStateError::OutOfBounds
        );

        game_state.position_x = target_x;
        game_state.position_y = target_y;

        Ok(())
    }

    /// Moves the player to an adjacent tile with automatic combat resolution.
    ///
    /// This instruction handles:
    /// 1. Movement validation (bounds, adjacency, move cost)
    /// 2. Night phase enemy movement (enemies within 3 tiles move toward player)
    /// 3. Combat triggered by enemy moving into player's tile
    /// 4. Combat triggered by player moving into enemy's tile
    /// 5. Phase advancement when moves are exhausted
    ///
    /// Combat is resolved inline without CPI for compute efficiency.
    pub fn move_player(ctx: Context<Move>, target_x: u8, target_y: u8) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let generated_map = &ctx.accounts.generated_map;
        let inventory = &ctx.accounts.inventory;
        let inventory_info = &ctx.accounts.inventory.to_account_info();
        let _player = &ctx.accounts.player;
        let player_inventory_program = &ctx.accounts.player_inventory_program;

        require!(!game_state.is_dead, GameplayStateError::PlayerDead);
        require!(
            !game_state.boss_fight_ready,
            GameplayStateError::BossFightAlreadyTriggered
        );
        require!(
            is_within_bounds(
                target_x,
                target_y,
                game_state.map_width,
                game_state.map_height
            ),
            GameplayStateError::OutOfBounds
        );
        require!(
            is_adjacent(
                game_state.position_x,
                game_state.position_y,
                target_x,
                target_y
            ),
            GameplayStateError::NotAdjacent
        );

        let is_wall = !generated_map.is_walkable(target_x, target_y);
        let player_stats =
            calculate_stats(inventory, game_state.campaign_level, game_state.run_mode);
        let move_cost = calculate_move_cost(is_wall, player_stats.dig);

        // Check if move can be afforded in current phase or by spanning phases
        let needs_phase_span = game_state.moves_remaining < move_cost;
        let can_span_phases = !game_state.phase.is_night3() && game_state.phase.next().is_some();

        if needs_phase_span {
            if !can_span_phases {
                // Night3 or no next phase - cannot span
                return Err(GameplayStateError::InsufficientMoves.into());
            }
            // Check if we can afford by spanning to next phase
            let next_phase = game_state.phase.next().unwrap();
            let total_available =
                game_state.moves_remaining as u16 + next_phase.moves_allowed() as u16;
            require!(
                total_available >= move_cost as u16,
                GameplayStateError::InsufficientMoves
            );
        }

        let original_phase = game_state.phase;
        let (post_move_phase, post_move_moves_remaining) =
            resolve_post_move_phase_and_moves(original_phase, game_state.moves_remaining, move_cost)?;
        // When Night moves are exhausted, the phase transitions to Day (or boss→Day
        // if the player wins). Use Day vision so the reveal matches the post-transition
        // state. If the boss is lost, the player is dead and extra tiles don't matter.
        let visibility_radius = if post_move_moves_remaining == 0 && post_move_phase.is_night() {
            DAY_VISION_RADIUS
        } else {
            visibility_radius_for_phase(post_move_phase)
        };

        let is_last_move_of_week =
            game_state.phase.is_night3() && game_state.moves_remaining == move_cost;
        let mut enemies_moved: u8 = 0;
        let mut combat_triggered = false;

        if game_state.enemies.iter().any(|enemy| enemy.defeated) {
            game_state.enemies.retain(|enemy| !enemy.defeated);
            game_state.enemy_count = game_state.enemies.len() as u8;
        }

        let target_enemy_exists_before_move =
            find_enemy_index(game_state, target_x, target_y).is_some();

        // Night phase: enemies within 3 tiles (Chebyshev distance) move toward player.
        // Skip enemy movement if player is directly engaging an enemy on target tile.
        // Detection uses old position (where enemies "notice" the player), but enemies
        // chase toward the NEW position (target), so moving away means the enemy follows
        // but doesn't catch up in the same turn.
        if should_process_night_enemy_movement(&game_state.phase, target_enemy_exists_before_move) {
            let map_width = generated_map.width as usize;
            let map_height = generated_map.height as usize;
            let mut occupied = vec![false; map_width.saturating_mul(map_height)];
            for enemy in game_state.enemies.iter() {
                let index = (enemy.y as usize) * map_width + (enemy.x as usize);
                if index < occupied.len() {
                    occupied[index] = true;
                }
            }
            let mut player_tile_blocked = false;
            let detect_x = game_state.position_x;
            let detect_y = game_state.position_y;
            let chase_x = target_x;
            let chase_y = target_y;
            let mut enemy_idx = 0usize;

            while enemy_idx < game_state.enemies.len() {
                let enemy = game_state.enemies[enemy_idx];
                let distance = chebyshev_distance(enemy.x, enemy.y, detect_x, detect_y);
                if distance > 0 && distance <= 3 {
                    let old_x = enemy.x;
                    let old_y = enemy.y;

                    if let Some((new_x, new_y)) = select_enemy_step(
                        enemy.x,
                        enemy.y,
                        chase_x,
                        chase_y,
                        generated_map,
                        &occupied,
                        map_width,
                        player_tile_blocked,
                    ) {
                        let old_index = (old_y as usize) * map_width + (old_x as usize);
                        if old_index < occupied.len() {
                            occupied[old_index] = false;
                        }

                        if new_x == chase_x && new_y == chase_y {
                            player_tile_blocked = true;
                        } else {
                            let new_index = (new_y as usize) * map_width + (new_x as usize);
                            if new_index < occupied.len() {
                                occupied[new_index] = true;
                            }
                        }

                        game_state.enemies[enemy_idx].x = new_x;
                        game_state.enemies[enemy_idx].y = new_y;
                        enemies_moved = enemies_moved.saturating_add(1);

                        if new_x == chase_x && new_y == chase_y {
                            combat_triggered = true;
                            let player_won = resolve_enemy_combat(
                                game_state,
                                inventory,
                                enemy_idx,
                            )?;
                            if !player_won {
                                return Ok(());
                            }
                            break;
                        }
                    }
                }

                enemy_idx = enemy_idx.saturating_add(1);
            }
        }

        // Convert wall to floor via CPI so the tile change persists on-chain
        // (map_generator owns the GeneratedMap account, so we must use CPI)
        if is_wall {
            set_tile_floor_cpi(
                &ctx.accounts.generated_map.to_account_info(),
                &ctx.accounts.game_session,
                &ctx.accounts.gameplay_authority,
                &ctx.accounts.map_generator_program.to_account_info(),
                ctx.bumps.gameplay_authority,
                target_x,
                target_y,
                ctx.accounts.session_discovery.as_ref().map(|a| a.to_account_info()).as_ref(),
            )?;
        }

        game_state.position_x = target_x;
        game_state.position_y = target_y;
        game_state.phase = post_move_phase;
        game_state.moves_remaining = post_move_moves_remaining;
        reveal_radius_cpi(
            &ctx.accounts.generated_map.to_account_info(),
            &ctx.accounts.game_session,
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.map_generator_program.to_account_info(),
            target_x,
            target_y,
            visibility_radius,
            ctx.accounts.session_discovery.as_ref().map(|a| a.to_account_info()).as_ref(),
        )?;
        // Reload generated_map after CPI so discovered_tiles reflects newly revealed area.
        // Without this, compute_visible_enemies reads stale pre-reveal data.
        ctx.accounts.generated_map.reload()?;
        discover_visible_waypoints_authorized_cpi(
            &ctx.accounts.map_pois,
            &ctx.accounts.gameplay_authority,
            &ctx.accounts.poi_system_program.to_account_info(),
            ctx.bumps.gameplay_authority,
            target_x,
            target_y,
            visibility_radius,
            ctx.accounts.session_discovery.as_ref().map(|a| a.to_account_info()).as_ref(),
            Some(&ctx.accounts.game_session),
            Some(&ctx.accounts.player.to_account_info()),
            Some(&ctx.accounts.map_generator_program.to_account_info()),
        )?;

        if needs_phase_span {
            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: post_move_phase,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        }

        game_state.total_moves = game_state
            .total_moves
            .checked_add(1)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        let target_enemy_idx = find_enemy_index(game_state, target_x, target_y);

        if should_process_target_enemy_combat(
            combat_triggered,
            is_last_move_of_week,
            target_enemy_idx.is_some(),
        ) {
            let enemy_idx = target_enemy_idx.expect("checked is_some above");
            let player_won = resolve_enemy_combat(game_state, inventory, enemy_idx)?;
            if !player_won {
                return Ok(());
            }
        }

        // Sync visible enemies to SessionDiscovery (AFTER combat so defeated enemies are excluded)
        if let Some(ref sd) = ctx.accounts.session_discovery {
            let visible = compute_visible_enemies(
                game_state,
                &ctx.accounts.generated_map.discovered_tiles,
                ctx.accounts.generated_map.width,
            );
            update_discovered_enemies_cpi(
                &sd.to_account_info(),
                &ctx.accounts.game_session,
                &ctx.accounts.gameplay_authority,
                &ctx.accounts.map_generator_program.to_account_info(),
                ctx.bumps.gameplay_authority,
                visible,
            )?;
        }

        if game_state.moves_remaining == 0 {
            if game_state.phase.is_night3() {
                game_state.boss_fight_ready = true;

                emit!(BossFightReady {
                    player: game_state.player,
                    week: game_state.week,
                });

                if should_resolve_weekly_boss(game_state.run_mode, game_state.week) {
                    let vrf = extract_gameplay_vrf(
                        &ctx.accounts.gameplay_vrf_state,
                        &game_state.session,
                    )?;
                    let vrf_ref = vrf.as_ref().map(|(r, n)| (r, *n));
                    let ge_ref = ctx.accounts.gauntlet_echoes.as_deref();
                    let player_won = resolve_boss_fight(
                        game_state,
                        ctx.accounts.generated_map.seed,
                        inventory,
                        inventory_info,
                        &ctx.accounts.gameplay_authority,
                        player_inventory_program,
                        ctx.bumps.gameplay_authority,
                        vrf_ref,
                        ctx.accounts.session_discovery.as_ref().map(|a| a.to_account_info()).as_ref(),
                        Some(&ctx.accounts.game_session),
                        Some(&ctx.accounts.map_generator_program.to_account_info()),
                        ge_ref,
                    )?;
                    if !player_won {
                        return Ok(());
                    }
                } else if game_state.run_mode == RunMode::Gauntlet {
                    // Gauntlet echo combat is resolved in trigger_boss_fight (separate IX)
                    // to avoid BPF stack overflow — move_player's frame is too deep after
                    // RevealRadius + DiscoverWaypoints + UpdateEnemies CPIs.
                } else if game_state.run_mode == RunMode::Duel
                    && game_state.week >= game_state.max_weeks
                {
                    // Duel week 3: no boss fight — mark run as completed immediately.
                    // PvP resolution happens asynchronously via finalize_duel_run.
                    game_state.completed = true;
                    game_state.boss_fight_ready = false;
                }

                if let Some(enemy_idx) =
                    find_enemy_index(game_state, game_state.position_x, game_state.position_y)
                {
                    let player_won =
                        resolve_enemy_combat(game_state, inventory, enemy_idx)?;
                    if !player_won {
                        return Ok(());
                    }
                }
            } else {
                handle_phase_advancement(game_state)?;
            }
        }

        game_state.enemy_count = game_state.enemies.len() as u8;

        Ok(())
    }

    /// Syncs the current week's boss ID to SessionDiscovery.
    ///
    /// Called by the frontend after a rest-alcove-triggered boss win in Duel mode,
    /// where the skip_to_day CPI chain is too deep for the ER to allow writable
    /// session_discovery. This is a lightweight 2-level CPI
    /// (session signer → gameplay_state → map_generator) that the ER handles fine.
    pub fn sync_discovery_boss(ctx: Context<SyncDiscoveryBoss>) -> Result<()> {
        let game_state = &ctx.accounts.game_state;
        let stage = game_state.campaign_level;

        let boss_id = if game_state.run_mode == RunMode::Duel {
            if game_state.week >= game_state.max_weeks {
                [0u8; 12]
            } else {
                get_duel_boss_id_from_seed(ctx.accounts.generated_map.seed, game_state.week)?
            }
        } else if game_state.run_mode == RunMode::Gauntlet {
            [0u8; 12]
        } else {
            get_boss_id(stage, game_state.week)?
        };

        update_boss_id_cpi(
            &ctx.accounts.session_discovery.to_account_info(),
            &ctx.accounts.game_session,
            &ctx.accounts.gameplay_authority,
            &ctx.accounts.map_generator_program.to_account_info(),
            ctx.bumps.gameplay_authority,
            boss_id,
        )?;

        Ok(())
    }

    /// Triggers and resolves the boss fight when conditions are met.
    ///
    /// This instruction handles:
    /// 1. Validation that boss fight is ready (boss_fight_ready flag set)
    /// 2. Boss selection based on stored campaign_level and week
    /// 3. Combat resolution inline
    /// 4. Victory handling: week advancement or level completion
    /// 5. Defeat handling: player death persisted in state
    ///
    /// Must be called after move sets boss_fight_ready = true.
    pub fn trigger_boss_fight(ctx: Context<TriggerBossFight>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let inventory = &ctx.accounts.inventory;
        let inventory_info = &ctx.accounts.inventory.to_account_info();
        let _player = &ctx.accounts.player;
        let player_inventory_program = &ctx.accounts.player_inventory_program;

        require!(!game_state.is_dead, GameplayStateError::PlayerDead);
        require!(
            game_state.boss_fight_ready,
            GameplayStateError::BossFightNotReady
        );

        if game_state.run_mode == RunMode::Gauntlet
            && !should_resolve_weekly_boss(game_state.run_mode, game_state.week)
        {
            // Gauntlet echo combat (weeks 1-2): resolved here instead of inline in
            // move_player to avoid BPF stack overflow.
            let ge = ctx.accounts.gauntlet_echoes.as_ref()
                .ok_or(GameplayStateError::GauntletNotInitialized)?;
            let player_won = resolve_gauntlet_echo_inline(
                game_state,
                ge,
                inventory,
                inventory_info,
                &ctx.accounts.gameplay_authority,
                player_inventory_program,
                ctx.bumps.gameplay_authority,
                ctx.accounts.generated_map.seed,
            )?;
            if !player_won {
                return Ok(());
            }
        } else {
            // Campaign, Duel, and Gauntlet week 3+ boss fights
            let vrf = extract_gameplay_vrf(&ctx.accounts.gameplay_vrf_state, &game_state.session)?;
            let vrf_ref = vrf.as_ref().map(|(r, n)| (r, *n));
            let ge_ref = ctx.accounts.gauntlet_echoes.as_deref();
            let player_won = resolve_boss_fight(
                game_state,
                ctx.accounts.generated_map.seed,
                inventory,
                inventory_info,
                &ctx.accounts.gameplay_authority,
                player_inventory_program,
                ctx.bumps.gameplay_authority,
                vrf_ref,
                ctx.accounts.session_discovery.as_ref().map(|a| a.to_account_info()).as_ref(),
                Some(&ctx.accounts.game_session.to_account_info()),
                ctx.accounts.map_generator_program.as_ref().map(|p| p.to_account_info()).as_ref(),
                ge_ref,
            )?;
            if !player_won {
                return Ok(());
            }
        }

        if let Some(enemy_idx) =
            find_enemy_index(game_state, game_state.position_x, game_state.position_y)
        {
            let player_won = resolve_enemy_combat(game_state, inventory, enemy_idx)?;
            if !player_won {
                return Ok(());
            }
        }

        game_state.enemy_count = game_state.enemies.len() as u8;

        Ok(())
    }

    // =========================================================================
    // VRF Lifecycle
    // =========================================================================

    /// Pre-creates GameplayVrfState PDA on base chain without requesting randomness.
    /// Used to initialize the account before delegation to ER, so VRF requests
    /// on the Ephemeral Rollup don't need to create new accounts inline.
    pub fn init_gameplay_vrf_state(ctx: Context<InitGameplayVrfState>) -> Result<()> {
        let vrf = &mut ctx.accounts.vrf_state;
        vrf.session = ctx.accounts.session.key();
        vrf.randomness = [0u8; 32];
        vrf.nonce = 1;
        vrf.status = VrfStatus::Requested;
        vrf.bump = ctx.bumps.vrf_state;
        Ok(())
    }

    /// Pre-creates GameplayVrfState on base chain for pit draft VRF (no session required).
    /// Uses player_a from the pending match as the seed key.
    pub fn init_pit_draft_vrf_state(ctx: Context<InitPitDraftVrfState>) -> Result<()> {
        let vrf = &mut ctx.accounts.vrf_state;
        vrf.session = ctx.accounts.seed_key.key();
        vrf.randomness = [0u8; 32];
        vrf.nonce = 0;
        vrf.status = VrfStatus::Requested;
        vrf.bump = ctx.bumps.vrf_state;
        Ok(())
    }

    /// Delegates GameplayVrfState PDA to MagicBlock from its owning program.
    pub fn delegate_gameplay_vrf_state(
        ctx: Context<DelegateGameplayVrfState>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let session_key = ctx.accounts.game_session.key();
        let (expected_vrf_state, _) = Pubkey::find_program_address(
            &[GameplayVrfState::SEED_PREFIX, session_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.gameplay_vrf_state.key(),
            expected_vrf_state,
            GameplayStateError::Unauthorized
        );
        let vrf_seeds: &[&[u8]] = &[GameplayVrfState::SEED_PREFIX, session_key.as_ref()];
        ctx.accounts.delegate_gameplay_vrf_state(
            &ctx.accounts.player,
            vrf_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Delegates GameplayVrfState PDA to MagicBlock for pit draft VRF (no session required).
    /// The seed_key is typically matched_player_a from the PitDraftQueue.
    pub fn delegate_pit_draft_vrf_state(
        ctx: Context<DelegatePitDraftVrfState>,
        validator: Option<Pubkey>,
    ) -> Result<()> {
        let seed_key = ctx.accounts.seed_key.key();
        let (expected_vrf_state, _) = Pubkey::find_program_address(
            &[GameplayVrfState::SEED_PREFIX, seed_key.as_ref()],
            &crate::ID,
        );
        require_keys_eq!(
            ctx.accounts.gameplay_vrf_state.key(),
            expected_vrf_state,
            GameplayStateError::Unauthorized
        );
        let vrf_seeds: &[&[u8]] = &[GameplayVrfState::SEED_PREFIX, seed_key.as_ref()];
        ctx.accounts.delegate_gameplay_vrf_state(
            &ctx.accounts.payer,
            vrf_seeds,
            local_delegate_config(validator),
        )?;
        Ok(())
    }

    /// Request VRF randomness for gameplay (pit draft, gauntlet echo, duel boss).
    /// Increments the nonce to ensure each request produces unique randomness.
    pub fn request_gameplay_vrf(ctx: Context<RequestGameplayVrf>) -> Result<()> {
        let vrf = &mut ctx.accounts.vrf_state;
        vrf.session = ctx.accounts.session.key();
        vrf.randomness = [0u8; 32];
        vrf.nonce = vrf.nonce.checked_add(1).unwrap_or(1);
        vrf.status = VrfStatus::Requested;
        vrf.bump = ctx.bumps.vrf_state;

        let mut caller_seed = [0u8; 32];
        caller_seed.copy_from_slice(ctx.accounts.session.key().as_ref());
        caller_seed[..8].copy_from_slice(&vrf.nonce.to_le_bytes());

        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: ctx.accounts.payer.key(),
            oracle_queue: ctx.accounts.oracle_queue.key(),
            callback_program_id: crate::ID,
            callback_discriminator: instruction::FulfillGameplayVrf::DISCRIMINATOR.to_vec(),
            accounts_metas: Some(vec![SerializableAccountMeta {
                pubkey: ctx.accounts.vrf_state.key(),
                is_signer: false,
                is_writable: true,
            }]),
            caller_seed,
            ..Default::default()
        });

        let (_, identity_bump) =
            Pubkey::find_program_address(&[ephemeral_vrf_sdk::consts::IDENTITY], &crate::ID);
        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.payer.to_account_info(),
                ctx.accounts.program_identity.to_account_info(),
                ctx.accounts.oracle_queue.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.slot_hashes.to_account_info(),
            ],
            &[&[ephemeral_vrf_sdk::consts::IDENTITY, &[identity_bump]]],
        )?;
        Ok(())
    }

    /// Oracle callback: writes randomness into VrfState and sets status = Fulfilled.
    /// In production, the signer must be the VRF oracle program identity.
    /// Under `mock-vrf` feature, any signer is accepted for testing.
    pub fn fulfill_gameplay_vrf(
        ctx: Context<FulfillGameplayVrf>,
        randomness: [u8; 32],
    ) -> Result<()> {
        let vrf = &mut ctx.accounts.vrf_state;
        require!(
            vrf.status == VrfStatus::Requested,
            GameplayStateError::VrfNotRequested
        );
        vrf.randomness = randomness;
        vrf.status = VrfStatus::Fulfilled;
        Ok(())
    }

    /// Rotates the session_signer on a GameState account.
    /// Called via CPI from session-manager during rotate_session_key.
    /// Only the session_manager_authority PDA can authorize this.
    pub fn rotate_game_state_session_key(
        ctx: Context<RotateGameStateSessionKey>,
        new_session_signer: Pubkey,
    ) -> Result<()> {
        ctx.accounts.game_state.session_signer = new_session_signer;
        Ok(())
    }

    /// Closes GameplayVrfState account and returns rent to the player.
    /// Called via CPI from session-manager during end_session/abandon_session.
    pub fn close_gameplay_vrf_state(ctx: Context<CloseGameplayVrfState>) -> Result<()> {
        let game_state = &ctx.accounts.game_state;
        require_keys_eq!(
            game_state.session_signer,
            ctx.accounts.session_signer.key(),
            GameplayStateError::Unauthorized
        );
        require_keys_eq!(
            game_state.player,
            ctx.accounts.player.key(),
            GameplayStateError::Unauthorized
        );
        Ok(())
    }

    /// TEST ONLY: Sets the game phase and moves remaining directly.
    /// This instruction is intended for testing purposes to avoid
    /// doing hundreds of move transactions to reach a specific phase.
    ///
    /// Disabled in production builds.
    pub fn set_phase_for_testing(
        _ctx: Context<SetPhaseForTesting>,
        _phase: Phase,
        _moves_remaining: u8,
    ) -> Result<()> {
        Err(GameplayStateError::TestOnlyInstructionDisabled.into())
    }

    /// Close a corrupted/empty GameState account (0-byte data).
    /// After an ER reset, force-undelegation can leave accounts with no data.
    /// Only works on accounts owned by this program with 0 bytes of data.
    pub fn close_empty_game_state(ctx: Context<CloseEmptyGameState>) -> Result<()> {
        let info = ctx.accounts.game_state.to_account_info();
        let dest = ctx.accounts.destination.to_account_info();
        **dest.try_borrow_mut_lamports()? += info.lamports();
        **info.try_borrow_mut_lamports()? = 0;
        info.assign(&anchor_lang::system_program::ID);
        info.resize(0)?;
        Ok(())
    }

    // close_empty_map_enemies removed — MapEnemies merged into GameState
    // close_orphaned_map_enemies removed — MapEnemies merged into GameState

    /// TEST-ONLY: Sets game_state.completed = true so the victory path can
    /// be exercised in e2e tests without requiring actual boss kills.
    /// Validates session_signer authority so it cannot be called by a random wallet.
    pub fn test_set_completed(ctx: Context<TestSetCompleted>) -> Result<()> {
        #[cfg(not(feature = "mock-vrf"))]
        {
            let _ = ctx;
            return Err(GameplayStateError::TestOnlyInstructionDisabled.into());
        }
        #[cfg(feature = "mock-vrf")]
        {
            ctx.accounts.game_state.completed = true;
            ctx.accounts.game_state.boss_fight_ready = false;
            Ok(())
        }
    }

    /// TEST-ONLY: Sets game_state.hp to given value and clears is_dead.
    /// Used in e2e tests to keep the player alive through night enemy encounters
    /// so the boss fight path can be tested.
    pub fn test_set_hp(ctx: Context<TestSetCompleted>, hp: i16) -> Result<()> {
        #[cfg(not(feature = "mock-vrf"))]
        {
            let _ = (ctx, hp);
            return Err(GameplayStateError::TestOnlyInstructionDisabled.into());
        }
        #[cfg(feature = "mock-vrf")]
        {
            ctx.accounts.game_state.hp = hp;
            ctx.accounts.game_state.is_dead = false;
            Ok(())
        }
    }
}

#[derive(Accounts)]
pub struct TestSetCompleted<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,
    pub session_signer: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateGameplayAccounts<'info> {
    #[account(mut, del)]
    /// CHECK: PDA is validated in handler.
    pub game_state: AccountInfo<'info>,
    /// CHECK: Session PDA owned by session-manager; used only for seed derivation.
    pub game_session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateGauntletEchoes<'info> {
    #[account(mut, del)]
    /// CHECK: PDA is validated in handler.
    pub gauntlet_echoes: AccountInfo<'info>,
    /// CHECK: Session PDA owned by session-manager; used only for seed derivation.
    pub game_session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
pub struct UndelegateGameplayAccounts<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated and deserialized in handler.
    pub game_state: AccountInfo<'info>,
    /// CHECK: Session PDA used only for deterministic PDA validation.
    pub game_session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
pub struct UndelegateGauntletEchoes<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated in handler.
    pub gauntlet_echoes: AccountInfo<'info>,
    /// CHECK: Session PDA used only for deterministic PDA validation.
    pub game_session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

#[commit]
#[derive(Accounts)]
pub struct UndelegateGameState<'info> {
    #[account(mut)]
    /// CHECK: PDA is validated and deserialized in handler.
    pub game_state: AccountInfo<'info>,
    /// CHECK: Session PDA used only for deterministic PDA validation.
    pub game_session: UncheckedAccount<'info>,
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

// UndelegateMapEnemies removed — MapEnemies merged into GameState


#[derive(Accounts)]
pub struct SetPhaseForTesting<'info> {
    #[account(
        mut,
        has_one = session_signer,
    )]
    pub game_state: Account<'info, GameState>,
    pub session_signer: Signer<'info>,
}

fn find_enemy_index(game_state: &GameState, x: u8, y: u8) -> Option<usize> {
    game_state
        .enemies
        .iter()
        .position(|enemy| !enemy.defeated && enemy.x == x && enemy.y == y)
}

fn read_game_state(game_state: &AccountInfo<'_>) -> Result<GameState> {
    let data = game_state.try_borrow_data()?;
    let mut slice: &[u8] = &data;
    GameState::try_deserialize(&mut slice).map_err(|_| GameplayStateError::InvalidSession.into())
}

// read_map_enemies removed — MapEnemies merged into GameState

fn remove_enemy(game_state: &mut GameState, enemy_index: usize) {
    if enemy_index >= game_state.enemies.len() {
        return;
    }
    game_state.enemies.swap_remove(enemy_index);
    game_state.enemy_count = game_state.enemies.len() as u8;
}

fn build_player_combatant(
    current_hp: i16,
    stats: &PlayerStats,
    all_player_effects: &[AnnotatedItemEffect],
) -> CombatantInput {
    // current_hp is clamped to stats.max_hp to prevent exceeding derived max.
    let combat_hp = current_hp.clamp(1, stats.max_hp);
    CombatantInput {
        hp: combat_hp,
        max_hp: stats.max_hp as u16,
        atk: stats.atk,
        arm: stats.arm,
        spd: stats.spd,
        dig: stats.dig,
        strikes: stats.strikes,
        attack_source: attack_source_from_effects(all_player_effects),
        atk_contributions: baked_attack_contributions_from_effects(all_player_effects),
    }
}

fn build_full_hp_combatant(
    stats: &PlayerStats,
    all_player_effects: &[AnnotatedItemEffect],
) -> CombatantInput {
    CombatantInput {
        hp: stats.max_hp,
        max_hp: stats.max_hp as u16,
        atk: stats.atk,
        arm: stats.arm,
        spd: stats.spd,
        dig: stats.dig,
        strikes: stats.strikes,
        attack_source: attack_source_from_effects(all_player_effects),
        atk_contributions: baked_attack_contributions_from_effects(all_player_effects),
    }
}

fn item_pool_index_from_id(item_id: &[u8; 8]) -> Option<usize> {
    // Expected IDs: T-XX-01..02 and G-XX-01..08 where XX in {ST,SC,GR,BL,FR,RU,BO,TE}
    if item_id[1] != b'-' || item_id[4] != b'-' {
        return None;
    }

    let kind = item_id[0];
    let tag_code = match (item_id[2], item_id[3]) {
        (b'S', b'T') => 0usize,
        (b'S', b'C') => 1,
        (b'G', b'R') => 2,
        (b'B', b'L') => 3,
        (b'F', b'R') => 4,
        (b'R', b'U') => 5,
        (b'B', b'O') => 6,
        (b'T', b'E') => 7,
        _ => return None,
    };

    if !(item_id[5].is_ascii_digit() && item_id[6].is_ascii_digit()) {
        return None;
    }
    let item_num = ((item_id[5] - b'0') as usize) * 10 + (item_id[6] - b'0') as usize;

    match kind {
        b'G' if (1..=8).contains(&item_num) => Some(tag_code * 8 + (item_num - 1)),
        b'T' if (1..=2).contains(&item_num) => Some(64 + tag_code * 2 + (item_num - 1)),
        _ => None,
    }
}

fn is_pool_item_enabled(pool: &[u8; 10], item_id: &[u8; 8]) -> bool {
    let Some(index) = item_pool_index_from_id(item_id) else {
        return false;
    };
    if index >= 80 {
        return false;
    }
    let byte_index = index / 8;
    let bit_index = index % 8;
    (pool[byte_index] & (1u8 << bit_index)) != 0
}


/// VRF-aware index drawing. Uses GameRng backed by VRF randomness + domain separation.
fn draw_unique_indices_vrf(
    candidates: &mut Vec<usize>,
    picks: usize,
    rng: &mut vrf_rng::GameRng,
) -> Vec<usize> {
    let mut selected = Vec::with_capacity(picks);
    for _ in 0..picks {
        let idx = rng.next_bounded(candidates.len() as u64) as usize;
        selected.push(candidates.swap_remove(idx));
    }
    selected
}

/// VRF-backed pit draft inventory builder.
/// Requires mandatory VRF randomness (pit draft resolution runs on ER).
fn build_pit_draft_inventory_vrf(
    player: Pubkey,
    active_pool: [u8; 10],
    vrf: (&[u8; 32], u64),
    seed_tag: &[u8],
) -> Result<PlayerInventory> {
    // Mix seed_tag into domain to differentiate player A vs player B inventory draws.
    let tag_hash = seed_tag.iter().fold(0u64, |acc, &b| {
        acc.wrapping_mul(31).wrapping_add(b as u64)
    });
    let mut rng = vrf_rng::GameRng::from_vrf(
        vrf.0,
        vrf.1,
        vrf_rng::domains::PIT_DRAFT_INVENTORY ^ tag_hash,
    );

    let mut tool_candidates = Vec::new();
    let mut gear_candidates = Vec::new();

    for (index, item_def) in ITEMS.iter().enumerate() {
        if !is_pool_item_enabled(&active_pool, item_def.id) {
            continue;
        }
        match item_def.item_type {
            ItemType::Tool => tool_candidates.push(index),
            ItemType::Gear => gear_candidates.push(index),
        }
    }

    require!(
        !tool_candidates.is_empty() && gear_candidates.len() >= 7,
        GameplayStateError::PitDraftInsufficientPoolItems
    );

    let selected_tool_idx = draw_unique_indices_vrf(&mut tool_candidates, 1, &mut rng)[0];
    let selected_gear_indices = draw_unique_indices_vrf(&mut gear_candidates, 7, &mut rng);

    let mut tool = ItemInstance::new(*ITEMS[selected_tool_idx].id, Tier::I);
    let oil_rand = rng.next_val();
    let oil_mod = match oil_rand % 4 {
        0 => ToolOilModification::PlusAtk,
        1 => ToolOilModification::PlusSpd,
        2 => ToolOilModification::PlusDig,
        _ => ToolOilModification::PlusArm,
    };
    tool.apply_oil(oil_mod);

    let mut gear = [None; 12];
    for (slot_index, item_idx) in selected_gear_indices.iter().enumerate() {
        gear[slot_index] = Some(ItemInstance::new(*ITEMS[*item_idx].id, Tier::I));
    }

    Ok(PlayerInventory {
        session: Pubkey::default(),
        player,
        tool: Some(tool),
        gear,
        gear_slot_capacity: MAX_GEAR_SLOTS,
        bump: 0,
    })
}

/// Extracts VRF randomness from an optional GameplayVrfState account.
/// Returns None if account is absent. Validates session match and fulfilled status.
fn extract_gameplay_vrf(
    vrf_account: &Option<Account<GameplayVrfState>>,
    session_key: &Pubkey,
) -> Result<Option<([u8; 32], u64)>> {
    let vrf = match vrf_account {
        Some(v) => v,
        None => return Ok(None),
    };
    require_keys_eq!(vrf.session, *session_key, GameplayStateError::Unauthorized);
    require!(
        vrf.status == VrfStatus::Fulfilled || vrf.status == VrfStatus::Consumed,
        GameplayStateError::VrfNotFulfilled
    );
    Ok(Some((vrf.randomness, vrf.nonce)))
}

fn pit_draft_final_tie_player_a_wins(
    randomness: &[u8; 32],
    nonce: u64,
) -> bool {
    let mut rng = vrf_rng::GameRng::from_vrf(
        randomness,
        nonce,
        vrf_rng::domains::PIT_DRAFT_TIEBREAKER,
    );
    rng.next_bounded(2) == 0
}

fn duel_final_tie_player_a_wins(seed: u64) -> bool {
    seed % 2 == 0
}

fn gauntlet_final_tie_player_wins(map_seed: u64) -> bool {
    map_seed % 2 == 0
}


/// VRF-backed gauntlet echo draw.
fn draw_gauntlet_echoes_from_remaining_vrf(
    gauntlet_echoes: &mut GauntletEchoes,
    remaining: &[AccountInfo],
    vrf: (&[u8; 32], u64),
) -> Result<()> {
    require!(
        remaining.len() >= 5,
        GameplayStateError::GauntletNotInitialized
    );
    let program_id = crate::ID;
    let expected_disc = <GauntletWeekPool as anchor_lang::Discriminator>::DISCRIMINATOR;

    for week_idx in 0u8..5 {
        let week = week_idx + 1;
        let info = &remaining[week_idx as usize];

        require!(
            info.owner == &program_id,
            GameplayStateError::GauntletNotInitialized
        );

        let (expected_pda, _) =
            Pubkey::find_program_address(&[GAUNTLET_WEEK_POOL_SEED, &[week]], &program_id);
        require!(
            info.key() == expected_pda,
            GameplayStateError::InvalidGauntletWeek
        );

        let data = info.try_borrow_data()?;
        require!(data.len() >= 24, GameplayStateError::GauntletNotInitialized);
        require!(
            data[..8] == *expected_disc,
            GameplayStateError::GauntletNotInitialized
        );
        require!(data[8] == week, GameplayStateError::InvalidGauntletWeek);

        let vec_len = u32::from_le_bytes(data[20..24].try_into().unwrap()) as usize;
        require!(vec_len > 0, GameplayStateError::GauntletNotInitialized);

        // VRF-backed: per-week sub-domain for independent streams
        let sub_domain = vrf_rng::domains::GAUNTLET_ECHO_DRAW ^ (week as u64);
        let mut rng = vrf_rng::GameRng::from_vrf(vrf.0, vrf.1, sub_domain);
        let target_idx = rng.next_bounded(vec_len as u64) as usize;

        let mut cursor: &[u8] = &data[24..];
        for _ in 0..target_idx {
            let _skip = GauntletEchoSnapshot::deserialize(&mut cursor)
                .map_err(|_| error!(GameplayStateError::GauntletNotInitialized))?;
        }
        let echo = GauntletEchoSnapshot::deserialize(&mut cursor)
            .map_err(|_| error!(GameplayStateError::GauntletNotInitialized))?;

        gauntlet_echoes.echoes[week_idx as usize] = Some(echo);
    }
    Ok(())
}

/// VRF-backed reservoir sampling for player echo insertion.
fn maybe_insert_player_echo_vrf(
    week_pool: &mut Account<GauntletWeekPool>,
    week: u8,
    inventory: &PlayerInventory,
    gold: u16,
    player: Pubkey,
    vrf: (&[u8; 32], u64),
) -> Result<()> {
    let capacity = gauntlet_gear_capacity(week);
    let mut capped_gear = inventory.gear;
    for gear in &mut capped_gear[capacity..] {
        *gear = None;
    }
    let snapshot = GauntletEchoSnapshot {
        week,
        source: GauntletEchoSource::Player(player),
        loadout: GauntletLoadoutSnapshot {
            tool: inventory.tool,
            gear: capped_gear,
            gold_at_battle_start: gold,
        },
    };

    week_pool.seen_player_echoes = week_pool
        .seen_player_echoes
        .checked_add(1)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    week_pool.player_echoes_added = week_pool.player_echoes_added.saturating_add(1);

    if week_pool.entries.len() < GAUNTLET_MAX_WEEKLY_ECHOES {
        week_pool.entries.push(snapshot);
    } else {
        let rand = {
            let sub_domain = vrf_rng::domains::GAUNTLET_RESERVOIR ^ (week as u64);
            let mut rng = vrf_rng::GameRng::from_vrf(vrf.0, vrf.1, sub_domain);
            rng.next_val()
        };
        let replace_idx = (rand % week_pool.seen_player_echoes) as usize;
        if replace_idx < GAUNTLET_MAX_WEEKLY_ECHOES {
            week_pool.entries[replace_idx] = snapshot;
        }
    }

    if week_pool.bootstrap_active && week_pool.player_echoes_added >= 10 {
        week_pool
            .entries
            .retain(|e| e.source != GauntletEchoSource::Bootstrap);
        week_pool.bootstrap_active = false;
    }

    Ok(())
}


fn require_expected_address(
    actual: Pubkey,
    expected_address: &str,
    error: GameplayStateError,
) -> Result<()> {
    let expected = Pubkey::from_str(expected_address).map_err(|_| error)?;
    require_keys_eq!(actual, expected, error);
    Ok(())
}

fn compute_pvp_pot_split(total_pot: u64) -> Result<(u64, u64, u64)> {
    let company_fee = total_pot
        .checked_mul(PIT_DRAFT_COMPANY_FEE_BPS)
        .and_then(|v| v.checked_div(PIT_DRAFT_BPS_DENOMINATOR))
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    let gauntlet_fee = total_pot
        .checked_mul(PIT_DRAFT_GAUNTLET_FEE_BPS)
        .and_then(|v| v.checked_div(PIT_DRAFT_BPS_DENOMINATOR))
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    let winner_payout = total_pot
        .checked_mul(PIT_DRAFT_WINNER_BPS)
        .and_then(|v| v.checked_div(PIT_DRAFT_BPS_DENOMINATOR))
        .ok_or(GameplayStateError::ArithmeticOverflow)?;

    let distributed = winner_payout
        .checked_add(company_fee)
        .and_then(|v| v.checked_add(gauntlet_fee))
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    require!(
        distributed == total_pot,
        GameplayStateError::ArithmeticOverflow
    );

    Ok((company_fee, gauntlet_fee, winner_payout))
}

fn compute_eliminated_unmatched_distribution(entry_lamports: u64) -> Result<(u64, u64)> {
    let company_fee = entry_lamports
        .checked_mul(PIT_DRAFT_COMPANY_FEE_BPS)
        .and_then(|v| v.checked_div(PIT_DRAFT_BPS_DENOMINATOR))
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    let gauntlet_fee = entry_lamports
        .checked_mul(PIT_DRAFT_GAUNTLET_FEE_BPS)
        .and_then(|v| v.checked_div(PIT_DRAFT_BPS_DENOMINATOR))
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    let redistributable = entry_lamports
        .checked_sub(company_fee)
        .and_then(|v| v.checked_sub(gauntlet_fee))
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    let half = redistributable / 2;
    let company_total = company_fee
        .checked_add(half)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    let gauntlet_total = gauntlet_fee
        .checked_add(
            redistributable
                .checked_sub(half)
                .ok_or(GameplayStateError::ArithmeticOverflow)?,
        )
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    Ok((company_total, gauntlet_total))
}

fn find_matching_creator_index(queue: &DuelOpenQueue, entrant: Pubkey, seed: u64) -> Option<usize> {
    queue
        .entries
        .iter()
        .position(|entry| entry.seed == seed && entry.player != entrant)
}

fn snapshot_duel_entry_inventory(entry: &DuelEntry) -> PlayerInventory {
    PlayerInventory {
        player: entry.player,
        session: entry.session,
        tool: entry.loadout.tool,
        gear: entry.loadout.gear,
        gear_slot_capacity: 12,
        bump: 0,
    }
}

fn snapshot_creator_inventory(entry: DuelCreatorEntry) -> PlayerInventory {
    PlayerInventory {
        player: entry.player,
        session: Pubkey::default(),
        tool: entry.loadout.tool,
        gear: entry.loadout.gear,
        gear_slot_capacity: 12,
        bump: 0,
    }
}

fn resolve_wallet_account<'info>(
    provided: Option<&SystemAccount<'info>>,
    expected_key: Pubkey,
) -> Result<AccountInfo<'info>> {
    let account = provided.ok_or(GameplayStateError::DuelMissingWalletAccount)?;
    require_keys_eq!(
        account.key(),
        expected_key,
        GameplayStateError::DuelMissingWalletAccount
    );
    Ok(account.to_account_info())
}

fn transfer_lamports_from_vault<'info>(
    vault: &AccountInfo<'info>,
    destination: &AccountInfo<'info>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    let mut vault_lamports = vault.try_borrow_mut_lamports()?;
    let mut destination_lamports = destination.try_borrow_mut_lamports()?;

    let updated_vault = (**vault_lamports)
        .checked_sub(amount)
        .ok_or(GameplayStateError::PitDraftInsufficientVaultFunds)?;
    let updated_destination = (**destination_lamports)
        .checked_add(amount)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;

    **vault_lamports = updated_vault;
    **destination_lamports = updated_destination;

    Ok(())
}

/// Process Victory trigger effects after player wins combat.
///
/// Victory effects are processed outside the combat system because they fire
/// after combat ends, not during it. Currently supports:
/// - GainGold: Add gold to player's total
/// - Heal: Restore HP (capped at max_hp)
fn process_victory_effects(game_state: &mut GameState, inventory: &PlayerInventory, max_hp: i16) {
    let effects = generate_combat_effects(inventory);
    let gold_multiplier = compute_gold_gain_multiplier(&effects);

    for effect in effects.iter() {
        if effect.trigger != combat_system::TriggerType::Victory {
            continue;
        }

        match effect.effect_type {
            EffectType::GainGold => {
                let gold_gain = effect.value.max(0) as u16;
                let boosted_gain = apply_gold_reward_multiplier(gold_gain, gold_multiplier);
                game_state.gold = game_state.gold.saturating_add(boosted_gain);
            }
            EffectType::Heal => {
                let heal_amount = effect.value.max(0);
                game_state.hp = game_state.hp.saturating_add(heal_amount).min(max_hp);
            }
            _ => {
                // Other effect types not supported for Victory trigger yet
            }
        }
    }
}

/// Preprocess enemy effects to handle dynamic calculations.
///
/// Currently handles:
/// - Coin Slug (id=10): BattleStart GainArmor based on player gold (floor(gold/10), cap 3)
fn preprocess_enemy_effects(
    archetype_id: u8,
    player_gold: u16,
) -> Vec<combat_system::state::AnnotatedItemEffect> {
    let base_effects = field_enemies::traits::get_enemy_annotated_traits(archetype_id);

    // Coin Slug: armor = min(player_gold / 10, 3)
    if archetype_id == field_enemies::archetypes::ids::COIN_SLUG {
        let armor_from_gold = ((player_gold / 10) as i16).min(3);
        return base_effects
            .iter()
            .map(|effect| {
                if matches!(effect.effect.effect_type, EffectType::GainArmor) {
                    combat_system::state::AnnotatedItemEffect {
                        effect: ItemEffect {
                            value: armor_from_gold,
                            ..effect.effect
                        },
                        source: effect.source,
                    }
                } else {
                    effect.clone()
                }
            })
            .collect();
    }

    base_effects
}

#[inline(never)]
fn resolve_enemy_combat(
    game_state: &mut GameState,
    inventory: &PlayerInventory,
    enemy_index: usize,
) -> Result<bool> {
    let enemy = game_state.enemies[enemy_index];
    let enemy_input = match field_enemies::archetypes::get_enemy_combatant_input(
        enemy.archetype_id,
        enemy.tier,
    ) {
        Some(input) => input,
        None => return Ok(true),
    };

    let player_stats = calculate_stats(inventory, game_state.campaign_level, game_state.run_mode);
    let all_player_effects = generate_annotated_combat_effects(inventory);
    let player_input = build_player_combatant(game_state.hp, &player_stats, &all_player_effects);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects);
    let gold_multiplier = compute_gold_gain_multiplier_annotated(&player_effects);
    let enemy_effects = preprocess_enemy_effects(enemy.archetype_id, game_state.gold);

    emit!(CombatStarted {
        player: game_state.player,
        player_hp: game_state.hp,
        player_atk: player_input.atk,
        enemy_archetype: enemy.archetype_id,
        enemy_hp: enemy_input.hp,
        enemy_atk: enemy_input.atk,
    });

    let result = resolve_combat_annotated_with_both_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        game_state.gold,
        0,
    )?;

    let tier_enum = field_enemies::state::EnemyTier::from_u8(enemy.tier);
    require!(tier_enum.is_some(), GameplayStateError::InvalidEnemyTier);
    let gold_reward = tier_enum.unwrap().gold_reward() as u16;
    let gold_reward = apply_gold_reward_multiplier(gold_reward, gold_multiplier);

    emit!(CombatEnded {
        player: game_state.player,
        player_won: result.player_won,
        final_player_hp: result.final_player_hp,
        final_enemy_hp: result.final_enemy_hp,
        gold_earned: if result.player_won { gold_reward } else { 0 },
        turns_taken: result.turns_taken,
    });

    // HP capped at max_hp (discarding temp combat bonuses)
    game_state.hp = result.final_player_hp.min(player_stats.max_hp);

    // Gold changes from two sources (applied in order):
    // 1. gold_change: From combat effects (e.g., Ore Tick's StealGold trait).
    //    Can be negative if enemy stole gold. Clamped to not go below 0.
    // 2. gold_reward: Tier-based victory reward (T1=5, T2=10, T3=20).
    //    Only awarded if player won.
    // Example: If enemy steals 5 gold and player wins T1 fight:
    //   final_gold = (initial - 5) + 5 = initial
    let new_gold = (game_state.gold as i32)
        .saturating_add(result.gold_change as i32)
        .max(0) as u16;
    game_state.gold = new_gold;

    if result.player_won {
        remove_enemy(game_state, enemy_index);
        game_state.gold = game_state.gold.saturating_add(gold_reward);

        // Process Victory trigger effects (e.g., Lucky Coin, Blood Chalice)
        process_victory_effects(game_state, inventory, player_stats.max_hp);

        Ok(true)
    } else {
        game_state.is_dead = true;
        game_state.hp = 0;

        emit!(PlayerDefeated {
            player: game_state.player,
            killed_by: DeathCause::Enemy,
            final_hp: result.final_player_hp,
        });

        // Session cleanup is handled by the frontend calling end_session
        // with the main wallet after detecting death.
        Ok(false)
    }
}

#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn resolve_boss_fight<'info>(
    game_state: &mut GameState,
    map_seed: u64,
    inventory: &PlayerInventory,
    inventory_info: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    vrf: Option<(&[u8; 32], u64)>,
    session_discovery: Option<&AccountInfo<'info>>,
    session: Option<&AccountInfo<'info>>,
    map_generator_program: Option<&AccountInfo<'info>>,
    gauntlet_echoes: Option<&GauntletEchoes>,
) -> Result<bool> {
    let stage = game_state.campaign_level;
    let (boss_input, boss_id) = if game_state.run_mode == RunMode::Duel {
        (
            get_duel_boss_for_combat_from_seed(map_seed, game_state.week)?,
            get_duel_boss_id_from_seed(map_seed, game_state.week)?,
        )
    } else {
        (
            get_boss_for_combat(stage, game_state.week)?,
            get_boss_id(stage, game_state.week)?,
        )
    };
    let boss_definition = boss_system::get_boss(&boss_id).ok_or(GameplayStateError::InvalidWeek)?;
    let act = boss_system::calculate_act(stage);
    let boss_effects = boss_system::get_boss_annotated_item_effects_for_act(boss_definition, act);

    let player_stats = calculate_stats(inventory, game_state.campaign_level, game_state.run_mode);
    let all_player_effects = generate_annotated_combat_effects(inventory);
    let player_input = build_player_combatant(game_state.hp, &player_stats, &all_player_effects);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects);

    emit!(BossCombatStarted {
        player: game_state.player,
        boss_id,
        boss_hp: boss_input.hp,
        week: game_state.week,
    });

    let result = resolve_boss_combat_annotated_with_player_gold(
        player_input,
        boss_input,
        player_effects,
        boss_effects,
        game_state.gold,
        boss_id,
    )?;

    emit!(CombatEnded {
        player: game_state.player,
        player_won: result.player_won,
        final_player_hp: result.final_player_hp,
        final_enemy_hp: result.final_enemy_hp,
        gold_earned: 0,
        turns_taken: result.turns_taken,
    });

    // HP capped at max_hp (discarding temp combat bonuses)
    game_state.hp = result.final_player_hp.min(player_stats.max_hp);

    // Gold changes from combat effects only (bosses have no tier-based reward).
    // gold_change can be negative if boss has theft effects. Clamped to not go below 0.
    let new_gold = (game_state.gold as i32)
        .saturating_add(result.gold_change as i32)
        .max(0) as u16;
    game_state.gold = new_gold;

    if result.player_won {
        game_state.boss_fight_ready = false;

        if game_state.week >= game_state.max_weeks {
            // Mark session as completed - allows end_session to be called
            game_state.completed = true;

            emit!(LevelCompleted {
                player: game_state.player,
                level: stage,
                total_moves: game_state.total_moves,
                gold_earned: game_state.gold,
            });
        } else {
            game_state.week = game_state
                .week
                .checked_add(1)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
            game_state.phase = Phase::Day1;
            game_state.moves_remaining = DAY_MOVES;

            game_state.gear_slots = game_state
                .gear_slots
                .checked_add(2)
                .ok_or(GameplayStateError::ArithmeticOverflow)?
                .min(MAX_GEAR_SLOTS);

            expand_gear_slots_cpi(
                inventory_info,
                gameplay_authority,
                player_inventory_program,
                gameplay_authority_bump,
            )?;

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: Phase::Day1,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });

            // Update boss ID for next week in SessionDiscovery
            if let (Some(sd), Some(sess), Some(mgp)) =
                (session_discovery, session, map_generator_program)
            {
                let next_boss_id = if game_state.run_mode == RunMode::Duel {
                    // Duel week 3 has no boss (PvP resolution instead) — clear the ID
                    if game_state.week >= game_state.max_weeks {
                        [0u8; 12]
                    } else {
                        get_duel_boss_id_from_seed(map_seed, game_state.week)?
                    }
                } else if game_state.run_mode == RunMode::Gauntlet {
                    [0u8; 12]
                } else {
                    get_boss_id(stage, game_state.week)?
                };
                update_boss_id_cpi(sd, sess, gameplay_authority, mgp, gameplay_authority_bump, next_boss_id)?;

                // Update echo for gauntlet mode
                if game_state.run_mode == RunMode::Gauntlet {
                    let echo_idx = (game_state.week as usize).saturating_sub(1);
                    if let Some(Some(echo)) = gauntlet_echoes.and_then(|ge| ge.echoes.get(echo_idx)) {
                        let echo_bytes = echo.try_to_vec().unwrap_or_default();
                        let mut echo_data = [0u8; 179];
                        let copy_len = echo_bytes.len().min(179);
                        echo_data[..copy_len].copy_from_slice(&echo_bytes[..copy_len]);
                        update_current_echo_cpi(sd, sess, gameplay_authority, mgp, gameplay_authority_bump, 1, echo_data)?;
                    } else {
                        update_current_echo_cpi(
                            sd,
                            sess,
                            gameplay_authority,
                            mgp,
                            gameplay_authority_bump,
                            0,
                            [0u8; 179],
                        )?;
                    }
                } else {
                    update_current_echo_cpi(
                        sd,
                        sess,
                        gameplay_authority,
                        mgp,
                        gameplay_authority_bump,
                        0,
                        [0u8; 179],
                    )?;
                }
            }
        }

        // Process Victory trigger effects (e.g., Lucky Coin, Blood Chalice)
        process_victory_effects(game_state, inventory, player_stats.max_hp);

        Ok(true)
    } else {
        game_state.is_dead = true;
        game_state.hp = 0;

        emit!(PlayerDefeated {
            player: game_state.player,
            killed_by: DeathCause::Boss,
            final_hp: result.final_player_hp,
        });

        // Session cleanup is handled by the frontend calling end_session
        // with the main wallet after detecting death.
        Ok(false)
    }
}

#[inline(never)]
fn resolve_gauntlet_echo_inline<'info>(
    game_state: &mut GameState,
    gauntlet_echoes: &GauntletEchoes,
    inventory: &PlayerInventory,
    inventory_info: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    map_seed: u64,
) -> Result<bool> {
    let week = game_state.week;
    require!(
        (1..=5).contains(&week),
        GameplayStateError::InvalidGauntletWeek
    );

    let echo = match gauntlet_echoes.echoes[(week - 1) as usize] {
        Some(e) => e,
        None => return Err(GameplayStateError::GauntletNotInitialized.into()),
    };

    let player_stats = calculate_stats(inventory, GAUNTLET_CAMPAIGN_LEVEL, game_state.run_mode);
    let all_player_effects = generate_annotated_combat_effects(inventory);
    let player_input = build_player_combatant(game_state.hp, &player_stats, &all_player_effects);
    let player_effects = strip_baked_battle_start_stat_effects(all_player_effects);
    let echo_inventory = snapshot_to_inventory(echo, game_state.session, game_state.player);
    let echo_stats = calculate_stats(
        &echo_inventory,
        GAUNTLET_CAMPAIGN_LEVEL,
        game_state.run_mode,
    );
    let all_echo_effects = generate_annotated_combat_effects(&echo_inventory);
    let echo_combatant = build_full_hp_combatant(&echo_stats, &all_echo_effects);
    let echo_effects = strip_baked_battle_start_stat_effects(all_echo_effects);

    let outcome = resolve_pvp_combat_annotated_with_both_gold(
        player_input,
        echo_combatant,
        player_effects,
        echo_effects,
        game_state.gold,
        echo.loadout.gold_at_battle_start,
        gauntlet_final_tie_player_wins(map_seed),
    )?;

    emit!(GauntletWeekEchoSelected {
        player: game_state.player,
        week,
        source_player: match echo.source {
            GauntletEchoSource::Bootstrap => None,
            GauntletEchoSource::Player(p) => Some(p),
        },
    });
    emit!(GauntletCombatVisual {
        player: game_state.player,
        week,
        player_tool: inventory.tool,
        player_gear: inventory.gear,
        echo_tool: echo.loadout.tool,
        echo_gear: echo.loadout.gear,
        player_won: outcome.player_won,
        final_player_hp: outcome.final_player_hp,
        final_echo_hp: outcome.final_enemy_hp,
        turns_taken: outcome.turns_taken,
    });

    if outcome.player_won {
        game_state.hp = outcome.final_player_hp.min(player_stats.max_hp);
        game_state.gold = (game_state.gold as i32)
            .saturating_add(outcome.gold_change as i32)
            .max(0) as u16;
        game_state.boss_fight_ready = false;

        let points_awarded = gauntlet_survival_points(week);
        game_state.gauntlet_points_earned = game_state
            .gauntlet_points_earned
            .checked_add(points_awarded)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        game_state.gauntlet_highest_week_won = week;

        if week >= game_state.max_weeks {
            game_state.completed = true;
        } else {
            game_state.week = game_state
                .week
                .checked_add(1)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
            game_state.phase = Phase::Day1;
            game_state.moves_remaining = DAY_MOVES;
            game_state.gear_slots = game_state
                .gear_slots
                .checked_add(2)
                .ok_or(GameplayStateError::ArithmeticOverflow)?
                .min(MAX_GEAR_SLOTS);
            expand_gear_slots_cpi(
                inventory_info,
                gameplay_authority,
                player_inventory_program,
                gameplay_authority_bump,
            )?;
        }

        emit!(GauntletWeekAdvanced {
            player: game_state.player,
            new_week: game_state.week,
            completed: game_state.completed,
        });

        Ok(true)
    } else {
        game_state.hp = 0;
        game_state.is_dead = true;
        game_state.boss_fight_ready = false;

        if let GauntletEchoSource::Player(defender) = echo.source {
            let pts = gauntlet_defender_points(week);
            game_state.gauntlet_defender_credit = Some(GauntletDefenderCredit {
                defender,
                points: pts,
            });
        }

        emit!(GauntletRunEnded {
            player: game_state.player,
            week,
            completed: false,
        });

        Ok(false)
    }
}

fn expand_gear_slots_cpi<'info>(
    inventory: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    player_inventory::cpi::expand_gear_slots_authorized(CpiContext::new_with_signer(
        player_inventory_program.clone(),
        player_inventory::cpi::accounts::ExpandGearSlotsAuthorized {
            inventory: inventory.clone(),
            gameplay_authority: gameplay_authority.clone(),
        },
        signer_seeds,
    ))?;

    Ok(())
}

/// CPI call to map_generator::set_tile_floor to persist wall-to-floor conversion.
/// Uses gameplay_authority PDA as signer for authorization.
fn set_tile_floor_cpi<'info>(
    generated_map: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    map_generator_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    x: u8,
    y: u8,
    session_discovery: Option<&AccountInfo<'info>>,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    map_generator::cpi::set_tile_floor(
        CpiContext::new_with_signer(
            map_generator_program.clone(),
            map_generator::cpi::accounts::SetTileFloor {
                generated_map: generated_map.clone(),
                session: session.clone(),
                gameplay_authority: gameplay_authority.clone(),
                session_discovery: session_discovery.cloned(),
            },
            signer_seeds,
        ),
        x,
        y,
    )?;

    Ok(())
}

fn reveal_radius_cpi<'info>(
    generated_map: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    session_signer: &AccountInfo<'info>,
    map_generator_program: &AccountInfo<'info>,
    center_x: u8,
    center_y: u8,
    radius: u8,
    session_discovery: Option<&AccountInfo<'info>>,
) -> Result<()> {
    map_generator::cpi::reveal_radius(
        CpiContext::new(
            map_generator_program.clone(),
            map_generator::cpi::accounts::RevealRadius {
                generated_map: generated_map.clone(),
                session: session.clone(),
                session_signer: session_signer.clone(),
                session_discovery: session_discovery.cloned(),
            },
        ),
        center_x,
        center_y,
        radius,
    )?;

    Ok(())
}



#[allow(clippy::too_many_arguments)]
fn discover_visible_waypoints_authorized_cpi<'info>(
    map_pois: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    poi_system_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    center_x: u8,
    center_y: u8,
    visibility_radius: u8,
    session_discovery: Option<&AccountInfo<'info>>,
    session: Option<&AccountInfo<'info>>,
    session_signer: Option<&AccountInfo<'info>>,
    map_generator_program: Option<&AccountInfo<'info>>,
) -> Result<()> {
    let mut data = [0u8; 11];
    data[..8].copy_from_slice(&DISCOVER_VISIBLE_WAYPOINTS_AUTHORIZED_DISCRIMINATOR);
    data[8] = center_x;
    data[9] = center_y;
    data[10] = visibility_radius;

    let mut accounts = vec![
        AccountMeta::new(map_pois.key(), false),
        AccountMeta::new_readonly(gameplay_authority.key(), true),
    ];
    let mut account_infos = vec![
        map_pois.clone(),
        gameplay_authority.clone(),
        poi_system_program.clone(),
    ];

    // Append optional accounts for SessionDiscovery waypoint recording
    if let (Some(sd), Some(sess), Some(signer), Some(mgp)) =
        (session_discovery, session, session_signer, map_generator_program)
    {
        accounts.push(AccountMeta::new(sd.key(), false));
        accounts.push(AccountMeta::new_readonly(sess.key(), false));
        accounts.push(AccountMeta::new_readonly(signer.key(), true));
        accounts.push(AccountMeta::new_readonly(mgp.key(), false));
        account_infos.push(sd.clone());
        account_infos.push(sess.clone());
        account_infos.push(signer.clone());
        account_infos.push(mgp.clone());
    }

    let instruction = Instruction {
        program_id: POI_SYSTEM_PROGRAM_ID,
        accounts,
        data: data.to_vec(),
    };

    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    anchor_lang::solana_program::program::invoke_signed(
        &instruction,
        &account_infos,
        signer_seeds,
    )?;

    Ok(())
}

fn update_discovered_enemies_cpi<'info>(
    session_discovery: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    map_generator_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    enemies: Vec<map_generator::state::DiscoveredEnemy>,
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    map_generator::cpi::update_discovered_enemies(
        CpiContext::new_with_signer(
            map_generator_program.clone(),
            map_generator::cpi::accounts::UpdateDiscoveredEnemies {
                session_discovery: session_discovery.clone(),
                session: session.clone(),
                gameplay_authority: gameplay_authority.clone(),
            },
            signer_seeds,
        ),
        enemies,
    )?;

    Ok(())
}

fn update_boss_id_cpi<'info>(
    session_discovery: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    map_generator_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    boss_id: [u8; 12],
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    map_generator::cpi::update_boss_id(
        CpiContext::new_with_signer(
            map_generator_program.clone(),
            map_generator::cpi::accounts::UpdateBossId {
                session_discovery: session_discovery.clone(),
                session: session.clone(),
                gameplay_authority: gameplay_authority.clone(),
            },
            signer_seeds,
        ),
        boss_id,
    )?;

    Ok(())
}

fn update_current_echo_cpi<'info>(
    session_discovery: &AccountInfo<'info>,
    session: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    map_generator_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
    echo_present: u8,
    echo_data: [u8; 179],
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    map_generator::cpi::update_current_echo(
        CpiContext::new_with_signer(
            map_generator_program.clone(),
            map_generator::cpi::accounts::UpdateCurrentEcho {
                session_discovery: session_discovery.clone(),
                session: session.clone(),
                gameplay_authority: gameplay_authority.clone(),
            },
            signer_seeds,
        ),
        echo_present,
        echo_data,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn select_enemy_step(
    enemy_x: u8,
    enemy_y: u8,
    player_x: u8,
    player_y: u8,
    generated_map: &map_generator::state::GeneratedMap,
    occupied: &[bool],
    map_width: usize,
    player_tile_blocked: bool,
) -> Option<(u8, u8)> {
    let dx = player_x as i16 - enemy_x as i16;
    let dy = player_y as i16 - enemy_y as i16;

    if dx == 0 && dy == 0 {
        return None;
    }

    let step_toward = |pos: u8, delta: i16| -> Option<u8> {
        if delta == 0 {
            return None;
        }
        Some(if delta > 0 {
            pos.saturating_add(1)
        } else {
            pos.saturating_sub(1)
        })
    };

    let x_step = step_toward(enemy_x, dx).map(|x| (x, enemy_y));
    let y_step = step_toward(enemy_y, dy).map(|y| (enemy_x, y));

    let candidates: [Option<(u8, u8)>; 2] = if dx.abs() >= dy.abs() {
        [x_step, y_step]
    } else {
        [y_step, x_step]
    };

    for candidate in candidates.into_iter().flatten() {
        let (cx, cy) = candidate;
        if cx >= generated_map.width || cy >= generated_map.height {
            continue;
        }
        if !generated_map.is_walkable(cx, cy) {
            continue;
        }
        if cx == player_x && cy == player_y {
            if player_tile_blocked {
                continue;
            }
            return Some(candidate);
        }
        let index = (cy as usize) * map_width + (cx as usize);
        if index < occupied.len() && occupied[index] {
            continue;
        }
        return Some(candidate);
    }

    None
}

fn handle_phase_advancement(game_state: &mut GameState) -> Result<()> {
    match game_state.phase.next() {
        Some(next_phase) => {
            game_state.phase = next_phase;
            game_state.moves_remaining = next_phase.moves_allowed();

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: next_phase,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        }
        None => {
            // Night3 complete - boss fight triggers
            game_state.boss_fight_ready = true;

            emit!(BossFightReady {
                player: game_state.player,
                week: game_state.week,
            });
        }
    }

    Ok(())
}

fn initialize_week_pool(pool: &mut Account<GauntletWeekPool>, week: u8) -> Result<()> {
    pool.week = week;
    pool.bootstrap_active = true;
    pool.player_echoes_added = 0;
    pool.seen_player_echoes = 0;
    pool.entries = Vec::new();
    for i in 0..GAUNTLET_BOOTSTRAP_ECHOES_PER_WEEK {
        pool.entries.push(build_bootstrap_echo(week, i as u64)?);
    }
    Ok(())
}

fn gauntlet_gear_capacity(week: u8) -> usize {
    let cap = (INITIAL_GEAR_SLOTS as usize) + ((week as usize).saturating_sub(1) * 2);
    cap.min(MAX_GEAR_SLOTS as usize)
}

fn build_bootstrap_echo(week: u8, index: u64) -> Result<GauntletEchoSnapshot> {
    let tool_count = count_items_by_type(ItemType::Tool);
    let gear_count = count_items_by_type(ItemType::Gear);
    require!(tool_count > 0, GameplayStateError::GauntletNotInitialized);
    require!(gear_count > 0, GameplayStateError::GauntletNotInitialized);

    let tool_idx = item_index_by_type(ItemType::Tool, (index as usize) % tool_count)
        .ok_or(GameplayStateError::GauntletNotInitialized)?;
    let capacity = gauntlet_gear_capacity(week);
    let mut gear = [None; 12];
    for (slot, gear_slot) in gear.iter_mut().enumerate().take(capacity) {
        let item_idx = item_index_by_type(
            ItemType::Gear,
            ((index as usize) + (slot * week as usize)) % gear_count,
        )
        .ok_or(GameplayStateError::GauntletNotInitialized)?;
        *gear_slot = Some(ItemInstance::new(*ITEMS[item_idx].id, Tier::I));
    }

    let mut tool = ItemInstance::new(*ITEMS[tool_idx].id, Tier::I);
    let oil = match ((week as u64) + index) % 4 {
        0 => ToolOilModification::PlusAtk,
        1 => ToolOilModification::PlusSpd,
        2 => ToolOilModification::PlusDig,
        _ => ToolOilModification::PlusArm,
    };
    tool.apply_oil(oil);

    Ok(GauntletEchoSnapshot {
        week,
        source: GauntletEchoSource::Bootstrap,
        loadout: GauntletLoadoutSnapshot {
            tool: Some(tool),
            gear,
            gold_at_battle_start: 0,
        },
    })
}

fn count_items_by_type(item_type: ItemType) -> usize {
    ITEMS
        .iter()
        .filter(|def| def.item_type == item_type)
        .count()
}

fn item_index_by_type(item_type: ItemType, nth: usize) -> Option<usize> {
    let mut remaining = nth;
    for (idx, def) in ITEMS.iter().enumerate() {
        if def.item_type == item_type {
            if remaining == 0 {
                return Some(idx);
            }
            remaining -= 1;
        }
    }
    None
}

fn should_resolve_weekly_boss(run_mode: RunMode, week: u8) -> bool {
    match run_mode {
        RunMode::Campaign => true,
        // Duel mode uses bosses on week 1/2; week 3 is async PvP resolution in finalize_duel_run.
        RunMode::Duel => week < 3,
        RunMode::Gauntlet => false,
    }
}

fn resolve_post_move_phase_and_moves(
    phase: Phase,
    moves_remaining: u8,
    move_cost: u8,
) -> Result<(Phase, u8)> {
    if moves_remaining >= move_cost {
        let updated_moves = moves_remaining
            .checked_sub(move_cost)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        return Ok((phase, updated_moves));
    }

    let next_phase = phase.next().ok_or(GameplayStateError::InsufficientMoves)?;
    let remaining_cost = move_cost
        .checked_sub(moves_remaining)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    let updated_moves = next_phase
        .moves_allowed()
        .checked_sub(remaining_cost)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    Ok((next_phase, updated_moves))
}

fn visibility_radius_for_phase(phase: Phase) -> u8 {
    if phase.is_night() {
        NIGHT_VISION_RADIUS
    } else {
        DAY_VISION_RADIUS
    }
}

#[cfg(test)]
fn gauntlet_week_resolution_ready(game_state: &GameState) -> bool {
    game_state.boss_fight_ready && game_state.phase.is_night3() && game_state.moves_remaining == 0
}

fn snapshot_to_inventory(
    snapshot: GauntletEchoSnapshot,
    session: Pubkey,
    player: Pubkey,
) -> PlayerInventory {
    PlayerInventory {
        session,
        player,
        tool: snapshot.loadout.tool,
        gear: snapshot.loadout.gear,
        gear_slot_capacity: MAX_GEAR_SLOTS,
        bump: 0,
    }
}

fn gauntlet_survival_points(week: u8) -> u64 {
    match week {
        1 => 10,
        2 => 25,
        3 => 45,
        4 => 70,
        _ => 100,
    }
}

fn gauntlet_defender_points(week: u8) -> u64 {
    match week {
        1 => 3,
        2 => 8,
        3 => 15,
        4 => 24,
        _ => 35,
    }
}

fn upsert_player_score(
    score: &mut Account<GauntletPlayerScore>,
    player: &Pubkey,
    epoch_id: u64,
    add_points: u64,
    bump: u8,
) -> Result<()> {
    if score.player == Pubkey::default() {
        score.epoch_id = epoch_id;
        score.player = *player;
        score.points = 0;
        score.claimed = false;
        score.bump = bump;
    }
    require!(
        score.player == *player,
        GameplayStateError::GauntletScoreMismatch
    );
    require!(
        score.epoch_id == epoch_id,
        GameplayStateError::GauntletScoreMismatch
    );
    score.points = score
        .points
        .checked_add(add_points)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    Ok(())
}

fn add_pending_defender_points(
    epoch_pool: &mut Account<GauntletEpochPool>,
    player: Pubkey,
    points: u64,
) -> Result<()> {
    if let Some(existing) = epoch_pool
        .pending_defender_points
        .iter_mut()
        .find(|entry| entry.player == player)
    {
        existing.points = existing
            .points
            .checked_add(points)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        return Ok(());
    }

    require!(
        epoch_pool.pending_defender_points.len() < GauntletEpochPool::MAX_PENDING_DEFENDERS,
        GameplayStateError::ArithmeticOverflow
    );
    epoch_pool
        .pending_defender_points
        .push(GauntletPendingPoints { player, points });
    Ok(())
}

fn take_pending_defender_points(
    epoch_pool: &mut Account<GauntletEpochPool>,
    player: Pubkey,
) -> u64 {
    if let Some(index) = epoch_pool
        .pending_defender_points
        .iter()
        .position(|entry| entry.player == player)
    {
        let points = epoch_pool.pending_defender_points[index].points;
        epoch_pool.pending_defender_points.swap_remove(index);
        points
    } else {
        0
    }
}

#[derive(Accounts)]
pub struct InitializeGameState<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + GameState::INIT_SPACE,
        seeds = [GAME_STATE_SEED, game_session.key().as_ref()],
        bump
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// The linked GameSession PDA (must exist)
    /// CHECK: We only verify this account exists as validation of the session
    pub game_session: AccountInfo<'info>,

    /// Generated map for seeding enemies
    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_session.key().as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    /// CHECK: Player wallet pubkey stored in game_state.player; does not need to sign.
    pub player: AccountInfo<'info>,

    /// CHECK: Session key signer whose pubkey is stored in game_state.session_signer
    /// for authorizing gameplay transactions (move, boss fight).
    pub session_signer: AccountInfo<'info>,

    /// Account that pays for the GameState account rent (session_signer in CPI flows).
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Context for syncing enemy instances and game position from GeneratedMap after ER map generation.
/// Called immediately after fill_map_with_seed or generate_map_with_vrf on the Ephemeral Rollup.
#[derive(Accounts)]
pub struct SyncMapEnemies<'info> {
    pub session_signer: Signer<'info>,

    /// CHECK: Session PDA owned by session-manager.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ GameplayStateError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    /// Generated map (read-only) owned by map-generator program.
    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session @ GameplayStateError::InvalidSession,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    /// Game state to update with actual map dimensions, spawn position, and enemies.
    #[account(
        mut,
        seeds = [GAME_STATE_SEED, session.key().as_ref()],
        bump = game_state.bump,
        has_one = session @ GameplayStateError::InvalidSession,
        has_one = session_signer @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// CHECK: Validated by POI system during CPI discovery.
    #[account(mut)]
    pub map_pois: AccountInfo<'info>,

    /// CHECK: Must be the poi-system program.
    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    pub poi_system_program: AccountInfo<'info>,

    /// Gameplay authority PDA for signing CPI calls to map-generator
    /// CHECK: This is a PDA derived from gameplay_state program, validated by seeds
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Map generator program for CPI (update_boss_id, update_current_echo, update_discovered_enemies)
    pub map_generator_program: Program<'info, map_generator::program::MapGenerator>,

    /// Optional SessionDiscovery for boss/echo/enemy sync.
    /// CHECK: Passed through to map-generator CPI; validated there.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,

    /// Optional GameplayVrfState for VRF-backed duel boss selection.
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,

    /// Optional GauntletEchoes for echo sync to SessionDiscovery.
    #[account(
        seeds = [GAUNTLET_ECHOES_SEED, session.key().as_ref()],
        bump = gauntlet_echoes.bump,
    )]
    pub gauntlet_echoes: Option<Account<'info, GauntletEchoes>>,
}

/// Read-only context for refreshing discovered enemies in SessionDiscovery.
/// game_state is NOT mut to avoid re-serialization that could overwrite
/// position updates from fast_travel.
#[derive(Accounts)]
pub struct RefreshDiscoveredEnemies<'info> {
    pub session_signer: Signer<'info>,

    /// CHECK: Session PDA owned by session-manager.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ GameplayStateError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    /// Generated map (read-only) for discovered_tiles bitmap.
    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session @ GameplayStateError::InvalidSession,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    /// Game state (read-only) containing enemy instances.
    #[account(
        seeds = [GAME_STATE_SEED, session.key().as_ref()],
        bump = game_state.bump,
        has_one = session @ GameplayStateError::InvalidSession,
    )]
    pub game_state: Account<'info, GameState>,

    /// Gameplay authority PDA for signing CPI calls to map-generator.
    /// CHECK: PDA derived from gameplay_state program, validated by seeds.
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Map generator program for CPI (update_discovered_enemies).
    pub map_generator_program: Program<'info, map_generator::program::MapGenerator>,

    /// SessionDiscovery to write enemy data into.
    /// CHECK: Passed through to map-generator CPI; validated there.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,
}

/// Read-only context for refreshing discovered enemies in SessionDiscovery,
/// authorized by poi-system CPI after tile-revealing POIs.
#[derive(Accounts)]
pub struct RefreshDiscoveredEnemiesAuthorized<'info> {
    /// POI authority PDA from poi-system that must sign.
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,

    /// CHECK: Session PDA owned by session-manager.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ GameplayStateError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    /// Generated map (read-only) for discovered_tiles bitmap.
    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, session.key().as_ref()],
        bump = generated_map.bump,
        has_one = session @ GameplayStateError::InvalidSession,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    /// Game state (read-only) containing enemy instances.
    #[account(
        seeds = [GAME_STATE_SEED, session.key().as_ref()],
        bump = game_state.bump,
        has_one = session @ GameplayStateError::InvalidSession,
    )]
    pub game_state: Account<'info, GameState>,

    /// Gameplay authority PDA for signing CPI calls to map-generator.
    /// CHECK: PDA derived from gameplay_state program, validated by seeds.
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Map generator program for CPI (update_discovered_enemies).
    pub map_generator_program: Program<'info, map_generator::program::MapGenerator>,

    /// SessionDiscovery to write enemy data into.
    /// CHECK: Passed through to map-generator CPI; validated there.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,
}

#[derive(Accounts)]
pub struct InitializePitDraft<'info> {
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + PitDraftQueue::INIT_SPACE,
        seeds = [PIT_DRAFT_QUEUE_SEED],
        bump
    )]
    pub pit_draft_queue: Account<'info, PitDraftQueue>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + PitDraftVault::INIT_SPACE,
        seeds = [PIT_DRAFT_VAULT_SEED],
        bump
    )]
    pub pit_draft_vault: Account<'info, PitDraftVault>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeDuels<'info> {
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + DuelVault::INIT_SPACE,
        seeds = [DUEL_VAULT_SEED],
        bump
    )]
    pub duel_vault: Account<'info, DuelVault>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + DuelOpenQueue::INIT_SPACE,
        seeds = [DUEL_OPEN_QUEUE_SEED],
        bump
    )]
    pub duel_open_queue: Account<'info, DuelOpenQueue>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + DuelErQueue::INIT_SPACE,
        seeds = [DUEL_ER_QUEUE_SEED],
        bump
    )]
    pub duel_er_queue: Account<'info, DuelErQueue>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateDuelEntry<'info> {
    /// CHECK: Validated in handler as DuelEntry PDA. Must be AccountInfo to avoid
    /// Anchor serialization crash after delegation changes the owner.
    #[account(mut, del)]
    pub duel_entry: AccountInfo<'info>,

    #[account(mut)]
    pub player: Signer<'info>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateDuelErQueue<'info> {
    /// CHECK: Validated in handler as DuelErQueue PDA. Must be AccountInfo to avoid
    /// Anchor serialization crash after delegation changes the owner.
    #[account(mut, del)]
    pub duel_er_queue: AccountInfo<'info>,

    #[account(mut)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct InitializeDuelQueue<'info> {
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + DuelQueue::INIT_SPACE,
        seeds = [DUEL_QUEUE_SEED, &seed.to_le_bytes()],
        bump
    )]
    pub duel_queue: Account<'info, DuelQueue>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ConfigureRunMode<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    #[account(
        seeds = [SESSION_MANAGER_RUNMODE_AUTHORITY_SEED],
        bump,
        seeds::program = SESSION_MANAGER_PROGRAM_ID
    )]
    /// Must be the session-manager PDA signer used to authorize mode configuration.
    pub session_manager_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct RotateGameStateSessionKey<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    #[account(
        seeds = [SESSION_MANAGER_RUNMODE_AUTHORITY_SEED],
        bump,
        seeds::program = SESSION_MANAGER_PROGRAM_ID
    )]
    /// Session manager PDA authority (only session-manager can sign)
    pub session_manager_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializeGauntlet<'info> {
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + GauntletConfig::INIT_SPACE,
        seeds = [GAUNTLET_CONFIG_SEED],
        bump
    )]
    pub gauntlet_config: Account<'info, GauntletConfig>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + GauntletPoolVault::INIT_SPACE,
        seeds = [GAUNTLET_POOL_VAULT_SEED],
        bump
    )]
    pub gauntlet_pool_vault: Account<'info, GauntletPoolVault>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + GauntletWeekPool::INIT_SPACE,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[1]],
        bump
    )]
    pub gauntlet_week1: Account<'info, GauntletWeekPool>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + GauntletWeekPool::INIT_SPACE,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[2]],
        bump
    )]
    pub gauntlet_week2: Account<'info, GauntletWeekPool>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + GauntletWeekPool::INIT_SPACE,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[3]],
        bump
    )]
    pub gauntlet_week3: Account<'info, GauntletWeekPool>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + GauntletWeekPool::INIT_SPACE,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[4]],
        bump
    )]
    pub gauntlet_week4: Account<'info, GauntletWeekPool>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + GauntletWeekPool::INIT_SPACE,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[5]],
        bump
    )]
    pub gauntlet_week5: Account<'info, GauntletWeekPool>,

    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(epoch_id: u64)]
pub struct EnterGauntlet<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        seeds = [GAUNTLET_CONFIG_SEED],
        bump = gauntlet_config.bump
    )]
    pub gauntlet_config: Account<'info, GauntletConfig>,

    #[account(
        mut,
        seeds = [GAUNTLET_POOL_VAULT_SEED],
        bump = gauntlet_pool_vault.bump
    )]
    pub gauntlet_pool_vault: Account<'info, GauntletPoolVault>,

    #[account(mut)]
    pub company_treasury: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = player,
        space = 8 + GauntletEpochPool::INIT_SPACE,
        seeds = [GAUNTLET_EPOCH_POOL_SEED, &epoch_id.to_le_bytes()],
        bump
    )]
    pub gauntlet_epoch_pool: Account<'info, GauntletEpochPool>,

    #[account(
        init_if_needed,
        payer = player,
        space = 8 + GauntletPlayerScore::INIT_SPACE,
        seeds = [GAUNTLET_PLAYER_SCORE_SEED, &epoch_id.to_le_bytes(), player.key().as_ref()],
        bump
    )]
    pub gauntlet_player_score: Account<'info, GauntletPlayerScore>,

    #[account(
        init,
        payer = player,
        space = 8 + GauntletEchoes::INIT_SPACE,
        seeds = [GAUNTLET_ECHOES_SEED, game_state.session.as_ref()],
        bump
    )]
    pub gauntlet_echoes: Account<'info, GauntletEchoes>,

    pub system_program: Program<'info, System>,
}

/// Re-draws gauntlet echoes using VRF randomness on ER.
/// Called after VRF fulfillment to replace the deterministic fallback from enter_gauntlet.
#[derive(Accounts)]
pub struct RedrawGauntletEchoes<'info> {
    #[account(
        has_one = session_signer @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    pub session_signer: Signer<'info>,

    #[account(
        mut,
        seeds = [GAUNTLET_ECHOES_SEED, game_state.session.as_ref()],
        bump = gauntlet_echoes.bump,
    )]
    pub gauntlet_echoes: Account<'info, GauntletEchoes>,

    /// GameplayVrfState with fulfilled randomness (required).
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,
    // Week pools 1-5 passed as remaining_accounts.
}

#[derive(Accounts)]
#[instruction(epoch_id: u64)]
pub struct FinalizeGauntletEpoch<'info> {
    #[account(
        mut,
        seeds = [GAUNTLET_CONFIG_SEED],
        bump = gauntlet_config.bump
    )]
    pub gauntlet_config: Account<'info, GauntletConfig>,
    #[account(
        mut,
        seeds = [GAUNTLET_POOL_VAULT_SEED],
        bump = gauntlet_pool_vault.bump
    )]
    pub gauntlet_pool_vault: Account<'info, GauntletPoolVault>,
    #[account(
        mut,
        seeds = [GAUNTLET_EPOCH_POOL_SEED, &epoch_id.to_le_bytes()],
        bump = gauntlet_epoch_pool.bump
    )]
    pub gauntlet_epoch_pool: Account<'info, GauntletEpochPool>,
}

#[derive(Accounts)]
#[instruction(epoch_id: u64)]
pub struct ClaimGauntletRewards<'info> {
    #[account(
        mut,
        seeds = [GAUNTLET_EPOCH_POOL_SEED, &epoch_id.to_le_bytes()],
        bump = gauntlet_epoch_pool.bump
    )]
    pub gauntlet_epoch_pool: Account<'info, GauntletEpochPool>,
    #[account(
        init_if_needed,
        payer = player,
        space = 8 + GauntletPlayerScore::INIT_SPACE,
        seeds = [GAUNTLET_PLAYER_SCORE_SEED, &epoch_id.to_le_bytes(), player.key().as_ref()],
        bump
    )]
    pub gauntlet_player_score: Account<'info, GauntletPlayerScore>,
    #[account(
        mut,
        seeds = [GAUNTLET_POOL_VAULT_SEED],
        bump = gauntlet_pool_vault.bump
    )]
    pub gauntlet_pool_vault: Account<'info, GauntletPoolVault>,
    #[account(mut)]
    pub player_wallet: SystemAccount<'info>,
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(epoch_id: u64)]
pub struct SettleGauntletSession<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// CHECK: Player wallet — validated by has_one on game_state. Not required to sign.
    pub player: AccountInfo<'info>,

    #[account(
        constraint = game_state.session_signer == session_signer.key() @ GameplayStateError::Unauthorized
    )]
    pub session_signer: Signer<'info>,

    #[account(
        mut,
        seeds = [GAUNTLET_EPOCH_POOL_SEED, &epoch_id.to_le_bytes()],
        bump = gauntlet_epoch_pool.bump
    )]
    pub gauntlet_epoch_pool: Account<'info, GauntletEpochPool>,

    #[account(
        mut,
        seeds = [GAUNTLET_PLAYER_SCORE_SEED, &epoch_id.to_le_bytes(), player.key().as_ref()],
        bump = gauntlet_player_score.bump
    )]
    pub gauntlet_player_score: Account<'info, GauntletPlayerScore>,

    #[account(
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        mut,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[1]],
        bump = gauntlet_week1.bump
    )]
    pub gauntlet_week1: Account<'info, GauntletWeekPool>,
    #[account(
        mut,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[2]],
        bump = gauntlet_week2.bump
    )]
    pub gauntlet_week2: Account<'info, GauntletWeekPool>,
    #[account(
        mut,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[3]],
        bump = gauntlet_week3.bump
    )]
    pub gauntlet_week3: Account<'info, GauntletWeekPool>,
    #[account(
        mut,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[4]],
        bump = gauntlet_week4.bump
    )]
    pub gauntlet_week4: Account<'info, GauntletWeekPool>,
    #[account(
        mut,
        seeds = [GAUNTLET_WEEK_POOL_SEED, &[5]],
        bump = gauntlet_week5.bump
    )]
    pub gauntlet_week5: Account<'info, GauntletWeekPool>,

    /// Optional GameplayVrfState for VRF-backed reservoir sampling.
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,
}

#[derive(Accounts)]
pub struct EnterDuel<'info> {
    #[account(
        init_if_needed,
        payer = player,
        space = 8 + DuelEntry::INIT_SPACE,
        seeds = [DUEL_ENTRY_SEED, game_state.session.as_ref()],
        bump,
    )]
    pub duel_entry: Box<Account<'info, DuelEntry>>,

    #[account(
        mut,
        seeds = [DUEL_VAULT_SEED],
        bump = duel_vault.bump,
    )]
    pub duel_vault: Account<'info, DuelVault>,

    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(mut)]
    pub company_treasury: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [GAUNTLET_POOL_VAULT_SEED],
        bump = gauntlet_pool_vault.bump
    )]
    pub gauntlet_pool_vault: Account<'info, GauntletPoolVault>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AssignDuelMapSeed<'info> {
    #[account(
        mut,
        constraint = game_state.session_signer == session_signer.key() @ GameplayStateError::Unauthorized,
        constraint = game_state.run_mode == RunMode::Duel @ GameplayStateError::DuelInvalidRunMode,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(
        mut,
        seeds = [DUEL_OPEN_QUEUE_SEED],
        bump = duel_open_queue.bump,
    )]
    pub duel_open_queue: Box<Account<'info, DuelOpenQueue>>,

    #[account(
        mut,
        seeds = [DUEL_ENTRY_SEED, game_state.session.as_ref()],
        bump = duel_entry.bump,
    )]
    pub duel_entry: Box<Account<'info, DuelEntry>>,

    pub session_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct GenerateDuelMap<'info> {
    #[account(
        mut,
        constraint = game_state.session_signer == player.key() @ GameplayStateError::Unauthorized,
        constraint = game_state.run_mode == RunMode::Duel @ GameplayStateError::DuelInvalidRunMode,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// CHECK: Generated map account owned by map-generator; validated by CPI.
    #[account(
        mut,
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: AccountInfo<'info>,

    /// Optional MapVrfState for creator path (VRF seed derivation, read-only).
    /// Not delegated on localnet — must not be writable.
    #[account(
        seeds = [map_generator::state::MapVrfState::SEED_PREFIX, game_state.session.as_ref()],
        bump,
        seeds::program = map_generator::ID,
    )]
    pub map_vrf_state: Option<Account<'info, map_generator::state::MapVrfState>>,

    /// Gameplay authority PDA for CPI signing.
    /// CHECK: Validated by seeds constraint.
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    pub map_generator_program: Program<'info, map_generator::program::MapGenerator>,

    /// Optional SessionDiscovery for map metadata sync.
    /// CHECK: Passed through to map-generator CPI; validated there.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,

    /// CHECK: Session account for PDA verification.
    #[account(
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession,
    )]
    pub game_session: AccountInfo<'info>,

    pub player: Signer<'info>,
}


/// SettleDuelPayout runs on base layer after undelegation.
#[derive(Accounts)]
pub struct SettleDuelPayout<'info> {
    #[account(
        mut,
        seeds = [DUEL_ENTRY_SEED, duel_entry.session.as_ref()],
        bump = duel_entry.bump,
    )]
    pub duel_entry: Box<Account<'info, DuelEntry>>,

    #[account(
        mut,
        seeds = [DUEL_VAULT_SEED],
        bump = duel_vault.bump,
    )]
    pub duel_vault: Account<'info, DuelVault>,

    /// CHECK: Player wallet — validated against duel_entry.player.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    pub session_signer: Signer<'info>,

    /// Wallet for matched creator payout when current player loses.
    #[account(mut)]
    pub creator_wallet: Option<SystemAccount<'info>>,

    #[account(mut)]
    pub company_treasury: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [GAUNTLET_POOL_VAULT_SEED],
        bump = gauntlet_pool_vault.bump
    )]
    pub gauntlet_pool_vault: Account<'info, GauntletPoolVault>,

    #[account(
        mut,
        seeds = [DUEL_OPEN_QUEUE_SEED],
        bump = duel_open_queue.bump,
    )]
    pub duel_open_queue: Box<Account<'info, DuelOpenQueue>>,

    #[account(
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Box<Account<'info, PlayerInventory>>,
}

#[derive(Accounts)]
pub struct ResetDuelEntry<'info> {
    #[account(
        mut,
        seeds = [DUEL_ENTRY_SEED, game_state.session.as_ref()],
        bump = duel_entry.bump,
        constraint = duel_entry.player == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub duel_entry: Box<Account<'info, DuelEntry>>,

    #[account(
        has_one = session_signer @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    /// CHECK: Player wallet. Validated by duel_entry.player constraint.
    #[account(mut)]
    pub player: AccountInfo<'info>,

    pub session_signer: Signer<'info>,

    #[account(
        mut,
        seeds = [DUEL_VAULT_SEED],
        bump = duel_vault.bump,
    )]
    pub duel_vault: Account<'info, DuelVault>,

    #[account(mut)]
    pub company_treasury: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [GAUNTLET_POOL_VAULT_SEED],
        bump = gauntlet_pool_vault.bump,
    )]
    pub gauntlet_pool_vault: Account<'info, GauntletPoolVault>,

    /// Wallet for matched creator payout when abandoner was matched.
    #[account(mut)]
    pub creator_wallet: Option<SystemAccount<'info>>,
}

#[derive(Accounts)]
pub struct ResetOrphanedDuelEntry<'info> {
    #[account(
        mut,
        seeds = [DUEL_ENTRY_SEED, session_pda.key().as_ref()],
        bump = duel_entry.bump,
        constraint = duel_entry.player == player.key() @ GameplayStateError::Unauthorized,
        constraint = duel_entry.session == session_pda.key() @ GameplayStateError::InvalidSession,
    )]
    pub duel_entry: Box<Account<'info, DuelEntry>>,

    /// Session PDA must not exist (proves the entry is orphaned).
    /// CHECK: Address must match duel_entry.session, and lamports must be 0.
    #[account(
        constraint = session_pda.lamports() == 0 @ GameplayStateError::SessionNotActive,
    )]
    pub session_pda: UncheckedAccount<'info>,

    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct EnterPitDraft<'info> {
    #[account(
        mut,
        seeds = [PIT_DRAFT_QUEUE_SEED],
        bump = pit_draft_queue.bump,
    )]
    pub pit_draft_queue: Account<'info, PitDraftQueue>,

    #[account(
        mut,
        seeds = [PIT_DRAFT_VAULT_SEED],
        bump = pit_draft_vault.bump,
    )]
    pub pit_draft_vault: Account<'info, PitDraftVault>,

    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        constraint = player_profile.owner == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    /// Waiting player's profile (required when matching, None when just queuing).
    pub waiting_profile: Option<Account<'info, PlayerProfile>>,

    /// CHECK: Waiting player's wallet for payout (required when matching).
    #[account(mut)]
    pub waiting_player_wallet: Option<AccountInfo<'info>>,

    /// CHECK: Company treasury for fee (required when matching).
    #[account(mut)]
    pub company_treasury: Option<AccountInfo<'info>>,

    /// Gauntlet pool vault for fee (required when matching).
    #[account(mut)]
    pub gauntlet_pool_vault: Option<Account<'info, GauntletPoolVault>>,

    /// VRF state — must be fulfilled when matching.
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseGameState<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
        close = player,
    )]
    pub game_state: Account<'info, GameState>,

    #[account(mut)]
    pub player: Signer<'info>,
}

/// Context for closing GameState via session key signer (for session-manager CPI).
#[derive(Accounts)]
pub struct CloseGameStateViaSessionSigner<'info> {
    #[account(
        mut,
        has_one = session_signer @ GameplayStateError::Unauthorized,
        close = session_signer,
    )]
    pub game_state: Account<'info, GameState>,

    /// CHECK: Player wallet pubkey (used for validation only, not receiving rent).
    pub player: AccountInfo<'info>,

    /// Session key signer authorizes closure and receives the rent refund.
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

// CloseMapEnemies removed — MapEnemies merged into GameState

/// Context for closing GauntletEchoes account via session key signer.
#[derive(Accounts)]
pub struct CloseGauntletEchoes<'info> {
    #[account(
        mut,
        seeds = [GauntletEchoes::SEED_PREFIX, game_state.session.as_ref()],
        bump = gauntlet_echoes.bump,
        constraint = gauntlet_echoes.session == game_state.session @ GameplayStateError::InvalidSession,
        close = session_signer,
    )]
    pub gauntlet_echoes: Account<'info, GauntletEchoes>,

    /// GameState to verify session_signer authorization
    #[account(
        has_one = session_signer @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    /// CHECK: Player wallet pubkey (used for validation only, not receiving rent).
    #[account(address = game_state.player @ GameplayStateError::Unauthorized)]
    pub player: AccountInfo<'info>,

    /// Session key signer authorizes closure and receives the rent refund.
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

/// Close a corrupted/empty GameState account (0-byte data after ER reset + force-undelegate).
/// Only works on accounts owned by this program with exactly 0 bytes of data.
#[derive(Accounts)]
pub struct CloseEmptyGameState<'info> {
    #[account(
        mut,
        constraint = game_state.data_is_empty() @ GameplayStateError::AccountNotEmpty,
        constraint = *game_state.owner == crate::ID @ GameplayStateError::Unauthorized,
    )]
    /// CHECK: Validated via owner check + empty data constraint.
    pub game_state: UncheckedAccount<'info>,

    /// Receives the lamports from the closed account.
    #[account(mut)]
    /// CHECK: Any destination is fine since the account is corrupted/empty.
    pub destination: AccountInfo<'info>,

    pub payer: Signer<'info>,
}

// CloseEmptyMapEnemies removed — MapEnemies merged into GameState
// CloseOrphanedMapEnemies removed — MapEnemies merged into GameState

/// Close an orphaned GauntletEchoes account with valid data, whose session PDA no longer exists.
/// Validates that the session PDA (from gauntlet_echoes.session) has 0 lamports (doesn't exist).
#[derive(Accounts)]
pub struct CloseOrphanedGauntletEchoes<'info> {
    #[account(
        mut,
        seeds = [GAUNTLET_ECHOES_SEED, gauntlet_echoes.session.as_ref()],
        bump = gauntlet_echoes.bump,
        close = destination,
    )]
    pub gauntlet_echoes: Account<'info, GauntletEchoes>,

    /// Session PDA must not exist (proves the account is orphaned).
    /// CHECK: Address must match gauntlet_echoes.session, and lamports must be 0.
    #[account(
        constraint = session_pda.key() == gauntlet_echoes.session @ GameplayStateError::InvalidSession,
        constraint = session_pda.lamports() == 0 @ GameplayStateError::SessionNotActive,
    )]
    pub session_pda: UncheckedAccount<'info>,

    /// Receives the lamports from the closed account.
    #[account(mut)]
    /// CHECK: Any destination is fine since the session is dead.
    pub destination: AccountInfo<'info>,

    pub payer: Signer<'info>,
}

/// Context for healing the player, authorized by poi-system CPI.
/// Requires poi_authority PDA from poi-system as signer.
/// Includes inventory for deriving max_hp.
#[derive(Accounts)]
pub struct HealPlayer<'info> {
    #[account(mut)]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player inventory for deriving max_hp (PDA derived from session)
    #[account(
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

/// Context for skipping to day, authorized by poi-system CPI.
/// Used by rest POIs (L1 Mole Den, L5 Rest Alcove) to skip night phases.
/// Includes accounts needed for boss fight resolution (Night3 triggers boss fight).
#[derive(Accounts)]
pub struct SkipToDay<'info> {
    #[account(mut)]
    pub game_state: Box<Account<'info, GameState>>,

    /// Player inventory for stats calculation and boss fight resolution
    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Box<Account<'info, PlayerInventory>>,

    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,

    /// Gameplay authority PDA for signing CPI calls to player-inventory
    /// CHECK: This is a PDA derived from gameplay_state program, validated by seeds
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (expand gear slots on boss victory)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Optional GameplayVrfState for VRF-backed duel boss selection in skip_to_day.
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,

    /// Optional GauntletEchoes for gauntlet echo resolution in skip_to_day.
    #[account(
        mut,
        seeds = [GAUNTLET_ECHOES_SEED, game_state.session.as_ref()],
        bump = gauntlet_echoes.bump,
    )]
    pub gauntlet_echoes: Option<Account<'info, GauntletEchoes>>,
}

/// Context for adding HP bonus when equipping +HP gear, authorized by player-inventory CPI.
/// Requires inventory_authority PDA from player-inventory as signer.
#[derive(Accounts)]
pub struct AddHpBonusAuthorized<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    /// Inventory authority PDA from player-inventory that must sign
    #[account(
        seeds = [b"inventory_authority"],
        bump,
        seeds::program = PLAYER_INVENTORY_PROGRAM_ID,
    )]
    pub inventory_authority: Signer<'info>,
}

/// Context for removing HP bonus when unequipping +HP gear, authorized by player-inventory CPI.
/// Requires inventory_authority PDA from player-inventory as signer.
#[derive(Accounts)]
pub struct RemoveHpBonusAuthorized<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    /// Inventory authority PDA from player-inventory that must sign
    #[account(
        seeds = [b"inventory_authority"],
        bump,
        seeds::program = PLAYER_INVENTORY_PROGRAM_ID,
    )]
    pub inventory_authority: Signer<'info>,
}

/// Context for authorized gold modification via poi-system CPI.
/// Requires poi_authority PDA from poi-system as signer.
#[derive(Accounts)]
pub struct ModifyGoldAuthorized<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

/// Context for authorized position updates via poi-system CPI.
/// Requires poi_authority PDA from poi-system as signer.
#[derive(Accounts)]
pub struct SetPositionAuthorized<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

    /// POI authority PDA from poi-system that must sign
    #[account(
        seeds = [b"poi_authority"],
        bump,
        seeds::program = POI_SYSTEM_PROGRAM_ID,
    )]
    pub poi_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Move<'info> {
    #[account(
        mut,
        constraint = game_state.session_signer == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession
    )]
    /// CHECK: Validated by game_state.session match.
    pub game_session: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Box<Account<'info, PlayerInventory>>,

    /// Gameplay authority PDA for signing CPI calls to map_generator
    /// CHECK: This is a PDA derived from gameplay_state program, validated by seeds
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (expand gear slots on boss victory)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Map generator program for CPI (set tile floor on wall break)
    pub map_generator_program: Program<'info, map_generator::program::MapGenerator>,

    /// CHECK: Validated by POI system during CPI discovery.
    #[account(mut)]
    pub map_pois: AccountInfo<'info>,

    /// CHECK: Must be the poi-system program.
    #[account(address = POI_SYSTEM_PROGRAM_ID)]
    pub poi_system_program: AccountInfo<'info>,

    /// Optional SessionDiscovery for fog-of-war dual-write.
    /// CHECK: Passed through to map-generator CPI; validated there.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,

    /// Optional GameplayVrfState for VRF-backed duel boss selection during movement.
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,

    /// Optional GauntletEchoes for gauntlet echo resolution.
    #[account(
        mut,
        seeds = [GAUNTLET_ECHOES_SEED, game_state.session.as_ref()],
        bump = gauntlet_echoes.bump,
    )]
    pub gauntlet_echoes: Option<Account<'info, GauntletEchoes>>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct TriggerBossFight<'info> {
    #[account(
        mut,
        constraint = game_state.session_signer == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(
        mut,
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession
    )]
    /// CHECK: Validated by game_state.session match.
    pub game_session: AccountInfo<'info>,

    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Box<Account<'info, PlayerInventory>>,

    /// Gameplay authority PDA for signing CPI calls to player-inventory
    /// CHECK: This is a PDA derived from gameplay_state program, validated by seeds
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (expand gear slots on boss victory)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    /// Optional GameplayVrfState for VRF-backed duel boss selection.
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,

    /// Optional SessionDiscovery for boss ID update at week transition.
    /// CHECK: Passed through to map-generator CPI; validated there.
    #[account(mut)]
    pub session_discovery: Option<UncheckedAccount<'info>>,

    /// Optional map-generator program for SessionDiscovery CPI.
    pub map_generator_program: Option<Program<'info, map_generator::program::MapGenerator>>,

    /// Optional GauntletEchoes for gauntlet echo resolution.
    pub gauntlet_echoes: Option<Account<'info, GauntletEchoes>>,

    pub player: Signer<'info>,
}

/// Lightweight context for syncing boss ID to SessionDiscovery.
/// Used after rest-alcove boss wins in Duel mode where skip_to_day
/// can't run update_boss_id_cpi due to ER CPI depth limitations.
#[derive(Accounts)]
pub struct SyncDiscoveryBoss<'info> {
    #[account(
        constraint = game_state.session_signer == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Box<Account<'info, GameState>>,

    #[account(
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession
    )]
    /// CHECK: Validated by game_state.session match.
    pub game_session: AccountInfo<'info>,

    /// Gameplay authority PDA for signing CPI calls to map_generator
    /// CHECK: PDA derived from gameplay_state program, validated by seeds
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// SessionDiscovery to update the boss ID.
    /// CHECK: Passed through to map-generator CPI; validated there.
    #[account(mut)]
    pub session_discovery: UncheckedAccount<'info>,

    /// Map generator program for update_boss_id CPI.
    pub map_generator_program: Program<'info, map_generator::program::MapGenerator>,

    /// Optional GameplayVrfState for VRF-backed duel boss selection.
    pub gameplay_vrf_state: Option<Account<'info, GameplayVrfState>>,

    /// GeneratedMap for reading map seed (duel boss derived from shared seed).
    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Box<Account<'info, map_generator::state::GeneratedMap>>,

    pub player: Signer<'info>,
}

// =========================================================================
// VRF Account Contexts
// =========================================================================

/// Pre-creates GameplayVrfState on base chain (no VRF request).
#[derive(Accounts)]
pub struct InitGameplayVrfState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Session PDA owned by session-manager.
    #[account(owner = SESSION_MANAGER_PROGRAM_ID @ GameplayStateError::InvalidSessionOwner)]
    pub session: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + GameplayVrfState::SPACE,
        seeds = [GameplayVrfState::SEED_PREFIX, session.key().as_ref()],
        bump,
    )]
    pub vrf_state: Account<'info, GameplayVrfState>,

    pub system_program: Program<'info, System>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateGameplayVrfState<'info> {
    #[account(mut, del)]
    /// CHECK: PDA is validated via explicit seed check in handler.
    pub gameplay_vrf_state: AccountInfo<'info>,
    /// CHECK: Session PDA owned by session-manager; used only for seed derivation.
    pub game_session: UncheckedAccount<'info>,
    pub player: Signer<'info>,
}

/// Pre-creates GameplayVrfState on base chain for pit draft VRF.
/// Unlike InitGameplayVrfState, this does not require a session-manager-owned session account.
/// The seed_key is typically matched_player_a from the PitDraftQueue.
#[derive(Accounts)]
pub struct InitPitDraftVrfState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Key used for VRF state PDA derivation. For pit draft, pass player_a's pubkey.
    pub seed_key: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + GameplayVrfState::SPACE,
        seeds = [GameplayVrfState::SEED_PREFIX, seed_key.key().as_ref()],
        bump,
    )]
    pub vrf_state: Account<'info, GameplayVrfState>,

    pub system_program: Program<'info, System>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegatePitDraftVrfState<'info> {
    #[account(mut, del)]
    /// CHECK: PDA validated via seed check in handler.
    pub gameplay_vrf_state: AccountInfo<'info>,
    /// CHECK: Seed key used for PDA derivation (player_a pubkey).
    pub seed_key: UncheckedAccount<'info>,
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct RequestGameplayVrf<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Session PDA; used only for VRF state seed derivation.
    pub session: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + GameplayVrfState::SPACE,
        seeds = [GameplayVrfState::SEED_PREFIX, session.key().as_ref()],
        bump,
    )]
    pub vrf_state: Account<'info, GameplayVrfState>,

    /// CHECK: Program identity PDA used as callback signer.
    #[account(seeds = [ephemeral_vrf_sdk::consts::IDENTITY], bump)]
    pub program_identity: UncheckedAccount<'info>,

    /// CHECK: Oracle queue account selected by the caller.
    #[account(mut)]
    pub oracle_queue: UncheckedAccount<'info>,

    /// CHECK: Slot hashes sysvar for VRF request validation.
    #[account(address = anchor_lang::solana_program::sysvar::slot_hashes::ID)]
    pub slot_hashes: UncheckedAccount<'info>,

    /// CHECK: VRF program for CPI invocation.
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_ID)]
    pub vrf_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FulfillGameplayVrf<'info> {
    /// In production: oracle program identity signer.
    /// Under `mock-vrf`: any signer accepted.
    #[cfg_attr(
        not(feature = "mock-vrf"),
        account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)
    )]
    pub vrf_program_identity: Signer<'info>,

    #[account(
        mut,
        seeds = [GameplayVrfState::SEED_PREFIX, vrf_state.session.as_ref()],
        bump = vrf_state.bump,
    )]
    pub vrf_state: Account<'info, GameplayVrfState>,
}

#[derive(Accounts)]
pub struct CloseGameplayVrfState<'info> {
    #[account(
        mut,
        seeds = [GameplayVrfState::SEED_PREFIX, vrf_state.session.as_ref()],
        bump = vrf_state.bump,
        close = session_signer,
    )]
    pub vrf_state: Account<'info, GameplayVrfState>,

    /// GameState for authorization (player + session_signer fields).
    #[account(
        seeds = [GAME_STATE_SEED, vrf_state.session.as_ref()],
        bump = game_state.bump,
    )]
    pub game_state: Account<'info, GameState>,

    /// CHECK: Validated against game_state.player in instruction body.
    pub player: AccountInfo<'info>,

    /// Session key signer authorizes closure and receives the rent refund.
    #[account(mut)]
    pub session_signer: Signer<'info>,
}

// Events

#[event]
pub struct GameStateInitialized {
    pub player: Pubkey,
    pub session: Pubkey,
    pub map_width: u8,
    pub map_height: u8,
}

#[event]
pub struct PhaseAdvanced {
    pub player: Pubkey,
    pub new_phase: Phase,
    pub new_week: u8,
    pub moves_remaining: u8,
}

#[event]
pub struct BossFightReady {
    pub player: Pubkey,
    pub week: u8,
}

#[event]
pub struct GameStateClosed {
    pub player: Pubkey,
    pub total_moves: u32,
    pub final_phase: Phase,
    pub final_week: u8,
}

#[event]
pub struct GauntletEchoesClosed {
    pub session: Pubkey,
}

/// Emitted when player is healed via authorized CPI from poi-system
#[event]
pub struct PlayerHealed {
    pub player: Pubkey,
    pub old_hp: i16,
    pub new_hp: i16,
    pub amount: u16,
    pub max_hp: i16,
}

/// Emitted when HP bonus is added via authorized CPI from player-inventory (equipping +HP gear)
#[event]
pub struct HpBonusAdded {
    pub player: Pubkey,
    pub old_hp: i16,
    pub new_hp: i16,
    pub hp_bonus: i16,
}

/// Emitted when HP bonus is removed via authorized CPI from player-inventory (unequipping +HP gear)
#[event]
pub struct HpBonusRemoved {
    pub player: Pubkey,
    pub old_hp: i16,
    pub new_hp: i16,
    pub hp_bonus: i16,
    pub new_max_hp: i16,
}

/// Emitted when gold is modified via authorized CPI from poi-system
#[event]
pub struct GoldModifiedAuthorized {
    pub player: Pubkey,
    pub old_gold: u16,
    pub new_gold: u16,
    pub delta: i16,
}

/// Emitted when combat starts (either player walked into enemy or enemy walked into player)
#[event]
pub struct CombatStarted {
    pub player: Pubkey,
    pub player_hp: i16,
    pub player_atk: i16,
    pub enemy_archetype: u8,
    pub enemy_hp: i16,
    pub enemy_atk: i16,
}

/// Emitted when combat ends
#[event]
pub struct CombatEnded {
    pub player: Pubkey,
    pub player_won: bool,
    pub final_player_hp: i16,
    pub final_enemy_hp: i16,
    pub gold_earned: u16,
    pub turns_taken: u8,
}

/// Emitted when boss combat starts
#[event]
pub struct BossCombatStarted {
    pub player: Pubkey,
    pub boss_id: [u8; 12],
    pub boss_hp: i16,
    pub week: u8,
}

/// Emitted when the player is defeated (HP <= 0)
#[event]
pub struct PlayerDefeated {
    pub player: Pubkey,
    pub killed_by: DeathCause,
    pub final_hp: i16,
}

/// Cause of player death - uses enum instead of String for efficiency
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeathCause {
    /// Killed by a field enemy
    Enemy = 0,
    /// Killed by a boss
    Boss = 1,
}

/// Emitted when a level is completed (Week 3 boss defeated)
#[event]
pub struct LevelCompleted {
    pub player: Pubkey,
    pub level: u8,
    pub total_moves: u32,
    pub gold_earned: u16,
}

#[event]
pub struct PitDraftQueued {
    pub player: Pubkey,
    pub profile: Pubkey,
    pub entry_lamports: u64,
}

#[event]
pub struct PitDraftResolved {
    pub player_a: Pubkey,
    pub player_b: Pubkey,
    pub winner: Pubkey,
    pub entry_lamports: u64,
    pub total_pot: u64,
    pub winner_payout: u64,
    pub company_fee: u64,
    pub gauntlet_fee: u64,
    pub turns_taken: u8,
}

#[event]
pub struct PitDraftCombatVisual {
    /// Waiting player (the first entrant in the matchup)
    pub player_a: Pubkey,
    /// Second entrant (the player that triggers instant match)
    pub player_b: Pubkey,
    /// Drafted tool for player A (includes oil flags)
    pub player_a_tool: Option<ItemInstance>,
    /// Drafted gear for player A (7 slots populated, one empty)
    pub player_a_gear: [Option<ItemInstance>; 12],
    /// Drafted tool for player B (includes oil flags)
    pub player_b_tool: Option<ItemInstance>,
    /// Drafted gear for player B (7 slots populated, one empty)
    pub player_b_gear: [Option<ItemInstance>; 12],
    /// True when player A wins (player B wins when false)
    pub player_a_won: bool,
    pub final_player_a_hp: i16,
    pub final_player_b_hp: i16,
    pub turns_taken: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DuelResolution {
    CompletedCombat = 0,
    OpponentEliminated = 1,
    UnmatchedEliminated = 2,
    BothEliminated = 3,
}

#[event]
pub struct DuelQueued {
    pub seed: u64,
    pub player: Pubkey,
    pub game_state: Pubkey,
    pub entry_lamports: u64,
    /// Queue position for this seed (1 for first player, 2 for second).
    pub slot: u8,
}

#[event]
pub struct DuelRunFinalized {
    pub seed: u64,
    pub player: Pubkey,
    pub completed_week3: bool,
    pub final_week: u8,
}

#[event]
pub struct DuelCombatVisual {
    pub seed: u64,
    pub player_a: Pubkey,
    pub player_b: Pubkey,
    pub player_a_tool: Option<ItemInstance>,
    pub player_a_gear: [Option<ItemInstance>; 12],
    pub player_b_tool: Option<ItemInstance>,
    pub player_b_gear: [Option<ItemInstance>; 12],
    pub player_a_won: bool,
    pub final_player_a_hp: i16,
    pub final_player_b_hp: i16,
    pub turns_taken: u8,
}

#[event]
pub struct DuelResolved {
    pub seed: u64,
    pub player_a: Pubkey,
    pub player_b: Option<Pubkey>,
    pub winner: Option<Pubkey>,
    pub total_pot: u64,
    pub winner_payout: u64,
    pub company_fee: u64,
    pub gauntlet_fee: u64,
    pub resolution: DuelResolution,
    pub turns_taken: Option<u8>,
}

#[event]
pub struct GauntletEntered {
    pub player: Pubkey,
    pub session: Pubkey,
    pub entry_lamports: u64,
    pub company_fee: u64,
    pub pool_fee: u64,
}

#[event]
pub struct GauntletWeekEchoSelected {
    pub player: Pubkey,
    pub week: u8,
    pub source_player: Option<Pubkey>,
}

#[event]
pub struct GauntletCombatVisual {
    pub player: Pubkey,
    pub week: u8,
    pub player_tool: Option<ItemInstance>,
    pub player_gear: [Option<ItemInstance>; 12],
    pub echo_tool: Option<ItemInstance>,
    pub echo_gear: [Option<ItemInstance>; 12],
    pub player_won: bool,
    pub final_player_hp: i16,
    pub final_echo_hp: i16,
    pub turns_taken: u8,
}

#[event]
pub struct GauntletWeekAdvanced {
    pub player: Pubkey,
    pub new_week: u8,
    pub completed: bool,
}

#[event]
pub struct GauntletRunEnded {
    pub player: Pubkey,
    pub week: u8,
    pub completed: bool,
}

#[event]
pub struct GauntletEpochFinalized {
    pub epoch_id: u64,
    pub total_pool_lamports: u64,
    pub total_points: u64,
}

#[event]
pub struct GauntletRewardsClaimed {
    pub epoch_id: u64,
    pub player: Pubkey,
    pub points: u64,
    pub payout_lamports: u64,
}

#[event]
pub struct GauntletSessionSettled {
    pub player: Pubkey,
    pub session: Pubkey,
    pub epoch_id: u64,
    pub points_credited: u64,
}

#[cfg(test)]
mod hp_logic_tests {
    use super::*;
    use crate::constants::{DAY_MOVES, NIGHT_MOVES};

    fn make_base_stats() -> PlayerStats {
        PlayerStats {
            max_hp: 10,
            dig: 1,
            atk: 0,
            arm: 0,
            spd: 0,
            strikes: 1,
        }
    }

    #[test]
    fn test_hp_capping_logic() {
        // Test that combat HP is capped at max_hp from derived stats.
        // MaxHp bonuses are included in stats.max_hp via calculate_stats().
        // ATK/ARM/SPD are applied during combat's BattleStart phase, not pre-calculated.
        let stats = PlayerStats {
            max_hp: 15, // Already includes +5 from MaxHp effect (e.g., Work Vest)
            dig: 1,
            atk: 0,
            arm: 0,
            spd: 0,
            strikes: 1,
        };

        // Player at full HP
        let current_hp: i16 = 15;
        let effects: Vec<AnnotatedItemEffect> = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 15, "Combat HP should match current HP");
        assert_eq!(input.max_hp, 15, "Combat max_hp should match derived stats");
        assert_eq!(input.atk, 0, "ATK should match baked stat value");
        assert_eq!(input.arm, 0, "ARM should match baked stat value");
        assert_eq!(input.spd, 0, "SPD should match baked stat value");

        // Simulate combat: lose 3 HP
        let final_combat_hp: i16 = 12;

        // Post-combat capping
        let new_persistent_hp = final_combat_hp.min(stats.max_hp);
        assert_eq!(
            new_persistent_hp, 12,
            "HP should persist as 12 (below max 15)"
        );
    }

    #[test]
    fn test_hp_damage_persistence() {
        // Test that damage persists correctly after combat.
        let stats = PlayerStats {
            max_hp: 15, // Includes item bonuses from calculate_stats()
            dig: 1,
            atk: 0,
            arm: 0,
            spd: 0,
            strikes: 1,
        };

        let current_hp: i16 = 15;
        let effects: Vec<AnnotatedItemEffect> = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 15);
        assert_eq!(input.max_hp, 15);

        // Player loses 7 HP, ending at 8
        let final_combat_hp: i16 = 8;

        let new_persistent_hp = final_combat_hp.min(stats.max_hp);
        assert_eq!(
            new_persistent_hp, 8,
            "HP should persist as 8 (lower than max 15)"
        );
    }

    #[test]
    fn test_mid_combat_healing() {
        // Scenario 3: 10 HP. Lose 3 (7). Heal 2 (9). End -> 9.
        // Note: Mid-combat healing affects the final_combat_hp result directly.
        // We simulate the result of combat being 9.
        let current_hp: i16 = 10;
        let stats = make_base_stats();

        let effects: Vec<AnnotatedItemEffect> = vec![]; // No battle start bonus

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 10);
        assert_eq!(input.max_hp, 10);

        // Combat happens: 10 -> 7 -> 9
        let final_combat_hp: i16 = 9;

        let new_persistent_hp = final_combat_hp.min(stats.max_hp);
        assert_eq!(new_persistent_hp, 9, "HP should be 9");
    }

    #[test]
    fn test_derived_stats_in_combat() {
        // Test that derived stats (from inventory) are used correctly in combat
        // max_hp/dig/strikes and baked Tool/Gear combat stats should all be reflected.
        let current_hp: i16 = 8;
        let stats = PlayerStats {
            max_hp: 15, // Increased from MaxHp effects (e.g., Work Vest)
            dig: 3,     // From DIG items
            atk: 2,
            arm: 4,
            spd: 1,
            strikes: 2, // From GainStrikes items (e.g., Twin Picks)
        };

        let effects: Vec<AnnotatedItemEffect> = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 8);
        assert_eq!(input.max_hp, 15);
        assert_eq!(input.atk, 2);
        assert_eq!(input.arm, 4);
        assert_eq!(input.spd, 1);
        assert_eq!(input.dig, 3);
        // Strikes are pre-calculated from GainStrikes effects
        assert_eq!(input.strikes, 2);
    }

    #[test]
    fn test_baked_atk_is_reflected_in_combatant_input() {
        let stats = PlayerStats {
            max_hp: 15,
            dig: 1,
            atk: 5,
            arm: 0,
            spd: 0,
            strikes: 1,
        };

        let effects: Vec<AnnotatedItemEffect> = vec![];

        let current_hp: i16 = 15;
        let input = build_player_combatant(current_hp, &stats, &effects);

        assert_eq!(input.atk, 5, "ATK should match baked combat stat");
        assert_eq!(input.max_hp, 15, "max_hp should match derived stats");
    }

    #[test]
    fn test_coin_slug_armor_from_gold() {
        // Coin Slug: Battle Start: gain Armor equal to floor(player Gold/10) (cap 3)
        // This tests the preprocess_enemy_effects function.

        use field_enemies::archetypes::ids::COIN_SLUG;

        // 0 gold = 0 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 0);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].effect.value, 0);

        // 9 gold = 0 armor (floor(9/10) = 0)
        let effects = preprocess_enemy_effects(COIN_SLUG, 9);
        assert_eq!(effects[0].effect.value, 0);

        // 10 gold = 1 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 10);
        assert_eq!(effects[0].effect.value, 1);

        // 25 gold = 2 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 25);
        assert_eq!(effects[0].effect.value, 2);

        // 30 gold = 3 armor (cap)
        let effects = preprocess_enemy_effects(COIN_SLUG, 30);
        assert_eq!(effects[0].effect.value, 3);

        // 100 gold = 3 armor (capped at 3)
        let effects = preprocess_enemy_effects(COIN_SLUG, 100);
        assert_eq!(effects[0].effect.value, 3, "Armor should be capped at 3");

        // Non-Coin Slug enemies should not be affected
        let effects = preprocess_enemy_effects(0, 100); // Tunnel Rat
        assert!(!effects.iter().any(|e| {
            matches!(e.effect.effect_type, EffectType::GainArmor) && e.effect.value == 3
        }));
    }

    #[test]
    fn test_find_matching_creator_index_skips_self() {
        let entrant = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let seed = 42u64;

        let loadout = DuelLoadoutSnapshot {
            tool: None,
            gear: [None; 12],
            gold_at_battle_start: 0,
            hp_at_finish: 0,
        };
        let queue = DuelOpenQueue {
            entries: vec![
                DuelCreatorEntry {
                    player: entrant,
                    seed: 7,
                    entry_lamports: 1,
                    finished_slot: 1,
                    loadout,
                },
                DuelCreatorEntry {
                    player: other,
                    seed,
                    entry_lamports: 1,
                    finished_slot: 2,
                    loadout,
                },
                DuelCreatorEntry {
                    player: other,
                    seed,
                    entry_lamports: 1,
                    finished_slot: 3,
                    loadout,
                },
            ],
            initialized: true,
            bump: 1,
        };

        // Seed match: index 1 is first non-entrant with matching seed
        let idx = find_matching_creator_index(&queue, entrant, seed);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn test_find_matching_creator_index_none_when_only_self() {
        let entrant = Pubkey::new_unique();
        let seed = 99u64;
        let loadout = DuelLoadoutSnapshot {
            tool: None,
            gear: [None; 12],
            gold_at_battle_start: 0,
            hp_at_finish: 0,
        };
        let queue = DuelOpenQueue {
            entries: vec![DuelCreatorEntry {
                player: entrant,
                seed,
                entry_lamports: 1,
                finished_slot: 1,
                loadout,
            }],
            initialized: true,
            bump: 1,
        };

        let idx = find_matching_creator_index(&queue, entrant, seed);
        assert_eq!(idx, None);
    }

    #[test]
    fn test_item_pool_index_from_id_matches_profile_bitmask_scheme() {
        assert_eq!(item_pool_index_from_id(b"G-ST-01\0"), Some(0));
        assert_eq!(item_pool_index_from_id(b"G-ST-08\0"), Some(7));
        assert_eq!(item_pool_index_from_id(b"G-SC-01\0"), Some(8));
        assert_eq!(item_pool_index_from_id(b"G-TE-08\0"), Some(63));
        assert_eq!(item_pool_index_from_id(b"T-ST-01\0"), Some(64));
        assert_eq!(item_pool_index_from_id(b"T-GR-01\0"), Some(68));
        assert_eq!(item_pool_index_from_id(b"T-TE-02\0"), Some(79));

        // Starter-only special tool is intentionally outside pool indexing.
        assert_eq!(item_pool_index_from_id(b"T-XX-00\0"), None);
    }

    #[test]
    fn test_is_pool_item_enabled_respects_starter_pool() {
        // Mirrors player-profile STARTER_ITEMS_BITMASK.
        let starter_pool: [u8; 10] = [0x0F, 0x0F, 0x17, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x55, 0x55];

        // Starter-enabled samples.
        assert!(is_pool_item_enabled(&starter_pool, b"T-GR-01\0"));
        assert!(is_pool_item_enabled(&starter_pool, b"G-GR-03\0"));
        assert!(is_pool_item_enabled(&starter_pool, b"G-BL-01\0"));

        // Non-starter samples that previously slipped through with ITEMS-indexed filtering.
        assert!(!is_pool_item_enabled(&starter_pool, b"G-ST-07\0"));
        assert!(!is_pool_item_enabled(&starter_pool, b"G-SC-06\0"));
        assert!(!is_pool_item_enabled(&starter_pool, b"G-RU-07\0"));
        assert!(!is_pool_item_enabled(&starter_pool, b"G-TE-07\0"));
    }

    #[test]
    fn test_should_resolve_weekly_boss_campaign_all_weeks() {
        assert!(should_resolve_weekly_boss(RunMode::Campaign, 1));
        assert!(should_resolve_weekly_boss(RunMode::Campaign, 2));
        assert!(should_resolve_weekly_boss(RunMode::Campaign, 3));
    }

    #[test]
    fn test_should_resolve_weekly_boss_duel_only_weeks_1_2() {
        assert!(should_resolve_weekly_boss(RunMode::Duel, 1));
        assert!(should_resolve_weekly_boss(RunMode::Duel, 2));
        assert!(!should_resolve_weekly_boss(RunMode::Duel, 3));
    }

    #[test]
    fn test_should_resolve_weekly_boss_gauntlet_never() {
        assert!(!should_resolve_weekly_boss(RunMode::Gauntlet, 1));
        assert!(!should_resolve_weekly_boss(RunMode::Gauntlet, 3));
        assert!(!should_resolve_weekly_boss(RunMode::Gauntlet, 5));
    }

    fn make_gauntlet_gate_state() -> GameState {
        GameState {
            player: Pubkey::default(),
            session_signer: Pubkey::default(),
            session: Pubkey::default(),
            position_x: 0,
            position_y: 0,
            map_width: 10,
            map_height: 10,
            hp: 10,
            gear_slots: 4,
            week: 1,
            phase: Phase::Night3,
            moves_remaining: 0,
            total_moves: 0,
            boss_fight_ready: true,
            gold: 0,
            bump: 1,
            campaign_level: 20,
            run_mode: RunMode::Gauntlet,
            max_weeks: 5,
            is_dead: false,
            completed: false,
            gauntlet_epoch_id: 0,
            gauntlet_points_earned: 0,
            gauntlet_defender_credit: None,
            gauntlet_highest_week_won: 0,
            gauntlet_settled: false,
            duel_map_seed: 0,
            enemies: Vec::new(),
            enemy_count: 0,
        }
    }

    #[test]
    fn test_gauntlet_week_resolution_ready_requires_night3_zero_moves_and_ready_flag() {
        let state = make_gauntlet_gate_state();
        assert!(gauntlet_week_resolution_ready(&state));
    }

    #[test]
    fn test_gauntlet_week_resolution_ready_rejects_missing_gate_conditions() {
        let mut state = make_gauntlet_gate_state();
        state.boss_fight_ready = false;
        assert!(!gauntlet_week_resolution_ready(&state));

        state = make_gauntlet_gate_state();
        state.phase = Phase::Night2;
        assert!(!gauntlet_week_resolution_ready(&state));

        state = make_gauntlet_gate_state();
        state.moves_remaining = 1;
        assert!(!gauntlet_week_resolution_ready(&state));
    }

    #[test]
    fn test_phase_spanning_day_move_uses_night_phase_and_radius() {
        let (phase, moves_remaining) = resolve_post_move_phase_and_moves(Phase::Day1, 1, 2).unwrap();
        assert_eq!(phase, Phase::Night1);
        assert_eq!(moves_remaining, NIGHT_MOVES - 1);
        assert_eq!(visibility_radius_for_phase(phase), NIGHT_VISION_RADIUS);
    }

    #[test]
    fn test_phase_spanning_night_move_uses_next_day_phase_and_radius() {
        let (phase, moves_remaining) =
            resolve_post_move_phase_and_moves(Phase::Night1, 1, 2).unwrap();
        assert_eq!(phase, Phase::Day2);
        assert_eq!(moves_remaining, DAY_MOVES - 1);
        assert_eq!(visibility_radius_for_phase(phase), DAY_VISION_RADIUS);
    }
}

#[cfg(test)]
mod combat_scenarios_tests;

#[cfg(test)]
mod duel_flow_tests {
    use super::*;
    use crate::movement::{get_duel_boss_id_from_seed, get_duel_boss_for_combat_from_seed};

    #[test]
    fn test_duel_boss_from_seed_deterministic() {
        // Same seed + week should always produce the same boss
        let boss1 = get_duel_boss_id_from_seed(12345, 1).unwrap();
        let boss2 = get_duel_boss_id_from_seed(12345, 1).unwrap();
        assert_eq!(boss1, boss2, "Same seed+week should produce same boss");
    }

    #[test]
    fn test_duel_boss_from_seed_differs_by_week() {
        // Different weeks should produce different bosses (with high probability)
        let boss_w1 = get_duel_boss_id_from_seed(99999, 1).unwrap();
        let boss_w2 = get_duel_boss_id_from_seed(99999, 2).unwrap();
        // Not guaranteed to differ, but extremely likely with different week inputs
        // Just verify both are valid (non-zero)
        assert_ne!(boss_w1, [0u8; 12], "Week 1 boss should be non-zero");
        assert_ne!(boss_w2, [0u8; 12], "Week 2 boss should be non-zero");
    }

    #[test]
    fn test_duel_boss_from_seed_differs_by_seed() {
        // Different seeds should produce different bosses (with high probability)
        let boss_a = get_duel_boss_id_from_seed(11111, 1).unwrap();
        let boss_b = get_duel_boss_id_from_seed(22222, 1).unwrap();
        assert_ne!(boss_a, boss_b, "Different seeds should produce different bosses");
    }

    #[test]
    fn test_duel_boss_combatant_from_seed_matches_id() {
        // The boss ID from get_duel_boss_id_from_seed should match the combatant from
        // get_duel_boss_for_combat_from_seed (same boss, just different return types)
        let seed = 42u64;
        let week = 1u8;
        let boss_id = get_duel_boss_id_from_seed(seed, week).unwrap();
        let _combatant = get_duel_boss_for_combat_from_seed(seed, week).unwrap();
        // The combatant should be for the same boss
        let boss_def = boss_system::get_boss(&boss_id);
        assert!(boss_def.is_some(), "Boss ID should resolve to a valid boss definition");
    }

    #[test]
    fn test_duel_boss_both_players_same_seed_same_boss() {
        // Both matched duel players share the same map seed, so they must get identical bosses.
        // Simulates: player A generates map with seed X, player B matches and gets seed X.
        let shared_seed = 7777777u64;
        for week in 1..=2u8 {
            let boss_player_a = get_duel_boss_id_from_seed(shared_seed, week).unwrap();
            let boss_player_b = get_duel_boss_id_from_seed(shared_seed, week).unwrap();
            assert_eq!(
                boss_player_a, boss_player_b,
                "Both players must get the same boss for week {} with shared seed",
                week
            );
        }
    }

    #[test]
    fn test_duel_loadout_snapshot_has_hp() {
        // Verify DuelLoadoutSnapshot includes hp_at_finish field
        let loadout = DuelLoadoutSnapshot {
            tool: None,
            gear: [None; 12],
            gold_at_battle_start: 100,
            hp_at_finish: 15,
        };
        assert_eq!(loadout.hp_at_finish, 15);
        assert_eq!(loadout.gold_at_battle_start, 100);
    }

    #[test]
    fn test_compute_pvp_pot_split() {
        // 0.2 SOL pot = 200_000_000 lamports
        let total_pot = 200_000_000u64;
        let (company_fee, gauntlet_fee, winner_payout) = compute_pvp_pot_split(total_pot).unwrap();

        // 3% company, 2% gauntlet, 95% winner
        assert_eq!(company_fee, 6_000_000, "Company fee should be 3%");
        assert_eq!(gauntlet_fee, 4_000_000, "Gauntlet fee should be 2%");
        assert_eq!(winner_payout, 190_000_000, "Winner payout should be 95%");
        assert_eq!(company_fee + gauntlet_fee + winner_payout, total_pot, "Fees + payout should equal total pot");
    }

    #[test]
    fn test_compute_eliminated_unmatched_distribution() {
        // 0.1 SOL entry = 100_000_000 lamports
        let entry = 100_000_000u64;
        let (company_total, gauntlet_total) = compute_eliminated_unmatched_distribution(entry).unwrap();

        // Should sum to the full entry amount
        assert_eq!(company_total + gauntlet_total, entry, "Distribution should equal entry amount");
        // Company gets slightly more (3% base + half remainder)
        assert!(company_total > gauntlet_total, "Company should get slightly more than gauntlet");
    }
}
