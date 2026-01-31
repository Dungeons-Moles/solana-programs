/// Maximum length of player display name
pub const MAX_NAME_LENGTH: usize = 32;

/// Initial level (highest level unlocked at start)
pub const INITIAL_LEVEL: u8 = 1;

/// Initial total runs
pub const INITIAL_TOTAL_RUNS: u32 = 0;

/// Initial available runs (changed from 40 to 20)
pub const INITIAL_AVAILABLE_RUNS: u32 = 20;

/// Maximum campaign level (40 levels total)
pub const MAX_CAMPAIGN_LEVEL: u8 = 40;

/// Total number of items in the game
pub const TOTAL_ITEMS: u8 = 80;

/// Number of starter items (indices 0-39)
pub const STARTER_ITEMS_COUNT: u8 = 40;

/// Minimum size for active item pool
pub const MIN_ACTIVE_POOL_SIZE: u8 = 40;

/// Cost to purchase runs in lamports (0.005 SOL)
pub const RUN_PURCHASE_COST_LAMPORTS: u64 = 5_000_000;

/// Number of runs received per purchase
pub const RUNS_PER_PURCHASE: u32 = 20;

/// Size of item bitmask in bytes (80 bits = 10 bytes)
pub const ITEM_BITMASK_SIZE: usize = 10;

/// Treasury wallet for run purchases (5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8)
/// NOTE: This MUST be updated to the actual treasury pubkey before mainnet deployment
pub const TREASURY_PUBKEY: [u8; 32] = [
    0x40, 0x88, 0xf7, 0x83, 0x7b, 0xd0, 0x04, 0x6c, 0x2e, 0x19, 0x63, 0xf5, 0x10, 0x22, 0x23, 0x23,
    0xb6, 0xf7, 0xee, 0xc0, 0x3b, 0xfd, 0xa8, 0x84, 0xfd, 0x3c, 0x91, 0x6b, 0x20, 0x5f, 0x91, 0x35,
];
