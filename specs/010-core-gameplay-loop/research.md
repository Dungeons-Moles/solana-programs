# Research: Core Gameplay Loop

**Feature**: 010-core-gameplay-loop  
**Date**: 2026-01-21

## Research Topics

### 1. Atomic Session Creation with Map Data

**Decision**: Bundle session + game state + enemies + POIs initialization into a single transaction using instruction bundling (not CPI).

**Rationale**:

- Solana transactions can contain multiple instructions
- Frontend constructs transaction with: `start_session` → `initialize_game_state` → `spawn_enemies` → `spawn_pois` → `initialize_inventory`
- All instructions use the same session PDA as a reference
- If any instruction fails, entire transaction rolls back

**Alternatives Considered**:

- Single mega-instruction with CPI: Rejected due to compute unit limits and complexity
- Lazy initialization: Rejected because user wants all data on-chain immediately

**Implementation Notes**:

- Each instruction must validate session PDA exists
- Use `init_if_needed` pattern where appropriate
- Frontend SDK provides `createSessionBundle()` helper

---

### 2. Session PDA for Multi-Session Support

**Decision**: Change session PDA seeds from `["session", player]` to `["session", player, &[level]]`.

**Rationale**:

- Enables one session per level per player
- Level is u8, fitting in a single byte seed
- Existing `GameSession.campaign_level` field becomes redundant but kept for clarity

**Alternatives Considered**:

- Session counter per player: More complex, requires additional account
- Random session ID: Loses deterministic PDA derivation

**Implementation Notes**:

```rust
// Old: seeds = [b"session", player.key().as_ref()]
// New: seeds = [b"session", player.key().as_ref(), &[campaign_level]]
```

---

### 3. Movement with Automatic Combat

**Decision**: Create `move_with_combat` instruction in `gameplay-state` that inlines combat logic (no CPI).

**Rationale**:

- CPI adds ~5k compute units per call
- Combat logic is deterministic and can be inlined
- Events can be emitted from a single instruction
- Simplifies transaction construction

**Alternatives Considered**:

- CPI to combat-system: Higher compute cost, more complex account validation
- Separate move then combat: Allows front-running/abandonment exploit

**Implementation Notes**:

- Import combat engine functions directly (no CPI)
- Emit `CombatAction` events during resolution
- Check `MapEnemies.get_enemy_at_position(target_x, target_y)`
- If enemy exists and not defeated, resolve combat inline

---

### 4. Night Enemy Movement (Chebyshev Distance)

**Decision**: Use Chebyshev distance (max of |dx|, |dy|) for "3 tiles in every direction".

**Rationale**:

- User specified "3 tiles away in every direction"
- Chebyshev distance ≤ 3 means a 7x7 square centered on player
- Matches common game grid movement patterns

**Alternatives Considered**:

- Manhattan distance: Would create diamond shape, not square
- Euclidean distance: Non-integer, harder to reason about

**Implementation Notes**:

```rust
fn chebyshev_distance(x1: u8, y1: u8, x2: u8, y2: u8) -> u8 {
    let dx = (x1 as i16 - x2 as i16).abs() as u8;
    let dy = (y1 as i16 - y2 as i16).abs() as u8;
    dx.max(dy)
}

// Enemy moves if chebyshev_distance(enemy, player) <= 3
```

---

### 5. Enemy Movement Toward Player

**Decision**: Enemies move 1 tile toward player using greedy pathfinding (reduce largest delta first).

**Rationale**:

- Simple, deterministic, compute-efficient
- No complex pathfinding needed for this game design
- Walls are not obstacles for enemies (they phase through during night)

**Alternatives Considered**:

- A\* pathfinding: Too compute-intensive
- Random movement: Not strategic, doesn't match user description

**Implementation Notes**:

```rust
fn move_toward(enemy: &mut EnemyInstance, player_x: u8, player_y: u8) {
    let dx = player_x as i16 - enemy.x as i16;
    let dy = player_y as i16 - enemy.y as i16;

    // Move in direction of largest delta
    if dx.abs() >= dy.abs() {
        enemy.x = if dx > 0 { enemy.x + 1 } else { enemy.x.saturating_sub(1) };
    } else {
        enemy.y = if dy > 0 { enemy.y + 1 } else { enemy.y.saturating_sub(1) };
    }
}
```

---

### 6. Boss Trigger Timing

**Decision**: Boss fight triggers when `moves_remaining` becomes 0 AND `phase` is `Night3`.

**Rationale**:

- Matches user spec: "last move of week"
- Deterministic timing
- Already have `boss_fight_ready` flag in GameState

**Alternatives Considered**:

- Trigger on phase transition: Less precise timing
- Manual trigger: Allows avoidance, doesn't match spec

**Implementation Notes**:

- In `move_with_combat`, after deducting moves:
  1. Check if `moves_remaining == 0 && phase == Night3`
  2. If true, resolve boss combat inline
  3. On victory: advance week or complete level

---

### 7. Atomic Death/Victory Handling

**Decision**: Death and victory handling occur inline within the same instruction that triggers them.

**Rationale**:

- User requires "same transaction"
- Combat result immediately determines outcome
- CPI to player-profile for run deduction and level unlock

**Alternatives Considered**:

- Separate `finalize_session` instruction: Allows exploitation
- Event-based finalization: Not atomic

**Implementation Notes**:

- On player death (HP ≤ 0):
  1. CPI to `player-profile::record_run_result(victory: false)`
  2. Close session account (return rent)
  3. Close all associated accounts (game_state, map_enemies, map_pois, inventory)
- On Week 3 boss victory:
  1. CPI to `player-profile::record_run_result(victory: true, level)`
  2. Same cleanup as death

---

### 8. Item Unlock Randomization

**Decision**: Use deterministic PRNG seeded with `(player_pubkey, level, slot.unix_timestamp)` for random item selection.

**Rationale**:

- On-chain randomness is impossible without oracle
- Deterministic PRNG is auditable and fair
- Slot timestamp provides entropy without external dependency

**Alternatives Considered**:

- Sequential unlock: Less exciting, predictable
- Off-chain oracle (Switchboard/Pyth): Adds complexity and cost
- Player-provided seed: Exploitable

**Implementation Notes**:

```rust
fn select_random_locked_item(
    unlocked_items: [u8; 10],
    player: Pubkey,
    level: u8,
    slot: u64,
) -> Option<u8> {
    // Find all locked items (indices 40-79 not set in bitmask)
    let locked: Vec<u8> = (40..80)
        .filter(|&i| !is_bit_set(unlocked_items, i))
        .collect();

    if locked.is_empty() {
        return None;
    }

    // PRNG: hash(player || level || slot) mod locked.len()
    let seed = hash(&[player.as_ref(), &[level], &slot.to_le_bytes()].concat());
    let index = u64::from_le_bytes(seed[0..8].try_into().unwrap()) % locked.len() as u64;

    Some(locked[index as usize])
}
```

---

### 9. Basic Pickaxe Implementation

**Decision**: Add Basic Pickaxe as a special item with ID `T-XX-00` that is auto-equipped on session start.

**Rationale**:

- User spec: "starter tool... only gives him 1 ATK... can't be found on the map"
- Separate ID prefix (`XX`) distinguishes from droppable items
- Common rarity, no tags, simple 1 ATK effect

**Alternatives Considered**:

- Virtual item (not in registry): Complicates inventory logic
- Regular item with drop rate 0: Could still be generated by bugs

**Implementation Notes**:

```rust
pub const BASIC_PICKAXE: ItemDefinition = ItemDefinition {
    id: b"T-XX-00\0",
    name: "Basic Pickaxe",
    item_type: ItemType::Tool,
    tag: ItemTag::None,  // New tag variant needed
    rarity: Rarity::Common,
    effects: &[EffectDefinition::new(
        TriggerType::BattleStart,
        EffectType::GainAtk,
        false,
        [1, 1, 1], // 1 ATK at all tiers (unfusable)
    )],
};
```

---

### 10. SOL Transfer to Burner Wallet

**Decision**: `start_session` instruction accepts `burner_wallet` account and transfers specified lamports.

**Rationale**:

- User spec: "send SOL to burner wallet... signs transactions"
- Simple system transfer within instruction
- Amount specified as instruction parameter

**Alternatives Considered**:

- Fixed amount: Less flexible
- Separate transfer instruction: Not atomic

**Implementation Notes**:

```rust
pub fn start_session(
    ctx: Context<StartSession>,
    campaign_level: u8,
    burner_lamports: u64,
) -> Result<()> {
    // ... session creation logic ...

    // Transfer SOL to burner wallet
    let transfer_ix = anchor_lang::solana_program::system_instruction::transfer(
        &ctx.accounts.player.key(),
        &ctx.accounts.burner_wallet.key(),
        burner_lamports,
    );
    anchor_lang::solana_program::program::invoke(
        &transfer_ix,
        &[
            ctx.accounts.player.to_account_info(),
            ctx.accounts.burner_wallet.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    Ok(())
}
```

---

### 11. Bitmask Operations for Item Tracking

**Decision**: Use `[u8; 10]` bitmask (80 bits) for `unlocked_items` and `active_item_pool`.

**Rationale**:

- Compact: 10 bytes vs 80 bools (80 bytes)
- Efficient: Bitwise operations are fast
- Standard pattern for feature flags

**Alternatives Considered**:

- Vec<u8> of item indices: Variable size, harder to validate
- HashMap: Not serializable in Anchor

**Implementation Notes**:

```rust
// In bitmask.rs
pub fn is_bit_set(mask: [u8; 10], index: u8) -> bool {
    let byte_idx = (index / 8) as usize;
    let bit_idx = index % 8;
    mask[byte_idx] & (1 << bit_idx) != 0
}

pub fn set_bit(mask: &mut [u8; 10], index: u8) {
    let byte_idx = (index / 8) as usize;
    let bit_idx = index % 8;
    mask[byte_idx] |= 1 << bit_idx;
}

pub fn count_bits(mask: [u8; 10]) -> u8 {
    mask.iter().map(|b| b.count_ones() as u8).sum()
}

// Starter items: indices 0-39
pub const STARTER_ITEMS_BITMASK: [u8; 10] = [
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // bits 0-39 set
    0x00, 0x00, 0x00, 0x00, 0x00, // bits 40-79 clear
];
```

---

### 12. POI Position Validation

**Decision**: `interact_poi` instruction validates `player.position == poi.position` before allowing interaction.

**Rationale**:

- User spec: "check to look if the player is on a tile with the POI"
- Prevents remote interaction exploits
- Simple coordinate comparison

**Implementation Notes**:

```rust
pub fn interact_poi(ctx: Context<InteractPoi>, poi_index: u8) -> Result<()> {
    let game_state = &ctx.accounts.game_state;
    let map_pois = &ctx.accounts.map_pois;

    let poi = map_pois.pois.get(poi_index as usize)
        .ok_or(PoiError::InvalidPoiIndex)?;

    require!(
        game_state.position_x == poi.x && game_state.position_y == poi.y,
        PoiError::PlayerNotOnPoiTile
    );

    // ... interaction logic ...
}
```

---

## Summary

All technical unknowns have been resolved. Key architectural decisions:

1. **Transaction bundling** (not CPI) for atomic session creation
2. **Inline combat** in movement instruction for atomicity
3. **Chebyshev distance** for night enemy detection
4. **Deterministic PRNG** for random item unlocks
5. **Bitmask storage** for item tracking (10 bytes for 80 items)
6. **PDA seeds include level** for multi-session support
