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

/// Cost to purchase runs in lamports (0.05 SOL)
pub const RUN_PURCHASE_COST_LAMPORTS: u64 = 50_000_000;

/// Number of runs received per purchase
pub const RUNS_PER_PURCHASE: u32 = 20;

/// Size of item bitmask in bytes (80 bits = 10 bytes)
pub const ITEM_BITMASK_SIZE: usize = 10;

/// Session Manager program ID (CrU4bUFreKy2XsoU2oksdJWKim11w2VpagKBQ2MTkyMz)
/// Used to validate session account ownership in CPI authorization.
pub const SESSION_MANAGER_PROGRAM_ID: [u8; 32] = [
    0xb0, 0x1c, 0x9d, 0x6a, 0x40, 0xc6, 0xe0, 0xa9, 0xe4, 0xaf, 0xa9, 0xa9, 0xd9, 0xad, 0x02, 0x15,
    0x89, 0xbd, 0xf1, 0x36, 0x79, 0x88, 0x02, 0x94, 0xc2, 0x24, 0x9f, 0xd9, 0xa4, 0x21, 0xd7, 0x39,
];

/// Session Manager program ID as Pubkey for use in Anchor account owner constraints.
pub const SESSION_MANAGER_PROGRAM_PUBKEY: anchor_lang::prelude::Pubkey =
    anchor_lang::prelude::Pubkey::new_from_array(SESSION_MANAGER_PROGRAM_ID);

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

/// Gameplay-state program ID (3rzGGgHRRnMATmYJkjidPMapEMesvA16PTs5HhfAep4V).
/// Used to derive and validate the canonical gauntlet pool vault PDA.
pub const GAMEPLAY_STATE_PROGRAM_ID: [u8; 32] = [
    0x2a, 0x85, 0x94, 0xcf, 0xca, 0x5f, 0x00, 0x45, 0x30, 0xce, 0x64, 0xd1, 0x54, 0x94, 0x6b, 0x36,
    0xcb, 0xd4, 0x94, 0x56, 0x16, 0x97, 0xa1, 0x82, 0x0d, 0x72, 0x1b, 0x7e, 0x89, 0xb7, 0xbf, 0x7e,
];

/// Gameplay-state program ID as Pubkey for use in Anchor account owner constraints.
pub const GAMEPLAY_STATE_PROGRAM_PUBKEY: anchor_lang::prelude::Pubkey =
    anchor_lang::prelude::Pubkey::new_from_array(GAMEPLAY_STATE_PROGRAM_ID);

/// Anchor discriminator for `PitDraftQueue` account: sha256("account:PitDraftQueue")[..8].
pub const PIT_DRAFT_QUEUE_DISCRIMINATOR: [u8; 8] =
    [0xfa, 0x28, 0xca, 0x89, 0x8c, 0xa2, 0xda, 0x4d];

/// Seed used by gameplay-state for the canonical gauntlet pool vault PDA.
pub const GAUNTLET_POOL_VAULT_SEED: &[u8] = b"gauntlet_pool_vault";

/// Seed used by gameplay-state for the global pit draft queue PDA.
pub const PIT_DRAFT_QUEUE_SEED: &[u8] = b"pit_draft_queue";

/// Metaplex Core program ID for NFT skin asset validation (CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d)
pub const MPL_CORE_PROGRAM_ID: anchor_lang::prelude::Pubkey =
    anchor_lang::pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
