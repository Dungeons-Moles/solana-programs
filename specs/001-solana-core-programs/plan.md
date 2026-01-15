# Implementation Plan: Solana Core Programs

**Branch**: `001-solana-core-programs` | **Date**: 2025-01-15 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-solana-core-programs/spec.md`

## Summary

Implement three foundational Solana programs for Dungeons & Moles: Player Profile (identity, progression, payments), Session Manager (MagicBlock ephemeral rollup delegation), and Map Generation (deterministic procedural generation with configurable seeds). Programs use Anchor framework with MagicBlock SDK integration for high-performance gaming sessions.

## Technical Context

**Language/Version**: Rust 1.75+ (Solana BPF target)
**Primary Dependencies**: Anchor 0.30+, ephemeral_rollups_sdk, solana-program
**Storage**: On-chain Solana accounts (PDAs)
**Testing**: Bankrun (local validator), Anchor test framework (TypeScript), cargo test (unit)
**Target Platform**: Solana Mainnet-beta, MagicBlock Ephemeral Rollup
**Project Type**: Single Anchor workspace with multiple programs
**Performance Goals**: <2s session delegation, <5s payment confirmation, 100% deterministic map generation
**Constraints**: Compute unit limits (~200k CU per instruction), account size limits (10 MB)
**Scale/Scope**: 10,000+ concurrent profiles, 81 campaign levels, unlimited sessions

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Requirement | Status |
|-----------|-------------|--------|
| I. Security-First | Account validation via Anchor constraints | ✅ Planned |
| I. Security-First | Checked arithmetic for payments | ✅ Planned |
| I. Security-First | PDA seeds deterministic, collision-resistant | ✅ Planned |
| I. Security-First | Signer checks explicit | ✅ Planned |
| II. Test-Driven | Tests before implementation | ✅ Required |
| II. Test-Driven | 80% minimum coverage | ✅ Required |
| II. Test-Driven | Edge case coverage | ✅ Required |
| III. Composability | Clean IDL via Anchor | ✅ Planned |
| III. Composability | Event emission for indexers | ✅ Planned |
| IV. Anchor Framework | Latest stable Anchor (0.30+) | ✅ Planned |
| IV. Anchor Framework | Custom error enums | ✅ Planned |
| V. MagicBlock | Ephemeral rollup compatibility | ✅ Via SDK |
| V. MagicBlock | Delegation authority validation | ✅ Required |
| Security Standards | cargo clippy, cargo audit | ✅ Required |
| Development Workflow | PR reviews, CI checks | ✅ Required |

**Gate Status**: ✅ PASSED - All principles addressed in design

## Project Structure

### Documentation (this feature)

```text
specs/001-solana-core-programs/
├── plan.md              # This file
├── research.md          # Phase 0 output (completed)
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (IDL definitions)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
programs/
├── player-profile/
│   └── src/
│       ├── lib.rs           # Program entry, instructions
│       ├── state.rs         # Account definitions
│       ├── errors.rs        # Custom error codes
│       └── constants.rs     # Program constants
├── session-manager/
│   └── src/
│       ├── lib.rs           # Program entry, instructions
│       ├── state.rs         # Account definitions
│       ├── errors.rs        # Custom error codes
│       └── delegation.rs    # MagicBlock integration
└── map-generator/
    └── src/
        ├── lib.rs           # Program entry, instructions
        ├── state.rs         # Account definitions
        ├── rng.rs           # Seeded RNG implementation
        └── generator.rs     # Map generation logic

tests/
├── player-profile.ts    # Integration tests
├── session-manager.ts   # Integration tests
└── map-generator.ts     # Integration tests

Anchor.toml              # Anchor workspace config
Cargo.toml               # Rust workspace config
```

**Structure Decision**: Multi-program Anchor workspace with shared tests directory. Each program is self-contained with its own state, errors, and constants modules. This follows Anchor best practices for modular program development.

## Complexity Tracking

> No constitution violations requiring justification.

| Decision | Rationale |
|----------|-----------|
| 3 separate programs | Clean separation of concerns, independent upgradability |
| Off-chain map generation | Compute unit limits prevent full on-chain generation |
| Native SOL payments | Simplest payment mechanism, SPL tokens deferred |
