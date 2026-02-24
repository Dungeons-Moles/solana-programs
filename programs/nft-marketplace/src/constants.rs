use anchor_lang::prelude::Pubkey;
use anchor_lang::pubkey;

/// Metaplex Core program ID
pub const MPL_CORE_PROGRAM_ID: Pubkey = pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

/// Company treasury (same as player-profile treasury)
/// 5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8
pub const COMPANY_TREASURY: Pubkey = pubkey!("5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8");

/// Default company marketplace fee: 3% (300 bps)
pub const DEFAULT_COMPANY_FEE_BPS: u16 = 300;

/// Default gauntlet pool marketplace fee: 2% (200 bps)
pub const DEFAULT_GAUNTLET_FEE_BPS: u16 = 200;

/// BPS denominator
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Royalty bps for collection creation: 500 bps = 5%
pub const ROYALTY_BPS: u16 = 500;

/// Player Profile program ID (Ch3bbL1oQk2z5rX1jiun3KuSWZqnXZ1MnrfrtKj4MKun)
pub const PLAYER_PROFILE_PROGRAM_ID: Pubkey =
    pubkey!("Ch3bbL1oQk2z5rX1jiun3KuSWZqnXZ1MnrfrtKj4MKun");

/// PDA seed for player profile: ["player", owner]
pub const PLAYER_PROFILE_SEED: &[u8] = b"player";

/// Gameplay-state program ID (C8hK4qsqsSYQeqyXuTPTUUS3T7N74WnZCuzvChTpK1Mo)
pub const GAMEPLAY_STATE_PROGRAM_ID: Pubkey =
    pubkey!("C8hK4qsqsSYQeqyXuTPTUUS3T7N74WnZCuzvChTpK1Mo");

/// Seed used by gameplay-state for the canonical gauntlet pool vault PDA
pub const GAUNTLET_POOL_VAULT_SEED: &[u8] = b"gauntlet_pool_vault";
