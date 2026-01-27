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

/// Treasury wallet for run purchases
/// NOTE: This MUST be updated to the actual treasury pubkey before mainnet deployment
/// Using a real devnet wallet for testing. System program address (11111...1) is NOT valid.
pub const TREASURY_PUBKEY: [u8; 32] = [
    0x9b, 0x5c, 0xf8, 0x4e, 0x3a, 0x2d, 0x1f, 0x6c, 0x8e, 0x7b, 0x4a, 0x3d, 0x2c, 0x1e, 0x0f, 0x9a,
    0x8b, 0x7c, 0x6d, 0x5e, 0x4f, 0x3a, 0x2b, 0x1c, 0x0d, 0x9e, 0x8f, 0x7a, 0x6b, 0x5c, 0x4d, 0x3e,
];
