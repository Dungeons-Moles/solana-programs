# Feature Specification: Solana Core Programs

**Feature Branch**: `001-solana-core-programs`
**Created**: 2025-01-15
**Status**: Draft
**Input**: User description: "Implement foundational Solana programs: Player Profile (progression, payments), Session Manager (MagicBlock ephemeral rollups), and Map Generation (deterministic procedural generation)"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - New Player Registration (Priority: P1)

A new player connects their Solana wallet to Dungeons & Moles for the first time. The system creates their player profile, storing their wallet address and allowing them to set a display name. The player can immediately start playing campaign levels 0-39 without any payment.

**Why this priority**: This is the entry point for all players. Without profile creation, no other game features can function. It establishes player identity on-chain.

**Independent Test**: Can be fully tested by connecting a wallet and verifying the profile account is created with correct initial values. Delivers immediate value as the foundation for all gameplay.

**Acceptance Scenarios**:

1. **Given** a player with a Solana wallet who has never played, **When** they connect their wallet and request profile creation, **Then** a new profile is created with their wallet address, empty name, 0 total runs, and campaign level 0.
2. **Given** a player with an existing profile, **When** they attempt to create a new profile, **Then** the system rejects the request and returns the existing profile.
3. **Given** a player with a profile, **When** they update their display name, **Then** the name is stored and persisted (max 32 characters).

---

### User Story 2 - Campaign Tier Unlocking (Priority: P2)

A player has completed campaign levels 0-39 and wants to continue to levels 40-79. They pay 0.05 SOL to unlock the next tier of 40 campaign levels. The payment is processed and their profile is updated to allow access to the new levels.

**Why this priority**: This is the primary monetization mechanism. It enables sustainable game development while keeping initial gameplay free.

**Independent Test**: Can be tested by having a player at campaign level 39, attempting to advance to level 40, making the payment, and verifying access is granted.

**Acceptance Scenarios**:

1. **Given** a player at campaign level 39 (end of free tier), **When** they pay 0.05 SOL to unlock the next tier, **Then** their max campaign level increases to 79 and the payment is transferred to the game treasury.
2. **Given** a player who has not yet completed levels 0-39, **When** they attempt to pay for the next tier, **Then** the system rejects the payment (cannot skip tiers).
3. **Given** a player at campaign level 79, **When** they pay 0.05 SOL, **Then** their max campaign level increases to 119 (system supports future expansion).
4. **Given** a player attempting to unlock with insufficient SOL balance, **When** the transaction is submitted, **Then** the system rejects the transaction and no state changes occur.

---

### User Story 3 - Game Session Delegation (Priority: P3)

A player starts a game session. Their game state is delegated to the MagicBlock ephemeral rollup for high-performance gameplay. When the session ends, the final state is committed back to the main chain.

**Why this priority**: This enables the real-time gameplay experience required for a responsive dungeon crawler. Without session delegation, the game would be too slow for enjoyable play.

**Independent Test**: Can be tested by starting a session, verifying delegation occurs, performing game actions on the ephemeral rollup, then ending the session and verifying state is committed.

**Acceptance Scenarios**:

1. **Given** a player with a valid profile, **When** they start a new game session, **Then** a session account is created and their game state is delegated to the ephemeral rollup.
2. **Given** a player with an active session, **When** they end their session, **Then** the final game state is committed back to mainnet and the session is marked complete.
3. **Given** a player with an active session, **When** they attempt to start another session, **Then** the system rejects the request (one active session per player).
4. **Given** a session that has been idle for more than 1 hour, **When** the timeout is checked, **Then** the session can be forcibly closed and state committed.

---

### User Story 4 - Map Generation for Campaign Level (Priority: P4)

A player starts a campaign level. The system generates a deterministic 50x50 dungeon map based on the level's seed. The map includes corridors, rooms, points of interest, enemy spawn locations, and a boss spawn point. The same seed always produces the identical map.

**Why this priority**: This is the core gameplay content. However, it depends on profiles and sessions being functional first.

**Independent Test**: Can be tested by requesting map generation for a specific campaign level, verifying the output matches expected structure, and confirming that repeated generation with the same seed produces identical results.

**Acceptance Scenarios**:

1. **Given** a campaign level number (0-80), **When** map generation is requested, **Then** a deterministic 50x50 map is generated using the configured seed for that level.
2. **Given** the same campaign level, **When** map generation is called multiple times, **Then** the output is identical each time (deterministic).
3. **Given** an admin with proper authority, **When** they update the seed mapping for a campaign level, **Then** future map generations for that level use the new seed.
4. **Given** a generated map, **When** inspected, **Then** it contains valid tile types (Floor, Wall, Gravel, Water), POI positions, enemy spawn positions, and a boss spawn point.

---

### User Story 5 - Run Completion Tracking (Priority: P5)

A player completes a dungeon run (win or lose). Their total run count is incremented. If they completed a new campaign level, their progress is updated.

**Why this priority**: This provides progression tracking and feeds into future features like leaderboards and achievements.

**Independent Test**: Can be tested by completing a run and verifying the run counter increments and campaign level updates appropriately.

**Acceptance Scenarios**:

1. **Given** a player who completes a run, **When** the run result is submitted, **Then** their total_runs counter increments by 1.
2. **Given** a player who defeats a boss on campaign level N, **When** the victory is recorded, **Then** their current_campaign_level advances to N+1 (if within their unlocked tier).
3. **Given** a player who loses a run, **When** the defeat is recorded, **Then** total_runs increments but campaign level does not advance.

---

### Edge Cases

- What happens when a player's profile account runs out of rent? System should ensure minimum rent-exempt balance at creation.
- How does the system handle network interruptions during session undelegation? Session timeout mechanism allows recovery.
- What happens if map generation is called with an invalid level number (>80)? System should reject with clear error.
- What if the treasury account is not initialized when a player tries to pay? Transaction should fail gracefully.
- What happens if the seed-to-level mapping is not initialized? System should use level number as default seed.

## Requirements *(mandatory)*

### Functional Requirements

**Player Profile Program**

- **FR-001**: System MUST create a player profile account when a new player initializes, storing wallet address, display name, total runs, and current campaign level.
- **FR-002**: System MUST enforce uniqueness of player profiles (one profile per wallet address).
- **FR-003**: System MUST allow players to update their display name (max 32 characters, UTF-8 encoded).
- **FR-004**: System MUST track total_runs as a cumulative counter that only increments.
- **FR-005**: System MUST track current_campaign_level (0-80+) representing the highest level unlocked.
- **FR-006**: System MUST accept 0.05 SOL payment to unlock the next 40 campaign levels.
- **FR-007**: System MUST transfer payments to a designated treasury account.
- **FR-008**: System MUST prevent tier unlocking if player has not reached the tier boundary (multiples of 40).

**Session Manager Program**

- **FR-009**: System MUST create a session account when a player starts a game session.
- **FR-010**: System MUST delegate session state to MagicBlock ephemeral rollup on session start.
- **FR-011**: System MUST commit session state back to mainnet on session end.
- **FR-012**: System MUST enforce one active session per player.
- **FR-013**: System MUST track session start time for timeout purposes.
- **FR-014**: System MUST allow forced session closure after 1 hour of inactivity.
- **FR-015**: System MUST validate delegation authority on all session operations.

**Map Generation Program**

- **FR-016**: System MUST generate deterministic 50x50 maps from a seed value.
- **FR-017**: System MUST produce identical output for identical seeds (pure function).
- **FR-018**: System MUST store seed-to-level mapping in a configuration account.
- **FR-019**: System MUST use seeds 0-80 as default mapping for campaign levels 0-80.
- **FR-020**: System MUST allow admin authority to update seed mappings.
- **FR-021**: Generated maps MUST include tile grid (Floor, Wall, Gravel, Water types).
- **FR-022**: Generated maps MUST include POI spawn positions with rarity distribution.
- **FR-023**: Generated maps MUST include enemy spawn positions with zone-based tier selection.
- **FR-024**: Generated maps MUST include boss spawn point location.
- **FR-025**: System MUST reject map generation requests for invalid level numbers.

### Key Entities

- **PlayerProfile**: Represents a player's on-chain identity and progression. Contains wallet address (owner), display name, total run count, current campaign level, and unlocked tier indicator.

- **GameSession**: Represents an active gameplay session. Contains player reference, session start timestamp, delegation status, and session state hash.

- **MapConfig**: Stores the seed-to-level mapping configuration. Contains admin authority, array of 81 seed values (levels 0-80), and version number for future migrations.

- **GeneratedMap**: Output structure for map generation. Contains dimensions, tile grid, POI positions with types, enemy spawn positions with tiers, and boss spawn location.

- **Treasury**: Receives payment for campaign tier unlocks. Contains accumulated SOL and withdrawal authority.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Players can create a profile and start playing within 30 seconds of connecting their wallet.
- **SC-002**: Payment transactions for tier unlocking complete within 5 seconds on mainnet.
- **SC-003**: Session delegation completes within 2 seconds, enabling immediate gameplay.
- **SC-004**: Map generation produces consistent results 100% of the time for the same seed (determinism verified).
- **SC-005**: System supports 10,000+ concurrent player profiles without degradation.
- **SC-006**: Session state commits successfully 99.9% of the time during normal undelegation.
- **SC-007**: 95% of players successfully complete profile creation on first attempt.
- **SC-008**: Map generation can be verified client-side to match on-chain generation exactly.

## Assumptions

- MagicBlock ephemeral rollup infrastructure is available and compatible with Anchor programs.
- Treasury account will be initialized before any players attempt tier unlocking.
- Campaign will initially have 81 levels (0-80), with tiers at 0-39 (free), 40-79 (paid), 80+ (future expansion).
- Default seed mapping uses level number as seed (level 0 = seed 0, level 1 = seed 1, etc.).
- Session timeout of 1 hour is acceptable for the game's session length expectations.
- Display names do not require uniqueness enforcement (multiple players can have same name).
