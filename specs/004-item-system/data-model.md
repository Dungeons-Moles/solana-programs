# Data Model: Item System

**Feature**: 004-item-system
**Date**: 2026-01-19

## Entities

### 1. ItemDefinition (Compile-time Constant)

Static data for an item type. Stored as compile-time constants in the program binary.

| Field | Type | Description |
|-------|------|-------------|
| id | [u8; 8] | Unique ID (e.g., "T-ST-01") |
| name | &'static str | Display name (e.g., "Bulwark Shovel") |
| item_type | ItemType | Tool or Gear |
| tag | ItemTag | One of 8 tags |
| rarity | Rarity | Common, Rare, Heroic, or Mythic |
| effects | &'static [EffectDefinition] | Combat effects with tier-scaled values |

### 2. EffectDefinition (Compile-time Constant)

A single effect on an item, with values that scale by tier.

| Field | Type | Description |
|-------|------|-------------|
| trigger | TriggerType | When effect activates (BattleStart, OnHit, etc.) |
| effect_type | EffectType | What the effect does |
| once_per_turn | bool | Whether effect is limited to once per turn |
| values | [i16; 3] | Tier I, II, III values |

### 3. ItemsetDefinition (Compile-time Constant)

Static data for an itemset bonus.

| Field | Type | Description |
|-------|------|-------------|
| id | &'static str | Unique ID (e.g., "union-standard") |
| name | &'static str | Display name (e.g., "Union Standard") |
| emoji | &'static str | Visual indicator (e.g., "🧰") |
| required_items | &'static [[u8; 8]] | Array of 3-4 required item IDs |
| bonus_effect | EffectDefinition | The set bonus effect |

### 4. PlayerInventory (Account)

Player's equipped items. PDA derived from player pubkey.

**PDA Seeds**: `["inventory", player.key()]`

| Field | Type | Description |
|-------|------|-------------|
| player | Pubkey | Owner's wallet |
| tool | Option<ItemInstance> | Equipped tool (0 or 1) |
| gear | [Option<ItemInstance>; 8] | Equipped gear (0-8) |
| gear_slot_capacity | u8 | Current gear limit (4, 6, or 8) |
| bump | u8 | PDA bump seed |

**Size**: 8 (discriminator) + 32 (player) + 1 + 10 (tool option) + 8 × (1 + 10) (gear array) + 1 + 1 = ~143 bytes

### 5. ItemInstance (Struct)

A specific item owned by a player. Embedded in PlayerInventory.

| Field | Type | Description |
|-------|------|-------------|
| item_id | [u8; 8] | References ItemDefinition ID |
| tier | Tier | I, II, or III |
| tool_oil_flags | u8 | Bitflags: 0x01=+ATK, 0x02=+SPD, 0x04=+DIG |

**Size**: 8 + 1 + 1 = 10 bytes

### 6. Enums

#### ItemType

| Variant | Value | Description |
|---------|-------|-------------|
| Tool | 0 | Equipped in tool slot, provides ATK and strikes |
| Gear | 1 | Equipped in gear slots, provides various effects |

#### ItemTag

| Variant | Value | Description |
|---------|-------|-------------|
| Stone | 0 | Armor, Shrapnel, durability |
| Scout | 1 | DIG, multi-strike, mobility |
| Greed | 2 | Gold generation, shards |
| Blast | 3 | Countdown bombs, non-weapon damage |
| Frost | 4 | Chill, SPD manipulation |
| Rust | 5 | Armor destruction |
| Blood | 6 | Bleed, sustain |
| Tempo | 7 | SPD, Turn 1 effects |

#### Rarity

| Variant | Value | Description |
|---------|-------|-------------|
| Common | 0 | Most items, base stats |
| Rare | 1 | Enhanced effects |
| Heroic | 2 | Powerful effects |
| Mythic | 3 | Build-defining effects |

#### Tier

| Variant | Value | Description |
|---------|-------|-------------|
| I | 0 | Base values (index 0 of values array) |
| II | 1 | Middle values (index 1) |
| III | 2 | Maximum values (index 2) |

### 7. ItemOffer (Struct)

A generated item offer at a POI. Not stored on-chain, computed on demand.

| Field | Type | Description |
|-------|------|-------------|
| item_id | [u8; 8] | Which item is offered |
| tier | Tier | Always Tier::I for fresh offers |
| price | u16 | Gold cost (if applicable) |

## Relationships

```
ItemDefinition (compile-time, 80 items)
    │
    └── referenced by ──► ItemInstance.item_id
                              │
                              └── contained in ──► PlayerInventory
                                                      │
                                                      └── owned by ──► Player (Pubkey)

ItemsetDefinition (compile-time, 12 sets)
    │
    └── required_items[] references ──► ItemDefinition.id
    │
    └── detected from ──► PlayerInventory (computed on demand)
```

## State Transitions

### Inventory Lifecycle

```
EMPTY                       New player, no items
    │
    ▼ (receive_item)
HAS_ITEMS                   Player has 1+ items equipped
    │
    ├─► (equip_gear)        Add gear if slots available
    │       └─► SLOTS_FULL  All gear slots used
    │
    ├─► (equip_tool)        Replace tool (old tool removed)
    │
    ├─► (fuse_items)        Combine 2 identical → 1 higher tier
    │
    └─► (unequip_item)      Remove item from slot
```

### Gear Slot Expansion

```
Week 1 Start: gear_slot_capacity = 4
    │
    ▼ (defeat week 1 boss)
Week 2 Start: gear_slot_capacity = 6
    │
    ▼ (defeat week 2 boss)
Week 3 Start: gear_slot_capacity = 8 (max)
```

### Item Fusion

```
[Item A: Tier I] + [Item A: Tier I]
    │
    ▼ (fuse_items, same ID, same tier)
[Item A: Tier II]

[Item A: Tier II] + [Item A: Tier II]
    │
    ▼ (fuse_items)
[Item A: Tier III]

[Item A: Tier III] + [Item A: Tier III]
    │
    ▼ (fuse_items)
ERROR: Already max tier
```

## Validation Rules

### Inventory

- `tool` can only hold ItemType::Tool
- `gear` slots can only hold ItemType::Gear
- Number of Some values in `gear` must not exceed `gear_slot_capacity`
- `gear_slot_capacity` must be 4, 6, or 8

### ItemInstance

- `item_id` must reference a valid ItemDefinition
- `tier` must be I, II, or III
- `tool_oil_flags` only applies to Tool items

### Fusion

- Both items must exist in inventory
- Both must have identical `item_id`
- Both must have identical `tier`
- `tier` must be I or II (not III)
- Both must be same type (both Tool or both Gear)

### Itemset Detection

- All items in `required_items` must be present in inventory
- Items can be any tier (tier doesn't affect set membership)

## Item Data Reference

### Item Count by Tag

| Tag | Tools | Gear | Total |
|-----|-------|------|-------|
| STONE | 2 | 8 | 10 |
| SCOUT | 2 | 8 | 10 |
| GREED | 2 | 8 | 10 |
| BLAST | 2 | 8 | 10 |
| FROST | 2 | 8 | 10 |
| RUST | 2 | 8 | 10 |
| BLOOD | 2 | 8 | 10 |
| TEMPO | 2 | 8 | 10 |
| **Total** | **16** | **64** | **80** |

### Itemsets (12)

| Set Name | Required Items | Bonus |
|----------|----------------|-------|
| Union Standard | G-ST-01, G-ST-02, G-SC-01 | Battle Start: +4 ARM, +1 DIG |
| Shard Circuit | G-GR-05, G-GR-06, G-GR-07, G-GR-08 | Shards trigger every turn |
| Demolition Permit | G-BL-01, G-BL-02, G-BL-03 | Bombs tick 1 turn faster |
| Fuse Network | T-BL-02, G-BL-05, G-BL-04 | First non-weapon damage +2 |
| Shrapnel Harness | G-ST-03, G-ST-06, T-ST-01 | Keep up to 3 Shrapnel at turn end |
| Rust Ritual | T-RU-01, G-RU-02, G-RU-03 | On Hit: +1 extra Rust |
| Swift Digger Kit | T-SC-01, G-SC-01, G-SC-06 | If DIG > enemy DIG: +2 strikes |
| Royal Extraction | G-GR-01, G-GR-04, T-GR-02 | Gold→Armor becomes 1→4 |
| Whiteout Initiative | G-FR-04, G-FR-03, G-TE-05 | Battle Start: +1 SPD; act first Turn 1: +2 Chill |
| Bloodrush Protocol | T-BO-01, G-BO-05, G-TE-01 | Turn 1: 2 Bleed; Bleed dmg: +1 SPD |
| Corrosion Payload | G-RU-02, G-BL-03, G-BL-05 | Bomb damage applies 1 Rust |
| Golden Shrapnel Exchange | G-GR-04, G-ST-06, G-GR-03 | Gold→Armor also gives +3 Shrapnel |
