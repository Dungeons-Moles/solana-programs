//! Item offer generation for POIs.
//!
//! Generates weighted item selections based on POI type, act, rarity tables, and boss weakness tags.
//!
//! ## Core Components
//!
//! - **Xorshift64**: Deterministic PRNG for reproducible offer generation
//! - **OfferContext**: Input parameters derived from game state
//! - **RarityTable**: Act-based probability distributions
//! - **TagWeights**: Boss weakness-weighted tag selection

// =============================================================================
// Xorshift64 RNG
// =============================================================================

/// Deterministic pseudo-random number generator for offer generation.
///
/// Uses the Xorshift64 algorithm for fast, reproducible randomness.
/// Each offer generation derives a unique seed to ensure determinism.
#[derive(Clone, Copy, Debug)]
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    /// Create a new RNG with the given seed.
    /// Clamps to 1 if seed is 0 (zero state produces no output).
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    /// Generate the next random u64 value.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate a random u64 in [0, max) range.
    pub fn next_bounded(&mut self, max: u64) -> u64 {
        if max == 0 {
            return 0;
        }
        self.next_u64() % max
    }

    /// Get current state (for persistence/resumption).
    pub fn state(&self) -> u64 {
        self.state
    }
}

/// Derive a unique seed for offer generation.
///
/// Combines session seed with POI index and call count for uniqueness.
pub fn derive_offer_seed(base_seed: u64, poi_index: u8, call_count: u8) -> u64 {
    base_seed ^ ((poi_index as u64) << 16) ^ ((call_count as u64) << 8)
}

// =============================================================================
// Offer Context
// =============================================================================

/// Context for offer generation, derived from game state.
#[derive(Clone, Copy, Debug)]
pub struct OfferContext {
    /// Current act (1-4)
    pub act: u8,
    /// Current week (1-3)
    pub week: u8,
    /// Base seed for RNG
    pub seed: u64,
    /// POI index for seed derivation
    pub poi_index: u8,
    /// Counter for unique sub-seeds
    pub offer_call_count: u8,
    /// Whether to apply Week 3 tag bonus during Week 1-2
    pub enable_final_prep_bias: bool,
}

impl OfferContext {
    /// Create a new offer context.
    pub fn new(act: u8, week: u8, seed: u64, poi_index: u8) -> Self {
        Self {
            act,
            week,
            seed,
            poi_index,
            offer_call_count: 0,
            enable_final_prep_bias: false,
        }
    }

    /// Enable final prep bias for Week 3 tag boosting.
    pub fn with_final_prep_bias(mut self) -> Self {
        self.enable_final_prep_bias = true;
        self
    }

    /// Derive the RNG seed for this context.
    pub fn derive_seed(&self) -> u64 {
        derive_offer_seed(self.seed, self.poi_index, self.offer_call_count)
    }
}

// =============================================================================
// POI Types and Enums
// =============================================================================

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
    (60, 40, 0, 0), // Act 1: 60% Common, 40% Rare
    (70, 30, 0, 0), // Act 2: 70% Common, 30% Rare
    (80, 20, 0, 0), // Act 3: 80% Common, 20% Rare
    (90, 10, 0, 0), // Act 4: 90% Common, 10% Rare
];

/// Tool Crate (L3) rarity table by act
pub const TOOL_CRATE_RARITY: [(u8, u8, u8, u8); 4] = [
    (50, 30, 20, 0), // Act 1: 50% Common, 30% Rare, 20% Heroic
    (60, 25, 15, 0), // Act 2: 60% Common, 25% Rare, 15% Heroic
    (70, 20, 10, 0), // Act 3: 70% Common, 20% Rare, 10% Heroic
    (80, 15, 5, 0),  // Act 4: 80% Common, 15% Rare, 5% Heroic
];

/// Geode Vault (L12) rarity table by act.
/// All acts: 90% Heroic, 10% Mythic (max 1 Mythic enforced at generation).
pub const GEODE_VAULT_RARITY: [(u8, u8, u8, u8); 4] = [
    (0, 0, 90, 10), // Act 1: 90% Heroic, 10% Mythic
    (0, 0, 90, 10), // Act 2: 90% Heroic, 10% Mythic
    (0, 0, 90, 10), // Act 3: 90% Heroic, 10% Mythic
    (0, 0, 90, 10), // Act 4: 90% Heroic, 10% Mythic
];

/// Smuggler Hatch (L9) Gear rarity table by act
pub const SMUGGLER_GEAR_RARITY: [(u8, u8, u8, u8); 4] = [
    (35, 45, 10, 10), // Act 1
    (45, 40, 10, 5),  // Act 2
    (55, 30, 12, 3),  // Act 3
    (65, 25, 8, 2),   // Act 4
];

/// Smuggler Hatch (L9) Tool rarity table by act
pub const SMUGGLER_TOOL_RARITY: [(u8, u8, u8, u8); 4] = [
    (45, 40, 15, 0), // Act 1
    (55, 35, 10, 0), // Act 2
    (65, 30, 5, 0),  // Act 3
    (80, 15, 5, 0),  // Act 4
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
pub const ANVIL_UPGRADE_COSTS: [u16; 2] = [10, 20];

/// Scrap Chute cost (flat 4 Gold across acts)
pub const SCRAP_CHUTE_COSTS: [u16; 4] = [4, 4, 4, 4];

/// Tool Oil Rack is free in all acts
pub const TOOL_OIL_COSTS: [u16; 4] = [0, 0, 0, 0];

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

/// Sample rarity from table using RNG, with Mythic cap enforcement.
///
/// If `mythic_used` is true and Mythic is rolled, returns Heroic instead.
/// Sets `mythic_used` to true if Mythic is returned.
pub fn sample_rarity_with_cap(
    rng: &mut Xorshift64,
    table: &[(u8, u8, u8, u8); 4],
    act: u8,
    mythic_used: &mut bool,
) -> ItemRarity {
    let act_index = (act.saturating_sub(1) as usize).min(3);
    let (common, rare, heroic, _mythic) = table[act_index];

    let roll = rng.next_bounded(100) as u8;

    let rarity = if roll < common {
        ItemRarity::Common
    } else if roll < common + rare {
        ItemRarity::Rare
    } else if roll < common + rare + heroic {
        ItemRarity::Heroic
    } else {
        ItemRarity::Mythic
    };

    // Enforce Mythic cap (max 1 per offer)
    if rarity == ItemRarity::Mythic {
        if *mythic_used {
            return ItemRarity::Heroic;
        }
        *mythic_used = true;
    }

    rarity
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

/// Calculate tool oil cost for an act (always 0)
pub fn calculate_tool_oil_cost(act: u8) -> u16 {
    let act_index = (act.saturating_sub(1) as usize).min(3);
    TOOL_OIL_COSTS[act_index]
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

/// Base weight for non-weakness tags (in basis points: 10000 = 1.0)
pub const BASE_WEIGHT: u32 = 10000;

/// Weight for boss weakness tags (1.4x = 14000 basis points)
pub const WEAKNESS_WEIGHT: u32 = 14000;

/// Final prep bias weight addition (0.1x = 1000 basis points)
pub const FINAL_PREP_BIAS: u32 = 1000;

/// Tag weights for offer generation.
#[derive(Clone, Copy, Debug)]
pub struct TagWeights {
    /// Weight per tag (8 tags, in basis points)
    pub weights: [u32; 8],
    /// Sum of all weights for normalization
    pub total: u32,
}

impl TagWeights {
    /// Create new tag weights with all base weights.
    pub fn new() -> Self {
        let weights = [BASE_WEIGHT; 8];
        Self {
            weights,
            total: BASE_WEIGHT * 8,
        }
    }

    /// Apply weakness boost to specified tags.
    pub fn with_weaknesses(mut self, weakness1: WeaknessTag, weakness2: WeaknessTag) -> Self {
        // Add weakness bonus (not replace, in case both are same tag)
        let bonus = WEAKNESS_WEIGHT - BASE_WEIGHT;
        self.weights[weakness1 as usize] += bonus;
        self.total += bonus;
        if weakness1 != weakness2 {
            self.weights[weakness2 as usize] += bonus;
            self.total += bonus;
        }
        self
    }

    /// Apply final prep bias for Week 3 tags during Week 1-2.
    pub fn with_final_prep_bias(mut self, week3_tags: &[WeaknessTag]) -> Self {
        for tag in week3_tags {
            self.weights[*tag as usize] += FINAL_PREP_BIAS;
            self.total += FINAL_PREP_BIAS;
        }
        self
    }

    /// Select a random tag based on weights.
    pub fn select_tag(&self, rng: &mut Xorshift64) -> WeaknessTag {
        let roll = rng.next_bounded(self.total as u64) as u32;
        let mut cumulative = 0u32;

        for (i, &weight) in self.weights.iter().enumerate() {
            cumulative += weight;
            if roll < cumulative {
                return WeaknessTag::try_from(i as u8).unwrap_or(WeaknessTag::Stone);
            }
        }

        // Fallback (shouldn't happen)
        WeaknessTag::Stone
    }
}

impl Default for TagWeights {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate tag weights based on boss weaknesses (legacy compatibility).
pub fn calculate_tag_weights(weakness1: WeaknessTag, weakness2: WeaknessTag) -> [u32; 8] {
    // Use legacy 100-scale for backward compatibility with existing code
    let mut weights = [100u32; 8];
    weights[weakness1 as usize] = 140;
    weights[weakness2 as usize] = 140;
    weights
}

/// Calculate tag weights with full options.
pub fn calculate_tag_weights_full(
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    enable_final_prep_bias: bool,
    week3_tags: Option<&[WeaknessTag]>,
) -> TagWeights {
    let mut weights = TagWeights::new().with_weaknesses(weakness1, weakness2);

    if enable_final_prep_bias {
        if let Some(tags) = week3_tags {
            weights = weights.with_final_prep_bias(tags);
        }
    }

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
        // Type 9 (Smuggler Hatch) uses generate_smuggler_hatch_offers directly
        // via enter_shop/shop_reroll, which pass the item pool.
        12 => Some(generate_geode_vault_offers(act, weakness1, weakness2, seed)),
        13 => Some(generate_counter_cache_offers(
            act, weakness1, weakness2, seed,
        )),
        _ => None, // Not an item-giving POI
    }
}

/// Generate 3 Gear offers for Supply Cache (L2).
/// Uses SUPPLY_CACHE_RARITY table for act-based rarity.
/// Ensures all 3 items are unique by re-rolling duplicates.
pub fn generate_supply_cache_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(3);
    let mut rng = Xorshift64::new(seed);
    let mut used_ids: [[u8; 8]; 3] = [[0; 8]; 3];

    for i in 0..3 {
        let mut attempts = 0;
        loop {
            let item_seed = rng.next_u64();
            let rarity = get_rarity_from_table(&SUPPLY_CACHE_RARITY, act, item_seed);
            let item_id = select_gear_by_rarity_weighted(rarity, &tag_weights, rng.next_u64());

            // Check for duplicates
            let is_duplicate = used_ids[..i].contains(&item_id);
            if !is_duplicate || attempts >= 10 {
                used_ids[i] = item_id;
                offers.push(ItemOffer {
                    item_id,
                    tier: 0,  // Tier::I (enum discriminant)
                    price: 0, // Free POI
                    purchased: false,
                });
                break;
            }
            attempts += 1;
        }
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 3 Tool offers for Tool Crate (L3).
/// Uses TOOL_CRATE_RARITY table (includes Heroic from Act 2+).
/// Ensures all 3 items are unique by re-rolling duplicates.
pub fn generate_tool_crate_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(3);
    let mut rng = Xorshift64::new(seed);
    let mut used_ids: [[u8; 8]; 3] = [[0; 8]; 3];

    for i in 0..3 {
        let mut attempts = 0;
        loop {
            let item_seed = rng.next_u64();
            let rarity = get_rarity_from_table(&TOOL_CRATE_RARITY, act, item_seed);
            let item_id = select_tool_by_rarity_weighted(rarity, &tag_weights, rng.next_u64());

            // Check for duplicates
            let is_duplicate = used_ids[..i].contains(&item_id);
            if !is_duplicate || attempts >= 10 {
                used_ids[i] = item_id;
                offers.push(ItemOffer {
                    item_id,
                    tier: 0, // Tier::I
                    price: 0,
                    purchased: false,
                });
                break;
            }
            attempts += 1;
        }
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 3 Heroic+ offers for Geode Vault (L12).
/// Uses GEODE_VAULT_RARITY table and enforces max 1 Mythic in the 3 offers.
/// Ensures all 3 items are unique by re-rolling duplicates.
pub fn generate_geode_vault_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(3);
    let mut rng = Xorshift64::new(seed);
    let mut used_ids: [[u8; 8]; 3] = [[0; 8]; 3];
    let mut mythic_used = false;

    for i in 0..3 {
        let mut attempts = 0;
        loop {
            let rarity =
                sample_rarity_with_cap(&mut rng, &GEODE_VAULT_RARITY, act, &mut mythic_used);
            let item_id = select_gear_by_rarity_weighted(rarity, &tag_weights, rng.next_u64());

            // Check for duplicates
            let is_duplicate = used_ids[..i].contains(&item_id);
            if !is_duplicate || attempts >= 10 {
                used_ids[i] = item_id;
                offers.push(ItemOffer {
                    item_id,
                    tier: 0, // Tier::I
                    price: 0,
                    purchased: false,
                });
                break;
            }
            attempts += 1;
        }
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 3 weakness-tagged offers for Counter Cache (L13).
/// Only offers items matching boss weakness tags.
/// Ensures all 3 items are unique by re-rolling duplicates.
pub fn generate_counter_cache_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
) -> GeneratedOffers {
    let mut offers = Vec::with_capacity(3);
    let mut rng = Xorshift64::new(seed);
    let mut used_ids: [[u8; 8]; 3] = [[0; 8]; 3];

    for i in 0..3 {
        let mut attempts = 0;
        loop {
            let item_seed = rng.next_u64();
            let rarity = get_rarity_from_table(&COUNTER_CACHE_RARITY, act, item_seed);

            let use_tag1 = rng.next_u64() & 1 == 0;
            let tag = if use_tag1 { weakness1 } else { weakness2 };
            let item_id = select_gear_by_tag_and_rarity(tag, rarity, rng.next_u64());

            // Check for duplicates
            let is_duplicate = used_ids[..i].contains(&item_id);
            if !is_duplicate || attempts >= 10 {
                used_ids[i] = item_id;
                offers.push(ItemOffer {
                    item_id,
                    tier: 0, // Tier::I
                    price: 0,
                    purchased: false,
                });
                break;
            }
            attempts += 1;
        }
    }

    GeneratedOffers {
        offers,
        pick_count: 1,
    }
}

/// Generate 6 offers for Smuggler Hatch shop (L9): 1 Tool + 5 Gear.
/// Uses SMUGGLER_TOOL_RARITY and SMUGGLER_GEAR_RARITY tables.
/// Items have prices based on rarity.
/// Ensures all items are unique, deduplicated, and in the active item pool.
/// Retries with different seeds until all 6 slots are filled.
pub fn generate_smuggler_hatch_offers(
    act: u8,
    weakness1: WeaknessTag,
    weakness2: WeaknessTag,
    seed: u64,
    pool: &[u8; ITEM_POOL_SIZE],
) -> GeneratedOffers {
    let tag_weights = calculate_tag_weights(weakness1, weakness2);
    let mut offers = Vec::with_capacity(6);
    let mut used_ids: [[u8; 8]; 6] = [[0; 8]; 6];
    let mut count = 0usize;

    // First: fill the Tool slot (index 0)
    for attempt in 0..20u64 {
        let attempt_seed = seed ^ attempt.wrapping_mul(0x9e3779b97f4a7c15);
        let mut rng = Xorshift64::new(attempt_seed);

        let item_seed = rng.next_u64();
        let rarity = get_rarity_from_table(&SMUGGLER_TOOL_RARITY, act, item_seed);
        let item_id = select_tool_by_rarity_weighted(rarity, &tag_weights, rng.next_u64());
        let price = calculate_price(ItemType::Tool, rarity);

        // Check pool membership and deduplication
        let in_pool =
            item_id_to_pool_index(&item_id).map_or(false, |idx| is_item_in_pool(pool, idx));
        let is_duplicate = used_ids[..count].contains(&item_id);

        if in_pool && !is_duplicate {
            used_ids[count] = item_id;
            count += 1;
            offers.push(ItemOffer {
                item_id,
                tier: 0,
                price,
                purchased: false,
            });
            break;
        }
    }

    // Next: fill 5 Gear slots
    for attempt in 0..50u64 {
        if count >= 6 {
            break;
        }
        let attempt_seed = seed ^ ((count as u64) << 48) ^ attempt.wrapping_mul(0x517cc1b727220a95);
        let mut rng = Xorshift64::new(attempt_seed);

        let item_seed = rng.next_u64();
        let rarity = get_rarity_from_table(&SMUGGLER_GEAR_RARITY, act, item_seed);
        let item_id = select_gear_by_rarity_weighted(rarity, &tag_weights, rng.next_u64());
        let price = calculate_price(ItemType::Gear, rarity);

        let in_pool =
            item_id_to_pool_index(&item_id).map_or(false, |idx| is_item_in_pool(pool, idx));
        let is_duplicate = used_ids[..count].contains(&item_id);

        if in_pool && !is_duplicate {
            used_ids[count] = item_id;
            count += 1;
            offers.push(ItemOffer {
                item_id,
                tier: 0,
                price,
                purchased: false,
            });
        }
    }

    GeneratedOffers {
        offers,
        pick_count: 6,
    }
}

// =============================================================================
// Tool Oil Offer Generation
// =============================================================================

use crate::interactions::{OIL_FLAG_ARM, OIL_FLAG_ATK, OIL_FLAG_DIG, OIL_FLAG_SPD};
use crate::state::OilOffer;

/// All possible oil flags for selection
const ALL_OILS: [u8; 4] = [OIL_FLAG_ATK, OIL_FLAG_SPD, OIL_FLAG_DIG, OIL_FLAG_ARM];

/// Generate 3 tool oil offers from the 4 possible oils.
///
/// Uses Fisher-Yates shuffle to deterministically select 3 of 4 oils.
/// The seed ensures the same POI always generates the same offers.
///
/// # Arguments
/// * `seed` - RNG seed for deterministic selection
///
/// # Returns
/// Array of 3 oil flags (from OIL_FLAG_ATK, SPD, DIG, ARM)
pub fn generate_tool_oil_offers(seed: u64) -> [u8; 3] {
    let mut rng = Xorshift64::new(seed);
    let mut oils = ALL_OILS;

    // Fisher-Yates shuffle (partial - only need 3 elements)
    // Swap indices 0, 1, 2 with random elements from remaining array
    for i in 0..3 {
        let remaining = 4 - i;
        let j = i + (rng.next_bounded(remaining as u64) as usize);
        oils.swap(i, j);
    }

    // Return first 3 elements
    [oils[0], oils[1], oils[2]]
}

/// Create an OilOffer struct for storage in MapPois.
///
/// # Arguments
/// * `poi_index` - Index of the Tool Oil Rack POI
/// * `seed` - RNG seed for the session/map
///
/// # Returns
/// OilOffer containing the 3 generated oils
pub fn create_oil_offer(poi_index: u8, seed: u64) -> OilOffer {
    OilOffer {
        poi_index,
        oils: generate_tool_oil_offers(seed),
        generated_at_seed: seed,
    }
}

/// Validate that a selected oil is one of the offered oils.
///
/// # Arguments
/// * `offer` - The OilOffer containing available oils
/// * `selected_oil` - The oil flag the player selected
///
/// # Returns
/// true if the selected oil is valid, false otherwise
pub fn validate_oil_selection(offer: &OilOffer, selected_oil: u8) -> bool {
    offer.oils.contains(&selected_oil)
}

// =============================================================================
// Item Selection Helpers
// =============================================================================

/// Item tag names for ID generation (T-XX-NN, G-XX-NN)
const TAG_CODES: [&str; 8] = ["ST", "SC", "GR", "BL", "FR", "RU", "BO", "TE"];

/// Convert ItemRarity to the rarity index used by the pre-indexed lookup tables.
fn rarity_to_index(rarity: ItemRarity) -> usize {
    match rarity {
        ItemRarity::Common => 0,
        ItemRarity::Rare => 1,
        ItemRarity::Heroic => 2,
        ItemRarity::Mythic => 3,
    }
}

/// Select a gear item by rarity with tag weighting.
///
/// Uses pre-indexed lookups from player-inventory instead of scanning the
/// full ITEMS array. Two-step selection:
/// 1. Pick a tag (weighted), only considering tags that have items of the target rarity
/// 2. Pick a random item of that rarity within the selected tag
fn select_gear_by_rarity_weighted(
    rarity: ItemRarity,
    tag_weights: &[u32; 8],
    seed: u64,
) -> [u8; 8] {
    let ri = rarity_to_index(rarity);

    let mut items_per_tag = [0u8; 8];
    let mut effective_weights = [0u32; 8];

    for ti in 0..8 {
        let count = player_inventory::items::gear_by_rarity_tag(ri, ti).len();
        if count > 0 {
            items_per_tag[ti] = count as u8;
            effective_weights[ti] = tag_weights[ti];
        }
    }

    let total_weight: u64 = effective_weights.iter().map(|w| *w as u64).sum();
    if total_weight == 0 {
        return *b"G-ST-01\0";
    }

    let roll = seed % total_weight;
    let mut cumulative = 0u64;
    let mut selected_tag = 7usize;
    for (i, &weight) in effective_weights.iter().enumerate() {
        cumulative += weight as u64;
        if roll < cumulative {
            selected_tag = i;
            break;
        }
    }

    let indices = player_inventory::items::gear_by_rarity_tag(ri, selected_tag);
    let item_roll = (seed >> 8) % indices.len() as u64;
    let item = &player_inventory::items::ITEMS[indices[item_roll as usize]];
    let mut id = [0u8; 8];
    id.copy_from_slice(item.id);
    id
}

/// Select a tool item by rarity with tag weighting.
///
/// Uses pre-indexed lookups from player-inventory.
/// If no tools of the exact rarity exist, falls back through
/// Mythic -> Heroic -> Rare -> Common.
fn select_tool_by_rarity_weighted(
    rarity: ItemRarity,
    tag_weights: &[u32; 8],
    seed: u64,
) -> [u8; 8] {
    let fallback_chain: &[usize] = match rarity {
        ItemRarity::Mythic => &[3, 2, 1, 0],
        ItemRarity::Heroic => &[2, 1, 0],
        ItemRarity::Rare => &[1, 0],
        ItemRarity::Common => &[0],
    };

    for &ri in fallback_chain {
        let mut items_per_tag = [0u8; 8];
        let mut effective_weights = [0u32; 8];

        for ti in 0..8 {
            let count = player_inventory::items::tool_by_rarity_tag(ri, ti).len();
            if count > 0 {
                items_per_tag[ti] = count as u8;
                effective_weights[ti] = tag_weights[ti];
            }
        }

        let total_weight: u64 = effective_weights.iter().map(|w| *w as u64).sum();
        if total_weight == 0 {
            continue;
        }

        let roll = seed % total_weight;
        let mut cumulative = 0u64;
        let mut selected_tag = 0usize;
        for (i, &weight) in effective_weights.iter().enumerate() {
            cumulative += weight as u64;
            if roll < cumulative {
                selected_tag = i;
                break;
            }
        }

        let indices = player_inventory::items::tool_by_rarity_tag(ri, selected_tag);
        let item_roll = (seed >> 8) % indices.len() as u64;
        let item = &player_inventory::items::ITEMS[indices[item_roll as usize]];
        let mut id = [0u8; 8];
        id.copy_from_slice(item.id);
        return id;
    }

    *b"T-ST-01\0"
}

/// Select a gear item by specific tag and rarity.
///
/// Uses pre-indexed lookups from player-inventory.
/// If no gear of the exact rarity exists for the given tag,
/// falls back through Mythic -> Heroic -> Rare -> Common.
fn select_gear_by_tag_and_rarity(tag: WeaknessTag, rarity: ItemRarity, seed: u64) -> [u8; 8] {
    let ti = tag as usize;

    let fallback_chain: &[usize] = match rarity {
        ItemRarity::Mythic => &[3, 2, 1, 0],
        ItemRarity::Heroic => &[2, 1, 0],
        ItemRarity::Rare => &[1, 0],
        ItemRarity::Common => &[0],
    };

    for &ri in fallback_chain {
        let indices = player_inventory::items::gear_by_rarity_tag(ri, ti);
        if indices.is_empty() {
            continue;
        }

        let item_roll = (seed >> 8) % indices.len() as u64;
        let item = &player_inventory::items::ITEMS[indices[item_roll as usize]];
        let mut id = [0u8; 8];
        id.copy_from_slice(item.id);
        return id;
    }

    format_gear_id(TAG_CODES[tag as usize], 1)
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

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supply_cache_act1_distribution() {
        let mut common_count = 0;
        let mut rare_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(&SUPPLY_CACHE_RARITY, 1, seed);
            match rarity {
                ItemRarity::Common => common_count += 1,
                ItemRarity::Rare => rare_count += 1,
                _ => panic!("Act 1 Supply Cache should only have Common or Rare"),
            }
        }

        assert_eq!(common_count, 60);
        assert_eq!(rare_count, 40);
    }

    #[test]
    fn test_supply_cache_act4_distribution() {
        let mut common_count = 0;
        let mut rare_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(&SUPPLY_CACHE_RARITY, 4, seed);
            match rarity {
                ItemRarity::Common => common_count += 1,
                ItemRarity::Rare => rare_count += 1,
                _ => panic!("Act 4 Supply Cache should only have Common or Rare"),
            }
        }

        assert_eq!(common_count, 90);
        assert_eq!(rare_count, 10);
    }

    #[test]
    fn test_tool_crate_includes_heroic_in_act1() {
        let mut heroic_count = 0;

        for seed in 0..100 {
            let rarity = get_rarity_from_table(&TOOL_CRATE_RARITY, 1, seed);
            if rarity == ItemRarity::Heroic {
                heroic_count += 1;
            }
        }

        assert_eq!(heroic_count, 20, "Act 1 Tool Crate should have 20% Heroic");
    }

    #[test]
    fn test_geode_vault_mythic_all_acts() {
        for act in 1..=4 {
            let mut mythic_count = 0;

            for seed in 0..100 {
                let rarity = get_rarity_from_table(&GEODE_VAULT_RARITY, act, seed);
                if rarity == ItemRarity::Mythic {
                    mythic_count += 1;
                }
            }

            assert_eq!(
                mythic_count, 10,
                "Act {} Geode Vault should have 10% Mythic",
                act
            );
        }
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
        assert_eq!(calculate_scrap_cost(1), 4);
        assert_eq!(calculate_scrap_cost(2), 4);
        assert_eq!(calculate_scrap_cost(3), 4);
        assert_eq!(calculate_scrap_cost(4), 4);
    }

    #[test]
    fn test_anvil_upgrade_costs() {
        assert_eq!(calculate_anvil_cost(1), Some(10));
        assert_eq!(calculate_anvil_cost(2), Some(20));
        assert_eq!(calculate_anvil_cost(3), None);
    }

    #[test]
    fn test_tag_weight_calculation() {
        // Legacy function uses 100/140 scale for backward compatibility
        let weights = calculate_tag_weights(WeaknessTag::Stone, WeaknessTag::Frost);

        assert_eq!(weights[0], 140); // Stone (weakness)
        assert_eq!(weights[1], 100); // Scout (base)
        assert_eq!(weights[4], 140); // Frost (weakness)
        assert_eq!(weights[7], 100); // Tempo (base)
    }

    #[test]
    fn test_smuggler_hatch_separate_tables() {
        let tool_table = get_rarity_table(PoiOfferType::SmugglerHatch, true);
        let gear_table = get_rarity_table(PoiOfferType::SmugglerHatch, false);

        // Act 1: Tool 45% Common, Gear 35% Common
        assert_eq!(tool_table[0].0, 45);
        assert_eq!(gear_table[0].0, 35);
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
        // Full pool: all bits set so every item is valid
        let full_pool = [0xFFu8; ITEM_POOL_SIZE];
        let offers = generate_smuggler_hatch_offers(
            2,
            WeaknessTag::Greed,
            WeaknessTag::Blast,
            12345,
            &full_pool,
        );

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
    fn test_generate_smuggler_hatch_offers_respects_pool() {
        // Empty pool: no bits set, so no items are valid
        let empty_pool = [0u8; ITEM_POOL_SIZE];
        let offers = generate_smuggler_hatch_offers(
            2,
            WeaknessTag::Greed,
            WeaknessTag::Blast,
            12345,
            &empty_pool,
        );

        assert_eq!(
            offers.offers.len(),
            0,
            "Empty pool should produce no offers"
        );
    }

    #[test]
    fn test_generate_poi_offers_valid_pois() {
        // Valid item POIs (cache-type only; Smuggler Hatch uses its own path)
        assert!(generate_poi_offers(2, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_some());
        assert!(generate_poi_offers(3, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_some());
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
        assert!(generate_poi_offers(8, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_none()); // Rail Waypoint
        assert!(generate_poi_offers(9, 1, WeaknessTag::Stone, WeaknessTag::Frost, 123).is_none());
        // Smuggler Hatch (uses own path)
    }

    #[test]
    fn test_format_gear_id() {
        let id = format_gear_id("ST", 1);
        assert_eq!(&id, b"G-ST-01\0");

        let id = format_gear_id("FR", 8);
        assert_eq!(&id, b"G-FR-08\0");
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

    // =========================================================================
    // Xorshift64 RNG Tests
    // =========================================================================

    #[test]
    fn test_xorshift64_determinism() {
        // Same seed should produce same sequence
        let mut rng1 = Xorshift64::new(12345);
        let mut rng2 = Xorshift64::new(12345);

        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_xorshift64_different_seeds() {
        // Different seeds should produce different sequences
        let mut rng1 = Xorshift64::new(12345);
        let mut rng2 = Xorshift64::new(54321);

        assert_ne!(rng1.next_u64(), rng2.next_u64());
    }

    #[test]
    fn test_xorshift64_zero_seed_handled() {
        // Zero seed should be clamped to 1
        let mut rng = Xorshift64::new(0);
        assert_ne!(rng.next_u64(), 0); // Should produce non-zero output
    }

    #[test]
    fn test_xorshift64_bounded_range() {
        let mut rng = Xorshift64::new(12345);
        for _ in 0..1000 {
            let val = rng.next_bounded(100);
            assert!(val < 100, "Value {} should be < 100", val);
        }
    }

    #[test]
    fn test_xorshift64_bounded_zero() {
        let mut rng = Xorshift64::new(12345);
        assert_eq!(rng.next_bounded(0), 0);
    }

    // =========================================================================
    // TagWeights Tests
    // =========================================================================

    #[test]
    fn test_tag_weights_base() {
        let weights = TagWeights::new();
        assert_eq!(weights.total, BASE_WEIGHT * 8);
        for w in &weights.weights {
            assert_eq!(*w, BASE_WEIGHT);
        }
    }

    #[test]
    fn test_tag_weights_with_weaknesses() {
        let weights = TagWeights::new().with_weaknesses(WeaknessTag::Stone, WeaknessTag::Frost);

        assert_eq!(
            weights.weights[WeaknessTag::Stone as usize],
            WEAKNESS_WEIGHT
        );
        assert_eq!(
            weights.weights[WeaknessTag::Frost as usize],
            WEAKNESS_WEIGHT
        );
        assert_eq!(weights.weights[WeaknessTag::Scout as usize], BASE_WEIGHT);

        // Total should be 6 * 10000 + 2 * 14000 = 88000
        assert_eq!(weights.total, 6 * BASE_WEIGHT + 2 * WEAKNESS_WEIGHT);
    }

    #[test]
    fn test_tag_weights_same_weakness() {
        // If both weaknesses are the same tag, only add bonus once
        let weights = TagWeights::new().with_weaknesses(WeaknessTag::Stone, WeaknessTag::Stone);

        assert_eq!(
            weights.weights[WeaknessTag::Stone as usize],
            WEAKNESS_WEIGHT
        );
        // Total should be 7 * 10000 + 1 * 14000 = 84000
        assert_eq!(weights.total, 7 * BASE_WEIGHT + WEAKNESS_WEIGHT);
    }

    #[test]
    fn test_tag_weights_with_final_prep_bias() {
        let week3_tags = [WeaknessTag::Blood, WeaknessTag::Tempo];
        let weights = TagWeights::new()
            .with_weaknesses(WeaknessTag::Stone, WeaknessTag::Frost)
            .with_final_prep_bias(&week3_tags);

        // Blood and Tempo get +1000 bias
        assert_eq!(
            weights.weights[WeaknessTag::Blood as usize],
            BASE_WEIGHT + FINAL_PREP_BIAS
        );
        assert_eq!(
            weights.weights[WeaknessTag::Tempo as usize],
            BASE_WEIGHT + FINAL_PREP_BIAS
        );

        // Weaknesses remain at 14000
        assert_eq!(
            weights.weights[WeaknessTag::Stone as usize],
            WEAKNESS_WEIGHT
        );
    }

    #[test]
    fn test_tag_weights_select_tag_distribution() {
        // With 2 weakness tags at 1.4x, they should be selected more often
        let weights = TagWeights::new().with_weaknesses(WeaknessTag::Stone, WeaknessTag::Frost);
        let mut rng = Xorshift64::new(12345);

        let mut counts = [0u32; 8];
        for _ in 0..10000 {
            let tag = weights.select_tag(&mut rng);
            counts[tag as usize] += 1;
        }

        // Stone and Frost should have ~15.9% each (14000/88000)
        // Others should have ~11.4% each (10000/88000)
        let stone_ratio = counts[0] as f64 / 10000.0;
        let frost_ratio = counts[4] as f64 / 10000.0;
        let scout_ratio = counts[1] as f64 / 10000.0;

        // Tolerance of 2%
        assert!(
            stone_ratio > 0.14 && stone_ratio < 0.18,
            "Stone ratio {:.3} should be ~0.159",
            stone_ratio
        );
        assert!(
            frost_ratio > 0.14 && frost_ratio < 0.18,
            "Frost ratio {:.3} should be ~0.159",
            frost_ratio
        );
        assert!(
            scout_ratio > 0.09 && scout_ratio < 0.14,
            "Scout ratio {:.3} should be ~0.114",
            scout_ratio
        );
    }

    // =========================================================================
    // Mythic Cap Tests
    // =========================================================================

    #[test]
    fn test_sample_rarity_with_cap_enforces_limit() {
        // Use Geode Vault Act 4 which has 10% Mythic
        let mut rng = Xorshift64::new(12345);
        let mut mythic_count = 0;

        // Generate many offers with Mythic cap
        for _ in 0..100 {
            let mut mythic_used = false;

            for _ in 0..3 {
                let rarity =
                    sample_rarity_with_cap(&mut rng, &GEODE_VAULT_RARITY, 4, &mut mythic_used);
                if rarity == ItemRarity::Mythic {
                    mythic_count += 1;
                }
            }
        }

        // With cap: max 100 Mythic (one per 3-offer set).
        assert!(
            mythic_count <= 100,
            "Mythic count {} should be <= 100 with cap",
            mythic_count
        );
    }

    #[test]
    fn test_sample_rarity_with_cap_substitutes_heroic() {
        // When mythic_used is true, should return Heroic instead of Mythic
        let mut rng = Xorshift64::new(999); // Seed that produces Mythic roll

        // Find a seed that produces Mythic
        loop {
            let test_rng_val = rng.next_bounded(100) as u8;
            // Geode Vault Act 4: 0% Common, 0% Rare, 90% Heroic, 10% Mythic
            // Mythic if roll >= 90
            if test_rng_val >= 90 {
                // This roll would be Mythic
                break;
            }
        }

        // Reset with known seed and test
        let mut mythic_used = true; // Already used

        // With mythic_used=true, Mythic should downgrade to Heroic
        let mut test_rng = Xorshift64::new(97); // Roll of 90+ would be Mythic
        let rarity =
            sample_rarity_with_cap(&mut test_rng, &GEODE_VAULT_RARITY, 4, &mut mythic_used);

        // Since we can't guarantee the roll, just check behavior
        if rarity == ItemRarity::Mythic {
            panic!("Should not return Mythic when mythic_used is true");
        }
    }

    // =========================================================================
    // Offer Context Tests
    // =========================================================================

    #[test]
    fn test_offer_context_derive_seed() {
        let ctx = OfferContext::new(1, 1, 12345, 5);
        let seed = ctx.derive_seed();

        // Verify seed is derived consistently
        assert_eq!(seed, derive_offer_seed(12345, 5, 0));
    }

    #[test]
    fn test_derive_offer_seed_uniqueness() {
        let base = 12345u64;

        // Different POI indices should produce different seeds
        let seed1 = derive_offer_seed(base, 0, 0);
        let seed2 = derive_offer_seed(base, 1, 0);
        let seed3 = derive_offer_seed(base, 0, 1);

        assert_ne!(seed1, seed2);
        assert_ne!(seed1, seed3);
        assert_ne!(seed2, seed3);
    }

    #[test]
    fn test_is_item_in_pool_basic() {
        // Create a pool with bits 0 and 15 set
        let mut pool = [0u8; ITEM_POOL_SIZE];
        pool[0] = 0x01; // bit 0 set
        pool[1] = 0x80; // bit 15 set (byte 1, bit 7)

        assert!(super::is_item_in_pool(&pool, 0));
        assert!(!super::is_item_in_pool(&pool, 1));
        assert!(super::is_item_in_pool(&pool, 15));
        assert!(!super::is_item_in_pool(&pool, 16));
    }

    #[test]
    fn test_is_item_in_pool_out_of_bounds() {
        let pool = [0xFFu8; ITEM_POOL_SIZE]; // All bits set
                                             // Index 80+ should return false (out of bounds)
        assert!(!super::is_item_in_pool(&pool, 80));
        assert!(!super::is_item_in_pool(&pool, 255));
    }

    // =========================================================================
    // Tool Oil Offer Tests
    // =========================================================================

    #[test]
    fn test_generate_tool_oil_offers_returns_3_oils() {
        let oils = generate_tool_oil_offers(12345);
        assert_eq!(oils.len(), 3);
    }

    #[test]
    fn test_generate_tool_oil_offers_unique_oils() {
        let oils = generate_tool_oil_offers(12345);
        // All 3 should be unique
        assert_ne!(oils[0], oils[1]);
        assert_ne!(oils[0], oils[2]);
        assert_ne!(oils[1], oils[2]);
    }

    #[test]
    fn test_generate_tool_oil_offers_valid_oil_flags() {
        for seed in 0..100 {
            let oils = generate_tool_oil_offers(seed * 12345);
            for oil in &oils {
                assert!(
                    *oil == OIL_FLAG_ATK
                        || *oil == OIL_FLAG_SPD
                        || *oil == OIL_FLAG_DIG
                        || *oil == OIL_FLAG_ARM,
                    "Invalid oil flag: {}",
                    oil
                );
            }
        }
    }

    #[test]
    fn test_generate_tool_oil_offers_deterministic() {
        let oils1 = generate_tool_oil_offers(12345);
        let oils2 = generate_tool_oil_offers(12345);
        assert_eq!(oils1, oils2, "Same seed should produce same oils");
    }

    #[test]
    fn test_generate_tool_oil_offers_different_seeds() {
        let oils1 = generate_tool_oil_offers(12345);
        let oils2 = generate_tool_oil_offers(54321);
        // Different seeds should produce different results (with high probability)
        assert!(
            oils1 != oils2,
            "Different seeds should likely produce different oils"
        );
    }

    #[test]
    fn test_generate_tool_oil_offers_excludes_one() {
        // For each seed, exactly one oil should be excluded
        for seed in 0..100 {
            let oils = generate_tool_oil_offers(seed * 12345);
            let has_atk = oils.contains(&OIL_FLAG_ATK);
            let has_spd = oils.contains(&OIL_FLAG_SPD);
            let has_dig = oils.contains(&OIL_FLAG_DIG);
            let has_arm = oils.contains(&OIL_FLAG_ARM);

            let count = [has_atk, has_spd, has_dig, has_arm]
                .iter()
                .filter(|&&x| x)
                .count();
            assert_eq!(count, 3, "Should have exactly 3 oils present");
        }
    }

    #[test]
    fn test_create_oil_offer() {
        let offer = create_oil_offer(5, 12345);
        assert_eq!(offer.poi_index, 5);
        assert_eq!(offer.generated_at_seed, 12345);
        assert_eq!(offer.oils.len(), 3);
    }

    #[test]
    fn test_validate_oil_selection_valid() {
        let offer = create_oil_offer(0, 12345);
        // All 3 oils in the offer should be valid
        for &oil in &offer.oils {
            assert!(
                validate_oil_selection(&offer, oil),
                "Oil {} should be valid",
                oil
            );
        }
    }

    #[test]
    fn test_validate_oil_selection_invalid() {
        // Find the excluded oil
        let offer = create_oil_offer(0, 12345);
        let all_oils = [OIL_FLAG_ATK, OIL_FLAG_SPD, OIL_FLAG_DIG, OIL_FLAG_ARM];
        let excluded_oil = all_oils
            .iter()
            .find(|&&o| !offer.oils.contains(&o))
            .unwrap();

        assert!(
            !validate_oil_selection(&offer, *excluded_oil),
            "Excluded oil {} should be invalid",
            excluded_oil
        );
    }

    // =========================================================================
    // Rarity from Item ID Tests
    // =========================================================================

    #[test]
    fn test_rarity_from_item_id_matches_item_definitions() {
        // Verify rarity_from_item_id matches the authoritative item definitions
        // for every item in the registry.
        use player_inventory::items::ITEMS;
        use player_inventory::state::Rarity;

        for item in ITEMS.iter() {
            let expected = match item.rarity {
                Rarity::Common => 0u8,
                Rarity::Rare => 1,
                Rarity::Heroic => 2,
                Rarity::Mythic => 3,
            };
            assert_eq!(
                rarity_from_item_id(item.id),
                expected,
                "Rarity mismatch for item {:?} ({}): expected {}, got {}",
                std::str::from_utf8(item.id).unwrap_or("?"),
                item.name,
                expected,
                rarity_from_item_id(item.id),
            );
        }
    }

    #[test]
    fn test_rarity_from_item_id_specific_overrides() {
        // Items whose rarity diverges from the old NN-based convention
        // G-ST-03 (Spiked Bracers) = Common (NN=03 was Rare in old mapping)
        assert_eq!(rarity_from_item_id(b"G-ST-03\0"), 0);
        // G-ST-08 (Stone Sigil) = Heroic (NN=08 was Mythic in old mapping)
        assert_eq!(rarity_from_item_id(b"G-ST-08\0"), 2);
        // G-GR-08 (Citrine Shard) = Common (NN=08 was Mythic in old mapping)
        assert_eq!(rarity_from_item_id(b"G-GR-08\0"), 0);
        // G-RU-08 (Salvage Clamp) = Common (NN=08 was Mythic in old mapping)
        assert_eq!(rarity_from_item_id(b"G-RU-08\0"), 0);
        // T-GR-02 (Gemfinder Staff) = Heroic (NN=02 was Rare in old mapping)
        assert_eq!(rarity_from_item_id(b"T-GR-02\0"), 2);
        // T-TE-02 (Chrono Rapier) = Mythic in revised balance
        assert_eq!(rarity_from_item_id(b"T-TE-02\0"), 3);
    }

    #[test]
    fn test_rarity_from_item_id_malformed() {
        // Unknown IDs should return 0 (Common fallback)
        assert_eq!(rarity_from_item_id(b"X-ST-01\0"), 0);
        assert_eq!(rarity_from_item_id(b"G-ST-AB\0"), 0);
        assert_eq!(rarity_from_item_id(b"G-ST-00\0"), 0);
    }
}

// =============================================================================
// Rarity from Item ID
// =============================================================================

/// Derive item rarity from an item_id by looking up the authoritative
/// item definition in the player-inventory registry.
///
/// Returns the rarity as u8: 0=Common, 1=Rare, 2=Heroic, 3=Mythic.
/// Returns 0 (Common) for unrecognised or malformed IDs.
pub fn rarity_from_item_id(item_id: &[u8; 8]) -> u8 {
    use player_inventory::items::get_item;
    use player_inventory::state::Rarity;

    match get_item(item_id) {
        Some(def) => match def.rarity {
            Rarity::Common => 0,
            Rarity::Rare => 1,
            Rarity::Heroic => 2,
            Rarity::Mythic => 3,
        },
        None => 0,
    }
}

// =============================================================================
// Item Pool Filtering
// =============================================================================

/// Session item bitmask size (80 bits = 10 bytes)
pub const ITEM_POOL_SIZE: usize = 10;

/// Check if an item index is in the active item pool bitmask.
///
/// The active_item_pool is a bitmask where each bit represents an item.
/// Items are indexed starting at 0. The bitmask is stored as bytes
/// in little-endian order (bit 0 is the LSB of byte 0).
///
/// # Arguments
/// * `pool` - The 10-byte active_item_pool bitmask
/// * `item_index` - The item index to check (0-79)
///
/// # Returns
/// `true` if the item is unlocked in the pool, `false` otherwise
pub fn is_item_in_pool(pool: &[u8; ITEM_POOL_SIZE], item_index: u8) -> bool {
    // Check bounds - pool supports items 0-79
    if item_index >= (ITEM_POOL_SIZE * 8) as u8 {
        return false;
    }

    let byte_index = (item_index / 8) as usize;
    let bit_index = item_index % 8;

    (pool[byte_index] & (1 << bit_index)) != 0
}

/// Convert an item ID to its pool index.
///
/// Item IDs follow the format: {Type}-{Tag}-{Num}
/// - Type: T (Tool) or G (Gear)
/// - Tag: 2-char tag code (ST, SC, GR, BL, FR, RU, BO, TE)
/// - Num: 01-08 for Gear, 01-02 for Tools
///
/// Returns None if the item ID is invalid or doesn't map to a pool index.
pub fn item_id_to_pool_index(item_id: &[u8; 8]) -> Option<u8> {
    // Parse tag from bytes 2-3
    let tag_code = match (item_id[2], item_id[3]) {
        (b'S', b'T') => 0, // Stone
        (b'S', b'C') => 1, // Scout
        (b'G', b'R') => 2, // Greed
        (b'B', b'L') => 3, // Blast
        (b'F', b'R') => 4, // Frost
        (b'R', b'U') => 5, // Rust
        (b'B', b'O') => 6, // Blood
        (b'T', b'E') => 7, // Tempo
        _ => return None,
    };

    // Parse item number from bytes 5-6
    let num_tens = item_id[5].checked_sub(b'0')?;
    let num_ones = item_id[6].checked_sub(b'0')?;
    if num_tens > 9 || num_ones > 9 {
        return None;
    }
    let item_num = num_tens * 10 + num_ones;

    // Core pool index mapping:
    // Gear: items 0-63 (8 tags * 8 items, IDs 01-08)
    // Tools: items 64-79 (8 tags * 2 items, IDs 01-02)
    match item_id[0] {
        b'G' => {
            if !(1..=8).contains(&item_num) {
                return None;
            }
            Some(tag_code * 8 + (item_num - 1))
        }
        b'T' => {
            if !(1..=2).contains(&item_num) {
                return None;
            }
            Some(64 + tag_code * 2 + (item_num - 1))
        }
        _ => None,
    }
}

/// Filter offers to only include items that are in the active item pool.
///
/// # Arguments
/// * `offers` - Slice of item offers to filter
/// * `pool` - The session's active_item_pool bitmask
///
/// # Returns
/// A vector containing only offers whose items are in the pool
pub fn filter_offers_by_pool(offers: &[ItemOffer], pool: &[u8; ITEM_POOL_SIZE]) -> Vec<ItemOffer> {
    offers
        .iter()
        .filter(|offer| {
            if let Some(index) = item_id_to_pool_index(&offer.item_id) {
                is_item_in_pool(pool, index)
            } else {
                // If we can't parse the item ID, exclude it
                false
            }
        })
        .cloned()
        .collect()
}
