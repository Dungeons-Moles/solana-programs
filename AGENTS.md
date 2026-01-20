# solana-programs Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-20

## Active Technologies

- Rust 1.75+ (Solana BPF target) + Anchor 0.32+, Solana CLI 2.3+, solana-program (001-solana-core-programs)
- Field Enemies (005-field-enemies): Static lookup tables for stats/traits, on-chain RNG for spawns
- Boss System (006-boss-system): 24 boss definitions, stat scaling, phase mechanics, weakness tags
- MagicBlock integration is stubbed (SDK pending toolchain update)

## Project Structure

```text
programs/
  session-manager/
  map-generator/
  combat-system/
  field-enemies/
  boss-system/
tests/
  ...
```

## Commands

anchor build
anchor test
cargo test
cargo clippy

## Code Style

Rust 1.75+ (Solana BPF target): Follow standard conventions

## Recent Changes

- 006-boss-system: Implemented 24 boss definitions with traits, scaling, selection, and combat integration
- 005-field-enemies: Implemented enemy spawning, combat reward logic, and static data for 12 archetypes
- 001-solana-core-programs: Implemented session manager + map generator, added tests

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
