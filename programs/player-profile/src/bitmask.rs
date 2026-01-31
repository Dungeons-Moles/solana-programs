//! Bitmask helper module for item tracking.
//!
//! Provides utilities for managing 80-bit bitmasks used for `unlocked_items`
//! and `active_item_pool` in PlayerProfile.

use anchor_lang::prelude::*;

/// Starter items bitmask: 40 specific items across all 8 tags.
///
/// Pool index mapping:
/// - Gear: tag_code * 8 + (item_num - 1), indices 0-63
/// - Tools: 64 + tag_code * 2 + (item_num - 1), indices 64-79
///
/// Starter items (40 total = 32 gear + 8 tools):
/// - Stone: G-ST-01..04 (0-3), T-ST-01 (64)
/// - Scout: G-SC-01..04 (8-11), T-SC-01 (66)
/// - Greed: G-GR-01..03,05 (16-18,20), T-GR-01 (68) — note: skips G-GR-04
/// - Blast: G-BL-01..04 (24-27), T-BL-01 (70)
/// - Frost: G-FR-01..04 (32-35), T-FR-01 (72)
/// - Rust: G-RU-01..04 (40-43), T-RU-01 (74)
/// - Blood: G-BO-01..04 (48-51), T-BO-01 (76)
/// - Tempo: G-TE-01..04 (56-59), T-TE-01 (78)
pub const STARTER_ITEMS_BITMASK: [u8; 10] = [
    0x0F, // byte 0: bits 0-3 (Stone gear 01-04)
    0x0F, // byte 1: bits 8-11 (Scout gear 01-04)
    0x17, // byte 2: bits 16-18,20 (Greed gear 01-03,05)
    0x0F, // byte 3: bits 24-27 (Blast gear 01-04)
    0x0F, // byte 4: bits 32-35 (Frost gear 01-04)
    0x0F, // byte 5: bits 40-43 (Rust gear 01-04)
    0x0F, // byte 6: bits 48-51 (Blood gear 01-04)
    0x0F, // byte 7: bits 56-59 (Tempo gear 01-04)
    0x55, // byte 8: bits 64,66,68,70 (Tools -01: ST,SC,GR,BL)
    0x55, // byte 9: bits 72,74,76,78 (Tools -01: FR,RU,BO,TE)
];

/// Check if a specific bit is set in the bitmask.
///
/// # Arguments
/// * `mask` - The 80-bit bitmask (10 bytes)
/// * `index` - Bit index (0-79)
///
/// # Returns
/// `true` if the bit at `index` is set, `false` otherwise or if index is out of range
pub fn is_bit_set(mask: [u8; 10], index: u8) -> bool {
    if index >= 80 {
        return false;
    }
    let byte_idx = (index / 8) as usize;
    let bit_idx = index % 8;
    mask[byte_idx] & (1 << bit_idx) != 0
}

/// Set a specific bit in the bitmask.
///
/// # Arguments
/// * `mask` - Mutable reference to the 80-bit bitmask
/// * `index` - Bit index (0-79)
///
/// # Returns
/// `true` if the bit was successfully set, `false` if index is out of range
pub fn set_bit(mask: &mut [u8; 10], index: u8) -> bool {
    if index >= 80 {
        return false;
    }
    let byte_idx = (index / 8) as usize;
    let bit_idx = index % 8;
    mask[byte_idx] |= 1 << bit_idx;
    true
}

/// Clear a specific bit in the bitmask.
///
/// # Arguments
/// * `mask` - Mutable reference to the 80-bit bitmask
/// * `index` - Bit index (0-79)
///
/// # Returns
/// `true` if the bit was successfully cleared, `false` if index is out of range
pub fn clear_bit(mask: &mut [u8; 10], index: u8) -> bool {
    if index >= 80 {
        return false;
    }
    let byte_idx = (index / 8) as usize;
    let bit_idx = index % 8;
    mask[byte_idx] &= !(1 << bit_idx);
    true
}

/// Count the number of set bits in the bitmask.
///
/// # Arguments
/// * `mask` - The 80-bit bitmask
///
/// # Returns
/// Number of bits that are set (0-80)
pub fn count_bits(mask: [u8; 10]) -> u8 {
    mask.iter().map(|b| b.count_ones() as u8).sum()
}

/// Check if `pool` is a subset of `unlocked`.
/// A pool is a valid subset if every set bit in pool is also set in unlocked.
///
/// # Arguments
/// * `pool` - The active item pool bitmask
/// * `unlocked` - The unlocked items bitmask
///
/// # Returns
/// `true` if every item in pool is also in unlocked
pub fn is_subset(pool: [u8; 10], unlocked: [u8; 10]) -> bool {
    for i in 0..10 {
        // If pool has any bit set that unlocked doesn't have, it's not a subset
        if pool[i] & !unlocked[i] != 0 {
            return false;
        }
    }
    true
}

/// Select a random locked item from all 80 indices (0-79).
/// Uses deterministic PRNG based on player, level, and slot.
///
/// # Arguments
/// * `unlocked_items` - Current unlocked items bitmask
/// * `player` - Player's public key for PRNG seed
/// * `level` - Completed level for PRNG seed
/// * `slot` - Solana slot for PRNG seed
///
/// # Returns
/// `Some(index)` of a randomly selected locked item, or `None` if all items are unlocked
pub fn select_random_locked_item(
    unlocked_items: [u8; 10],
    player: &Pubkey,
    level: u8,
    slot: u64,
) -> Option<u8> {
    // Find all locked items in range 0-79
    let mut locked_items: [u8; 80] = [0; 80];
    let mut locked_count: usize = 0;

    for index in 0..80u8 {
        if !is_bit_set(unlocked_items, index) {
            locked_items[locked_count] = index;
            locked_count += 1;
        }
    }

    if locked_count == 0 {
        return None;
    }

    // Create deterministic seed from player, level, and slot
    let mut seed_data = [0u8; 41]; // 32 (pubkey) + 1 (level) + 8 (slot)
    seed_data[..32].copy_from_slice(player.as_ref());
    seed_data[32] = level;
    seed_data[33..41].copy_from_slice(&slot.to_le_bytes());

    // Simple hash: XOR all bytes and use modulo
    let hash: u64 = seed_data.iter().fold(0u64, |acc, &byte| {
        acc.wrapping_mul(31).wrapping_add(byte as u64)
    });

    let selected_idx = (hash % locked_count as u64) as usize;
    Some(locked_items[selected_idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_bit_set_returns_correct_values() {
        let mask: [u8; 10] = [0b00000001, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(is_bit_set(mask, 0));
        assert!(!is_bit_set(mask, 1));
        assert!(!is_bit_set(mask, 8));

        let mask2: [u8; 10] = [0, 0b10000000, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(is_bit_set(mask2, 15)); // 8th bit of 2nd byte = index 15
        assert!(!is_bit_set(mask2, 14));
    }

    #[test]
    fn test_is_bit_set_out_of_range() {
        let mask: [u8; 10] = [0xFF; 10];
        assert!(!is_bit_set(mask, 80));
        assert!(!is_bit_set(mask, 100));
        assert!(!is_bit_set(mask, 255));
    }

    #[test]
    fn test_set_bit_correctly_sets_bits() {
        let mut mask: [u8; 10] = [0; 10];

        assert!(set_bit(&mut mask, 0));
        assert_eq!(mask[0], 0b00000001);

        assert!(set_bit(&mut mask, 7));
        assert_eq!(mask[0], 0b10000001);

        assert!(set_bit(&mut mask, 8));
        assert_eq!(mask[1], 0b00000001);

        assert!(set_bit(&mut mask, 79));
        assert_eq!(mask[9], 0b10000000);
    }

    #[test]
    fn test_set_bit_out_of_range() {
        let mut mask: [u8; 10] = [0; 10];
        assert!(!set_bit(&mut mask, 80));
        assert!(!set_bit(&mut mask, 255));
        assert_eq!(mask, [0; 10]); // No change
    }

    #[test]
    fn test_count_bits_returns_correct_count() {
        let empty: [u8; 10] = [0; 10];
        assert_eq!(count_bits(empty), 0);

        let full: [u8; 10] = [0xFF; 10];
        assert_eq!(count_bits(full), 80);

        let starter = STARTER_ITEMS_BITMASK;
        assert_eq!(count_bits(starter), 40);

        let one_bit: [u8; 10] = [0b00000001, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(count_bits(one_bit), 1);
    }

    #[test]
    fn test_is_subset_correctly_validates_subsets() {
        let unlocked = STARTER_ITEMS_BITMASK;
        let pool = STARTER_ITEMS_BITMASK;

        // Same bitmask is a valid subset
        assert!(is_subset(pool, unlocked));

        // Empty pool is always a valid subset
        let empty: [u8; 10] = [0; 10];
        assert!(is_subset(empty, unlocked));

        // Subset with fewer items (only Stone gear 01-04)
        let subset: [u8; 10] = [0x0F, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(is_subset(subset, unlocked));

        // Pool with item not in unlocked is invalid (bit 19 = G-GR-04 is NOT in starter)
        let invalid: [u8; 10] = [0x0F, 0x0F, 0x1F, 0, 0, 0, 0, 0, 0, 0]; // 0x1F sets bit 19
        assert!(!is_subset(invalid, unlocked));
    }

    #[test]
    fn test_starter_items_bitmask_has_exactly_40_bits_set() {
        assert_eq!(count_bits(STARTER_ITEMS_BITMASK), 40);

        // Expected starter item indices:
        // Gear (32 items): 0-3 (Stone), 8-11 (Scout), 16-18,20 (Greed), 24-27 (Blast),
        //                  32-35 (Frost), 40-43 (Rust), 48-51 (Blood), 56-59 (Tempo)
        // Tools (8 items): 64,66,68,70,72,74,76,78 (all -01 tools)
        let expected_set: [u8; 40] = [
            0, 1, 2, 3,       // Stone gear 01-04
            8, 9, 10, 11,     // Scout gear 01-04
            16, 17, 18, 20,   // Greed gear 01-03, 05 (skips 04)
            24, 25, 26, 27,   // Blast gear 01-04
            32, 33, 34, 35,   // Frost gear 01-04
            40, 41, 42, 43,   // Rust gear 01-04
            48, 49, 50, 51,   // Blood gear 01-04
            56, 57, 58, 59,   // Tempo gear 01-04
            64, 66, 68, 70,   // Tools -01: ST, SC, GR, BL
            72, 74, 76, 78,   // Tools -01: FR, RU, BO, TE
        ];

        for &i in &expected_set {
            assert!(
                is_bit_set(STARTER_ITEMS_BITMASK, i),
                "Bit {} should be set",
                i
            );
        }

        // Verify non-starter bits are NOT set
        for i in 0..80u8 {
            if !expected_set.contains(&i) {
                assert!(
                    !is_bit_set(STARTER_ITEMS_BITMASK, i),
                    "Bit {} should not be set",
                    i
                );
            }
        }
    }

    #[test]
    fn test_select_random_locked_item_returns_locked_item() {
        let unlocked = STARTER_ITEMS_BITMASK; // 40 specific items set
        let player = Pubkey::new_unique();
        let level = 5u8;
        let slot = 12345u64;

        let result = select_random_locked_item(unlocked, &player, level, slot);
        assert!(result.is_some());

        let index = result.unwrap();
        assert!(
            index < 80,
            "Index {} should be in range 0-79",
            index
        );
        assert!(
            !is_bit_set(unlocked, index),
            "Selected item at index {} should be locked",
            index
        );
    }

    #[test]
    fn test_select_random_locked_item_returns_none_when_all_unlocked() {
        let all_unlocked: [u8; 10] = [0xFF; 10];
        let player = Pubkey::new_unique();
        let level = 5u8;
        let slot = 12345u64;

        let result = select_random_locked_item(all_unlocked, &player, level, slot);
        assert!(result.is_none());
    }

    #[test]
    fn test_select_random_locked_item_is_deterministic() {
        let unlocked = STARTER_ITEMS_BITMASK;
        let player = Pubkey::new_unique();
        let level = 5u8;
        let slot = 12345u64;

        let result1 = select_random_locked_item(unlocked, &player, level, slot);
        let result2 = select_random_locked_item(unlocked, &player, level, slot);

        assert_eq!(result1, result2, "Same inputs should produce same output");
    }

    #[test]
    fn test_select_random_locked_item_varies_with_different_seeds() {
        let unlocked = STARTER_ITEMS_BITMASK;
        let player = Pubkey::new_unique();

        // Different slots should (usually) produce different results
        let result1 = select_random_locked_item(unlocked, &player, 1, 100);
        let result2 = select_random_locked_item(unlocked, &player, 1, 200);
        let result3 = select_random_locked_item(unlocked, &player, 2, 100);

        // At least one should differ (statistically very likely with 40 options)
        let all_same = result1 == result2 && result2 == result3;
        // Note: This could theoretically fail but is extremely unlikely
        assert!(
            !all_same || result1.is_none(),
            "Different seeds should produce varied results"
        );
    }

    #[test]
    fn test_clear_bit() {
        let mut mask: [u8; 10] = [0xFF; 10];

        assert!(clear_bit(&mut mask, 0));
        assert_eq!(mask[0], 0b11111110);

        assert!(clear_bit(&mut mask, 79));
        assert_eq!(mask[9], 0b01111111);

        assert!(!clear_bit(&mut mask, 80)); // Out of range
    }
}
