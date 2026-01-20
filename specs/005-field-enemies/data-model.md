# Data Model: Field Enemies

**Feature**: 005-field-enemies  
**Date**: 2026-01-19

## Entity Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      STATIC DATA (Compiled)                     │
├─────────────────────────────────────────────────────────────────┤
│  EnemyArchetype (12)  ◄───────┬───────► EnemyStats (36)        │
│  - id: u8                     │         - hp: u16               │
│  - emoji: [u8; 4]             │         - atk: u8               │
│  - biome_weight_a: u8         │         - arm: u8               │
│  - biome_weight_b: u8         │         - spd: u8               │
│  - trait: EnemyTrait          │         - dig: u8               │
│                               │                                 │
│                      ┌────────┴────────┐                        │
│                      │   EnemyTier     │                        │
│                      │   T1 | T2 | T3  │                        │
│                      └─────────────────┘                        │
└─────────────────────────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    RUNTIME DATA (On-chain)                      │
├─────────────────────────────────────────────────────────────────┤
│  MapEnemies (PDA per session)                                   │
│  - session: Pubkey                                              │
│  - enemies: [EnemyInstance; MAX_ENEMIES]                        │
│  - count: u8                                                    │
│  - bump: u8                                                     │
│                                                                 │
│  EnemyInstance (5 bytes each)                                   │
│  - archetype_id: u8                                             │
│  - tier: u8                                                     │
│  - x: u8                                                        │
│  - y: u8                                                        │
│  - defeated: bool                                               │
└─────────────────────────────────────────────────────────────────┘
```

## Entities

### EnemyArchetype (Static, 12 instances)

Represents a template for an enemy type. Compiled into program as const data.

| Field          | Type                  | Description                                    |
| -------------- | --------------------- | ---------------------------------------------- |
| id             | u8                    | Unique identifier (0-11)                       |
| name           | &'static str          | Display name (e.g., "Tunnel Rat")              |
| emoji          | [u8; 4]               | UTF-8 encoded emoji bytes                      |
| biome_weight_a | u8                    | Spawn weight in Biome A (1=base, 2=emphasized) |
| biome_weight_b | u8                    | Spawn weight in Biome B                        |
| traits         | &'static [EnemyTrait] | Trait effects (1-2 per archetype)              |

**Archetype IDs**:

- 0: Tunnel Rat
- 1: Cave Bat
- 2: Spore Slime
- 3: Rust Mite Swarm
- 4: Collapsed Miner
- 5: Shard Beetle
- 6: Tunnel Warden
- 7: Burrow Ambusher
- 8: Frost Wisp
- 9: Powder Tick
- 10: Coin Slug
- 11: Blood Mosquito

### EnemyStats (Static, 36 instances)

Stats for each archetype at each tier. Indexed by `archetype_id * 3 + tier`.

| Field | Type | Description  |
| ----- | ---- | ------------ |
| hp    | u16  | Hit points   |
| atk   | u8   | Attack power |
| arm   | u8   | Armor        |
| spd   | u8   | Speed        |
| dig   | u8   | Dig stat     |

### EnemyTier (Enum)

```rust
#[repr(u8)]
pub enum EnemyTier {
    T1 = 0,
    T2 = 1,
    T3 = 2,
}
```

**Gold Rewards**:

- T1 → 2 Gold
- T2 → 4 Gold
- T3 → 6 Gold

### EnemyTrait (Static)

Reuses existing `ItemEffect` structure from combat-system.

| Field         | Type        | Description                                |
| ------------- | ----------- | ------------------------------------------ |
| trigger       | TriggerType | When effect fires                          |
| once_per_turn | bool        | Limit to once per turn                     |
| effect_type   | EffectType  | What happens                               |
| value         | i16         | Effect magnitude                           |
| countdown     | Option<u8>  | Turns until trigger (for Countdown traits) |

### EnemyInstance (Runtime, 5 bytes)

Represents a spawned enemy on the map.

| Field        | Type | Size | Description               |
| ------------ | ---- | ---- | ------------------------- |
| archetype_id | u8   | 1    | References EnemyArchetype |
| tier         | u8   | 1    | 0=T1, 1=T2, 2=T3          |
| x            | u8   | 1    | Map X coordinate          |
| y            | u8   | 1    | Map Y coordinate          |
| defeated     | bool | 1    | True if already killed    |

### MapEnemies (PDA Account)

On-chain account storing all enemy instances for a map.

| Field   | Type                | Size | Description          |
| ------- | ------------------- | ---- | -------------------- |
| session | Pubkey              | 32   | Parent session PDA   |
| enemies | [EnemyInstance; 48] | 240  | Enemy array (max 48) |
| count   | u8                  | 1    | Actual enemy count   |
| bump    | u8                  | 1    | PDA bump seed        |

**PDA Seeds**: `["map_enemies", session.as_ref()]`

**Account Size**: 32 + 240 + 1 + 1 = 274 bytes + 8 (discriminator) = 282 bytes

## Tier Distribution Configuration (Static)

```rust
pub struct TierDistribution {
    pub t1_threshold: u8,  // 0..t1 = T1
    pub t2_threshold: u8,  // t1..t2 = T2, t2..100 = T3
}

pub const TIER_DISTRIBUTIONS: [TierDistribution; 4] = [
    TierDistribution { t1_threshold: 70, t2_threshold: 95 },  // Act 1
    TierDistribution { t1_threshold: 55, t2_threshold: 90 },  // Act 2
    TierDistribution { t1_threshold: 45, t2_threshold: 85 },  // Act 3
    TierDistribution { t1_threshold: 35, t2_threshold: 80 },  // Act 4
];

pub const SPAWN_COUNTS: [u8; 4] = [36, 40, 44, 48];
```

## Biome Weights (Static)

```rust
// Index matches archetype_id, values are relative weights
pub const BIOME_A_WEIGHTS: [u8; 12] = [
    2,  // Tunnel Rat (emphasized)
    1,  // Cave Bat
    1,  // Spore Slime
    1,  // Rust Mite Swarm
    2,  // Collapsed Miner (emphasized)
    2,  // Shard Beetle (emphasized)
    1,  // Tunnel Warden
    1,  // Burrow Ambusher
    1,  // Frost Wisp
    1,  // Powder Tick
    2,  // Coin Slug (emphasized)
    1,  // Blood Mosquito
];

pub const BIOME_B_WEIGHTS: [u8; 12] = [
    1,  // Tunnel Rat
    1,  // Cave Bat
    1,  // Spore Slime
    2,  // Rust Mite Swarm (emphasized)
    1,  // Collapsed Miner
    1,  // Shard Beetle
    1,  // Tunnel Warden
    2,  // Burrow Ambusher (emphasized)
    2,  // Frost Wisp (emphasized)
    2,  // Powder Tick (emphasized)
    1,  // Coin Slug
    2,  // Blood Mosquito (emphasized)
];
```

## Relationships

```
GameSession (session-manager)
    │
    ├──► GameState (gameplay-state)
    │        └── player stats, position, gold
    │
    └──► MapEnemies (field-enemies)
             └── EnemyInstance[]
                     │
                     └── references → EnemyArchetype (static)
                                           │
                                           └── EnemyStats[tier] (static)
                                           └── EnemyTrait[] (static)
```

## Validation Rules

### EnemyInstance Validation

- `archetype_id` MUST be 0-11
- `tier` MUST be 0-2
- `x` MUST be < map_width
- `y` MUST be < map_height
- Position MUST be on Floor tile (not Wall)

### MapEnemies Validation

- `count` MUST equal actual populated entries in `enemies`
- `count` MUST match SPAWN_COUNTS[act] ± 10%
- No duplicate positions among non-defeated enemies

### Combat Integration

- On combat start: lookup EnemyStats by `archetype_id * 3 + tier`
- On combat end (victory): set `defeated = true`, return tier for Gold calculation
- Gold reward: `(tier + 1) * 2`

## State Transitions

```
┌─────────────────┐
│ Map Generation  │
│ (spawn enemies) │
└────────┬────────┘
         │ create MapEnemies PDA
         │ populate EnemyInstance[]
         ▼
┌─────────────────┐
│ Enemy Active    │◄─────────────────┐
│ defeated=false  │                  │
└────────┬────────┘                  │
         │ player steps on tile      │
         ▼                           │
┌─────────────────┐                  │
│ Combat Started  │                  │
│                 │                  │
└────────┬────────┘                  │
         │                           │
    ┌────┴────┐                      │
    ▼         ▼                      │
┌───────┐ ┌───────┐                  │
│Player │ │Enemy  │                  │
│ Wins  │ │ Wins  │                  │
└───┬───┘ └───┬───┘                  │
    │         │                      │
    ▼         ▼                      │
┌───────┐ ┌───────────┐              │
│Defeat │ │ Run Ends  │              │
│Enemy  │ │(GameState)│              │
│+Gold  │ └───────────┘              │
└───┬───┘                            │
    │ defeated=true                  │
    ▼                                │
┌─────────────────┐                  │
│ Enemy Defeated  │ (exploration     │
│ defeated=true   │  continues)──────┘
└─────────────────┘
```
