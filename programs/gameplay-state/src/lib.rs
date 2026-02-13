use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke;
use anchor_lang::system_program;
use core::str::FromStr;

pub mod constants;
pub mod errors;
pub mod movement;
pub mod state;
pub mod stats;

use combat_system::state::CombatantInput;
#[cfg(test)]
use combat_system::state::Condition;
use combat_system::{
    resolve_combat_with_both_gold, resolve_combat_with_player_gold, CombatLogEntry, EffectType, ItemEffect,
};
use constants::{
    BASE_ARM, BASE_ATK, BASE_HP, BASE_SPD, COMPANY_TREASURY_ADDRESS, DAY_MOVES, DUEL_ENTRY_LAMPORTS,
    DUEL_ENTRY_SEED, DUEL_OPEN_QUEUE_SEED, DUEL_QUEUE_SEED, DUEL_VAULT_SEED, GAME_STATE_SEED,
    GAUNTLET_BOOTSTRAP_ECHOES_PER_WEEK,
    GAUNTLET_COMPANY_FEE_BPS, GAUNTLET_CONFIG_SEED, GAUNTLET_ENTRY_LAMPORTS, GAUNTLET_EPOCH_DURATION_SECONDS,
    GAUNTLET_EPOCH_POOL_SEED, GAUNTLET_MAX_WEEKLY_ECHOES, GAUNTLET_PLAYER_SCORE_SEED,
    GAUNTLET_POOL_FEE_BPS, GAUNTLET_POOL_VAULT_SEED, GAUNTLET_SINK_ADDRESS, GAUNTLET_WEEK_POOL_SEED,
    INITIAL_GEAR_SLOTS, MAX_GEAR_SLOTS, PIT_DRAFT_BPS_DENOMINATOR, PIT_DRAFT_COMPANY_FEE_BPS,
    PIT_DRAFT_ENTRY_LAMPORTS, PIT_DRAFT_GAUNTLET_FEE_BPS, PIT_DRAFT_QUEUE_SEED, PIT_DRAFT_VAULT_SEED,
    PIT_DRAFT_WINNER_BPS,
};
use errors::GameplayStateError;

/// Seed for gameplay_authority PDA used for CPI calls to other programs
pub const GAMEPLAY_AUTHORITY_SEED: &[u8] = b"gameplay_authority";
use movement::{
    calculate_move_cost, chebyshev_distance, get_boss_for_combat, get_boss_id, get_duel_boss_for_combat,
    get_duel_boss_id, is_adjacent,
    is_within_bounds, should_process_night_enemy_movement, should_process_target_enemy_combat,
};
use player_inventory::effects::generate_combat_effects;
use player_inventory::items::ITEMS;
use player_inventory::state::{ItemInstance, ItemType, PlayerInventory, Tier, ToolOilModification};
use player_profile::state::PlayerProfile;
use state::{
    DuelCreatorEntry, DuelEntry, DuelLoadoutSnapshot, DuelOpenQueue, DuelQueue, DuelRunOutcome,
    DuelVault, GauntletConfig,
    GauntletEchoSnapshot, GauntletEchoSource, GauntletEpochPool, GauntletLoadoutSnapshot, GauntletPendingPoints,
    GauntletPlayerScore, GauntletPoolVault, GauntletWeekPool, GameState, MapEnemies, Phase, PitDraftQueue,
    PitDraftVault,
    RunMode,
};
use stats::{calculate_stats, PlayerStats};

declare_id!("5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n");

/// Session manager program ID for session ownership checks
pub const SESSION_MANAGER_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    217, 18, 17, 128, 79, 140, 152, 73, 103, 95, 134, 179, 31, 109, 34, 82, 250, 167, 91, 67, 186,
    23, 209, 2, 80, 255, 118, 192, 175, 242, 222, 183,
]);

/// POI system program ID for authorized HP/Gold modifications
/// Derived from "6E27r1Cyo2CNPvtRsonn3uHUAdznS3cMXEBX4HRbfBQY"
pub const POI_SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    77, 160, 63, 209, 182, 56, 149, 181, 2, 195, 173, 95, 65, 136, 88, 122, 235, 166, 235, 216,
    241, 107, 2, 35, 185, 14, 177, 21, 150, 103, 215, 77,
]);
pub const NIGHT_VISION_RADIUS: u8 = 2;
pub const DAY_VISION_RADIUS: u8 = 4;
pub const PIT_DRAFT_MAX_START_GOLD: u16 = 30;
pub const DISCOVER_VISIBLE_WAYPOINTS_DISCRIMINATOR: [u8; 8] =
    [0x3b, 0x26, 0x6a, 0x00, 0x3a, 0xb1, 0x50, 0xfc];

/// Player inventory program ID for authorized HP modifications via CPI
/// Derived from "5BtqiWegvVAgEnTRUofB9oUoQvPztYqSkMPwRpYQacP8"
pub const PLAYER_INVENTORY_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    62, 57, 43, 224, 11, 129, 112, 214, 10, 252, 5, 51, 143, 242, 213, 246, 99, 145, 248, 173, 61,
    133, 58, 191, 234, 132, 254, 214, 152, 21, 230, 167,
]);

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
            start_x < map_width && start_y < map_height,
            GameplayStateError::OutOfBounds
        );

        let game_state = &mut ctx.accounts.game_state;
        game_state.player = ctx.accounts.player.key();
        game_state.burner_wallet = ctx.accounts.burner_wallet.key();
        game_state.session = ctx.accounts.game_session.key();
        game_state.position_x = start_x;
        game_state.position_y = start_y;
        game_state.map_width = map_width;
        game_state.map_height = map_height;
        game_state.hp = BASE_HP;
        game_state.gear_slots = INITIAL_GEAR_SLOTS;
        game_state.week = 1;
        game_state.phase = Phase::Day1;
        game_state.moves_remaining = DAY_MOVES;
        game_state.total_moves = 0;
        game_state.boss_fight_ready = false;
        game_state.gold = 0;
        game_state.bump = ctx.bumps.game_state;
        game_state.campaign_level = campaign_level;
        game_state.run_mode = RunMode::Campaign;
        game_state.max_weeks = 3;
        game_state.is_dead = false;
        game_state.completed = false;

        let map_enemies = &mut ctx.accounts.map_enemies;
        let generated_map = &ctx.accounts.generated_map;

        map_enemies.session = ctx.accounts.game_session.key();
        map_enemies.bump = ctx.bumps.map_enemies;
        map_enemies.enemies = Vec::with_capacity(generated_map.enemy_count as usize);

        for idx in 0..generated_map.enemy_count as usize {
            let enemy = generated_map.enemies[idx];
            map_enemies.enemies.push(state::EnemyInstance {
                archetype_id: enemy.archetype_id,
                tier: enemy.tier,
                x: enemy.x,
                y: enemy.y,
                defeated: false,
            });
        }

        map_enemies.count = map_enemies.enemies.len() as u8;

        emit!(GameStateInitialized {
            player: game_state.player,
            session: game_state.session,
            map_width,
            map_height,
        });

        Ok(())
    }

    /// Initializes global pit draft queue/vault PDAs.
    pub fn initialize_pit_draft(ctx: Context<InitializePitDraft>) -> Result<()> {
        let queue = &mut ctx.accounts.pit_draft_queue;
        let vault = &mut ctx.accounts.pit_draft_vault;

        if queue.bump == 0 {
            queue.waiting_player = None;
            queue.waiting_profile = None;
            queue.bump = ctx.bumps.pit_draft_queue;
        }
        if vault.bump == 0 {
            vault.bump = ctx.bumps.pit_draft_vault;
        }

        Ok(())
    }

    /// Initializes global duel vault PDA.
    pub fn initialize_duels(ctx: Context<InitializeDuels>) -> Result<()> {
        let vault = &mut ctx.accounts.duel_vault;
        if vault.bump == 0 {
            vault.bump = ctx.bumps.duel_vault;
        }
        let open_queue = &mut ctx.accounts.duel_open_queue;
        if open_queue.bump == 0 {
            open_queue.entries = Vec::new();
            open_queue.bump = ctx.bumps.duel_open_queue;
        }
        Ok(())
    }

    /// Initializes a seed-scoped duel queue PDA.
    pub fn initialize_duel_queue(ctx: Context<InitializeDuelQueue>, seed: u64) -> Result<()> {
        let queue = &mut ctx.accounts.duel_queue;
        if queue.bump == 0 {
            queue.seed = seed;
            queue.player_a = None;
            queue.player_b = None;
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
        require!((1..=5).contains(&max_weeks), GameplayStateError::InvalidWeek);
        let game_state = &mut ctx.accounts.game_state;
        game_state.run_mode = run_mode;
        game_state.max_weeks = max_weeks;
        Ok(())
    }

    /// Initializes Gauntlet config, pool vault and weekly echo pools.
    pub fn initialize_gauntlet(ctx: Context<InitializeGauntlet>) -> Result<()> {
        let config = &mut ctx.accounts.gauntlet_config;
        let clock = Clock::get()?;

        if config.bump == 0 {
            config.entry_lamports = GAUNTLET_ENTRY_LAMPORTS;
            config.company_fee_bps = GAUNTLET_COMPANY_FEE_BPS as u16;
            config.pool_fee_bps = GAUNTLET_POOL_FEE_BPS as u16;
            config.current_epoch_id = 0;
            config.current_epoch_start_ts = clock.unix_timestamp;
            config.epoch_duration_seconds = GAUNTLET_EPOCH_DURATION_SECONDS;
            config.bump = ctx.bumps.gauntlet_config;
        }

        if ctx.accounts.gauntlet_pool_vault.bump == 0 {
            ctx.accounts.gauntlet_pool_vault.bump = ctx.bumps.gauntlet_pool_vault;
        }

        if ctx.accounts.gauntlet_week1.bump == 0 {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week1, 1)?;
            ctx.accounts.gauntlet_week1.bump = ctx.bumps.gauntlet_week1;
        } else {
            require!(ctx.accounts.gauntlet_week1.week == 1, GameplayStateError::InvalidGauntletWeek);
        }
        if ctx.accounts.gauntlet_week2.bump == 0 {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week2, 2)?;
            ctx.accounts.gauntlet_week2.bump = ctx.bumps.gauntlet_week2;
        } else {
            require!(ctx.accounts.gauntlet_week2.week == 2, GameplayStateError::InvalidGauntletWeek);
        }
        if ctx.accounts.gauntlet_week3.bump == 0 {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week3, 3)?;
            ctx.accounts.gauntlet_week3.bump = ctx.bumps.gauntlet_week3;
        } else {
            require!(ctx.accounts.gauntlet_week3.week == 3, GameplayStateError::InvalidGauntletWeek);
        }
        if ctx.accounts.gauntlet_week4.bump == 0 {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week4, 4)?;
            ctx.accounts.gauntlet_week4.bump = ctx.bumps.gauntlet_week4;
        } else {
            require!(ctx.accounts.gauntlet_week4.week == 4, GameplayStateError::InvalidGauntletWeek);
        }
        if ctx.accounts.gauntlet_week5.bump == 0 {
            initialize_week_pool(&mut ctx.accounts.gauntlet_week5, 5)?;
            ctx.accounts.gauntlet_week5.bump = ctx.bumps.gauntlet_week5;
        } else {
            require!(ctx.accounts.gauntlet_week5.week == 5, GameplayStateError::InvalidGauntletWeek);
        }

        Ok(())
    }

    /// Pays gauntlet entry and marks this run as gauntlet mode.
    pub fn enter_gauntlet(ctx: Context<EnterGauntlet>) -> Result<()> {
        let company_treasury_expected = Pubkey::from_str(COMPANY_TREASURY_ADDRESS)
            .map_err(|_| GameplayStateError::InvalidGauntletFeeAccount)?;
        require_keys_eq!(
            ctx.accounts.company_treasury.key(),
            company_treasury_expected,
            GameplayStateError::InvalidGauntletFeeAccount
        );

        let game_state = &mut ctx.accounts.game_state;
        require!(
            !game_state.is_dead && !game_state.completed,
            GameplayStateError::GauntletRunEnded
        );
        require!(
            game_state.run_mode != RunMode::Gauntlet,
            GameplayStateError::GauntletAlreadyEntered
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

        emit!(GauntletEntered {
            player: ctx.accounts.player.key(),
            session: game_state.session,
            entry_lamports: entry,
            company_fee,
            pool_fee,
        });
        Ok(())
    }

    /// Resolves the end-of-week gauntlet echo combat.
    pub fn resolve_gauntlet_week(ctx: Context<ResolveGauntletWeek>, epoch_id: u64) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        require!(
            game_state.run_mode == RunMode::Gauntlet,
            GameplayStateError::GauntletRunNotActive
        );
        require!(!game_state.is_dead, GameplayStateError::GauntletRunEnded);
        require!(
            game_state.week >= 1 && game_state.week <= 5,
            GameplayStateError::InvalidGauntletWeek
        );

        let week = game_state.week;
        require!(
            ctx.accounts.gauntlet_config.current_epoch_id == epoch_id,
            GameplayStateError::GauntletScoreMismatch
        );
        ctx.accounts.gauntlet_epoch_pool.epoch_id = epoch_id;
        if ctx.accounts.gauntlet_epoch_pool.bump == 0 {
            ctx.accounts.gauntlet_epoch_pool.bump = ctx.bumps.gauntlet_epoch_pool;
            ctx.accounts.gauntlet_epoch_pool.finalized = false;
        }
        let week_pool = match week {
            1 => &mut ctx.accounts.gauntlet_week1,
            2 => &mut ctx.accounts.gauntlet_week2,
            3 => &mut ctx.accounts.gauntlet_week3,
            4 => &mut ctx.accounts.gauntlet_week4,
            _ => &mut ctx.accounts.gauntlet_week5,
        };
        require!(week_pool.week == week, GameplayStateError::InvalidGauntletWeek);
        require!(!week_pool.entries.is_empty(), GameplayStateError::GauntletNotInitialized);

        let rand = derive_u64_random(&[
            b"gauntlet_draw",
            &week.to_le_bytes(),
            game_state.session.as_ref(),
            ctx.accounts.player.key().as_ref(),
        ]);
        let idx = (rand % week_pool.entries.len() as u64) as usize;
        let echo = week_pool.entries[idx];

        let player_inventory = &ctx.accounts.inventory;
        let player_stats = calculate_stats(player_inventory);
        let player_effects = generate_combat_effects(player_inventory);
        let echo_inventory = snapshot_to_inventory(echo, game_state.session, ctx.accounts.player.key());
        let echo_stats = calculate_stats(&echo_inventory);
        let echo_effects = generate_combat_effects(&echo_inventory);

        let outcome = resolve_combat_with_both_gold(
            build_player_combatant(game_state.hp, &player_stats, &player_effects),
            build_full_hp_combatant(&echo_stats),
            player_effects,
            echo_effects,
            game_state.gold,
            echo.loadout.gold_at_battle_start,
        )?;

        emit!(GauntletWeekEchoSelected {
            player: ctx.accounts.player.key(),
            week,
            source_player: match echo.source {
                GauntletEchoSource::Bootstrap => None,
                GauntletEchoSource::Player(p) => Some(p),
            },
        });
        emit!(GauntletCombatVisual {
            player: ctx.accounts.player.key(),
            week,
            player_tool: player_inventory.tool,
            player_gear: player_inventory.gear,
            echo_tool: echo.loadout.tool,
            echo_gear: echo.loadout.gear,
            combat_log: outcome.log.clone(),
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
            upsert_player_score(
                &mut ctx.accounts.gauntlet_player_score,
                &ctx.accounts.player.key(),
                epoch_id,
                points_awarded,
                ctx.bumps.gauntlet_player_score,
            )?;
            ctx.accounts.gauntlet_epoch_pool.total_points = ctx
                .accounts
                .gauntlet_epoch_pool
                .total_points
                .checked_add(points_awarded)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;

            maybe_insert_player_echo(week_pool, game_state.week, player_inventory, game_state.gold, ctx.accounts.player.key())?;

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
                    &ctx.accounts.inventory.to_account_info(),
                    &ctx.accounts.gameplay_authority.to_account_info(),
                    &ctx.accounts.player_inventory_program.to_account_info(),
                    ctx.bumps.gameplay_authority,
                )?;
            }

            emit!(GauntletWeekAdvanced {
                player: ctx.accounts.player.key(),
                new_week: game_state.week,
                completed: game_state.completed,
            });
        } else {
            game_state.hp = 0;
            game_state.is_dead = true;
            game_state.boss_fight_ready = false;
            maybe_award_defender_points(
                &echo,
                week,
                &mut ctx.accounts.gauntlet_epoch_pool,
            )?;

            emit!(GauntletRunEnded {
                player: ctx.accounts.player.key(),
                week,
                completed: false,
            });
        }

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
        epoch_pool.total_pool_lamports = ctx.accounts.gauntlet_pool_vault.to_account_info().lamports();
        if epoch_pool.pending_defender_points.is_empty() {
            epoch_pool.pending_defender_points = Vec::new();
        }
        epoch_pool.finalized = true;
        if epoch_pool.bump == 0 {
            epoch_pool.bump = 1;
        }

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

        let pending_defender_points =
            take_pending_defender_points(&mut ctx.accounts.gauntlet_epoch_pool, ctx.accounts.player.key());
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

    /// Pays duel entry and registers this run in async duel matchmaking.
    pub fn enter_duel(ctx: Context<EnterDuel>, seed: u64) -> Result<()> {
        let company_treasury_expected = Pubkey::from_str(COMPANY_TREASURY_ADDRESS)
            .map_err(|_| GameplayStateError::InvalidDuelFeeAccount)?;
        let gauntlet_sink_expected = Pubkey::from_str(GAUNTLET_SINK_ADDRESS)
            .map_err(|_| GameplayStateError::InvalidDuelFeeAccount)?;

        require_keys_eq!(
            ctx.accounts.company_treasury.key(),
            company_treasury_expected,
            GameplayStateError::InvalidDuelFeeAccount
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_sink.key(),
            gauntlet_sink_expected,
            GameplayStateError::InvalidDuelFeeAccount
        );
        require!(
            !ctx.accounts.game_state.is_dead && !ctx.accounts.game_state.completed,
            GameplayStateError::DuelRunNotFinished
        );
        require!(
            ctx.accounts.game_state.run_mode == RunMode::Duel,
            GameplayStateError::DuelInvalidRunMode
        );
        require!(
            ctx.accounts.generated_map.seed == seed,
            GameplayStateError::DuelSeedMismatch
        );

        let player_key = ctx.accounts.player.key();
        let duel_entry = &mut ctx.accounts.duel_entry;
        require!(duel_entry.entry_lamports == 0, GameplayStateError::DuelAlreadyQueued);

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
        duel_entry.seed = seed;
        duel_entry.entry_lamports = DUEL_ENTRY_LAMPORTS;
        duel_entry.finalized = false;
        duel_entry.outcome = DuelRunOutcome::Pending;
        duel_entry.loadout = DuelLoadoutSnapshot {
            tool: None,
            gear: [None; 12],
            gold_at_battle_start: 0,
        };
        duel_entry.matched_creator = None;
        duel_entry.settled = false;
        duel_entry.bump = ctx.bumps.duel_entry;

        let open_queue = &mut ctx.accounts.duel_open_queue;
        let slot = if let Some(matched_idx) =
            find_matching_creator_index(open_queue, player_key, seed)
        {
            let creator = open_queue.entries.remove(matched_idx);
            duel_entry.matched_creator = Some(creator);
            2
        } else {
            1
        };

        emit!(DuelQueued {
            seed,
            player: player_key,
            game_state: ctx.accounts.game_state.key(),
            entry_lamports: DUEL_ENTRY_LAMPORTS,
            slot,
        });

        Ok(())
    }

    /// Finalizes this player's duel run and resolves duel outcomes when possible.
    pub fn finalize_duel_run(ctx: Context<FinalizeDuelRun>, seed: u64) -> Result<()> {
        let company_treasury_expected = Pubkey::from_str(COMPANY_TREASURY_ADDRESS)
            .map_err(|_| GameplayStateError::InvalidDuelFeeAccount)?;
        let gauntlet_sink_expected = Pubkey::from_str(GAUNTLET_SINK_ADDRESS)
            .map_err(|_| GameplayStateError::InvalidDuelFeeAccount)?;

        require_keys_eq!(
            ctx.accounts.company_treasury.key(),
            company_treasury_expected,
            GameplayStateError::InvalidDuelFeeAccount
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_sink.key(),
            gauntlet_sink_expected,
            GameplayStateError::InvalidDuelFeeAccount
        );
        require!(
            ctx.accounts.generated_map.seed == seed,
            GameplayStateError::DuelSeedMismatch
        );

        let game_state = &ctx.accounts.game_state;
        require!(
            game_state.run_mode == RunMode::Duel,
            GameplayStateError::DuelInvalidRunMode
        );
        require!(
            game_state.completed || game_state.is_dead,
            GameplayStateError::DuelRunNotFinished
        );

        let player_key = ctx.accounts.player.key();
        let duel_entry = &mut ctx.accounts.duel_entry;
        require_keys_eq!(duel_entry.player, player_key, GameplayStateError::DuelNotQueued);
        require_keys_eq!(
            duel_entry.game_state,
            ctx.accounts.game_state.key(),
            GameplayStateError::DuelGameStateMismatch
        );
        require_keys_eq!(
            duel_entry.session,
            game_state.session,
            GameplayStateError::DuelGameStateMismatch
        );
        require!(duel_entry.seed == seed, GameplayStateError::DuelSeedMismatch);
        require!(!duel_entry.finalized, GameplayStateError::DuelAlreadyQueued);

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
        };

        emit!(DuelRunFinalized {
            seed,
            player: player_key,
            completed_week3: game_state.completed,
            final_week: game_state.week,
        });

        if let Some(creator) = duel_entry.matched_creator {
            let creator_inventory = snapshot_creator_inventory(creator);
            if duel_entry.outcome == DuelRunOutcome::CompletedWeek3 {
                let opponent_inventory = snapshot_duel_entry_inventory(duel_entry);
                let creator_stats = calculate_stats(&creator_inventory);
                let opponent_stats = calculate_stats(&opponent_inventory);
                let creator_effects = generate_combat_effects(&creator_inventory);
                let opponent_effects = generate_combat_effects(&opponent_inventory);
                let combat_outcome = resolve_combat_with_both_gold(
                    build_full_hp_combatant(&creator_stats),
                    build_full_hp_combatant(&opponent_stats),
                    creator_effects,
                    opponent_effects,
                    creator.loadout.gold_at_battle_start,
                    duel_entry.loadout.gold_at_battle_start,
                )?;

                emit!(DuelCombatVisual {
                    seed,
                    player_a: creator.player,
                    player_b: player_key,
                    player_a_tool: creator_inventory.tool,
                    player_a_gear: creator_inventory.gear,
                    player_b_tool: opponent_inventory.tool,
                    player_b_gear: opponent_inventory.gear,
                    combat_log: combat_outcome.log.clone(),
                    player_a_won: combat_outcome.player_won,
                    final_player_a_hp: combat_outcome.final_player_hp,
                    final_player_b_hp: combat_outcome.final_enemy_hp,
                    turns_taken: combat_outcome.turns_taken,
                });

                let total_pot = creator
                    .entry_lamports
                    .checked_add(duel_entry.entry_lamports)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;
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
                    let creator_wallet =
                        resolve_wallet_account(ctx.accounts.creator_wallet.as_ref(), creator.player)?;
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
                    &ctx.accounts.gauntlet_sink.to_account_info(),
                    gauntlet_fee,
                )?;

                emit!(DuelResolved {
                    seed,
                    player_a: creator.player,
                    player_b: Some(player_key),
                    winner: Some(winner_key),
                    total_pot,
                    winner_payout,
                    company_fee,
                    gauntlet_fee,
                    resolution: DuelResolution::CompletedCombat,
                    turns_taken: Some(combat_outcome.turns_taken),
                });
            } else {
                let total_pot = creator
                    .entry_lamports
                    .checked_add(duel_entry.entry_lamports)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;
                let creator_wallet =
                    resolve_wallet_account(ctx.accounts.creator_wallet.as_ref(), creator.player)?;
                transfer_lamports_from_vault(
                    &ctx.accounts.duel_vault.to_account_info(),
                    &creator_wallet,
                    total_pot,
                )?;
                emit!(DuelResolved {
                    seed,
                    player_a: creator.player,
                    player_b: Some(player_key),
                    winner: Some(creator.player),
                    total_pot,
                    winner_payout: total_pot,
                    company_fee: 0,
                    gauntlet_fee: 0,
                    resolution: DuelResolution::OpponentEliminated,
                    turns_taken: None,
                });
            }

            duel_entry.settled = true;
            return Ok(());
        }

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

        let (company_fee, gauntlet_fee) =
            compute_eliminated_unmatched_distribution(duel_entry.entry_lamports)?;
        transfer_lamports_from_vault(
            &ctx.accounts.duel_vault.to_account_info(),
            &ctx.accounts.company_treasury.to_account_info(),
            company_fee,
        )?;
        transfer_lamports_from_vault(
            &ctx.accounts.duel_vault.to_account_info(),
            &ctx.accounts.gauntlet_sink.to_account_info(),
            gauntlet_fee,
        )?;

        emit!(DuelResolved {
            seed,
            player_a: player_key,
            player_b: None,
            winner: None,
            total_pot: duel_entry.entry_lamports,
            winner_payout: 0,
            company_fee,
            gauntlet_fee,
            resolution: DuelResolution::UnmatchedEliminated,
            turns_taken: None,
        });

        duel_entry.settled = true;
        Ok(())
    }

    /// Enters Pit Draft.
    ///
    /// Behavior:
    /// - If queue is empty: player pays stake and becomes waiting player.
    /// - If queue has a waiting player: player pays stake, match resolves immediately,
    ///   winner receives 95% of pot, company gets 3%, gauntlet sink gets 2%.
    pub fn enter_pit_draft(ctx: Context<EnterPitDraft>) -> Result<()> {
        let company_treasury_expected = Pubkey::from_str(COMPANY_TREASURY_ADDRESS)
            .map_err(|_| GameplayStateError::InvalidPitDraftFeeAccount)?;
        let gauntlet_sink_expected = Pubkey::from_str(GAUNTLET_SINK_ADDRESS)
            .map_err(|_| GameplayStateError::InvalidPitDraftFeeAccount)?;

        require_keys_eq!(
            ctx.accounts.company_treasury.key(),
            company_treasury_expected,
            GameplayStateError::InvalidPitDraftFeeAccount
        );
        require_keys_eq!(
            ctx.accounts.gauntlet_sink.key(),
            gauntlet_sink_expected,
            GameplayStateError::InvalidPitDraftFeeAccount
        );

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

        // Queue has waiting player. Resolve immediate match with the entrant.
        let waiting_player = queue
            .waiting_player
            .ok_or(GameplayStateError::PitDraftInvalidWaitingState)?;
        let waiting_profile_key = queue
            .waiting_profile
            .ok_or(GameplayStateError::PitDraftInvalidWaitingState)?;

        require!(
            waiting_player != player_key,
            GameplayStateError::PitDraftSelfMatch
        );

        let waiting_profile = ctx
            .accounts
            .waiting_profile
            .as_ref()
            .ok_or(GameplayStateError::PitDraftMissingWaitingAccounts)?;
        let waiting_player_wallet = ctx
            .accounts
            .waiting_player_wallet
            .as_ref()
            .ok_or(GameplayStateError::PitDraftMissingWaitingAccounts)?;

        require_keys_eq!(
            waiting_profile.key(),
            waiting_profile_key,
            GameplayStateError::PitDraftWaitingAccountMismatch
        );
        require!(
            waiting_profile.owner == waiting_player,
            GameplayStateError::PitDraftWaitingAccountMismatch
        );
        require_keys_eq!(
            waiting_player_wallet.key(),
            waiting_player,
            GameplayStateError::PitDraftWaitingAccountMismatch
        );

        let clock = Clock::get()?;
        let waiting_inventory = build_pit_draft_inventory(
            waiting_player,
            waiting_profile.active_item_pool,
            b"pit_waiting",
            clock.slot,
        )?;
        let entrant_inventory = build_pit_draft_inventory(
            player_key,
            ctx.accounts.player_profile.active_item_pool,
            b"pit_entrant",
            clock.slot,
        )?;

        let waiting_stats = calculate_stats(&waiting_inventory);
        let entrant_stats = calculate_stats(&entrant_inventory);
        let waiting_effects = generate_combat_effects(&waiting_inventory);
        let entrant_effects = generate_combat_effects(&entrant_inventory);

        let waiting_start_gold = (derive_u64_random(&[
            b"pit_waiting_gold",
            waiting_player.as_ref(),
            player_key.as_ref(),
            &clock.slot.to_le_bytes(),
        ]) % (u64::from(PIT_DRAFT_MAX_START_GOLD) + 1)) as u16;
        let entrant_start_gold = (derive_u64_random(&[
            b"pit_entrant_gold",
            player_key.as_ref(),
            waiting_player.as_ref(),
            &clock.slot.to_le_bytes(),
        ]) % (u64::from(PIT_DRAFT_MAX_START_GOLD) + 1)) as u16;

        let combat_outcome = resolve_combat_with_both_gold(
            build_full_hp_combatant(&waiting_stats),
            build_full_hp_combatant(&entrant_stats),
            waiting_effects,
            entrant_effects,
            waiting_start_gold,
            entrant_start_gold,
        )?;

        emit!(PitDraftCombatVisual {
            player_a: waiting_player,
            player_b: player_key,
            player_a_tool: waiting_inventory.tool,
            player_a_gear: waiting_inventory.gear,
            player_b_tool: entrant_inventory.tool,
            player_b_gear: entrant_inventory.gear,
            combat_log: combat_outcome.log.clone(),
            player_a_won: combat_outcome.player_won,
            final_player_a_hp: combat_outcome.final_player_hp,
            final_player_b_hp: combat_outcome.final_enemy_hp,
            turns_taken: combat_outcome.turns_taken,
        });

        let total_pot = PIT_DRAFT_ENTRY_LAMPORTS
            .checked_mul(2)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
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
            &ctx.accounts.company_treasury.to_account_info(),
            company_fee,
        )?;
        transfer_lamports_from_vault(
            &ctx.accounts.pit_draft_vault.to_account_info(),
            &ctx.accounts.gauntlet_sink.to_account_info(),
            gauntlet_fee,
        )?;

        // Clear queue after match resolution.
        queue.waiting_player = None;
        queue.waiting_profile = None;

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

    /// Closes the GameState account via burner wallet authorization.
    /// Used by session-manager CPI during end_session to clean up game state.
    /// Rent is returned to the player wallet.
    pub fn close_game_state_via_burner(ctx: Context<CloseGameStateViaBurner>) -> Result<()> {
        let game_state = &ctx.accounts.game_state;

        emit!(GameStateClosed {
            player: game_state.player,
            total_moves: game_state.total_moves,
            final_phase: game_state.phase,
            final_week: game_state.week,
        });

        Ok(())
    }

    /// Closes the MapEnemies account via burner wallet authorization.
    /// Used by session-manager CPI during end_session to clean up.
    /// Rent is returned to the player wallet.
    pub fn close_map_enemies(ctx: Context<CloseMapEnemies>) -> Result<()> {
        emit!(MapEnemiesClosed {
            session: ctx.accounts.map_enemies.session,
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
        let player_stats = calculate_stats(inventory);

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

            if game_state.run_mode != RunMode::Gauntlet {
                // Resolve boss fight inline (same as move_player does)
                let player_won = resolve_boss_fight(
                    game_state,
                    ctx.accounts.generated_map.seed,
                    inventory,
                    inventory_info,
                    &ctx.accounts.gameplay_authority,
                    player_inventory_program,
                    ctx.bumps.gameplay_authority,
                )?;

                if !player_won {
                    return Ok(());
                }
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
        require!(new_max_hp >= BASE_HP, GameplayStateError::InvalidHpBonus);

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

        let from_x = game_state.position_x;
        let from_y = game_state.position_y;
        game_state.position_x = target_x;
        game_state.position_y = target_y;

        emit!(PositionSetAuthorized {
            player: game_state.player,
            from_x,
            from_y,
            to_x: target_x,
            to_y: target_y,
        });

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
        let map_enemies = &mut ctx.accounts.map_enemies;
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

        let is_night_move = game_state.phase.is_night();
        let visibility_radius = if is_night_move {
            NIGHT_VISION_RADIUS
        } else {
            DAY_VISION_RADIUS
        };
        let is_wall = !generated_map.is_walkable(target_x, target_y);
        let player_stats = calculate_stats(inventory);
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

        let is_last_move_of_week =
            game_state.phase.is_night3() && game_state.moves_remaining == move_cost;
        let from_x = game_state.position_x;
        let from_y = game_state.position_y;

        let mut enemies_moved: u8 = 0;
        let mut combat_triggered = false;

        if map_enemies.enemies.iter().any(|enemy| enemy.defeated) {
            map_enemies.enemies.retain(|enemy| !enemy.defeated);
            map_enemies.count = map_enemies.enemies.len() as u8;
        }

        let map_width = generated_map.width as usize;
        let map_height = generated_map.height as usize;
        let mut occupied = vec![false; map_width.saturating_mul(map_height)];
        for enemy in map_enemies.enemies.iter() {
            let index = (enemy.y as usize) * map_width + (enemy.x as usize);
            if index < occupied.len() {
                occupied[index] = true;
            }
        }

        let mut player_tile_blocked = false;

        let target_enemy_exists_before_move =
            find_enemy_index(map_enemies, target_x, target_y).is_some();

        // Night phase: enemies within 3 tiles (Chebyshev distance) move toward player.
        // Skip enemy movement if player is directly engaging an enemy on target tile.
        if should_process_night_enemy_movement(&game_state.phase, target_enemy_exists_before_move) {
            let player_x = game_state.position_x;
            let player_y = game_state.position_y;
            let mut enemy_idx = 0usize;

            while enemy_idx < map_enemies.enemies.len() {
                let enemy = map_enemies.enemies[enemy_idx];
                let distance = chebyshev_distance(enemy.x, enemy.y, player_x, player_y);
                if distance > 0 && distance <= 3 {
                    let old_x = enemy.x;
                    let old_y = enemy.y;

                    if let Some((new_x, new_y)) = select_enemy_step(
                        enemy.x,
                        enemy.y,
                        player_x,
                        player_y,
                        generated_map,
                        &occupied,
                        map_width,
                        player_tile_blocked,
                    ) {
                        let old_index = (old_y as usize) * map_width + (old_x as usize);
                        if old_index < occupied.len() {
                            occupied[old_index] = false;
                        }

                        if new_x == player_x && new_y == player_y {
                            player_tile_blocked = true;
                        } else {
                            let new_index = (new_y as usize) * map_width + (new_x as usize);
                            if new_index < occupied.len() {
                                occupied[new_index] = true;
                            }
                        }

                        map_enemies.enemies[enemy_idx].x = new_x;
                        map_enemies.enemies[enemy_idx].y = new_y;
                        enemies_moved = enemies_moved.saturating_add(1);

                        emit!(EnemyMoved {
                            enemy_index: enemy_idx as u8,
                            from_x: old_x,
                            from_y: old_y,
                            to_x: new_x,
                            to_y: new_y,
                        });

                        if new_x == player_x && new_y == player_y {
                            combat_triggered = true;
                            let player_won = resolve_enemy_combat(
                                game_state,
                                inventory,
                                map_enemies,
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
            )?;
        }

        game_state.position_x = target_x;
        game_state.position_y = target_y;
        discover_visible_waypoints_cpi(
            &ctx.accounts.map_pois,
            &game_state.to_account_info(),
            &ctx.accounts.player.to_account_info(),
            &ctx.accounts.poi_system_program.to_account_info(),
            visibility_radius,
        )?;

        // Handle move cost consumption, potentially spanning phases
        if needs_phase_span {
            // Consume all moves from current phase
            let moves_from_current = game_state.moves_remaining;
            let remaining_cost = move_cost - moves_from_current;

            // Advance to next phase
            let next_phase = game_state.phase.next().unwrap();
            game_state.phase = next_phase;
            game_state.moves_remaining = next_phase
                .moves_allowed()
                .checked_sub(remaining_cost)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;

            emit!(PhaseAdvanced {
                player: game_state.player,
                new_phase: next_phase,
                new_week: game_state.week,
                moves_remaining: game_state.moves_remaining,
            });
        } else {
            // Simple subtraction within same phase
            game_state.moves_remaining = game_state
                .moves_remaining
                .checked_sub(move_cost)
                .ok_or(GameplayStateError::ArithmeticOverflow)?;
        }

        game_state.total_moves = game_state
            .total_moves
            .checked_add(1)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        let target_enemy_idx = find_enemy_index(map_enemies, target_x, target_y);

        if should_process_target_enemy_combat(
            combat_triggered,
            is_last_move_of_week,
            target_enemy_idx.is_some(),
        ) {
            let enemy_idx = target_enemy_idx.expect("checked is_some above");
            combat_triggered = true;
            let player_won = resolve_enemy_combat(game_state, inventory, map_enemies, enemy_idx)?;
            if !player_won {
                return Ok(());
            }
        } else {
            combat_triggered = combat_triggered || is_last_move_of_week;
        }

        emit!(PlayerMoved {
            player: game_state.player,
            from_x,
            from_y,
            to_x: target_x,
            to_y: target_y,
            moves_remaining: game_state.moves_remaining,
            is_dig: is_wall,
            combat_triggered,
            enemies_moved,
        });

        if game_state.moves_remaining == 0 {
            if game_state.phase.is_night3() {
                game_state.boss_fight_ready = true;

                emit!(BossFightReady {
                    player: game_state.player,
                    week: game_state.week,
                });

                if game_state.run_mode != RunMode::Gauntlet {
                    let player_won = resolve_boss_fight(
                        game_state,
                        ctx.accounts.generated_map.seed,
                        inventory,
                        inventory_info,
                        &ctx.accounts.gameplay_authority,
                        player_inventory_program,
                        ctx.bumps.gameplay_authority,
                    )?;
                    if !player_won {
                        return Ok(());
                    }
                }

                if let Some(enemy_idx) =
                    find_enemy_index(map_enemies, game_state.position_x, game_state.position_y)
                {
                    let player_won =
                        resolve_enemy_combat(game_state, inventory, map_enemies, enemy_idx)?;
                    if !player_won {
                        return Ok(());
                    }
                }
            } else {
                handle_phase_advancement(game_state)?;
            }
        }

        map_enemies.count = map_enemies.enemies.len() as u8;

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
        let map_enemies = &mut ctx.accounts.map_enemies;
        let inventory = &ctx.accounts.inventory;
        let inventory_info = &ctx.accounts.inventory.to_account_info();
        let _player = &ctx.accounts.player;
        let player_inventory_program = &ctx.accounts.player_inventory_program;

        require!(!game_state.is_dead, GameplayStateError::PlayerDead);
        require!(
            game_state.boss_fight_ready,
            GameplayStateError::BossFightNotReady
        );
        require!(
            game_state.run_mode != RunMode::Gauntlet,
            GameplayStateError::GauntletRunNotActive
        );

        let player_won = resolve_boss_fight(
            game_state,
            ctx.accounts.generated_map.seed,
            inventory,
            inventory_info,
            &ctx.accounts.gameplay_authority,
            player_inventory_program,
            ctx.bumps.gameplay_authority,
        )?;
        if !player_won {
            return Ok(());
        }

        if let Some(enemy_idx) =
            find_enemy_index(map_enemies, game_state.position_x, game_state.position_y)
        {
            let player_won = resolve_enemy_combat(game_state, inventory, map_enemies, enemy_idx)?;
            if !player_won {
                return Ok(());
            }
        }

        map_enemies.count = map_enemies.enemies.len() as u8;

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
}

#[derive(Accounts)]
pub struct SetPhaseForTesting<'info> {
    #[account(
        mut,
        has_one = burner_wallet,
    )]
    pub game_state: Account<'info, GameState>,
    pub burner_wallet: Signer<'info>,
}

fn find_enemy_index(map_enemies: &MapEnemies, x: u8, y: u8) -> Option<usize> {
    map_enemies
        .enemies
        .iter()
        .position(|enemy| !enemy.defeated && enemy.x == x && enemy.y == y)
}

fn remove_enemy(map_enemies: &mut MapEnemies, enemy_index: usize) {
    if enemy_index >= map_enemies.enemies.len() {
        return;
    }
    map_enemies.enemies.swap_remove(enemy_index);
    map_enemies.count = map_enemies.enemies.len() as u8;
}

fn build_player_combatant(
    current_hp: i16,
    stats: &PlayerStats,
    _player_effects: &[ItemEffect],
) -> CombatantInput {
    // current_hp is clamped to stats.max_hp to prevent exceeding derived max.
    let combat_hp = current_hp.clamp(1, stats.max_hp);

    // Combat stats (ATK/ARM/SPD) start at BASE values (0).
    // BattleStart effects from items will be applied during combat's BattleStart phase.
    // This prevents double-counting that would occur if we pre-calculated these stats.
    //
    // Pre-calculated stats:
    // - max_hp: Includes permanent HP bonuses (e.g., Work Vest's +HP)
    // - dig: Used for movement cost AND combat comparators (e.g., "if DIG > enemy DIG")
    // - strikes: Base 1 + GainStrikes bonuses (e.g., Twin Picks, Pneumatic Drill)
    CombatantInput {
        hp: combat_hp,
        max_hp: stats.max_hp as u16,
        atk: BASE_ATK,
        arm: BASE_ARM,
        spd: BASE_SPD,
        dig: stats.dig,
        strikes: stats.strikes,
    }
}

fn build_full_hp_combatant(stats: &PlayerStats) -> CombatantInput {
    CombatantInput {
        hp: stats.max_hp,
        max_hp: stats.max_hp as u16,
        atk: BASE_ATK,
        arm: BASE_ARM,
        spd: BASE_SPD,
        dig: stats.dig,
        strikes: stats.strikes,
    }
}

fn is_pool_item_enabled(pool: &[u8; 10], index: usize) -> bool {
    if index >= 80 {
        return false;
    }
    let byte_index = index / 8;
    let bit_index = index % 8;
    (pool[byte_index] & (1u8 << bit_index)) != 0
}

fn derive_u64_random(seeds: &[&[u8]]) -> u64 {
    // Lightweight deterministic mixer for on-chain pseudo-random selection.
    let mut acc: u64 = 0xcbf2_9ce4_8422_2325;
    for seed in seeds {
        for byte in *seed {
            acc ^= *byte as u64;
            acc = acc.wrapping_mul(0x1000_0000_01b3);
        }
    }
    acc ^= acc >> 30;
    acc = acc.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    acc ^= acc >> 27;
    acc = acc.wrapping_mul(0x94d0_49bb_1331_11eb);
    acc ^ (acc >> 31)
}

fn draw_unique_indices(
    candidates: &mut Vec<usize>,
    picks: usize,
    seed_tag: &[u8],
    player: &Pubkey,
    slot: u64,
) -> Vec<usize> {
    let mut selected = Vec::with_capacity(picks);
    for i in 0..picks {
        let rand = derive_u64_random(&[
            seed_tag,
            player.as_ref(),
            &slot.to_le_bytes(),
            &(i as u64).to_le_bytes(),
            &(candidates.len() as u64).to_le_bytes(),
        ]);
        let idx = (rand % candidates.len() as u64) as usize;
        selected.push(candidates.swap_remove(idx));
    }
    selected
}

fn build_pit_draft_inventory(
    player: Pubkey,
    active_pool: [u8; 10],
    seed_tag: &[u8],
    slot: u64,
) -> Result<PlayerInventory> {
    let mut tool_candidates = Vec::new();
    let mut gear_candidates = Vec::new();

    for (index, item_def) in ITEMS.iter().enumerate() {
        if !is_pool_item_enabled(&active_pool, index) {
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

    let selected_tool_idx =
        draw_unique_indices(&mut tool_candidates, 1, seed_tag, &player, slot)[0];
    let selected_gear_indices =
        draw_unique_indices(&mut gear_candidates, 7, seed_tag, &player, slot);

    let mut tool = ItemInstance::new(*ITEMS[selected_tool_idx].id, Tier::I);
    let oil_rand =
        derive_u64_random(&[seed_tag, b"tool_oil", player.as_ref(), &slot.to_le_bytes()]);
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

    for effect in effects.iter() {
        if effect.trigger != combat_system::TriggerType::Victory {
            continue;
        }

        match effect.effect_type {
            EffectType::GainGold => {
                let gold_gain = effect.value.max(0) as u16;
                game_state.gold = game_state.gold.saturating_add(gold_gain);
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
fn preprocess_enemy_effects(archetype_id: u8, player_gold: u16) -> Vec<ItemEffect> {
    let base_effects = field_enemies::traits::get_enemy_traits(archetype_id);

    // Coin Slug: armor = min(player_gold / 10, 3)
    if archetype_id == field_enemies::archetypes::ids::COIN_SLUG {
        let armor_from_gold = ((player_gold / 10) as i16).min(3);
        return base_effects
            .iter()
            .map(|effect| {
                if matches!(effect.effect_type, EffectType::GainArmor) {
                    ItemEffect {
                        value: armor_from_gold,
                        ..*effect
                    }
                } else {
                    *effect
                }
            })
            .collect();
    }

    base_effects.to_vec()
}

fn resolve_enemy_combat(
    game_state: &mut GameState,
    inventory: &PlayerInventory,
    map_enemies: &mut MapEnemies,
    enemy_index: usize,
) -> Result<bool> {
    let enemy = map_enemies.enemies[enemy_index];
    let enemy_input = match field_enemies::archetypes::get_enemy_combatant_input(
        enemy.archetype_id,
        enemy.tier,
    ) {
        Some(input) => input,
        None => return Ok(true),
    };

    let player_stats = calculate_stats(inventory);
    let player_effects = generate_combat_effects(inventory);
    let player_input = build_player_combatant(game_state.hp, &player_stats, &player_effects);
    let enemy_effects = preprocess_enemy_effects(enemy.archetype_id, game_state.gold);

    emit!(CombatStarted {
        player: game_state.player,
        player_hp: game_state.hp,
        player_atk: BASE_ATK, // ATK bonuses applied during combat's BattleStart phase
        enemy_archetype: enemy.archetype_id,
        enemy_hp: enemy_input.hp,
        enemy_atk: enemy_input.atk,
    });

    let result = resolve_combat_with_player_gold(
        player_input,
        enemy_input,
        player_effects,
        enemy_effects,
        game_state.gold,
    )?;

    let tier_enum = field_enemies::state::EnemyTier::from_u8(enemy.tier);
    require!(tier_enum.is_some(), GameplayStateError::InvalidEnemyTier);
    let gold_reward = tier_enum.unwrap().gold_reward() as u16;

    emit!(CombatEnded {
        player: game_state.player,
        player_won: result.player_won,
        final_player_hp: result.final_player_hp,
        final_enemy_hp: result.final_enemy_hp,
        gold_earned: if result.player_won { gold_reward } else { 0 },
        turns_taken: result.turns_taken,
    });

    emit!(CombatLog {
        player: game_state.player,
        entries: result.log,
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
        remove_enemy(map_enemies, enemy_index);
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

fn resolve_boss_fight<'info>(
    game_state: &mut GameState,
    map_seed: u64,
    inventory: &PlayerInventory,
    inventory_info: &AccountInfo<'info>,
    gameplay_authority: &AccountInfo<'info>,
    player_inventory_program: &AccountInfo<'info>,
    gameplay_authority_bump: u8,
) -> Result<bool> {
    let stage = game_state.campaign_level;
    let (boss_input, boss_id) = if game_state.run_mode == RunMode::Duel {
        (
            get_duel_boss_for_combat(map_seed, game_state.week)?,
            get_duel_boss_id(map_seed, game_state.week)?,
        )
    } else {
        (
            get_boss_for_combat(stage, game_state.week)?,
            get_boss_id(stage, game_state.week)?,
        )
    };
    let boss_definition = boss_system::get_boss(&boss_id).ok_or(GameplayStateError::InvalidWeek)?;
    let boss_effects = boss_system::get_boss_item_effects(boss_definition);

    let player_stats = calculate_stats(inventory);
    let player_effects = generate_combat_effects(inventory);
    let player_input = build_player_combatant(game_state.hp, &player_stats, &player_effects);

    emit!(BossCombatStarted {
        player: game_state.player,
        boss_id,
        boss_hp: boss_input.hp,
        week: game_state.week,
    });

    let result = resolve_combat_with_player_gold(
        player_input,
        boss_input,
        player_effects,
        boss_effects,
        game_state.gold,
    )?;

    emit!(CombatEnded {
        player: game_state.player,
        player_won: result.player_won,
        final_player_hp: result.final_player_hp,
        final_enemy_hp: result.final_enemy_hp,
        gold_earned: 0,
        turns_taken: result.turns_taken,
    });

    emit!(CombatLog {
        player: game_state.player,
        entries: result.log,
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
) -> Result<()> {
    let signer_seeds: &[&[&[u8]]] = &[&[GAMEPLAY_AUTHORITY_SEED, &[gameplay_authority_bump]]];

    map_generator::cpi::set_tile_floor(
        CpiContext::new_with_signer(
            map_generator_program.clone(),
            map_generator::cpi::accounts::SetTileFloor {
                generated_map: generated_map.clone(),
                session: session.clone(),
                gameplay_authority: gameplay_authority.clone(),
            },
            signer_seeds,
        ),
        x,
        y,
    )?;

    Ok(())
}

fn discover_visible_waypoints_cpi<'info>(
    map_pois: &AccountInfo<'info>,
    game_state: &AccountInfo<'info>,
    player: &AccountInfo<'info>,
    poi_system_program: &AccountInfo<'info>,
    visibility_radius: u8,
) -> Result<()> {
    let mut data = [0u8; 9];
    data[..8].copy_from_slice(&DISCOVER_VISIBLE_WAYPOINTS_DISCRIMINATOR);
    data[8] = visibility_radius;

    let instruction = Instruction {
        program_id: POI_SYSTEM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(map_pois.key(), false),
            AccountMeta::new_readonly(game_state.key(), false),
            AccountMeta::new_readonly(player.key(), true),
        ],
        data: data.to_vec(),
    };

    invoke(
        &instruction,
        &[
            map_pois.clone(),
            game_state.clone(),
            player.clone(),
            poi_system_program.clone(),
        ],
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

fn build_bootstrap_echo(week: u8, index: u64) -> Result<GauntletEchoSnapshot> {
    let tool_count = count_items_by_type(ItemType::Tool);
    let gear_count = count_items_by_type(ItemType::Gear);
    require!(tool_count > 0, GameplayStateError::GauntletNotInitialized);
    require!(gear_count > 0, GameplayStateError::GauntletNotInitialized);

    let tool_idx = item_index_by_type(ItemType::Tool, (index as usize) % tool_count)
        .ok_or(GameplayStateError::GauntletNotInitialized)?;
    let mut gear = [None; 12];
    for (slot, slot_item) in gear.iter_mut().enumerate() {
        let item_idx = item_index_by_type(
            ItemType::Gear,
            ((index as usize) + (slot * week as usize)) % gear_count,
        )
        .ok_or(GameplayStateError::GauntletNotInitialized)?;
        *slot_item = Some(ItemInstance::new(*ITEMS[item_idx].id, Tier::I));
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
    ITEMS.iter().filter(|def| def.item_type == item_type).count()
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
    require!(score.player == *player, GameplayStateError::GauntletScoreMismatch);
    require!(score.epoch_id == epoch_id, GameplayStateError::GauntletScoreMismatch);
    score.points = score
        .points
        .checked_add(add_points)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    Ok(())
}

fn maybe_award_defender_points(
    echo: &GauntletEchoSnapshot,
    week: u8,
    epoch_pool: &mut Account<GauntletEpochPool>,
) -> Result<()> {
    let defender = match echo.source {
        GauntletEchoSource::Bootstrap => return Ok(()),
        GauntletEchoSource::Player(p) => p,
    };

    let pts = gauntlet_defender_points(week);
    add_pending_defender_points(epoch_pool, defender, pts)?;
    epoch_pool.total_points = epoch_pool
        .total_points
        .checked_add(pts)
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

fn take_pending_defender_points(epoch_pool: &mut Account<GauntletEpochPool>, player: Pubkey) -> u64 {
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

fn maybe_insert_player_echo(
    week_pool: &mut Account<GauntletWeekPool>,
    week: u8,
    inventory: &PlayerInventory,
    gold: u16,
    player: Pubkey,
) -> Result<()> {
    let snapshot = GauntletEchoSnapshot {
        week,
        source: GauntletEchoSource::Player(player),
        loadout: GauntletLoadoutSnapshot {
            tool: inventory.tool,
            gear: inventory.gear,
            gold_at_battle_start: gold,
        },
    };

    week_pool.seen_player_echoes = week_pool
        .seen_player_echoes
        .checked_add(1)
        .ok_or(GameplayStateError::ArithmeticOverflow)?;
    week_pool.player_echoes_added = week_pool
        .player_echoes_added
        .saturating_add(1);

    if week_pool.entries.len() < GAUNTLET_MAX_WEEKLY_ECHOES {
        week_pool.entries.push(snapshot);
    } else {
        let rand = derive_u64_random(&[
            b"gauntlet_reservoir",
            &week.to_le_bytes(),
            &week_pool.seen_player_echoes.to_le_bytes(),
            player.as_ref(),
        ]);
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

#[derive(Accounts)]
pub struct InitializeGameState<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + GameState::INIT_SPACE,
        seeds = [GAME_STATE_SEED, game_session.key().as_ref()],
        bump
    )]
    pub game_state: Account<'info, GameState>,

    /// The linked GameSession PDA (must exist)
    /// CHECK: We only verify this account exists as validation of the session
    pub game_session: AccountInfo<'info>,

    /// Generated map for seeding enemies
    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_session.key().as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

    /// Enemy instances seeded from generated map
    #[account(
        init,
        payer = player,
        space = 8 + MapEnemies::INIT_SPACE,
        seeds = [MapEnemies::SEED_PREFIX, game_session.key().as_ref()],
        bump
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    #[account(mut)]
    pub player: Signer<'info>,

    /// CHECK: Burner wallet whose pubkey is stored in game_state.burner_wallet
    /// for authorizing gameplay transactions (move, boss fight).
    pub burner_wallet: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
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

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
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
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,
    pub player: Signer<'info>,
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
pub struct EnterGauntlet<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

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

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(epoch_id: u64)]
pub struct ResolveGauntletWeek<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    /// CHECK: Gameplay authority PDA signer for authorized inventory slot expansion CPI.
    pub gameplay_authority: AccountInfo<'info>,

    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    #[account(
        mut,
        seeds = [GAUNTLET_CONFIG_SEED],
        bump = gauntlet_config.bump
    )]
    pub gauntlet_config: Account<'info, GauntletConfig>,

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

    pub system_program: Program<'info, System>,
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
#[instruction(seed: u64)]
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
        seeds = [DUEL_OPEN_QUEUE_SEED],
        bump = duel_open_queue.bump
    )]
    pub duel_open_queue: Box<Account<'info, DuelOpenQueue>>,

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
    pub game_state: Account<'info, GameState>,

    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

    #[account(mut)]
    pub company_treasury: SystemAccount<'info>,

    #[account(mut)]
    pub gauntlet_sink: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct FinalizeDuelRun<'info> {
    #[account(
        mut,
        seeds = [DUEL_ENTRY_SEED, game_state.session.as_ref()],
        bump = duel_entry.bump,
    )]
    pub duel_entry: Box<Account<'info, DuelEntry>>,

    #[account(
        mut,
        seeds = [DUEL_OPEN_QUEUE_SEED],
        bump = duel_open_queue.bump
    )]
    pub duel_open_queue: Box<Account<'info, DuelOpenQueue>>,

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
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

    /// Wallet for matched creator payout when current player loses.
    #[account(mut)]
    pub creator_wallet: Option<SystemAccount<'info>>,

    #[account(mut)]
    pub company_treasury: SystemAccount<'info>,

    #[account(mut)]
    pub gauntlet_sink: SystemAccount<'info>,
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

    /// Waiting player's profile, required when queue is occupied.
    #[account(mut)]
    pub waiting_profile: Option<Account<'info, PlayerProfile>>,

    /// Waiting player's main wallet, required when queue is occupied.
    #[account(mut)]
    pub waiting_player_wallet: Option<SystemAccount<'info>>,

    #[account(mut)]
    pub company_treasury: SystemAccount<'info>,

    #[account(mut)]
    pub gauntlet_sink: SystemAccount<'info>,

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

/// Context for closing GameState via burner wallet (for session-manager CPI).
#[derive(Accounts)]
pub struct CloseGameStateViaBurner<'info> {
    #[account(
        mut,
        has_one = burner_wallet @ GameplayStateError::Unauthorized,
        close = player,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player wallet receives the rent refund (not a signer)
    /// CHECK: Validated by game_state.player via close constraint
    #[account(mut)]
    pub player: AccountInfo<'info>,

    /// Burner wallet must sign to authorize closure
    pub burner_wallet: Signer<'info>,
}

/// Context for closing MapEnemies account via burner wallet.
#[derive(Accounts)]
pub struct CloseMapEnemies<'info> {
    #[account(
        mut,
        seeds = [MapEnemies::SEED_PREFIX, game_state.session.as_ref()],
        bump = map_enemies.bump,
        constraint = map_enemies.session == game_state.session @ GameplayStateError::InvalidSession,
        close = player,
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    /// GameState to verify burner_wallet authorization
    #[account(
        has_one = burner_wallet @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    /// Player wallet receives the rent refund (not a signer)
    /// CHECK: Validated via game_state.player
    #[account(mut, address = game_state.player @ GameplayStateError::Unauthorized)]
    pub player: AccountInfo<'info>,

    /// Burner wallet must sign to authorize closure
    pub burner_wallet: Signer<'info>,
}

/// Context for healing the player, authorized by poi-system CPI.
/// Requires poi_authority PDA from poi-system as signer.
/// Includes inventory for deriving max_hp.
#[derive(Accounts)]
pub struct HealPlayer<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,

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
    pub game_state: Account<'info, GameState>,

    /// Player inventory for stats calculation and boss fight resolution
    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

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
        constraint = game_state.burner_wallet == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession
    )]
    /// CHECK: Validated by game_state.session match.
    pub game_session: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [MapEnemies::SEED_PREFIX, game_state.session.as_ref()],
        bump = map_enemies.bump,
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    #[account(
        mut,
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

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

    #[account(mut)]
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct TriggerBossFight<'info> {
    #[account(
        mut,
        constraint = game_state.burner_wallet == player.key() @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    #[account(
        mut,
        constraint = game_state.session == game_session.key() @ GameplayStateError::InvalidSession
    )]
    /// CHECK: Validated by game_state.session match.
    pub game_session: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [MapEnemies::SEED_PREFIX, game_state.session.as_ref()],
        bump = map_enemies.bump,
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    #[account(
        seeds = [map_generator::state::GeneratedMap::SEED_PREFIX, game_state.session.as_ref()],
        bump = generated_map.bump,
        seeds::program = map_generator::ID,
    )]
    pub generated_map: Account<'info, map_generator::state::GeneratedMap>,

    #[account(
        mut,
        seeds = [b"inventory", game_state.session.as_ref()],
        bump = inventory.bump,
        seeds::program = player_inventory::ID,
    )]
    pub inventory: Account<'info, PlayerInventory>,

    /// Gameplay authority PDA for signing CPI calls to player-inventory
    /// CHECK: This is a PDA derived from gameplay_state program, validated by seeds
    #[account(
        seeds = [GAMEPLAY_AUTHORITY_SEED],
        bump,
    )]
    pub gameplay_authority: AccountInfo<'info>,

    /// Player inventory program for CPI (expand gear slots on boss victory)
    pub player_inventory_program: Program<'info, player_inventory::program::PlayerInventory>,

    pub player: Signer<'info>,
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
pub struct PlayerMoved {
    pub player: Pubkey,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
    pub moves_remaining: u8,
    pub is_dig: bool,
    pub combat_triggered: bool,
    pub enemies_moved: u8,
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
pub struct MapEnemiesClosed {
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

/// Emitted when position is updated via authorized CPI from poi-system.
#[event]
pub struct PositionSetAuthorized {
    pub player: Pubkey,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
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

/// Detailed combat log for turn-by-turn visualization.
/// Contains a serialized vector of CombatLogEntry for replay.
/// Note: Solana logs have ~30KB limit; this compact format allows ~300-400 actions per battle.
#[event]
pub struct CombatLog {
    pub player: Pubkey,
    /// Serialized Vec<CombatLogEntry> - each entry is ~5 bytes
    pub entries: Vec<CombatLogEntry>,
}

/// Emitted when an enemy moves during night phase
#[event]
pub struct EnemyMoved {
    pub enemy_index: u8,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
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
    /// Full deterministic combat trace (same semantics as PvE CombatLog entries)
    pub combat_log: Vec<CombatLogEntry>,
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
    pub combat_log: Vec<CombatLogEntry>,
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
    pub combat_log: Vec<CombatLogEntry>,
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

#[cfg(test)]
mod hp_logic_tests {
    use super::*;

    fn make_base_stats() -> PlayerStats {
        PlayerStats {
            max_hp: 10,
            dig: 1,
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
            strikes: 1,
        };

        // Player at full HP
        let current_hp: i16 = 15;
        let effects = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 15, "Combat HP should match current HP");
        assert_eq!(input.max_hp, 15, "Combat max_hp should match derived stats");
        // ATK/ARM/SPD start at base (0) and get bonuses from BattleStart effects
        assert_eq!(input.atk, BASE_ATK, "ATK should be base value");
        assert_eq!(input.arm, BASE_ARM, "ARM should be base value");
        assert_eq!(input.spd, BASE_SPD, "SPD should be base value");

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
            strikes: 1,
        };

        let current_hp: i16 = 15;
        let effects = vec![];

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

        let effects = vec![]; // No battle start bonus

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
        // Note: Only max_hp, dig, and strikes are pre-calculated in PlayerStats.
        // ATK/ARM/SPD start at base values and get bonuses from BattleStart effects.
        let current_hp: i16 = 8;
        let stats = PlayerStats {
            max_hp: 15, // Increased from MaxHp effects (e.g., Work Vest)
            dig: 3,     // From DIG items
            strikes: 2, // From GainStrikes items (e.g., Twin Picks)
        };

        let effects = vec![];

        let input = build_player_combatant(current_hp, &stats, &effects);
        assert_eq!(input.hp, 8);
        assert_eq!(input.max_hp, 15);
        // ATK/ARM/SPD start at base (0) - bonuses applied during BattleStart phase
        assert_eq!(input.atk, BASE_ATK);
        assert_eq!(input.arm, BASE_ARM);
        assert_eq!(input.spd, BASE_SPD);
        assert_eq!(input.dig, 3);
        // Strikes are pre-calculated from GainStrikes effects
        assert_eq!(input.strikes, 2);
    }

    #[test]
    fn test_battlestart_atk_not_double_counted() {
        // Regression test: BattleStart ATK/ARM/SPD bonuses should NOT be pre-calculated.
        // They are applied during combat's BattleStart phase.
        //
        // If this test fails, it means ATK/ARM/SPD is being double-counted:
        // - Once in calculate_stats() -> stats (WRONG - we removed this)
        // - And again in combat's BattleStart phase
        //
        // The fix ensures build_player_combatant() uses base values for ATK/ARM/SPD.

        use combat_system::{EffectType, TriggerType};

        // PlayerStats has max_hp, dig, and strikes
        let stats = PlayerStats {
            max_hp: 15,
            dig: 1,
            strikes: 1,
        };

        // BattleStart ATK effect from an item (e.g., Rime Pike)
        let effects = vec![ItemEffect {
            effect_type: EffectType::GainAtk,
            trigger: TriggerType::BattleStart,
            value: 5,
            once_per_turn: false,
            condition: Condition::None,
        }];

        let current_hp: i16 = 15;
        let input = build_player_combatant(current_hp, &stats, &effects);

        // CORRECT: combat_atk = 0 (base), effect applied during BattleStart phase
        // BUG: combat_atk = 5 (pre-calculated, would be doubled in combat)
        assert_eq!(
            input.atk, BASE_ATK,
            "ATK should be base value, not pre-calculated from BattleStart effects"
        );
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
        assert_eq!(effects[0].value, 0);

        // 9 gold = 0 armor (floor(9/10) = 0)
        let effects = preprocess_enemy_effects(COIN_SLUG, 9);
        assert_eq!(effects[0].value, 0);

        // 10 gold = 1 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 10);
        assert_eq!(effects[0].value, 1);

        // 25 gold = 2 armor
        let effects = preprocess_enemy_effects(COIN_SLUG, 25);
        assert_eq!(effects[0].value, 2);

        // 30 gold = 3 armor (cap)
        let effects = preprocess_enemy_effects(COIN_SLUG, 30);
        assert_eq!(effects[0].value, 3);

        // 100 gold = 3 armor (capped at 3)
        let effects = preprocess_enemy_effects(COIN_SLUG, 100);
        assert_eq!(effects[0].value, 3, "Armor should be capped at 3");

        // Non-Coin Slug enemies should not be affected
        let effects = preprocess_enemy_effects(0, 100); // Tunnel Rat
        assert!(!effects
            .iter()
            .any(|e| { matches!(e.effect_type, EffectType::GainArmor) && e.value == 3 }));
    }

    #[test]
    fn test_find_matching_creator_index_skips_wrong_seed_and_self() {
        let entrant = Pubkey::new_unique();
        let other = Pubkey::new_unique();
        let seed = 42u64;

        let loadout = DuelLoadoutSnapshot {
            tool: None,
            gear: [None; 12],
            gold_at_battle_start: 0,
        };
        let queue = DuelOpenQueue {
            entries: vec![
                DuelCreatorEntry {
                    player: other,
                    seed: 7,
                    entry_lamports: 1,
                    finished_slot: 1,
                    loadout,
                },
                DuelCreatorEntry {
                    player: entrant,
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
            bump: 1,
        };

        let idx = find_matching_creator_index(&queue, entrant, seed);
        assert_eq!(idx, Some(2));
    }

    #[test]
    fn test_find_matching_creator_index_none_when_no_eligible_entry() {
        let entrant = Pubkey::new_unique();
        let seed = 99u64;
        let loadout = DuelLoadoutSnapshot {
            tool: None,
            gear: [None; 12],
            gold_at_battle_start: 0,
        };
        let queue = DuelOpenQueue {
            entries: vec![DuelCreatorEntry {
                player: entrant,
                seed,
                entry_lamports: 1,
                finished_slot: 1,
                loadout,
            }],
            bump: 1,
        };

        let idx = find_matching_creator_index(&queue, entrant, seed);
        assert_eq!(idx, None);
    }
}

#[cfg(test)]
mod combat_scenarios_tests;
