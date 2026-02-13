# solana-programs Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-02-13

## Active Technologies

- Rust 2021 edition (Solana BPF target) + Anchor 0.32.0 across all programs/crates
- Solana SDK/CLI 2.3+ and `solana-program` account/PDA patterns
- TypeScript integration tests via `ts-mocha` + `@coral-xyz/anchor` 0.32.0
- Shared gameplay crates: `combat-system`, `field-enemies`, `boss-system`
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
yarn test

## Code Style

Rust 1.75+ (Solana BPF target): Follow standard conventions

## Recent Changes

- Core gameplay loop integrated across session, map, gameplay-state, POI, inventory, and combat systems
- PvP mode plumbing is active in `gameplay-state` (`Duel`, `Gauntlet`, `Pit Draft`) with on-chain settlement vaults
- Profile run economy is active in `player-profile` (20 initial runs, paid top-ups, treasury/pool split)
- Item registry and offer generation use an 80-item core set plus starter Basic Pickaxe handling
- Enemy and boss systems are data-driven via `field-enemies` and `boss-system` crates

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
