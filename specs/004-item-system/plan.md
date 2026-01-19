# Implementation Plan: Item System

**Branch**: `004-item-system` | **Date**: 2026-01-19 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/004-item-system/spec.md`

## Summary

Implement the item system for Dungeons & Moles, including: a registry of 80 item definitions (10 per tag across 8 tags), player inventory management (Tool slot + Gear slots), item tier upgrades via fusion, 12 itemset bonuses, weighted item offer generation for POIs, and integration with the combat system via ItemEffect arrays.

## Technical Context

**Language/Version**: Rust 2021 edition (Solana BPF target)
**Primary Dependencies**: Anchor 0.31.1 (matching existing programs), solana-program
**Storage**: Solana accounts (PDA-based state) + on-chain item registry
**Testing**: Bankrun for integration tests, cargo test for unit tests
**Target Platform**: Solana mainnet/devnet, MagicBlock Ephemeral Rollups
**Project Type**: Single Anchor workspace with multiple programs
**Performance Goals**: Item lookups and inventory operations must complete within Solana compute limits
**Constraints**: Deterministic, compute-efficient, ephemeral rollup compatible
**Scale/Scope**: 80 item definitions, 12 itemsets, inventory up to 9 items (1 Tool + 8 Gear)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Security-First | PASS | Anchor constraints for inventory validation, checked arithmetic |
| II. Test-Driven Development | PASS | Tests for all item operations before implementation |
| III. Program Composability | PASS | Clean IDL via Anchor, items converted to combat-compatible format |
| IV. Anchor Framework | PASS | Using Anchor 0.31.1, custom error enums, generated IDL |
| V. MagicBlock Ephemeral Rollups | PASS | Inventory state is session-scoped, delegatable |

**Gate Result**: PASS - No violations, proceed to Phase 0.

## Project Structure

### Documentation (this feature)

```text
specs/004-item-system/
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
├── item-registry/           # NEW - Static item/itemset definitions
│   └── src/
│       ├── lib.rs           # Program entry, query instructions
│       ├── items.rs         # All 80 item definitions
│       ├── itemsets.rs      # All 12 itemset definitions
│       ├── state.rs         # ItemDefinition, ItemsetDefinition structs
│       └── errors.rs        # Custom error types
├── player-inventory/        # NEW - Player item ownership and management
│   └── src/
│       ├── lib.rs           # Program entry, inventory instructions
│       ├── state.rs         # PlayerInventory, ItemInstance structs
│       ├── fusion.rs        # Item tier upgrade logic
│       ├── effects.rs       # Convert inventory to ItemEffect arrays
│       ├── constants.rs     # Slot limits, tier values
│       └── errors.rs        # Custom error types
├── combat-system/           # EXISTING - Will consume ItemEffect arrays
├── gameplay-state/          # EXISTING - Tracks week/boss for offer generation
├── player-profile/          # EXISTING - No changes
├── session-manager/         # EXISTING - No changes
└── map-generator/           # EXISTING - No changes

tests/
├── item-registry/           # NEW
│   └── integration/
│       └── item_queries.ts
├── player-inventory/        # NEW
│   ├── unit/
│   │   ├── fusion.rs
│   │   └── effects.rs
│   └── integration/
│       ├── inventory_management.ts
│       ├── itemsets.ts
│       └── combat_integration.ts
```

**Structure Decision**: Two programs - `item-registry` for static data (item/itemset definitions) and `player-inventory` for player state (owned items, equipped items). This separation allows:
1. Item definitions to be deployed once and never change
2. Inventory to be delegated to ephemeral rollups independently
3. Clear ownership boundaries for security

## Complexity Tracking

> No violations requiring justification.
