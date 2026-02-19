use anchor_lang::prelude::*;

use crate::constants::ITEM_BITMASK_SIZE;

/// Player profile account storing identity and progression data.
/// PDA Seeds: [b"player", owner.key()]
#[account]
pub struct PlayerProfile {
    /// Wallet address that owns this profile
    pub owner: Pubkey,

    /// Display name (max 32 chars)
    pub name: String,

    /// Total dungeon runs completed
    pub total_runs: u32,

    /// Highest campaign level unlocked (1-40)
    /// Renamed from `current_level` to better reflect its purpose
    pub highest_level_unlocked: u8,

    /// Remaining available dungeon runs
    pub available_runs: u32,

    /// Unix timestamp of profile creation
    pub created_at: i64,

    /// PDA bump seed
    pub bump: u8,

    /// Bitmask of unlocked items (80 bits = 10 bytes)
    /// Bits 0-39 are starter items, bits 40-92 are unlockable items
    pub unlocked_items: [u8; ITEM_BITMASK_SIZE],

    /// Bitmask of items in the active pool for new sessions
    /// Must be a subset of `unlocked_items` and have >= 40 items
    pub active_item_pool: [u8; ITEM_BITMASK_SIZE],

    /// Currently equipped Metaplex Core skin NFT (if any)
    pub equipped_skin: Option<Pubkey>,

    /// Free gauntlet entries from quest rewards
    pub gauntlet_boosters: u8,
}

impl PlayerProfile {
    /// Seed prefix for PDA derivation
    pub const SEED_PREFIX: &'static [u8] = b"player";

    /// Account space calculation
    /// 8 (discriminator) + 32 (owner) + 4 + 32 (name max) + 4 (total_runs) +
    /// 1 (highest_level_unlocked) + 4 (available_runs) + 8 (created_at) +
    /// 1 (bump) + 10 (unlocked_items) + 10 (active_item_pool) +
    /// 1 + 32 (equipped_skin Option<Pubkey>) + 1 (gauntlet_boosters)
    pub const INIT_SPACE: usize = 32 + 4 + 32 + 4 + 1 + 4 + 8 + 1 + 10 + 10 + 1 + 32 + 1;
}
