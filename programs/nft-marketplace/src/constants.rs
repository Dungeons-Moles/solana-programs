use quasar_lang::prelude::*;

pub const MARKETPLACE_PROGRAM_ADDRESS: Address =
    address!("GLKxBpZ8hc7qzvD9VHAVsJEjHSu2JVp1HaPrGH4fpTci");

/// Metaplex Core program ID.
pub const MPL_CORE_PROGRAM_ADDRESS: Address =
    address!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");

/// SPL Noop program used by Metaplex Core logging.
pub const SPL_NOOP_PROGRAM_ADDRESS: Address =
    address!("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

/// Company treasury used by marketplace fee collection.
pub const COMPANY_TREASURY: Address = address!("5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8");

pub const DEFAULT_COMPANY_FEE_BPS: u16 = 300;
pub const DEFAULT_GAUNTLET_FEE_BPS: u16 = 200;
pub const BPS_DENOMINATOR: u64 = 10_000;
pub const MIN_LISTING_PRICE: u64 = 10_000;

/// Player Profile program ID used for relic ownership CPIs.
pub const PLAYER_PROFILE_PROGRAM_ADDRESS: Address =
    address!("GSLNDrNoHeZXVxB7Yu7tUe8417PpZ5XV7JPYupPw9WQy");

pub const PLAYER_PROFILE_SEED: &[u8] = b"player";

/// Gameplay-state program ID used only to validate the gauntlet pool PDA.
pub const GAMEPLAY_STATE_PROGRAM_ADDRESS: Address =
    address!("3rzGGgHRRnMATmYJkjidPMapEMesvA16PTs5HhfAep4V");

pub const GAUNTLET_POOL_VAULT_SEED: &[u8] = b"gauntlet_pool_vault";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";

pub const MAX_MPL_NAME_LENGTH: usize = 64;
pub const MAX_MPL_URI_LENGTH: usize = 200;
pub const MPL_CREATE_MAX_DATA: usize = 280;

pub const MPL_CORE_ASSET_V1_DISCRIMINATOR: u8 = 1;
pub const MPL_CORE_OWNER_OFFSET: usize = 1;
pub const MPL_CORE_MIN_DATA_LEN: usize = MPL_CORE_OWNER_OFFSET + 32;

pub const PLAYER_PROFILE_DISCRIMINATOR: [u8; 8] = [82, 226, 99, 87, 164, 130, 181, 80];
pub const PROFILE_EQUIPPED_SKIN_OFFSET: usize = 78;
