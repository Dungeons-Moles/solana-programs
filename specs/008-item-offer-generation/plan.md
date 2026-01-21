# Implementation Plan: Item Offer Generation

**Branch**: `008-item-offer-generation` | **Date**: 2026-01-20 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/008-item-offer-generation/spec.md`

## Summary

Implement a deterministic item offer generation system that creates weighted item selections for POI interactions. The system generates offers based on POI type (Supply Cache, Tool Crate, Geode Vault, Counter Cache, Smuggler Hatch), applies act-based rarity tables, and weights tag selection toward current boss weakness tags (1.4x for weaknesses vs 1.0x baseline). Uses seeded random generation for on-chain verifiability.

## Technical Context

**Language/Version**: Rust 2021 edition (Solana BPF target)
**Primary Dependencies**: Anchor 0.32+, solana-program, combat-system crate (for ItemTag reuse), player-inventory crate (for ItemDefinition access), boss-system crate (for BossDefinition/weakness tags)
**Storage**: PDA-based state via Anchor (`MapPois` extended with offer state, or new `ItemOffers` account)
**Testing**: cargo test (unit), anchor test with Bankrun (integration)
**Target Platform**: Solana mainnet/devnet (BPF target)
**Project Type**: Single Anchor program extending poi-system
**Performance Goals**: Offer generation completes within single transaction compute budget (~200k CU)
**Constraints**: Deterministic results for same seed, on-chain verifiable, no off-chain dependencies
**Scale/Scope**: 80 items (16 Tools, 64 Gear), 8 tags, 4 acts, 3 weeks, 5 item-granting POI types

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Pre-Design Check (Phase 0)

| Principle | Status | Notes |
|-----------|--------|-------|
| **I. Security-First** | PASS | Deterministic RNG with session seed prevents manipulation; all account validation via Anchor constraints |
| **II. Test-Driven Development** | PASS | Unit tests for rarity sampling, tag weighting; integration tests for POI interaction flows |
| **III. Program Composability** | PASS | Extends poi-system with clean instruction interface; reuses ItemTag from combat-system |
| **IV. Anchor Framework** | PASS | Using Anchor 0.32+; constraint-based validation; custom error enums |
| **V. MagicBlock Ephemeral Rollups** | PASS | Offer state is session-scoped, compatible with delegation pattern |
| **Security & Verification** | PASS | No unsafe Rust; checked arithmetic for weight normalization |
| **Development Workflow** | PASS | PR-based; CI with build/test/lint/audit |

### Post-Design Check (Phase 1)

| Principle | Status | Verification |
|-----------|--------|--------------|
| **I. Security-First** | PASS | XorShift64 RNG seeded from session prevents manipulation; Anchor constraints validate all accounts; basis-point arithmetic avoids overflow |
| **II. Test-Driven Development** | PASS | Test plan includes: unit tests (rarity_tables, tag_weights, xorshift64), integration tests (offer_generation, shop_reroll), statistical validation (distribution tests) |
| **III. Program Composability** | PASS | New instructions documented in IDL; reuses ItemTag, BossDefinition, ItemDefinition from existing crates; events emitted for indexer compatibility |
| **IV. Anchor Framework** | PASS | All new structs use AnchorSerialize/Deserialize; error codes defined with descriptive messages; PDA seeds follow existing patterns |
| **V. MagicBlock Ephemeral Rollups** | PASS | MapPois account extension is session-scoped; no cross-session state; offer/shop state clears on session end |
| **Security & Verification** | PASS | Fixed-point arithmetic (basis points); no floating point; checked math for weight totals; item pool indices validated at compile time |
| **Development Workflow** | PASS | IDL extensions documented; quickstart guide provided; test files planned |

**Post-Design Verdict**: All constitution principles satisfied. No violations requiring justification.

## Project Structure

### Documentation (this feature)

```text
specs/008-item-offer-generation/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (Anchor IDL extensions)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
programs/
├── poi-system/
│   └── src/
│       ├── lib.rs                 # Extended with generate_offer instruction
│       ├── offer_generation.rs    # NEW: Core offer generation logic
│       ├── rarity_tables.rs       # NEW: Act-based rarity distributions
│       ├── tag_weights.rs         # NEW: Tag weighting with boss weakness
│       └── state.rs               # Extended with ItemOffer, ShopState
├── player-inventory/
│   └── src/
│       └── items.rs               # Read-only: ItemDefinition registry
├── boss-system/
│   └── src/
│       └── bosses.rs              # Read-only: BossDefinition with weakness_tags
└── combat-system/
    └── src/
        └── state.rs               # Read-only: ItemTag enum

tests/
├── poi-system/
│   ├── offer_generation.ts        # NEW: Offer generation tests
│   ├── rarity_distribution.ts     # NEW: Statistical rarity validation
│   ├── tag_weighting.ts           # NEW: Boss weakness weighting tests
│   └── shop_reroll.ts             # NEW: Smuggler Hatch reroll tests
```

**Structure Decision**: Extending existing poi-system program rather than creating a new program. Offer generation is tightly coupled to POI interactions and shares session state.

## Complexity Tracking

> No constitution violations to justify. Design follows established patterns.
