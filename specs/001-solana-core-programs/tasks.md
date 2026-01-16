# Tasks: Solana Core Programs

**Input**: Design documents from `/specs/001-solana-core-programs/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

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

- [X] T001 Initialize Anchor workspace with `anchor init` at repository root
- [X] T002 Configure Anchor.toml with three programs (player_profile, session_manager, map_generator)
- [X] T003 [P] Configure root Cargo.toml as workspace with member programs
- [X] T004 [P] Create programs/player-profile/Cargo.toml with anchor-lang dependency
- [X] T005 [P] Create programs/session-manager/Cargo.toml with anchor-lang and ephemeral_rollups_sdk dependencies
- [X] T006 [P] Create programs/map-generator/Cargo.toml with anchor-lang dependency
- [X] T007 [P] Configure rustfmt.toml and .clippy.toml for code quality
- [X] T008 [P] Create .gitignore for Anchor project (target/, .anchor/, node_modules/)
- [X] T009 Setup package.json with test dependencies (@coral-xyz/anchor, chai, mocha)
- [X] T010 Create tsconfig.json for TypeScript tests

**Checkpoint**: Anchor workspace builds successfully with `anchor build`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared types, constants, and error definitions that ALL programs depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T011 [P] Define PlayerProfileError enum in programs/player-profile/src/errors.rs
- [X] T012 [P] Define SessionManagerError enum in programs/session-manager/src/errors.rs
- [X] T013 [P] Define MapGeneratorError enum in programs/map-generator/src/errors.rs
- [X] T014 [P] Define constants (TIER_UNLOCK_COST, LEVELS_PER_TIER, SESSION_TIMEOUT) in programs/player-profile/src/constants.rs
- [X] T015 [P] Define constants (MAX_LEVEL, DEFAULT_SEEDS) in programs/map-generator/src/constants.rs
- [X] T016 Implement SeededRNG struct in programs/map-generator/src/rng.rs (XorShift algorithm matching TypeScript)
- [X] T017 Write unit tests for SeededRNG determinism in programs/map-generator/src/rng.rs
- [X] T018 Run `anchor build` to verify all programs compile without errors
- [X] T019 Run `cargo clippy` and fix any warnings across all programs

**Checkpoint**: Foundation ready - all programs compile, shared types defined

---

## Phase 3: User Story 1 - New Player Registration (Priority: P1) 🎯 MVP

**Goal**: Players can create profiles and set display names on-chain

**Independent Test**: Connect wallet, call initialize_profile, verify PDA created with correct data

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T020 [P] [US1] Write test "initializes player profile" in tests/player-profile.ts
- [X] T021 [P] [US1] Write test "rejects duplicate profile creation" in tests/player-profile.ts
- [X] T022 [P] [US1] Write test "updates profile name" in tests/player-profile.ts
- [X] T023 [P] [US1] Write test "rejects name longer than 32 chars" in tests/player-profile.ts

### Implementation for User Story 1

- [X] T024 [US1] Define PlayerProfile account struct in programs/player-profile/src/state.rs
- [X] T025 [US1] Implement InitializeProfile context in programs/player-profile/src/lib.rs
- [X] T026 [US1] Implement initialize_profile instruction handler in programs/player-profile/src/lib.rs
- [X] T027 [US1] Implement UpdateProfileName context in programs/player-profile/src/lib.rs
- [X] T028 [US1] Implement update_profile_name instruction handler in programs/player-profile/src/lib.rs
- [X] T029 [US1] Add ProfileCreated event emission in initialize_profile
- [X] T030 [US1] Run tests and verify all US1 tests pass

**Checkpoint**: Player profile creation and name updates work independently

---

## Phase 4: User Story 2 - Campaign Tier Unlocking (Priority: P2)

**Goal**: Players can pay 0.05 SOL to unlock next 40 campaign levels

**Independent Test**: Create profile at level 39, pay for unlock, verify tier increases

### Tests for User Story 2

- [X] T031 [P] [US2] Write test "initializes treasury account" in tests/player-profile.ts
- [X] T032 [P] [US2] Write test "unlocks tier with 0.05 SOL payment" in tests/player-profile.ts
- [X] T033 [P] [US2] Write test "rejects unlock before tier boundary" in tests/player-profile.ts
- [X] T034 [P] [US2] Write test "transfers SOL to treasury" in tests/player-profile.ts
- [X] T035 [P] [US2] Write test "admin can withdraw from treasury" in tests/player-profile.ts

### Implementation for User Story 2

- [X] T036 [US2] Define Treasury account struct in programs/player-profile/src/state.rs
- [X] T037 [US2] Implement InitializeTreasury context in programs/player-profile/src/lib.rs
- [X] T038 [US2] Implement initialize_treasury instruction handler in programs/player-profile/src/lib.rs
- [X] T039 [US2] Implement UnlockCampaignTier context with SOL transfer in programs/player-profile/src/lib.rs
- [X] T040 [US2] Implement unlock_campaign_tier instruction with checked arithmetic in programs/player-profile/src/lib.rs
- [X] T041 [US2] Add tier boundary validation (must be at level 39, 79, etc.)
- [X] T042 [US2] Add TierUnlocked event emission
- [X] T043 [US2] Implement WithdrawTreasury context (admin only) in programs/player-profile/src/lib.rs
- [X] T044 [US2] Implement withdraw_treasury instruction handler
- [X] T045 [US2] Run tests and verify all US2 tests pass

**Checkpoint**: Payment flow works, treasury receives SOL, tier unlocks correctly

---

## Phase 5: User Story 3 - Game Session Delegation (Priority: P3)

**Goal**: Sessions delegate to MagicBlock ephemeral rollup for real-time gameplay

**Independent Test**: Start session, delegate to rollup, commit state, end session

### Tests for User Story 3

- [X] T046 [P] [US3] Write test "initializes session counter" in tests/session-manager.ts
- [X] T047 [P] [US3] Write test "starts new game session" in tests/session-manager.ts
- [X] T048 [P] [US3] Write test "rejects second session for same player" in tests/session-manager.ts
- [X] T049 [P] [US3] Write test "delegates session to ephemeral rollup" in tests/session-manager.ts
- [X] T050 [P] [US3] Write test "commits session state" in tests/session-manager.ts
- [X] T051 [P] [US3] Write test "ends session and closes account" in tests/session-manager.ts
- [X] T052 [P] [US3] Write test "force closes timed-out session" in tests/session-manager.ts

### Implementation for User Story 3

- [X] T053 [US3] Define GameSession account struct in programs/session-manager/src/state.rs
- [X] T054 [US3] Define SessionCounter account struct in programs/session-manager/src/state.rs
- [X] T055 [US3] Implement InitializeCounter context in programs/session-manager/src/lib.rs
- [X] T056 [US3] Implement initialize_counter instruction handler
- [X] T057 [US3] Implement StartSession context in programs/session-manager/src/lib.rs
- [X] T058 [US3] Implement start_session instruction handler with session_id increment
- [X] T059 [US3] Create delegation.rs module for MagicBlock integration in programs/session-manager/src/delegation.rs
- [X] T060 [US3] Implement DelegateSession context with MagicBlock accounts in programs/session-manager/src/lib.rs
- [X] T061 [US3] Implement delegate_session instruction using ephemeral_rollups_sdk::cpi::delegate_account
- [X] T062 [US3] Implement CommitSession context in programs/session-manager/src/lib.rs
- [X] T063 [US3] Implement commit_session instruction using commit_accounts
- [X] T064 [US3] Implement EndSession context in programs/session-manager/src/lib.rs
- [X] T065 [US3] Implement end_session instruction using commit_and_undelegate_accounts
- [X] T066 [US3] Implement ForceCloseSession context with timeout validation
- [X] T067 [US3] Implement force_close_session instruction (1 hour timeout check)
- [X] T068 [US3] Add SessionStarted, SessionDelegated, SessionEnded event emissions
- [X] T069 [US3] Run tests and verify all US3 tests pass

**Checkpoint**: Full session lifecycle works with MagicBlock delegation

---

## Phase 6: User Story 4 - Map Generation (Priority: P4)

**Goal**: Deterministic map generation from configurable seeds

**Independent Test**: Get seed for level, generate map off-chain, verify hash matches

### Tests for User Story 4

- [X] T070 [P] [US4] Write test "initializes map config with default seeds" in tests/map-generator.ts
- [X] T071 [P] [US4] Write test "returns correct seed for level" in tests/map-generator.ts
- [X] T072 [P] [US4] Write test "admin can update seed mapping" in tests/map-generator.ts
- [X] T073 [P] [US4] Write test "batch updates multiple seeds" in tests/map-generator.ts
- [X] T074 [P] [US4] Write test "rejects invalid level number" in tests/map-generator.ts
- [X] T075 [P] [US4] Write test "verifies map hash" in tests/map-generator.ts
- [X] T076 [P] [US4] Write test "RNG produces deterministic results" in tests/map-generator.ts

### Implementation for User Story 4

- [X] T077 [US4] Define MapConfig account struct in programs/map-generator/src/state.rs
- [X] T078 [US4] Define SeedUpdate, TileType, Position types in programs/map-generator/src/state.rs
- [X] T079 [US4] Implement InitializeMapConfig context in programs/map-generator/src/lib.rs
- [X] T080 [US4] Implement initialize_map_config with default seeds (0-80)
- [X] T081 [US4] Implement UpdateMapConfig context (admin only) in programs/map-generator/src/lib.rs
- [X] T082 [US4] Implement update_map_config instruction handler
- [X] T083 [US4] Implement BatchUpdateMapConfig context in programs/map-generator/src/lib.rs
- [X] T084 [US4] Implement batch_update_map_config instruction handler
- [X] T085 [US4] Implement GetMapSeed view function in programs/map-generator/src/lib.rs
- [X] T086 [US4] Implement VerifyMapHash instruction in programs/map-generator/src/lib.rs
- [X] T087 [US4] Create generator.rs with off-chain map generation logic in programs/map-generator/src/generator.rs
- [X] T088 [US4] Implement corridor maze generation algorithm in generator.rs
- [X] T089 [US4] Implement POI placement logic in generator.rs
- [X] T090 [US4] Implement enemy spawning with zone-based tier selection in generator.rs
- [X] T091 [US4] Add MapConfigUpdated, AdminTransferred events
- [X] T092 [US4] Implement TransferAdmin instruction for admin authority transfer
- [X] T093 [US4] Run tests and verify all US4 tests pass

**Checkpoint**: Map config works, seeds configurable, generation is deterministic

---

## Phase 7: User Story 5 - Run Completion Tracking (Priority: P5)

**Goal**: Track run completions and campaign progression

**Independent Test**: Complete run, verify total_runs increments, verify level advances on victory

### Tests for User Story 5

- [X] T094 [P] [US5] Write test "records run completion" in tests/player-profile.ts
- [X] T095 [P] [US5] Write test "increments total_runs on completion" in tests/player-profile.ts
- [X] T096 [P] [US5] Write test "advances level on victory" in tests/player-profile.ts
- [X] T097 [P] [US5] Write test "does not advance level on defeat" in tests/player-profile.ts
- [X] T098 [P] [US5] Write test "respects tier boundary on level advance" in tests/player-profile.ts

### Implementation for User Story 5

- [X] T099 [US5] Implement RecordRunResult context in programs/player-profile/src/lib.rs
- [X] T100 [US5] Implement record_run_result instruction handler
- [X] T101 [US5] Add victory check for level advancement logic
- [X] T102 [US5] Add tier boundary validation (cannot exceed unlocked_tier * 40 - 1)
- [X] T103 [US5] Add RunCompleted event emission
- [X] T104 [US5] Run tests and verify all US5 tests pass

**Checkpoint**: Run tracking complete, progression respects tier boundaries

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final quality checks and documentation

- [X] T105 [P] Run `cargo clippy` on all programs and fix any warnings
- [X] T106 [P] Run `cargo audit` and address any security advisories
- [X] T107 [P] Generate IDL files with `anchor build` and verify they match contracts/
- [X] T108 [P] Update CLAUDE.md with final tech stack and commands
- [X] T109 Run full test suite with `anchor test` and verify 80%+ coverage
- [X] T110 Validate quickstart.md steps work end-to-end
- [X] T111 Create deployment script in migrations/deploy.ts
- [X] T112 Document PDA derivation in README.md for client integration

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
  - US5 (P5): Depends on US1 (modifies PlayerProfile)
- **Polish (Phase 8)**: Depends on all user stories complete

### User Story Dependencies

```
US1 (Player Registration) ──┬──> US2 (Tier Unlocking)
                           ├──> US3 (Session Delegation)
                           └──> US5 (Run Completion)

US4 (Map Generation) ──────────> Independent
```

### Parallel Opportunities

- **Phase 1**: T003-T010 can run in parallel
- **Phase 2**: T011-T015 can run in parallel, then T016-T019 sequential
- **Phase 3-7**: All test tasks within a phase can run in parallel
- **Phase 3-7**: Once tests pass, implementation tasks follow dependency order
- **US3 and US4**: Can be developed in parallel (different programs)

---

## Parallel Example: Phase 2 Foundational

```bash
# Launch error definitions in parallel:
Task: "Define PlayerProfileError enum in programs/player-profile/src/errors.rs"
Task: "Define SessionManagerError enum in programs/session-manager/src/errors.rs"
Task: "Define MapGeneratorError enum in programs/map-generator/src/errors.rs"

# Launch constants in parallel:
Task: "Define constants in programs/player-profile/src/constants.rs"
Task: "Define constants in programs/map-generator/src/constants.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (Player Registration)
4. **STOP and VALIDATE**: Deploy to devnet, test profile creation
5. Demo MVP capability

### Incremental Delivery

1. Setup + Foundational → Build passes
2. User Story 1 → Profile creation works → **MVP!**
3. User Story 2 → Payment flow works → Monetization ready
4. User Story 3 → Session delegation works → Game sessions enabled
5. User Story 4 → Map generation works → Core gameplay ready
6. User Story 5 → Progression tracking works → Full feature set

### Suggested MVP Scope

**Minimum**: Phase 1 + Phase 2 + Phase 3 (User Story 1)
- Players can create profiles and set names
- Foundation for all other features
- ~30 tasks, testable end-to-end

---

## Task Summary

| Phase | User Story | Task Count | Parallel Tasks |
|-------|------------|------------|----------------|
| 1 | Setup | 10 | 6 |
| 2 | Foundational | 9 | 5 |
| 3 | US1 - Registration | 11 | 4 |
| 4 | US2 - Payments | 15 | 5 |
| 5 | US3 - Sessions | 24 | 7 |
| 6 | US4 - Map Gen | 24 | 7 |
| 7 | US5 - Run Tracking | 11 | 5 |
| 8 | Polish | 8 | 4 |
| **Total** | | **112** | **43** |

---

## Notes

- [P] tasks = different files, no dependencies within phase
- [Story] label maps task to specific user story
- Tests use TDD approach per Constitution (write tests first, verify they fail)
- Constitution requires 80% test coverage minimum
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
