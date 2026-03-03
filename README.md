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
- `nft-marketplace`: Metaplex Core NFT minting (skins & items), listing, buying, and quest system

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
  nft-marketplace/
  player-inventory/
  player-profile/
  poi-system/
  session-manager/
tests/
```

## Tooling

- Rust `1.75+` (edition `2021`, Solana BPF target)
- Anchor `0.32.1`
- Solana CLI `2.3+`
- Node.js `>=18` (TypeScript tests)

## Build, Test, Lint

```bash
anchor build
anchor test
cargo test
cargo clippy
```

## Local Testing

There are two local validator setups depending on what you need to test. You cannot run both simultaneously yet.

### Option A: MagicBlock Ephemeral Rollups (gameplay + session lifecycle)

1. Start the MagicBlock local validator:

```bash
mb-test-validator --reset --ledger .mb-ledger --rpc-port 8899 --faucet-port 9901
```

2. Start the ephemeral validator:

```bash
ephemeral-validator --remotes http://127.0.0.1:8899 --remotes ws://127.0.0.1:8900 --listen 127.0.0.1:7799 --storage /tmp/mb-er-storage --reset
```

3. Build, deploy, and initialize:

```bash
anchor build
anchor deploy
anchor run init
```

### Option B: Surfpool (NFT marketplace + Metaplex integrations)

Surfpool clones devnet state locally, which is required for Metaplex program availability.

1. Start Surfpool pointing at devnet:

```bash
surfpool start --rpc-url https://api.devnet.solana.com
```

2. Build, deploy, and initialize collections:

```bash
anchor build
anchor deploy
anchor run init-collections
```

> **Note:** Surfpool mode does not include the MagicBlock ephemeral validator, so delegation/undelegation flows cannot be tested in this setup.
