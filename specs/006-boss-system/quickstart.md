# Quickstart: Boss System

**Feature**: 006-boss-system
**Date**: 2026-01-19

## Overview

The boss system provides 24 boss definitions across 2 biomes with unique traits, stat scaling, and phase mechanics. Bosses are selected deterministically based on campaign stage and scaled for difficulty progression.

## Key Concepts

### Boss Structure

```
BossDefinition (compile-time constant)
├── id: "B-A-W1-01"
├── name: "The Broodmother"
├── biome: A
├── week: One
├── base_stats: { hp: 32, atk: 2, arm: 2, spd: 3, dig: 1 }
├── weakness_tags: [STONE, FROST]
└── traits: [
      { trigger: BattleStart, effect: GainStrikes, value: 2 },  // 3 total
      { trigger: EveryOtherTurn, effect: ApplyChill, value: 1 }
    ]
```

### Boss Distribution

| Week | Biome A | Biome B |
|------|---------|---------|
| Week 1 | 5 archetypes | 5 variants (+1 SPD) |
| Week 2 | 5 archetypes | 5 variants (+1 SPD) |
| Week 3 | 2 finals | 2 unique finals |
| **Total** | **12** | **12** |

### Act/Biome Mapping

| Act | Stages | Biome | Modifiers |
|-----|--------|-------|-----------|
| 1 | 1-20 | A | None |
| 2 | 21-40 | B | +1 SPD baseline |
| 3 | 41-60 | A+ | +1 ATK (W1/W2), +2 ATK (W3) |
| 4 | 61-80 | B+ | +1 ATK +1 SPD (W1/W2), +2 ATK +1 SPD (W3) |

## Usage Examples

### Select Boss for Stage (Rust)

```rust
use boss_system::{select_boss, Week, Biome};

// Get boss for Stage 1, Week 1
let boss = select_boss(1, Week::One);
assert_eq!(boss.name, "The Broodmother");
assert_eq!(boss.biome, Biome::A);

// Get boss for Stage 25, Week 2 (Biome B)
let boss = select_boss(25, Week::Two);
assert_eq!(boss.biome, Biome::B);
// Stats will have +1 SPD from Biome B baseline
```

### Get Scaled Boss Stats (Rust)

```rust
use boss_system::{get_scaled_boss_stats, Week};

// Stage 55 in Act 3, tier = (55-41)/5 = 2
let stats = get_scaled_boss_stats(55, Week::Two);
// Week 2 tier 2 scaling: +6 HP, +2 ARM, +1 ATK
// Act 3 baseline: +1 ATK
// If base boss has HP=46, ATK=2, ARM=10:
//   scaled: HP=52, ATK=4, ARM=12
```

### Get Boss for Combat (TypeScript)

```typescript
import { Program } from "@coral-xyz/anchor";
import { BossSystem } from "../target/types/boss_system";

// Get combat-ready boss data
const combatInput = await program.methods
  .getBossCombatInput(
    stage,           // u8: 1-80
    { one: {} }      // Week enum
  )
  .view();

// Pass to combat system
await combatProgram.methods
  .initializeCombat(playerInput, combatInput, playerEffects, bossEffects)
  .accounts({ /* ... */ })
  .rpc();
```

### Get Boss Weaknesses for Item Generation (TypeScript)

```typescript
// During Week 2 exploration, get boss weaknesses for item weighting
const weaknesses = await program.methods
  .getBossWeaknesses(stage, { two: {} })
  .view();

// weaknesses = [ItemTag.Frost, ItemTag.Tempo] for Drill Sergeant
// Use in item offer generation: 1.4x weight for FROST and TEMPO items
```

### Get Boss Effects for Combat (TypeScript)

```typescript
// Get all boss traits as ItemEffect array
const bossEffects = await program.methods
  .getBossEffects(stage, { one: {} })
  .view();

// bossEffects for The Broodmother:
// [
//   { trigger: "BattleStart", effectType: "GainStrikes", value: 2 },
//   { trigger: "EveryOtherTurn", effectType: "ApplyChill", value: 1 }
// ]
```

## Development Commands

```bash
# Build the program
anchor build -p boss_system

# Run unit tests
cargo test -p boss-system

# Run integration tests
anchor test --skip-local-validator

# Generate IDL
anchor idl parse -f programs/boss-system/src/lib.rs -o target/idl/boss_system.json
```

## Boss Lookup (Rust)

```rust
use boss_system::{BOSSES, get_boss, get_bosses_by_week};

// Get specific boss by ID
let broodmother = get_boss(b"B-A-W1-01\0\0\0").expect("Boss exists");
assert_eq!(broodmother.base_stats.hp, 32);

// Get all Week 1 Biome A bosses
let week1_bosses = get_bosses_by_week(Biome::A, Week::One);
assert_eq!(week1_bosses.len(), 5);

// Get Week 3 finals for current biome
let finals = get_bosses_by_week(Biome::B, Week::Three);
assert_eq!(finals.len(), 2);
```

## Stage Selection Algorithm

```rust
pub fn select_boss(stage: u8, week: Week) -> &'static BossDefinition {
    let act = (stage - 1) / 20;                    // 0-3
    let stage_in_act = (stage - 1) % 20 + 1;       // 1-20
    let biome = if act % 2 == 0 { Biome::A } else { Biome::B };

    let boss_index = match week {
        Week::One => (stage_in_act - 1) % 5,
        Week::Two => (stage_in_act - 1 + 2) % 5,   // Offset to avoid same pairing
        Week::Three => stage_in_act % 2,           // 0 or 1
    };

    get_boss_by_index(biome, week, boss_index as u8)
}
```

## Stat Scaling Example

```rust
// Stage 75 in Act 4 (B+), Week 3 Final
// tier = (75-61)/5 = 2 (stages 71-75)
// Boss: The Frostbound Leviathan (B-B-W3-01)
// Base: HP=74, ATK=4, ARM=14, SPD=2, DIG=3

// Week 3 tier 2 scaling: +8 HP, +2 ARM, +1 ATK
// Act 4 baseline: +2 ATK, +1 SPD

// Final scaled stats:
// HP = 74 + 8 = 82
// ATK = 4 + 1 + 2 = 7
// ARM = 14 + 2 = 16
// SPD = 2 + 1 = 3
// DIG = 3 (unchanged)
```

## Phase Mechanics (Week 3 Finals)

```rust
use boss_system::{PhaseState, check_phase_transition};

let mut phases = PhaseState::default();

// Combat progresses...
let boss_hp = 54;  // 75% of 72 max HP
let boss_max_hp = 72;

if let Some(phase) = check_phase_transition(boss_hp, boss_max_hp, &mut phases) {
    match phase {
        Phase::One => {
            // The Eldritch Mole: +10 Armor
            // (reduced by 10 if player DIG > boss DIG)
        }
        Phase::Two => {
            // Attacks twice per turn
        }
        Phase::Three => {
            // Turn Start: apply 2 Bleed
        }
    }
}
```

## Common Patterns

### Initializing Boss Combat

```rust
// In gameplay-state when entering boss fight
let boss = select_boss(game_state.current_stage, week);
let scaled = scale_boss(&boss, act, stage_in_act, week);
let boss_effects = get_boss_effects(&boss);

let boss_input = CombatantInput {
    hp: scaled.hp as i16,
    max_hp: scaled.hp,
    atk: scaled.atk as i16,
    arm: scaled.arm as i16,
    spd: scaled.spd as i16,
    dig: scaled.dig as i16,
    strikes: boss.get_base_strikes(),
};

// Initialize combat with combat-system
combat_system::initialize_combat(
    ctx,
    player_input,
    boss_input,
    player_effects,
    boss_effects,
)?;
```

### Handling Special Mechanics

```rust
// Crystal Mimic reflection
if boss.has_reflection() {
    combat_state.boss_combat_state.reflection_state = Some(ReflectionState {
        stacks: 2,
        glass_heart_active: false,
    });
}

// Powder Keg Baron countdown
if let Some(countdown) = boss.get_countdown_trait() {
    combat_state.boss_combat_state.countdown_states.push(CountdownState {
        trait_index: countdown.index,
        turns_remaining: countdown.turns,
    });
}
```

## Weakness Tags Reference

| Boss | Weaknesses |
|------|------------|
| The Broodmother | STONE, FROST |
| Obsidian Golem | RUST, BLAST |
| Gas Anomaly | BLOOD, TEMPO |
| Mad Miner | SCOUT, GREED |
| Shard Colossus | STONE, BLOOD |
| Drill Sergeant | FROST, TEMPO |
| Crystal Mimic | BLAST, SCOUT |
| Rust Regent | BLOOD, TEMPO |
| Powder Keg Baron | STONE, FROST |
| Greedkeeper | GREED, RUST |
| The Eldritch Mole | RUST, TEMPO |
| The Gilded Devourer | GREED, BLOOD |
| The Frostbound Leviathan | TEMPO, STONE |
| The Rusted Chronomancer | RUST, BLOOD |
