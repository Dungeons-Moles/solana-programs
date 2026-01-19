# Feature Specification: Combat System & Status Effects

**Feature Branch**: `003-combat-system`
**Created**: 2026-01-18
**Status**: Draft
**Input**: User description: "Combat + Status Effects"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Basic Auto-Combat Resolution (Priority: P1)

When a player moves onto a tile containing an enemy, combat automatically begins and resolves turn-by-turn until one combatant is defeated. The player watches the battle unfold with each combatant taking actions based on their speed stat.

**Why this priority**: Core gameplay loop depends on combat resolution. Without this, players cannot progress through the dungeon or earn gold from defeating enemies.

**Independent Test**: Can be fully tested by spawning a player and enemy on adjacent tiles, having the player move onto the enemy tile, and verifying combat resolves with a winner determined.

**Acceptance Scenarios**:

1. **Given** a player with ATK 3, ARM 0, SPD 2 and an enemy with HP 6, ATK 1, ARM 1, SPD 1, **When** combat initiates, **Then** the player acts first each turn (higher SPD) and deals 2 damage per hit (ATK minus enemy ARM)
2. **Given** a player and enemy with equal SPD, **When** combat initiates, **Then** the enemy acts first (deterministic tie-breaker)
3. **Given** combat where both combatants deal 0 damage to each other, **When** turn 25 is reached, **Then** sudden death begins with both gaining +1 ATK per turn
4. **Given** combat still unresolved at turn 50, **When** failsafe triggers, **Then** the combatant with higher remaining HP percentage wins (ties favor enemy)

---

### User Story 2 - Status Effect Application (Priority: P2)

During combat, items and abilities can apply status effects (Chill, Shrapnel, Rust, Bleed) that modify combatant behavior and deal damage over time. Status effects stack and decay according to their specific rules.

**Why this priority**: Status effects create build diversity and strategic depth. Items become meaningless without their effects being applied in combat.

**Independent Test**: Can be tested by equipping a player with a Chill-applying item, entering combat, and verifying Chill reduces enemy strikes and decays properly.

**Acceptance Scenarios**:

1. **Given** a combatant with 2 Chill stacks, **When** their turn starts, **Then** their strikes this turn are reduced by 2 (minimum 1 strike) and 1 Chill stack is removed at turn end
2. **Given** a combatant with 3 Shrapnel stacks who gets struck, **When** damage is dealt to them, **Then** the attacker takes 3 damage and Shrapnel clears at turn end
3. **Given** a combatant with 2 Rust stacks and 5 Armor, **When** their turn ends, **Then** they lose 2 Armor (to 3) and Rust stacks persist
4. **Given** a combatant with 3 Bleed stacks, **When** their turn ends, **Then** they take 3 damage and 1 Bleed stack is removed

---

### User Story 3 - Combat Trigger System (Priority: P3)

Items and abilities have trigger conditions (Battle Start, Turn Start, First Turn, Every Other Turn, Exposed, Wounded) that activate their effects at specific moments during combat.

**Why this priority**: Triggers enable the item system to function correctly. Without proper trigger timing, item synergies and builds cannot work as designed.

**Independent Test**: Can be tested by equipping items with different trigger conditions and verifying they activate at the correct combat phases.

**Acceptance Scenarios**:

1. **Given** a player with an item that grants +5 Armor on "Battle Start", **When** combat initiates, **Then** the player gains 5 Armor before Turn 1 begins
2. **Given** a player with an item that heals 2 HP on "Turn Start", **When** each combat turn begins, **Then** the player heals 2 HP
3. **Given** a combatant with 0 Armor and an item that triggers when "Exposed", **When** their Armor reaches 0, **Then** the Exposed effect activates
4. **Given** a combatant below 50% max HP with an item that triggers when "Wounded", **When** they become Wounded, **Then** the Wounded effect activates
5. **Given** an item that triggers "Every Other Turn", **When** turns 2, 4, 6, etc. occur, **Then** the effect activates on those turns only

---

### User Story 4 - Multi-Strike Combat (Priority: P4)

Some tools and items grant multiple strikes per turn. Each strike is resolved separately, applying damage and triggering "On Hit" effects independently.

**Why this priority**: Multi-strike is a core mechanic for SCOUT tag builds and creates distinct playstyles. Required for items like Twin Picks and Pneumatic Drill.

**Independent Test**: Can be tested by equipping a 2-strike tool, entering combat, and verifying both strikes deal damage and trigger effects separately.

**Acceptance Scenarios**:

1. **Given** a player with a tool granting 2 strikes and ATK 2 vs enemy with ARM 1 and HP 10, **When** the player's turn occurs, **Then** they deal 1 damage twice (total 2 damage)
2. **Given** a player with 2 strikes and an "On Hit (once/turn)" effect, **When** both strikes land, **Then** the On Hit effect triggers only once
3. **Given** a player with Chill reducing strikes by 1, **When** they have 3 base strikes, **Then** they make 2 strikes that turn

---

### User Story 5 - Deterministic Combat Resolution (Priority: P5)

All combat must be fully deterministic given the same inputs (player stats, enemy stats, items equipped). This enables replay verification and prevents manipulation.

**Why this priority**: On-chain verification requires deterministic outcomes. Any randomness would make the game unfair or exploitable.

**Independent Test**: Can be tested by running the same combat configuration multiple times and verifying identical outcomes.

**Acceptance Scenarios**:

1. **Given** identical combat inputs (stats, items, status effects), **When** combat resolves multiple times, **Then** the outcome is always identical
2. **Given** a combat log, **When** replayed with the same initial state, **Then** each turn produces the same actions and damage values

---

### Edge Cases

- What happens when a combatant's HP reaches exactly 0? (They are defeated immediately)
- How does the system handle negative damage values? (Clamped to 0, no healing from attacks)
- What happens if Chill reduces strikes below 1? (Minimum 1 strike always)
- What happens when Armor goes negative from Rust? (Clamped to 0)
- How are simultaneous effects ordered? (Defined priority: status application before damage, damage before status decay)
- What happens if both combatants die on the same turn? (Attacker's damage resolves first; if defender dies, combat ends)

## Requirements *(mandatory)*

### Functional Requirements

#### Core Combat

- **FR-001**: System MUST resolve combat turn-by-turn with higher SPD combatant acting first
- **FR-002**: System MUST use enemy-wins-ties rule when SPD is equal
- **FR-003**: System MUST calculate weapon damage as `max(0, attacker_ATK - target_ARM)`
- **FR-004**: System MUST treat non-weapon damage as ignoring Armor unless specified otherwise
- **FR-005**: System MUST end combat when either combatant's HP reaches 0 or below
- **FR-006**: System MUST implement sudden death starting at Turn 25 (+1 ATK per turn to both combatants)
- **FR-007**: System MUST implement failsafe at Turn 50 (higher HP% wins, ties favor enemy)

#### Status Effects

- **FR-008**: System MUST implement Chill: reduces holder's strikes by stack count (min 1 strike), removes 1 stack at turn end
- **FR-009**: System MUST implement Shrapnel: deals stack damage to attacker when struck, clears at turn end
- **FR-010**: System MUST implement Rust: reduces Armor by stack count at turn end, stacks persist
- **FR-011**: System MUST implement Bleed: deals stack damage at turn end, removes 1 stack at turn end

#### Combat Triggers

- **FR-012**: System MUST support "Battle Start" trigger (before Turn 1)
- **FR-013**: System MUST support "First Turn" trigger (during Turn 1 only)
- **FR-014**: System MUST support "Turn Start" trigger (start of each turn)
- **FR-015**: System MUST support "Every Other Turn" trigger (turns 2, 4, 6, ...)
- **FR-016**: System MUST support "Exposed" condition trigger (when combatant has 0 Armor)
- **FR-017**: System MUST support "Wounded" condition trigger (when HP below 50% max HP)
- **FR-018**: System MUST support "On Hit (once/turn)" trigger for strike-based effects

#### Multi-Strike

- **FR-019**: System MUST allow combatants to have multiple strikes per turn
- **FR-020**: System MUST resolve each strike independently for damage calculation
- **FR-021**: System MUST respect "once/turn" limits for On Hit effects across multiple strikes

#### Determinism

- **FR-022**: System MUST produce identical outcomes for identical inputs
- **FR-023**: System MUST NOT use any randomness in combat resolution
- **FR-024**: System MUST define explicit ordering for simultaneous effects

### Key Entities

- **Combatant**: Represents either player or enemy in combat. Has HP, ATK, ARM, SPD, DIG stats. Can have status effects and equipped items.
- **Combat State**: Tracks current turn number, both combatants' current stats, active status effects, and applied modifiers.
- **Status Effect**: A named effect (Chill, Shrapnel, Rust, Bleed) with a stack count that modifies combat behavior.
- **Combat Trigger**: A condition (Battle Start, Turn Start, etc.) that activates item or ability effects when met.
- **Strike**: A single attack action within a turn. A combatant may have multiple strikes per turn.
- **Combat Log**: A record of all actions taken during combat for replay and verification.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Combat between any valid combatant configurations resolves to a deterministic winner within 50 turns
- **SC-002**: All 4 status effects (Chill, Shrapnel, Rust, Bleed) function correctly according to their specifications
- **SC-003**: All 7 trigger types (Battle Start, First Turn, Turn Start, Every Other Turn, Exposed, Wounded, On Hit) activate at correct times
- **SC-004**: Combat with identical inputs produces identical outputs 100% of the time (determinism verification)
- **SC-005**: Multi-strike attacks correctly apply damage and respect once-per-turn effect limits
- **SC-006**: Sudden death and failsafe mechanics correctly resolve otherwise-stalemate combats

## Assumptions

- Combat stats (HP, ATK, ARM, SPD, DIG) are provided by the calling system (player profile, enemy data)
- Item effects and their triggers are defined in a separate item system and passed to combat as modifiers
- Gold rewards for defeating enemies are handled by the calling system, not the combat system itself
- The player's equipped tool determines their base ATK and strike count
- Boss special abilities and phases will use the same trigger system as items

## Dependencies

- Player Profile system (provides player stats and equipped items)
- Item system (provides item effect definitions)
- Gameplay State Tracking (provides enemy spawns and combat context)

## Out of Scope

- Item effect definitions (separate feature)
- Enemy AI behavior beyond stat-based auto-combat
- Visual/audio combat presentation
- Gold and loot distribution after combat
- Boss phase transitions (will use combat triggers but boss definitions are separate)
