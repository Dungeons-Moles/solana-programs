# solana-programs Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-15

## Active Technologies
- Rust 2021 edition, Solana SDK 2.x + Anchor 0.31.1 (matching existing programs) (002-gameplay-state-tracking)
- Solana accounts (PDA-based state) (002-gameplay-state-tracking)
- Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 (matching existing programs), solana-program (003-combat-system)
- Solana accounts (PDA-based state) + on-chain item registry (004-item-system)
- Rust 1.75+ (Solana BPF target) + Anchor 0.32+, anchor-lang, solana-program (005-field-enemies)
- On-chain Solana accounts via Anchor PDAs (005-field-enemies)
- Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 + solana-program, anchor-lang, combat-system crate (for ItemEffect/TriggerType reuse) (006-boss-system)
- Compile-time constants for boss definitions; BossInstance computed at runtime (006-boss-system)
- Rust 2021 edition (Solana BPF target) + Anchor 0.32.0, solana-program (007-poi-system)
- PDA-based state for per-session POI instances (MapPois account) (007-poi-system)
- Rust 2021 edition (Solana BPF target) + Anchor 0.32+, solana-program, combat-system crate (for ItemTag reuse), player-inventory crate (for ItemDefinition access), boss-system crate (for BossDefinition/weakness tags) (008-item-offer-generation)
- PDA-based state via Anchor (`MapPois` extended with offer state, or new `ItemOffers` account) (008-item-offer-generation)

- Rust 1.75+ (Solana BPF target) + Anchor 0.32+, Solana CLI 2.3+, solana-program (001-solana-core-programs)
- MagicBlock integration is stubbed (SDK pending toolchain update)

## Project Structure

```text
src/
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
- 008-item-offer-generation: Added Rust 2021 edition (Solana BPF target) + Anchor 0.32+, solana-program, combat-system crate (for ItemTag reuse), player-inventory crate (for ItemDefinition access), boss-system crate (for BossDefinition/weakness tags)
- 007-poi-system: Added Rust 2021 edition (Solana BPF target) + Anchor 0.32.0, solana-program
- 006-boss-system: Added Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 + solana-program, anchor-lang, combat-system crate (for ItemEffect/TriggerType reuse)


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
