# Implementation Plan: Combat System & Status Effects

**Branch**: `003-combat-system` | **Date**: 2026-01-18 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/003-combat-system/spec.md`

## Summary

Implement a deterministic auto-combat system for Dungeons & Moles that resolves battles turn-by-turn based on combatant stats (HP, ATK, ARM, SPD, DIG). The system includes four status effects (Chill, Shrapnel, Rust, Bleed), seven trigger types for item/ability activation, multi-strike support, and stalemate prevention (sudden death at Turn 25, failsafe at Turn 50). All combat must be fully deterministic for on-chain verification.

## Technical Context

**Language/Version**: Rust 2021 edition (Solana BPF target)
**Primary Dependencies**: Anchor 0.31.1 (matching existing programs), solana-program
**Storage**: Solana accounts (PDA-based state)
**Testing**: Bankrun for integration tests, cargo test for unit tests
**Target Platform**: Solana mainnet/devnet, MagicBlock Ephemeral Rollups
**Project Type**: Single Anchor workspace with multiple programs
**Performance Goals**: Combat resolution must complete within Solana compute limits (~200k CU)
**Constraints**: Deterministic (no randomness), compute-efficient, ephemeral rollup compatible
**Scale/Scope**: Single program handling combat logic, integrated with gameplay-state

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Security-First | PASS | All arithmetic will use checked operations, Anchor constraints for validation |
| II. Test-Driven Development | PASS | Tests written before implementation (spec requires determinism tests) |
| III. Program Composability | PASS | Clean IDL via Anchor, combat can be invoked via CPI |
| IV. Anchor Framework | PASS | Using Anchor 0.31.1, custom error enums, generated IDL |
| V. MagicBlock Ephemeral Rollups | PASS | Combat state is session-scoped, designed for ephemeral delegation |

**Gate Result**: PASS - No violations, proceed to Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/003-combat-system/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (Anchor IDL)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
programs/
├── combat-system/           # NEW - Combat resolution program
│   └── src/
│       ├── lib.rs           # Program entry, instruction handlers
│       ├── state.rs         # CombatState, StatusEffects accounts
│       ├── engine.rs        # Core combat loop logic
│       ├── triggers.rs      # Trigger system implementation
│       ├── effects.rs       # Status effect processing
│       ├── constants.rs     # Combat constants
│       └── errors.rs        # Custom error types
├── gameplay-state/          # EXISTING - Will invoke combat
├── player-profile/          # EXISTING - Provides player stats
├── session-manager/         # EXISTING - Session validation
└── map-generator/           # EXISTING - No changes

tests/
├── combat-system/           # NEW
│   ├── unit/               # Pure logic tests (no blockchain)
│   │   ├── damage.rs       # Damage calculation tests
│   │   ├── status.rs       # Status effect tests
│   │   └── triggers.rs     # Trigger timing tests
│   └── integration/        # Bankrun tests
│       ├── basic_combat.ts
│       ├── status_effects.ts
│       └── determinism.ts
```

**Structure Decision**: Single program `combat-system` following existing patterns. Combat logic is compute-intensive and benefits from being a separate program that can be invoked via CPI from gameplay-state when player encounters an enemy.

## Complexity Tracking

> No violations requiring justification.
