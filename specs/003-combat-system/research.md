# Research: Combat System & Status Effects

**Feature**: 003-combat-system
**Date**: 2026-01-18

## Research Topics

### 1. Combat Resolution Architecture

**Question**: Should combat resolve in a single transaction or span multiple transactions?

**Decision**: Single transaction with iterative loop

**Rationale**:
- Solana compute limits (~200k CU default, up to 1.4M with request) are sufficient for turn-based combat
- Single transaction ensures atomicity - either combat completes fully or not at all
- Simpler state management - no need to persist mid-combat state
- Maximum 50 turns × ~2000 CU per turn = ~100k CU (well within limits)

**Alternatives Considered**:
- Multi-transaction saga: Rejected due to complexity of handling interruptions and partial state
- Off-chain computation with on-chain verification: Rejected as overkill for deterministic combat

### 2. Status Effect Storage

**Question**: How to efficiently store and process status effects?

**Decision**: Bitfield for effect types + array of stack counts

**Rationale**:
- Only 4 status effects (Chill, Shrapnel, Rust, Bleed)
- Each effect needs stack count (u8 sufficient, max ~255 stacks)
- Bitfield for quick "has effect" checks
- Fixed-size array for predictable account space

**Data Structure**:
```rust
pub struct StatusEffects {
    pub chill: u8,      // Stack count
    pub shrapnel: u8,   // Stack count
    pub rust: u8,       // Stack count
    pub bleed: u8,      // Stack count
}
```

**Alternatives Considered**:
- HashMap/BTreeMap: Rejected due to variable size and allocation overhead
- Single u32 with packed bits: Rejected as insufficient for stack counts

### 3. Trigger System Design

**Question**: How to implement the 7 trigger types efficiently?

**Decision**: Enum-based trigger evaluation with combat phase hooks

**Rationale**:
- Triggers are well-defined moments in combat flow
- Combat engine calls trigger hooks at appropriate phases
- Item effects registered as trigger handlers
- Evaluation order is deterministic (by item slot order)

**Trigger Phases in Combat Loop**:
```
1. Battle Start (once, before Turn 1)
2. For each turn:
   a. Turn Start (both combatants)
   b. Check Exposed/Wounded conditions
   c. First Turn check (Turn 1 only)
   d. Every Other Turn check
   e. Attacker strikes (On Hit triggers)
   f. Defender Shrapnel retaliation
   g. End of turn (status decay)
3. Combat End
```

**Alternatives Considered**:
- Event-driven system: Rejected as more complex and harder to make deterministic
- Pre-computed effect schedule: Rejected as doesn't handle conditional triggers well

### 4. Multi-Strike Implementation

**Question**: How to handle multiple strikes per turn with once-per-turn limits?

**Decision**: Strike loop with trigger tracking flags

**Rationale**:
- Combatant has `strikes_per_turn` field (default 1, modified by items)
- Each strike is processed independently
- Maintain `once_per_turn_triggered` flags that reset at turn end
- Clear separation between "per strike" and "per turn" effects

**Implementation Pattern**:
```rust
for strike in 0..combatant.strikes_per_turn {
    // Calculate damage
    let damage = max(0, attacker.atk - defender.arm);
    defender.hp -= damage;

    // Process On Hit effects (check once-per-turn flags)
    if !triggered_this_turn[ON_HIT_EFFECT_ID] {
        apply_on_hit_effect();
        triggered_this_turn[ON_HIT_EFFECT_ID] = true;
    }

    // Process Shrapnel retaliation
    if defender.status.shrapnel > 0 {
        attacker.hp -= defender.status.shrapnel;
    }
}
```

### 5. Determinism Verification

**Question**: How to guarantee and verify deterministic combat?

**Decision**: Combat log with hash verification

**Rationale**:
- Combat produces a log of all actions (turn, actor, action, values)
- Log can be hashed to create a "combat signature"
- Same inputs must always produce same signature
- Log enables replay verification

**Verification Approach**:
- Store initial state hash + final state hash
- Optionally emit turn-by-turn events for indexing
- Integration tests run same combat multiple times to verify

### 6. Integration with gameplay-state

**Question**: How does combat-system integrate with the existing gameplay-state program?

**Decision**: CPI from gameplay-state to combat-system

**Rationale**:
- gameplay-state tracks player position and triggers combat on enemy tiles
- combat-system receives combatant data, resolves combat, returns result
- GameState stores combat outcome (won/lost, HP remaining)
- Clean separation of concerns

**Integration Flow**:
```
1. Player moves to enemy tile (gameplay-state)
2. gameplay-state invokes combat-system via CPI
3. combat-system resolves combat, emits result
4. gameplay-state updates HP, handles rewards/death
```

### 7. Compute Budget Analysis

**Question**: Will combat fit within Solana compute limits?

**Decision**: Yes, with optimization

**Rationale**:
- Basic damage calculation: ~100 CU
- Status effect processing: ~200 CU per effect
- Trigger evaluation: ~300 CU per trigger
- Per-turn estimate: ~1500-2500 CU
- Maximum turns: 50
- Total estimate: ~75k-125k CU (well under 200k limit)

**Optimization Strategies**:
- Early exit on combat end
- Lazy status effect evaluation (only if stacks > 0)
- Bitfield checks for trigger eligibility
- No heap allocations in hot loop

## Decisions Summary

| Topic | Decision | Impact |
|-------|----------|--------|
| Architecture | Single-transaction resolution | Simplicity, atomicity |
| Status Storage | Fixed-size struct with u8 counts | Predictable space, efficiency |
| Triggers | Enum + phase hooks | Deterministic, clear flow |
| Multi-Strike | Loop with tracking flags | Correct once-per-turn behavior |
| Determinism | Log + hash verification | Verifiable outcomes |
| Integration | CPI from gameplay-state | Clean separation |
| Compute | ~125k CU max | Within limits |
