# Dungeons & Moles Solana Programs

Monorepo for the on-chain gameplay stack used by Dungeons & Moles.

## Workspace Overview

### On-chain programs (`programs/`)

- `player-profile`: player identity, progression, and run metadata
- `session-manager`: gameplay session lifecycle and authority flow
- `map-generator`: deterministic map seed/config and map state
- `gameplay-state`: campaign state, movement/combat resolution, and PvP mode settlement (gauntlet/duel/pit-draft)
- `player-inventory`: inventory, item effects, offers, and fusion logic
- `poi-system`: point-of-interest spawn and interaction flow

### Shared gameplay crates (`crates/`)

- `combat-system`: combat engine and effect/trigger resolution
- `field-enemies`: enemy archetypes, scaling, and spawn support
- `boss-system`: boss definitions, traits, phases, and scaling

## Repository Layout

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

## Tooling

- Rust `1.75+` (edition `2021`, Solana BPF target)
- Anchor `0.32.0`
- Solana CLI `2.3+`
- Node.js `>=18` (TypeScript tests)

## Build, Test, Lint

```bash
anchor build
anchor test
cargo test
cargo clippy
```

## Test Suite

TypeScript integration tests live in `tests/`:

- `tests/session-manager/session-manager.ts`
- `tests/map-generator/map-generator.ts`
- `tests/gameplay-state/gameplay-state.ts`
- `tests/player-inventory/inventory_management.ts`
- `tests/player-profile/player-profile.ts`
- `tests/poi-system/poi-system.ts`

## Implemented Economy Constants

- Profile starts with `20` campaign runs.
- Run top-up is `20` runs for `0.005 SOL`, split `50% treasury / 50% gauntlet pool`.
- Gauntlet entry is `0.01 SOL` (`3% company / 97% gauntlet pool`).
- Duel entry is currently fixed at `0.1 SOL` (`3% company / 2% gauntlet pool / 95% winner`).
- Pit Draft entry is currently fixed at `0.1 SOL` (`3% company / 2% gauntlet pool / 95% winner`).

## Operational Notes

- MagicBlock integration is currently stubbed while SDK/toolchain compatibility is finalized.
- Some gameplay systems are split across program + crate boundaries to keep CPI surfaces focused and reusable.
