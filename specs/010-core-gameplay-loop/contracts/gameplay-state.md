# Contract: gameplay-state

**Program ID**: `5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n`

## Instructions

### initialize_game_state (MODIFIED)

Initializes game state with Basic Pickaxe equipped.

```rust
pub fn initialize_game_state(
    ctx: Context<InitializeGameState>,
    map_width: u8,
    map_height: u8,
    start_x: u8,
    start_y: u8,
) -> Result<()>
```

**Logic Changes**:

- Initializes player with Basic Pickaxe stats (+1 ATK at battle start)
- Sets `gear_slots = 4`
- All other initialization remains the same

---

### move_with_combat (NEW)

Moves player to adjacent tile, triggering combat if enemy present. Handles night enemy movement.

```rust
pub fn move_with_combat(
    ctx: Context<MoveWithCombat>,
    target_x: u8,
    target_y: u8,
    is_wall: bool,
) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| game_state | Account (mut) | Player's game state |
| map_enemies | Account (mut) | Enemies on map |
| map_pois | Account | POIs for reference (not mutated) |
| player_inventory | Account | Player's equipped items |
| game_session | Account | Parent session |
| player_profile | Account (mut) | For death/victory handling |
| player | Signer | Session owner or burner |

**Logic**:

1. Validate target is adjacent (Manhattan distance = 1)
2. Validate target is within bounds
3. Validate boss_fight_ready == false
4. Calculate move cost: floor=1, wall=max(2, 6-DIG)
5. Validate moves_remaining >= move_cost

6. **If night phase** (Night1, Night2, Night3):
   - For each alive enemy with chebyshev_distance <= 3:
     - Move enemy 1 tile toward player
     - If enemy.position == player.position:
       - Trigger combat (enemy attacking player)
       - If player HP <= 0: death handling
       - Else: mark enemy defeated

7. **Move player**:
   - Update position_x, position_y
   - Deduct move cost from moves_remaining
   - Increment total_moves

8. **Check for enemy at target**:
   - If enemy at (target_x, target_y) and not defeated:
     - Trigger combat inline
     - Emit all combat events
     - If player HP <= 0: death handling
     - Else: mark enemy defeated, award gold

9. **Check phase advancement**:
   - If moves_remaining == 0:
     - If phase == Night3:
       - If week < 3: trigger boss fight
       - If week == 3: trigger final boss fight
     - Else: advance phase, reset moves_remaining

10. **Handle combat results**:
    - If player defeated: CPI to close session (victory=false)
    - If Week 3 boss defeated: CPI to close session (victory=true)

11. Emit `PlayerMoved` event

---

### move_player (DEPRECATED)

Old movement instruction without combat. Kept for backward compatibility but marked deprecated.

---

## Combat Resolution (Inline)

Combat is resolved inline without CPI to avoid compute overhead:

```rust
fn resolve_combat_inline(
    player_stats: &PlayerCombatStats,
    enemy_stats: &EnemyCombatStats,
    player_effects: &[ItemEffect],
    enemy_effects: &[ItemEffect],
) -> CombatResult {
    // Deterministic combat loop (same logic as combat-system)
    // Emits TurnExecuted events for each turn
    // Returns final HP values for both combatants
}
```

**Events emitted during combat**:

- `CombatStarted { player_hp, enemy_hp, enemy_archetype }`
- `TurnExecuted { turn, player_hp, enemy_hp, player_damage, enemy_damage }`
- `StatusApplied { target, effect_type, stacks }`
- `CombatEnded { player_won, final_player_hp, final_enemy_hp }`

---

## Events

### PlayerMoved (MODIFIED)

```rust
#[event]
pub struct PlayerMoved {
    pub player: Pubkey,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
    pub moves_remaining: u8,
    pub is_dig: bool,
    pub combat_triggered: bool,
    pub enemies_moved: u8,  // NEW: count of enemies that moved during night
}
```

### CombatStarted (NEW)

```rust
#[event]
pub struct CombatStarted {
    pub player: Pubkey,
    pub player_hp: i16,
    pub player_atk: i16,
    pub enemy_archetype: u8,
    pub enemy_hp: i16,
    pub enemy_atk: i16,
}
```

### CombatEnded (NEW)

```rust
#[event]
pub struct CombatEnded {
    pub player: Pubkey,
    pub player_won: bool,
    pub final_player_hp: i16,
    pub final_enemy_hp: i16,
    pub gold_earned: u16,
    pub turns_taken: u8,
}
```

### BossCombatStarted (NEW)

```rust
#[event]
pub struct BossCombatStarted {
    pub player: Pubkey,
    pub boss_id: [u8; 12],
    pub boss_hp: i16,
    pub week: u8,
}
```

### EnemyMoved (NEW)

```rust
#[event]
pub struct EnemyMoved {
    pub enemy_index: u8,
    pub from_x: u8,
    pub from_y: u8,
    pub to_x: u8,
    pub to_y: u8,
}
```

### PlayerDefeated (NEW)

```rust
#[event]
pub struct PlayerDefeated {
    pub player: Pubkey,
    pub killed_by: String,  // "enemy" or "boss"
    pub final_hp: i16,
}
```

### LevelCompleted (NEW)

```rust
#[event]
pub struct LevelCompleted {
    pub player: Pubkey,
    pub level: u8,
    pub total_moves: u32,
    pub gold_earned: u16,
}
```
