use anchor_lang::prelude::*;

use crate::constants::{ITEM_BITMASK_SIZE, MAX_RELICS};

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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, Default, InitSpace)]
pub struct RelicEntry {
    pub item_id: [u8; 8],
    pub owned_count: u16,
    pub in_active_pool: bool,
}

#[account]
#[derive(InitSpace)]
pub struct PlayerRelicPool {
    pub owner: Pubkey,
    pub count: u8,
    #[max_len(MAX_RELICS)]
    pub relics: Vec<RelicEntry>,
    pub bump: u8,
}

impl PlayerRelicPool {
    pub const SEED_PREFIX: &'static [u8] = b"player_relics";

    pub fn find_index_by_item_id(&self, item_id: [u8; 8]) -> Option<usize> {
        self.relics.iter().position(|entry| entry.item_id == item_id)
    }
}
