# Gameplay State IDL Specification

**Program ID**: TBD (will be generated on first build)
**Version**: 0.1.0

## Instructions

### initialize_game_state

Initializes a new GameState account linked to an active GameSession.

**Arguments**:
| Name | Type | Description |
|------|------|-------------|
| map_width | u8 | Map boundary X dimension |
| map_height | u8 | Map boundary Y dimension |
| start_x | u8 | Starting X position |
| start_y | u8 | Starting Y position |

**Accounts**:
| Name | Writable | Signer | Description |
|------|----------|--------|-------------|
| game_state | ✅ | ❌ | GameState PDA (init) |
| game_session | ❌ | ❌ | Linked GameSession PDA |
| player | ✅ | ✅ | Session owner, payer |
| system_program | ❌ | ❌ | System program |

**Errors**:
- `SessionNotActive`: GameSession does not exist
- `Unauthorized`: Caller is not the session owner
- `OutOfBounds`: Starting position outside map boundaries

**Events**:
- `GameStateInitialized { player, session, map_width, map_height }`

---

### move_player

Moves the player to an adjacent tile, deducting move cost.

**Arguments**:
| Name | Type | Description |
|------|------|-------------|
| target_x | u8 | Target X coordinate |
| target_y | u8 | Target Y coordinate |
| is_wall | bool | True if target is a wall tile (digging) |

**Accounts**:
| Name | Writable | Signer | Description |
|------|----------|--------|-------------|
| game_state | ✅ | ❌ | GameState PDA |
| player | ❌ | ✅ | Session owner |

**Errors**:
- `OutOfBounds`: Target position outside map boundaries
- `NotAdjacent`: Target not adjacent to current position
- `InsufficientMoves`: Not enough moves_remaining
- `Unauthorized`: Caller is not the session owner
- `BossFightAlreadyTriggered`: Cannot move after boss fight ready

**Events**:
- `PlayerMoved { player, from_x, from_y, to_x, to_y, moves_remaining, is_dig }`
- `PhaseAdvanced { player, new_phase, new_week, moves_remaining }` (conditional)
- `BossFightReady { player, week }` (conditional)

---

### modify_stat

Modifies a player stat by a delta value.

**Arguments**:
| Name | Type | Description |
|------|------|-------------|
| stat | StatType | Which stat to modify |
| delta | i8 | Amount to add (negative for decrease) |

**Accounts**:
| Name | Writable | Signer | Description |
|------|----------|--------|-------------|
| game_state | ✅ | ❌ | GameState PDA |
| player | ❌ | ✅ | Session owner |

**Errors**:
- `Unauthorized`: Caller is not the session owner
- `HpUnderflow`: HP would go below 0
- `StatOverflow`: Stat value would overflow
- `InvalidStatModification`: Invalid stat type

**Events**:
- `StatModified { player, stat, old_value, new_value }`

---

### close_game_state

Closes the GameState account, returning rent to player.

**Arguments**: None

**Accounts**:
| Name | Writable | Signer | Description |
|------|----------|--------|-------------|
| game_state | ✅ | ❌ | GameState PDA (close) |
| player | ✅ | ✅ | Session owner, rent recipient |

**Errors**:
- `Unauthorized`: Caller is not the session owner

**Events**:
- `GameStateClosed { player, total_moves, final_phase, final_week }`

---

## Types

### Phase (Enum)

```
Day1 = 0
Night1 = 1
Day2 = 2
Night2 = 3
Day3 = 4
Night3 = 5
```

### StatType (Enum)

```
Hp = 0
MaxHp = 1
Atk = 2
Arm = 3
Spd = 4
Dig = 5
```

### GameState (Account)

```
player: Pubkey
session: Pubkey
position_x: u8
position_y: u8
map_width: u8
map_height: u8
hp: i8
max_hp: u8
atk: i8
arm: i8
spd: i8
dig: i8
gear_slots: u8
week: u8
phase: Phase
moves_remaining: u8
total_moves: u32
boss_fight_ready: bool
bump: u8
```

---

## Events

### GameStateInitialized
```
player: Pubkey
session: Pubkey
map_width: u8
map_height: u8
```

### PlayerMoved
```
player: Pubkey
from_x: u8
from_y: u8
to_x: u8
to_y: u8
moves_remaining: u8
is_dig: bool
```

### PhaseAdvanced
```
player: Pubkey
new_phase: Phase
new_week: u8
moves_remaining: u8
```

### BossFightReady
```
player: Pubkey
week: u8
```

### StatModified
```
player: Pubkey
stat: StatType
old_value: i8
new_value: i8
```

### GameStateClosed
```
player: Pubkey
total_moves: u32
final_phase: Phase
final_week: u8
```

---

## Error Codes

| Code | Name | Message |
|------|------|---------|
| 6000 | OutOfBounds | Target position is out of map boundaries |
| 6001 | InsufficientMoves | Not enough moves remaining for this action |
| 6002 | NotAdjacent | Target position is not adjacent to current position |
| 6003 | StatOverflow | Stat value would overflow |
| 6004 | HpUnderflow | HP cannot go below 0 |
| 6005 | InvalidStatModification | Invalid stat modification |
| 6006 | BossFightAlreadyTriggered | Boss fight already triggered |
| 6007 | Unauthorized | Unauthorized: only session owner can modify state |
| 6008 | SessionNotActive | Session is not active |
| 6009 | ArithmeticOverflow | Arithmetic overflow |
