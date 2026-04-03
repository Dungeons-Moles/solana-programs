//! NFT item definitions for NFT-backed items.
//!
//! These items exist outside the base 80-item system and are backed by
//! Metaplex Core NFTs. They use IDs starting with "S-XX-" to avoid
//! collision with base items.

use crate::items::ItemDefinition;
use crate::state::{EffectDefinition, EffectType, ItemTag, ItemType, Rarity, TriggerType};

pub const NFT_ITEMS: &[ItemDefinition] = &[
    // S-XX-07: Pioneer's Mattock — Tool, Heroic
    // BattleStart: +3/4/5 ATK
    // OnHit (once/turn): apply rotating debuff
    // Turn 1: Chill, Turn 2: Rust, Turn 3: Bleed, then repeat
    ItemDefinition {
        id: b"S-XX-07\0",
        name: "Pioneer's Mattock",
        item_type: ItemType::Tool,
        tag: ItemTag::None,
        rarity: Rarity::Heroic,
        effects: &[
            EffectDefinition::new(
                TriggerType::BattleStart,
                EffectType::GainAtk,
                false,
                [3, 4, 5],
            ),
            EffectDefinition::new(
                TriggerType::OnHit,
                EffectType::ApplyRotatingDebuff,
                true,
                [1, 1, 1],
            ),
        ],
    },
    // S-XX-01: Infernal Pickaxe — Blast/Gear, Heroic
    // On TurnStart: deal 2/3/4 non-weapon damage
    ItemDefinition {
        id: b"S-XX-01\0",
        name: "Infernal Pickaxe",
        item_type: ItemType::Gear,
        tag: ItemTag::Blast,
        rarity: Rarity::Heroic,
        effects: &[EffectDefinition::new(
            TriggerType::TurnStart,
            EffectType::DealNonWeaponDamage,
            false,
            [2, 3, 4],
        )],
    },
    // S-XX-02: Frostbite Amulet — Frost/Gear, Heroic
    // BattleStart: +3/5/7 ARM, OnHit: apply 1 Chill
    ItemDefinition {
        id: b"S-XX-02\0",
        name: "Frostbite Amulet",
        item_type: ItemType::Gear,
        tag: ItemTag::Frost,
        rarity: Rarity::Heroic,
        effects: &[
            EffectDefinition::new(
                TriggerType::BattleStart,
                EffectType::GainArmor,
                false,
                [3, 5, 7],
            ),
            EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyChill, true, [1, 1, 2]),
        ],
    },
    // S-XX-03: Bloodstone Ring — Blood/Gear, Mythic
    // BattleStart: +2/3/4 ATK, OnHit: apply 2/3/4 Bleed
    ItemDefinition {
        id: b"S-XX-03\0",
        name: "Bloodstone Ring",
        item_type: ItemType::Gear,
        tag: ItemTag::Blood,
        rarity: Rarity::Mythic,
        effects: &[
            EffectDefinition::new(
                TriggerType::BattleStart,
                EffectType::GainAtk,
                false,
                [2, 3, 4],
            ),
            EffectDefinition::new(TriggerType::OnHit, EffectType::ApplyBleed, true, [2, 3, 4]),
        ],
    },
    // S-XX-04: Rusty Crown — Rust/Gear, Heroic
    // BattleStart: remove 3/5/7 enemy armor
    ItemDefinition {
        id: b"S-XX-04\0",
        name: "Rusty Crown",
        item_type: ItemType::Gear,
        tag: ItemTag::Rust,
        rarity: Rarity::Heroic,
        effects: &[EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::RemoveArmor,
            false,
            [3, 5, 7],
        )],
    },
    // S-XX-05: Golden Compass — Greed/Gear, Rare
    // BattleStart: +3/5/7 Gold, +1/2/3 DIG
    ItemDefinition {
        id: b"S-XX-05\0",
        name: "Golden Compass",
        item_type: ItemType::Gear,
        tag: ItemTag::Greed,
        rarity: Rarity::Rare,
        effects: &[
            EffectDefinition::new(
                TriggerType::BattleStart,
                EffectType::GainGold,
                false,
                [3, 5, 7],
            ),
            EffectDefinition::new(
                TriggerType::BattleStart,
                EffectType::GainDig,
                false,
                [1, 2, 3],
            ),
        ],
    },
    // S-XX-06: Tempo Boots — Tempo/Gear, Heroic
    // BattleStart: +3/5/7 SPD
    ItemDefinition {
        id: b"S-XX-06\0",
        name: "Tempo Boots",
        item_type: ItemType::Gear,
        tag: ItemTag::Tempo,
        rarity: Rarity::Heroic,
        effects: &[EffectDefinition::new(
            TriggerType::BattleStart,
            EffectType::GainSpd,
            false,
            [3, 5, 7],
        )],
    },
];

/// Look up an NFT item by its ID.
pub fn get_nft_item(item_id: &[u8; 8]) -> Option<&'static ItemDefinition> {
    NFT_ITEMS.iter().find(|item| item.id == item_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nft_item_count() {
        assert_eq!(NFT_ITEMS.len(), 7, "Should have exactly 7 relic items");
    }

    #[test]
    fn test_get_nft_item_by_id() {
        let item = get_nft_item(b"S-XX-07\0");
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.name, "Pioneer's Mattock");
        assert_eq!(item.item_type, ItemType::Tool);
        assert_eq!(item.tag, ItemTag::None);
        assert_eq!(item.rarity, Rarity::Heroic);
    }

    #[test]
    fn test_get_nft_item_invalid() {
        let item = get_nft_item(b"S-XX-99\0");
        assert!(item.is_none());
    }

    #[test]
    fn test_nft_items_no_collision_with_base() {
        use crate::items::get_item;
        for nft_item in NFT_ITEMS {
            assert!(
                get_item(nft_item.id).is_none(),
                "NFT item {} should not exist in base item registry",
                nft_item.name
            );
        }
    }

    #[test]
    fn test_relic_catalog_has_tools_and_gear() {
        assert!(NFT_ITEMS.iter().any(|item| item.item_type == ItemType::Tool));
        assert!(NFT_ITEMS.iter().any(|item| item.item_type == ItemType::Gear));
    }
}
