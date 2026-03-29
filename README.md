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
- Anchor `0.32.x`
- Solana CLI `2.3+`
- Node.js `>=18` (TypeScript tests)
- Surfpool
- MagicBlock Ephemeral Validator CLI

## Build, Test, Lint

```bash
anchor build
anchor test
cargo test
cargo clippy
```

## Local Testing

There are two options for running the local validator stack. Both require the Ephemeral Validator CLI and VRF oracle.

### Prerequisites

```bash
# Ephemeral Validator CLI
npm install -g @magicblock-labs/ephemeral-validator@latest

# Surfpool (only needed for Option B)
curl -sL https://run.surfpool.run/ | bash
```

### Option A: mb-test-validator with --bpf-program (recommended)

Loads programs at genesis — no deploy step needed. Faster and more reliable than `anchor deploy`.

> **Note:** This method does not load Metaplex programs, so NFT marketplace features (minting, listing, buying skins/items) cannot be tested. Use Option B if you need Metaplex/NFT flows.

1. Build the programs:

```bash
anchor build
```

2. Start the base validator with all programs pre-loaded:

```bash
mb-test-validator --reset --ledger .mb-ledger --rpc-port 8899 --faucet-port 9902 \
  --bpf-program 3rzGGgHRRnMATmYJkjidPMapEMesvA16PTs5HhfAep4V target/deploy/gameplay_state.so \
  --bpf-program E6kc5Edg1s3AXVQQFRoYdAq4vPAFbkYbP7B5ujiuZwz4 target/deploy/map_generator.so \
  --bpf-program 7rTRqR6H8ztxpcPVKtAwXGi7PQFDYLgMkWSBRLPcYMH2 target/deploy/poi_system.so \
  --bpf-program CrU4bUFreKy2XsoU2oksdJWKim11w2VpagKBQ2MTkyMz target/deploy/session_manager.so \
  --bpf-program GrXaTaf7wZ74mTaWQ9QSUPAKG6M3Sf4xaZjNytTLa8yC target/deploy/player_inventory.so \
  --bpf-program GSLNDrNoHeZXVxB7Yu7tUe8417PpZ5XV7JPYupPw9WQy target/deploy/player_profile.so
```

3. Initialize the on-chain program state:

```bash
anchor run init
```

### Option B: Surfpool with anchor deploy

Surfpool stays local while using Solana Devnet as the upstream base layer. Supports all features including Metaplex/NFT marketplace.

1. Build the programs:

```bash
anchor build
```

2. Start Surfpool (programs are deployed on devnet via runbooks):

```bash
surfpool start --rpc-url https://api.devnet.solana.com
```

### Start the Ephemeral Rollup validator and VRF oracle

Both options require the ER validator and VRF oracle running in separate terminals:

```bash
ephemeral-validator --remotes "http://localhost:8899" --remotes "ws://localhost:8900" -l "7799" --lifecycle replica
```

```bash
VRF_ORACLE_SKIP_PREFLIGHT="true" RPC_URL="http://localhost:8899" WEBSOCKET_URL="ws://localhost:8999" RUST_LOG=info vrf-oracle
```

At this point the local stack is ready for gameplay and session lifecycle testing. NFT flows are only available with Option B.

## Minting NFTs

This repository includes a CLI helper at [scripts/mint-nft.ts](/home/ailton/Work/dungeons-and-moles/solana-programs/scripts/mint-nft.ts). It mints through the `nft-marketplace` program, which performs the Metaplex Core CPI on-chain.

### Mint a skin NFT

Mint to your current wallet:

```bash
MINT_TYPE=skin MINT_NAME="Your Skin Name" anchor run mint-nft
```

Mint to a specific wallet:

```bash
MINT_TYPE=skin MINT_NAME="Your Skin Name" OWNER=<wallet_pubkey> anchor run mint-nft
```

Mint with a custom metadata URI:

```bash
MINT_TYPE=skin MINT_NAME="Your Skin Name" MINT_URI="https://your-metadata-uri.json" OWNER=<wallet_pubkey> anchor run mint-nft
```

### Mint an NFT item

```bash
MINT_TYPE=item MINT_NAME="Your Item Name" NFT_ITEM_ID="S-XX-01" anchor run mint-nft
```

Mint an NFT item to a specific wallet:

```bash
MINT_TYPE=item MINT_NAME="Your Item Name" NFT_ITEM_ID="S-XX-01" OWNER=<wallet_pubkey> anchor run mint-nft
```

### Minting notes

- `MINT_TYPE` must be `skin` or `item`.
- `MINT_NAME` is required.
- `OWNER` is optional. If omitted, the NFT is minted to the wallet in `ANCHOR_WALLET` or `~/.config/solana/id.json`.
- `MINT_URI` is optional. If omitted, the script uses `https://arweave.net/placeholder`.
- `NFT_ITEM_ID` is required for item mints.
- The signer wallet must be the marketplace authority configured by `anchor run init-collections`.
