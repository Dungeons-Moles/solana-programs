# Feature Specification: Core Gameplay Loop

**Feature Branch**: `010-core-gameplay-loop`  
**Created**: 2026-01-21  
**Status**: Draft  
**Input**: User description covering session creation, atomic combat, night mechanics, progression, and item unlocks

## Overview

This specification defines the complete core gameplay loop for Dungeons & Moles, including:

- Atomic session creation with deterministic map generation
- Movement with automatic combat resolution
- Night-phase enemy behavior
- Boss encounters
- Death/victory handling with progression
- Multi-session support per player
- Run economy and item unlock progression

---

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Session Creation & Map Initialization (Priority: P1)

A player wants to start a new dungeon run on a specific campaign level. They sign a single transaction that creates the session, generates the deterministic map (enemies, POIs, boss data), initializes their character with the Basic Pickaxe, and transfers SOL to their burner wallet for gameplay transactions.

**Why this priority**: Without session creation, no gameplay is possible. This is the entry point for all runs.

**Independent Test**: Create a session for level 1, verify all map data is stored on-chain, verify Basic Pickaxe is equipped, verify burner wallet received SOL.

**Acceptance Scenarios**:

1. **Given** a player with available runs and an unlocked level, **When** they sign the session creation transaction, **Then** a new session is created with deterministic map data (enemies, POIs) stored on-chain, character spawns with Basic Pickaxe (1 ATK, no tags, common tier), 4 gear slots, and no oil.

2. **Given** a player creating a session, **When** the transaction completes, **Then** the specified SOL amount is transferred from main wallet to burner wallet for gameplay signing.

3. **Given** a player with no available runs, **When** they attempt to create a session, **Then** the transaction fails with "No available runs" error.

4. **Given** a player attempting to start a locked level, **When** they submit the transaction, **Then** it fails with "Level not unlocked" error.

5. **Given** a player with an existing session on level 5, **When** they create a session on level 3, **Then** a new session is created (multiple sessions on different levels allowed).

---

### User Story 2 - Movement with Automatic Combat (Priority: P1)

A player moves their character to an adjacent tile. If the tile contains an enemy, combat is automatically triggered and resolved within the same transaction. All combat actions are emitted as events for frontend display.

**Why this priority**: Movement and combat are the core gameplay actions that players perform every turn.

**Independent Test**: Move to a tile with an enemy, verify combat resolves atomically, verify all combat events are emitted.

**Acceptance Scenarios**:

1. **Given** a player on tile (2,3), **When** they move to adjacent floor tile (2,4) with no enemy, **Then** position updates to (2,4), 1 move is deducted, and no combat occurs.

2. **Given** a player on tile (2,3), **When** they move to adjacent wall tile (3,3) with DIG stat of 2, **Then** position updates to (3,3), max(2, 6-2)=4 moves are deducted.

3. **Given** a player on tile (2,3), **When** they move to tile (2,4) containing an enemy, **Then** movement occurs, combat automatically triggers, all combat turns are resolved, events for each action are emitted (damage dealt, status effects, HP changes), and final result (player HP, enemy defeated or player defeated) is determined in the same transaction.

4. **Given** a player who defeats an enemy in combat, **When** combat ends, **Then** the enemy is removed from the map state, rewards (gold, XP) are applied, and the player remains on that tile.

5. **Given** a player who is defeated in combat, **When** combat ends, **Then** the session is closed, one run is deducted from their profile, and all session data is cleaned up in the same transaction.

6. **Given** a player attempting to move outside map boundaries, **When** they submit the move, **Then** the transaction fails with "Out of bounds" error.

7. **Given** a player attempting to move to a non-adjacent tile, **When** they submit the move, **Then** the transaction fails with "Not adjacent" error.

---

### User Story 3 - Night Phase Enemy Movement (Priority: P1)

During night phases, enemies within 3 tiles of the player move toward the player. If an enemy moves into the player's tile, combat triggers. Enemy positions persist on-chain and remain fixed when day begins.

**Why this priority**: Night mechanics add strategic depth and danger, making the game more engaging.

**Independent Test**: Enter night phase, verify enemies within range move toward player, verify positions are saved on-chain.

**Acceptance Scenarios**:

1. **Given** it is a night phase and an enemy is 3 tiles away from the player, **When** the player makes a move, **Then** the enemy moves 1 tile closer to the player before the player's move resolves.

2. **Given** it is a night phase and an enemy is 4+ tiles away, **When** the player moves, **Then** that enemy does not move.

3. **Given** a night phase where an enemy moves into the player's current tile, **When** this happens, **Then** combat is automatically triggered and resolved.

4. **Given** the player moves toward an enemy that also moved toward them, **When** they meet on the same tile, **Then** combat triggers.

5. **Given** it is a day phase, **When** the player moves, **Then** no enemies move regardless of distance.

6. **Given** an enemy moved during night and the phase changes to day, **When** day begins, **Then** the enemy's new position is persisted and it remains there.

---

### User Story 4 - POI Interaction (Priority: P1)

A player can interact with Points of Interest (chests, shops, shrines, etc.) but only when standing on the POI's tile. POI interaction is a separate action from movement (not auto-triggered).

**Why this priority**: POIs provide loot, items, and strategic choices essential to progression.

**Independent Test**: Move to a POI tile, call interact, verify interaction succeeds. Call interact while not on POI tile, verify it fails.

**Acceptance Scenarios**:

1. **Given** a player standing on a tile with a chest POI, **When** they call the interact instruction, **Then** the chest opens and rewards are granted.

2. **Given** a player standing on tile (5,5) with no POI, **When** they attempt to interact with a chest at (5,6), **Then** the transaction fails with "Player not on POI tile" error.

3. **Given** a player standing on a shop POI, **When** they interact, **Then** shop inventory is presented (items filtered by active item pool).

4. **Given** a player interacting with a one-time POI (e.g., cache), **When** interaction completes, **Then** the POI is marked as consumed and cannot be interacted with again.

---

### User Story 5 - Boss Encounter at Week End (Priority: P1)

When a player exhausts all moves in the final night phase of a week, the boss fight automatically triggers. Combat resolves atomically with results returned.

**Why this priority**: Boss fights are the climactic moments of each week and gate progression.

**Independent Test**: Exhaust final move of Week 3 Night 3, verify boss combat triggers and resolves in same transaction.

**Acceptance Scenarios**:

1. **Given** a player on Week 1, Night 3 with 1 move remaining, **When** they use that final move, **Then** the week ends, boss fight automatically triggers, combat resolves, and results are emitted.

2. **Given** a player defeats the Week 3 boss, **When** combat ends with victory, **Then** the level is marked complete, the next level is unlocked (if not already), a new item is unlocked (if applicable), the session ends successfully, and one run is consumed.

3. **Given** a player is defeated by a boss, **When** combat ends, **Then** the session closes, one run is deducted, and no progression occurs.

4. **Given** a player defeats a Week 1 or Week 2 boss, **When** combat ends with victory, **Then** they advance to the next week (Week 2 or Week 3), gain 2 additional gear slots, and continue the run.

---

### User Story 6 - Death Handling (Priority: P1)

When a player dies (HP reaches 0) in any combat (enemy or boss), the session is closed and one run is deducted in the same transaction.

**Why this priority**: Proper death handling ensures game state consistency and prevents exploits.

**Independent Test**: Trigger combat that kills the player, verify session closes and run decrements atomically.

**Acceptance Scenarios**:

1. **Given** a player with 5 HP fighting an enemy that deals 10 damage, **When** combat resolves and HP becomes 0 or negative, **Then** the session is immediately closed, `available_runs` decrements by 1, `total_runs` increments by 1, and a `RunCompleted` event is emitted with `victory: false`.

2. **Given** a player who dies, **When** the transaction completes, **Then** all session-related accounts (GameSession, GameState, etc.) are closed and rent is returned.

---

### User Story 7 - Victory & Level Unlock (Priority: P1)

When a player defeats the Week 3 boss, they complete the level. If this is their highest level beaten, the next level unlocks and a new item is unlocked.

**Why this priority**: Progression and rewards drive player engagement.

**Independent Test**: Complete level 5 for the first time, verify level 6 unlocks and 1 new item is added to unlocked_items.

**Acceptance Scenarios**:

1. **Given** a player whose highest unlocked level is 5 defeating the Week 3 boss on level 5, **When** victory is achieved, **Then** `highest_level_unlocked` becomes 6, one random item from the locked pool (indices 40-79) is added to `unlocked_items`, and an `ItemUnlocked` event is emitted.

2. **Given** a player replaying level 3 (already beaten) who defeats Week 3 boss, **When** victory is achieved, **Then** no new level or item is unlocked (progression only on first-time victories).

3. **Given** a player completing level 40 (max campaign level), **When** victory is achieved, **Then** no further levels unlock, but an item still unlocks if any remain.

---

### User Story 8 - Run Economy (Priority: P2)

Players start with 20 runs and can purchase 20 additional runs for 0.001 SOL at any time.

**Why this priority**: Monetization and player retention, but not required for core gameplay testing.

**Independent Test**: Call purchase_runs instruction, verify 20 runs added and 0.001 SOL transferred.

**Acceptance Scenarios**:

1. **Given** a new player creating their profile, **When** profile is initialized, **Then** `available_runs` is set to 20.

2. **Given** a player with 5 runs remaining, **When** they call `purchase_runs` with 0.001 SOL, **Then** `available_runs` becomes 25 and SOL is transferred to treasury.

3. **Given** a player with insufficient SOL balance, **When** they attempt to purchase runs, **Then** the transaction fails with "Insufficient funds" error.

---

### User Story 9 - Multi-Session Support (Priority: P2)

A player can have active sessions on multiple different levels simultaneously. Each session is independent.

**Why this priority**: Allows flexible gameplay without forcing players to abandon runs.

**Independent Test**: Create sessions on levels 1, 3, and 5 for same player, verify all three exist independently.

**Acceptance Scenarios**:

1. **Given** a player with a session on level 5, **When** they create a session on level 3, **Then** both sessions exist and can be played independently.

2. **Given** a player with sessions on levels 1, 3, and 5, **When** they make a move in the level 3 session, **Then** only level 3 session state is modified.

3. **Given** a player attempting to create a second session on the same level, **When** they submit the transaction, **Then** it fails (one session per level).

---

### User Story 10 - Item Unlock Progression (Priority: P2)

40 items are available from the start. The remaining 40 items unlock one at a time as the player completes campaign levels (1 item per level for 40 levels).

**Why this priority**: Provides long-term progression and discovery.

**Independent Test**: Complete levels 1-5 for first time, verify 5 new items are in unlocked_items.

**Acceptance Scenarios**:

1. **Given** a new player profile, **When** initialized, **Then** `unlocked_items` bitmask has exactly 40 bits set (indices 0-39, the starter items).

2. **Given** a player completing level N for the first time, **When** victory is achieved, **Then** one random item from indices 40-79 that is not yet unlocked is added to `unlocked_items`.

3. **Given** a player with all 80 items unlocked, **When** they complete a new level, **Then** no item unlock occurs (all already unlocked).

4. **Given** the unlocked item, **When** it's added, **Then** it is automatically added to the player's `active_item_pool` for future runs.

---

### Edge Cases

- What happens if a player disconnects mid-transaction? The transaction either completes or fails atomically; no partial state.
- What happens if combat calculation exceeds compute limits? Combat engine must be optimized to complete within Solana's compute budget.
- What if an enemy and player move to same tile simultaneously at night? Enemy movement resolves first, then player movement, ensuring deterministic outcome.
- What happens when all 40 unlockable items have been unlocked? No further item unlocks; level progression continues normally.
- What if burner wallet has insufficient SOL for transaction fees? Frontend must ensure sufficient SOL is transferred at session start.

---

## Requirements _(mandatory)_

### Functional Requirements

**Session & Initialization**

- **FR-001**: System MUST create a session with deterministic map data (enemies, POIs, boss assignment) in a single transaction.
- **FR-002**: System MUST transfer specified SOL amount from main wallet to burner wallet during session creation.
- **FR-003**: System MUST spawn character with Basic Pickaxe (1 ATK, no tags, Common tier) that cannot be found on the map.
- **FR-004**: System MUST initialize character with 4 gear slots and 0 applied oils.
- **FR-005**: System MUST validate `available_runs > 0` before creating a session.
- **FR-006**: System MUST validate `campaign_level <= highest_level_unlocked` before creating a session.
- **FR-007**: System MUST allow multiple sessions per player on different levels (PDA: `["session", player, level]`).
- **FR-008**: System MUST prevent multiple sessions on the same level for the same player.

**Movement & Combat**

- **FR-009**: System MUST move player with cost: floor=1 move, wall=max(2, 6-DIG) moves.
- **FR-010**: System MUST validate movement is to adjacent tile (Manhattan distance = 1).
- **FR-011**: System MUST validate movement is within map boundaries.
- **FR-012**: System MUST automatically trigger combat when player moves to tile with enemy.
- **FR-013**: System MUST resolve combat completely within the same transaction as movement.
- **FR-014**: System MUST emit detailed combat events for each action (damage, status effects, HP changes).
- **FR-015**: System MUST remove defeated enemies from on-chain map state.

**Night Mechanics**

- **FR-016**: System MUST move enemies within 3 tiles (Chebyshev distance) toward player during night phases.
- **FR-017**: System MUST trigger combat if enemy moves into player's tile during night.
- **FR-018**: System MUST persist enemy positions on-chain after night movement.
- **FR-019**: System MUST NOT move enemies during day phases.
- **FR-020**: System MUST retain enemy positions when phase changes from night to day.

**POI Interaction**

- **FR-021**: System MUST validate player is on POI tile before allowing interaction.
- **FR-022**: System MUST NOT auto-trigger POI interaction on movement.
- **FR-023**: System MUST fail POI interaction if player is not on the correct tile.
- **FR-024**: System MUST filter item offers by player's active item pool during shop/cache interactions.

**Boss Encounters**

- **FR-025**: System MUST trigger boss fight when player exhausts final move of a week's Night 3 phase.
- **FR-026**: System MUST resolve boss combat atomically in same transaction as triggering move.
- **FR-027**: System MUST emit boss combat events similar to enemy combat.
- **FR-028**: System MUST advance player to next week upon Week 1 or Week 2 boss victory.
- **FR-029**: System MUST end session with victory upon Week 3 boss defeat.

**Death & Victory**

- **FR-030**: System MUST close session and deduct run atomically when player HP reaches 0.
- **FR-031**: System MUST increment `total_runs` and decrement `available_runs` on any session end.
- **FR-032**: System MUST unlock next level when player defeats Week 3 boss for first time on highest level.
- **FR-033**: System MUST unlock one random locked item (indices 40-79) on first-time level completion.
- **FR-034**: System MUST add newly unlocked item to player's active item pool automatically.
- **FR-035**: System MUST emit `ItemUnlocked` event with item details on unlock.

**Run Economy**

- **FR-036**: System MUST initialize new player profiles with 20 available runs.
- **FR-037**: System MUST provide `purchase_runs` instruction that adds 20 runs for 0.001 SOL.
- **FR-038**: System MUST transfer purchase SOL to designated treasury wallet.
- **FR-039**: System MUST validate sufficient SOL balance before completing purchase.

**Item Progression**

- **FR-040**: System MUST initialize `unlocked_items` with 40 starter items (indices 0-39).
- **FR-041**: System MUST track item unlock state using 80-bit bitmask (10 bytes).
- **FR-042**: System MUST unlock items randomly from locked pool, not sequentially.
- **FR-043**: System MUST NOT unlock items on replay of already-completed levels.

### Key Entities

- **PlayerProfile**: Player identity, progression (highest_level_unlocked), run counts, unlocked_items bitmask, active_item_pool bitmask.
- **GameSession**: Active run on a specific level; links to player; stores level, state hash, delegation status.
- **LevelState**: On-chain storage for current level's map data including enemy positions/HP, POI states, player position.
- **Enemy**: Instance of a field enemy on the map with position (x, y), archetype, current HP, alive status.
- **POI**: Instance of a point of interest with position, type, consumed status.
- **Basic Pickaxe**: Starter tool (item_id: `T-XX-00`, no tags, Common, 1 ATK), not droppable from map.

---

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Session creation completes in a single signed transaction with all map data persisted.
- **SC-002**: Movement + combat resolution completes within Solana's 200k compute unit budget.
- **SC-003**: All combat actions are emitted as events with sufficient detail for frontend replay.
- **SC-004**: Night enemy movement correctly processes all enemies within 3 tiles every night-phase turn.
- **SC-005**: Boss fights trigger automatically on week end with no additional user action required.
- **SC-006**: Death handling atomically closes session and updates profile in same transaction.
- **SC-007**: First-time level completion unlocks exactly 1 new item from the locked pool.
- **SC-008**: Players can maintain sessions on up to 40 different levels simultaneously.
- **SC-009**: Run purchases complete successfully and add exactly 20 runs.
- **SC-010**: All 40 starter items are available on new profile creation.

---

## Assumptions

1. **Compute Budget**: Combat resolution (player vs single enemy) fits within Solana's compute limits. Boss fights may require optimization.
2. **Map Size**: 9x9 grid per level with deterministic placement based on seed.
3. **Enemy Count**: Maximum ~10 enemies per level to fit within account size limits.
4. **Basic Pickaxe**: Uses item_id format `T-XX-00` where XX indicates "starter-only" category.
5. **Treasury Wallet**: A placeholder pubkey will be used until mainnet deployment.
6. **Burner Wallet**: Created by frontend; main wallet transfers SOL during session creation.
7. **Active Item Pool**: Defaults to all unlocked items; can be customized (separate feature).
8. **Night Detection**: Determined by Phase enum (Night1, Night2, Night3) in GameState.

---

## Starter Items Reference

The 40 starter items (indices 0-39) are:

| Set   | Tool    | Gear 1  | Gear 2  | Gear 3  | Gear 4  |
| ----- | ------- | ------- | ------- | ------- | ------- |
| STONE | T-ST-01 | G-ST-01 | G-ST-02 | G-ST-03 | G-ST-04 |
| SCOUT | T-SC-01 | G-SC-01 | G-SC-02 | G-SC-03 | G-SC-04 |
| GREED | T-GR-01 | G-GR-01 | G-GR-02 | G-GR-03 | G-GR-05 |
| BLAST | T-BL-01 | G-BL-01 | G-BL-02 | G-BL-03 | G-BL-04 |
| FROST | T-FR-01 | G-FR-01 | G-FR-02 | G-FR-03 | G-FR-04 |
| RUST  | T-RU-01 | G-RU-01 | G-RU-02 | G-RU-03 | G-RU-04 |
| BLOOD | T-BO-01 | G-BO-01 | G-BO-02 | G-BO-03 | G-BO-04 |
| TEMPO | T-TE-01 | G-TE-01 | G-TE-02 | G-TE-03 | G-TE-04 |

The remaining 40 items (indices 40-79) are unlocked progressively through campaign completion.
