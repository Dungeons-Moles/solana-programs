# Research: Field Enemies

**Feature**: 005-field-enemies  
**Date**: 2026-01-19  
**Status**: Complete

## Research Questions

### 1. Static vs Dynamic Enemy Data Storage

**Decision**: Static const arrays compiled into program

**Rationale**:

- Enemy archetypes and stats are immutable game data from GDD
- No runtime modification needed (balance changes require program upgrade)
- Avoids on-chain storage costs for read-only data
- Deterministic access pattern aligns with Solana compute model
- Existing `player-inventory` uses similar pattern for item definitions

**Alternatives Considered**:

- **On-chain config accounts**: Rejected - adds 12×3 = 36 accounts to manage, rent costs, no benefit since data doesn't change at runtime
- **External JSON loaded at build**: Rejected - complicates build process, Anchor IDL generation doesn't benefit

### 2. Enemy Instance State Management

**Decision**: Enemy instances stored as compact arrays within MapData account

**Rationale**:

- Per-map enemy state (position, tier, defeated flag) is session-scoped
- Compact representation: `struct EnemyInstance { archetype_id: u8, tier: u8, x: u8, y: u8, defeated: bool }` = 5 bytes
- 48 enemies max × 5 bytes = 240 bytes, fits easily in account limits
- Aligns with MagicBlock ephemeral state patterns (delegatable, session-scoped)

**Alternatives Considered**:

- **Separate PDA per enemy**: Rejected - 48 PDAs per map is excessive, CPI overhead
- **Bitmap for enemy positions**: Rejected - loses tier/archetype info, harder to query

### 3. Enemy Trait Implementation

**Decision**: Reuse existing `ItemEffect` struct from combat-system

**Rationale**:

- Enemy traits use identical trigger/effect patterns as items (BattleStart, OnHit, TurnStart, etc.)
- `ItemEffect { trigger: TriggerType, once_per_turn: bool, effect_type: EffectType, value: i16 }` handles all 12 enemy traits
- Existing `process_triggers_for_phase` function processes enemy effects without modification
- Reduces code duplication, ensures consistent combat behavior

**Alternatives Considered**:

- **Separate EnemyEffect struct**: Rejected - duplicates ItemEffect functionality
- **Trait callbacks/closures**: Rejected - not supported in Solana BPF, no dynamic dispatch

### 4. Tier Distribution Algorithm

**Decision**: Weighted sampling using seeded RNG with cumulative probability thresholds

**Rationale**:

- Existing `map-generator/rng.rs` provides seeded RNG compatible with deterministic replay
- Cumulative thresholds: Act 1 T1≤70, T2≤95, T3≤100 (sample 0-99, compare thresholds)
- Per-act distribution arrays as const data
- Matches GDD spec exactly while being deterministic from seed

**Alternatives Considered**:

- **Exact count enforcement**: Rejected - 70% of 36 = 25.2 enemies, rounding complicates determinism
- **Reservoir sampling**: Rejected - more complex, no benefit for small counts

### 5. Biome Weighting Implementation

**Decision**: Separate weight arrays per biome with weighted random selection

**Rationale**:

- Two biomes (A/B) with 4-5 emphasized archetypes each
- Base weight 1.0 for all, 2.0 for emphasized (GDD doesn't specify exact multiplier, using 2x)
- Normalize and sample using RNG
- Compact const arrays: `BIOME_A_WEIGHTS: [u8; 12]` and `BIOME_B_WEIGHTS: [u8; 12]`

**Alternatives Considered**:

- **Exclusion zones**: Rejected - GDD explicitly states all archetypes can appear
- **Per-enemy spawn probability tables**: Rejected - over-engineering for current scope

### 6. Combat Integration Pattern

**Decision**: Direct struct reuse, no CPI for enemy stat lookup

**Rationale**:

- Combat system accepts `CombatantInput` struct for enemy stats
- Field-enemies provides `get_enemy_combatant_input(archetype_id, tier)` helper
- No CPI needed since enemy data is statically linked
- Combat initiation instruction in gameplay-state builds enemy input from static data

**Alternatives Considered**:

- **CPI to field-enemies**: Rejected - unnecessary indirection for static data
- **Shared library crate**: Could work but Anchor workspace handles dependency correctly

### 7. Gold Reward Handling

**Decision**: Return Gold amount from combat resolution, gameplay-state applies

**Rationale**:

- Gold is player resource in GameState
- Combat system returns victory bool + resolution type
- gameplay-state reads tier from defeated enemy, looks up Gold (2/4/6), applies
- Matches existing stat modification pattern in gameplay-state

**Alternatives Considered**:

- **Combat system applies Gold directly**: Rejected - violates single-responsibility, needs GameState access
- **Event emission for off-chain tallying**: Rejected - Gold is on-chain state per GDD

## Countdown Trait Special Case

**Decision**: Add Countdown handling to combat-system engine

**Rationale**:

- Powder Tick uses `Countdown(2)` trigger not currently in `TriggerType`
- Add `Countdown` variant to TriggerType with countdown value in ItemEffect
- Engine tracks countdown per effect, decrements each turn, fires at 0
- Affects only combat-system, not field-enemies

**Implementation Notes**:

- Extend `ItemEffect` with optional `countdown: Option<u8>` field
- Add `CountdownState` to CombatState tracking active countdowns
- Process countdown decrements in turn loop before other triggers

## Dependencies Confirmed

| Dependency             | Status  | Notes                                        |
| ---------------------- | ------- | -------------------------------------------- |
| combat-system triggers | EXISTS  | TriggerType, EffectType, ItemEffect ready    |
| combat-system effects  | EXISTS  | Chill/Shrapnel/Rust/Bleed implemented        |
| map-generator RNG      | EXISTS  | Seeded deterministic random                  |
| gameplay-state         | EXISTS  | GameState with player stats, Gold implicit   |
| MagicBlock SDK         | STUBBED | Delegation patterns documented, not blocking |

## Open Items

None. All research questions resolved.
