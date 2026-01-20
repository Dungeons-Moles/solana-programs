# Field Enemies - Anchor IDL Contract

**Feature**: 005-field-enemies  
**Date**: 2026-01-19

## Program ID

```
field_enemies = "TODO_GENERATE_NEW_KEYPAIR"
```

## Instructions

### initialize_map_enemies

Creates the MapEnemies PDA for a session and spawns enemies based on act/biome.

```rust
#[derive(Accounts)]
pub struct InitializeMapEnemies<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Session PDA from session-manager
    pub session: Account<'info, GameSession>,

    /// Game state for act/biome context
    pub game_state: Account<'info, GameState>,

    /// Map config for seed lookup
    pub map_config: Account<'info, MapConfig>,

    #[account(
        init,
        payer = payer,
        space = 8 + MapEnemies::INIT_SPACE,
        seeds = [b"map_enemies", session.key().as_ref()],
        bump
    )]
    pub map_enemies: Account<'info, MapEnemies>,

    pub system_program: Program<'info, System>,
}

pub struct InitializeMapEnemiesArgs {
    pub act: u8,        // 1-4
    pub level: u8,      // 0-80
}
```

**Returns**: `()`

**Errors**:

- `InvalidAct` - act not in 1-4 range
- `SessionNotActive` - session not in active state
- `MapEnemiesAlreadyExists` - PDA already initialized

---

### get_enemy_at_position

View function to check if enemy exists at position (off-chain helper).

```rust
// No on-chain instruction needed - client reads MapEnemies directly
// Helper function in SDK:
pub fn get_enemy_at_position(
    map_enemies: &MapEnemies,
    x: u8,
    y: u8,
) -> Option<&EnemyInstance>;
```

---

### mark_enemy_defeated

Called by gameplay-state after combat victory to mark enemy as defeated.

```rust
#[derive(Accounts)]
pub struct MarkEnemyDefeated<'info> {
    /// Must be gameplay_state program
    pub caller: Signer<'info>,

    #[account(mut)]
    pub map_enemies: Account<'info, MapEnemies>,

    pub game_state: Account<'info, GameState>,
}

pub struct MarkEnemyDefeatedArgs {
    pub x: u8,
    pub y: u8,
}
```

**Returns**: `EnemyTier` (for Gold calculation)

**Errors**:

- `EnemyNotFound` - no enemy at position
- `EnemyAlreadyDefeated` - enemy already marked defeated
- `UnauthorizedCaller` - caller not gameplay_state program

---

### get_enemy_combatant_input

Compute-only helper to build CombatantInput for combat-system.

```rust
// Pure function, no account access
pub fn get_enemy_combatant_input(
    archetype_id: u8,
    tier: EnemyTier,
) -> CombatantInput;
```

---

### get_enemy_traits

Compute-only helper to get enemy traits for combat processing.

```rust
// Pure function, no account access
pub fn get_enemy_traits(
    archetype_id: u8,
) -> &'static [ItemEffect];
```

## Accounts

### MapEnemies

```rust
#[account]
#[derive(InitSpace)]
pub struct MapEnemies {
    /// Parent session PDA
    pub session: Pubkey,

    /// Enemy instances (max 48)
    #[max_len(48)]
    pub enemies: Vec<EnemyInstance>,

    /// Actual count of enemies
    pub count: u8,

    /// PDA bump
    pub bump: u8,
}
```

### EnemyInstance

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct EnemyInstance {
    pub archetype_id: u8,
    pub tier: u8,
    pub x: u8,
    pub y: u8,
    pub defeated: bool,
}
```

## Types (Re-exported from combat-system)

```rust
// From combat-system, re-exported for convenience
pub use combat_system::state::{
    CombatantInput,
    ItemEffect,
    TriggerType,
    EffectType,
    StatusEffects,
};
```

## Errors

```rust
#[error_code]
pub enum FieldEnemiesError {
    #[msg("Invalid act number, must be 1-4")]
    InvalidAct,

    #[msg("Session is not in active state")]
    SessionNotActive,

    #[msg("MapEnemies account already initialized")]
    MapEnemiesAlreadyExists,

    #[msg("No enemy found at specified position")]
    EnemyNotFound,

    #[msg("Enemy at position already defeated")]
    EnemyAlreadyDefeated,

    #[msg("Caller not authorized")]
    UnauthorizedCaller,

    #[msg("Invalid archetype ID")]
    InvalidArchetypeId,

    #[msg("Invalid tier value")]
    InvalidTier,
}
```

## Events

```rust
#[event]
pub struct EnemiesSpawned {
    pub session: Pubkey,
    pub count: u8,
    pub act: u8,
}

#[event]
pub struct EnemyDefeated {
    pub session: Pubkey,
    pub archetype_id: u8,
    pub tier: u8,
    pub x: u8,
    pub y: u8,
    pub gold_reward: u8,
}
```

## CPI Interface

### For gameplay-state to call field-enemies

```rust
// CPI context for marking enemy defeated
pub fn cpi_mark_enemy_defeated<'info>(
    ctx: CpiContext<'_, '_, '_, 'info, MarkEnemyDefeated<'info>>,
    x: u8,
    y: u8,
) -> Result<u8>; // Returns tier for Gold calculation
```

### For combat-system integration (no CPI needed)

```rust
// Direct imports for static enemy data
use field_enemies::archetypes::{
    get_enemy_stats,
    get_enemy_traits,
    ENEMY_ARCHETYPES,
};
```
