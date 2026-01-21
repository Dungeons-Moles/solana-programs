use anchor_lang::prelude::*;

use crate::constants::{DAY_MOVES, GAME_STATE_SEED, NIGHT_MOVES};

/// Time phase enumeration determining move allowance.
/// Each day has 50 moves, each night has 30 moves.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum Phase {
    Day1,
    Night1,
    Day2,
    Night2,
    Day3,
    Night3,
}

impl Phase {
    /// Returns the number of moves allowed for this phase
    pub fn moves_allowed(&self) -> u8 {
        match self {
            Phase::Day1 | Phase::Day2 | Phase::Day3 => DAY_MOVES,
            Phase::Night1 | Phase::Night2 | Phase::Night3 => NIGHT_MOVES,
        }
    }

    /// Returns the next phase, or None if at end of week (Night3)
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Day1 => Some(Phase::Night1),
            Phase::Night1 => Some(Phase::Day2),
            Phase::Day2 => Some(Phase::Night2),
            Phase::Night2 => Some(Phase::Day3),
            Phase::Day3 => Some(Phase::Night3),
            Phase::Night3 => None,
        }
    }

    /// Returns true if this is a night phase
    pub fn is_night(&self) -> bool {
        matches!(self, Phase::Night1 | Phase::Night2 | Phase::Night3)
    }
}

/// Stat type enumeration for modify_stat instruction
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum StatType {
    Hp,
    MaxHp,
    Atk,
    Arm,
    Spd,
    Dig,
}

/// Core gameplay state account linked to a GameSession.
/// Contains all mutable game data for a single run.
/// PDA Seeds: ["game_state", session_pda.as_ref()]
#[account]
#[derive(InitSpace)]
pub struct GameState {
    /// Session owner's wallet
    pub player: Pubkey,

    /// Linked GameSession PDA
    pub session: Pubkey,

    /// Current X coordinate (0 <= x < map_width)
    pub position_x: u8,

    /// Current Y coordinate (0 <= y < map_height)
    pub position_y: u8,

    /// Map boundary X (immutable after init)
    pub map_width: u8,

    /// Map boundary Y (immutable after init)
    pub map_height: u8,

    /// Current health points (0 <= hp <= max_hp)
    pub hp: i8,

    /// Maximum health points
    pub max_hp: u8,

    /// Attack stat (allows negative for debuffs)
    pub atk: i8,

    /// Armor stat (allows negative for debuffs)
    pub arm: i8,

    /// Speed stat (allows negative for debuffs)
    pub spd: i8,

    /// Digging stat (affects wall dig cost)
    pub dig: i8,

    /// Gear slot capacity (4 -> 6 -> 8)
    pub gear_slots: u8,

    /// Current week (1-3)
    pub week: u8,

    /// Current time phase
    pub phase: Phase,

    /// Moves remaining in current phase (0-50)
    pub moves_remaining: u8,

    /// Total moves made across session
    pub total_moves: u32,

    /// Boss fight triggered flag
    pub boss_fight_ready: bool,

    /// Current gold amount (0-65535)
    pub gold: u16,

    /// PDA bump seed
    pub bump: u8,
}

impl GameState {
    /// Returns the PDA seeds for this GameState
    pub fn seeds(session: &Pubkey) -> [&[u8]; 2] {
        [GAME_STATE_SEED, session.as_ref()]
    }
}
