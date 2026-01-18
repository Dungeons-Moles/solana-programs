# Data Model: Gameplay State Tracking

**Branch**: `002-gameplay-state-tracking` | **Date**: 2025-01-17

## Entities

### GameState

Core gameplay state account linked to a GameSession. Contains all mutable game data for a single run.

**PDA Seeds**: `["game_state", session_pda.as_ref()]`

| Field | Type | Description | Validation |
|-------|------|-------------|------------|
| player | Pubkey | Session owner's wallet | Must match session's player |
| session | Pubkey | Linked GameSession PDA | Must exist |
| position_x | u8 | Current X coordinate | 0 ≤ x < map_width |
| position_y | u8 | Current Y coordinate | 0 ≤ y < map_height |
| map_width | u8 | Map boundary X | Set on init, immutable |
| map_height | u8 | Map boundary Y | Set on init, immutable |
| hp | i8 | Current health points | 0 ≤ hp ≤ max_hp |
| max_hp | u8 | Maximum health points | Default: 10 |
| atk | i8 | Attack stat | Default: 1, allows negative (debuffs) |
| arm | i8 | Armor stat | Default: 0, allows negative |
| spd | i8 | Speed stat | Default: 0, allows negative |
| dig | i8 | Digging stat | Default: 1, affects wall dig cost |
| gear_slots | u8 | Gear slot capacity | 4 → 6 → 8 |
| week | u8 | Current week (1-3) | 1 ≤ week ≤ 3 |
| phase | Phase | Current time phase | Enum value |
| moves_remaining | u8 | Moves left in phase | 0-50 |
| total_moves | u32 | Total moves made | Accumulates across session |
| boss_fight_ready | bool | Boss fight triggered | Set when Week 3 Night 3 complete |
| bump | u8 | PDA bump seed | Stored on init |

**Size**: 92 bytes (including 8-byte discriminator)

### Phase (Enum)

Time phase enumeration determining move allowance.

| Variant | Move Allowance | Next Phase | Notes |
|---------|---------------|------------|-------|
| Day1 | 50 | Night1 | Start of week |
| Night1 | 30 | Day2 | |
| Day2 | 50 | Night2 | |
| Night2 | 30 | Day3 | |
| Day3 | 50 | Night3 | |
| Night3 | 30 | None (week end) | Triggers week transition or boss |

**Serialization**: Single byte (Anchor enum)

## State Transitions

### Phase Transitions

```
Day1 (50) → Night1 (30) → Day2 (50) → Night2 (30) → Day3 (50) → Night3 (30)
                                                                     ↓
                                                            Week ends (gear +2)
                                                                     ↓
                                                          Back to Day1 of next week
                                                                     OR
                                                          boss_fight_ready = true (Week 3)
```

### Gear Slots Progression

| Event | Gear Slots Before | Gear Slots After |
|-------|-------------------|------------------|
| Game Init | - | 4 |
| Week 1 Complete | 4 | 6 |
| Week 2 Complete | 6 | 8 |
| Week 3 Complete | 8 | 8 (capped) |

### Movement State Transitions

```
[Current Position] + move_player(target, is_wall)
    │
    ├─ [Adjacent floor tile] → Deduct 1 move → Update position
    │
    ├─ [Adjacent wall tile] → Calculate dig_cost = max(2, 6 - dig)
    │                       → Check moves_remaining ≥ dig_cost
    │                       → Deduct dig_cost → Update position
    │
    └─ [moves_remaining == 0 after move]
        │
        ├─ [Night3 of Week 3] → boss_fight_ready = true
        │
        ├─ [Night3 of Week 1/2] → week += 1, phase = Day1, gear_slots += 2
        │
        └─ [Other phases] → phase = phase.next(), moves_remaining = phase.moves_allowed()
```

## Relationships

```
┌─────────────────┐         ┌─────────────────┐
│  PlayerProfile  │         │   GameSession   │
│  (001-solana)   │────────>│  (001-solana)   │
│                 │  owns   │                 │
└─────────────────┘         └────────┬────────┘
                                     │
                                     │ links via PDA seed
                                     ▼
                            ┌─────────────────┐
                            │   GameState     │
                            │  (002-gameplay) │
                            │                 │
                            │ - position      │
                            │ - stats         │
                            │ - gear_slots    │
                            │ - phase/week    │
                            └─────────────────┘
```

## Validation Rules

### Position Validation
- `0 ≤ target_x < map_width`
- `0 ≤ target_y < map_height`
- `|target_x - position_x| + |target_y - position_y| == 1` (adjacent)

### Movement Cost Validation
- Floor tile: `moves_remaining ≥ 1`
- Wall tile: `moves_remaining ≥ max(2, 6 - dig)`

### Stat Modification Validation
- HP: `0 ≤ new_hp ≤ max_hp`
- Other stats: Allow negative values (debuffs)
- Use checked arithmetic to prevent overflow

### Week/Phase Validation
- `1 ≤ week ≤ 3`
- Phase is always a valid enum variant
- Cannot advance past boss_fight_ready state
