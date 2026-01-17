# Quickstart: Solana Core Programs

**Feature**: 001-solana-core-programs
**Date**: 2026-01-16 (validated)

## Prerequisites

- Rust 1.75+ with `rustup` (tested with 1.91.1)
- Solana CLI 2.0+ (tested with 2.3.13)
- Anchor CLI 0.32+ (tested with 0.32.1)
- Node.js 18+ (tested with 24.11.1)
- Yarn or npm

## Project Setup

### 1. Initialize Anchor Workspace

```bash
# From repository root
anchor init solana-programs --template=single
cd solana-programs

# Or if starting fresh
anchor new player-profile
anchor new session-manager
anchor new map-generator
```

### 2. Configure Anchor.toml

```toml
[features]
seeds = false
skip-lint = false

[programs.localnet]
player_profile = "29DPbP1zuCCRg63PiShMjxAmZos97BR5TmhpijUYQzze"
session_manager = "FcMT7MzBLVQGaMATEMws3fjsL2Q77QSHmoEPdowTMxJa"
map_generator = "BYdGuEGf8NqtLnHpSRuZFrPGEgvdxMfGfTt71QVBxYHa"

[programs.devnet]
player_profile = "29DPbP1zuCCRg63PiShMjxAmZos97BR5TmhpijUYQzze"
session_manager = "FcMT7MzBLVQGaMATEMws3fjsL2Q77QSHmoEPdowTMxJa"
map_generator = "BYdGuEGf8NqtLnHpSRuZFrPGEgvdxMfGfTt71QVBxYHa"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "localnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "yarn run ts-mocha -p ./tsconfig.json -t 1000000 tests/**/*.ts"
```

### 3. Add Dependencies

```bash
# In programs/player-profile/Cargo.toml
[dependencies]
anchor-lang = "0.32.0"

# In programs/session-manager/Cargo.toml
[dependencies]
anchor-lang = "0.32.0"
# NOTE: ephemeral-rollups-sdk is temporarily disabled due to blake3 requiring
# Rust edition 2024 which is not yet supported by Solana platform tools.
# ephemeral-rollups-sdk = { version = "0.8.0", features = ["anchor"] }

# In programs/map-generator/Cargo.toml
[dependencies]
anchor-lang = "0.32.0"
```

## Build & Test

### Build All Programs

```bash
anchor build
```

### Run Tests

```bash
# All tests
anchor test

# Specific program
anchor test -- --grep "player-profile"

# With logs
RUST_LOG=solana_runtime::message=debug anchor test
```

### Deploy to Devnet

```bash
# Configure for devnet
solana config set --url devnet

# Airdrop SOL for deployment
solana airdrop 2

# Deploy
anchor deploy --program-name player_profile
anchor deploy --program-name session_manager
anchor deploy --program-name map_generator
```

## Quick Verification Steps

### 1. Player Profile

```typescript
import * as anchor from "@coral-xyz/anchor";
import { PlayerProfile } from "../target/types/player_profile";

// Initialize profile
const [profilePda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("player"), wallet.publicKey.toBuffer()],
  program.programId,
);

await program.methods
  .initializeProfile("MyPlayerName")
  .accounts({
    playerProfile: profilePda,
    owner: wallet.publicKey,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();

// Fetch profile
const profile = await program.account.playerProfile.fetch(profilePda);
console.log("Profile:", profile);
```

### 2. Session Manager

```typescript
import { SessionManager } from "../target/types/session_manager";

// First initialize counter (one-time admin operation)
const [counterPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("session_counter")],
  sessionProgram.programId,
);

await sessionProgram.methods
  .initializeCounter()
  .accounts({
    sessionCounter: counterPda,
    admin: wallet.publicKey,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();

// Start session
const [sessionPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("session"), wallet.publicKey.toBuffer()],
  sessionProgram.programId,
);

await sessionProgram.methods
  .startSession(0) // Campaign level 0
  .accounts({
    gameSession: sessionPda,
    sessionCounter: counterPda,
    player: wallet.publicKey,
    systemProgram: anchor.web3.SystemProgram.programId,
  })
  .rpc();
```

### 3. Map Generator

```typescript
import { MapGenerator } from "../target/types/map_generator";

// Get seed for level
const seed = await mapProgram.methods
  .getMapSeed(0)
  .accounts({
    mapConfig: configPda,
  })
  .view();

console.log("Seed for level 0:", seed.toString());
```

## Directory Structure After Setup

```
solana-programs/
├── Anchor.toml
├── Cargo.toml
├── programs/
│   ├── player-profile/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs
│   │       ├── errors.rs
│   │       └── constants.rs
│   ├── session-manager/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── state.rs
│   │       ├── errors.rs
│   │       └── constants.rs
│   └── map-generator/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── state.rs
│           ├── errors.rs
│           ├── constants.rs
│           └── rng.rs
├── tests/
│   ├── player-profile.ts
│   ├── session-manager.ts
│   └── map-generator.ts
├── target/
│   ├── idl/
│   │   ├── player_profile.json
│   │   ├── session_manager.json
│   │   └── map_generator.json
│   └── types/
│       ├── player_profile.ts
│       ├── session_manager.ts
│       └── map_generator.ts
└── migrations/
    └── deploy.ts
```

## Environment Variables

```bash
# .env (for tests)
ANCHOR_PROVIDER_URL=http://127.0.0.1:8899
ANCHOR_WALLET=~/.config/solana/id.json

# For MagicBlock (devnet)
MAGICBLOCK_RPC_URL=https://devnet.magicblock.app
```

## Common Commands

```bash
# Generate new keypair for program
solana-keygen new -o target/deploy/player_profile-keypair.json

# Check program logs
solana logs -u devnet

# Get program account data
solana account <PROGRAM_ID> -u devnet

# Upgrade program
anchor upgrade target/deploy/player_profile.so --program-id <PROGRAM_ID>
```

## Troubleshooting

### "Account not found" Error

- Ensure the PDA seeds match exactly between client and program
- Check that prerequisite accounts (e.g., profile before session) exist

### "Insufficient funds" Error

- Run `solana airdrop 2` on devnet
- Check rent exemption requirements

### Build Errors

- Run `anchor clean` then `anchor build`
- Update Anchor: `avm install 0.32.1 && avm use 0.32.1`

### MagicBlock Delegation Fails

- MagicBlock SDK is currently disabled due to Rust edition 2024 requirement
- Delegation instructions use stub implementations until SDK tooling is updated
- Ensure program is marked with `#[ephemeral]` attribute when SDK is enabled
- Verify MagicBlock accounts are passed correctly
