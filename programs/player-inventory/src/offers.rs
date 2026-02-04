//! Item offer generation for the player-inventory program.
//!
//! Generates weighted item offers for POIs based on act rarity table and boss weaknesses.

use crate::items::{get_items_by_rarity, get_items_by_type, ItemDefinition, ITEMS};
use crate::state::{ItemOffer, ItemTag, ItemType, PoiType, Rarity, Tier};

/// Base weight for non-weakness tags
const BASE_WEIGHT: u32 = 100;

/// Weight multiplier for boss weakness tags (1.4x = 140%)
const WEAKNESS_WEIGHT: u32 = 140;

/// Rarity drop table by act (percentages for Common/Rare/Heroic/Mythic)
/// Each tuple is (common%, rare%, heroic%, mythic%)
const ACT_RARITY_TABLE: [(u8, u8, u8, u8); 4] = [
    (100, 0, 0, 0),   // Act 1: 100% Common
    (60, 40, 0, 0),   // Act 2: 60% Common, 40% Rare
    (30, 50, 20, 0),  // Act 3: 30% Common, 50% Rare, 20% Heroic
    (20, 40, 30, 10), // Act 4: 20% Common, 40% Rare, 30% Heroic, 10% Mythic
];

/// Geode Vault rarity table (higher rarity chance)
const GEODE_RARITY_TABLE: [(u8, u8, u8, u8); 4] = [
    (70, 30, 0, 0),   // Act 1
    (40, 45, 15, 0),  // Act 2
    (15, 40, 35, 10), // Act 3
    (10, 30, 40, 20), // Act 4
];

/// Get the rarity for an offer based on POI type and act
pub fn get_rarity_from_table(poi_type: PoiType, act: u8, seed: u64) -> Rarity {
    let table = match poi_type {
        PoiType::GeodeVault => &GEODE_RARITY_TABLE,
        _ => &ACT_RARITY_TABLE,
    };

    let act_index = (act.saturating_sub(1) as usize).min(3);
    let (common, rare, heroic, _mythic) = table[act_index];

    // Use seed to get a random value 0-99
    let roll = (seed % 100) as u8;

    if roll < common {
        Rarity::Common
    } else if roll < common + rare {
        Rarity::Rare
    } else if roll < common + rare + heroic {
        Rarity::Heroic
    } else {
        Rarity::Mythic
    }
}

/// Calculate tag weights based on boss weaknesses
/// Takes two non-None ItemTags representing the boss's weaknesses
pub fn calculate_tag_weights(boss_weaknesses: (ItemTag, ItemTag)) -> [u32; 8] {
    let mut weights = [BASE_WEIGHT; 8];

    // Apply 1.4x weight to weakness tags
    // Use tag_index_safe which properly maps tags to weight array indices
    weights[tag_index_for_weight(boss_weaknesses.0)] = WEAKNESS_WEIGHT;
    weights[tag_index_for_weight(boss_weaknesses.1)] = WEAKNESS_WEIGHT;

    weights
}

/// Get the index of a tag in the weight array (0-7 for Stone-Tempo)
/// Panics for ItemTag::None as it should never appear in offer selection
fn tag_index(tag: ItemTag) -> usize {
    match tag {
        ItemTag::None => unreachable!(
            "ItemTag::None should never appear in offer selection - this indicates a bug"
        ),
        ItemTag::Stone => 0,
        ItemTag::Scout => 1,
        ItemTag::Greed => 2,
        ItemTag::Blast => 3,
        ItemTag::Frost => 4,
        ItemTag::Rust => 5,
        ItemTag::Blood => 6,
        ItemTag::Tempo => 7,
    }
}

/// Get the index of a tag in the weight array for weight calculation
/// Returns the same indices as tag_index for valid tags, panics for None
fn tag_index_for_weight(tag: ItemTag) -> usize {
    tag_index(tag)
}

/// Weighted selection from candidates using deterministic seed
pub fn weighted_select<'a>(
    candidates: &[&'a ItemDefinition],
    tag_weights: &[u32; 8],
    seed: u64,
) -> Option<&'a ItemDefinition> {
    if candidates.is_empty() {
        return None;
    }

    // Calculate total weight
    let total_weight: u64 = candidates
        .iter()
        .map(|item| tag_weights[tag_index(item.tag)] as u64)
        .sum();

    if total_weight == 0 {
        return candidates.first().copied();
    }

    // Use seed to select
    let roll = seed % total_weight;
    let mut cumulative = 0u64;

    for item in candidates {
        cumulative += tag_weights[tag_index(item.tag)] as u64;
        if roll < cumulative {
            return Some(item);
        }
    }

    candidates.last().copied()
}

/// Generate an item offer for a POI
pub fn generate_item_offer(
    poi_type: PoiType,
    act: u8,
    _week: u8,
    boss_weaknesses: (ItemTag, ItemTag),
    seed: u64,
) -> Option<ItemOffer> {
    // 1. Determine rarity from table
    let rarity = get_rarity_from_table(poi_type, act, seed);

    // 2. Get candidate items based on POI type
    let candidates: Vec<&ItemDefinition> = match poi_type {
        PoiType::ToolCrate => {
            // Only tools of the selected rarity
            get_items_by_type(ItemType::Tool)
                .into_iter()
                .filter(|i| i.rarity == rarity)
                .collect()
        }
        PoiType::CounterCache => {
            // Only items from boss weakness tags
            ITEMS
                .iter()
                .filter(|i| {
                    i.rarity == rarity && (i.tag == boss_weaknesses.0 || i.tag == boss_weaknesses.1)
                })
                .collect()
        }
        _ => {
            // All items of the selected rarity
            get_items_by_rarity(rarity)
        }
    };

    if candidates.is_empty() {
        // Fallback to any item of the rarity
        let fallback = get_items_by_rarity(rarity);
        if fallback.is_empty() {
            return None;
        }
        let item = fallback[(seed as usize) % fallback.len()];
        return Some(ItemOffer {
            item_id: *item.id,
            tier: Tier::I,
            price: calculate_price(item),
        });
    }

    // 3. Calculate weights
    let tag_weights = calculate_tag_weights(boss_weaknesses);

    // 4. Select item
    let selected = weighted_select(&candidates, &tag_weights, seed >> 8)?;

    Some(ItemOffer {
        item_id: *selected.id,
        tier: Tier::I,
        price: calculate_price(selected),
    })
}

/// Calculate the price for an item based on rarity and type
fn calculate_price(item: &ItemDefinition) -> u16 {
    match (item.item_type, item.rarity) {
        (ItemType::Tool, Rarity::Common) => 10,
        (ItemType::Tool, Rarity::Rare) => 18,
        (ItemType::Tool, Rarity::Heroic) => 28,
        (ItemType::Tool, Rarity::Mythic) => 40,
        (ItemType::Gear, Rarity::Common) => 8,
        (ItemType::Gear, Rarity::Rare) => 14,
        (ItemType::Gear, Rarity::Heroic) => 22,
        (ItemType::Gear, Rarity::Mythic) => 34,
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_act1_always_common() {
        for seed in 0..100 {
            let rarity = get_rarity_from_table(PoiType::SupplyCache, 1, seed);
            assert_eq!(rarity, Rarity::Common, "Act 1 should always be Common");
        }
    }

    #[test]
    fn test_act2_common_or_rare() {
        let mut common_count = 0;
        let mut rare_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(PoiType::SupplyCache, 2, seed);
            match rarity {
                Rarity::Common => common_count += 1,
                Rarity::Rare => rare_count += 1,
                _ => panic!("Act 2 should only have Common or Rare"),
            }
        }

        assert_eq!(common_count, 60);
        assert_eq!(rare_count, 40);
    }

    #[test]
    fn test_tag_weight_calculation() {
        let weights = calculate_tag_weights((ItemTag::Stone, ItemTag::Frost));

        assert_eq!(weights[0], WEAKNESS_WEIGHT); // Stone
        assert_eq!(weights[1], BASE_WEIGHT); // Scout
        assert_eq!(weights[4], WEAKNESS_WEIGHT); // Frost
        assert_eq!(weights[7], BASE_WEIGHT); // Tempo
    }

    #[test]
    fn test_counter_cache_only_weakness_tags() {
        let offer = generate_item_offer(
            PoiType::CounterCache,
            1,
            1,
            (ItemTag::Stone, ItemTag::Frost),
            12345,
        );

        assert!(offer.is_some());
        let offer = offer.unwrap();

        // Get the item to check its tag
        let item = crate::items::get_item(&offer.item_id);
        assert!(item.is_some());
        let item = item.unwrap();

        assert!(
            item.tag == ItemTag::Stone || item.tag == ItemTag::Frost,
            "Counter Cache should only offer weakness-tagged items"
        );
    }

    #[test]
    fn test_tool_crate_only_tools() {
        for seed in 0..50 {
            let offer = generate_item_offer(
                PoiType::ToolCrate,
                2, // Need act 2+ for rare tools
                1,
                (ItemTag::Stone, ItemTag::Frost),
                seed * 1000,
            );

            if let Some(offer) = offer {
                let item = crate::items::get_item(&offer.item_id).unwrap();
                assert_eq!(
                    item.item_type,
                    ItemType::Tool,
                    "Tool Crate should only offer Tools"
                );
            }
        }
    }

    #[test]
    fn test_geode_vault_higher_rarity() {
        let mut heroic_geode = 0;
        let mut heroic_supply = 0;

        for seed in 0..1000 {
            if get_rarity_from_table(PoiType::GeodeVault, 3, seed) == Rarity::Heroic {
                heroic_geode += 1;
            }
            if get_rarity_from_table(PoiType::SupplyCache, 3, seed) == Rarity::Heroic {
                heroic_supply += 1;
            }
        }

        assert!(
            heroic_geode > heroic_supply,
            "Geode Vault should have higher Heroic rate"
        );
    }

    #[test]
    fn test_weakness_bias_statistical() {
        // Generate 1000 offers and check bias
        let mut stone_frost_count = 0;
        let mut other_count = 0;

        for seed in 0..1000 {
            let offer = generate_item_offer(
                PoiType::SupplyCache,
                1,
                1,
                (ItemTag::Stone, ItemTag::Frost),
                seed * 12345,
            );

            if let Some(offer) = offer {
                let item = crate::items::get_item(&offer.item_id).unwrap();
                if item.tag == ItemTag::Stone || item.tag == ItemTag::Frost {
                    stone_frost_count += 1;
                } else {
                    other_count += 1;
                }
            }
        }

        // With 1.4x weight, weakness tags should appear more often
        // 2 tags with 1.4x weight vs 6 tags with 1.0x weight
        // Expected ratio: (2 * 1.4) / (2 * 1.4 + 6 * 1.0) = 2.8 / 8.8 = 31.8%
        // Allow some variance
        let total = stone_frost_count + other_count;
        let weakness_ratio = stone_frost_count as f64 / total as f64;

        assert!(
            weakness_ratio > 0.25,
            "Weakness tags should appear at least 25% of the time (got {:.1}%)",
            weakness_ratio * 100.0
        );
    }

    #[test]
    fn test_mythic_cap() {
        // In Act 4 with Geode Vault, mythic chance is 20%
        let mut mythic_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(PoiType::GeodeVault, 4, seed);
            if rarity == Rarity::Mythic {
                mythic_count += 1;
            }
        }

        // Should be around 20 mythics out of 100
        assert!(
            mythic_count >= 15 && mythic_count <= 25,
            "Mythic rate should be around 20% (got {}%)",
            mythic_count
        );
    }

    #[test]
    fn test_price_calculation() {
        use crate::items::get_item;

        // Common tool (T-ST-01)
        let bulwark = get_item(b"T-ST-01\0").unwrap();
        assert_eq!(calculate_price(bulwark), 10);

        // Common gear (G-ST-01)
        let helmet = get_item(b"G-ST-01\0").unwrap();
        assert_eq!(calculate_price(helmet), 8);

        // Common gear (G-ST-03) - per GDD Spiked Bracers is Common
        let bracers = get_item(b"G-ST-03\0").unwrap();
        assert_eq!(calculate_price(bracers), 8);

        // Rare gear (G-RU-02) - Rust Spike is Rare per GDD
        let rust_spike = get_item(b"G-RU-02\0").unwrap();
        assert_eq!(calculate_price(rust_spike), 14);

        // Mythic gear (G-SC-08) - Phantom Pickaxe is Mythic
        let phantom = get_item(b"G-SC-08\0").unwrap();
        assert_eq!(calculate_price(phantom), 34); // Mythic gear = 34
    }
}
