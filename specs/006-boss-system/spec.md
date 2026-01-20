# Feature Specification: Boss System

**Feature Branch**: `006-boss-system`
**Created**: 2026-01-19
**Status**: Draft
**Input**: User description: "for bosses"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Boss Definition Registry (Priority: P1)

The game needs a complete registry of all 24 boss definitions with their stats, traits, and weakness tags. Each boss has unique mechanics that create varied combat experiences across the campaign.

**Why this priority**: Without boss definitions, no boss combat can occur. This is the foundational data layer.

**Independent Test**: Can be tested by querying any boss by ID and verifying its stats, traits, and weakness tags match the GDD specifications.

**Acceptance Scenarios**:

1. **Given** a boss ID like "B-A-W1-01", **When** querying the boss registry, **Then** returns The Broodmother with HP=32, ATK=2, ARM=2, SPD=3, DIG=1, weaknesses=[STONE, FROST]
2. **Given** any of the 24 boss IDs, **When** queried, **Then** returns complete definition including all traits
3. **Given** an invalid boss ID, **When** queried, **Then** returns an error indicating boss not found

---

### User Story 2 - Boss Selection by Stage (Priority: P1)

The game needs to deterministically select which boss appears at each stage based on act, week, and stage number within the act. This ensures consistent campaign progression.

**Why this priority**: Boss selection is required for campaign flow - players need to know which boss they'll face.

**Independent Test**: Given any stage number (1-80), can determine exactly which boss appears for each of the 3 weekly boss fights.

**Acceptance Scenarios**:

1. **Given** Act 1 Stage 1 Week 1, **When** determining boss, **Then** selects B-A-W1-01 (The Broodmother)
2. **Given** Act 2 Stage 21 Week 1, **When** determining boss, **Then** selects a Biome B variant with +1 SPD baseline
3. **Given** any stage with odd number in act for Week 3, **When** determining boss, **Then** selects Final 1 for that biome
4. **Given** any stage with even number in act for Week 3, **When** determining boss, **Then** selects Final 2 for that biome

---

### User Story 3 - Boss Stat Scaling (Priority: P2)

Boss stats scale based on act and stage progression. Acts 3 and 4 (A+ and B+) apply additional modifiers to increase difficulty.

**Why this priority**: Scaling ensures difficulty progression across the 80-stage campaign, but basic boss combat works without it.

**Independent Test**: Can verify stat calculations for any boss at any stage produce correct scaled values.

**Acceptance Scenarios**:

1. **Given** a Week 1 boss at Act 3 Stage 43 (tier=0), **When** calculating stats, **Then** adds +1 ATK baseline (Act 3 modifier)
2. **Given** a Week 2 boss at Act 3 Stage 55 (tier=2), **When** calculating stats, **Then** adds +3 HP×2, +1 ARM×2, +1 ATK (tier modifier) plus +1 ATK baseline
3. **Given** a Week 3 final at Act 4 Stage 80 (tier=3), **When** calculating stats, **Then** adds +4 HP×3, +1 ARM×3, +1 ATK (tier>=1) plus +2 ATK +1 SPD baseline

---

### User Story 4 - Boss Trait Execution (Priority: P2)

Boss traits are special abilities that trigger during combat at specific moments (Battle Start, Turn Start, On Hit, etc.). Each boss has 2-3 unique traits.

**Why this priority**: Traits make bosses unique and challenging, but basic damage-dealing combat can work without complex trait mechanics.

**Independent Test**: Can execute a boss combat and verify each trait triggers at the correct moment with correct effects.

**Acceptance Scenarios**:

1. **Given** combat with The Broodmother, **When** combat begins, **Then** Swarm Queen trait grants 3 strikes per turn
2. **Given** combat with Obsidian Golem, **When** any turn starts, **Then** Hardened Core trait grants +4 Armor
3. **Given** combat with Powder Keg Baron with Countdown(3), **When** 3 turns pass, **Then** Volatile Countdown deals 10 non-weapon damage to both combatants
4. **Given** combat with Crystal Mimic with reflection stacks, **When** player applies status effect, **Then** effect reflects back to player and stack decrements

---

### User Story 5 - Boss Phase Mechanics (Priority: P2)

Week 3 final bosses have phase-based mechanics that activate at HP thresholds (75%, 50%, 25%), changing combat dynamics.

**Why this priority**: Phase mechanics add depth to final boss fights but are only needed for 4 specific bosses.

**Independent Test**: Can run Week 3 final combat and verify phase transitions occur at correct HP thresholds with correct effects.

**Acceptance Scenarios**:

1. **Given** The Eldritch Mole at 100% HP, **When** HP drops to 75%, **Then** Phase 1 activates (+10 Armor, reduced by 10 if player DIG > boss DIG)
2. **Given** The Eldritch Mole at 74% HP, **When** HP drops to 50%, **Then** Phase 2 activates (attacks twice per turn)
3. **Given** The Eldritch Mole at 49% HP, **When** HP drops to 25%, **Then** Phase 3 activates (Turn Start apply 2 Bleed)
4. **Given** The Gilded Devourer wounded, **When** at any HP threshold, **Then** Hunger trait applies 3 Bleed to player

---

### User Story 6 - Boss Weakness Integration (Priority: P3)

Each boss has 2 weakness tags that affect item offer generation. Items matching weakness tags are weighted 1.4x higher when generating offers during that boss's week.

**Why this priority**: Weakness integration enhances gameplay strategy but the item system already handles tag weighting; this just provides the data.

**Independent Test**: Can query any boss and retrieve its 2 weakness tags for use by the item offer system.

**Acceptance Scenarios**:

1. **Given** The Broodmother is current week boss, **When** generating item offers, **Then** STONE and FROST tagged items have 1.4x weight
2. **Given** a stage in Week 2, **When** querying boss weaknesses, **Then** returns the Week 2 boss's weakness tags
3. **Given** Week 3 exploration, **When** querying boss weaknesses, **Then** returns the Week 3 final boss's weakness tags

---

### Edge Cases

- What happens when boss HP reaches exactly a phase threshold? Phase activates immediately before next action.
- How does Countdown interact with combat ending early? Countdown damage occurs at end of countdown turn regardless of other damage.
- What happens if reflection stacks are depleted mid-turn? Subsequent status applications proceed normally.
- How do multiple traits with same trigger interact? Execute in definition order (first trait, then second).
- What if boss stat scaling would exceed reasonable bounds? No explicit cap; tuning handled by GDD values.

## Requirements *(mandatory)*

### Functional Requirements

**Boss Registry**
- **FR-001**: System MUST store 24 boss definitions as compile-time constants (12 Biome A + 12 Biome B variants/finals)
- **FR-002**: Each boss definition MUST include: ID, name, emoji, base stats (HP/ATK/ARM/SPD/DIG), weakness tags (2), and traits (2-3)
- **FR-003**: System MUST provide lookup function to retrieve boss by ID
- **FR-004**: System MUST provide function to list all bosses for a given biome and week

**Boss Selection**
- **FR-005**: System MUST determine boss for any stage (1-80) based on act, week, and stage-within-act
- **FR-006**: Week 1 bosses MUST cycle through 5 archetypes in order within each act
- **FR-007**: Week 2 bosses MUST cycle through 5 archetypes with offset to avoid repeating Week 1 pairing
- **FR-008**: Week 3 finals MUST alternate: odd stage = Final 1, even stage = Final 2
- **FR-009**: Biome B bosses (Acts 2, 4) MUST use Biome B variants with +1 SPD baseline (cap 4)

**Stat Scaling**
- **FR-010**: System MUST calculate tier from stage: `tier = floor((stageInAct - 1) / 5)` where tier = 0..3
- **FR-011**: Week 1 boss scaling MUST apply: +2 HP×tier, +1 ARM×tier
- **FR-012**: Week 2 boss scaling MUST apply: +3 HP×tier, +1 ARM×tier, +1 ATK at tier>=2
- **FR-013**: Week 3 final scaling MUST apply: +4 HP×tier, +1 ARM×tier, +1 ATK at tier>=1
- **FR-014**: Act 3 (A+) MUST add: Week 1/2 +1 ATK baseline; Week 3 +2 ATK baseline
- **FR-015**: Act 4 (B+) MUST add: Week 1/2 +1 ATK +1 SPD baseline; Week 3 +2 ATK +1 SPD baseline

**Trait System**
- **FR-016**: Each trait MUST specify trigger type (BattleStart, TurnStart, OnHit, EveryOtherTurn, Wounded, FirstTurn, Countdown)
- **FR-017**: Traits with "once per turn" MUST only activate once per turn
- **FR-018**: Countdown traits MUST track remaining turns and execute effect when countdown reaches 0
- **FR-019**: Reflection traits MUST track remaining stacks and reflect status applications back to player
- **FR-020**: Phase traits MUST activate at specified HP thresholds (75%, 50%, 25%)

**Combat Integration**
- **FR-021**: System MUST convert boss definition + scaling into CombatantInput format for combat system
- **FR-022**: Boss traits MUST generate ItemEffect arrays compatible with combat system
- **FR-023**: System MUST provide boss weakness tags for item offer generation

**Trait Definitions (All 24 Bosses)**
- **FR-024**: The Broodmother MUST have: Swarm Queen (3 strikes/turn), Webbed Strikes (every other turn first strike applies 1 Chill)
- **FR-025**: Obsidian Golem MUST have: Hardened Core (Turn Start +4 ARM), Cracked Shell (non-weapon damage removes 2 ARM)
- **FR-026**: Gas Anomaly MUST have: Toxic Seep (Turn Start 2 non-weapon damage), Fume Panic (Wounded +1 SPD)
- **FR-027**: Mad Miner MUST have: Undermine (Battle Start Exposed if player DIG < boss DIG), Claim Jump (First Turn +1 strike if player Exposed)
- **FR-028**: Shard Colossus MUST have: Prismatic Spines (Battle Start +8 Shrapnel), Refracting Hide (every other turn +4 Shrapnel)
- **FR-029**: Drill Sergeant MUST have: Rev Up (Turn Start +1 ATK +1 SPD), Formation (every other turn +2 ARM)
- **FR-030**: Crystal Mimic MUST have: Prismatic Reflection (2 stacks, reflect status), Glass Heart (after reflection gone +2 non-weapon damage taken)
- **FR-031**: Rust Regent MUST have: Corroding Edict (On Hit once/turn apply 1 Rust), Execution Tax (Exposed at Turn Start take 2 non-weapon damage)
- **FR-032**: Powder Keg Baron MUST have: Volatile Countdown(3) (deal 10 non-weapon to both), Short Fuse (Wounded reduce countdown by 1)
- **FR-033**: Greedkeeper MUST have: Toll Collector (Battle Start steal 10 Gold), Gilded Barrier (gain ARM = floor(stolen/5) cap 6)
- **FR-034**: The Eldritch Mole MUST have: Three Phases (75% +10 ARM, 50% 2 strikes, 25% Turn Start 2 Bleed), Deep Dig (player DIG > boss reduces Phase 1 armor)
- **FR-035**: The Gilded Devourer MUST have: Tax Feast (Battle Start Gold→ARM +1 per 5 cap 10), Hunger (Wounded apply 3 Bleed)
- **FR-036**: The Frostbound Leviathan MUST have: Whiteout (Battle Start 3 Chill), Glacial Bulk (every other turn +4 ARM), Crack Ice (Exposed removes Chill +2 SPD)
- **FR-037**: The Rusted Chronomancer MUST have: Time Shear (First Turn 2 strikes), Oxidized Future (Turn Start 1 Rust), Blood Price (Wounded 4 Bleed)

### Key Entities

- **BossDefinition**: Static boss data including ID, name, emoji, base stats (HP/ATK/ARM/SPD/DIG), weakness tags, and trait definitions
- **BossTrait**: A special ability with trigger type, effect type, values, and optional modifiers (once per turn, countdown, phases)
- **BossInstance**: Runtime boss with scaled stats for a specific stage, derived from BossDefinition + act/stage modifiers
- **PhaseState**: Tracks which phases have activated for Week 3 final bosses (75%, 50%, 25% thresholds)
- **CountdownState**: Tracks remaining turns for countdown-based traits
- **ReflectionState**: Tracks remaining reflection stacks for Crystal Mimic

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 24 boss definitions can be queried and return correct stats matching GDD within 1 second
- **SC-002**: Boss selection for any of 80 stages returns deterministic, correct boss ID
- **SC-003**: Stat scaling calculations produce values matching GDD formulas for all tier/act combinations
- **SC-004**: All 37 unique boss traits (FR-024 through FR-037) execute correctly at their trigger points
- **SC-005**: Phase transitions for Week 3 finals activate at exactly 75%, 50%, 25% HP thresholds
- **SC-006**: Countdown traits decrement correctly and execute damage at turn 0
- **SC-007**: Boss weakness tags integrate with item system for offer weighting

## Assumptions

- Combat system (003) is implemented and provides CombatantInput/ItemEffect interfaces
- Item system (004) handles tag weighting; boss system only provides weakness data
- Biome B Week 1/2 bosses are variants of Biome A bosses with adjusted stats and one trait tweak
- "Act+ trait line" mentioned in GDD will be data-driven intensification of existing traits, not new mechanics
- Boss combat uses same turn order rules as field enemy combat (SPD comparison, enemy wins ties)
