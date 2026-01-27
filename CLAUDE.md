# solana-programs Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-26

## Active Technologies

- Rust 1.75+ (Solana BPF target) + Anchor 0.32+, Solana CLI 2.3+, solana-program (001-solana-core-programs)
- Rust 2021 edition, Solana SDK 2.x + Anchor 0.31.1 (matching existing programs) (002-gameplay-state-tracking)
- Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 (matching existing programs), solana-program (003-combat-system)
- Solana accounts (PDA-based state) + on-chain item registry (004-item-system)
- Rust 1.75+ (Solana BPF target) + Anchor 0.32+, anchor-lang, solana-program (005-field-enemies)
- Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 + solana-program, anchor-lang, combat-system crate (006-boss-system)
- Rust 2021 edition (Solana BPF target) + Anchor 0.32.0, solana-program (007-poi-system)
- Rust 2021 edition (Solana BPF target) + Anchor 0.32+, solana-program, combat-system crate, player-inventory crate (008-item-offer-generation)
- Rust 1.75+ (Solana BPF target) + Anchor 0.32+, solana-program, combat-system, field-enemies, boss-system, poi-system (010-core-gameplay-loop)
- MagicBlock integration is stubbed (SDK pending toolchain update)

## Project Structure

```text
crates/
  boss-system/
  combat-system/
  field-enemies/
programs/
  gameplay-state/
  map-generator/
  player-inventory/
  player-profile/
  poi-system/
  session-manager/
tests/
```

## Commands

anchor build
anchor test
cargo test
cargo clippy

## Code Style

Rust 1.75+ (Solana BPF target): Follow standard conventions

## Recent Changes

- 010-core-gameplay-loop: Integrated core gameplay loop with combat, enemies, bosses, and POI systems
- 008-item-offer-generation: Implemented item offer generation logic and state
- 007-poi-system: Implemented POI system with PDA-based state for per-session instances
- 006-boss-system: Implemented 24 boss definitions, stat scaling, phase mechanics
- 005-field-enemies: Implemented enemy spawning and reward logic (in crates/field-enemies)
- 001-solana-core-programs: Implemented session manager and map generator

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
