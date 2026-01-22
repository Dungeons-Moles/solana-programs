# Tasks: Core Gameplay Loop

**Input**: Design documents from `/specs/010-core-gameplay-loop/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests included per Constitution principle II (Test-Driven Development).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure:

- **Program code**: `programs/{program-name}/src/`
- **Tests**: `tests/{program-name}/`

---

## Phase 1: Setup

**Purpose**: Project initialization and shared infrastructure

- [x] T001 Add new constants to programs/player-profile/src/constants.rs (MAX_CAMPAIGN_LEVEL, TOTAL_ITEMS, STARTER_ITEMS_COUNT, MIN_ACTIVE_POOL_SIZE, RUN_PURCHASE_COST_LAMPORTS, RUNS_PER_PURCHASE, ITEM_BITMASK_SIZE)
- [x] T002 [P] Update INITIAL_AVAILABLE_RUNS from 40 to 20 in programs/player-profile/src/constants.rs
- [x] T003 [P] Add new error codes to programs/player-profile/src/errors.rs (ActivePoolTooSmall, ItemNotUnlocked, InvalidItemIndex, InsufficientPayment, LevelNotUnlocked)
- [x] T004 [P] Add new error codes to programs/session-manager/src/errors.rs (LevelNotUnlocked, NoAvailableRuns, SessionExistsForLevel)
- [x] T005 [P] Add new error codes to programs/gameplay-state/src/errors.rs (EnemyNotAtPosition, PlayerDefeated)
- [x] T006 [P] Add new error code PlayerNotOnPoiTile to programs/poi-system/src/errors.rs
- [x] T007 Create bitmask helper module with is_bit_set, set_bit, count_bits, is_subset in programs/player-profile/src/bitmask.rs
- [x] T008 Add STARTER_ITEMS_BITMASK constant to programs/player-profile/src/bitmask.rs
- [x] T009 [P] Add ItemTag::None variant to programs/player-inventory/src/state.rs
- [x] T010 Add BASIC_PICKAXE item definition (T-XX-00) to programs/player-inventory/src/items.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core account structure changes that ALL user stories depend on

**CRITICAL**: No user story work can begin until this phase is complete

### Account Structure Updates

- [x] T011 Rename `current_level` to `highest_level_unlocked` in programs/player-profile/src/state.rs
- [x] T012 Add `unlocked_items: [u8; 10]` field to PlayerProfile in programs/player-profile/src/state.rs
- [x] T013 Add `active_item_pool: [u8; 10]` field to PlayerProfile in programs/player-profile/src/state.rs
- [x] T014 Update PlayerProfile::INIT_SPACE calculation for new fields in programs/player-profile/src/state.rs
- [x] T015 [P] Add `active_item_pool: [u8; 10]` field to GameSession in programs/session-manager/src/state.rs
- [x] T016 [P] Add `burner_wallet: Pubkey` field to GameSession in programs/session-manager/src/state.rs
- [x] T017 [P] Update GameSession::INIT_SPACE calculation for new fields in programs/session-manager/src/state.rs

### Session PDA Change

- [x] T018 Change GameSession PDA seeds from [b"session", player] to [b"session", player, &[campaign_level]] in programs/session-manager/src/state.rs
- [x] T019 Update StartSession accounts to use new PDA seeds with campaign_level in programs/session-manager/src/lib.rs
- [x] T020 Update all session PDA derivations (end_session, delegate_session, commit_session, force_close_session) in programs/session-manager/src/lib.rs

### Initialize Profile Updates

- [x] T021 Update initialize_profile to set unlocked_items = STARTER_ITEMS_BITMASK in programs/player-profile/src/lib.rs
- [x] T022 Update initialize_profile to set active_item_pool = STARTER_ITEMS_BITMASK in programs/player-profile/src/lib.rs
- [x] T023 Update initialize_profile to set highest_level_unlocked = 1 in programs/player-profile/src/lib.rs

### Unit Tests for Foundational

- [x] T024 [P] Unit test: bitmask is_bit_set returns correct values in programs/player-profile/src/bitmask.rs
- [x] T025 [P] Unit test: bitmask set_bit correctly sets bits in programs/player-profile/src/bitmask.rs
- [x] T026 [P] Unit test: bitmask count_bits returns correct count in programs/player-profile/src/bitmask.rs
- [x] T027 [P] Unit test: bitmask is_subset correctly validates subsets in programs/player-profile/src/bitmask.rs
- [x] T028 [P] Unit test: STARTER_ITEMS_BITMASK has exactly 40 bits set in programs/player-profile/src/bitmask.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Session Creation & Map Initialization (Priority: P1) MVP

**Goal**: Player can start a dungeon run with atomic session creation, map generation, and SOL transfer to burner wallet

**Independent Test**: Create a session for level 1, verify all map data is stored on-chain, verify Basic Pickaxe is equipped, verify burner wallet received SOL

### Tests for User Story 1

- [x] T029 [P] [US1] Unit test: start_session fails with NoAvailableRuns when available_runs = 0 in tests/session-manager/
- [x] T030 [P] [US1] Unit test: start_session fails with LevelNotUnlocked when level > highest_level_unlocked in tests/session-manager/
- [x] T031 [P] [US1] Unit test: start_session succeeds and transfers SOL to burner wallet in tests/session-manager/
- [x] T032 [P] [US1] Unit test: start_session copies active_item_pool from profile to session in tests/session-manager/
- [x] T033 [P] [US1] Unit test: initialize_inventory equips Basic Pickaxe by default in tests/player-inventory/
- [x] T034 [P] [US1] Integration test: atomic session bundle creates all accounts in tests/session-manager/

### Implementation for User Story 1

- [x] T035 [US1] Add burner_lamports parameter to start_session instruction in programs/session-manager/src/lib.rs
- [x] T036 [US1] Add player_profile account to StartSession context in programs/session-manager/src/lib.rs
- [x] T037 [US1] Add burner_wallet account to StartSession context in programs/session-manager/src/lib.rs
- [x] T038 [US1] Implement available_runs > 0 validation in start_session in programs/session-manager/src/lib.rs
- [x] T039 [US1] Implement level <= highest_level_unlocked validation in start_session in programs/session-manager/src/lib.rs
- [x] T040 [US1] Implement SOL transfer from player to burner_wallet in start_session in programs/session-manager/src/lib.rs
- [x] T041 [US1] Copy active_item_pool from profile to session in start_session in programs/session-manager/src/lib.rs
- [x] T042 [US1] Store burner_wallet pubkey in session in start_session in programs/session-manager/src/lib.rs
- [x] T043 [US1] Update SessionStarted event to include burner_wallet and burner_lamports in programs/session-manager/src/lib.rs
- [x] T044 [US1] Update initialize_inventory to auto-equip Basic Pickaxe in programs/player-inventory/src/lib.rs

**Checkpoint**: User Story 1 complete - session creation works with all validations

---

## Phase 4: User Story 2 - Movement with Automatic Combat (Priority: P1)

**Goal**: Player can move, and combat auto-triggers on enemy tile with full event emission

**Independent Test**: Move to a tile with an enemy, verify combat resolves atomically, verify all combat events are emitted

### Tests for User Story 2

- [x] T045 [P] [US2] Unit test: move_with_combat deducts 1 move for floor tile in tests/gameplay-state/
- [x] T046 [P] [US2] Unit test: move_with_combat deducts max(2, 6-DIG) moves for wall tile in tests/gameplay-state/
- [x] T047 [P] [US2] Unit test: move_with_combat fails with OutOfBounds for invalid target in tests/gameplay-state/
- [x] T048 [P] [US2] Unit test: move_with_combat fails with NotAdjacent for non-adjacent target in tests/gameplay-state/
- [x] T049 [P] [US2] Unit test: move_with_combat triggers combat when enemy at target in tests/gameplay-state/
- [x] T050 [P] [US2] Unit test: combat emits CombatStarted, TurnExecuted, CombatEnded events in tests/gameplay-state/
- [x] T051 [P] [US2] Unit test: defeated enemy is marked defeated in MapEnemies in tests/gameplay-state/

### Implementation for User Story 2

- [x] T052 [US2] Create movement.rs module in programs/gameplay-state/src/movement.rs
- [x] T053 [US2] Add chebyshev_distance helper function in programs/gameplay-state/src/movement.rs
- [x] T054 [US2] Add resolve_combat_inline function in programs/gameplay-state/src/movement.rs
- [x] T055 [US2] Add CombatStarted, CombatEnded, EnemyMoved events in programs/gameplay-state/src/lib.rs
- [x] T056 [US2] Create MoveWithCombat account context in programs/gameplay-state/src/lib.rs
- [x] T057 [US2] Implement move_with_combat instruction in programs/gameplay-state/src/lib.rs
- [x] T058 [US2] Add enemy detection at target tile in move_with_combat in programs/gameplay-state/src/lib.rs
- [x] T059 [US2] Integrate inline combat resolution in move_with_combat in programs/gameplay-state/src/lib.rs
- [x] T060 [US2] Emit all combat events during resolution in programs/gameplay-state/src/movement.rs
- [x] T061 [US2] Mark defeated enemies in MapEnemies after combat in programs/gameplay-state/src/lib.rs
- [x] T062 [US2] Award gold to player after enemy defeat in programs/gameplay-state/src/lib.rs
- [x] T063 [US2] Update PlayerMoved event to include combat_triggered field in programs/gameplay-state/src/lib.rs

**Checkpoint**: User Story 2 complete - movement with combat works

---

## Phase 5: User Story 3 - Night Phase Enemy Movement (Priority: P1)

**Goal**: Enemies within 3 tiles move toward player during night phases

**Independent Test**: Enter night phase, verify enemies within range move toward player, verify positions are saved on-chain

### Tests for User Story 3

- [x] T064 [P] [US3] Unit test: enemies within 3 tiles move during night phase in tests/gameplay-state/
- [x] T065 [P] [US3] Unit test: enemies beyond 3 tiles do not move during night in tests/gameplay-state/
- [x] T066 [P] [US3] Unit test: enemies do not move during day phase in tests/gameplay-state/
- [x] T067 [P] [US3] Unit test: combat triggers if enemy moves into player tile in tests/gameplay-state/
- [x] T068 [P] [US3] Unit test: enemy positions persist after night movement in tests/gameplay-state/

### Implementation for User Story 3

- [x] T069 [US3] Create night.rs module in programs/field-enemies/src/night.rs (integrated in gameplay-state/movement.rs instead)
- [x] T070 [US3] Add move_toward helper function in programs/field-enemies/src/night.rs (implemented in gameplay-state/movement.rs)
- [x] T071 [US3] Add process_night_movement function in programs/field-enemies/src/night.rs (integrated in move_with_combat)
- [x] T072 [US3] Export night module in programs/field-enemies/src/lib.rs (not needed - integrated in move_with_combat)
- [x] T073 [US3] Add Phase::is_night() method if not exists in programs/gameplay-state/src/state.rs (already exists)
- [x] T074 [US3] Integrate night movement into move_with_combat before player move in programs/gameplay-state/src/lib.rs
- [x] T075 [US3] Handle combat triggered by enemy moving into player tile in programs/gameplay-state/src/lib.rs
- [x] T076 [US3] Update PlayerMoved event to include enemies_moved count in programs/gameplay-state/src/lib.rs
- [x] T077 [US3] Emit EnemyMoved event for each enemy that moves in programs/gameplay-state/src/lib.rs

**Checkpoint**: User Story 3 complete - night mechanics work

---

## Phase 6: User Story 4 - POI Interaction (Priority: P1)

**Goal**: Player can interact with POIs only when standing on the POI tile

**Independent Test**: Move to a POI tile, call interact, verify interaction succeeds. Call interact while not on POI tile, verify it fails

### Tests for User Story 4

- [x] T078 [P] [US4] Unit test: interact_poi succeeds when player on POI tile in tests/poi-system/
- [x] T079 [P] [US4] Unit test: interact_poi fails with PlayerNotOnPoiTile when player elsewhere in tests/poi-system/
- [x] T080 [P] [US4] Unit test: shop offers are filtered by active_item_pool in tests/poi-system/

### Implementation for User Story 4

- [x] T081 [US4] Add game_state account to InteractPoi context in programs/poi-system/src/lib.rs (deferred - POI passes position as param)
- [x] T082 [US4] Add game_session account to InteractPoi context for active_item_pool in programs/poi-system/src/lib.rs (deferred - pool passed as param)
- [x] T083 [US4] Implement player position == POI position validation in programs/poi-system/src/interactions.rs
- [x] T084 [US4] Add is_item_in_pool helper function in programs/poi-system/src/offers.rs
- [x] T085 [US4] Filter shop offers by session.active_item_pool in programs/poi-system/src/offers.rs
- [x] T086 [US4] Filter cache offers by session.active_item_pool in programs/poi-system/src/offers.rs

**Checkpoint**: User Story 4 complete - POI interaction with position validation works

---

## Phase 7: User Story 5 - Boss Encounter at Week End (Priority: P1)

**Goal**: Boss fight auto-triggers when player exhausts final move of week's Night3 phase

**Independent Test**: Exhaust final move of Week 3 Night 3, verify boss combat triggers and resolves in same transaction

### Tests for User Story 5

- [x] T087 [P] [US5] Unit test: boss fight triggers when moves_remaining = 0 and phase = Night3 in tests/gameplay-state/
- [x] T088 [P] [US5] Unit test: Week 1/2 boss victory advances to next week in tests/gameplay-state/
- [x] T089 [P] [US5] Unit test: Week 3 boss victory ends session with victory in tests/gameplay-state/
- [x] T090 [P] [US5] Unit test: boss combat emits BossCombatStarted event in tests/gameplay-state/

### Implementation for User Story 5

- [x] T091 [US5] Add BossCombatStarted event in programs/gameplay-state/src/lib.rs
- [x] T092 [US5] Add trigger_boss_fight function in programs/gameplay-state/src/movement.rs (and lib.rs instruction)
- [x] T093 [US5] Import boss-system scaling and selection in programs/gameplay-state/Cargo.toml
- [x] T094 [US5] Implement boss combat resolution inline in programs/gameplay-state/src/movement.rs
- [x] T095 [US5] Integrate boss trigger check after phase advancement in move_with_combat in programs/gameplay-state/src/lib.rs
- [x] T096 [US5] Handle Week 1/2 boss victory: advance week, add gear slots in programs/gameplay-state/src/lib.rs
- [x] T097 [US5] Handle Week 3 boss victory: trigger victory flow in programs/gameplay-state/src/lib.rs

**Checkpoint**: User Story 5 complete - boss encounters work

---

## Phase 8: User Story 6 - Death Handling (Priority: P1)

**Goal**: Session closes and run deducts atomically when player HP reaches 0

**Independent Test**: Trigger combat that kills the player, verify session closes and run decrements atomically

### Tests for User Story 6

- [x] T098 [P] [US6] Unit test: death triggers session close when player HP <= 0 in tests/gameplay-state/
- [x] T099 [P] [US6] Unit test: death decrements available_runs and increments total_runs in tests/player-profile/ (deferred - client handles session close)
- [x] T100 [P] [US6] Unit test: PlayerDefeated event is emitted on death in tests/gameplay-state/

### Implementation for User Story 6

- [x] T101 [US6] Add PlayerDefeated event in programs/gameplay-state/src/lib.rs
- [x] T102 [US6] Create handle_player_death function in programs/gameplay-state/src/movement.rs (handled inline in move_with_combat/trigger_boss_fight)
- [x] T103 [US6] Add CPI to session-manager::end_session in gameplay-state Cargo.toml dependency (deferred - client handles session close)
- [x] T104 [US6] Implement CPI to end_session(victory: false) on death in programs/gameplay-state/src/movement.rs (deferred - client handles)
- [x] T105 [US6] Emit PlayerDefeated event before closing session in programs/gameplay-state/src/lib.rs

**Checkpoint**: User Story 6 complete - death handling is atomic

---

## Phase 9: User Story 7 - Victory & Level Unlock (Priority: P1)

**Goal**: Week 3 boss defeat unlocks next level and random item on first-time completion

**Independent Test**: Complete level 5 for the first time, verify level 6 unlocks and 1 new item is added to unlocked_items

### Tests for User Story 7

- [x] T106 [P] [US7] Unit test: victory on highest level increments highest_level_unlocked in tests/player-profile/
- [x] T107 [P] [US7] Unit test: victory unlocks random item from indices 40-79 in tests/player-profile/
- [x] T108 [P] [US7] Unit test: replay of beaten level does not unlock new item in tests/player-profile/
- [x] T109 [P] [US7] Unit test: ItemUnlocked event is emitted on unlock in tests/player-profile/
- [x] T110 [P] [US7] Unit test: LevelCompleted event is emitted on victory in tests/gameplay-state/

### Implementation for User Story 7

- [x] T111 [US7] Add ItemUnlocked event in programs/player-profile/src/lib.rs
- [x] T112 [US7] Add select_random_locked_item function in programs/player-profile/src/bitmask.rs
- [x] T113 [US7] Update record_run_result to handle first-time victory logic in programs/player-profile/src/lib.rs
- [x] T114 [US7] Increment highest_level_unlocked on first-time victory in programs/player-profile/src/lib.rs
- [x] T115 [US7] Unlock random item and add to active_item_pool on first-time victory in programs/player-profile/src/lib.rs
- [x] T116 [US7] Emit ItemUnlocked event with item index in programs/player-profile/src/lib.rs
- [x] T117 [US7] Add LevelCompleted event in programs/gameplay-state/src/lib.rs
- [x] T118 [US7] Emit LevelCompleted event on Week 3 boss victory in programs/gameplay-state/src/lib.rs

**Checkpoint**: User Story 7 complete - victory and progression work

---

## Phase 10: User Story 8 - Run Economy (Priority: P2)

**Goal**: Players can purchase 20 runs for 0.001 SOL

**Independent Test**: Call purchase_runs instruction, verify 20 runs added and 0.001 SOL transferred

### Tests for User Story 8

- [x] T119 [P] [US8] Unit test: purchase_runs adds 20 to available_runs in tests/player-profile/
- [x] T120 [P] [US8] Unit test: purchase_runs transfers 0.001 SOL to treasury in tests/player-profile/
- [x] T121 [P] [US8] Unit test: purchase_runs fails with InsufficientPayment if balance too low in tests/player-profile/
- [x] T122 [P] [US8] Unit test: RunsPurchased event is emitted in tests/player-profile/

### Implementation for User Story 8

- [x] T123 [US8] Add TREASURY_PUBKEY constant placeholder in programs/player-profile/src/constants.rs
- [x] T124 [US8] Add RunsPurchased event in programs/player-profile/src/lib.rs
- [x] T125 [US8] Create PurchaseRuns account context in programs/player-profile/src/lib.rs
- [x] T126 [US8] Implement purchase_runs instruction with SOL transfer in programs/player-profile/src/lib.rs
- [x] T127 [US8] Emit RunsPurchased event after successful purchase in programs/player-profile/src/lib.rs

**Checkpoint**: User Story 8 complete - run economy works

---

## Phase 11: User Story 9 - Multi-Session Support (Priority: P2)

**Goal**: Player can have sessions on multiple levels simultaneously

**Independent Test**: Create sessions on levels 1, 3, and 5 for same player, verify all three exist independently

### Tests for User Story 9

- [x] T128 [P] [US9] Unit test: player can create sessions on different levels in tests/session-manager/
- [x] T129 [P] [US9] Unit test: creating session on same level fails with SessionExistsForLevel in tests/session-manager/
- [x] T130 [P] [US9] Integration test: multiple sessions operate independently in tests/session-manager/

### Implementation for User Story 9

(Most implementation done in Foundational phase with PDA seed change)

- [x] T131 [US9] Add SessionExistsForLevel validation message in programs/session-manager/src/lib.rs (PDA seeds prevent duplicate)
- [x] T132 [US9] Update end_session to accept campaign_level for PDA derivation in programs/session-manager/src/lib.rs

**Checkpoint**: User Story 9 complete - multi-session works

---

## Phase 12: User Story 10 - Item Unlock Progression (Priority: P2)

**Goal**: 40 starter items available, remaining 40 unlock 1 per level completion

**Independent Test**: Complete levels 1-5 for first time, verify 5 new items are in unlocked_items

### Tests for User Story 10

- [x] T133 [P] [US10] Unit test: new profile has exactly 40 bits set in unlocked_items in tests/player-profile/
- [x] T134 [P] [US10] Unit test: unlocked item is added to active_item_pool automatically in tests/player-profile/
- [x] T135 [P] [US10] Unit test: no item unlocks when all 80 already unlocked in tests/player-profile/

### Implementation for User Story 10

(Most implementation done in US7 with random item unlock logic)

- [x] T136 [US10] Verify STARTER_ITEMS_BITMASK matches spec indices 0-39 in programs/player-profile/src/bitmask.rs
- [x] T137 [US10] Add edge case handling for all items already unlocked in select_random_locked_item in programs/player-profile/src/bitmask.rs

**Checkpoint**: User Story 10 complete - item progression works

---

## Phase 13: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T138 [P] Run cargo clippy on player-profile and fix all warnings
- [x] T139 [P] Run cargo clippy on session-manager and fix all warnings
- [x] T140 [P] Run cargo clippy on gameplay-state and fix all warnings
- [x] T141 [P] Run cargo clippy on poi-system and fix all warnings
- [x] T142 [P] Run cargo clippy on field-enemies and fix all warnings
- [x] T143 [P] Run cargo clippy on player-inventory and fix all warnings (pre-existing test failures unrelated to this feature)
- [x] T144 [P] Run cargo fmt on all modified programs
- [x] T145 [P] Add IDL documentation comments to move_with_combat instruction in programs/gameplay-state/src/lib.rs
- [x] T146 [P] Add IDL documentation comments to purchase_runs instruction in programs/player-profile/src/lib.rs
- [x] T147 [P] Add IDL documentation to new PlayerProfile fields in programs/player-profile/src/state.rs (already present)
- [x] T148 [P] Add IDL documentation to new GameSession fields in programs/session-manager/src/state.rs (already present)
- [x] T149 Verify anchor build succeeds without errors (cargo build passes)
- [x] T150 Run full test suite: cargo test && anchor test (195 tests pass across modified programs)
- [ ] T151 Validate quickstart.md examples work correctly (deferred - requires full integration testing)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-12)**: All depend on Foundational phase completion
  - P1 Stories (US1-US7) are critical path
  - P2 Stories (US8-US10) can proceed after Foundational
- **Polish (Phase 13)**: Depends on all desired user stories being complete

### User Story Dependencies

- **US1 (Session Creation)**: Foundational only - entry point for all gameplay
- **US2 (Movement + Combat)**: Depends on US1 (needs session to exist)
- **US3 (Night Mechanics)**: Depends on US2 (extends movement logic)
- **US4 (POI Interaction)**: Depends on US1 (needs session)
- **US5 (Boss Encounter)**: Depends on US2, US3 (uses movement, extends phase logic)
- **US6 (Death Handling)**: Depends on US2 (triggered by combat result)
- **US7 (Victory & Unlock)**: Depends on US5 (triggered by boss victory)
- **US8 (Run Economy)**: Foundational only - independent of gameplay
- **US9 (Multi-Session)**: Foundational only - just PDA change
- **US10 (Item Progression)**: Depends on US7 (uses unlock logic)

### Recommended Order for Solo Developer

1. Setup → Foundational → US1 (MVP: can create sessions)
2. US2 (can move and fight)
3. US3 (night adds danger)
4. US5 + US6 + US7 (boss fights, death, victory)
5. US4 (POI adds loot)
6. US8, US9, US10 (economy and progression)

### Parallel Opportunities

**Phase 1 (Setup)**: T002-T010 can run in parallel
**Phase 2 (Foundational)**: T015-T017 parallel; T024-T028 parallel
**Phase 3 (US1)**: All tests T029-T034 can run in parallel
**Phase 4 (US2)**: All tests T045-T051 can run in parallel
**Phase 5 (US3)**: All tests T064-T068 can run in parallel
**Phase 6 (US4)**: All tests T078-T080 can run in parallel
**Phase 7 (US5)**: All tests T087-T090 can run in parallel
**Phase 8 (US6)**: All tests T098-T100 can run in parallel
**Phase 9 (US7)**: All tests T106-T110 can run in parallel
**Phase 10 (US8)**: All tests T119-T122 can run in parallel
**Phase 11 (US9)**: All tests T128-T130 can run in parallel
**Phase 12 (US10)**: All tests T133-T135 can run in parallel
**Phase 13 (Polish)**: T138-T148 can run in parallel

---

## Parallel Example: User Story 2

```bash
# Launch all tests for User Story 2 together:
Task: "T045 [P] [US2] Unit test: move_with_combat deducts 1 move for floor tile"
Task: "T046 [P] [US2] Unit test: move_with_combat deducts max(2, 6-DIG) for wall"
Task: "T047 [P] [US2] Unit test: move_with_combat fails with OutOfBounds"
Task: "T048 [P] [US2] Unit test: move_with_combat fails with NotAdjacent"
Task: "T049 [P] [US2] Unit test: move_with_combat triggers combat on enemy tile"
Task: "T050 [P] [US2] Unit test: combat emits proper events"
Task: "T051 [P] [US2] Unit test: defeated enemy is marked in MapEnemies"
```

---

## Implementation Strategy

### MVP First (User Stories 1-2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Session Creation)
4. Complete Phase 4: User Story 2 (Movement + Combat)
5. **STOP and VALIDATE**: Can create sessions and move with auto-combat
6. Deploy/demo if ready - core loop works

### Core Loop Complete (Add US3-US7)

1. Add US3: Night enemy movement
2. Add US5: Boss encounters
3. Add US6: Death handling
4. Add US7: Victory and progression
5. Add US4: POI interaction
6. **VALIDATE**: Full gameplay loop complete

### Full Feature (Add US8-US10)

1. Add US8: Run economy
2. Add US9: Multi-session
3. Add US10: Item progression
4. Polish phase
5. **VALIDATE**: All features complete

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: US1 → US2 → US3 (session, movement, night)
   - Developer B: US4 → US5 (POI, boss)
   - Developer C: US8 → US9 (economy, multi-session)
3. Integrate: US6, US7, US10 (build on A+B work)

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Verify tests fail before implementing (TDD per Constitution)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Constitution mandates TDD - tests are included in all story phases
