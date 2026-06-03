use quasar_lang::prelude::*;

use crate::constants::{ITEM_BITMASK_SIZE, MAX_RELICS};

pub const PLAYER_PROFILE_DISCRIMINATOR: [u8; 8] = [82, 226, 99, 87, 164, 130, 181, 80];
pub const PLAYER_RELIC_POOL_DISCRIMINATOR: [u8; 8] = [1, 105, 67, 203, 111, 254, 159, 128];
pub const RELIC_ENTRY_SIZE: usize = 11;
pub const RELIC_ENTRY_BYTES: usize = MAX_RELICS * RELIC_ENTRY_SIZE;

/// Player profile account storing identity and progression data.
/// PDA seeds: [b"player", owner].
#[account(
    discriminator = [82, 226, 99, 87, 164, 130, 181, 80],
    fixed_capacity,
    set_inner
)]
#[seeds(b"player", owner: Address)]
pub struct PlayerProfile {
    /// Wallet address that owns this profile.
    pub owner: Address,
    /// Total dungeon runs completed.
    pub total_runs: u32,
    /// Highest campaign level unlocked (1-40).
    pub highest_level_unlocked: u8,
    /// Remaining available dungeon runs.
    pub available_runs: u32,
    /// Unix timestamp of profile creation.
    pub created_at: i64,
    /// PDA bump seed.
    pub bump: u8,
    /// Bitmask of unlocked items.
    pub unlocked_items: [u8; ITEM_BITMASK_SIZE],
    /// Bitmask of items in the active pool for new sessions.
    pub active_item_pool: [u8; ITEM_BITMASK_SIZE],
    /// Currently equipped Metaplex Core skin NFT.
    pub equipped_skin: Option<Address>,
    /// Free gauntlet entries from quest rewards.
    pub gauntlet_boosters: u8,
    /// Display name.
    pub name: String<32>,
}

impl PlayerProfile {
    pub const SEED_PREFIX: &'static [u8] = b"player";
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RelicEntry {
    pub item_id: [u8; 8],
    pub owned_count: u16,
    pub in_active_pool: bool,
}

impl RelicEntry {
    #[inline(always)]
    pub fn new(item_id: [u8; 8], owned_count: u16, in_active_pool: bool) -> Self {
        Self {
            item_id,
            owned_count,
            in_active_pool,
        }
    }
}

/// Per-player relic ownership pool.
#[account(
    discriminator = [1, 105, 67, 203, 111, 254, 159, 128],
    fixed_capacity,
    set_inner
)]
#[seeds(b"player_relics", owner: Address)]
pub struct PlayerRelicPool {
    pub owner: Address,
    pub count: u8,
    pub bump: u8,
    pub relic_entries: [u8; RELIC_ENTRY_BYTES],
}

impl PlayerRelicPool {
    pub const SEED_PREFIX: &'static [u8] = b"player_relics";

    #[inline(always)]
    pub fn find_index_by_item_id(&self, item_id: [u8; 8]) -> Option<usize> {
        let count = self.count.min(MAX_RELICS as u8) as usize;
        let mut index = 0usize;
        while index < count {
            if self.entry(index).item_id == item_id {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    #[inline(always)]
    pub fn entry(&self, index: usize) -> RelicEntry {
        let offset = index * RELIC_ENTRY_SIZE;
        let mut item_id = [0u8; 8];
        item_id.copy_from_slice(&self.relic_entries[offset..offset + 8]);
        let owned_count = u16::from_le_bytes([
            self.relic_entries[offset + 8],
            self.relic_entries[offset + 9],
        ]);
        RelicEntry {
            item_id,
            owned_count,
            in_active_pool: self.relic_entries[offset + 10] != 0,
        }
    }

    #[inline(always)]
    pub fn set_entry(&mut self, index: usize, entry: RelicEntry) {
        let offset = index * RELIC_ENTRY_SIZE;
        self.relic_entries[offset..offset + 8].copy_from_slice(&entry.item_id);
        let count_bytes = entry.owned_count.to_le_bytes();
        self.relic_entries[offset + 8] = count_bytes[0];
        self.relic_entries[offset + 9] = count_bytes[1];
        self.relic_entries[offset + 10] = if entry.in_active_pool { 1 } else { 0 };
    }

    #[inline(always)]
    pub fn push_entry(&mut self, entry: RelicEntry) -> Result<(), ProgramError> {
        let count = self.count as usize;
        if count >= MAX_RELICS {
            return Err(crate::errors::PlayerProfileError::RelicPoolFull.into());
        }
        self.set_entry(count, entry);
        self.count = self.count.saturating_add(1);
        Ok(())
    }

    #[inline(always)]
    pub fn swap_remove(&mut self, index: usize) {
        let count = self.count as usize;
        if index >= count {
            return;
        }
        let last = count - 1;
        if index != last {
            let last_entry = self.entry(last);
            self.set_entry(index, last_entry);
        }
        let offset = last * RELIC_ENTRY_SIZE;
        self.relic_entries[offset..offset + RELIC_ENTRY_SIZE].fill(0);
        self.count = self.count.saturating_sub(1);
    }
}
