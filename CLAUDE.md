# solana-programs Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-15

## Active Technologies
- Rust 2021 edition, Solana SDK 2.x + Anchor 0.31.1 (matching existing programs) (002-gameplay-state-tracking)
- Solana accounts (PDA-based state) (002-gameplay-state-tracking)
- Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 (matching existing programs), solana-program (003-combat-system)
- Solana accounts (PDA-based state) + on-chain item registry (004-item-system)
- Rust 1.75+ (Solana BPF target) + Anchor 0.32+, anchor-lang, solana-program (005-field-enemies)
- On-chain Solana accounts via Anchor PDAs (005-field-enemies)

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
- 005-field-enemies: Added Rust 1.75+ (Solana BPF target) + Anchor 0.32+, anchor-lang, solana-program
- 004-item-system: Added Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 (matching existing programs), solana-program
- 003-combat-system: Added Rust 2021 edition (Solana BPF target) + Anchor 0.31.1 (matching existing programs), solana-program


<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
