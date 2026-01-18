# Implementation Plan: Gameplay State Tracking

**Branch**: `002-gameplay-state-tracking` | **Date**: 2025-01-17 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/002-gameplay-state-tracking/spec.md`

## Summary

This feature extends the Solana programs to track core gameplay state on-chain: player position, movement, stats (HP/ATK/ARM/SPD/DIG), gear slots, and day/night/week progression. The implementation creates a new `gameplay-state` program that links to existing `session-manager` sessions, enabling verifiable runs for future PvP and leaderboard features. Combat, enemies, POIs, and gold are explicitly out of scope.

## Technical Context

**Language/Version**: Rust 2021 edition, Solana SDK 2.x
**Primary Dependencies**: Anchor 0.31.1 (matching existing programs)
**Storage**: Solana accounts (PDA-based state)
**Testing**: Bankrun for integration tests, `cargo test` for unit tests
**Target Platform**: Solana mainnet/devnet, MagicBlock ephemeral rollups
**Project Type**: Single project (new program in existing workspace)
**Performance Goals**: All operations < 200,000 compute units, confirmation < 2 seconds on devnet
**Constraints**: GameState account < 200 bytes (per SC-007), checked arithmetic for all operations
**Scale/Scope**: 80 campaign levels, 3 weeks × 6 phases per run, 5 stats per player

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Security-First | ✅ PASS | Account validation via Anchor constraints, checked arithmetic, PDA seeds documented |
| II. Test-Driven Development | ✅ PASS | Tests will be written before implementation, edge cases defined in spec |
| III. Program Composability | ✅ PASS | Clean IDL via Anchor, links to session-manager via CPI context |
| IV. Anchor Framework | ✅ PASS | Using Anchor 0.31.1, custom error enums planned |
| V. MagicBlock Ephemeral Rollups | ✅ PASS | GameState designed for delegation pattern (session-scoped, separate from persistent PlayerProfile) |

**Gate Result**: PASS - No violations detected

## Project Structure

### Documentation (this feature)

```text
specs/002-gameplay-state-tracking/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (IDL specifications)
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
programs/
├── player-profile/      # Existing (001-solana-core-programs)
├── session-manager/     # Existing (001-solana-core-programs)
├── map-generator/       # Existing (001-solana-core-programs)
└── gameplay-state/      # NEW - This feature
    ├── Cargo.toml
    └── src/
        ├── lib.rs           # Program entry, instruction handlers
        ├── state.rs         # GameState, Phase enum
        ├── errors.rs        # Custom error enum
        └── constants.rs     # Move costs, stat defaults, phase durations

tests/
└── gameplay-state/      # Bankrun integration tests
    └── gameplay-state.ts
```

**Structure Decision**: New program `gameplay-state` added to existing workspace. Follows the established pattern from `session-manager` and `player-profile` programs.

## Complexity Tracking

> No violations detected - table not required.
