//! Item offer generation for POIs.
//!
//! Generates weighted item selections based on POI type, act, rarity tables, and boss weakness tags.

/// POI type enum for offer generation (maps to L-codes)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoiOfferType {
    /// L2: Supply Cache - 3 Gear items
    SupplyCache,
    /// L3: Tool Crate - 3 Tools
    ToolCrate,
    /// L9: Smuggler Hatch - 1 Tool + 5 Gear
    SmugglerHatch,
    /// L12: Geode Vault - 3 Heroic+ items
    GeodeVault,
    /// L13: Counter Cache - 3 weakness-only items
    CounterCache,
}

/// Item rarity for offer generation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemRarity {
    Common,
    Rare,
    Heroic,
    Mythic,
}

/// Item type for filtering
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemType {
    Tool,
    Gear,
}

// =============================================================================
// Rarity Tables (percentages for Common/Rare/Heroic/Mythic)
// =============================================================================

/// Supply Cache (L2) rarity table by act
pub const SUPPLY_CACHE_RARITY: [(u8, u8, u8, u8); 4] = [
    (100, 0, 0, 0), // Act 1: 100% Common
    (85, 15, 0, 0), // Act 2: 85% Common, 15% Rare
    (75, 25, 0, 0), // Act 3: 75% Common, 25% Rare
    (65, 35, 0, 0), // Act 4: 65% Common, 35% Rare
];

/// Tool Crate (L3) rarity table by act
pub const TOOL_CRATE_RARITY: [(u8, u8, u8, u8); 4] = [
    (85, 15, 0, 0),  // Act 1: 85% Common, 15% Rare
    (70, 25, 5, 0),  // Act 2: 70% Common, 25% Rare, 5% Heroic
    (60, 30, 10, 0), // Act 3: 60% Common, 30% Rare, 10% Heroic
    (50, 35, 15, 0), // Act 4: 50% Common, 35% Rare, 15% Heroic
];

/// Geode Vault (L12) rarity table by act (higher rarity)
pub const GEODE_VAULT_RARITY: [(u8, u8, u8, u8); 4] = [
    (0, 10, 90, 0),  // Act 1: 10% Rare, 90% Heroic
    (0, 10, 80, 10), // Act 2: 10% Rare, 80% Heroic, 10% Mythic
    (0, 5, 75, 20),  // Act 3: 5% Rare, 75% Heroic, 20% Mythic
    (0, 0, 70, 30),  // Act 4: 70% Heroic, 30% Mythic
];

/// Smuggler Hatch (L9) Gear rarity table by act
pub const SMUGGLER_GEAR_RARITY: [(u8, u8, u8, u8); 4] = [
    (70, 27, 3, 0),  // Act 1
    (55, 38, 7, 0),  // Act 2
    (45, 42, 13, 0), // Act 3
    (35, 45, 18, 2), // Act 4
];

/// Smuggler Hatch (L9) Tool rarity table by act
pub const SMUGGLER_TOOL_RARITY: [(u8, u8, u8, u8); 4] = [
    (80, 20, 0, 0),  // Act 1
    (65, 30, 5, 0),  // Act 2
    (55, 35, 10, 0), // Act 3
    (45, 40, 15, 0), // Act 4
];

/// Counter Cache (L13) rarity table (uses standard act table)
pub const COUNTER_CACHE_RARITY: [(u8, u8, u8, u8); 4] = [
    (60, 40, 0, 0),  // Act 1
    (40, 50, 10, 0), // Act 2
    (30, 45, 25, 0), // Act 3
    (20, 40, 35, 5), // Act 4
];

// =============================================================================
// Price Tables
// =============================================================================

/// Gear prices by rarity: [Common, Rare, Heroic, Mythic]
pub const GEAR_PRICES: [u16; 4] = [8, 14, 22, 34];

/// Tool prices by rarity: [Common, Rare, Heroic, Mythic]
pub const TOOL_PRICES: [u16; 4] = [10, 16, 24, 38];

/// Rusty Anvil upgrade costs: [I->II, II->III]
pub const ANVIL_UPGRADE_COSTS: [u16; 2] = [8, 16];

/// Scrap Chute costs by act
pub const SCRAP_CHUTE_COSTS: [u16; 4] = [8, 8, 10, 12];

/// Shop reroll base cost and increment
pub const REROLL_BASE_COST: u16 = 4;
pub const REROLL_INCREMENT: u16 = 2;

// =============================================================================
// Offer Generation Functions
// =============================================================================

/// Get rarity from a rarity table using a seed
pub fn get_rarity_from_table(table: &[(u8, u8, u8, u8); 4], act: u8, seed: u64) -> ItemRarity {
    let act_index = (act.saturating_sub(1) as usize).min(3);
    let (common, rare, heroic, _mythic) = table[act_index];

    // Use seed to get a random value 0-99
    let roll = (seed % 100) as u8;

    if roll < common {
        ItemRarity::Common
    } else if roll < common + rare {
        ItemRarity::Rare
    } else if roll < common + rare + heroic {
        ItemRarity::Heroic
    } else {
        ItemRarity::Mythic
    }
}

/// Calculate price for an item based on type and rarity
pub fn calculate_price(item_type: ItemType, rarity: ItemRarity) -> u16 {
    let index = match rarity {
        ItemRarity::Common => 0,
        ItemRarity::Rare => 1,
        ItemRarity::Heroic => 2,
        ItemRarity::Mythic => 3,
    };

    match item_type {
        ItemType::Tool => TOOL_PRICES[index],
        ItemType::Gear => GEAR_PRICES[index],
    }
}

/// Calculate reroll cost based on reroll count
pub fn calculate_reroll_cost(reroll_count: u8) -> u16 {
    REROLL_BASE_COST + (REROLL_INCREMENT * reroll_count as u16)
}

/// Calculate scrap chute cost for an act
pub fn calculate_scrap_cost(act: u8) -> u16 {
    let act_index = (act.saturating_sub(1) as usize).min(3);
    SCRAP_CHUTE_COSTS[act_index]
}

/// Calculate anvil upgrade cost for a tier upgrade
pub fn calculate_anvil_cost(current_tier: u8) -> Option<u16> {
    match current_tier {
        1 => Some(ANVIL_UPGRADE_COSTS[0]), // I -> II
        2 => Some(ANVIL_UPGRADE_COSTS[1]), // II -> III
        _ => None,                         // Already max tier
    }
}

/// Get the rarity table for a POI type
pub fn get_rarity_table(poi_type: PoiOfferType, is_tool: bool) -> &'static [(u8, u8, u8, u8); 4] {
    match poi_type {
        PoiOfferType::SupplyCache => &SUPPLY_CACHE_RARITY,
        PoiOfferType::ToolCrate => &TOOL_CRATE_RARITY,
        PoiOfferType::GeodeVault => &GEODE_VAULT_RARITY,
        PoiOfferType::CounterCache => &COUNTER_CACHE_RARITY,
        PoiOfferType::SmugglerHatch => {
            if is_tool {
                &SMUGGLER_TOOL_RARITY
            } else {
                &SMUGGLER_GEAR_RARITY
            }
        }
    }
}

/// Tag index for weight calculations (matches ItemTag order)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaknessTag {
    Stone = 0,
    Scout = 1,
    Greed = 2,
    Blast = 3,
    Frost = 4,
    Rust = 5,
    Blood = 6,
    Tempo = 7,
}

impl TryFrom<u8> for WeaknessTag {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(WeaknessTag::Stone),
            1 => Ok(WeaknessTag::Scout),
            2 => Ok(WeaknessTag::Greed),
            3 => Ok(WeaknessTag::Blast),
            4 => Ok(WeaknessTag::Frost),
            5 => Ok(WeaknessTag::Rust),
            6 => Ok(WeaknessTag::Blood),
            7 => Ok(WeaknessTag::Tempo),
            _ => Err(()),
        }
    }
}

/// Base weight for non-weakness tags
pub const BASE_WEIGHT: u32 = 100;

/// Weight multiplier for boss weakness tags (1.4x = 140%)
pub const WEAKNESS_WEIGHT: u32 = 140;

/// Calculate tag weights based on boss weaknesses
pub fn calculate_tag_weights(weakness1: WeaknessTag, weakness2: WeaknessTag) -> [u32; 8] {
    let mut weights = [BASE_WEIGHT; 8];
    weights[weakness1 as usize] = WEAKNESS_WEIGHT;
    weights[weakness2 as usize] = WEAKNESS_WEIGHT;
    weights
}

// =============================================================================
// Offer Generation (generates item IDs based on POI type)
// =============================================================================

use crate::state::ItemOffer;

/// Generated offer set for a POI
#[derive(Clone, Debug)]
pub struct GeneratedOffers {
    /// Item offers generated
    pub offers: Vec<ItemOffer>,
    /// Number of offers to pick from (usually 3 for pick POIs, 6 for shop)
    pub pick_count: u8,
}

/// Generate item offers for a POI interaction.
///
/// This is a wrapper that delegates to specific POI offer generators.
///
/// Arguments:
/// - `poi_type`: The L-code (2, 3, 9, 12, 13) for item-giving POIs
/// - `act`: Current act (1-4)
/// - `weakness1`, `weakness2`: Boss weakness tags for weighting
/// - `seed`: RNG seed for deterministic selection
pub fn generate_poi_offers(
    poi_type: u8,
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> Option<GeneratedOffers> {
    match poi_type {
        2 => Some(generate_supply_cache_offers(
            act, weakness1, weakness2, seed,
        )),
        3 => Some(generate_tool_crate_offers(act, weakness1, weakness2, seed)),
        9 => Some(generate_smuggler_hatch_offers(
            act, weakness1, weakness2, seed,
        )),
        12 => Some(generate_geode_vault_offers(act, weakness1, weakness2, seed)),
        13 => Some(generate_counter_cache_offers(
            act, weakness1, weakness2, seed,
        )),
        _ => None, // Not an item-giving POI
    }
}

/// Generate 3 Gear offers for Supply Cache (L2).
/// Uses SUPPLY_CACHE_RARITY table for act-based rarity.
pub fn generate_supply_cache_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(3);

    for i in 0..3 {
        let item_seed = seed.wrapping_add(i as u64 * 1337);
        let rarity = get_rarity_from_table(&SUPPLY_CACHE_RARITY, act, item_seed);
        let item_id = select_gear_by_rarity_weighted(rarity, &tag_weights, item_seed >> 8);

        offers.push(ItemOffer {
            item_id,
            tier: 1,  // Tier::I
            price: 0, // Free POI
            purchased: false,
        });
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 3 Tool offers for Tool Crate (L3).
/// Uses TOOL_CRATE_RARITY table (includes Heroic from Act 2+).
pub fn generate_tool_crate_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(3);

    for i in 0..3 {
        let item_seed = seed.wrapping_add(i as u64 * 1337);
        let rarity = get_rarity_from_table(&TOOL_CRATE_RARITY, act, item_seed);
        let item_id = select_tool_by_rarity_weighted(rarity, &tag_weights, item_seed >> 8);

        offers.push(ItemOffer {
            item_id,
            tier: 1,
            price: 0,
            purchased: false,
        });
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 3 Heroic+ offers for Geode Vault (L12).
/// Uses GEODE_VAULT_RARITY table (higher rarity chance, includes Mythic).
pub fn generate_geode_vault_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(3);

    for i in 0..3 {
        let item_seed = seed.wrapping_add(i as u64 * 1337);
        let rarity = get_rarity_from_table(&GEODE_VAULT_RARITY, act, item_seed);
        let item_id = select_gear_by_rarity_weighted(rarity, &tag_weights, item_seed >> 8);

        offers.push(ItemOffer {
            item_id,
            tier: 1,
            price: 0,
            purchased: false,
        });
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 3 weakness-tagged offers for Counter Cache (L13).
/// Only offers items matching boss weakness tags.
pub fn generate_counter_cache_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let mut offers = Vec::with_capacity(3);

    for i in 0..3 {
        let item_seed = seed.wrapping_add(i as u64 * 1337);
        let rarity = get_rarity_from_table(&COUNTER_CACHE_RARITY, act, item_seed);

        // Select only from weakness tags
        let use_tag1 = (item_seed >> 16) % 2 == 0;
        let tag = if use_tag1 { weakness1 } else { weakness2 };
        let item_id = select_gear_by_tag_and_rarity(tag, rarity, item_seed >> 24);

        offers.push(ItemOffer {
            item_id,
            tier: 1,
            price: 0,
            purchased: false,
        });
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 6 offers for Smuggler Hatch shop (L9): 1 Tool + 5 Gear.
/// Uses SMUGGLER_TOOL_RARITY and SMUGGLER_GEAR_RARITY tables.
/// Items have prices based on rarity.
pub fn generate_smuggler_hatch_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(6);

    // First offer: 1 Tool
    {
        let item_seed = seed;
        let rarity = get_rarity_from_table(&SMUGGLER_TOOL_RARITY, act, item_seed);
        let item_id = select_tool_by_rarity_weighted(rarity, &tag_weights, item_seed >> 8);
        let price = calculate_price(ItemType::Tool, rarity);

        offers.push(ItemOffer {
            item_id,
            tier: 1,
            price,
            purchased: false,
        });
    }

    // Next 5 offers: Gear
    for i in 0..5 {
        let item_seed = seed.wrapping_add((i + 1) as u64 * 1337);
        let rarity = get_rarity_from_table(&SMUGGLER_GEAR_RARITY, act, item_seed);
        let item_id = select_gear_by_rarity_weighted(rarity, &tag_weights, item_seed >> 8);
        let price = calculate_price(ItemType::Gear, rarity);

        offers.push(ItemOffer {
            item_id,
            tier: 1,
            price,
            purchased: false,
        });
    }

    GeneratedOffers {
        offers,
        pick_count: 6, // Can purchase multiple
    }
}

// =============================================================================
// Item Selection Helpers
// =============================================================================

/// Item tag names for ID generation (T-XX-NN, G-XX-NN)
const TAG_CODES: [&str; 8] = ["ST", "SC", "GR", "BL", "FR", "RU", "BO", "TE"];

/// Select a gear item by rarity with tag weighting.
fn select_gear_by_rarity_weighted(
    rarity: ItemRarity,
    tag_weights: &[u32; 8],
    seed: u64,
) -> [u8; 8] {
    // Calculate total weight
    let total_weight: u64 = tag_weights.iter().map(|w| *w as u64).sum();
    let roll = seed % total_weight;

    // Select tag based on weights
    let mut cumulative = 0u64;
    let mut selected_tag = 0usize;
    for (i, &weight) in tag_weights.iter().enumerate() {
        cumulative += weight as u64;
        if roll < cumulative {
            selected_tag = i;
            break;
        }
    }

    // Select item within tag based on rarity
    // Gear items: G-XX-01 to G-XX-08
    // Common: 01, 02 (2 items)
    // Rare: 03, 04, 05 (3 items)
    // Heroic: 06, 07 (2 items)
    // Mythic: 08 (1 item)
    let item_num = match rarity {
        ItemRarity::Common => ((seed >> 8) % 2) as u8 + 1, // 01 or 02
        ItemRarity::Rare => ((seed >> 8) % 3) as u8 + 3,   // 03, 04, or 05
        ItemRarity::Heroic => ((seed >> 8) % 2) as u8 + 6, // 06 or 07
        ItemRarity::Mythic => 8,                           // 08
    };

    format_gear_id(TAG_CODES[selected_tag], item_num)
}

/// Select a tool item by rarity with tag weighting.
fn select_tool_by_rarity_weighted(
    rarity: ItemRarity,
    tag_weights: &[u32; 8],
    seed: u64,
) -> [u8; 8] {
    // Calculate total weight
    let total_weight: u64 = tag_weights.iter().map(|w| *w as u64).sum();
    let roll = seed % total_weight;

    // Select tag based on weights
    let mut cumulative = 0u64;
    let mut selected_tag = 0usize;
    for (i, &weight) in tag_weights.iter().enumerate() {
        cumulative += weight as u64;
        if roll < cumulative {
            selected_tag = i;
            break;
        }
    }

    // Tools: T-XX-01 (Common), T-XX-02 (Rare)
    // No Heroic/Mythic tools, fall back to Rare
    let item_num = match rarity {
        ItemRarity::Common => 1,
        ItemRarity::Rare | ItemRarity::Heroic | ItemRarity::Mythic => 2,
    };

    format_tool_id(TAG_CODES[selected_tag], item_num)
}

/// Select a gear item by specific tag and rarity.
fn select_gear_by_tag_and_rarity(tag: WeaknessTag, rarity: ItemRarity, seed: u64) -> [u8; 8] {
    let tag_code = TAG_CODES[tag as usize];

    let item_num = match rarity {
        ItemRarity::Common => ((seed >> 8) % 2) as u8 + 1,
        ItemRarity::Rare => ((seed >> 8) % 3) as u8 + 3,
        ItemRarity::Heroic => ((seed >> 8) % 2) as u8 + 6,
        ItemRarity::Mythic => 8,
    };

    format_gear_id(tag_code, item_num)
}

/// Format a gear item ID (e.g., "G-ST-01\0")
fn format_gear_id(tag: &str, num: u8) -> [u8; 8] {
    let mut id = [0u8; 8];
    id[0] = b'G';
    id[1] = b'-';
    id[2] = tag.as_bytes()[0];
    id[3] = tag.as_bytes()[1];
    id[4] = b'-';
    id[5] = b'0' + (num / 10);
    id[6] = b'0' + (num % 10);
    id[7] = 0; // null terminator
    id
}

/// Format a tool item ID (e.g., "T-ST-01\0")
fn format_tool_id(tag: &str, num: u8) -> [u8; 8] {
    let mut id = [0u8; 8];
    id[0] = b'T';
    id[1] = b'-';
    id[2] = tag.as_bytes()[0];
    id[3] = tag.as_bytes()[1];
    id[4] = b'-';
    id[5] = b'0' + (num / 10);
    id[6] = b'0' + (num % 10);
    id[7] = 0;
    id
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supply_cache_act1_always_common() {
        for seed in 0..100 {
            let rarity = get_rarity_from_table(&SUPPLY_CACHE_RARITY, 1, seed);
            assert_eq!(
                rarity,
                ItemRarity::Common,
                "Act 1 seed {} should be Common",
                seed
            );
        }
    }

    #[test]
    fn test_supply_cache_act2_distribution() {
        let mut common_count = 0;
        let mut rare_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(&SUPPLY_CACHE_RARITY, 2, seed);
            match rarity {
                ItemRarity::Common => common_count += 1,
                ItemRarity::Rare => rare_count += 1,
                _ => panic!("Act 2 should only have Common or Rare"),
            }
        }

        assert_eq!(common_count, 85);
        assert_eq!(rare_count, 15);
    }

    #[test]
    fn test_tool_crate_includes_heroic_in_act2() {
        let mut heroic_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(&TOOL_CRATE_RARITY, 2, seed);
            if rarity == ItemRarity::Heroic {
                heroic_count += 1;
            }
        }

        assert_eq!(heroic_count, 5, "Act 2 Tool Crate should have 5% Heroic");
    }

    #[test]
    fn test_geode_vault_heroic_dominant() {
        let mut heroic_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(&GEODE_VAULT_RARITY, 1, seed);
            if rarity == ItemRarity::Heroic {
                heroic_count += 1;
            }
        }

        assert_eq!(heroic_count, 90, "Act 1 Geode Vault should be 90% Heroic");
    }

    #[test]
    fn test_geode_vault_mythic_in_act4() {
        let mut mythic_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(&GEODE_VAULT_RARITY, 4, seed);
            if rarity == ItemRarity::Mythic {
                mythic_count += 1;
            }
        }

        assert_eq!(mythic_count, 30, "Act 4 Geode Vault should have 30% Mythic");
    }

    #[test]
    fn test_gear_prices() {
        assert_eq!(calculate_price(ItemType::Gear, ItemRarity::Common), 8);
        assert_eq!(calculate_price(ItemType::Gear, ItemRarity::Rare), 14);
        assert_eq!(calculate_price(ItemType::Gear, ItemRarity::Heroic), 22);
        assert_eq!(calculate_price(ItemType::Gear, ItemRarity::Mythic), 34);
    }

    #[test]
    fn test_tool_prices() {
        assert_eq!(calculate_price(ItemType::Tool, ItemRarity::Common), 10);
        assert_eq!(calculate_price(ItemType::Tool, ItemRarity::Rare), 16);
        assert_eq!(calculate_price(ItemType::Tool, ItemRarity::Heroic), 24);
        assert_eq!(calculate_price(ItemType::Tool, ItemRarity::Mythic), 38);
    }

    #[test]
    fn test_reroll_cost_progression() {
        assert_eq!(calculate_reroll_cost(0), 4);
        assert_eq!(calculate_reroll_cost(1), 6);
        assert_eq!(calculate_reroll_cost(2), 8);
        assert_eq!(calculate_reroll_cost(3), 10);
        assert_eq!(calculate_reroll_cost(4), 12);
    }

    #[test]
    fn test_scrap_chute_costs() {
        assert_eq!(calculate_scrap_cost(1), 8);
        assert_eq!(calculate_scrap_cost(2), 8);
        assert_eq!(calculate_scrap_cost(3), 10);
        assert_eq!(calculate_scrap_cost(4), 12);
    }

    #[test]
    fn test_anvil_upgrade_costs() {
        assert_eq!(calculate_anvil_cost(1), Some(8));
        assert_eq!(calculate_anvil_cost(2), Some(16));
        assert_eq!(calculate_anvil_cost(3), None);
    }

    #[test]
    fn test_tag_weight_calculation() {
        let weights = calculate_tag_weights(WeaknessTag::Stone, WeaknessTag::Frost);

        assert_eq!(weights[0], WEAKNESS_WEIGHT); // Stone
        assert_eq!(weights[1], BASE_WEIGHT); // Scout
        assert_eq!(weights[4], WEAKNESS_WEIGHT); // Frost
        assert_eq!(weights[7], BASE_WEIGHT); // Tempo
    }

    #[test]
    fn test_smuggler_hatch_separate_tables() {
        let tool_table = get_rarity_table(PoiOfferType::SmugglerHatch, true);
        let gear_table = get_rarity_table(PoiOfferType::SmugglerHatch, false);

        // Act 1: Tool 80% Common, Gear 70% Common
        assert_eq!(tool_table[0].0, 80);
        assert_eq!(gear_table[0].0, 70);
    }

    // =========================================================================
    // Offer Generation Tests
    // =========================================================================

    #[test]
    fn test_generate_supply_cache_offers_count() {
        let offers = generate_supply_cache_offers(1, WeaknessTag::Stone, WeaknessTag::Frost, 12345);
        assert_eq!(offers.offers.len(), 3, "Should generate 3 offers");
        assert_eq!(offers.pick_count, 1, "Should pick 1");
    }

    #[test]
    fn test_generate_supply_cache_offers_are_gear() {
        let offers = generate_supply_cache_offers(1, WeaknessTag::Stone, WeaknessTag::Frost, 12345);
        for offer in &offers.offers {
            assert_eq!(
                offer.item_id[0], b'G',
                "Supply Cache should only offer Gear"
            );
        }
    }

    #[test]
    fn test_generate_tool_crate_offers_are_tools() {
        let offers = generate_tool_crate_offers(2, WeaknessTag::Stone, WeaknessTag::Frost, 12345);
        assert_eq!(offers.offers.len(), 3, "Should generate 3 offers");
        for offer in &offers.offers {
            assert_eq!(offer.item_id[0], b'T', "Tool Crate should only offer Tools");
        }
    }

    #[test]
    fn test_generate_geode_vault_offers_count() {
        let offers = generate_geode_vault_offers(4, WeaknessTag::Stone, WeaknessTag::Frost, 12345);
        assert_eq!(offers.offers.len(), 3, "Should generate 3 offers");
        assert_eq!(offers.pick_count, 1, "Should pick 1");
    }

    #[test]
    fn test_generate_counter_cache_weakness_only() {
        // This is T022: Counter Cache should ONLY offer items from weakness tags
        for seed in 0..100 {
            let offers = generate_counter_cache_offers(
                2,
                WeaknessTag::Stone, // ST
                WeaknessTag::Frost, // FR
                seed * 12345,
            );

            for offer in &offers.offers {
                let tag_code = std::str::from_utf8(&offer.item_id[2..4]).unwrap();
                assert!(
                    tag_code == "ST" || tag_code == "FR",
                    "Counter Cache item {} should only have Stone or Frost tag, got {}",
                    std::str::from_utf8(&offer.item_id[..7]).unwrap_or("?"),
                    tag_code
                );
            }
        }
    }

    #[test]
    fn test_generate_smuggler_hatch_offers_structure() {
        let offers =
            generate_smuggler_hatch_offers(2, WeaknessTag::Greed, WeaknessTag::Blast, 12345);

        assert_eq!(
            offers.offers.len(),
            6,
            "Smuggler Hatch should generate 6 offers"
        );
        assert_eq!(
            offers.pick_count, 6,
            "Smuggler Hatch allows multiple purchases"
        );

        // First offer should be a Tool
        assert_eq!(
            offers.offers[0].item_id[0], b'T',
            "First offer should be Tool"
        );

        // Remaining 5 should be Gear
        for offer in &offers.offers[1..] {
            assert_eq!(offer.item_id[0], b'G', "Offers 2-6 should be Gear");
        }

        // All should have prices > 0
        for offer in &offers.offers {
            assert!(offer.price > 0, "Smuggler Hatch items should have prices");
        }
    }

    #[test]
    fn test_generate_poi_offers_valid_pois() {
        // Valid item POIs
        assert!(generate_poi_offers(2, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_some());
        assert!(generate_poi_offers(3, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_some());
        assert!(generate_poi_offers(9, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_some());
        assert!(generate_poi_offers(12, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_some());
        assert!(generate_poi_offers(13, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_some());
    }

    #[test]
    fn test_generate_poi_offers_invalid_pois() {
        // Non-item POIs should return None
        assert!(generate_poi_offers(1, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_none()); // Mole Den
        assert!(generate_poi_offers(4, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_none()); // Tool Oil
        assert!(generate_poi_offers(5, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_none()); // Rest Alcove
        assert!(generate_poi_offers(6, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_none()); // Survey Beacon
        assert!(generate_poi_offers(8, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_none());
        // Rail Waypoint
    }

    #[test]
    fn test_format_gear_id() {
        let id = format_gear_id("ST", 1);
        assert_eq!(&id, b"G-ST-01\0");

        let id = format_gear_id("FR", 8);
        assert_eq!(&id, b"G-FR-08\0");
    }

    #[test]
    fn test_format_tool_id() {
        let id = format_tool_id("SC", 2);
        assert_eq!(&id, b"T-SC-02\0");
    }

    #[test]
    fn test_offers_are_free_for_pickup_pois() {
        // Supply Cache, Tool Crate, Geode Vault, Counter Cache should be free
        let supply = generate_supply_cache_offers(1, WeaknessTag::Stone, WeaknessTag::Frost, 123);
        let tool = generate_tool_crate_offers(1, WeaknessTag::Stone, WeaknessTag::Frost, 123);
        let geode = generate_geode_vault_offers(1, WeaknessTag::Stone, WeaknessTag::Frost, 123);
        let counter = generate_counter_cache_offers(1, WeaknessTag::Stone, WeaknessTag::Frost, 123);

        for offer in supply
            .offers
            .iter()
            .chain(tool.offers.iter())
            .chain(geode.offers.iter())
            .chain(counter.offers.iter())
        {
            assert_eq!(offer.price, 0, "Pickup POI items should be free");
        }
    }
}
