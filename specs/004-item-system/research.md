# Research: Item System

**Feature**: 004-item-system
**Date**: 2026-01-19

## Research Topics

### 1. Item Definition Storage Architecture

**Question**: How to store 80 item definitions efficiently on Solana?

**Decision**: Compile-time constants with lookup functions

**Rationale**:
- Item definitions are static and never change during gameplay
- Storing in accounts would require 80 separate PDAs or a large account
- Compile-time constants are free to access (no account reads)
- Functions like `get_item(id)` return item data directly from program memory
- Solana programs can hold significant read-only data in the binary

**Implementation**:
```rust
// items.rs - compile-time constant
pub const ITEMS: &[ItemDefinition] = &[
    ItemDefinition {
        id: "T-ST-01",
        name: "Bulwark Shovel",
        item_type: ItemType::Tool,
        tag: ItemTag::Stone,
        rarity: Rarity::Common,
        effects: &[/* ... */],
    },
    // ... 79 more items
];

pub fn get_item(id: &str) -> Option<&ItemDefinition> {
    ITEMS.iter().find(|i| i.id == id)
}
```

**Alternatives Considered**:
- Account-based storage: Rejected due to unnecessary account reads and rent costs
- PDAs per item: Rejected due to excessive account overhead (80 PDAs)
- Off-chain indexing: Rejected as items must be verifiable on-chain

### 2. Player Inventory State Design

**Question**: How to structure player inventory for efficient access and updates?

**Decision**: Single PDA per player with fixed-size arrays

**Rationale**:
- Player has at most 9 items (1 Tool + 8 Gear)
- Fixed-size array avoids reallocation and is predictable
- Single account for all inventory operations
- PDA derived from player pubkey for deterministic addressing

**Data Structure**:
```rust
pub struct PlayerInventory {
    pub player: Pubkey,
    pub tool: Option<ItemInstance>,      // 0 or 1 tool
    pub gear: [Option<ItemInstance>; 8], // 0-8 gear items
    pub gear_slot_capacity: u8,          // 4, 6, or 8
    pub bump: u8,
}

pub struct ItemInstance {
    pub item_id: [u8; 8],  // Fixed-size ID (e.g., "T-ST-01\0")
    pub tier: Tier,        // I, II, or III
    pub tool_oil_applied: ToolOilFlags, // Bitflags for +ATK/+SPD/+DIG
}
```

**Alternatives Considered**:
- Separate accounts per item: Rejected due to transaction complexity
- Dynamic Vec: Rejected due to reallocation overhead and variable account size

### 3. Itemset Detection Strategy

**Question**: How to efficiently check if player has an active itemset?

**Decision**: Check all 12 sets on demand when generating combat effects

**Rationale**:
- Only 12 itemsets to check
- Player has max 9 items, so membership checks are O(12 × 4 × 9) = O(432) - trivial
- No need to track active sets in state - compute on demand
- Deterministic results

**Implementation**:
```rust
pub fn get_active_itemsets(inventory: &PlayerInventory) -> Vec<&'static ItemsetDefinition> {
    ITEMSETS.iter()
        .filter(|set| set.required_items.iter().all(|id| inventory.has_item(id)))
        .collect()
}
```

**Alternatives Considered**:
- Cache active sets in inventory state: Rejected as unnecessary complexity
- Event-driven updates: Rejected as adds complexity without performance benefit

### 4. Item Effect Format for Combat Integration

**Question**: How to convert items to ItemEffect format the combat system expects?

**Decision**: Build ItemEffect arrays from inventory at combat start

**Rationale**:
- Combat system already accepts `Vec<ItemEffect>` as input
- Items store effects in a compatible format
- Tier scaling applied during conversion
- Itemset bonuses added to the effect list

**Flow**:
```
Inventory → for each equipped item:
    → Look up ItemDefinition
    → For each effect in definition:
        → Scale value by tier
        → Create ItemEffect
    → Add to player_effects
→ For each active itemset:
    → Add itemset bonus effect
→ Return player_effects: Vec<ItemEffect>
```

### 5. Item Offer Generation with Weighted Tags

**Question**: How to implement tag-weighted item generation?

**Decision**: Weighted random selection using deterministic seed

**Rationale**:
- Need deterministic results for verification
- Tag weights: base 1.0, weakness tags 1.4
- Rarity table determines pool (Common/Rare/Heroic/Mythic)
- Seed from game state (session + action count) ensures reproducibility

**Algorithm**:
```rust
fn generate_item_offer(
    poi_type: PoiType,
    act: u8,
    week: u8,
    boss_weaknesses: (ItemTag, ItemTag),
    seed: u64,
) -> ItemDefinition {
    // 1. Get rarity from POI + Act table
    let rarity = get_rarity_from_table(poi_type, act, seed);

    // 2. Filter items by rarity (and type for Tool Crate)
    let candidates: Vec<_> = ITEMS.iter()
        .filter(|i| i.rarity == rarity)
        .collect();

    // 3. Calculate weights (1.0 base, 1.4 for weakness tags)
    let weights: Vec<f32> = candidates.iter()
        .map(|i| if i.tag == boss_weaknesses.0 || i.tag == boss_weaknesses.1 {
            1.4
        } else {
            1.0
        })
        .collect();

    // 4. Weighted selection using seed
    weighted_select(&candidates, &weights, seed)
}
```

### 6. Tier Upgrade (Fusion) Validation

**Question**: How to validate and execute item fusion?

**Decision**: Instruction that takes two item slot indices and validates match

**Rationale**:
- Player specifies which two items to fuse
- System validates: same item ID, same tier, not Tier III
- Result replaces first slot, second slot emptied
- Single atomic transaction

**Validation Rules**:
1. Both items must exist in inventory
2. Both items must have same `item_id`
3. Both items must have same `tier`
4. Neither can be Tier III
5. For gear, both must be in gear slots (can't fuse tool with gear)

### 7. Compute Budget Analysis

**Question**: Will item operations fit within Solana compute limits?

**Decision**: Yes, with efficient data structures

**Rationale**:
- Item lookup: O(80) linear scan or O(1) with hash lookup = ~500 CU
- Inventory read: single account deserialize = ~2000 CU
- Itemset detection: O(12 × 4) = ~500 CU
- Effect generation: O(9 × 3) = ~300 CU
- Offer generation: O(80) filter + weighted select = ~1000 CU
- Total per operation: ~5000 CU (well under 200k limit)

## Decisions Summary

| Topic | Decision | Impact |
|-------|----------|--------|
| Item Storage | Compile-time constants | Zero account reads, instant lookup |
| Inventory State | Single PDA with fixed arrays | Predictable size, atomic updates |
| Itemset Detection | Compute on demand | Simple, no state tracking |
| Combat Integration | Build effects at combat start | Clean interface with combat system |
| Offer Generation | Weighted selection with deterministic seed | Reproducible, bias toward weaknesses |
| Fusion | Index-based with full validation | Safe, atomic tier upgrades |
| Compute Budget | ~5000 CU per operation | Well within limits |
