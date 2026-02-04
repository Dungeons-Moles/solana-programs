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

    /// Returns true if this is specifically Night3 (end of week boss phase)
    pub fn is_night3(&self) -> bool {
        matches!(self, Phase::Night3)
    }
}

/// Core gameplay state account linked to a GameSession.
/// Contains all mutable game data for a single run.
/// Stats (ATK, ARM, SPD, DIG, MaxHP) are derived from inventory at runtime.
/// PDA Seeds: ["game_state", session_pda.as_ref()]
#[account]
#[derive(InitSpace)]
pub struct GameState {
    /// Session owner's main wallet (used for lifecycle operations like close)
    pub player: Pubkey,

    /// Burner wallet authorized for gameplay transactions (move, boss fight)
    pub burner_wallet: Pubkey,

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

    /// Current health points (0 <= hp <= derived max_hp)
    pub hp: i16,

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

    /// Campaign level for this session (1-40)
    /// Stored at init to prevent user manipulation of boss stage
    pub campaign_level: u8,

    /// Player death flag - once set, all gameplay instructions are blocked
    pub is_dead: bool,

    /// Level completion flag - set when week 3 boss is defeated
    /// Used by session_manager to validate end_session calls
    pub completed: bool,
}

impl GameState {
    /// Returns the PDA seeds for this GameState
    pub fn seeds(session: &Pubkey) -> [&[u8]; 2] {
        [GAME_STATE_SEED, session.as_ref()]
    }
}

/// A spawned enemy instance on the map
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, InitSpace)]
pub struct EnemyInstance {
    /// References EnemyArchetype ID (0-11)
    pub archetype_id: u8,
    /// Tier: 0=T1, 1=T2, 2=T3
    pub tier: u8,
    /// Map X coordinate
    pub x: u8,
    /// Map Y coordinate
    pub y: u8,
    /// True if already defeated
    pub defeated: bool,
}

/// On-chain account storing all enemy instances for a map
/// PDA Seeds: ["map_enemies", session.as_ref()]
#[account]
#[derive(InitSpace)]
pub struct MapEnemies {
    /// Parent session PDA
    pub session: Pubkey,

    /// Enemy instances (max 48)
    #[max_len(48)]
    pub enemies: Vec<EnemyInstance>,

    /// Actual count of enemies
    pub count: u8,

    /// PDA bump seed
    pub bump: u8,
}

impl MapEnemies {
    /// PDA seed prefix
    pub const SEED_PREFIX: &'static [u8] = b"map_enemies";
}
