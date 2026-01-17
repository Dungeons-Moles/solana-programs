---
description: "Task list for Solana core programs"
---

# Tasks: Solana Core Programs

**Input**: Design documents from `/specs/001-solana-core-programs/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, quickstart.md

**Tests**: Included per Constitution requirement (TDD with 80% minimum coverage)

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Programs**: `programs/<program-name>/src/`
- **Tests**: `tests/<program-name>.ts`
- **Config**: Root level (`Anchor.toml`, `Cargo.toml`)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Anchor workspace initialization and project structure

- [x] T001 Initialize Anchor workspace with `anchor init` at repository root (`Anchor.toml`, `Cargo.toml`)
- [x] T002 Configure `Anchor.toml` with three programs (player_profile, session_manager, map_generator) in `Anchor.toml`
- [x] T003 [P] Configure root workspace members in `Cargo.toml`
- [x] T004 [P] Create `programs/player-profile/Cargo.toml` with `anchor-lang` dependency
- [x] T005 [P] Create `programs/session-manager/Cargo.toml` with `anchor-lang` and `ephemeral_rollups_sdk` dependencies
- [x] T006 [P] Create `programs/map-generator/Cargo.toml` with `anchor-lang` dependency
- [x] T007 [P] Configure rustfmt and clippy settings in `rustfmt.toml` and `.clippy.toml`
- [x] T008 [P] Create `.gitignore` entries for Anchor artifacts (target/, .anchor/, node_modules/)
- [x] T009 Add test dependencies in `package.json` for Anchor/Chai/Mocha
- [x] T010 Create `tsconfig.json` for TypeScript tests

**Checkpoint**: Anchor workspace builds successfully with `anchor build`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared types, constants, and error definitions that ALL programs depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T011 [P] Define PlayerProfileError enum in `programs/player-profile/src/errors.rs`
- [x] T012 [P] Define SessionManagerError enum in `programs/session-manager/src/errors.rs`
- [x] T013 [P] Define MapGeneratorError enum in `programs/map-generator/src/errors.rs`
- [x] T014 [P] Define player-profile constants (MAX_NAME_LENGTH, INITIAL_LEVEL, INITIAL_TOTAL_RUNS, INITIAL_AVAILABLE_RUNS) in `programs/player-profile/src/constants.rs`
- [x] T015 [P] Define session-manager constants (EMPTY_STATE_HASH) in `programs/session-manager/src/constants.rs`
- [x] T016 [P] Define map-generator constants (MAX_LEVEL, DEFAULT_SEEDS) in `programs/map-generator/src/constants.rs`
- [x] T017 Implement SeededRNG struct in `programs/map-generator/src/rng.rs` (XorShift algorithm matching TypeScript)
- [x] T018 Write unit tests for SeededRNG determinism in `programs/map-generator/src/rng.rs`
- [x] T019 Run `anchor build` to verify all programs compile (`Anchor.toml`, `Cargo.toml`)
- [x] T020 Run `cargo clippy` and fix warnings across programs (`Cargo.toml`)

**Checkpoint**: Foundation ready - all programs compile, shared types defined

---

## Phase 3: User Story 1 - New Player Registration (Priority: P1) 🎯 MVP

**Goal**: Players can create profiles and set display names on-chain

**Independent Test**: Connect wallet, call initialize_profile, verify PDA created with correct data

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T021 [P] [US1] Write test "initializes player profile" in `tests/player-profile.ts`
- [x] T022 [P] [US1] Write test "rejects duplicate profile creation" in `tests/player-profile.ts`
- [x] T023 [P] [US1] Write test "updates profile name" in `tests/player-profile.ts`
- [x] T024 [P] [US1] Write test "rejects name longer than 32 chars" in `tests/player-profile.ts`

### Implementation for User Story 1

- [x] T025 [US1] Define PlayerProfile account struct in `programs/player-profile/src/state.rs`
- [x] T026 [US1] Implement InitializeProfile context in `programs/player-profile/src/lib.rs`
- [x] T027 [US1] Implement initialize_profile instruction handler in `programs/player-profile/src/lib.rs`
- [x] T028 [US1] Implement UpdateProfileName context in `programs/player-profile/src/lib.rs`
- [x] T029 [US1] Implement update_profile_name instruction handler in `programs/player-profile/src/lib.rs`
- [x] T030 [US1] Add ProfileCreated event emission in `programs/player-profile/src/lib.rs`
- [x] T031 [US1] Run tests in `tests/player-profile.ts` and verify US1 scenarios

**Checkpoint**: Player profile creation and name updates work independently

---

## Phase 4: User Story 2 - Available Run Tracking (Priority: P2)

**Goal**: Players start with 40 available runs and consume one per run

**Independent Test**: Create profile, record run results, verify available_runs decrements and zero blocks runs

### Tests for User Story 2

- [x] T032 [P] [US2] Write test "initializes available_runs to 40" in `tests/player-profile.ts`
- [x] T033 [P] [US2] Write test "decrements available_runs on run completion" in `tests/player-profile.ts`
- [x] T034 [P] [US2] Write test "rejects run completion with zero available_runs" in `tests/player-profile.ts`

### Implementation for User Story 2

- [x] T035 [US2] Add available_runs field to PlayerProfile in `programs/player-profile/src/state.rs`
- [x] T036 [US2] Initialize available_runs during profile creation in `programs/player-profile/src/lib.rs`
- [x] T037 [US2] Add NoAvailableRuns error in `programs/player-profile/src/errors.rs`
- [x] T038 [US2] Decrement available_runs in record_run_result in `programs/player-profile/src/lib.rs`
- [x] T039 [US2] Include available_runs in RunCompleted event in `programs/player-profile/src/lib.rs`
- [x] T040 [US2] Run tests in `tests/player-profile.ts` and verify US2 scenarios

**Checkpoint**: Run allowance tracking works and prevents runs at zero

---

## Phase 5: User Story 3 - Game Session Delegation (Priority: P3)

**Goal**: Sessions delegate to MagicBlock ephemeral rollup for real-time gameplay

**Independent Test**: Start session, delegate to rollup, commit state, end session

### Tests for User Story 3

- [x] T041 [P] [US3] Write test "initializes session counter" in `tests/session-manager.ts`
- [x] T042 [P] [US3] Write test "starts new game session" in `tests/session-manager.ts`
- [x] T043 [P] [US3] Write test "rejects second session for same player" in `tests/session-manager.ts`
- [x] T044 [P] [US3] Write test "delegates session to ephemeral rollup" in `tests/session-manager.ts`
- [x] T045 [P] [US3] Write test "commits session state" in `tests/session-manager.ts`
- [x] T046 [P] [US3] Write test "ends session and closes account" in `tests/session-manager.ts`
- [x] T047 [P] [US3] Write test "force closes session without timeout" in `tests/session-manager.ts`

### Implementation for User Story 3

- [x] T048 [US3] Define GameSession account struct in `programs/session-manager/src/state.rs`
- [x] T049 [US3] Define SessionCounter account struct in `programs/session-manager/src/state.rs`
- [x] T050 [US3] Implement InitializeCounter context in `programs/session-manager/src/lib.rs`
- [x] T051 [US3] Implement initialize_counter instruction handler in `programs/session-manager/src/lib.rs`
- [x] T052 [US3] Implement StartSession context in `programs/session-manager/src/lib.rs`
- [x] T053 [US3] Implement start_session instruction handler with session_id increment in `programs/session-manager/src/lib.rs`
- [x] T054 [US3] Create delegation.rs module for MagicBlock integration in `programs/session-manager/src/delegation.rs`
- [x] T055 [US3] Implement DelegateSession context with MagicBlock accounts in `programs/session-manager/src/lib.rs`
- [ ] T056 [US3] Implement delegate_session instruction using `delegate_account` in `programs/session-manager/src/lib.rs`
- [x] T057 [US3] Implement CommitSession context in `programs/session-manager/src/lib.rs`
- [ ] T058 [US3] Implement commit_session instruction using `commit_accounts` in `programs/session-manager/src/lib.rs`
- [x] T059 [US3] Implement EndSession context in `programs/session-manager/src/lib.rs`
- [ ] T060 [US3] Implement end_session instruction using `commit_and_undelegate_accounts` in `programs/session-manager/src/lib.rs`
- [x] T061 [US3] Implement ForceCloseSession context without timeout validation in `programs/session-manager/src/lib.rs`
- [x] T062 [US3] Implement force_close_session instruction without timeout checks in `programs/session-manager/src/lib.rs`
- [x] T063 [US3] Add SessionStarted, SessionDelegated, SessionEnded event emissions in `programs/session-manager/src/lib.rs`
- [x] T064 [US3] Run tests in `tests/session-manager.ts` and verify US3 scenarios

**Checkpoint**: Full session lifecycle works with MagicBlock delegation

---

## Phase 6: User Story 4 - Map Generation (Priority: P4)

**Goal**: Deterministic map generation from configurable seeds

**Independent Test**: Get seed for level, generate map off-chain, verify hash matches

### Tests for User Story 4

- [x] T065 [P] [US4] Write test "initializes map config with default seeds" in `tests/map-generator.ts`
- [x] T066 [P] [US4] Write test "returns correct seed for level" in `tests/map-generator.ts`
- [x] T067 [P] [US4] Write test "admin can update seed mapping" in `tests/map-generator.ts`
- [x] T068 [P] [US4] Write test "batch updates multiple seeds" in `tests/map-generator.ts`
- [x] T069 [P] [US4] Write test "rejects invalid level number" in `tests/map-generator.ts`
- [x] T070 [P] [US4] Write test "verifies map hash" in `tests/map-generator.ts`
- [x] T071 [P] [US4] Write test "RNG produces deterministic results" in `tests/map-generator.ts`

### Implementation for User Story 4

- [x] T072 [US4] Define MapConfig account struct in `programs/map-generator/src/state.rs`
- [x] T073 [US4] Define SeedUpdate, TileType, Position types in `programs/map-generator/src/state.rs`
- [x] T074 [US4] Implement InitializeMapConfig context in `programs/map-generator/src/lib.rs`
- [x] T075 [US4] Implement initialize_map_config with default seeds (0-80) in `programs/map-generator/src/lib.rs`
- [x] T076 [US4] Implement UpdateMapConfig context (admin only) in `programs/map-generator/src/lib.rs`
- [x] T077 [US4] Implement update_map_config instruction handler in `programs/map-generator/src/lib.rs`
- [x] T078 [US4] Implement BatchUpdateMapConfig context in `programs/map-generator/src/lib.rs`
- [x] T079 [US4] Implement batch_update_map_config instruction handler in `programs/map-generator/src/lib.rs`
- [x] T080 [US4] Implement GetMapSeed view function in `programs/map-generator/src/lib.rs`
- [x] T081 [US4] Implement VerifyMapHash instruction in `programs/map-generator/src/lib.rs`
- [x] T082 [US4] Create generator.rs with off-chain map generation logic in `programs/map-generator/src/generator.rs`
- [x] T083 [US4] Implement corridor maze generation algorithm in `programs/map-generator/src/generator.rs`
- [x] T084 [US4] Implement POI placement logic in `programs/map-generator/src/generator.rs`
- [x] T085 [US4] Implement enemy spawning with zone-based tier selection in `programs/map-generator/src/generator.rs`
- [x] T086 [US4] Add MapConfigUpdated, AdminTransferred events in `programs/map-generator/src/lib.rs`
- [x] T087 [US4] Implement TransferAdmin instruction for admin authority transfer in `programs/map-generator/src/lib.rs`
- [x] T088 [US4] Run tests in `tests/map-generator.ts` and verify US4 scenarios

**Checkpoint**: Map config works, seeds configurable, generation is deterministic

---

## Phase 7: User Story 5 - Run Completion Tracking (Priority: P5)

**Goal**: Track run completions and campaign progression

**Independent Test**: Complete run, verify total_runs increments, verify level advances on victory

### Tests for User Story 5

- [x] T089 [P] [US5] Write test "records run completion" in `tests/player-profile.ts`
- [x] T090 [P] [US5] Write test "increments total_runs on completion" in `tests/player-profile.ts`
- [x] T091 [P] [US5] Write test "advances level on victory" in `tests/player-profile.ts`
- [x] T092 [P] [US5] Write test "does not advance level on defeat" in `tests/player-profile.ts`

### Implementation for User Story 5

- [x] T093 [US5] Implement RecordRunResult context in `programs/player-profile/src/lib.rs`
- [x] T094 [US5] Implement record_run_result to increment total_runs in `programs/player-profile/src/lib.rs`
- [x] T095 [US5] Add victory check for level advancement in `programs/player-profile/src/lib.rs`
- [x] T096 [US5] Emit RunCompleted event details in `programs/player-profile/src/lib.rs`
- [x] T097 [US5] Run tests in `tests/player-profile.ts` and verify US5 scenarios

**Checkpoint**: Run tracking complete, progression advances on victory

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final quality checks and documentation

- [x] T098 [P] Run `cargo clippy` on all programs and fix warnings (`Cargo.toml`)
- [x] T099 [P] Run `cargo audit` and address advisories (`Cargo.toml`)
- [x] T100 [P] Generate IDL files with `anchor build` and verify against `target/idl/`
- [x] T101 [P] Validate quickstart steps in `specs/001-solana-core-programs/quickstart.md`
- [x] T102 Create deployment script in `migrations/deploy.ts`
- [x] T103 Run full test suite with `anchor test` and verify coverage for `tests/`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase
  - US1 (P1): No dependencies on other stories
  - US2 (P2): Depends on US1 (needs PlayerProfile account)
  - US3 (P3): Depends on US1 (needs player profile existence check)
  - US4 (P4): Independent of other stories
  - US5 (P5): Depends on US1 + US2 (uses available_runs and profile)
- **Polish (Phase 8)**: Depends on all desired user stories complete

### User Story Dependencies

```
US1 (Player Registration) ──┬──> US2 (Available Run Tracking)
                            ├──> US3 (Session Delegation)
                            └──> US5 (Run Completion)
US2 (Available Runs) ────────┘

US4 (Map Generation) ──────────> Independent
```

### Parallel Opportunities

- **Phase 1**: T003-T008 can run in parallel
- **Phase 2**: T011-T016 can run in parallel, then T017-T020 sequential
- **Phase 3-7**: All test tasks within a phase can run in parallel
- **Phase 3-7**: Once tests pass, implementation tasks follow dependency order
- **US3 and US4**: Can be developed in parallel (different programs)

---

## Parallel Example: User Story 2

```bash
# Launch all tests for User Story 2 together:
Task: "Write test \"initializes available_runs to 40\" in tests/player-profile.ts"
Task: "Write test \"decrements available_runs on run completion\" in tests/player-profile.ts"
Task: "Write test \"rejects run completion with zero available_runs\" in tests/player-profile.ts"

# Launch data model updates for User Story 2 together:
Task: "Add available_runs field to PlayerProfile in programs/player-profile/src/state.rs"
Task: "Add NoAvailableRuns error in programs/player-profile/src/errors.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (Player Registration)
4. **STOP and VALIDATE**: Test User Story 1 independently
5. Demo MVP capability

### Incremental Delivery

1. Setup + Foundational → Build passes
2. User Story 1 → Profile creation works → **MVP!**
3. User Story 2 → Available runs tracking works
4. User Story 3 → Session delegation works → Game sessions enabled
5. User Story 4 → Map generation works → Core gameplay ready
6. User Story 5 → Run completion tracking works → Full progression ready

### Suggested MVP Scope

**Minimum**: Phase 1 + Phase 2 + Phase 3 (User Story 1)

- Players can create profiles and set names
- Foundation for all other features
- ~31 tasks, testable end-to-end

---

## Task Summary

| Phase     | User Story           | Task Count | Parallel Tasks |
| --------- | -------------------- | ---------- | -------------- |
| 1         | Setup                | 10         | 6              |
| 2         | Foundational         | 10         | 6              |
| 3         | US1 - Registration   | 11         | 4              |
| 4         | US2 - Available Runs | 9          | 3              |
| 5         | US3 - Sessions       | 24         | 7              |
| 6         | US4 - Map Gen        | 24         | 7              |
| 7         | US5 - Run Tracking   | 9          | 4              |
| 8         | Polish               | 6          | 4              |
| **Total** |                      | **103**    | **41**         |

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story
- Tests use TDD approach per Constitution (write tests first, verify they fail)
- Constitution requires 80% test coverage minimum
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
