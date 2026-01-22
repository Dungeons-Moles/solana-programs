# Data Model: Core Gameplay Loop

**Feature**: 010-core-gameplay-loop  
**Date**: 2026-01-21

## Entity Overview

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  PlayerProfile  │────▶│   GameSession    │────▶│    GameState    │
│   (persistent)  │     │  (per level/run) │     │  (in-run state) │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                │
                    ┌───────────┼───────────┐
                    ▼           ▼           ▼
              ┌──────────┐ ┌──────────┐ ┌──────────────┐
              │MapEnemies│ │ MapPois  │ │PlayerInventory│
              └──────────┘ └──────────┘ └──────────────┘
```

---

## PlayerProfile (Modified)

**PDA Seeds**: `["player", owner.key()]`

| Field                  | Type            | Description                         | Change                       |
| ---------------------- | --------------- | ----------------------------------- | ---------------------------- |
| owner                  | Pubkey          | Wallet address                      | -                            |
| name                   | String (max 32) | Display name                        | -                            |
| total_runs             | u32             | Total runs completed                | -                            |
| highest_level_unlocked | u8              | Max level player can access (1-40)  | RENAMED from `current_level` |
| available_runs         | u32             | Remaining runs                      | Default: 20 (was 40)         |
| created_at             | i64             | Unix timestamp                      | -                            |
| bump                   | u8              | PDA bump                            | -                            |
| unlocked_items         | [u8; 10]        | Bitmask of unlocked items (80 bits) | NEW                          |
| active_item_pool       | [u8; 10]        | Bitmask of items in active pool     | NEW                          |

**Size Calculation**:

- Base: 32 + 4 + 32 + 4 + 1 + 4 + 8 + 1 = 86 bytes
- New fields: 10 + 10 = 20 bytes
- **Total**: 106 bytes (+ 8 discriminator = 114 bytes)

**Validation Rules**:

- `highest_level_unlocked` starts at 1, max 40
- `unlocked_items` initialized with STARTER_ITEMS_BITMASK (bits 0-39 set)
- `active_item_pool` must be subset of `unlocked_items`
- `active_item_pool` must have >= 40 bits set

---

## GameSession (Modified)

**PDA Seeds**: `["session", player.key(), &[campaign_level]]` (CHANGED)

| Field            | Type     | Description                        | Change |
| ---------------- | -------- | ---------------------------------- | ------ |
| player           | Pubkey   | Session owner                      | -      |
| session_id       | u64      | Global unique ID                   | -      |
| campaign_level   | u8       | Level being played                 | -      |
| started_at       | i64      | Unix timestamp                     | -      |
| last_activity    | i64      | Last action timestamp              | -      |
| is_delegated     | bool     | MagicBlock delegation flag         | -      |
| state_hash       | [u8; 32] | State verification hash            | -      |
| bump             | u8       | PDA bump                           | -      |
| active_item_pool | [u8; 10] | Snapshot of player's pool at start | NEW    |
| burner_wallet    | Pubkey   | Burner wallet for gameplay         | NEW    |

**Size Calculation**:

- Base: 32 + 8 + 1 + 8 + 8 + 1 + 32 + 1 = 91 bytes
- New fields: 10 + 32 = 42 bytes
- **Total**: 133 bytes (+ 8 discriminator = 141 bytes)

**Validation Rules**:

- `campaign_level <= player_profile.highest_level_unlocked`
- One session per (player, level) pair
- `active_item_pool` copied from PlayerProfile at session start

---

## GameState (Modified)

**PDA Seeds**: `["game_state", session.key()]`

| Field            | Type   | Description           | Change |
| ---------------- | ------ | --------------------- | ------ |
| player           | Pubkey | Session owner         | -      |
| session          | Pubkey | Parent session PDA    | -      |
| position_x       | u8     | Player X coord        | -      |
| position_y       | u8     | Player Y coord        | -      |
| map_width        | u8     | Map boundary X        | -      |
| map_height       | u8     | Map boundary Y        | -      |
| hp               | i8     | Current HP            | -      |
| max_hp           | u8     | Maximum HP            | -      |
| atk              | i8     | Attack stat           | -      |
| arm              | i8     | Armor stat            | -      |
| spd              | i8     | Speed stat            | -      |
| dig              | i8     | Dig stat              | -      |
| gear_slots       | u8     | Gear capacity (4→6→8) | -      |
| week             | u8     | Current week (1-3)    | -      |
| phase            | Phase  | Day1-Night3           | -      |
| moves_remaining  | u8     | Moves left in phase   | -      |
| total_moves      | u32    | Total moves made      | -      |
| boss_fight_ready | bool   | Boss trigger flag     | -      |
| gold             | u16    | Current gold          | -      |
| bump             | u8     | PDA bump              | -      |

**Size**: 94 bytes (+ 8 discriminator = 102 bytes)

No changes to GameState - all enemy/POI data stored in separate accounts.

---

## MapEnemies (Existing - No Changes)

**PDA Seeds**: `["map_enemies", session.key()]`

| Field   | Type                        | Description       |
| ------- | --------------------------- | ----------------- |
| session | Pubkey                      | Parent session    |
| enemies | Vec<EnemyInstance> (max 48) | Enemy instances   |
| count   | u8                          | Number of enemies |
| bump    | u8                          | PDA bump          |

**EnemyInstance** (5 bytes each):
| Field | Type | Description |
|-------|------|-------------|
| archetype_id | u8 | Enemy type (0-11) |
| tier | u8 | T1/T2/T3 |
| x | u8 | Position X |
| y | u8 | Position Y |
| defeated | bool | Is dead |

---

## MapPois (Existing - No Changes)

**PDA Seeds**: `["map_pois", session.key()]`

Already has all needed fields for position validation and state tracking.

---

## PlayerInventory (Modified)

**PDA Seeds**: `["inventory", player.key()]`

| Field              | Type                      | Description    | Change                        |
| ------------------ | ------------------------- | -------------- | ----------------------------- |
| player             | Pubkey                    | Owner          | -                             |
| tool               | Option<ItemInstance>      | Equipped tool  | Initialize with Basic Pickaxe |
| gear               | [Option<ItemInstance>; 8] | Equipped gear  | -                             |
| gear_slot_capacity | u8                        | Unlocked slots | -                             |
| bump               | u8                        | PDA bump       | -                             |

**ItemInstance** (Modified):
| Field | Type | Description | Change |
|-------|------|-------------|--------|
| item_id | [u8; 8] | Item definition ID | - |
| tier | Tier | Common/Rare/Heroic/Mythic | - |
| oil_atk | bool | +ATK oil applied | - |
| oil_spd | bool | +SPD oil applied | - |
| oil_dig | bool | +DIG oil applied | - |

---

## New: Basic Pickaxe Item Definition

Added to `player-inventory/src/items.rs`:

```rust
pub const BASIC_PICKAXE: ItemDefinition = ItemDefinition {
    id: b"T-XX-00\0",
    name: "Basic Pickaxe",
    item_type: ItemType::Tool,
    tag: ItemTag::None,
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainAtk,
        false,
        [1, 1, 1],
    )],
};
```

---

## New Constants

**player-profile/src/constants.rs**:

```rust
pub const INITIAL_AVAILABLE_RUNS: u32 = 20;
pub const MAX_CAMPAIGN_LEVEL: u8 = 40;
pub const TOTAL_ITEMS: u8 = 80;
pub const STARTER_ITEMS_COUNT: u8 = 40;
pub const MIN_ACTIVE_POOL_SIZE: u8 = 40;
pub const RUN_PURCHASE_COST_LAMPORTS: u64 = 1_000_000; // 0.001 SOL
pub const RUNS_PER_PURCHASE: u32 = 20;
pub const ITEM_BITMASK_SIZE: usize = 10;
```

**player-profile/src/bitmask.rs** (NEW):

```rust
pub const STARTER_ITEMS_BITMASK: [u8; 10] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0x00, 0x00, 0x00, 0x00, 0x00,
];

pub fn is_bit_set(mask: [u8; 10], index: u8) -> bool;
pub fn set_bit(mask: &mut [u8; 10], index: u8);
pub fn count_bits(mask: [u8; 10]) -> u8;
pub fn is_subset(pool: [u8; 10], unlocked: [u8; 10]) -> bool;
```

---

## New Error Codes

**player-profile/src/errors.rs**:

```rust
pub enum PlayerProfileError {
    // ... existing ...
    ActivePoolTooSmall,      // Pool has < 40 items
    ItemNotUnlocked,         // Item not in unlocked_items
    InvalidItemIndex,        // Index out of range
    InsufficientPayment,     // Not enough SOL for purchase
    LevelNotUnlocked,        // Level > highest_level_unlocked
}
```

**session-manager/src/errors.rs**:

```rust
pub enum SessionManagerError {
    // ... existing ...
    LevelNotUnlocked,        // Cannot start level above unlocked
    NoAvailableRuns,         // No runs remaining
    SessionExistsForLevel,   // Already have session on this level
}
```

**gameplay-state/src/errors.rs**:

```rust
pub enum GameplayStateError {
    // ... existing ...
    EnemyNotAtPosition,      // No enemy at target tile
    PlayerDefeated,          // HP <= 0
}
```

**poi-system/src/errors.rs**:

```rust
pub enum PoiError {
    // ... existing ...
    PlayerNotOnPoiTile,      // Player position != POI position
}
```

---

## State Transitions

### Session Lifecycle

```
[No Session]
    │ start_session(level, burner_lamports)
    ▼
[Active Session]
    │ move_with_combat / interact_poi
    ▼
[Combat Result]
    ├─ Player HP <= 0 ──▶ [Death] ──▶ close_session(victory=false)
    └─ Enemy/Boss HP <= 0 ──▶ [Victory Check]
                                 ├─ Week 1/2 Boss ──▶ [Next Week]
                                 └─ Week 3 Boss ──▶ close_session(victory=true)
```

### Item Unlock Flow

```
[Level Complete (first time)]
    │ victory=true, level > highest_level_unlocked - 1
    ▼
[Select Random Item]
    │ PRNG(player, level, slot) → locked_item_index
    ▼
[Unlock Item]
    │ set_bit(unlocked_items, index)
    │ set_bit(active_item_pool, index)
    ▼
[Emit ItemUnlocked Event]
```

### Night Enemy Movement

```
[Player Move During Night]
    │
    ▼
[For Each Enemy]
    │ if chebyshev_distance(enemy, player) <= 3
    │   move_toward(enemy, player)
    │   if enemy.position == player.position
    │     trigger_combat(enemy)
    ▼
[Player Move Executes]
    │ if enemy_at(target_x, target_y)
    │   trigger_combat(enemy)
```
