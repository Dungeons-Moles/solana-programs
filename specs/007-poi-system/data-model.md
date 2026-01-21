# Data Model: POI System

**Feature**: 007-poi-system
**Date**: 2026-01-20

## Entities

### PoiDefinition (Compile-time Constant)

Static POI type data. 14 definitions stored as compile-time constants.

| Field | Type | Description |
|-------|------|-------------|
| id | u8 | POI type ID (1-14 mapping to L1-L14) |
| name | &'static str | Display name (e.g., "Mole Den") |
| emoji | [u8; 4] | UTF-8 encoded emoji |
| rarity | PoiRarity | Fixed, Common, Uncommon, Rare |
| use_type | UseType | OneTime, Repeatable, RepeatablePerTool |
| active_condition | ActiveCondition | Anytime, NightOnly |
| interaction_type | InteractionType | Enum of interaction categories |

**Validation Rules**:
- ID must be 1-14
- Name must be non-empty
- Rarity matches GDD Section 12

### PoiInstance (Runtime State)

Runtime POI placed on the map.

| Field | Type | Description |
|-------|------|-------------|
| poi_type | u8 | References PoiDefinition ID (1-14) |
| x | u8 | X coordinate on map (0-31) |
| y | u8 | Y coordinate on map (0-31) |
| used | bool | True if one-time POI has been used |
| discovered | bool | True if Rail Waypoint discovered |
| week_spawned | u8 | Week when spawned (for Counter Cache) |

**Validation Rules**:
- poi_type must be 1-14
- x, y must be within map bounds
- used starts false for one-time POIs
- discovered starts false for Rail Waypoints

**State Transitions**:
- `used`: false → true (irreversible for one-time POIs)
- `discovered`: false → true (irreversible for Rail Waypoints)

### MapPois (Account)

Session-scoped account holding all POI instances for a map.

| Field | Type | Description |
|-------|------|-------------|
| session | Pubkey | Reference to GameSession PDA |
| bump | u8 | PDA bump seed |
| pois | Vec<PoiInstance> | All POIs on this map (max 50) |
| count | u8 | Number of POIs |
| shop_state | Option<ShopState> | Active shop session (if any) |

**PDA Seeds**: `["map_pois", session.key().as_ref()]`

**Space**: 8 + 32 + 1 + 4 + (50 * 6) + 1 + (1 + 60) = 407 bytes

**Validation Rules**:
- session must be valid GameSession PDA
- pois.len() == count
- count <= 50

### ShopState (Inline Struct)

State for active Smuggler Hatch shopping session.

| Field | Type | Description |
|-------|------|-------------|
| poi_index | u8 | Index of Smuggler Hatch in MapPois.pois |
| offers | [ItemOffer; 6] | Current item offers (1 Tool + 5 Gear) |
| reroll_count | u8 | Number of rerolls this visit |
| active | bool | Whether shop session is active |

**Validation Rules**:
- poi_index must reference a Smuggler Hatch (L9)
- reroll_count starts at 0
- Reroll cost = 4 + (2 * reroll_count) Gold

**State Transitions**:
- Enter shop: active=false → active=true, generate offers
- Reroll: reroll_count += 1, regenerate offers
- Purchase: remove offer from display
- Leave shop: active=true → active=false, clear state

### ItemOffer (Inline Struct)

From player-inventory, represents a single item offer at a POI.

| Field | Type | Description |
|-------|------|-------------|
| item_id | [u8; 8] | Item definition ID (e.g., "T-ST-01\0") |
| tier | Tier | Always Tier::I for fresh offers |
| price | u16 | Gold cost (0 for free POIs) |

### ActSpawnConfig (Compile-time Constant)

Per-act configuration for POI spawning.

| Field | Type | Description |
|-------|------|-------------|
| act | u8 | Act number (1-4) |
| baseline | &'static [(u8, u8)] | (poi_type, count) pairs for baseline spawns |
| guarantees | &'static [SpawnGuarantee] | Guaranteed POI placements |

### SpawnGuarantee (Compile-time Constant)

Single guaranteed POI placement rule.

| Field | Type | Description |
|-------|------|-------------|
| poi_type | u8 | POI type to place |
| count | u8 | Number to place |
| week | Option<u8> | Week restriction (None = any week) |
| chance_percent | u8 | 100 = guaranteed, <100 = probability |
| placement | PlacementRule | Placement strategy |

## Enums

### PoiRarity
```
Fixed      // L1 Mole Den only
Common     // L2, L4, L5, L6
Uncommon   // L3, L7, L8, L9, L10, L13, L14
Rare       // L11, L12
```

### UseType
```
OneTime           // Can only be used once per map
Repeatable        // Can be used multiple times
RepeatablePerTool // Can be used once per different tool
```

### ActiveCondition
```
Anytime   // Can be used day or night
NightOnly // Can only be used during night phases
```

### InteractionType
```
RestFull         // L1: Skip to day, restore all HP
RestPartial      // L5: Skip to day, heal 10 HP
PickItem         // L2, L3, L12, L13: Pick 1 of N items
ToolOil          // L4: Modify tool stats
MapRevealRadius  // L6: Reveal tiles in radius
MapRevealNearest // L7: Reveal nearest POI of category
FastTravel       // L8: Teleport between waypoints
Shop             // L9: Purchase items with gold
Upgrade          // L10: Upgrade tool tier for gold
Fuse             // L11: Fuse identical items
Scrap            // L14: Destroy gear for gold cost
```

### PlacementRule
```
Random           // Place randomly on map
AdjacentToStart  // Place next to starting position (L1)
```

## Relationships

```
GameSession (1) ──────────── (1) MapPois
                                    │
                                    ├── (0..50) PoiInstance
                                    │              │
                                    │              └── references PoiDefinition
                                    │
                                    └── (0..1) ShopState
                                                   │
                                                   └── (6) ItemOffer
                                                              │
                                                              └── references ItemDefinition

GameState ────────── referenced by POI interactions
    │                  (phase, gold, HP)
    │
    └── needs gold: u16 field added

BossSystem ────────── provides weakness tags
                       for item offer generation

PlayerInventory ────── provides item definitions
                        and ItemOffer type
```

## Entity Dependency Graph

```
[PoiDefinition]          (static, compile-time)
       │
       ▼
[PoiInstance] ◄──────── [MapPois] ◄──────── [GameSession]
       │                     │
       │                     ▼
       │              [ShopState]
       │                     │
       ▼                     ▼
[Interaction Logic] ◄── [ItemOffer] ◄── [ItemDefinition]
       │
       ▼
[GameState] (HP, Gold, Phase)
```

## Indexes / Queries

| Query | Access Pattern |
|-------|----------------|
| Get POI by position | MapPois.pois.iter().find(\|p\| p.x == x && p.y == y) |
| Get POI by index | MapPois.pois[index] |
| Get all unused POIs | MapPois.pois.iter().filter(\|p\| !p.used) |
| Get discovered waypoints | MapPois.pois.iter().filter(\|p\| p.poi_type == 8 && p.discovered) |
| Get POI definition | POIS[poi_type - 1] (compile-time array) |
| Get spawn config | SPAWN_CONFIGS[act - 1] (compile-time array) |
