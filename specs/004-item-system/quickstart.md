# Quickstart: Item System

**Feature**: 004-item-system
**Date**: 2026-01-19

## Overview

The item system provides 80 item definitions across 8 tags, player inventory management, item tier upgrades, 12 itemset bonuses, and weighted item offer generation for POIs.

## Key Concepts

### Item Structure

```
ItemDefinition (compile-time constant)
├── id: "T-ST-01"
├── name: "Bulwark Shovel"
├── item_type: Tool
├── tag: Stone
├── rarity: Common
└── effects: [
      { trigger: BattleStart, effect: GainArmor, values: [4, 6, 8] },
      ...
    ]
```

### Tier Scaling

Items have 3 tiers with increasing values:
- Tier I: `values[0]` (e.g., +3 ARM)
- Tier II: `values[1]` (e.g., +6 ARM)
- Tier III: `values[2]` (e.g., +9 ARM)

### Inventory Slots

| Slot Type | Count | Notes |
|-----------|-------|-------|
| Tool | 1 | Always exactly 1 tool |
| Gear | 4-8 | Starts at 4, expands after boss defeats |

### Item Tags (8)

| Tag | Theme |
|-----|-------|
| STONE | Armor, Shrapnel, durability |
| SCOUT | DIG, multi-strike, mobility |
| GREED | Gold generation, shard effects |
| BLAST | Countdown bombs, non-weapon damage |
| FROST | Chill, SPD manipulation |
| RUST | Armor destruction |
| BLOOD | Bleed, sustain |
| TEMPO | SPD, Turn 1 effects |

## Usage Examples

### Initialize Inventory (TypeScript)

```typescript
import { Program } from "@coral-xyz/anchor";
import { PlayerInventory } from "../target/types/player_inventory";

// Initialize player inventory
const [inventoryPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("inventory"), wallet.publicKey.toBuffer()],
  program.programId
);

await program.methods
  .initializeInventory()
  .accounts({
    inventory: inventoryPda,
    player: wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### Equip Items

```typescript
// Equip a tool
await program.methods
  .equipTool(
    [...Buffer.from("T-SC-01\0")], // Twin Picks
    { i: {} }                       // Tier I
  )
  .accounts({
    inventory: inventoryPda,
    player: wallet.publicKey,
  })
  .rpc();

// Equip gear
await program.methods
  .equipGear(
    [...Buffer.from("G-ST-01\0")], // Miner Helmet
    { i: {} }                       // Tier I
  )
  .accounts({
    inventory: inventoryPda,
    player: wallet.publicKey,
  })
  .rpc();
```

### Fuse Items (Upgrade Tier)

```typescript
// Fuse two Tier I gear items in slots 0 and 1 to get Tier II
await program.methods
  .fuseItems(0, 1) // gear slot indices
  .accounts({
    inventory: inventoryPda,
    player: wallet.publicKey,
  })
  .rpc();
```

### Get Combat Effects

```typescript
// Get effects for combat system
const effects = await program.methods
  .getCombatEffects()
  .accounts({
    inventory: inventoryPda,
    player: wallet.publicKey,
  })
  .view();

// Pass to combat system
await combatProgram.methods
  .resolveCombat(effects, enemyEffects)
  .accounts({ /* ... */ })
  .rpc();
```

### Check Active Itemsets (Rust)

```rust
use crate::items::{get_active_itemsets, ITEMSETS};

fn check_itemsets(inventory: &PlayerInventory) -> Vec<&'static ItemsetDefinition> {
    ITEMSETS.iter()
        .filter(|set| {
            set.required_items.iter().all(|id| {
                inventory.has_item(id)
            })
        })
        .collect()
}

// Usage in effect generation
let active_sets = check_itemsets(&inventory);
for set in active_sets {
    effects.push(set.bonus_effect.to_item_effect());
}
```

### Generate Item Offer

```typescript
// At a Supply Cache POI
const offer = await program.methods
  .generateItemOffer(
    { supplyCache: {} },  // POI type
    new BN(12345)         // deterministic seed
  )
  .accounts({
    gameState: gameStatePda,
  })
  .view();

console.log(`Offered: ${String.fromCharCode(...offer.itemId)}`);
console.log(`Price: ${offer.price} gold`);
```

## Development Commands

```bash
# Build both programs
anchor build -p player_inventory

# Run unit tests
cargo test -p player-inventory

# Run integration tests
anchor test --skip-local-validator

# Generate IDL
anchor idl parse -f programs/player-inventory/src/lib.rs -o target/idl/player_inventory.json
```

## Item Lookup (Rust)

```rust
// items.rs contains all 80 item definitions
use crate::items::{ITEMS, get_item, get_items_by_tag, get_items_by_rarity};

// Get specific item
let bulwark_shovel = get_item(b"T-ST-01\0").expect("Item exists");
assert_eq!(bulwark_shovel.name, "Bulwark Shovel");

// Get all FROST items
let frost_items = get_items_by_tag(ItemTag::Frost);
assert_eq!(frost_items.len(), 10);

// Get all Heroic items
let heroic_items = get_items_by_rarity(Rarity::Heroic);
```

## Itemset Detection

```rust
// itemsets.rs contains all 12 itemset definitions
use crate::itemsets::{ITEMSETS, get_active_itemsets};

// Example: Union Standard set
// Required: G-ST-01 (Miner Helmet), G-ST-02 (Work Vest), G-SC-01 (Miner Boots)
// Bonus: Battle Start +4 Armor, +1 DIG

let active = get_active_itemsets(&inventory);
for set in active {
    println!("Active set: {} {}", set.emoji, set.name);
    // Add bonus to combat effects
}
```

## Integration with Combat

```rust
// effects.rs - convert inventory to combat effects
pub fn generate_combat_effects(inventory: &PlayerInventory) -> Vec<ItemEffect> {
    let mut effects = Vec::new();

    // 1. Add tool effects
    if let Some(tool) = &inventory.tool {
        let def = get_item(&tool.item_id).unwrap();
        for effect in def.effects {
            effects.push(effect.to_item_effect(tool.tier));
        }
        // Add Tool Oil bonuses if applied
        if tool.tool_oil_flags & 0x01 != 0 {
            effects.push(ItemEffect::stat_bonus(EffectType::GainAtk, 1));
        }
    }

    // 2. Add gear effects
    for slot in inventory.gear.iter().flatten() {
        let def = get_item(&slot.item_id).unwrap();
        for effect in def.effects {
            effects.push(effect.to_item_effect(slot.tier));
        }
    }

    // 3. Add itemset bonuses
    for set in get_active_itemsets(inventory) {
        effects.push(set.bonus_effect.to_item_effect(Tier::I));
    }

    effects
}
```

## Common Patterns

### Adding a New Item

1. Add definition to `items.rs` ITEMS constant
2. Ensure ID follows pattern: `T-XX-##` for Tools, `G-XX-##` for Gear
3. Define all effects with tier-scaled values
4. Update tests

### Adding a New Itemset

1. Add definition to `itemsets.rs` ITEMSETS constant
2. List all required item IDs (3-4 items)
3. Define bonus effect
4. Update tests

### Handling Full Inventory

```rust
// Check before equipping
if inventory.gear_count() >= inventory.gear_slot_capacity {
    return Err(InventoryError::InventoryFull);
}
```
