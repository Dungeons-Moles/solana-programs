# Feature Specification: Gameplay State Tracking

**Feature Branch**: `002-gameplay-state-tracking`
**Created**: 2025-01-17
**Status**: Draft
**Input**: Track player movement, stats, gear slots, and day/night progression on-chain for the Dungeons and Moles game

## Overview

This feature extends the Solana programs to track core gameplay state on-chain, enabling verifiable runs and laying the foundation for future PvP and leaderboard features. The scope is limited to fundamental mechanics: movement, stats, gear slots, and time progression. Combat, enemies, POIs, bosses, and gold are explicitly excluded from this batch.

## User Scenarios & Testing

### User Story 1 - Player Movement Tracking (Priority: P1)

As a player, I want my movement through the dungeon to be tracked on-chain so that my run can be verified and replayed.

**Why this priority**: Movement is the core gameplay action. Every other feature depends on knowing where the player is and how many moves they've made.

**Independent Test**: Can be fully tested by initializing a game state, executing a series of move commands, and verifying the position and move count update correctly.

**Acceptance Scenarios**:

1. **Given** a game state at position (5, 5), **When** player moves to an adjacent floor tile (5, 6), **Then** position updates to (5, 6) and moves_remaining decreases by 1
2. **Given** a game state at position (5, 5) with DIG=1, **When** player digs into adjacent wall tile (5, 6), **Then** position updates to (5, 6) and moves_remaining decreases by 5 (max(2, 6-1))
3. **Given** a game state at position (5, 5) with DIG=4, **When** player digs into adjacent wall tile (5, 6), **Then** position updates to (5, 6) and moves_remaining decreases by 2 (max(2, 6-4))
4. **Given** a game state at position (0, 0), **When** player attempts to move to (-1, 0), **Then** the action is rejected with an out-of-bounds error
5. **Given** a game state with moves_remaining=0, **When** player attempts to move, **Then** the action is rejected with insufficient moves error

---

### User Story 2 - Time and Phase Progression (Priority: P2)

As a player, I want the day/night cycle and week progression to be tracked on-chain so that the game time advances correctly and boss encounters trigger at the right moments.

**Why this priority**: Time progression determines when phases end and boss fights occur. It's the second most fundamental mechanic after movement.

**Independent Test**: Can be tested by initializing game state in Day 1, consuming all 50 moves, and verifying automatic transition to Night 1 with 30 moves.

**Acceptance Scenarios**:

1. **Given** game state in Day 1 (50 moves), **When** all moves are consumed, **Then** phase advances to Night 1 with 30 moves
2. **Given** game state in Night 1 (30 moves), **When** all moves are consumed, **Then** phase advances to Day 2 with 50 moves
3. **Given** game state in Night 3 of Week 1, **When** all moves are consumed, **Then** phase advances to Week 2 Day 1 with 50 moves and gear_slots increases by 2
4. **Given** game state in Night 3 of Week 3, **When** all moves are consumed, **Then** game state is marked as ready for boss fight

---

### User Story 3 - Player Stats Management (Priority: P3)

As a player, I want my stats (HP, ATK, ARM, SPD, DIG) to be tracked on-chain so that item effects and game events can modify them verifiably.

**Why this priority**: Stats are essential for future combat and item integration, but can be tested independently of those features.

**Independent Test**: Can be tested by initializing game state with default stats and calling modify_stat to adjust values, verifying bounds and persistence.

**Acceptance Scenarios**:

1. **Given** a new game state, **When** initialized, **Then** stats are HP=10, ATK=1, ARM=0, SPD=0, DIG=1
2. **Given** game state with HP=10, **When** HP is modified by -3, **Then** HP becomes 7
3. **Given** game state with HP=2, **When** HP is modified by -5, **Then** HP becomes 0 (not negative)
4. **Given** game state with ARM=5, **When** ARM is modified by +3, **Then** ARM becomes 8
5. **Given** game state with DIG=3, **When** player digs a wall, **Then** dig cost is max(2, 6-3) = 3 moves

---

### User Story 4 - Gear Slots Progression (Priority: P4)

As a player, I want my gear slot capacity to increase after defeating weekly bosses so that I can equip more items as the run progresses.

**Why this priority**: Gear slots are a reward mechanic that depends on time progression. Lower priority since it's a simple counter.

**Independent Test**: Can be tested by simulating week transitions and verifying gear_slots increases at the correct times.

**Acceptance Scenarios**:

1. **Given** a new game state, **When** initialized, **Then** gear_slots is 4
2. **Given** game state in Week 1 Night 3, **When** week ends, **Then** gear_slots increases to 6
3. **Given** game state in Week 2 Night 3, **When** week ends, **Then** gear_slots increases to 8
4. **Given** game state with gear_slots=8, **When** Week 3 ends, **Then** gear_slots remains 8 (max capacity)

---

### Edge Cases

- What happens when player has exactly 1 move remaining but needs 5 moves to dig? Action is rejected.
- What happens when map boundaries are not defined? Require map dimensions to be set on initialization.
- What happens when a stat modification would overflow? Cap at maximum value (u8 max for unsigned).
- What happens when SPD goes negative? Allow negative values (i8) since SPD can be debuffed.
- What happens during Week 3 Night 3 completion? Signal boss fight ready but don't auto-transition.

## Requirements

### Functional Requirements

#### Movement & Position

- **FR-001**: System MUST track player position as (x, y) coordinates within map boundaries
- **FR-002**: System MUST validate that target position is within map dimensions before allowing movement
- **FR-003**: System MUST deduct 1 move for movement to a floor tile
- **FR-004**: System MUST deduct max(2, 6 - DIG) moves for digging through a wall tile
- **FR-005**: System MUST reject movement when moves_remaining is insufficient for the action
- **FR-006**: System MUST track total moves made throughout the session

#### Time & Phase

- **FR-007**: System MUST track current week (1, 2, or 3)
- **FR-008**: System MUST track current phase (Day1, Night1, Day2, Night2, Day3, Night3)
- **FR-009**: System MUST set moves_remaining to 50 for Day phases and 30 for Night phases
- **FR-010**: System MUST automatically advance phase when moves_remaining reaches 0
- **FR-011**: System MUST advance week after Night 3 completes (Week 1 → Week 2, Week 2 → Week 3)
- **FR-012**: System MUST signal boss fight ready when Week 3 Night 3 moves are exhausted

#### Player Stats

- **FR-013**: System MUST track HP, ATK, ARM, SPD, and DIG stats for the player
- **FR-014**: System MUST initialize stats to default values: HP=10, ATK=1, ARM=0, SPD=0, DIG=1
- **FR-015**: System MUST allow stat modifications (positive or negative) from authorized sources
- **FR-016**: System MUST prevent HP from going below 0
- **FR-017**: System MUST allow SPD to be negative (debuff scenarios)
- **FR-018**: System MUST track max_hp separately from current hp

#### Gear Slots

- **FR-019**: System MUST track gear slot capacity starting at 4
- **FR-020**: System MUST increase gear_slots by 2 when Week 1 ends (total: 6)
- **FR-021**: System MUST increase gear_slots by 2 when Week 2 ends (total: 8)
- **FR-022**: System MUST cap gear_slots at maximum of 8

#### Session Integration

- **FR-023**: Game state MUST be linked to an existing game session
- **FR-024**: Only the session owner can modify game state
- **FR-025**: Game state MUST be closeable when session ends

### Key Entities

- **GameState**: Core gameplay state including position, stats, gear slots, and time progression. Linked to a GameSession account. Contains all mutable game data for a single run.

- **Phase**: Enumeration of time phases (Day1, Night1, Day2, Night2, Day3, Night3) determining move allowance per phase.

- **Position**: Player's current location on the map as (x, y) coordinates, validated against map boundaries.

## Success Criteria

### Measurable Outcomes

- **SC-001**: All movement operations complete and confirm within 2 seconds on devnet
- **SC-002**: Game state correctly tracks position across 100+ consecutive moves without error
- **SC-003**: Phase transitions occur exactly when moves_remaining reaches 0, verified by tests
- **SC-004**: Dig cost calculation matches formula max(2, 6-DIG) for all DIG values 0-10
- **SC-005**: Gear slots increase exactly at Week 1 and Week 2 boundaries
- **SC-006**: 100% of out-of-bounds and insufficient-moves scenarios are rejected with appropriate errors
- **SC-007**: Game state account size is under 200 bytes for efficient rent

## Assumptions

- Map dimensions are provided when initializing game state (not dynamically resized)
- Tile type (floor vs wall) is passed by the client; on-chain program trusts client for tile data
- Combat, enemies, POIs, bosses, and gold tracking are out of scope for this feature
- Item effects on stats will be implemented in a future feature
- The existing GameSession from 001-solana-core-programs is used as the parent account

## Out of Scope

- Combat resolution and damage calculation
- Enemy spawning and tracking
- POI interactions
- Boss fight mechanics
- Gold economy
- Item inventory management (only slot count is tracked)
- Map generation or storage on-chain
