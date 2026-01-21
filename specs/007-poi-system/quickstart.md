# Quickstart: POI System

**Feature**: 007-poi-system
**Date**: 2026-01-20

## Overview

The POI (Points of Interest) system provides 14 types of interactive locations that players encounter during exploration. POIs offer item acquisition, healing, upgrades, fast travel, and map reveal mechanics.

## Prerequisites

- Rust 1.75+ with Solana BPF target
- Anchor CLI 0.32.0+
- Node.js 18+ (for tests)
- Existing programs: gameplay-state, player-inventory, boss-system

## Setup

```bash
# From repository root
cd programs/poi-system

# Build the program
anchor build

# Run tests
anchor test
```

## Key Concepts

### POI Types

| ID | Name | Use | Description |
|----|------|-----|-------------|
| L1 | Mole Den | Repeatable | Night-only: Full heal, skip to day |
| L2 | Supply Cache | One-time | Pick 1 of 3 Gear items |
| L3 | Tool Crate | One-time | Pick 1 of 3 Tools |
| L4 | Tool Oil Rack | Per-tool | Modify tool: +1 ATK/SPD/DIG |
| L5 | Rest Alcove | One-time | Night-only: Heal 10 HP, skip to day |
| L6 | Survey Beacon | One-time | Reveal tiles in radius 13 |
| L7 | Seismic Scanner | One-time | Reveal nearest POI of category |
| L8 | Rail Waypoint | Repeatable | Fast travel between discovered waypoints |
| L9 | Smuggler Hatch | Repeatable | Shop: 1 Tool + 5 Gear, reroll for gold |
| L10 | Rusty Anvil | One-time | Upgrade tool tier (8/16 Gold) |
| L11 | Rune Kiln | Repeatable | Fuse 2 identical items (free) |
| L12 | Geode Vault | One-time | Pick 1 of 3 Heroic items |
| L13 | Counter Cache | One-time | Pick 1 of 3 weakness-tagged items |
| L14 | Scrap Chute | One-time | Destroy 1 Gear (8-12 Gold) |

### State Architecture

```
GameSession (existing)
    │
    ├── MapPois (new PDA per session)
    │       ├── pois: Vec<PoiInstance>
    │       └── shop_state: Option<ShopState>
    │
    ├── GameState (needs gold: u16)
    │
    └── PlayerInventory (existing)
```

## Usage Examples

### Initialize POIs for a Session

```typescript
import { PoiSystem } from "../target/types/poi_system";

const tx = await program.methods
  .initializeMapPois(act, week, seed)
  .accounts({
    payer: wallet.publicKey,
    session: sessionPda,
    mapPois: mapPoisPda,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Query POI at Position

```typescript
const poi = await program.methods
  .getPoiAtPosition(x, y)
  .accounts({ mapPois: mapPoisPda })
  .view();

if (poi) {
  console.log(`POI type ${poi.poiType} at (${poi.x}, ${poi.y})`);
}
```

### Pick Item from Supply Cache

```typescript
const result = await program.methods
  .interactPickItem(poiIndex, offerIndex, seed)
  .accounts({
    player: wallet.publicKey,
    gameState: gameStatePda,
    mapPois: mapPoisPda,
    playerInventory: inventoryPda,
  })
  .rpc();
```

### Shop Flow (Smuggler Hatch)

```typescript
// Enter shop
const offers = await program.methods
  .enterShop(poiIndex, seed)
  .accounts({
    player: wallet.publicKey,
    gameState: gameStatePda,
    mapPois: mapPoisPda,
  })
  .rpc();

// Purchase item
await program.methods
  .shopPurchase(offerIndex)
  .accounts({
    player: wallet.publicKey,
    gameState: gameStatePda,
    mapPois: mapPoisPda,
    playerInventory: inventoryPda,
  })
  .rpc();

// Reroll (costs gold)
const newOffers = await program.methods
  .shopReroll(newSeed)
  .accounts({
    player: wallet.publicKey,
    gameState: gameStatePda,
    mapPois: mapPoisPda,
  })
  .rpc();

// Leave shop
await program.methods
  .leaveShop()
  .accounts({
    player: wallet.publicKey,
    mapPois: mapPoisPda,
  })
  .rpc();
```

### Fast Travel (Rail Waypoint)

```typescript
// First visit - discover waypoint
await program.methods
  .interactRailWaypoint(poiIndex, null)
  .accounts({
    player: wallet.publicKey,
    gameState: gameStatePda,
    mapPois: mapPoisPda,
  })
  .rpc();

// Travel to another discovered waypoint
await program.methods
  .interactRailWaypoint(poiIndex, destinationIndex)
  .accounts({
    player: wallet.publicKey,
    gameState: gameStatePda,
    mapPois: mapPoisPda,
  })
  .rpc();
```

## Testing

### Unit Tests (Rust)

```bash
cargo test --package poi-system
```

### Integration Tests (TypeScript)

```bash
anchor test --skip-local-validator
```

### Key Test Cases

1. **POI Definition Tests**
   - All 14 POI types queryable
   - Properties match GDD specifications

2. **Item Offer Tests**
   - Rarity distribution matches act tables
   - Tag weighting biases toward boss weaknesses
   - Counter Cache only offers weakness tags

3. **Interaction Tests**
   - Night-only POIs reject day interactions
   - One-time POIs cannot be reused
   - Gold costs deducted correctly
   - Inventory updated correctly

4. **Shop Tests**
   - 6 offers generated (1 Tool + 5 Gear)
   - Reroll cost increases correctly
   - Purchase removes gold and adds item

5. **Fast Travel Tests**
   - First visit discovers waypoint
   - Cannot travel to undiscovered waypoints
   - Position updates on travel

## Common Patterns

### Deterministic Item Generation

All item offers use a deterministic seed:

```rust
let seed = combine_seeds(session_key, poi_index, action_count);
let offer = generate_item_offer(poi_type, act, week, boss_weaknesses, seed);
```

### Night-Only Validation

```rust
if poi_def.active_condition == ActiveCondition::NightOnly {
    require!(game_state.phase.is_night(), PoiSystemError::NightOnlyPoi);
}
```

### One-Time Use Tracking

```rust
require!(!poi_instance.used, PoiSystemError::PoiAlreadyUsed);
poi_instance.used = true;
```

## Dependencies

| Program | Purpose |
|---------|---------|
| gameplay-state | Phase, gold, HP, position |
| player-inventory | Items, inventory, offers |
| boss-system | Weakness tags for item weighting |

## Next Steps

1. Add `gold: u16` field to GameState (gameplay-state extension)
2. Implement poi-system program scaffold
3. Write failing tests for each POI type
4. Implement POI definitions
5. Implement interactions
6. Integration testing
