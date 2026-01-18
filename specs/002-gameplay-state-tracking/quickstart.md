# Quickstart: Gameplay State Tracking

**Branch**: `002-gameplay-state-tracking` | **Date**: 2025-01-17

## Prerequisites

- Rust 1.75+ with Solana toolchain
- Anchor CLI 0.31.1
- Node.js 18+ (for tests)
- Existing 001-solana-core-programs deployed (session-manager)

## Setup

```bash
# Clone and checkout branch
cd solana-programs
git checkout 002-gameplay-state-tracking

# Add new program to workspace (Cargo.toml)
# Already done if following tasks.md

# Build all programs
anchor build

# Run tests
anchor test
```

## Program Structure

```
programs/gameplay-state/
├── Cargo.toml
└── src/
    ├── lib.rs           # Instructions: initialize, move, modify_stat, close
    ├── state.rs         # GameState account, Phase enum, StatType enum
    ├── errors.rs        # GameplayStateError enum
    └── constants.rs     # DAY_MOVES=50, NIGHT_MOVES=30, DEFAULT_* stats
```

## Key Usage Flows

### 1. Initialize Game State (after starting a session)

```typescript
// Prerequisites: session already started via session-manager
const [gameStatePda] = PublicKey.findProgramAddressSync(
  [Buffer.from("game_state"), sessionPda.toBuffer()],
  GAMEPLAY_STATE_PROGRAM_ID
);

await program.methods
  .initializeGameState(
    mapWidth,    // u8
    mapHeight,   // u8
    startX,      // u8
    startY       // u8
  )
  .accounts({
    gameState: gameStatePda,
    gameSession: sessionPda,
    player: wallet.publicKey,
    systemProgram: SystemProgram.programId,
  })
  .rpc();
```

### 2. Move Player

```typescript
await program.methods
  .movePlayer(
    targetX,   // u8
    targetY,   // u8
    isWall     // bool - true if digging through wall
  )
  .accounts({
    gameState: gameStatePda,
    player: wallet.publicKey,
  })
  .rpc();
```

### 3. Modify Stat

```typescript
// StatType: { hp: {}, maxHp: {}, atk: {}, arm: {}, spd: {}, dig: {} }
await program.methods
  .modifyStat(
    { hp: {} },  // stat type
    -3           // delta (i8)
  )
  .accounts({
    gameState: gameStatePda,
    player: wallet.publicKey,
  })
  .rpc();
```

### 4. Close Game State (after session ends)

```typescript
await program.methods
  .closeGameState()
  .accounts({
    gameState: gameStatePda,
    player: wallet.publicKey,
  })
  .rpc();
```

## Testing

### Run All Tests

```bash
anchor test
```

### Test Categories

1. **Movement Tests** (US1)
   - Floor movement deducts 1 move
   - Wall dig deducts max(2, 6-DIG) moves
   - Out of bounds rejected
   - Insufficient moves rejected

2. **Phase Tests** (US2)
   - Day phase = 50 moves
   - Night phase = 30 moves
   - Auto-advance on moves_remaining = 0
   - Week transition with gear slot increase

3. **Stats Tests** (US3)
   - Default values: HP=10, ATK=1, ARM=0, SPD=0, DIG=1
   - HP cannot go below 0
   - Other stats allow negative
   - Overflow prevention

4. **Gear Slots Tests** (US4)
   - Starts at 4
   - +2 after Week 1 (total: 6)
   - +2 after Week 2 (total: 8)
   - Capped at 8

## Constants Reference

| Constant | Value | Notes |
|----------|-------|-------|
| DAY_MOVES | 50 | Moves allowed per day phase |
| NIGHT_MOVES | 30 | Moves allowed per night phase |
| DEFAULT_HP | 10 | Starting HP |
| DEFAULT_MAX_HP | 10 | Starting max HP |
| DEFAULT_ATK | 1 | Starting attack |
| DEFAULT_ARM | 0 | Starting armor |
| DEFAULT_SPD | 0 | Starting speed |
| DEFAULT_DIG | 1 | Starting dig |
| INITIAL_GEAR_SLOTS | 4 | Starting gear slots |
| MAX_GEAR_SLOTS | 8 | Maximum gear slots |
| BASE_DIG_COST | 6 | Base cost for digging (6 - DIG stat) |
| MIN_DIG_COST | 2 | Minimum dig cost |

## Verification Checklist

- [ ] Build succeeds: `anchor build`
- [ ] Tests pass: `anchor test`
- [ ] GameState size < 200 bytes
- [ ] All movement scenarios from spec work
- [ ] Phase transitions work correctly
- [ ] Gear slots increase at week boundaries
- [ ] Boss fight ready triggers at Week 3 Night 3

## Troubleshooting

**Error: SessionNotActive**
- Ensure GameSession exists before initializing GameState
- Session must be created via session-manager first

**Error: OutOfBounds**
- Check target coordinates are within map_width/map_height
- Coordinates are 0-indexed

**Error: InsufficientMoves**
- Check moves_remaining before moving
- Wall tiles cost max(2, 6-DIG) moves

**Error: NotAdjacent**
- Target must be exactly 1 step away (Manhattan distance = 1)
- Diagonal movement not supported
