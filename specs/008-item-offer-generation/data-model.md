# Data Model: Item Offer Generation

**Feature**: 008-item-offer-generation
**Date**: 2026-01-20

## Entity Overview

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  OfferContext   │────▶│  RarityTable     │     │   ItemPool      │
│                 │     │                  │     │                 │
│ act, week,      │     │ common, rare,    │     │ indices[]       │
│ biome, poi_type │     │ heroic, mythic   │     │ (by tag/rarity) │
│ seed            │     │ (basis points)   │     │                 │
└────────┬────────┘     └──────────────────┘     └────────┬────────┘
         │                                                 │
         ▼                                                 │
┌─────────────────┐     ┌──────────────────┐              │
│   TagWeights    │────▶│  Xorshift64      │◀─────────────┘
│                 │     │                  │
│ weights[8]      │     │ state (u64)      │
│ (basis points)  │     │ next(), bounded  │
└─────────────────┘     └────────┬─────────┘
                                 │
                                 ▼
┌─────────────────┐     ┌──────────────────┐
│   ItemOffer     │◀────│  ShopInventory   │
│                 │     │                  │
│ items[3]        │     │ items[6]         │
│ (item_id,       │     │ (1 Tool +        │
│  rarity)        │     │  5 Gear + price) │
└─────────────────┘     └──────────────────┘
```

## Core Entities

### 1. OfferContext

**Purpose**: Input parameters for offer generation, derived from game state.

| Field | Type | Description |
|-------|------|-------------|
| `act` | `u8` | Current act (1-4) |
| `week` | `u8` | Current week (1-3) |
| `biome` | `Biome` | Current biome (A or B) |
| `poi_type` | `PoiType` | POI triggering the offer |
| `seed` | `u64` | Session seed for deterministic RNG |
| `poi_index` | `u8` | Index of POI for seed derivation |
| `offer_call_count` | `u8` | Counter for unique sub-seeds |
| `enable_final_prep_bias` | `bool` | Whether to apply Week 3 tag bonus |

**Derived From**: `MapPois` account + `GameState` account

---

### 2. RarityTable

**Purpose**: Probability distribution for item rarities, indexed by POI type and act.

| Field | Type | Description |
|-------|------|-------------|
| `common` | `u16` | Common probability (0-10000 basis points) |
| `rare` | `u16` | Rare probability (0-10000 basis points) |
| `heroic` | `u16` | Heroic probability (0-10000 basis points) |
| `mythic` | `u16` | Mythic probability (0-10000 basis points) |

**Validation**: `common + rare + heroic + mythic == 10000`

**Compile-Time Constants** (20 tables = 5 POI types × 4 acts):

#### Supply Cache (L2) - Gear Only
| Act | Common | Rare | Heroic | Mythic |
|-----|--------|------|--------|--------|
| 1 | 10000 | 0 | 0 | 0 |
| 2 | 8500 | 1500 | 0 | 0 |
| 3 | 7500 | 2500 | 0 | 0 |
| 4 | 6500 | 3500 | 0 | 0 |

#### Tool Crate (L3) - Tools Only
| Act | Common | Rare | Heroic | Mythic |
|-----|--------|------|--------|--------|
| 1 | 8500 | 1500 | 0 | 0 |
| 2 | 7000 | 2500 | 500 | 0 |
| 3 | 6000 | 3000 | 1000 | 0 |
| 4 | 5000 | 3500 | 1500 | 0 |

#### Geode Vault (L12) - Heroic+ Only
| Act | Common | Rare | Heroic | Mythic |
|-----|--------|------|--------|--------|
| 1-3 | 0 | 0 | 10000 | 0 |
| 4 | 0 | 0 | 9000 | 1000 |

#### Smuggler Hatch (L9) - Gear Rarity
| Act | Common | Rare | Heroic | Mythic |
|-----|--------|------|--------|--------|
| 1 | 7000 | 2700 | 300 | 0 |
| 2 | 5500 | 3800 | 700 | 0 |
| 3 | 4500 | 4200 | 1300 | 0 |
| 4 | 3500 | 4500 | 1800 | 200 |

#### Smuggler Hatch (L9) - Tool Rarity
| Act | Common | Rare | Heroic | Mythic |
|-----|--------|------|--------|--------|
| 1 | 8000 | 2000 | 0 | 0 |
| 2 | 6500 | 3000 | 500 | 0 |
| 3 | 5500 | 3500 | 1000 | 0 |
| 4 | 4500 | 4000 | 1500 | 0 |

#### Counter Cache (L13) - Same as Supply Cache
Uses Supply Cache rarity tables.

---

### 3. TagWeights

**Purpose**: Weighted probabilities for tag selection, computed dynamically per offer.

| Field | Type | Description |
|-------|------|-------------|
| `weights` | `[u16; 8]` | Weight per tag in basis points |
| `total` | `u32` | Sum of all weights for normalization |

**Tag Indices**:
```
0: Stone, 1: Scout, 2: Greed, 3: Blast,
4: Frost, 5: Rust, 6: Blood, 7: Tempo
```

**Weight Computation**:
- Base weight: 10000 (1.0)
- Boss weakness bonus: +4000 (1.4x total)
- Final prep bias: +1000 (for Week 3 tags during Week 1-2)

**Example** (Week 1, Biome A, Broodmother boss with Stone+Frost weakness):
```
weights = [14000, 10000, 10000, 10000, 14000, 10000, 10000, 10000]
total = 88000
```

---

### 4. ItemPool

**Purpose**: Pre-indexed item IDs for efficient lookup by (tag, rarity, type).

| Field | Type | Description |
|-------|------|-------------|
| `indices` | `&'static [u8]` | Indices into `ALL_ITEMS` array |

**Compile-Time Index Structure**:
```
ITEM_POOLS[tag: 0-7][rarity: 0-3][type: 0-1] -> ItemPool
```

**Example Pools**:
- `ITEM_POOLS[Stone][Common][Gear]` → `[G-ST-01, G-ST-02]`
- `ITEM_POOLS[Scout][Rare][Tool]` → `[T-SC-02]`
- `ITEM_POOLS[Greed][Mythic][Gear]` → `[]` (no Mythic Greed items)

---

### 5. ItemOffer

**Purpose**: Generated offer presented to player at a POI.

| Field | Type | Description |
|-------|------|-------------|
| `item_id` | `[u8; 8]` | Item identifier (e.g., "G-ST-01\0") |
| `rarity` | `Rarity` | Item rarity tier |

**Container Variants**:
- Standard offer: `[ItemOffer; 3]` (Supply Cache, Tool Crate, Geode Vault, Counter Cache)
- Shop offer: `[ShopItem; 6]` (Smuggler Hatch)

---

### 6. ShopItem

**Purpose**: Extended offer item with pricing for shop POIs.

| Field | Type | Description |
|-------|------|-------------|
| `item_id` | `[u8; 8]` | Item identifier |
| `rarity` | `Rarity` | Item rarity tier |
| `item_type` | `ItemType` | Tool or Gear |
| `price` | `u16` | Gold cost |

**Pricing Table**:
| Type | Common | Rare | Heroic | Mythic |
|------|--------|------|--------|--------|
| Gear | 8 | 14 | 22 | 34 |
| Tool | 10 | 16 | 24 | - |

---

### 7. ShopState (Extended)

**Purpose**: Persisted state for Smuggler Hatch shop within `MapPois` account.

| Field | Type | Description |
|-------|------|-------------|
| `active` | `bool` | Whether shop is currently active |
| `poi_index` | `u8` | Which POI this shop belongs to |
| `reroll_count` | `u8` | Number of rerolls this visit |
| `inventory` | `[ShopItem; 6]` | Current shop inventory |
| `rng_state` | `u64` | RNG state for next reroll |

**Reroll Cost**: `4 + 2 * reroll_count` Gold (0-indexed reroll_count)
- First visit (reroll_count=0): free initial generation
- First reroll (reroll_count=1): 4 Gold
- Second reroll (reroll_count=2): 6 Gold
- Third reroll (reroll_count=3): 8 Gold

---

### 8. Xorshift64

**Purpose**: Deterministic pseudo-random number generator.

| Field | Type | Description |
|-------|------|-------------|
| `state` | `u64` | Current RNG state (non-zero) |

**Methods**:
- `new(seed: u64)` → Initialize with seed (clamped to avoid zero)
- `next()` → Generate next u64
- `next_bounded(max: u64)` → Generate u64 in `[0, max)`

**Seed Derivation**:
```
offer_seed = session_seed ^ (poi_index << 16) ^ (offer_call_count << 8)
```

---

## State Transitions

### Offer Generation Flow

```
[POI Interaction Triggered]
         │
         ▼
┌─────────────────────┐
│ Build OfferContext  │
│ from MapPois +      │
│ GameState accounts  │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Get RarityTable     │
│ for (poi_type, act) │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐     ┌─────────────────┐
│ Compute TagWeights  │────▶│ Lookup boss     │
│ with boss weakness  │     │ weakness tags   │
└──────────┬──────────┘     └─────────────────┘
           │
           ▼
┌─────────────────────┐
│ For each item slot: │
│ 1. Sample rarity    │
│ 2. Select tag       │◀──┐
│ 3. Pick item from   │   │ (loop for item count)
│    (tag, rarity)    │───┘
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Return ItemOffer[]  │
│ or ShopInventory    │
└─────────────────────┘
```

### Shop Reroll Flow

```
[Reroll Requested]
         │
         ▼
┌─────────────────────┐
│ Validate Gold >=    │
│ reroll_cost         │
│ (4 + 2*count)       │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Deduct Gold from    │
│ player              │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Increment           │
│ reroll_count        │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Regenerate shop     │
│ inventory with      │
│ updated RNG state   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ Store new inventory │
│ in ShopState        │
└─────────────────────┘
```

---

## Validation Rules

### OfferContext
- `act` must be 1-4
- `week` must be 1-3
- `biome` must be valid enum variant
- `poi_type` must be item-granting POI (L2, L3, L9, L12, L13)
- `seed` must be non-zero

### RarityTable
- Sum of probabilities must equal 10000
- Individual probabilities must be 0-10000

### TagWeights
- All weights must be positive
- Total must be sum of individual weights

### ItemPool
- All indices must be valid into ALL_ITEMS array
- Pool for (tag, rarity, type) must contain only matching items

### ShopState
- `poi_index` must reference valid Smuggler Hatch POI
- `reroll_count` reasonable upper bound (e.g., < 20)
- `inventory[0]` must be Tool type
- `inventory[1-5]` must be Gear type

---

## Relationships to Existing Entities

### From player-inventory (read-only)
- `ItemDefinition` - Item metadata including tag and rarity
- `ALL_ITEMS` - Static array of all 80 items
- `ItemType` - Tool or Gear enum
- `Rarity` - Common, Rare, Heroic, Mythic enum

### From boss-system (read-only)
- `BossDefinition` - Boss metadata including weakness_tags
- `get_boss_for_stage()` - Lookup boss by stage/week/biome
- `ItemTag` - Re-exported from combat-system

### From poi-system (extended)
- `MapPois` - Extended with offer generation state
- `ShopState` - Existing struct to be extended
- `PoiType` - POI type enum for dispatch
