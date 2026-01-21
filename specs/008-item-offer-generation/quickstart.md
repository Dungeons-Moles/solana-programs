# Quickstart: Item Offer Generation

**Feature**: 008-item-offer-generation
**Date**: 2026-01-20

## Overview

This feature adds item offer generation to the POI system. When players interact with item-granting POIs, the system generates weighted, deterministic item offers based on:

- **POI type** (Supply Cache, Tool Crate, Geode Vault, Counter Cache, Smuggler Hatch)
- **Current act** (1-4) for rarity distribution
- **Current week** (1-3) and boss weakness tags for tag weighting

## Key Concepts

### Tag Weighting

Items are tagged with one of 8 categories: Stone, Scout, Greed, Blast, Frost, Rust, Blood, Tempo.

When generating offers, tags matching the current week's boss weaknesses get 1.4x weight, making counter-builds more likely without guaranteeing them.

### Rarity Tables

Each POI type has act-specific rarity distributions:
- **Act 1**: Mostly Common items
- **Act 4**: Better odds for Rare/Heroic/Mythic

### Deterministic RNG

All randomness uses the session seed, ensuring identical offers for replay verification.

## Quick Examples

### 1. Supply Cache Interaction (Act 1)

```typescript
// After player moves to Supply Cache tile
const mapPoisPda = getMapPoisPda(session);
const gameStatePda = getGameStatePda(session);

// Generate offer (3 Common Gear items, tag-weighted)
await program.methods
  .generateCacheOffer()
  .accounts({
    player: wallet.publicKey,
    session: sessionPda,
    mapPois: mapPoisPda,
    gameState: gameStatePda,
  })
  .rpc();

// Read the generated offer
const mapPois = await program.account.mapPois.fetch(mapPoisPda);
console.log("Offer:", mapPois.currentOffer);
// { poiIndex: 5, items: [{ itemId: "G-ST-01", rarity: 0 }, ...] }
```

### 2. Smuggler Hatch Shop

```typescript
// Generate shop inventory (1 Tool + 5 Gear)
await program.methods
  .generateShopInventory()
  .accounts({
    player: wallet.publicKey,
    session: sessionPda,
    mapPois: mapPoisPda,
    gameState: gameStatePda,
  })
  .rpc();

// Reroll for better items (costs 4 Gold first time)
await program.methods
  .rerollShop()
  .accounts({
    player: wallet.publicKey,
    session: sessionPda,
    mapPois: mapPoisPda,
    gameState: gameStatePda,
  })
  .rpc();

// Check updated inventory
const mapPois = await program.account.mapPois.fetch(mapPoisPda);
console.log("Shop:", mapPois.shopState);
// { rerollCount: 1, inventory: [...], ... }
```

### 3. Counter Cache (Boss Weakness Tags Only)

```typescript
// Counter Cache draws exclusively from boss weakness tags
// If current boss is Broodmother (Stone + Frost weakness),
// all 3 items will be Stone or Frost tagged

await program.methods
  .generateCacheOffer()
  .accounts({
    player: wallet.publicKey,
    session: sessionPda,
    mapPois: mapPoisPda,
    gameState: gameStatePda,
  })
  .rpc();
```

## Testing Locally

```bash
# Build the program
anchor build

# Run unit tests (offer generation logic)
cargo test -p poi-system

# Run integration tests (full flow)
anchor test --skip-local-validator
```

## Key Files

| File | Purpose |
|------|---------|
| `programs/poi-system/src/offer_generation.rs` | Core generation logic |
| `programs/poi-system/src/rarity_tables.rs` | Act-based rarity distributions |
| `programs/poi-system/src/tag_weights.rs` | Boss weakness weighting |
| `tests/poi-system/offer_generation.ts` | Integration tests |

## Common Patterns

### Getting Boss Weakness Tags

```rust
use boss_system::get_boss_for_stage;

let boss = get_boss_for_stage(act, week, biome, stage_index);
let weakness_tags = boss.weakness_tags; // [ItemTag; 2]
```

### Computing Tag Weights

```rust
use crate::tag_weights::compute_tag_weights;

let weights = compute_tag_weights(
    &weakness_tags,
    enable_final_prep_bias,
    week,
    biome,
);
// weights.weights[Stone] = 14000 if Stone is a weakness
// weights.total = sum of all weights
```

### Sampling Item

```rust
use crate::offer_generation::{sample_rarity, select_item};

let mut rng = Xorshift64::new(seed);
let rarity = sample_rarity(&mut rng, &rarity_table, &mut mythic_used);
let tag = select_weighted_tag(&mut rng, &weights);
let item = select_item(&mut rng, tag, rarity, ItemType::Gear)?;
```

## Troubleshooting

### "NoItemsAvailable" Error

The selected (tag, rarity, type) combination has no items. This happens when:
- Requesting Mythic for a tag with no Mythic items
- Requesting Tool with Mythic rarity (no Mythic tools exist)

**Solution**: The system automatically falls back to next-lower rarity.

### Offers Not Matching Expected Distribution

Verify the session seed is consistent. Different seeds produce different offers. For testing, use a known seed:

```rust
let test_seed: u64 = 12345;
```

### Shop Reroll Cost Confusion

Reroll cost formula: `4 + 2 * (reroll_count - 1)`
- 1st reroll: 4 Gold
- 2nd reroll: 6 Gold
- 3rd reroll: 8 Gold

Note: `reroll_count` is 1-indexed for cost calculation.
