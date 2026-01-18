use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod state;

use constants::*;
use errors::GameplayStateError;
use state::{GameState, Phase, StatType};

declare_id!("5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n");

#[program]
pub mod gameplay_state {
    use super::*;

    /// Initializes a new GameState account linked to an active GameSession.
    pub fn initialize_game_state(
        ctx: Context<InitializeGameState>,
        map_width: u8,
        map_height: u8,
        start_x: u8,
        start_y: u8,
    ) -> Result<()> {
        // Validate starting position is within bounds
        require!(
            start_x < map_width && start_y < map_height,
            GameplayStateError::OutOfBounds
        );

        let game_state = &mut ctx.accounts.game_state;

        // Initialize core fields
        game_state.player = ctx.accounts.player.key();
        game_state.session = ctx.accounts.game_session.key();
        game_state.position_x = start_x;
        game_state.position_y = start_y;
        game_state.map_width = map_width;
        game_state.map_height = map_height;

        // Initialize default stats
        game_state.hp = DEFAULT_HP;
        game_state.max_hp = DEFAULT_MAX_HP;
        game_state.atk = DEFAULT_ATK;
        game_state.arm = DEFAULT_ARM;
        game_state.spd = DEFAULT_SPD;
        game_state.dig = DEFAULT_DIG;

        // Initialize game progression
        game_state.gear_slots = INITIAL_GEAR_SLOTS;
        game_state.week = 1;
        game_state.phase = Phase::Day1;
        game_state.moves_remaining = DAY_MOVES;
        game_state.total_moves = 0;
        game_state.boss_fight_ready = false;
        game_state.bump = ctx.bumps.game_state;

        emit!(GameStateInitialized {
            player: game_state.player,
            session: game_state.session,
            map_width,
            map_height,
        });

        Ok(())
    }

    /// Moves the player to an adjacent tile, deducting move cost.
    pub fn move_player(
        ctx: Context<MovePlayer>,
        target_x: u8,
        target_y: u8,
        is_wall: bool,
    ) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        // Check if boss fight already triggered
        require!(
            !game_state.boss_fight_ready,
            GameplayStateError::BossFightAlreadyTriggered
        );

        // Validate target is within bounds
        require!(
            target_x < game_state.map_width && target_y < game_state.map_height,
            GameplayStateError::OutOfBounds
        );

        // Validate target is adjacent (Manhattan distance = 1)
        let dx = (target_x as i16 - game_state.position_x as i16).abs();
        let dy = (target_y as i16 - game_state.position_y as i16).abs();
        require!(dx + dy == 1, GameplayStateError::NotAdjacent);

        // Calculate move cost
        let move_cost = if is_wall {
            // Wall dig cost: max(2, 6 - DIG)
            let dig_stat = game_state.dig as i16;
            (BASE_DIG_COST as i16 - dig_stat).max(MIN_DIG_COST as i16) as u8
        } else {
            FLOOR_MOVE_COST
        };

        // Check sufficient moves
        require!(
            game_state.moves_remaining >= move_cost,
            GameplayStateError::InsufficientMoves
        );

        // Store old position for event
        let from_x = game_state.position_x;
        let from_y = game_state.position_y;

        // Update position and moves
        game_state.position_x = target_x;
        game_state.position_y = target_y;
        game_state.moves_remaining = game_state
            .moves_remaining
            .checked_sub(move_cost)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;
        game_state.total_moves = game_state
            .total_moves
            .checked_add(1)
            .ok_or(GameplayStateError::ArithmeticOverflow)?;

        emit!(PlayerMoved {
            player: game_state.player,
            from_x,
            from_y,
            to_x: target_x,
            to_y: target_y,
            moves_remaining: game_state.moves_remaining,
            is_dig: is_wall,
        });

        // Handle phase advancement if moves exhausted
        if game_state.moves_remaining == 0 {
            handle_phase_advancement(game_state)?;
        }

        Ok(())
    }

    /// Modifies a player stat by a delta value.
    pub fn modify_stat(ctx: Context<ModifyStat>, stat: StatType, delta: i8) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;

        match stat {
            StatType::Hp => {
                let new_hp = (game_state.hp as i16)
                    .checked_add(delta as i16)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;

                // HP cannot go below 0
                require!(new_hp >= 0, GameplayStateError::HpUnderflow);

                // HP cannot exceed max_hp
                let clamped = new_hp.min(game_state.max_hp as i16) as i8;
                let old_value = game_state.hp;
                game_state.hp = clamped;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: clamped,
                });
            }
            StatType::MaxHp => {
                let new_max_hp = (game_state.max_hp as i16)
                    .checked_add(delta as i16)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;

                require!(
                    new_max_hp >= 0 && new_max_hp <= u8::MAX as i16,
                    GameplayStateError::StatOverflow
                );

                let old_value = game_state.max_hp as i8;
                game_state.max_hp = new_max_hp as u8;

                // Clamp current HP if it exceeds new max
                if game_state.hp > game_state.max_hp as i8 {
                    game_state.hp = game_state.max_hp as i8;
                }

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_max_hp as i8,
                });
            }
            StatType::Atk => {
                let new_atk = game_state
                    .atk
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.atk;
                game_state.atk = new_atk;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_atk,
                });
            }
            StatType::Arm => {
                let new_arm = game_state
                    .arm
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.arm;
                game_state.arm = new_arm;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_arm,
                });
            }
            StatType::Spd => {
                let new_spd = game_state
                    .spd
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.spd;
                game_state.spd = new_spd;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_spd,
                });
            }
            StatType::Dig => {
                let new_dig = game_state
                    .dig
                    .checked_add(delta)
                    .ok_or(GameplayStateError::StatOverflow)?;
                let old_value = game_state.dig;
                game_state.dig = new_dig;

                emit!(StatModified {
                    player: game_state.player,
                    stat,
                    old_value,
                    new_value: new_dig,
                });
            }
        }

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
}

/// Helper function to handle phase advancement when moves are exhausted
fn handle_phase_advancement(game_state: &mut GameState) -> Result<()> {
    match game_state.phase.next() {
        Some(next_phase) => {
            // Advance to next phase
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
            // Night3 complete - handle week transition or boss fight
            if game_state.week >= 3 {
                // Week 3 Night 3 complete - trigger boss fight
                game_state.boss_fight_ready = true;

                emit!(BossFightReady {
                    player: game_state.player,
                    week: game_state.week,
                });
            } else {
                // Advance to next week
                game_state.week = game_state
                    .week
                    .checked_add(1)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?;
                game_state.phase = Phase::Day1;
                game_state.moves_remaining = DAY_MOVES;

                // Increase gear slots (capped at MAX_GEAR_SLOTS)
                game_state.gear_slots = game_state
                    .gear_slots
                    .checked_add(2)
                    .ok_or(GameplayStateError::ArithmeticOverflow)?
                    .min(MAX_GEAR_SLOTS);

                emit!(PhaseAdvanced {
                    player: game_state.player,
                    new_phase: Phase::Day1,
                    new_week: game_state.week,
                    moves_remaining: game_state.moves_remaining,
                });
            }
        }
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

    #[account(mut)]
    pub player: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MovePlayer<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct ModifyStat<'info> {
    #[account(
        mut,
        has_one = player @ GameplayStateError::Unauthorized,
    )]
    pub game_state: Account<'info, GameState>,

    pub player: Signer<'info>,
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
pub struct StatModified {
    pub player: Pubkey,
    pub stat: StatType,
    pub old_value: i8,
    pub new_value: i8,
}

#[event]
pub struct GameStateClosed {
    pub player: Pubkey,
    pub total_moves: u32,
    pub final_phase: Phase,
    pub final_week: u8,
}
