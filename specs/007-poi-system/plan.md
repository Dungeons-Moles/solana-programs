# Implementation Plan: POI System

**Branch**: `007-poi-system` | **Date**: 2026-01-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/007-poi-system/spec.md`

## Summary

Implement a POI (Points of Interest) system with 14 POI types supporting item offers, shop mechanics, healing, upgrades, fast travel, and map reveal. Following established patterns from boss-system and player-inventory, POI definitions will be compile-time constants with stateless view functions for queries, while runtime POI state (used/unused) will be tracked in a session-scoped account.

## Technical Context

**Language/Version**: Rust 2021 edition (Solana BPF target)
**Primary Dependencies**: Anchor 0.32.0, solana-program
**Storage**: PDA-based state for per-session POI instances (MapPois account)
**Testing**: cargo test + anchor test (ts-mocha), Bankrun for integration
**Target Platform**: Solana mainnet/devnet, MagicBlock Ephemeral Rollups
**Project Type**: Single Anchor program (poi-system) + dependencies on player-inventory, boss-system, gameplay-state
**Performance Goals**: Deterministic item generation, ~1000 CU per POI interaction
**Constraints**: Account size limits, compile-time constants for static data
**Scale/Scope**: 14 POI definitions, up to ~50 POI instances per map

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Security-First | ✅ | Account validation via Anchor constraints; checked arithmetic; PDA seeds documented |
| II. Test-Driven Development | ✅ | Tests will be written before implementation |
| III. Program Composability | ✅ | Clean IDL exposed; CPI interfaces documented; event emission for indexers |
| IV. Anchor Framework | ✅ | Using Anchor 0.32.0; constraints for validation; custom error enums |
| V. MagicBlock Ephemeral Rollups | ✅ | Session-scoped state; supports delegation pattern |

## Project Structure

### Documentation (this feature)

```text
specs/007-poi-system/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
programs/
├── poi-system/
│   └── src/
│       ├── lib.rs           # Program entry, instructions
│       ├── pois.rs          # 14 POI definitions (compile-time constants)
│       ├── offers.rs        # Item offer generation per POI type
│       ├── interactions.rs  # POI interaction logic
│       ├── spawn.rs         # Spawn configuration per act
│       ├── state.rs         # MapPois, PoiInstance, ShopState
│       └── errors.rs        # Custom error enum
├── player-inventory/        # Dependency (items, offers module exists)
├── boss-system/             # Dependency (weakness tags)
└── gameplay-state/          # Dependency (gold, HP, phase)

tests/
├── poi-system.ts            # Integration tests
└── unit/                    # Rust unit tests (in-program)
```

**Structure Decision**: Single Anchor program following established pattern (boss-system, field-enemies). POI definitions as compile-time constants. Runtime state in MapPois PDA per session.

## Complexity Tracking

No constitution violations requiring justification.
