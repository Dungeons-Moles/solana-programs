//! Itemset definitions for the player-inventory program.
//!
//! Contains all 12 itemset definitions as compile-time constants.

use crate::state::{EffectDefinition, EffectType, PlayerInventory, TriggerType};

/// Static itemset definition (compile-time constant)
#[derive(Clone, Copy, Debug)]
pub struct ItemsetDefinition {
    /// Unique ID (e.g., "union-standard")
    pub id: &'static str,
    /// Display name (e.g., "Union Standard")
    pub name: &'static str,
    /// Visual indicator emoji
    pub emoji: &'static str,
    /// Required item IDs (3-4 items)
    pub required_items: &'static [[u8; 8]],
    /// The set bonus effect
    pub bonus_effect: &'static [EffectDefinition],
}

// =============================================================================
// Itemset Definitions (12 total)
// =============================================================================

/// Union Standard: G-ST-01, G-ST-02, G-SC-01
/// Bonus: Battle Start +4 Armor, +1 DIG
pub const UNION_STANDARD: ItemsetDefinition = ItemsetDefinition {
    id: "union-standard",
    name: "Union Standard",
    emoji: "🧰",
    required_items: &[*b"G-ST-01\0", *b"G-ST-02\0", *b"G-SC-01\0"],
    bonus_effect: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainArmor,
            false,
            [4, 4, 4],
        ),
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainDig,
            false,
            [1, 1, 1],
        ),
    ],
};

/// Shard Circuit: G-GR-05, G-GR-06, G-GR-07, G-GR-08
/// Bonus: Shards trigger every turn (represented as extra gold per turn)
pub const SHARD_CIRCUIT: ItemsetDefinition = ItemsetDefinition {
    id: "shard-circuit",
    name: "Shard Circuit",
    emoji: "💎",
    required_items: &[*b"G-GR-05\0", *b"G-GR-06\0", *b"G-GR-07\0", *b"G-GR-08\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::GainGold,
        false,
        [3, 3, 3],
    )],
};

/// Demolition Permit: G-BL-01, G-BL-02, G-BL-03
/// Bonus: Bombs tick 1 turn faster (extra bomb damage)
pub const DEMOLITION_PERMIT: ItemsetDefinition = ItemsetDefinition {
    id: "demolition-permit",
    name: "Demolition Permit",
    emoji: "💣",
    required_items: &[*b"G-BL-01\0", *b"G-BL-02\0", *b"G-BL-03\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::DealNonWeaponDamage,
        false,
        [2, 2, 2],
    )],
};

/// Fuse Network: T-BL-02, G-BL-05, G-BL-04
/// Bonus: First non-weapon damage +2
pub const FUSE_NETWORK: ItemsetDefinition = ItemsetDefinition {
    id: "fuse-network",
    name: "Fuse Network",
    emoji: "🔥",
    required_items: &[*b"T-BL-02\0", *b"G-BL-05\0", *b"G-BL-04\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::FirstTurn,
        EffectType::DealNonWeaponDamage,
        true,
        [2, 2, 2],
    )],
};

/// Shrapnel Harness: G-ST-03, G-ST-06, T-ST-01
/// Bonus: Keep up to 3 Shrapnel at turn end (extra shrapnel gain)
pub const SHRAPNEL_HARNESS: ItemsetDefinition = ItemsetDefinition {
    id: "shrapnel-harness",
    name: "Shrapnel Harness",
    emoji: "🔩",
    required_items: &[*b"G-ST-03\0", *b"G-ST-06\0", *b"T-ST-01\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::ApplyShrapnel,
        false,
        [3, 3, 3],
    )],
};

/// Rust Ritual: T-RU-01, G-RU-02, G-RU-03
/// Bonus: On Hit +1 extra Rust
pub const RUST_RITUAL: ItemsetDefinition = ItemsetDefinition {
    id: "rust-ritual",
    name: "Rust Ritual",
    emoji: "🦠",
    required_items: &[*b"T-RU-01\0", *b"G-RU-02\0", *b"G-RU-03\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::OnHit,
        EffectType::ApplyRust,
        true,
        [1, 1, 1],
    )],
};

/// Swift Digger Kit: T-SC-01, G-SC-01, G-SC-06
/// Bonus: If DIG > enemy DIG: +2 strikes (represented as bonus damage)
pub const SWIFT_DIGGER_KIT: ItemsetDefinition = ItemsetDefinition {
    id: "swift-digger-kit",
    name: "Swift Digger Kit",
    emoji: "⛏️",
    required_items: &[*b"T-SC-01\0", *b"G-SC-01\0", *b"G-SC-06\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainDig,
        false,
        [3, 3, 3],
    )],
};

/// Royal Extraction: G-GR-01, G-GR-04, T-GR-02
/// Bonus: Gold→Armor becomes 1→4 (extra armor from gold)
pub const ROYAL_EXTRACTION: ItemsetDefinition = ItemsetDefinition {
    id: "royal-extraction",
    name: "Royal Extraction",
    emoji: "👑",
    required_items: &[*b"G-GR-01\0", *b"G-GR-04\0", *b"T-GR-02\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::GainArmor,
        false,
        [2, 2, 2],
    )],
};

/// Whiteout Initiative: G-FR-04, G-FR-03, G-TE-05
/// Bonus: Battle Start +1 SPD; act first Turn 1: +2 Chill
pub const WHITEOUT_INITIATIVE: ItemsetDefinition = ItemsetDefinition {
    id: "whiteout-initiative",
    name: "Whiteout Initiative",
    emoji: "❄️",
    required_items: &[*b"G-FR-04\0", *b"G-FR-03\0", *b"G-TE-05\0"],
    bonus_effect: &[
        EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [1, 1, 1],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::ApplyChill,
            false,
            [2, 2, 2],
        ),
    ],
};

/// Bloodrush Protocol: T-BO-01, G-BO-05, G-TE-01
/// Bonus: Turn 1: 2 Bleed; Bleed dmg: +1 SPD
pub const BLOODRUSH_PROTOCOL: ItemsetDefinition = ItemsetDefinition {
    id: "bloodrush-protocol",
    name: "Bloodrush Protocol",
    emoji: "🩸",
    required_items: &[*b"T-BO-01\0", *b"G-BO-05\0", *b"G-TE-01\0"],
    bonus_effect: &[
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::ApplyBleed,
            false,
            [2, 2, 2],
        ),
        EffectDefinition::new(
            TriggerType::FirstTurn,
            EffectType::GainSpd,
            false,
            [1, 1, 1],
        ),
    ],
};

/// Corrosion Payload: G-RU-02, G-BL-03, G-BL-05
/// Bonus: Bomb damage applies 1 Rust
pub const CORROSION_PAYLOAD: ItemsetDefinition = ItemsetDefinition {
    id: "corrosion-payload",
    name: "Corrosion Payload",
    emoji: "☢️",
    required_items: &[*b"G-RU-02\0", *b"G-BL-03\0", *b"G-BL-05\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyRust,
        false,
        [1, 1, 1],
    )],
};

/// Golden Shrapnel Exchange: G-GR-04, G-ST-06, G-GR-03
/// Bonus: Gold→Armor also gives +3 Shrapnel
pub const GOLDEN_SHRAPNEL_EXCHANGE: ItemsetDefinition = ItemsetDefinition {
    id: "golden-shrapnel-exchange",
    name: "Golden Shrapnel Exchange",
    emoji: "✨",
    required_items: &[*b"G-GR-04\0", *b"G-ST-06\0", *b"G-GR-03\0"],
    bonus_effect: &[EffectDefinition::new(
        TriggerType::TurnStart,
        EffectType::ApplyShrapnel,
        false,
        [3, 3, 3],
    )],
};

// =============================================================================
// All Itemsets Array (12 total)
// =============================================================================

/// All 12 itemset definitions as a compile-time constant array
pub const ITEMSETS: &[ItemsetDefinition] = &[
    UNION_STANDARD,
    SHARD_CIRCUIT,
    DEMOLITION_PERMIT,
    FUSE_NETWORK,
    SHRAPNEL_HARNESS,
    RUST_RITUAL,
    SWIFT_DIGGER_KIT,
    ROYAL_EXTRACTION,
    WHITEOUT_INITIATIVE,
    BLOODRUSH_PROTOCOL,
    CORROSION_PAYLOAD,
    GOLDEN_SHRAPNEL_EXCHANGE,
];

// =============================================================================
// Query Functions
// =============================================================================

/// Get an itemset by its ID
pub fn get_itemset(id: &str) -> Option<&'static ItemsetDefinition> {
    ITEMSETS.iter().find(|s| s.id == id)
}

/// Get all active itemsets for a player's inventory
pub fn get_active_itemsets(inventory: &PlayerInventory) -> Vec<&'static ItemsetDefinition> {
    ITEMSETS
        .iter()
        .filter(|set| set.required_items.iter().all(|id| inventory.has_item(id)))
        .collect()
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ItemInstance, Tier};
    use anchor_lang::prelude::Pubkey;

    #[test]
    fn test_total_itemset_count() {
        assert_eq!(ITEMSETS.len(), 12, "Should have exactly 12 itemsets");
    }

    #[test]
    fn test_get_itemset_by_id() {
        let set = get_itemset("union-standard");
        assert!(set.is_some());
        let set = set.unwrap();
        assert_eq!(set.name, "Union Standard");
        assert_eq!(set.required_items.len(), 3);
    }

    #[test]
    fn test_union_standard_requirements() {
        let set = get_itemset("union-standard").unwrap();
        assert_eq!(set.required_items[0], *b"G-ST-01\0");
        assert_eq!(set.required_items[1], *b"G-ST-02\0");
        assert_eq!(set.required_items[2], *b"G-SC-01\0");
    }

    #[test]
    fn test_active_itemsets_none() {
        let inventory = PlayerInventory {
            player: Pubkey::default(),
            tool: None,
            gear: [None; 8],
            gear_slot_capacity: 4,
            bump: 0,
        };

        let active = get_active_itemsets(&inventory);
        assert_eq!(
            active.len(),
            0,
            "Empty inventory should have no active sets"
        );
    }

    #[test]
    fn test_active_itemsets_partial() {
        let mut inventory = PlayerInventory {
            player: Pubkey::default(),
            tool: None,
            gear: [None; 8],
            gear_slot_capacity: 4,
            bump: 0,
        };

        // Only 2 of 3 items for Union Standard
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I));

        let active = get_active_itemsets(&inventory);
        assert_eq!(active.len(), 0, "Partial set should not activate");
    }

    #[test]
    fn test_active_itemsets_complete() {
        let mut inventory = PlayerInventory {
            player: Pubkey::default(),
            tool: None,
            gear: [None; 8],
            gear_slot_capacity: 4,
            bump: 0,
        };

        // All 3 items for Union Standard
        inventory.gear[0] = Some(ItemInstance::new(*b"G-ST-01\0", Tier::I));
        inventory.gear[1] = Some(ItemInstance::new(*b"G-ST-02\0", Tier::I));
        inventory.gear[2] = Some(ItemInstance::new(*b"G-SC-01\0", Tier::I));

        let active = get_active_itemsets(&inventory);
        assert_eq!(active.len(), 1, "Complete set should activate");
        assert_eq!(active[0].id, "union-standard");
    }

    #[test]
    fn test_shard_circuit_requirements() {
        let set = get_itemset("shard-circuit").unwrap();
        assert_eq!(
            set.required_items.len(),
            4,
            "Shard Circuit requires 4 items"
        );
    }
}
