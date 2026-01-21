# Research: Item Offer Generation

**Feature**: 008-item-offer-generation
**Date**: 2026-01-20

## Research Topics

### 1. Deterministic Random Number Generation on Solana

**Decision**: Use session seed with XorShift64 algorithm

**Rationale**:
- Solana programs must be deterministic - same inputs produce same outputs
- XorShift64 is lightweight, fast, and sufficient for game randomness (not cryptographic)
- Session seed (from `MapPois.seed`) provides unique per-session randomness
- Derive sub-seeds for each offer generation call: `seed = base_seed ^ (poi_index << 8) ^ (offer_call_count)`

**Alternatives Considered**:
- **Slot hash**: Too unpredictable across transactions; not suitable for deterministic verification
- **Recent blockhash**: Changes every slot; breaks determinism for replay
- **VRF (Switchboard/Pyth)**: Overkill for PvE game; adds external dependency and latency

**Implementation Pattern**:
```rust
pub struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) } // Avoid zero state
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Returns value in [0, max) range
    pub fn next_bounded(&mut self, max: u64) -> u64 {
        self.next() % max
    }
}
```

---

### 2. Weighted Random Sampling Algorithm

**Decision**: Normalized weight sampling with pre-computed cumulative distribution

**Rationale**:
- 8 tags with dynamic weights (1.0 base, 1.4 for weakness tags)
- Small tag count makes linear scan acceptable (no need for alias method)
- Weights are normalized to sum to 1.0 (or fixed-point equivalent)
- Cumulative distribution function (CDF) enables single random draw

**Algorithm**:
1. Compute raw weights: `w[i] = 1.0` for normal tags, `1.4` for weakness tags, `+0.1` for final prep bias
2. Sum total weight: `total = sum(w[i])`
3. Generate random `r` in `[0, total)`
4. Linear scan to find first tag where `cumsum > r`

**Fixed-Point Implementation** (avoid floats on-chain):
- Use basis points (1.0 = 10000, 1.4 = 14000, 0.1 = 1000)
- Total for 8 tags with 2 weakness: `6*10000 + 2*14000 = 88000`
- With final prep bias: `6*10000 + 2*14000 + 2*1000 = 90000`

**Alternatives Considered**:
- **Alias method**: O(1) sampling but O(n) setup; overkill for 8 tags
- **Rejection sampling**: Simple but unbounded worst-case; not ideal for on-chain
- **Binary search on CDF**: O(log n) but complexity not warranted for n=8

---

### 3. Rarity Table Representation

**Decision**: Compile-time const arrays with basis-point probabilities

**Rationale**:
- Rarity tables are static per (POI type, act) combination
- 5 POI types × 4 acts = 20 table entries (manageable as consts)
- Basis points (0-10000) avoid floating point
- Separate tables for Tool vs Gear where they differ

**Data Structure**:
```rust
pub struct RarityTable {
    pub common: u16,   // Probability in basis points (0-10000)
    pub rare: u16,
    pub heroic: u16,
    pub mythic: u16,   // Only non-zero for specific POIs/acts
}

// Example: Supply Cache Act 1 = 100% Common
pub const SUPPLY_CACHE_ACT1: RarityTable = RarityTable {
    common: 10000, rare: 0, heroic: 0, mythic: 0
};

// Example: Supply Cache Act 4 = 65% Common, 35% Rare
pub const SUPPLY_CACHE_ACT4: RarityTable = RarityTable {
    common: 6500, rare: 3500, heroic: 0, mythic: 0
};
```

**Alternatives Considered**:
- **Runtime calculation**: Wastes compute units; tables are fixed
- **Floating point**: Not available on Solana BPF
- **Percentage (0-100)**: Less precision than basis points

---

### 4. Item Registry Indexing

**Decision**: Precompute item indices by (tag, rarity, type) at compile time

**Rationale**:
- 80 items must be filterable by tag (8), rarity (4), and type (2)
- Runtime filtering wastes compute units
- Compile-time index arrays enable O(1) lookup of valid item pools

**Data Structure**:
```rust
// For each (tag, rarity, type) combination, store item indices
pub struct ItemPool {
    pub indices: &'static [u8],  // Indices into ALL_ITEMS array
}

// Example: Stone tag, Common rarity, Gear type
pub const STONE_COMMON_GEAR: ItemPool = ItemPool {
    indices: &[2, 3], // G-ST-01, G-ST-02
};

// Lookup table: [tag][rarity][item_type] -> ItemPool
pub const ITEM_POOLS: [[[ItemPool; 2]; 4]; 8] = /* ... */;
```

**Alternatives Considered**:
- **Runtime filtering**: O(80) scan per item selection; too expensive
- **HashMap**: No std HashMap on Solana; would need custom implementation
- **Sorted arrays + binary search**: More complex than needed for 80 items

---

### 5. Counter Cache Special Handling

**Decision**: Dual-tag pool aggregation with deduplication

**Rationale**:
- Counter Cache draws from both boss weakness tags exclusively
- Some items might theoretically appear in both tags (currently none in GDD)
- Pool is union of both tag's items at requested rarity
- No tag weighting needed - uniform selection from combined pool

**Algorithm**:
1. Get current boss weakness tags `[tag1, tag2]`
2. Aggregate items: `pool = items_by_tag[tag1][rarity] ∪ items_by_tag[tag2][rarity]`
3. Random select from pool uniformly

---

### 6. Smuggler Hatch Shop State

**Decision**: Extend `ShopState` in `MapPois` account

**Rationale**:
- Shop state includes current inventory and reroll count
- Already partially defined in poi-system state
- Reroll regenerates all 6 items with new RNG state
- Reroll cost formula: `4 + 2 * (reroll_count - 1)` = `2 + 2 * reroll_count`

**State Extension**:
```rust
pub struct ShopState {
    pub poi_index: u8,           // Which POI this shop belongs to
    pub reroll_count: u8,        // Number of rerolls this visit
    pub inventory: [ShopItem; 6], // 1 Tool + 5 Gear
    pub last_seed: u64,          // Seed used for current inventory
}

pub struct ShopItem {
    pub item_id: [u8; 8],
    pub rarity: Rarity,
    pub price: u16,              // Gold cost
}
```

**Alternatives Considered**:
- **Separate ShopState account**: Extra PDA; unnecessary complexity
- **Regenerate on each view**: Breaks player experience; inventory should persist

---

### 7. Mythic Rarity Cap Enforcement

**Decision**: Track mythic count during generation, substitute Heroic when capped

**Rationale**:
- Max 1 Mythic per offer (FR-010)
- Only applies to Geode Vault (Act 4) and Smuggler Hatch (Act 4)
- If rarity roll yields Mythic but cap reached, downgrade to Heroic

**Algorithm**:
```rust
fn generate_offer_item(rng: &mut Xorshift64, table: &RarityTable, mythic_used: &mut bool) -> Rarity {
    let roll = rng.next_bounded(10000) as u16;
    let mut cumsum = 0;

    cumsum += table.common;
    if roll < cumsum { return Rarity::Common; }

    cumsum += table.rare;
    if roll < cumsum { return Rarity::Rare; }

    cumsum += table.heroic;
    if roll < cumsum { return Rarity::Heroic; }

    // Mythic
    if *mythic_used {
        return Rarity::Heroic; // Substitute
    }
    *mythic_used = true;
    Rarity::Mythic
}
```

---

### 8. Final Prep Bias Implementation

**Decision**: Configurable flag in generation context; +0.1 (1000 basis points) to Week 3 tags

**Rationale**:
- Optional feature per FR-007
- During Week 1-2, add +1000 to Week 3 final boss weakness tags
- Requires looking up Week 3 boss even when in earlier weeks

**Context Structure**:
```rust
pub struct OfferContext {
    pub act: u8,
    pub week: u8,
    pub biome: Biome,
    pub poi_type: PoiType,
    pub seed: u64,
    pub enable_final_prep_bias: bool,
}
```

---

## Integration Points

### Dependencies (Read-Only)

| Crate | Type | Used For |
|-------|------|----------|
| `combat-system` | ItemTag enum | Tag identification and matching |
| `player-inventory` | ItemDefinition, ALL_ITEMS | Item registry and filtering |
| `boss-system` | BossDefinition, get_boss_for_week | Boss weakness tag lookup |

### Exports (New)

| Module | Exports | Used By |
|--------|---------|---------|
| `offer_generation` | `generate_item_offer()`, `generate_shop_inventory()` | POI interaction handlers |
| `rarity_tables` | `get_rarity_table()` | Offer generation |
| `tag_weights` | `compute_tag_weights()`, `select_weighted_tag()` | Offer generation |

---

## Resolved Clarifications

All technical unknowns from spec have been resolved:

1. ✅ RNG approach: XorShift64 with session seed
2. ✅ Weight representation: Basis points (10000 = 1.0)
3. ✅ Item lookup: Compile-time indexed pools
4. ✅ Shop state: Extended MapPois.shop_state
5. ✅ Mythic cap: Track-and-substitute pattern
