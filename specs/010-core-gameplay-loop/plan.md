# Implementation Plan: Core Gameplay Loop

**Branch**: `010-core-gameplay-loop` | **Date**: 2026-01-21 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/010-core-gameplay-loop/spec.md`

## Summary

Implement the complete core gameplay loop for Dungeons & Moles, including atomic session creation with deterministic map/enemy/POI generation, movement with automatic combat resolution, night-phase enemy movement, boss encounter triggers, atomic death/victory handling with progression, multi-session support, run economy, and item unlock progression.

## Technical Context

**Language/Version**: Rust 1.75+ (Solana BPF target)  
**Primary Dependencies**: Anchor 0.32+, solana-program, combat-system, field-enemies, boss-system, poi-system  
**Storage**: Solana accounts (PDAs)  
**Testing**: cargo test (unit), anchor test (integration with Bankrun)  
**Target Platform**: Solana mainnet-beta / devnet  
**Project Type**: Solana program monorepo  
**Performance Goals**: All operations complete within 200k compute units  
**Constraints**: Max account size ~10KB, max enemies per map ~10-15 for compute budget  
**Scale/Scope**: 40 campaign levels, 80 items, up to 40 concurrent sessions per player

## Constitution Check

_GATE: Must pass before Phase 0 research. Re-check after Phase 1 design._

| Principle                       | Status | Notes                                                                                      |
| ------------------------------- | ------ | ------------------------------------------------------------------------------------------ |
| I. Security-First               | PASS   | All account validation via Anchor constraints; checked arithmetic; deterministic PDA seeds |
| II. Test-Driven Development     | PASS   | Tests specified per user story; TDD workflow required                                      |
| III. Program Composability      | PASS   | Clean IDL; CPI for combat resolution; standard event emission                              |
| IV. Anchor Framework            | PASS   | Anchor 0.32+; constraint-based validation; custom error enums                              |
| V. MagicBlock Ephemeral Rollups | PASS   | Session/state separation maintained; delegation-compatible design                          |

## Project Structure

### Documentation (this feature)

```text
specs/010-core-gameplay-loop/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── session-manager.md
│   ├── gameplay-state.md
│   ├── player-profile.md
│   └── game-loop.md     # New unified program
└── tasks.md             # Phase 2 output
```

### Source Code (repository root)

```text
programs/
├── player-profile/src/
│   ├── lib.rs           # Add purchase_runs, update item unlock logic
│   ├── state.rs         # Add unlocked_items, active_item_pool, highest_level_unlocked
│   ├── constants.rs     # Update INITIAL_AVAILABLE_RUNS, add new constants
│   ├── errors.rs        # Add new error codes
│   └── bitmask.rs       # NEW: Bitmask helper module
├── session-manager/src/
│   ├── lib.rs           # Update start_session with validations, atomic init
│   ├── state.rs         # Update PDA seeds, add active_item_pool
│   └── errors.rs        # Add LevelNotUnlocked, NoAvailableRuns
├── gameplay-state/src/
│   ├── lib.rs           # Add move_with_combat instruction
│   ├── state.rs         # Add enemy/POI references
│   └── movement.rs      # NEW: Movement + combat logic
├── field-enemies/src/
│   ├── state.rs         # Already has MapEnemies, EnemyInstance
│   └── night.rs         # NEW: Night movement logic
├── combat-system/src/
│   └── lib.rs           # Expose inline combat resolution
├── poi-system/src/
│   ├── state.rs         # Already has MapPois, PoiInstance
│   └── interactions.rs  # Add position validation
└── player-inventory/src/
    └── items.rs         # Add Basic Pickaxe (T-XX-00)

tests/
├── player-profile/
│   ├── run_economy.rs
│   └── item_progression.rs
├── session-manager/
│   └── multi_session.rs
└── gameplay-state/
    ├── movement_combat.rs
    ├── night_mechanics.rs
    └── boss_trigger.rs
```

**Structure Decision**: Existing Solana program monorepo structure maintained. Changes distributed across existing programs with new modules for movement/night logic.

## Complexity Tracking

No constitution violations requiring justification.
