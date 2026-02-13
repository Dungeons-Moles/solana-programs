/// Moves allowed per day phase
pub const DAY_MOVES: u8 = 50;

/// Moves allowed per night phase
pub const NIGHT_MOVES: u8 = 30;

// =============================================================================
// Base Stats (derived from inventory at runtime)
// =============================================================================

/// Base HP before item bonuses
pub const BASE_HP: i16 = 10;

/// Base attack stat before item bonuses
pub const BASE_ATK: i16 = 0;

/// Base armor stat before item bonuses
pub const BASE_ARM: i16 = 0;

/// Base speed stat before item bonuses
pub const BASE_SPD: i16 = 0;

/// Base dig stat before item bonuses
pub const BASE_DIG: i16 = 1;

/// Initial gear slots at game start
pub const INITIAL_GEAR_SLOTS: u8 = 4;

/// Maximum gear slots (capped)
pub const MAX_GEAR_SLOTS: u8 = 12;

/// Base cost for digging through walls (before DIG stat reduction)
pub const BASE_DIG_COST: u8 = 6;

/// Minimum cost for digging (even with high DIG stat)
pub const MIN_DIG_COST: u8 = 2;

/// Cost for moving to a floor tile
pub const FLOOR_MOVE_COST: u8 = 1;

/// PDA seed prefix for GameState accounts
pub const GAME_STATE_SEED: &[u8] = b"game_state";

/// PDA seed prefix for the global pit draft queue account
pub const PIT_DRAFT_QUEUE_SEED: &[u8] = b"pit_draft_queue";

/// PDA seed prefix for the pit draft vault account
pub const PIT_DRAFT_VAULT_SEED: &[u8] = b"pit_draft_vault";

/// PDA seed prefix for seed-scoped duel queues
pub const DUEL_QUEUE_SEED: &[u8] = b"duel_queue";

/// PDA seed prefix for duel stake vault
pub const DUEL_VAULT_SEED: &[u8] = b"duel_vault";
/// PDA seed prefix for open finished-duel creator queue
pub const DUEL_OPEN_QUEUE_SEED: &[u8] = b"duel_open_queue";
/// PDA seed prefix for per-session duel entry state
pub const DUEL_ENTRY_SEED: &[u8] = b"duel_entry";
/// Maximum number of finished creators buffered for async duel matching.
// Must stay under Solana CPI account allocation limit (~10,240 bytes account data).
// DuelCreatorEntry::INIT_SPACE is currently 201 bytes, so 50 entries keeps the queue account safely below the limit
// (8-byte discriminator + DuelOpenQueue::INIT_SPACE = 10,063 bytes total).
pub const DUEL_OPEN_QUEUE_CAPACITY: usize = 50;

/// PDA seed prefix for gauntlet config account
pub const GAUNTLET_CONFIG_SEED: &[u8] = b"gauntlet_config";

/// PDA seed prefix for gauntlet pool vault
pub const GAUNTLET_POOL_VAULT_SEED: &[u8] = b"gauntlet_pool_vault";

/// PDA seed prefix for gauntlet week pools
pub const GAUNTLET_WEEK_POOL_SEED: &[u8] = b"gauntlet_week_pool";

/// PDA seed prefix for gauntlet player score accounts
pub const GAUNTLET_PLAYER_SCORE_SEED: &[u8] = b"gauntlet_player_score";

/// PDA seed prefix for gauntlet epoch pool accounts
pub const GAUNTLET_EPOCH_POOL_SEED: &[u8] = b"gauntlet_epoch_pool";

/// Pit Draft v1 stake amount (0.1 SOL)
pub const PIT_DRAFT_ENTRY_LAMPORTS: u64 = 100_000_000;
/// Duel stake amount (0.1 SOL)
pub const DUEL_ENTRY_LAMPORTS: u64 = 100_000_000;

/// Gauntlet v1 entry amount (0.01 SOL)
pub const GAUNTLET_ENTRY_LAMPORTS: u64 = 10_000_000;

/// Fee split configuration in basis points (10000 = 100%)
pub const PIT_DRAFT_BPS_DENOMINATOR: u64 = 10_000;
pub const PIT_DRAFT_COMPANY_FEE_BPS: u64 = 300; // 3%
pub const PIT_DRAFT_GAUNTLET_FEE_BPS: u64 = 200; // 2%
pub const PIT_DRAFT_WINNER_BPS: u64 = 9_500; // 95%
pub const GAUNTLET_COMPANY_FEE_BPS: u64 = 300; // 3%
pub const GAUNTLET_POOL_FEE_BPS: u64 = 9_700; // 97%

pub const GAUNTLET_EPOCH_DURATION_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const GAUNTLET_MAX_WEEKLY_ECHOES: usize = 50;
pub const GAUNTLET_BOOTSTRAP_ECHOES_PER_WEEK: usize = 10;

/// Fixed campaign level used by Gauntlet session identity (must match session-manager).
pub const GAUNTLET_CAMPAIGN_LEVEL: u8 = 19;

/// Company treasury and gauntlet sink addresses.
pub const COMPANY_TREASURY_ADDRESS: &str = "5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8";
pub const GAUNTLET_SINK_ADDRESS: &str = "1nc1nerator11111111111111111111111111111111";
