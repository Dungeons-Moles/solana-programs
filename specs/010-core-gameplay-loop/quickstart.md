# Quickstart: Core Gameplay Loop

**Feature**: 010-core-gameplay-loop  
**Date**: 2026-01-21

## Overview

This guide covers the main gameplay flows for Dungeons & Moles using the Solana programs.

## Prerequisites

```bash
# Install Anchor CLI
cargo install --git https://github.com/coral-xyz/anchor anchor-cli

# Build all programs
anchor build

# Start local validator
solana-test-validator
```

## 1. Create Player Profile

```typescript
import { Program } from "@coral-xyz/anchor";
import { PlayerProfile } from "../target/types/player_profile";

const program = anchor.workspace.PlayerProfile as Program<PlayerProfile>;

// Create profile with 20 runs and 40 starter items
const tx = await program.methods
  .initializeProfile("PlayerName")
  .accounts({
    playerProfile: profilePda,
    owner: wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();

// Profile now has:
// - available_runs: 20
// - highest_level_unlocked: 1
// - unlocked_items: 40 bits set (starter items)
// - active_item_pool: 40 bits set (all starter items active)
```

## 2. Purchase Additional Runs

```typescript
const treasury = new PublicKey("TREASURY_PUBKEY");

const tx = await program.methods
  .purchaseRuns()
  .accounts({
    playerProfile: profilePda,
    owner: wallet.publicKey,
    treasury: treasury,
    systemProgram: SystemProgram.programId,
  })
  .rpc();

// Transfers 0.001 SOL, adds 20 runs
```

## 3. Start a Session (Atomic Bundle)

```typescript
import { Transaction } from "@solana/web3.js";

// Generate burner wallet
const burnerWallet = Keypair.generate();

// Session PDA: ["session", player, level]
const [sessionPda] = PublicKey.findProgramAddressSync(
  [Buffer.from("session"), wallet.publicKey.toBuffer(), Buffer.from([level])],
  sessionProgram.programId,
);

// Build atomic transaction bundle
const tx = new Transaction();

// 1. Start session + transfer SOL to burner
tx.add(
  await sessionProgram.methods
    .startSession(level, new BN(50_000_000)) // 0.05 SOL for gameplay fees
    .accounts({
      gameSession: sessionPda,
      sessionCounter: counterPda,
      playerProfile: profilePda,
      player: wallet.publicKey,
      burnerWallet: burnerWallet.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
);

// 2. Initialize game state
tx.add(
  await gameplayProgram.methods
    .initializeGameState(9, 9, 4, 4) // 9x9 map, spawn at center
    .accounts({
      gameState: gameStatePda,
      gameSession: sessionPda,
      player: wallet.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
);

// 3. Spawn enemies
tx.add(
  await enemiesProgram.methods
    .spawnEnemies(level, seed)
    .accounts({
      mapEnemies: enemiesPda,
      gameSession: sessionPda,
      player: wallet.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
);

// 4. Spawn POIs
tx.add(
  await poisProgram.methods
    .spawnPois(level, seed)
    .accounts({
      mapPois: poisPda,
      gameSession: sessionPda,
      player: wallet.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
);

// 5. Initialize inventory with Basic Pickaxe
tx.add(
  await inventoryProgram.methods
    .initializeInventory()
    .accounts({
      inventory: inventoryPda,
      player: wallet.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .instruction(),
);

// Sign with main wallet (session creation)
await sendAndConfirmTransaction(connection, tx, [wallet]);
```

## 4. Movement with Combat (Using Burner Wallet)

```typescript
// All gameplay actions signed by burner wallet
const tx = await gameplayProgram.methods
  .moveWithCombat(targetX, targetY, isWall)
  .accounts({
    gameState: gameStatePda,
    mapEnemies: enemiesPda,
    mapPois: poisPda,
    playerInventory: inventoryPda,
    gameSession: sessionPda,
    playerProfile: profilePda,
    player: burnerWallet.publicKey, // Burner signs
  })
  .signers([burnerWallet])
  .rpc();

// Events emitted:
// - PlayerMoved (always)
// - EnemyMoved (during night, for each enemy that moved)
// - CombatStarted (if enemy at target)
// - TurnExecuted (for each combat turn)
// - CombatEnded (after combat)
// - PlayerDefeated (if HP <= 0)
// - LevelCompleted (if Week 3 boss defeated)
```

## 5. POI Interaction

```typescript
// Player must be standing on POI tile
const tx = await poiProgram.methods
  .interactPoi(poiIndex)
  .accounts({
    mapPois: poisPda,
    gameState: gameStatePda,
    gameSession: sessionPda,
    playerInventory: inventoryPda,
    player: burnerWallet.publicKey,
  })
  .signers([burnerWallet])
  .rpc();
```

## 6. Handle Events

```typescript
// Subscribe to program events
program.addEventListener("PlayerMoved", (event) => {
  console.log(`Moved to (${event.toX}, ${event.toY})`);
  if (event.combatTriggered) {
    console.log("Combat started!");
  }
  if (event.enemiesMoved > 0) {
    console.log(`${event.enemiesMoved} enemies moved during night`);
  }
});

program.addEventListener("CombatEnded", (event) => {
  if (event.playerWon) {
    console.log(`Victory! Earned ${event.goldEarned} gold`);
  } else {
    console.log("Defeated...");
  }
});

program.addEventListener("ItemUnlocked", (event) => {
  console.log(`New item unlocked: index ${event.itemIndex}`);
});
```

## 7. Multi-Session Support

```typescript
// Player can have sessions on multiple levels
const session1 = await startSession(1); // Level 1
const session2 = await startSession(3); // Level 3
const session3 = await startSession(5); // Level 5

// Each session is independent
await moveWithCombat(session1, ...); // Affects only level 1 session
await moveWithCombat(session3, ...); // Affects only level 3 session
```

## Session Flow Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                     ATOMIC SESSION CREATION                      │
│  1. start_session (validates runs, level, transfers SOL)        │
│  2. initialize_game_state (spawns player with Basic Pickaxe)    │
│  3. spawn_enemies (deterministic from seed)                      │
│  4. spawn_pois (deterministic from seed)                         │
│  5. initialize_inventory (Basic Pickaxe equipped)                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      GAMEPLAY LOOP                               │
│  • move_with_combat (signed by burner wallet)                   │
│    - Night: enemies within 3 tiles move toward player           │
│    - Combat auto-triggers on enemy tile                         │
│    - Boss triggers on final move of week                        │
│  • interact_poi (must be on POI tile)                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      SESSION END                                 │
│  DEATH:                                                          │
│    - Player HP <= 0                                              │
│    - Session closes, run deducted (atomic)                       │
│  VICTORY (Week 3 Boss):                                          │
│    - Session closes, run deducted                                │
│    - If first-time: level unlocks, random item unlocks           │
└─────────────────────────────────────────────────────────────────┘
```

## Testing

```bash
# Run unit tests
cargo test -p player-profile
cargo test -p session-manager
cargo test -p gameplay-state

# Run integration tests
anchor test
```
