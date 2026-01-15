# Quickstart: Solana Core Programs

**Feature**: 001-solana-core-programs
**Date**: 2025-01-15

## Prerequisites

- Rust 1.75+ with `rustup`
- Solana CLI 1.18+
- Anchor CLI 0.30+
- Node.js 18+
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
player_profile = "Prof1LeXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
session_manager = "Sess1onXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"
map_generator = "MapGenXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX"

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
anchor-lang = "0.30.0"

# In programs/session-manager/Cargo.toml
[dependencies]
anchor-lang = "0.30.0"
ephemeral_rollups_sdk = "0.1.0"

# In programs/map-generator/Cargo.toml
[dependencies]
anchor-lang = "0.30.0"
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
  program.programId
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

// Start session
const [sessionPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("session"), wallet.publicKey.toBuffer()],
  sessionProgram.programId
);

await sessionProgram.methods
  .startSession(0) // Campaign level 0
  .accounts({
    gameSession: sessionPda,
    sessionCounter: counterPda,
    playerProfile: profilePda,
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
│   │       └── delegation.rs
│   └── map-generator/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── state.rs
│           ├── rng.rs
│           └── generator.rs
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
- Update Anchor: `avm install 0.30.0 && avm use 0.30.0`

### MagicBlock Delegation Fails
- Ensure program is marked with `#[ephemeral]` attribute
- Verify MagicBlock accounts are passed correctly
