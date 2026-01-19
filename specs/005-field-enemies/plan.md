# Implementation Plan: Field Enemies

**Branch**: `005-field-enemies` | **Date**: 2026-01-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/005-field-enemies/spec.md`

## Summary

Implement field enemies system for Dungeons & Moles based on GDD Section 11. This includes 12 enemy archetypes with 3 tiers each, tier-based stat scaling, biome-weighted spawning, per-act distribution rules, and Gold rewards on defeat. Enemy traits integrate with the existing combat system triggers/effects infrastructure.

## Technical Context

**Language/Version**: Rust 1.75+ (Solana BPF target)  
**Primary Dependencies**: Anchor 0.32+, anchor-lang, solana-program  
**Storage**: On-chain Solana accounts via Anchor PDAs  
**Testing**: cargo test (unit), anchor test with Bankrun (integration)  
**Target Platform**: Solana mainnet/devnet, MagicBlock Ephemeral Rollups  
**Project Type**: Single Solana program workspace with multiple program crates  
**Performance Goals**: Deterministic seeded enemy generation, <200ms compute per instruction  
**Constraints**: Solana compute budget limits (~200k CU), account size limits, deterministic execution  
**Scale/Scope**: 12 archetypes × 3 tiers = 36 enemy configurations, 36-48 enemies per map

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

| Principle                       | Status      | Notes                                                                                   |
| ------------------------------- | ----------- | --------------------------------------------------------------------------------------- |
| I. Security-First               | PASS        | Enemy data is read-only static tables; Gold transfers via existing combat_system checks |
| II. Test-Driven Development     | WILL COMPLY | Tests written before implementation per workflow                                        |
| III. Program Composability      | PASS        | Enemy definitions expose clean structs via Anchor; compatible with combat_system CPI    |
| IV. Anchor Framework            | PASS        | Uses Anchor for all account structures and constraints                                  |
| V. MagicBlock Ephemeral Rollups | PASS        | Enemy instances are session-scoped; state compatible with delegation patterns           |

## Project Structure

### Documentation (this feature)

```text
specs/005-field-enemies/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
programs/
├── combat-system/src/
│   ├── state.rs         # Existing: StatusEffects, TriggerType, EffectType, ItemEffect
│   ├── triggers.rs      # Existing: Trigger/effect processing
│   ├── effects.rs       # Existing: Status effect processing
│   └── engine.rs        # Existing: Combat engine
├── gameplay-state/src/
│   ├── state.rs         # Existing: GameState with player position/stats
│   └── lib.rs           # Will need: enemy encounter integration
├── map-generator/src/
│   ├── state.rs         # Existing: MapConfig, TileType, Position
│   └── rng.rs           # Existing: Seeded RNG
└── field-enemies/src/   # NEW PROGRAM
    ├── lib.rs           # Program entry, instructions
    ├── state.rs         # EnemyArchetype, EnemyTier, FieldEnemy
    ├── archetypes.rs    # Static enemy definitions (12 archetypes × 3 tiers)
    ├── spawner.rs       # Tier distribution, biome weighting, spawn logic
    ├── traits.rs        # Enemy trait definitions as ItemEffect arrays
    ├── constants.rs     # Spawn counts, distribution percentages
    └── errors.rs        # Custom error types

tests/
├── field-enemies/
│   ├── unit/            # Per-archetype stat verification
│   ├── integration/     # Spawn distribution, combat integration
│   └── traits/          # Trait trigger verification
```

**Structure Decision**: New `field-enemies` program crate added to workspace. Enemy data is static/constant (compiled into program) with runtime enemy instances created as ephemeral state during map generation. Combat integration via CPI to existing `combat-system`.

## Complexity Tracking

No constitution violations to justify.
