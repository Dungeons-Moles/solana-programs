use anchor_lang::prelude::*;

/// Represents an active game session for a player.
/// One session allowed per player (enforced via PDA uniqueness).
#[account]
#[derive(InitSpace)]
pub struct GameSession {
    /// Player profile owner's wallet
    pub player: Pubkey,
    /// Unique session identifier (incrementing)
    pub session_id: u64,
    /// Level being played in this session (campaign mode)
    pub campaign_level: u8,
    /// Unix timestamp when session started
    pub started_at: i64,
    /// Unix timestamp of last activity
    pub last_activity: i64,
    /// Whether state is delegated to ephemeral rollup
    pub is_delegated: bool,
    /// Hash of current game state (for verification)
    pub state_hash: [u8; 32],
    /// PDA bump seed
    pub bump: u8,
}

impl GameSession {
    /// PDA seed prefix for session accounts
    pub const SEED_PREFIX: &'static [u8] = b"session";
}

/// Global counter for generating unique session IDs
#[account]
#[derive(InitSpace)]
pub struct SessionCounter {
    /// Global session counter for unique IDs
    pub count: u64,
    /// PDA bump seed
    pub bump: u8,
}

impl SessionCounter {
    /// PDA seed prefix for the session counter
    pub const SEED_PREFIX: &'static [u8] = b"session_counter";
}
