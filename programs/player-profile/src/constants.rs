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

/// Session Manager program ID (6w1XVMSTRmZU9AWCKVvKohGAHSFMENhda7vqhKPQ8TPn)
/// Used to validate session account ownership in CPI authorization.
pub const SESSION_MANAGER_PROGRAM_ID: [u8; 32] = [
    0x58, 0x20, 0x64, 0x87, 0xdf, 0xd8, 0x68, 0xf1, 0xa4, 0x79, 0x15, 0x8b, 0xb2, 0x8a, 0x56, 0x0c,
    0xa9, 0x4f, 0x56, 0x2e, 0x62, 0x85, 0x26, 0xb7, 0x4f, 0x8b, 0xa1, 0x4d, 0x08, 0x36, 0x20, 0x99,
];

/// Seed for session-manager authority PDA that signs authorized CPIs.
pub const SESSION_MANAGER_AUTHORITY_SEED: &[u8] = b"session_manager_authority";

/// Byte offset of `campaign_level` field in GameSession account data.
/// Layout: 8 (discriminator) + 32 (player) + 8 (session_id) = 48
pub const SESSION_CAMPAIGN_LEVEL_OFFSET: usize = 48;

/// Byte offset of `session_signer` field in GameSession account data.
/// Layout: 8 (discriminator) + 32 (player) + 8 (session_id) + 1 (campaign_level) +
/// 8 (started_at) + 8 (last_activity) + 1 (is_delegated) + 1 (bump) +
/// 10 (active_item_pool) = 77
pub const SESSION_SESSION_SIGNER_OFFSET: usize = 77;

/// Minimum session account data length required to read through session_signer.
/// SESSION_SESSION_SIGNER_OFFSET + 32 (Pubkey size) = 109
pub const SESSION_MIN_DATA_LEN: usize = SESSION_SESSION_SIGNER_OFFSET + 32;

/// Treasury wallet for run purchases (5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8)
/// NOTE: This MUST be updated to the actual treasury pubkey before mainnet deployment
pub const TREASURY_PUBKEY: [u8; 32] = [
    0x40, 0x88, 0xf7, 0x83, 0x7b, 0xd0, 0x04, 0x6c, 0x2e, 0x19, 0x63, 0xf5, 0x10, 0x22, 0x23, 0x23,
    0xb6, 0xf7, 0xee, 0xc0, 0x3b, 0xfd, 0xa8, 0x84, 0xfd, 0x3c, 0x91, 0x6b, 0x20, 0x5f, 0x91, 0x35,
];

/// Gameplay-state program ID (C8hK4qsqsSYQeqyXuTPTUUS3T7N74WnZCuzvChTpK1Mo).
/// Used to derive and validate the canonical gauntlet pool vault PDA.
pub const GAMEPLAY_STATE_PROGRAM_ID: [u8; 32] = [
    0xa5, 0x69, 0x33, 0xc3, 0x32, 0x44, 0x5d, 0xb7, 0x52, 0x8d, 0x7a, 0x6b, 0xc3, 0x01, 0x56, 0x1e,
    0x68, 0x50, 0xaa, 0x96, 0x7a, 0x85, 0xea, 0x62, 0xb5, 0x79, 0xe3, 0x23, 0xe4, 0xa8, 0x88, 0x36,
];

/// Seed used by gameplay-state for the canonical gauntlet pool vault PDA.
pub const GAUNTLET_POOL_VAULT_SEED: &[u8] = b"gauntlet_pool_vault";

/// Seed used by gameplay-state for the global pit draft queue PDA.
pub const PIT_DRAFT_QUEUE_SEED: &[u8] = b"pit_draft_queue";

/// Metaplex Core program ID for NFT skin asset validation (CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d)
pub const MPL_CORE_PROGRAM_ID: anchor_lang::prelude::Pubkey =
    anchor_lang::pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
