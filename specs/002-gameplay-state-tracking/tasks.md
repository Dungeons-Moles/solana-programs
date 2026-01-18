# Tasks: Gameplay State Tracking

**Input**: Design documents from `/specs/002-gameplay-state-tracking/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3, US4)
- Include exact file paths in descriptions

---

## Phase 1: Setup (Program Scaffolding)

**Purpose**: Initialize the new gameplay-state program within the existing workspace

- [x] T001 Create program directory structure at programs/gameplay-state/
- [x] T002 Create Cargo.toml for gameplay-state program with anchor-lang 0.32.0 dependency in programs/gameplay-state/Cargo.toml
- [x] T003 Add gameplay-state to workspace members in Cargo.toml at repository root
- [x] T004 [P] Create constants module at programs/gameplay-state/src/constants.rs with DAY_MOVES, NIGHT_MOVES, DEFAULT_STATS, etc.
- [x] T005 [P] Create errors module at programs/gameplay-state/src/errors.rs with GameplayStateError enum per RD-007
- [x] T006 Create state module at programs/gameplay-state/src/state.rs with GameState struct and Phase enum per data-model.md

---

## Phase 2: Foundational (Core Infrastructure)

**Purpose**: Set up the base program structure and PDA derivation

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T007 Create lib.rs entry point at programs/gameplay-state/src/lib.rs with declare_id! and module declarations
- [x] T008 Add GameState PDA derivation constant (SEED_PREFIX = "game_state") in programs/gameplay-state/src/state.rs
- [x] T009 Implement Phase::moves_allowed() and Phase::next() methods in programs/gameplay-state/src/state.rs per RD-003
- [x] T010 Add StatType enum with Hp, MaxHp, Atk, Arm, Spd, Dig variants in programs/gameplay-state/src/state.rs
- [x] T011 Run `anchor build` to verify program compiles

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Player Movement Tracking (Priority: P1) 🎯 MVP

**Goal**: Track player position and movement on-chain with floor/wall tile costs

**Independent Test**: Initialize game state, execute move commands, verify position and moves_remaining update correctly

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation (Constitution II)**

- [x] T012 [P] [US1] Create test file at tests/gameplay-state.ts with test setup and helper functions
- [x] T013 [P] [US1] Write test: floor movement deducts 1 move in tests/gameplay-state.ts
- [x] T014 [P] [US1] Write test: wall dig deducts max(2, 6-DIG) moves in tests/gameplay-state.ts
- [x] T015 [P] [US1] Write test: out-of-bounds movement rejected in tests/gameplay-state.ts
- [x] T016 [P] [US1] Write test: insufficient moves rejected in tests/gameplay-state.ts
- [x] T017 [P] [US1] Write test: non-adjacent movement rejected in tests/gameplay-state.ts

### Implementation for User Story 1

- [x] T018 [US1] Implement InitializeGameState account context in programs/gameplay-state/src/lib.rs per contracts/gameplay-state.idl.md
- [x] T019 [US1] Implement initialize_game_state instruction handler with position, map bounds, and default stats in programs/gameplay-state/src/lib.rs
- [x] T020 [US1] Add GameStateInitialized event emission in initialize_game_state instruction
- [x] T021 [US1] Implement MovePlayer account context with player signer validation in programs/gameplay-state/src/lib.rs
- [x] T022 [US1] Implement move_player instruction with adjacency validation in programs/gameplay-state/src/lib.rs
- [x] T023 [US1] Add bounds checking (target_x < map_width, target_y < map_height) in move_player
- [x] T024 [US1] Implement floor tile movement (deduct 1 move) in move_player
- [x] T025 [US1] Implement wall tile digging with max(2, 6-DIG) cost formula in move_player per FR-004
- [x] T026 [US1] Add insufficient moves validation (reject if moves_remaining < cost) per FR-005
- [x] T027 [US1] Update total_moves counter on each successful move per FR-006
- [x] T028 [US1] Add PlayerMoved event emission with from/to coordinates and is_dig flag
- [x] T029 [US1] Run tests to verify all US1 acceptance scenarios pass

**Checkpoint**: User Story 1 complete - position tracking works independently

---

## Phase 4: User Story 2 - Time and Phase Progression (Priority: P2)

**Goal**: Track day/night cycle and week progression with automatic phase advancement

**Independent Test**: Initialize in Day 1, consume 50 moves, verify transition to Night 1 with 30 moves

### Tests for User Story 2

- [x] T030 [P] [US2] Write test: Day phase has 50 moves in tests/gameplay-state.ts
- [x] T031 [P] [US2] Write test: Night phase has 30 moves in tests/gameplay-state.ts
- [x] T032 [P] [US2] Write test: phase advances when moves_remaining reaches 0 in tests/gameplay-state.ts
- [x] T033 [P] [US2] Write test: week advances after Night3 in tests/gameplay-state.ts
- [x] T034 [P] [US2] Write test: boss_fight_ready triggers at Week 3 Night 3 end in tests/gameplay-state.ts

### Implementation for User Story 2

- [x] T035 [US2] Add automatic phase advancement logic in move_player when moves_remaining reaches 0 per RD-006
- [x] T036 [US2] Implement Phase::next() transition chain (Day1→Night1→Day2→...) in move_player
- [x] T037 [US2] Add week increment logic when Night3 completes (week < 3) per FR-011
- [x] T038 [US2] Set boss_fight_ready = true when Week 3 Night 3 completes per FR-012
- [x] T039 [US2] Add BossFightAlreadyTriggered error check - reject moves after boss_fight_ready
- [x] T040 [US2] Add PhaseAdvanced event emission with new_phase, new_week, moves_remaining
- [x] T041 [US2] Add BossFightReady event emission when triggered
- [x] T042 [US2] Run tests to verify all US2 acceptance scenarios pass

**Checkpoint**: User Story 2 complete - time progression works independently

---

## Phase 5: User Story 3 - Player Stats Management (Priority: P3)

**Goal**: Track and modify player stats (HP, ATK, ARM, SPD, DIG) with proper bounds

**Independent Test**: Initialize with default stats, call modify_stat, verify bounds and persistence

### Tests for User Story 3

- [x] T043 [P] [US3] Write test: default stats initialized correctly (HP=10, ATK=1, ARM=0, SPD=0, DIG=1) in tests/gameplay-state.ts
- [x] T044 [P] [US3] Write test: HP modification works with positive and negative deltas in tests/gameplay-state.ts
- [x] T045 [P] [US3] Write test: HP cannot go below 0 in tests/gameplay-state.ts
- [x] T046 [P] [US3] Write test: other stats allow negative values in tests/gameplay-state.ts
- [x] T047 [P] [US3] Write test: stat overflow is prevented in tests/gameplay-state.ts

### Implementation for User Story 3

- [x] T048 [US3] Implement ModifyStat account context with player signer validation in programs/gameplay-state/src/lib.rs
- [x] T049 [US3] Implement modify_stat instruction handler with StatType and delta parameters
- [x] T050 [US3] Add HP bounds checking (0 ≤ hp ≤ max_hp) per FR-016
- [x] T051 [US3] Allow negative values for ATK, ARM, SPD, DIG per FR-017
- [x] T052 [US3] Add checked arithmetic for overflow prevention per RD-007
- [x] T053 [US3] Add StatModified event emission with stat type, old_value, new_value
- [x] T054 [US3] Run tests to verify all US3 acceptance scenarios pass

**Checkpoint**: User Story 3 complete - stats management works independently

---

## Phase 6: User Story 4 - Gear Slots Progression (Priority: P4)

**Goal**: Gear slots increase from 4 → 6 → 8 at week boundaries

**Independent Test**: Simulate week transitions, verify gear_slots increases correctly

### Tests for User Story 4

- [x] T055 [P] [US4] Write test: gear_slots initialized to 4 in tests/gameplay-state.ts
- [x] T056 [P] [US4] Write test: gear_slots increases to 6 after Week 1 in tests/gameplay-state.ts
- [x] T057 [P] [US4] Write test: gear_slots increases to 8 after Week 2 in tests/gameplay-state.ts
- [x] T058 [P] [US4] Write test: gear_slots capped at 8 after Week 3 in tests/gameplay-state.ts

### Implementation for User Story 4

- [x] T059 [US4] Initialize gear_slots to INITIAL_GEAR_SLOTS (4) in initialize_game_state per FR-019
- [x] T060 [US4] Add gear_slots += 2 on week transition (already in phase advancement logic) per FR-020/FR-021
- [x] T061 [US4] Add gear_slots cap at MAX_GEAR_SLOTS (8) using .min(8) per FR-022
- [x] T062 [US4] Run tests to verify all US4 acceptance scenarios pass

**Checkpoint**: User Story 4 complete - gear slots progression works independently

---

## Phase 7: Session Integration & Cleanup

**Purpose**: Integrate with session-manager and add close instruction

### Tests for Session Integration

- [x] T063 [P] Write test: GameState requires valid GameSession PDA in tests/gameplay-state.ts
- [x] T064 [P] Write test: only session owner can modify game state in tests/gameplay-state.ts
- [x] T065 [P] Write test: close_game_state returns rent to player in tests/gameplay-state.ts

### Implementation for Session Integration

- [x] T066 Verify GameSession existence via account in InitializeGameState context per FR-023
- [x] T067 Add has_one = player constraint on GameState account contexts per FR-024
- [x] T068 Implement CloseGameState account context with close = player constraint per FR-025
- [x] T069 Implement close_game_state instruction handler
- [x] T070 Add GameStateClosed event emission with total_moves, final_phase, final_week
- [x] T071 Run tests to verify session integration scenarios pass

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final validation, documentation, and cleanup

- [x] T072 Add gameplay_state entry to Anchor.toml with program ID
- [x] T073 Run `anchor build` and verify IDL generation
- [x] T074 Run `cargo clippy` on gameplay-state program - fix any warnings
- [x] T075 Run `cargo test` for any Rust unit tests
- [x] T076 Run full `anchor test` suite
- [x] T077 Verify GameState account size is under 200 bytes per SC-007
- [x] T078 Run quickstart.md verification steps
- [x] T079 Verify all acceptance scenarios from spec.md pass

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-6)**: All depend on Foundational phase completion
  - US1 (Movement) should complete first as MVP
  - US2 (Phase/Time) depends on US1 move_player instruction existing
  - US3 (Stats) independent - can parallel with US2
  - US4 (Gear Slots) depends on US2 (week transitions)
- **Session Integration (Phase 7)**: Depends on US1 complete
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: No dependencies on other stories - MVP baseline
- **User Story 2 (P2)**: Enhances move_player from US1 (add phase advancement)
- **User Story 3 (P3)**: Independent - can parallel with US2
- **User Story 4 (P4)**: Uses week transition logic from US2

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Constitution II)
- Account contexts before instruction handlers
- Core logic before events
- Story complete before moving to next priority

### Parallel Opportunities

**Phase 1 (Setup):**

```
T004, T005 can run in parallel (different files)
```

**Phase 3 (US1 Tests):**

```
T012-T017 can all run in parallel
```

**Phase 4 (US2 Tests):**

```
T030-T034 can all run in parallel
```

**Phase 5 (US3 Tests):**

```
T043-T047 can all run in parallel
```

**Phase 6 (US4 Tests):**

```
T055-T058 can all run in parallel
```

**Phase 7 (Session Tests):**

```
T063-T065 can all run in parallel
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T006)
2. Complete Phase 2: Foundational (T007-T011)
3. Complete Phase 3: User Story 1 (T012-T029)
4. **STOP and VALIDATE**: Initialize game state, make moves, verify position tracking
5. Deploy/demo if ready

### Incremental Delivery

1. **MVP**: Setup + Foundational + US1 → Players can track movement
2. **+US2**: Add time progression → Day/night cycle works
3. **+US3**: Add stats management → Full stats tracking
4. **+US4**: Add gear slots → Complete gameplay state
5. **+Session Integration**: Full integration with session-manager
6. **+Polish**: Production ready

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Constitution II requires tests written FIRST and FAILING before implementation
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
