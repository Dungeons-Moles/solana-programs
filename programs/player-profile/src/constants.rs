/// Cost to unlock a campaign tier (0.05 SOL in lamports)
pub const TIER_UNLOCK_COST: u64 = 50_000_000;

/// Number of levels per tier
pub const LEVELS_PER_TIER: u8 = 40;

/// Session timeout in seconds (1 hour)
pub const SESSION_TIMEOUT: i64 = 3600;

/// Maximum length of player display name
pub const MAX_NAME_LENGTH: usize = 32;

/// Initial tier (free tier)
pub const INITIAL_TIER: u8 = 0;

/// Initial level
pub const INITIAL_LEVEL: u8 = 0;

/// Initial total runs
pub const INITIAL_TOTAL_RUNS: u32 = 0;
