use anchor_lang::prelude::*;
use player_inventory::state::ItemInstance;

use crate::constants::{DAY_MOVES, DUEL_OPEN_QUEUE_CAPACITY, GAME_STATE_SEED, NIGHT_MOVES};

/// Session mode for gameplay progression rules.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum RunMode {
    Campaign,
    Duel,
    Gauntlet,
}

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

    /// Gear slot capacity (4 -> 6 -> 8 -> 10 -> 12)
    pub gear_slots: u8,

    /// Current week (1-max_weeks)
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

    /// Session mode controls week cap and weekly resolver.
    pub run_mode: RunMode,

    /// Maximum number of weeks for this run mode (3 campaign/duel, 5 gauntlet).
    pub max_weeks: u8,

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

/// Global queue for pit draft matchmaking.
/// Stores one waiting player; the next entrant is matched immediately.
#[account]
#[derive(InitSpace)]
pub struct PitDraftQueue {
    /// Waiting player's main wallet.
    pub waiting_player: Option<Pubkey>,
    /// Waiting player's profile account.
    pub waiting_profile: Option<Pubkey>,
    /// PDA bump seed.
    pub bump: u8,
}

/// Vault account holding pit draft stakes.
#[account]
#[derive(InitSpace)]
pub struct PitDraftVault {
    /// PDA bump seed.
    pub bump: u8,
}

/// Terminal outcome for a duel run participant.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum DuelRunOutcome {
    /// Run is still in progress or not finalized yet.
    Pending,
    /// Player reached end of week 3 (completed run).
    CompletedWeek3,
    /// Player died before completing week 3.
    EliminatedBeforeWeek3,
}

/// Snapshot of a finalized duel loadout.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct DuelLoadoutSnapshot {
    /// Final equipped tool at run end.
    pub tool: Option<ItemInstance>,
    /// Final equipped gear at run end.
    pub gear: [Option<ItemInstance>; 12],
    /// Gold held by this player when duel PvP combat starts.
    pub gold_at_battle_start: u16,
}

impl DuelLoadoutSnapshot {
    /// Option<ItemInstance> is 1-byte tag + 10-byte payload.
    pub const ITEM_OPTION_SPACE: usize = 11;
    pub const INIT_SPACE: usize = Self::ITEM_OPTION_SPACE + (12 * Self::ITEM_OPTION_SPACE) + 2;
}

/// One participant in a duel queue for a specific seed.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct DuelParticipant {
    /// Player main wallet.
    pub player: Pubkey,
    /// Session bound to this duel entry.
    pub session: Pubkey,
    /// GameState account for terminal verification.
    pub game_state: Pubkey,
    /// Fixed entry amount paid for this participant.
    pub entry_lamports: u64,
    /// True once run terminal state has been recorded.
    pub finalized: bool,
    /// Terminal run outcome.
    pub outcome: DuelRunOutcome,
    /// Finalized loadout snapshot.
    pub loadout: DuelLoadoutSnapshot,
}

impl DuelParticipant {
    pub const INIT_SPACE: usize =
        32 + 32 + 32 + 8 + 1 + 1 + DuelLoadoutSnapshot::INIT_SPACE;
}

/// Seed-scoped duel queue state.
#[account]
pub struct DuelQueue {
    /// Seed this duel queue is bound to.
    pub seed: u64,
    /// First participant.
    pub player_a: Option<DuelParticipant>,
    /// Second participant.
    pub player_b: Option<DuelParticipant>,
    /// PDA bump seed.
    pub bump: u8,
}

impl DuelQueue {
    pub const INIT_SPACE: usize = 8 + (1 + DuelParticipant::INIT_SPACE) + (1 + DuelParticipant::INIT_SPACE) + 1;
}

/// Vault account holding duel stakes.
#[account]
pub struct DuelVault {
    /// PDA bump seed.
    pub bump: u8,
}

impl DuelVault {
    pub const INIT_SPACE: usize = 1;
}

/// Finished creator snapshot waiting for an async duel opponent.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct DuelCreatorEntry {
    pub player: Pubkey,
    pub seed: u64,
    pub entry_lamports: u64,
    pub finished_slot: u64,
    pub loadout: DuelLoadoutSnapshot,
}

impl DuelCreatorEntry {
    pub const INIT_SPACE: usize = 32 + 8 + 8 + 8 + DuelLoadoutSnapshot::INIT_SPACE;
}

/// Global FIFO queue of finished duel creators available to be matched.
#[account]
pub struct DuelOpenQueue {
    pub entries: Vec<DuelCreatorEntry>,
    pub bump: u8,
}

impl DuelOpenQueue {
    pub const INIT_SPACE: usize = 4 + (DUEL_OPEN_QUEUE_CAPACITY * DuelCreatorEntry::INIT_SPACE) + 1;
}

/// Per-session duel staking/match state.
#[account]
pub struct DuelEntry {
    pub player: Pubkey,
    pub session: Pubkey,
    pub game_state: Pubkey,
    pub seed: u64,
    pub entry_lamports: u64,
    pub finalized: bool,
    pub outcome: DuelRunOutcome,
    pub loadout: DuelLoadoutSnapshot,
    pub matched_creator: Option<DuelCreatorEntry>,
    pub settled: bool,
    pub bump: u8,
}

impl DuelEntry {
    pub const INIT_SPACE: usize = 32
        + 32
        + 32
        + 8
        + 8
        + 1
        + 1
        + DuelLoadoutSnapshot::INIT_SPACE
        + (1 + DuelCreatorEntry::INIT_SPACE)
        + 1
        + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GauntletEchoSource {
    Bootstrap,
    Player(Pubkey),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct GauntletLoadoutSnapshot {
    pub tool: Option<ItemInstance>,
    pub gear: [Option<ItemInstance>; 12],
    pub gold_at_battle_start: u16,
}

impl GauntletLoadoutSnapshot {
    pub const ITEM_OPTION_SPACE: usize = 11;
    pub const INIT_SPACE: usize =
        Self::ITEM_OPTION_SPACE + (12 * Self::ITEM_OPTION_SPACE) + 2;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct GauntletEchoSnapshot {
    pub week: u8,
    pub source: GauntletEchoSource,
    pub loadout: GauntletLoadoutSnapshot,
}

impl GauntletEchoSnapshot {
    pub const INIT_SPACE: usize = 1 + (1 + 32) + GauntletLoadoutSnapshot::INIT_SPACE;
}

#[account]
pub struct GauntletConfig {
    pub entry_lamports: u64,
    pub company_fee_bps: u16,
    pub pool_fee_bps: u16,
    pub current_epoch_id: u64,
    pub current_epoch_start_ts: i64,
    pub epoch_duration_seconds: i64,
    pub bump: u8,
}

impl GauntletConfig {
    pub const INIT_SPACE: usize = 8 + 2 + 2 + 8 + 8 + 8 + 1;
}

#[account]
pub struct GauntletPoolVault {
    pub bump: u8,
}

impl GauntletPoolVault {
    pub const INIT_SPACE: usize = 1;
}

#[account]
pub struct GauntletWeekPool {
    pub week: u8,
    pub bootstrap_active: bool,
    pub player_echoes_added: u16,
    pub seen_player_echoes: u64,
    pub entries: Vec<GauntletEchoSnapshot>,
    pub bump: u8,
}

impl GauntletWeekPool {
    pub const INIT_SPACE: usize = 1
        + 1
        + 2
        + 8
        + 4
        + (crate::constants::GAUNTLET_MAX_WEEKLY_ECHOES * GauntletEchoSnapshot::INIT_SPACE)
        + 1;
}

#[account]
pub struct GauntletEpochPool {
    pub epoch_id: u64,
    pub total_pool_lamports: u64,
    pub total_points: u64,
    pub pending_defender_points: Vec<GauntletPendingPoints>,
    pub finalized: bool,
    pub bump: u8,
}

impl GauntletEpochPool {
    pub const MAX_PENDING_DEFENDERS: usize = 200;
    pub const INIT_SPACE: usize =
        8 + 8 + 8 + 4 + (Self::MAX_PENDING_DEFENDERS * GauntletPendingPoints::INIT_SPACE) + 1 + 1;
}

#[account]
pub struct GauntletPlayerScore {
    pub epoch_id: u64,
    pub player: Pubkey,
    pub points: u64,
    pub claimed: bool,
    pub bump: u8,
}

impl GauntletPlayerScore {
    pub const INIT_SPACE: usize = 8 + 32 + 8 + 1 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct GauntletPendingPoints {
    pub player: Pubkey,
    pub points: u64,
}

impl GauntletPendingPoints {
    pub const INIT_SPACE: usize = 32 + 8;
}
