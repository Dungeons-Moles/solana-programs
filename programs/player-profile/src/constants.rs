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

/// Total bitmask capacity for item indices.
/// Pool is fixed to the core 80-item set.
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

/// Session Manager program ID (FcMT7MzBLVQGaMATEMws3fjsL2Q77QSHmoEPdowTMxJa)
/// Used to validate session account ownership in CPI authorization.
pub const SESSION_MANAGER_PROGRAM_ID: [u8; 32] = [
    0xd9, 0x12, 0x11, 0x80, 0x4f, 0x8c, 0x98, 0x49, 0x67, 0x5f, 0x86, 0xb3, 0x1f, 0x6d, 0x22, 0x52,
    0xfa, 0xa7, 0x5b, 0x43, 0xba, 0x17, 0xd1, 0x02, 0x50, 0xff, 0x76, 0xc0, 0xaf, 0xf2, 0xde, 0xb7,
];

/// Byte offset of `burner_wallet` field in GameSession account data.
/// Layout: 8 (discriminator) + 32 (player) + 8 (session_id) + 1 (campaign_level) +
/// 8 (started_at) + 8 (last_activity) + 1 (is_delegated) + 1 (bump) +
/// 10 (active_item_pool) = 77
pub const SESSION_BURNER_WALLET_OFFSET: usize = 77;

/// Minimum session account data length required to read through burner_wallet.
/// SESSION_BURNER_WALLET_OFFSET + 32 (Pubkey size) = 109
pub const SESSION_MIN_DATA_LEN: usize = SESSION_BURNER_WALLET_OFFSET + 32;

/// Treasury wallet for run purchases (5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8)
/// NOTE: This MUST be updated to the actual treasury pubkey before mainnet deployment
pub const TREASURY_PUBKEY: [u8; 32] = [
    0x40, 0x88, 0xf7, 0x83, 0x7b, 0xd0, 0x04, 0x6c, 0x2e, 0x19, 0x63, 0xf5, 0x10, 0x22, 0x23, 0x23,
    0xb6, 0xf7, 0xee, 0xc0, 0x3b, 0xfd, 0xa8, 0x84, 0xfd, 0x3c, 0x91, 0x6b, 0x20, 0x5f, 0x91, 0x35,
];

/// Gameplay-state program ID (5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n).
/// Used to derive and validate the canonical gauntlet pool vault PDA.
pub const GAMEPLAY_STATE_PROGRAM_ID: [u8; 32] = [
    66, 165, 213, 208, 125, 103, 44, 88, 115, 217, 192, 197, 1, 117, 7, 170, 78, 32, 208, 143, 119,
    94, 47, 124, 229, 196, 47, 149, 235, 227, 237, 31,
];

/// Seed used by gameplay-state for the canonical gauntlet pool vault PDA.
pub const GAUNTLET_POOL_VAULT_SEED: &[u8] = b"gauntlet_pool_vault";
