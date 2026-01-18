# Research: Gameplay State Tracking

**Branch**: `002-gameplay-state-tracking` | **Date**: 2025-01-17

## RD-001: GameState Account Design

**Question**: How should GameState be structured to meet the < 200 byte constraint while storing all required data?

**Decision**: Use compact representation with u8/i8 for small values and efficient Phase enum.

**Rationale**:
- Position: 2 × u8 (x, y) = 2 bytes - map is max 256×256
- Stats: 5 × i8 (HP, ATK, ARM, SPD, DIG) = 5 bytes - small signed values sufficient
- MaxHP: 1 × u8 = 1 byte
- GearSlots: 1 × u8 = 1 byte
- Week: 1 × u8 = 1 byte (values 1-3)
- Phase: 1 byte enum (6 variants)
- MovesRemaining: 1 × u8 = 1 byte (max 50)
- TotalMoves: 1 × u32 = 4 bytes (track across session)
- MapWidth/Height: 2 × u8 = 2 bytes
- Linked session: 32 bytes (Pubkey)
- Player: 32 bytes (Pubkey)
- BossFightReady: 1 byte (bool)
- Bump: 1 byte
- **Total**: ~85 bytes + 8 (discriminator) = ~93 bytes ✅ Well under 200 bytes

**Alternatives Considered**:
- u16 for stats: Rejected - stats never exceed ±127 in GDD
- Separate account for stats: Rejected - increases complexity and transaction costs

## RD-002: Session Linking Strategy

**Question**: How should GameState link to existing GameSession from session-manager?

**Decision**: Use session PDA as seed in GameState PDA derivation: `["game_state", session_pda]`

**Rationale**:
- Ensures 1:1 mapping between session and game state
- Validates session existence during GameState initialization
- Follows composition pattern from constitution (III. Program Composability)
- Does NOT require CPI - just uses session PDA as seed

**Alternatives Considered**:
- Store session reference in GameState: Still valid, but PDA seed provides stronger linkage
- CPI to session-manager: Rejected - adds complexity, session just needs to exist

## RD-003: Phase Enum Design

**Question**: How to represent the Day/Night cycle with proper move allowances?

**Decision**: Enum with 6 variants, each mapping to a move count.

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Day1,   // 50 moves
    Night1, // 30 moves
    Day2,   // 50 moves
    Night2, // 30 moves
    Day3,   // 50 moves
    Night3, // 30 moves
}

impl Phase {
    pub fn moves_allowed(&self) -> u8 {
        match self {
            Phase::Day1 | Phase::Day2 | Phase::Day3 => 50,
            Phase::Night1 | Phase::Night2 | Phase::Night3 => 30,
        }
    }

    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::Day1 => Some(Phase::Night1),
            Phase::Night1 => Some(Phase::Day2),
            Phase::Day2 => Some(Phase::Night2),
            Phase::Night2 => Some(Phase::Day3),
            Phase::Day3 => Some(Phase::Night3),
            Phase::Night3 => None, // Week ends
        }
    }
}
```

**Rationale**:
- Type-safe phase transitions
- Compile-time guarantee of valid phases
- Easy to add move allowance logic

**Alternatives Considered**:
- Two fields (day_number: u8, is_night: bool): Rejected - less type-safe, more error-prone
- Single u8 with magic numbers: Rejected - loses type safety

## RD-004: Movement Instruction Design

**Question**: How to handle the two movement types (floor vs wall digging)?

**Decision**: Single `move_player` instruction with `tile_type` parameter.

```rust
pub fn move_player(
    ctx: Context<MovePlayer>,
    target_x: u8,
    target_y: u8,
    is_wall: bool,  // Client reports tile type
) -> Result<()>
```

**Rationale**:
- Simpler than two instructions
- Client already knows tile type from map data
- Move cost calculated on-chain: `is_wall ? max(2, 6 - dig) : 1`
- Spec explicitly states: "Tile type is passed by the client; on-chain program trusts client for tile data"

**Alternatives Considered**:
- Store map on-chain: Rejected - out of scope, significant storage cost
- Two separate instructions (walk/dig): Rejected - unnecessary complexity

## RD-005: Stat Modification Authorization

**Question**: Who is authorized to modify player stats?

**Decision**: For this batch, only the player (session owner) can modify stats via a dedicated instruction.

**Rationale**:
- Combat and item effects are out of scope
- Future features can add CPI-based modification with proper authorization
- Keeps this batch simple and focused

**Future Consideration**: Add `authorized_modifier` field or use PDA-based authority pattern when combat/items are added.

## RD-006: Automatic Phase Advancement

**Question**: When and how should phase advancement occur?

**Decision**: Phase advances within the `move_player` instruction when `moves_remaining` reaches 0 after deduction.

**Rationale**:
- No separate "advance phase" instruction needed
- Atomically handles the transition
- Matches GDD behavior: phase ends when moves exhausted

**Implementation**:
```rust
// After move cost deduction
if state.moves_remaining == 0 {
    match state.phase.next() {
        Some(next_phase) => {
            state.phase = next_phase;
            state.moves_remaining = next_phase.moves_allowed();
            // Handle week transition if Night3 → Day1
            if matches!(state.phase, Phase::Day1) && state.week < 3 {
                state.week += 1;
                state.gear_slots = (state.gear_slots + 2).min(8);
            }
        }
        None => {
            // Night3 of current week complete
            if state.week == 3 {
                state.boss_fight_ready = true;
            } else {
                state.week += 1;
                state.phase = Phase::Day1;
                state.moves_remaining = 50;
                state.gear_slots = (state.gear_slots + 2).min(8);
            }
        }
    }
}
```

## RD-007: Error Handling Strategy

**Question**: What custom errors are needed?

**Decision**: Custom error enum covering all validation failures.

```rust
#[error_code]
pub enum GameplayStateError {
    #[msg("Target position is out of map boundaries")]
    OutOfBounds,
    #[msg("Not enough moves remaining for this action")]
    InsufficientMoves,
    #[msg("Target position is not adjacent to current position")]
    NotAdjacent,
    #[msg("Stat value would overflow")]
    StatOverflow,
    #[msg("HP cannot go below 0")]
    HpUnderflow,
    #[msg("Invalid stat modification")]
    InvalidStatModification,
    #[msg("Boss fight already triggered")]
    BossFightAlreadyTriggered,
    #[msg("Unauthorized: only session owner can modify state")]
    Unauthorized,
    #[msg("Session is not active")]
    SessionNotActive,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
```

**Rationale**: Follows constitution requirement for "custom Anchor error enums with descriptive messages" (IV. Anchor Framework).

## RD-008: Account Size Calculation

**Question**: Verify final account size meets SC-007 (< 200 bytes).

**Calculation**:
| Field | Type | Size |
|-------|------|------|
| Discriminator | - | 8 |
| player | Pubkey | 32 |
| session | Pubkey | 32 |
| position_x | u8 | 1 |
| position_y | u8 | 1 |
| map_width | u8 | 1 |
| map_height | u8 | 1 |
| hp | i8 | 1 |
| max_hp | u8 | 1 |
| atk | i8 | 1 |
| arm | i8 | 1 |
| spd | i8 | 1 |
| dig | i8 | 1 |
| gear_slots | u8 | 1 |
| week | u8 | 1 |
| phase | enum | 1 |
| moves_remaining | u8 | 1 |
| total_moves | u32 | 4 |
| boss_fight_ready | bool | 1 |
| bump | u8 | 1 |
| **Total** | | **92 bytes** |

**Result**: ✅ 92 bytes < 200 bytes - SC-007 satisfied.

## Summary

All technical decisions resolved. No NEEDS CLARIFICATION markers remain. Ready for Phase 1 design artifacts.
