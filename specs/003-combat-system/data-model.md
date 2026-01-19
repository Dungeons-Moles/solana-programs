# Data Model: Combat System & Status Effects

**Feature**: 003-combat-system
**Date**: 2026-01-18

## Entities

### 1. CombatState (Account)

Temporary account created at combat start, closed at combat end. Tracks the state of an active combat encounter.

**PDA Seeds**: `["combat_state", game_state.key()]`

| Field | Type | Description |
|-------|------|-------------|
| game_state | Pubkey | Reference to GameState PDA |
| player | Pubkey | Player wallet (for authorization) |
| turn | u8 | Current turn number (1-50) |
| player_hp | i16 | Player's current HP |
| player_max_hp | u16 | Player's maximum HP |
| player_atk | i16 | Player's ATK (includes sudden death bonus) |
| player_arm | i16 | Player's ARM |
| player_spd | i16 | Player's SPD |
| player_strikes | u8 | Player's strikes per turn |
| player_status | StatusEffects | Player's active status effects |
| enemy_hp | i16 | Enemy's current HP |
| enemy_max_hp | u16 | Enemy's maximum HP |
| enemy_atk | i16 | Enemy's ATK (includes sudden death bonus) |
| enemy_arm | i16 | Enemy's ARM |
| enemy_spd | i16 | Enemy's SPD |
| enemy_strikes | u8 | Enemy's strikes per turn |
| enemy_status | StatusEffects | Enemy's active status effects |
| sudden_death_bonus | i16 | ATK bonus from sudden death (Turn 25+) |
| combat_ended | bool | Flag indicating combat is resolved |
| player_won | bool | True if player won (only valid if combat_ended) |
| bump | u8 | PDA bump seed |

**Size**: 8 (discriminator) + 32 + 32 + 1 + 2 + 2 + 2 + 2 + 2 + 1 + 4 + 2 + 2 + 2 + 2 + 2 + 1 + 4 + 2 + 1 + 1 + 1 = ~98 bytes

### 2. StatusEffects (Struct)

Embedded struct representing active status effects on a combatant.

| Field | Type | Description |
|-------|------|-------------|
| chill | u8 | Chill stacks (reduces strikes, decays 1/turn) |
| shrapnel | u8 | Shrapnel stacks (damage on hit, clears at turn end) |
| rust | u8 | Rust stacks (armor decay, persists) |
| bleed | u8 | Bleed stacks (DoT, decays 1/turn) |

**Size**: 4 bytes

### 3. CombatResult (Event)

Emitted when combat concludes.

| Field | Type | Description |
|-------|------|-------------|
| game_state | Pubkey | Reference to GameState |
| player | Pubkey | Player wallet |
| player_won | bool | True if player won |
| final_turn | u8 | Turn on which combat ended |
| player_remaining_hp | i16 | Player's HP at end |
| enemy_remaining_hp | i16 | Enemy's HP at end (0 or less if player won) |
| resolution_type | ResolutionType | How combat ended |

### 4. ResolutionType (Enum)

How combat was resolved.

| Variant | Description |
|---------|-------------|
| PlayerDefeated | Player HP reached 0 |
| EnemyDefeated | Enemy HP reached 0 |
| SuddenDeathPlayerWin | Player won during sudden death (Turn 25+) |
| SuddenDeathEnemyWin | Enemy won during sudden death |
| FailsafePlayerWin | Player had higher HP% at Turn 50 |
| FailsafeEnemyWin | Enemy had higher or equal HP% at Turn 50 |

### 5. TriggerType (Enum)

Defines when an effect triggers during combat.

| Variant | Value | Description |
|---------|-------|-------------|
| BattleStart | 0 | Before Turn 1 |
| FirstTurn | 1 | During Turn 1 only |
| TurnStart | 2 | Start of each turn |
| EveryOtherTurn | 3 | Turns 2, 4, 6, ... |
| OnHit | 4 | When a strike lands (once per turn) |
| Exposed | 5 | When combatant has 0 ARM |
| Wounded | 6 | When HP < 50% max HP |

### 6. CombatantInput (Struct)

Input data for a combatant when starting combat. Used to initialize combat state.

| Field | Type | Description |
|-------|------|-------------|
| hp | i16 | Starting HP |
| max_hp | u16 | Maximum HP |
| atk | i16 | Base ATK |
| arm | i16 | Base ARM |
| spd | i16 | Base SPD |
| dig | i16 | DIG stat (for comparisons) |
| strikes | u8 | Strikes per turn |

**Size**: 15 bytes

### 7. ItemEffect (Struct)

Represents an item's combat effect. Passed as input when starting combat.

| Field | Type | Description |
|-------|------|-------------|
| trigger | TriggerType | When effect activates |
| once_per_turn | bool | If true, only triggers once per turn |
| effect_type | EffectType | What the effect does |
| value | i16 | Magnitude of effect |

### 8. EffectType (Enum)

Types of effects items can have.

| Variant | Description |
|---------|-------------|
| DealDamage | Deal weapon damage |
| DealNonWeaponDamage | Deal damage ignoring armor |
| Heal | Restore HP |
| GainArmor | Add ARM |
| GainAtk | Add ATK |
| GainSpd | Add SPD |
| ApplyChill | Apply Chill stacks to target |
| ApplyShrapnel | Apply Shrapnel stacks to self |
| ApplyRust | Apply Rust stacks to target |
| ApplyBleed | Apply Bleed stacks to target |
| RemoveArmor | Remove ARM from target |

## Relationships

```
GameState (gameplay-state program)
    │
    ├── creates/closes ─────► CombatState (combat-system program)
    │                              │
    │                              ├── StatusEffects (player)
    │                              └── StatusEffects (enemy)
    │
    └── receives ◄───────────── CombatResult (event)
```

## State Transitions

### Combat Lifecycle

```
INACTIVE                    No CombatState account exists
    │
    ▼ (initialize_combat)
ACTIVE                      CombatState created, turn = 1
    │
    ├─► PLAYER_TURN        Player acts first (higher SPD)
    │       │
    │       ▼
    │   ENEMY_TURN         Enemy acts
    │       │
    │       ▼
    │   TURN_END           Status decay, turn++
    │       │
    │       ├─► (turn < 50 && both alive) ──► PLAYER_TURN/ENEMY_TURN
    │       │
    │       └─► (turn >= 25) ──► sudden_death_bonus++
    │
    └─► RESOLVED           combat_ended = true
            │
            ▼ (close_combat)
        INACTIVE           CombatState closed, rent returned
```

### Status Effect Lifecycle

| Effect | Application | Per-Turn | Decay | Clear |
|--------|-------------|----------|-------|-------|
| Chill | Apply N stacks | Reduces strikes by stacks | -1 stack at turn end | When stacks = 0 |
| Shrapnel | Apply N stacks | Deals stacks damage when struck | - | Clears at turn end |
| Rust | Apply N stacks | Reduces ARM by stacks at turn end | Never | Manual only |
| Bleed | Apply N stacks | Deals stacks damage at turn end | -1 stack at turn end | When stacks = 0 |

## Validation Rules

### CombatState

- `turn` must be 1-50
- `player_hp` can be negative (for overkill tracking)
- `player_arm`, `enemy_arm` clamped to min 0 after Rust
- `player_strikes`, `enemy_strikes` always >= 1 (Chill minimum)
- `sudden_death_bonus` = max(0, turn - 24)

### Combat Resolution

- Combat ends immediately when either HP <= 0
- At Turn 50: compare HP percentages (current/max), enemy wins ties
- Winner is set in `player_won` field

### Trigger Evaluation

- Effects evaluated in slot order (deterministic)
- `once_per_turn` effects track triggered state, reset at turn end
- Condition triggers (Exposed, Wounded) check after each action
