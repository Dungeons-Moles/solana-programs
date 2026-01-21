# Tasks: POI System

**Input**: Design documents from `/specs/007-poi-system/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/

**Tests**: Constitution mandates TDD. Tests will be written before implementation per Principle II.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Programs**: `programs/poi-system/src/`
- **Tests**: `tests/poi-system.ts` (integration), in-program unit tests
- **Dependencies**: `programs/gameplay-state/` (needs gold field)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic program structure

- [x] T001 Create poi-system program directory structure at programs/poi-system/
- [x] T002 Initialize Anchor program with Cargo.toml in programs/poi-system/Cargo.toml
- [x] T003 [P] Add program to Anchor.toml with new program ID
- [x] T004 [P] Create errors.rs with PoiSystemError enum in programs/poi-system/src/errors.rs
- [x] T005 Create lib.rs program entry with declare_id! in programs/poi-system/src/lib.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 Add gold: u16 field to GameState in programs/gameplay-state/src/state.rs
- [x] T007 Add modify_gold instruction to gameplay-state in programs/gameplay-state/src/lib.rs
- [x] T008 [P] Create state.rs with MapPois, PoiInstance, ShopState structs in programs/poi-system/src/state.rs
- [x] T009 [P] Create enums (PoiRarity, UseType, ActiveCondition, InteractionType) in programs/poi-system/src/state.rs
- [x] T010 Add dependency declarations in programs/poi-system/Cargo.toml for player-inventory, boss-system, gameplay-state

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - POI Definition Registry (Priority: P1) 🎯 MVP

**Goal**: Complete registry of all 14 POI types with properties queryable by ID

**Independent Test**: Query any POI type by ID and verify properties match GDD specifications

### Tests for User Story 1

- [x] T011 [P] [US1] Unit test for all 14 POI definitions in programs/poi-system/src/pois.rs (test module)
- [x] T012 [P] [US1] Unit test for get_poi_definition returns correct properties in programs/poi-system/src/pois.rs
- [x] T013 [P] [US1] Unit test for invalid POI ID returns error in programs/poi-system/src/pois.rs

### Implementation for User Story 1

- [x] T014 [P] [US1] Create PoiDefinition struct in programs/poi-system/src/pois.rs
- [x] T015 [US1] Define all 14 POI constants (L1-L14) with properties in programs/poi-system/src/pois.rs
- [x] T016 [US1] Implement get_poi_definition lookup function in programs/poi-system/src/pois.rs
- [x] T017 [US1] Implement get_pois_by_rarity filter function in programs/poi-system/src/pois.rs
- [ ] T018 [US1] Add get_poi_definition instruction to lib.rs in programs/poi-system/src/lib.rs

**Checkpoint**: User Story 1 complete - POI definitions queryable

---

## Phase 4: User Story 2 - Item Offer Generation (Priority: P1)

**Goal**: POIs generate weighted item selections based on act, rarity tables, and boss weakness tags

**Independent Test**: Generate offers for each POI type and verify rarity distribution and tag weighting

### Tests for User Story 2

- [x] T019 [P] [US2] Unit test for Supply Cache rarity table (100% Common Act 1) in programs/poi-system/src/offers.rs
- [x] T020 [P] [US2] Unit test for Tool Crate rarity table (includes Heroic Act 2+) in programs/poi-system/src/offers.rs
- [x] T021 [P] [US2] Unit test for Geode Vault (90/10 Heroic/Mythic Act 4) in programs/poi-system/src/offers.rs
- [ ] T022 [P] [US2] Unit test for Counter Cache weakness-only filtering in programs/poi-system/src/offers.rs
- [x] T023 [P] [US2] Unit test for tag weighting (1.4x for weaknesses) in programs/poi-system/src/offers.rs
- [x] T024 [P] [US2] Unit test for Smuggler Hatch (1 Tool + 5 Gear) in programs/poi-system/src/offers.rs

### Implementation for User Story 2

- [x] T025 [P] [US2] Define rarity tables (SUPPLY_CACHE, TOOL_CRATE, GEODE, SMUGGLER) in programs/poi-system/src/offers.rs
- [ ] T026 [US2] Implement generate_poi_offers function in programs/poi-system/src/offers.rs
- [ ] T027 [US2] Implement generate_supply_cache_offers (3 Gear) in programs/poi-system/src/offers.rs
- [ ] T028 [US2] Implement generate_tool_crate_offers (3 Tools) in programs/poi-system/src/offers.rs
- [ ] T029 [US2] Implement generate_geode_vault_offers (Heroic+) in programs/poi-system/src/offers.rs
- [ ] T030 [US2] Implement generate_counter_cache_offers (weakness tags only) in programs/poi-system/src/offers.rs
- [ ] T031 [US2] Implement generate_smuggler_hatch_offers (1 Tool + 5 Gear) in programs/poi-system/src/offers.rs

**Checkpoint**: User Story 2 complete - Item offers generate correctly

---

## Phase 5: User Story 3 - POI Interaction Execution (Priority: P1)

**Goal**: Players can interact with POIs to perform actions with correct state changes

**Independent Test**: Execute each POI interaction type and verify state changes

### Tests for User Story 3

- [x] T032 [P] [US3] Unit test for Mole Den (full heal, night-only) in programs/poi-system/src/interactions.rs
- [x] T033 [P] [US3] Unit test for Rest Alcove (10 HP heal, one-time) in programs/poi-system/src/interactions.rs
- [x] T034 [P] [US3] Unit test for Tool Oil Rack modifications in programs/poi-system/src/interactions.rs
- [x] T035 [P] [US3] Unit test for night-only POI rejection during day in programs/poi-system/src/interactions.rs
- [x] T036 [P] [US3] Unit test for one-time POI already used error in programs/poi-system/src/interactions.rs

### Implementation for User Story 3

- [x] T037 [US3] Create interactions.rs module scaffold in programs/poi-system/src/interactions.rs
- [ ] T038 [US3] Implement interact_rest for L1 and L5 in programs/poi-system/src/interactions.rs
- [ ] T039 [US3] Implement interact_pick_item for L2, L3, L12, L13 in programs/poi-system/src/interactions.rs
- [ ] T040 [US3] Implement interact_tool_oil for L4 in programs/poi-system/src/interactions.rs
- [ ] T041 [US3] Add interact_rest instruction with accounts in programs/poi-system/src/lib.rs
- [ ] T042 [US3] Add interact_pick_item instruction with accounts in programs/poi-system/src/lib.rs
- [ ] T043 [US3] Add interact_tool_oil instruction with accounts in programs/poi-system/src/lib.rs
- [x] T044 [US3] Implement initialize_map_pois instruction in programs/poi-system/src/lib.rs
- [x] T045 [US3] Add event emissions for POI interactions in programs/poi-system/src/lib.rs

**Checkpoint**: User Story 3 complete - Core POI interactions work

---

## Phase 6: User Story 4 - Shop and Pricing (Priority: P2)

**Goal**: Smuggler Hatch shop with gold-based purchases and reroll mechanic

**Independent Test**: Enter shop, purchase items, reroll with correct gold costs

### Tests for User Story 4

- [ ] T046 [P] [US4] Unit test for shop enter generates 6 offers in programs/poi-system/src/interactions.rs
- [x] T047 [P] [US4] Unit test for purchase deducts correct gold (8/14/22/34 Gear, 10/16/24 Tool) in programs/poi-system/src/interactions.rs
- [x] T048 [P] [US4] Unit test for reroll cost progression (4, 6, 8, 10...) in programs/poi-system/src/interactions.rs
- [ ] T049 [P] [US4] Unit test for insufficient gold error in programs/poi-system/src/interactions.rs

### Implementation for User Story 4

- [ ] T050 [US4] Implement enter_shop instruction in programs/poi-system/src/lib.rs
- [ ] T051 [US4] Implement shop_purchase instruction in programs/poi-system/src/lib.rs
- [ ] T052 [US4] Implement shop_reroll instruction in programs/poi-system/src/lib.rs
- [ ] T053 [US4] Implement leave_shop instruction in programs/poi-system/src/lib.rs
- [x] T054 [US4] Add ShopState management in MapPois in programs/poi-system/src/state.rs

**Checkpoint**: User Story 4 complete - Shop mechanics work

---

## Phase 7: User Story 5 - Item Upgrade POIs (Priority: P2)

**Goal**: Rusty Anvil and Rune Kiln upgrade item tiers

**Independent Test**: Upgrade items at each POI with correct costs and results

### Tests for User Story 5

- [x] T055 [P] [US5] Unit test for Rusty Anvil I→II costs 8 Gold in programs/poi-system/src/interactions.rs
- [x] T056 [P] [US5] Unit test for Rusty Anvil II→III costs 16 Gold in programs/poi-system/src/interactions.rs
- [ ] T057 [P] [US5] Unit test for Rune Kiln fusion (identical items, free) in programs/poi-system/src/interactions.rs
- [ ] T058 [P] [US5] Unit test for Rune Kiln non-matching items error in programs/poi-system/src/interactions.rs

### Implementation for User Story 5

- [ ] T059 [US5] Implement interact_rusty_anvil instruction in programs/poi-system/src/lib.rs
- [ ] T060 [US5] Implement interact_rune_kiln instruction in programs/poi-system/src/lib.rs
- [x] T061 [US5] Add upgrade cost constants (ANVIL_COSTS: [8, 16]) in programs/poi-system/src/interactions.rs

**Checkpoint**: User Story 5 complete - Upgrade mechanics work

---

## Phase 8: User Story 6 - Fast Travel (Priority: P2)

**Goal**: Rail Waypoints enable fast travel between discovered waypoints

**Independent Test**: Discover waypoints and travel between them

### Tests for User Story 6

- [x] T062 [P] [US6] Unit test for waypoint discovery on first visit in programs/poi-system/src/interactions.rs
- [x] T063 [P] [US6] Unit test for fast travel updates player position in programs/poi-system/src/interactions.rs
- [x] T064 [P] [US6] Unit test for undiscovered destination error in programs/poi-system/src/interactions.rs
- [x] T065 [P] [US6] Unit test for no destinations available error in programs/poi-system/src/interactions.rs

### Implementation for User Story 6

- [ ] T066 [US6] Implement interact_rail_waypoint instruction in programs/poi-system/src/lib.rs
- [x] T067 [US6] Add discovered field tracking in PoiInstance in programs/poi-system/src/state.rs
- [x] T068 [US6] Implement get_discovered_waypoints helper in programs/poi-system/src/interactions.rs

**Checkpoint**: User Story 6 complete - Fast travel works

---

## Phase 9: User Story 7 - Map Reveal POIs (Priority: P3)

**Goal**: Survey Beacon and Seismic Scanner reveal map information

**Independent Test**: Use reveal POIs and verify correct tiles/POIs are revealed

### Tests for User Story 7

- [x] T069 [P] [US7] Unit test for Survey Beacon reveals radius 13 tiles in programs/poi-system/src/interactions.rs
- [x] T070 [P] [US7] Unit test for Seismic Scanner finds nearest POI of category in programs/poi-system/src/interactions.rs
- [ ] T071 [P] [US7] Unit test for one-time reveal POI already used error in programs/poi-system/src/interactions.rs

### Implementation for User Story 7

- [ ] T072 [US7] Implement interact_survey_beacon instruction in programs/poi-system/src/lib.rs
- [ ] T073 [US7] Implement interact_seismic_scanner instruction in programs/poi-system/src/lib.rs
- [x] T074 [US7] Implement calculate_revealed_tiles helper (radius 13) in programs/poi-system/src/interactions.rs
- [x] T075 [US7] Implement find_nearest_poi_by_category helper in programs/poi-system/src/interactions.rs
- [x] T076 [US7] Define PoiCategory enum for scanner in programs/poi-system/src/state.rs

**Checkpoint**: User Story 7 complete - Map reveal works

---

## Phase 10: User Story 8 - POI Spawn Configuration (Priority: P3)

**Goal**: Configurable POI spawn counts and guarantees per act

**Independent Test**: Retrieve spawn configuration for any act and verify counts match GDD

### Tests for User Story 8

- [ ] T077 [P] [US8] Unit test for Act 1 baseline spawns (L2 x10, L3 x2, etc.) in programs/poi-system/src/spawn.rs
- [ ] T078 [P] [US8] Unit test for Act 1 guarantees (L1 adjacent, L8 x2, etc.) in programs/poi-system/src/spawn.rs
- [ ] T079 [P] [US8] Unit test for Counter Cache week-based chances in programs/poi-system/src/spawn.rs
- [ ] T080 [P] [US8] Unit test for all 4 acts return valid configs in programs/poi-system/src/spawn.rs

### Implementation for User Story 8

- [ ] T081 [P] [US8] Create spawn.rs module scaffold in programs/poi-system/src/spawn.rs
- [ ] T082 [US8] Define ActSpawnConfig and SpawnGuarantee structs in programs/poi-system/src/spawn.rs
- [ ] T083 [US8] Define SPAWN_CONFIGS constant array for acts 1-4 in programs/poi-system/src/spawn.rs
- [ ] T084 [US8] Implement get_spawn_config function in programs/poi-system/src/spawn.rs
- [ ] T085 [US8] Add get_spawn_config instruction to lib.rs in programs/poi-system/src/lib.rs

**Checkpoint**: User Story 8 complete - Spawn configuration queryable

---

## Phase 11: Polish & Cross-Cutting Concerns

**Purpose**: Integration, cleanup, and final validation

- [ ] T086 [P] Implement interact_scrap_chute instruction (L14) in programs/poi-system/src/lib.rs
- [ ] T087 [P] Implement close_map_pois instruction in programs/poi-system/src/lib.rs
- [ ] T088 [P] Implement get_poi_at_position query in programs/poi-system/src/lib.rs
- [ ] T089 Create integration test file at tests/poi-system.ts
- [ ] T090 Integration test: initialize POIs, interact, verify state in tests/poi-system.ts
- [ ] T091 Integration test: shop flow (enter, purchase, reroll, leave) in tests/poi-system.ts
- [ ] T092 Integration test: upgrade flow (anvil, kiln) in tests/poi-system.ts
- [ ] T093 Run anchor build and fix any compilation errors
- [ ] T094 Run anchor test and verify all tests pass
- [ ] T095 Run cargo clippy and fix warnings
- [ ] T096 Validate quickstart.md examples work as documented

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-10)**: All depend on Foundational phase completion
  - US1 (Registry) can start first
  - US2 (Offers) depends on US1 for POI types
  - US3 (Interactions) depends on US1 and US2
  - US4-8 can proceed after US3 core is in place
- **Polish (Phase 11)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundation only - defines POI types
- **User Story 2 (P1)**: Needs US1 POI types for offer generation context
- **User Story 3 (P1)**: Needs US1 + US2 for interactions with item offers
- **User Story 4 (P2)**: Needs US3 interaction framework
- **User Story 5 (P2)**: Needs US3 interaction framework
- **User Story 6 (P2)**: Needs US3 interaction framework + discovered field
- **User Story 7 (P3)**: Needs US3 interaction framework
- **User Story 8 (P3)**: Independent (static data) - can start after Foundation

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- State/types before logic
- Core implementation before instructions
- Instructions before events

### Parallel Opportunities

**Setup (can run in parallel)**:

- T003, T004 (different files)

**Foundation (can run in parallel)**:

- T008, T009 (same file but different sections)

**US1 Tests (all parallel)**:

- T011, T012, T013

**US2 Tests (all parallel)**:

- T019, T020, T021, T022, T023, T024

**US3 Tests (all parallel)**:

- T032, T033, T034, T035, T036

**Different user stories can be worked on in parallel after their dependencies are met**

---

## Parallel Example: User Story 2

```bash
# Launch all tests for User Story 2 together:
Task: "Unit test for Supply Cache rarity table in programs/poi-system/src/offers.rs"
Task: "Unit test for Tool Crate rarity table in programs/poi-system/src/offers.rs"
Task: "Unit test for Geode Vault in programs/poi-system/src/offers.rs"
Task: "Unit test for Counter Cache weakness-only filtering in programs/poi-system/src/offers.rs"
Task: "Unit test for tag weighting in programs/poi-system/src/offers.rs"
Task: "Unit test for Smuggler Hatch in programs/poi-system/src/offers.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1-3 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (POI Registry)
4. Complete Phase 4: User Story 2 (Item Offers)
5. Complete Phase 5: User Story 3 (Core Interactions)
6. **STOP and VALIDATE**: Test POI definitions, offers, and basic interactions
7. Deploy/demo if ready - players can explore and pick items

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add US1-3 → Test → Deploy (MVP with item acquisition!)
3. Add US4 (Shop) → Test → Deploy (economic depth)
4. Add US5 (Upgrades) → Test → Deploy (build progression)
5. Add US6 (Fast Travel) → Test → Deploy (convenience)
6. Add US7-8 (Map Reveal, Spawn) → Test → Deploy (exploration)

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: US1 → US2 → US3 (critical path)
   - Developer B: US8 (spawn config, independent)
3. After US3:
   - Developer A: US4 (Shop)
   - Developer B: US5 (Upgrades)
   - Developer C: US6 (Fast Travel)
4. Finally:
   - US7 (Map Reveal)
   - Integration tests and polish

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- TDD enforced: Write tests first, verify they fail, then implement
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Avoid: vague tasks, same file conflicts, cross-story dependencies that break independence
