# solana-programs Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-01-15

## Active Technologies
- Rust 2021 edition, Solana SDK 2.x + Anchor 0.31.1 (matching existing programs) (002-gameplay-state-tracking)
- Solana accounts (PDA-based state) (002-gameplay-state-tracking)

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
- 002-gameplay-state-tracking: Added Rust 2021 edition, Solana SDK 2.x + Anchor 0.31.1 (matching existing programs)

- 001-solana-core-programs: Implemented session manager + map generator, added tests

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
