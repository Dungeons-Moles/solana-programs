# Data Model: Solana Core Programs

**Feature**: 001-solana-core-programs
**Date**: 2025-01-15

## Overview

This document defines the on-chain account structures for the three Solana programs: Player Profile, Session Manager, and Map Generator.

---

## Player Profile Program

### PlayerProfile Account

**PDA Seeds**: `[b"player", owner.key()]`

| Field | Type | Size | Description |
|-------|------|------|-------------|
| owner | Pubkey | 32 | Wallet address that owns this profile |
| name | String (max 32) | 36 | Display name (4-byte length prefix + 32 chars) |
| total_runs | u32 | 4 | Cumulative count of all dungeon runs |
| current_level | u8 | 1 | Current campaign level (0-80+) |
| unlocked_tier | u8 | 1 | Highest tier unlocked (0=free, 1=paid tier 1, etc.) |
| created_at | i64 | 8 | Unix timestamp of profile creation |
| bump | u8 | 1 | PDA bump seed |

**Total Size**: 8 (discriminator) + 83 = **91 bytes**

**Validation Rules**:
- `owner` must be a signer for all mutations
- `name` max length 32 UTF-8 characters
- `current_level` cannot exceed `(unlocked_tier + 1) * 40 - 1`
- `total_runs` only increments, never decrements

**State Transitions**:
```
[Not Exists] --initialize--> [Active]
[Active] --update_name--> [Active]
[Active] --increment_run--> [Active]
[Active] --unlock_tier--> [Active]
[Active] --close--> [Not Exists]
```

### Treasury Account

**PDA Seeds**: `[b"treasury"]`

| Field | Type | Size | Description |
|-------|------|------|-------------|
| admin | Pubkey | 32 | Authority that can withdraw funds |
| total_collected | u64 | 8 | Total SOL collected from tier unlocks |
| bump | u8 | 1 | PDA bump seed |

**Total Size**: 8 + 41 = **49 bytes**

**Note**: The Treasury PDA holds lamports directly. `total_collected` is for accounting purposes.

---

## Session Manager Program

### GameSession Account

**PDA Seeds**: `[b"session", player.key()]`

| Field | Type | Size | Description |
|-------|------|------|-------------|
| player | Pubkey | 32 | Player profile owner's wallet |
| session_id | u64 | 8 | Unique session identifier (incrementing) |
| campaign_level | u8 | 1 | Level being played in this session |
| started_at | i64 | 8 | Unix timestamp when session started |
| last_activity | i64 | 8 | Unix timestamp of last action |
| is_delegated | bool | 1 | Whether state is delegated to ephemeral rollup |
| state_hash | [u8; 32] | 32 | Hash of current game state (for verification) |
| bump | u8 | 1 | PDA bump seed |

**Total Size**: 8 + 91 = **99 bytes**

**Validation Rules**:
- One active session per player (enforced via PDA uniqueness)
- `is_delegated` must be true before ephemeral rollup operations
- Session timeout: `last_activity` older than 1 hour allows forced close

**State Transitions**:
```
[Not Exists] --start_session--> [Active, Not Delegated]
[Active, Not Delegated] --delegate--> [Active, Delegated]
[Active, Delegated] --commit--> [Active, Delegated] (state updated)
[Active, Delegated] --undelegate--> [Active, Not Delegated]
[Active, *] --end_session--> [Not Exists]
[Active, *] --force_close (timeout)--> [Not Exists]
```

### SessionCounter Account

**PDA Seeds**: `[b"session_counter"]`

| Field | Type | Size | Description |
|-------|------|------|-------------|
| count | u64 | 8 | Global session counter for unique IDs |
| bump | u8 | 1 | PDA bump seed |

**Total Size**: 8 + 9 = **17 bytes**

---

## Map Generator Program

### MapConfig Account

**PDA Seeds**: `[b"map_config"]`

| Field | Type | Size | Description |
|-------|------|------|-------------|
| admin | Pubkey | 32 | Authority that can update seed mappings |
| seeds | [u64; 81] | 648 | Seed values for campaign levels 0-80 |
| version | u8 | 1 | Config version for migrations |
| bump | u8 | 1 | PDA bump seed |

**Total Size**: 8 + 682 = **690 bytes**

**Validation Rules**:
- Only `admin` can call `update_map_config`
- Default seeds: `seeds[i] = i` for all levels
- Seed value 0 is valid (uses level number as fallback)

### Generated Map Structure (Off-Chain)

The map generation function produces this structure (not stored on-chain):

```rust
pub struct GeneratedMap {
    pub width: u8,                    // Always 50
    pub height: u8,                   // Always 50
    pub seed: u64,                    // Input seed
    pub tiles: [[TileType; 50]; 50],  // 2500 bytes
    pub spawn: Position,              // Player spawn point
    pub mole_den: Position,           // Home base position
    pub pois: Vec<MapPOI>,            // Up to ~40 POIs
    pub enemies: Vec<MapEnemy>,       // Up to ~125 enemies
    pub boss_spawn: Position,         // Boss spawn location
}

pub enum TileType {
    Floor = 0,
    Wall = 1,
    Gravel = 2,
    Water = 3,
}

pub struct Position {
    pub x: u8,
    pub y: u8,
}

pub struct MapPOI {
    pub poi_type: u8,     // POI definition ID (L1-L14)
    pub position: Position,
}

pub struct MapEnemy {
    pub enemy_type: u8,   // Enemy definition ID
    pub tier: u8,         // 1-3
    pub position: Position,
}
```

**Verification**: Client generates map from seed, submits hash to on-chain for verification.

---

## Relationships

```
┌─────────────────┐
│  PlayerProfile  │
│  (per wallet)   │
└────────┬────────┘
         │ 1:1
         ▼
┌─────────────────┐
│   GameSession   │
│  (per player)   │
└────────┬────────┘
         │ uses
         ▼
┌─────────────────┐
│    MapConfig    │
│   (singleton)   │
└─────────────────┘

┌─────────────────┐
│    Treasury     │
│   (singleton)   │
└─────────────────┘
```

---

## Events

### Player Profile Events

```rust
#[event]
pub struct ProfileCreated {
    pub owner: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TierUnlocked {
    pub owner: Pubkey,
    pub tier: u8,
    pub amount_paid: u64,
    pub timestamp: i64,
}

#[event]
pub struct RunCompleted {
    pub owner: Pubkey,
    pub total_runs: u32,
    pub level_reached: u8,
    pub victory: bool,
    pub timestamp: i64,
}
```

### Session Manager Events

```rust
#[event]
pub struct SessionStarted {
    pub player: Pubkey,
    pub session_id: u64,
    pub campaign_level: u8,
    pub timestamp: i64,
}

#[event]
pub struct SessionDelegated {
    pub player: Pubkey,
    pub session_id: u64,
    pub timestamp: i64,
}

#[event]
pub struct SessionEnded {
    pub player: Pubkey,
    pub session_id: u64,
    pub final_state_hash: [u8; 32],
    pub timestamp: i64,
}
```

### Map Generator Events

```rust
#[event]
pub struct MapConfigUpdated {
    pub admin: Pubkey,
    pub level: u8,
    pub old_seed: u64,
    pub new_seed: u64,
    pub timestamp: i64,
}
```
