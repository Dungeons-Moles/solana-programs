use anchor_lang::prelude::*;

/// Size of item bitmask in bytes (80 bits = 10 bytes)
pub const SESSION_ITEM_BITMASK_SIZE: usize = 10;

/// Empty state hash constant (all zeros)
pub const EMPTY_STATE_HASH: [u8; 32] = [0u8; 32];

/// Represents an active game session for a player.
/// Campaign PDA seeds: [b"session", player.key(), &[campaign_level]]
/// Duel/Gauntlet use dedicated prefixes to avoid collisions with campaign levels.
#[account]
pub struct GameSession {
    /// Player profile owner's wallet
    pub player: Pubkey,
    /// Unique session identifier (incrementing)
    pub session_id: u64,
    /// Level being played in this session (campaign mode, 1-40)
    pub campaign_level: u8,
    /// Unix timestamp when session started
    pub started_at: i64,
    /// Unix timestamp of last activity
    pub last_activity: i64,
    /// Whether state is delegated to ephemeral rollup
    pub is_delegated: bool,
    /// PDA bump seed
    pub bump: u8,
    /// Snapshot of player's active_item_pool at session start
    /// Determines which items can appear in POI offers during this session
    pub active_item_pool: [u8; SESSION_ITEM_BITMASK_SIZE],
    /// Burner wallet pubkey for gameplay transactions
    /// SOL is transferred to this wallet at session start
    pub burner_wallet: Pubkey,
    /// Hash of the current game state (for validation)
    pub state_hash: [u8; 32],
}

impl GameSession {
    /// PDA seed prefix for session accounts
    pub const SEED_PREFIX: &'static [u8] = b"session";
    /// PDA seed prefix for duel sessions
    pub const DUEL_SEED_PREFIX: &'static [u8] = b"duel_session";
    /// PDA seed prefix for gauntlet sessions
    pub const GAUNTLET_SEED_PREFIX: &'static [u8] = b"gauntlet_session";

    /// Account space calculation
    /// 8 (discriminator) + 32 (player) + 8 (session_id) + 1 (campaign_level) +
    /// 8 (started_at) + 8 (last_activity) + 1 (is_delegated) + 1 (bump) +
    /// 10 (active_item_pool) + 32 (burner_wallet) + 32 (state_hash)
    pub const INIT_SPACE: usize = 32 + 8 + 1 + 8 + 8 + 1 + 1 + 10 + 32 + 32;

    /// Byte offset of `player` field in serialized account data (after 8-byte discriminator).
    pub const PLAYER_OFFSET: usize = 8;
    /// Byte offset of `burner_wallet` field in serialized account data.
    pub const BURNER_WALLET_OFFSET: usize = 77;
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
