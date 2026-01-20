# Implementation Plan: Boss System

**Branch**: `006-boss-system` | **Date**: 2026-01-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/006-boss-system/spec.md`

## Summary

The boss system provides 24 boss definitions (12 Biome A, 12 Biome B) with unique traits, stat scaling by act/stage, and phase mechanics for Week 3 finals. Bosses are stored as compile-time constants (like items and field enemies) with helper functions to select and scale bosses for any of the 80 campaign stages.

## Technical Context

**Language/Version**: Rust 2021 edition (Solana BPF target) + Anchor 0.31.1
**Primary Dependencies**: solana-program, anchor-lang, combat-system crate (for ItemEffect/TriggerType reuse)
**Storage**: Compile-time constants for boss definitions; BossInstance computed at runtime
**Testing**: cargo test (unit), Bankrun (integration)
**Target Platform**: Solana mainnet-beta compatible, MagicBlock ephemeral rollup compatible
**Project Type**: Single program within Anchor workspace
**Performance Goals**: Boss lookup and scaling < 1000 compute units
**Constraints**: All boss selection deterministic from stage number; no randomness
**Scale/Scope**: 24 bosses, 14 unique trait mechanics, 80-stage selection mapping

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
|-----------|--------|----------|
| I. Security-First | PASS | No accounts modified; pure lookup functions; no arithmetic overflow risk (stats are small u8/u16 values) |
| II. Test-Driven Development | PASS | Unit tests for all 24 bosses, selection logic, scaling formulas; integration tests for combat |
| III. Program Composability | PASS | Exposes clean IDL; BossInstance converts to CombatantInput for combat-system CPI |
| IV. Anchor Framework | PASS | Using Anchor 0.31.1; error enums defined; IDL generated |
| V. MagicBlock Ephemeral Rollups | PASS | Boss definitions are read-only; no state accounts; compatible with ephemeral sessions |

**Gate Status**: PASS - All constitution principles satisfied.

## Project Structure

### Documentation (this feature)

```text
specs/006-boss-system/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── boss_system.idl.json
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
programs/boss-system/
├── Cargo.toml
└── src/
    ├── lib.rs           # Program entry, instructions
    ├── bosses.rs        # BOSSES const array, lookup functions
    ├── traits.rs        # BossTrait definitions per boss
    ├── selection.rs     # Stage→boss mapping logic
    ├── scaling.rs       # Act/tier stat modifiers
    └── errors.rs        # Custom error enum

tests/
├── boss_lookup.rs       # Unit tests for boss registry
├── boss_selection.rs    # Unit tests for stage mapping
├── boss_scaling.rs      # Unit tests for stat calculations
└── integration/
    └── boss_combat.rs   # Integration tests with combat system
```

**Structure Decision**: Single program `boss-system` following existing pattern from `player-inventory` and `field-enemies`. Static data in Rust consts, helper functions for selection and scaling, IDL exposed for client queries.

## Complexity Tracking

> No constitution violations requiring justification.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| N/A | - | - |
