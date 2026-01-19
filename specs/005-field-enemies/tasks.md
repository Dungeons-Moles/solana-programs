# Tasks: Field Enemies

**Input**: Design documents from `/specs/005-field-enemies/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: REQUIRED per Constitution (II. Test-Driven Development)

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US5)
- Include exact file paths in descriptions

## Path Conventions

```text
programs/field-enemies/src/   # New program crate
tests/field-enemies/          # Test files
```

---

## Phase 1: Setup

**Purpose**: Create field-enemies program crate and configure workspace

- [x] T001 Create program crate with `anchor init field-enemies` in programs/field-enemies/
- [x] T002 Add field-enemies to workspace in Cargo.toml members array
- [x] T003 [P] Add combat-system dependency in programs/field-enemies/Cargo.toml
- [x] T004 [P] Add map-generator dependency in programs/field-enemies/Cargo.toml
- [x] T005 Generate program keypair and add to Anchor.toml

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and infrastructure that MUST be complete before ANY user story

**CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 Create error types in programs/field-enemies/src/errors.rs (FieldEnemiesError enum)
- [x] T007 [P] Create EnemyTier enum in programs/field-enemies/src/state.rs
- [x] T008 [P] Create EnemyStats struct in programs/field-enemies/src/state.rs
- [x] T009 [P] Create EnemyInstance struct (5 bytes: archetype_id, tier, x, y, defeated) in programs/field-enemies/src/state.rs
- [x] T010 Create MapEnemies account struct with PDA seeds in programs/field-enemies/src/state.rs
- [x] T011 Create constants module with SPAWN_COUNTS and TIER_DISTRIBUTIONS in programs/field-enemies/src/constants.rs
- [x] T012 Create module declarations in programs/field-enemies/src/lib.rs
- [x] T013 Run `anchor build` to verify compilation

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Player Encounters Field Enemies (Priority: P1)

**Goal**: Enable player to encounter enemies on map tiles, initiate combat, receive Gold on victory

**Independent Test**: Spawn single Tunnel Rat (T1), step onto tile, verify combat resolves with +2 Gold

### Tests for User Story 1

> **TDD**: Write tests FIRST, ensure they FAIL before implementation

- [x] T014 [P] [US1] Unit test for get_enemy_combatant_input() returning correct CombatantInput in programs/field-enemies/src/archetypes.rs
- [x] T015 [P] [US1] Unit test for mark_enemy_defeated() setting defeated=true in programs/field-enemies/src/lib.rs
- [x] T016 [P] [US1] Unit test for Gold reward calculation (tier+1)\*2 in programs/field-enemies/src/state.rs

### Implementation for User Story 1

- [x] T017 [P] [US1] Implement EnemyArchetype struct with id, name, biome weights in programs/field-enemies/src/archetypes.rs
- [x] T018 [P] [US1] Define ENEMY_ARCHETYPES static array (12 archetypes) in programs/field-enemies/src/archetypes.rs
- [x] T019 [US1] Implement get_enemy_combatant_input(archetype_id, tier) helper in programs/field-enemies/src/archetypes.rs
- [x] T020 [US1] Implement get_enemy_at_position(map_enemies, x, y) helper in programs/field-enemies/src/state.rs
- [x] T021 [US1] Implement EnemyTier::gold_reward() method returning 2/4/6 in programs/field-enemies/src/state.rs
- [x] T022 [US1] Create MarkEnemyDefeated accounts struct in programs/field-enemies/src/lib.rs
- [x] T023 [US1] Implement mark_enemy_defeated instruction handler in programs/field-enemies/src/lib.rs
- [x] T024 [US1] Add EnemyDefeated event emission in mark_enemy_defeated handler
- [x] T025 [US1] Verify tests pass with `cargo test -p field-enemies`

**Checkpoint**: Player can encounter enemy, defeat it, receive Gold

---

## Phase 4: User Story 2 - Enemy Tier Distribution Per Act (Priority: P1)

**Goal**: Spawn enemies with correct tier distribution (70/25/5 → 35/45/20) per act

**Independent Test**: Generate 100 maps per act, verify tier ratios within 5% of target

### Tests for User Story 2

- [x] T026 [P] [US2] Unit test for sample_tier() matching Act 1 distribution (70/25/5) in programs/field-enemies/src/spawner.rs
- [x] T027 [P] [US2] Unit test for sample_tier() matching Act 4 distribution (35/45/20) in programs/field-enemies/src/spawner.rs
- [x] T028 [P] [US2] Unit test for spawn_count matching SPAWN_COUNTS[act] in programs/field-enemies/src/spawner.rs

### Implementation for User Story 2

- [x] T029 [P] [US2] Create TierDistribution struct in programs/field-enemies/src/constants.rs
- [x] T030 [US2] Implement sample_tier(rng, act) using cumulative thresholds in programs/field-enemies/src/spawner.rs
- [x] T031 [US2] Implement get_spawn_count(act) returning 36/40/44/48 in programs/field-enemies/src/spawner.rs
- [x] T032 [US2] Create InitializeMapEnemies accounts struct in programs/field-enemies/src/lib.rs
- [x] T033 [US2] Implement initialize_map_enemies instruction (spawn loop with tier sampling) in programs/field-enemies/src/lib.rs
- [x] T034 [US2] Add EnemiesSpawned event emission in initialize_map_enemies handler
- [x] T035 [US2] Verify tests pass with `cargo test -p field-enemies`

**Checkpoint**: Maps spawn correct enemy counts with proper tier distribution

---

## Phase 5: User Story 3 - Enemy Trait Execution During Combat (Priority: P1)

**Goal**: Each enemy archetype has unique trait that triggers correctly in combat

**Independent Test**: Fight each enemy type, verify trait fires at correct trigger time with correct effect

### Tests for User Story 3

- [x] T036 [P] [US3] Unit test for Shard Beetle trait (BattleStart: gain 6 Shrapnel) in programs/field-enemies/src/traits.rs
- [x] T037 [P] [US3] Unit test for Blood Mosquito trait (OnHit: apply 1 Bleed) in programs/field-enemies/src/traits.rs
- [x] T038 [P] [US3] Unit test for Spore Slime trait (BattleStart: apply 2 Chill) in programs/field-enemies/src/traits.rs
- [x] T039 [P] [US3] Unit test for get_enemy_traits() returning correct ItemEffect array in programs/field-enemies/src/traits.rs

### Implementation for User Story 3

- [x] T040 [P] [US3] Define TUNNEL_RAT_TRAITS static (OnHit: steal 1 Gold) in programs/field-enemies/src/traits.rs
- [x] T041 [P] [US3] Define CAVE_BAT_TRAITS static (EveryOtherTurn: restore 1 HP) in programs/field-enemies/src/traits.rs
- [x] T042 [P] [US3] Define SPORE_SLIME_TRAITS static (BattleStart: apply 2 Chill) in programs/field-enemies/src/traits.rs
- [x] T043 [P] [US3] Define RUST_MITE_TRAITS static (OnHit: apply 1 Rust) in programs/field-enemies/src/traits.rs
- [x] T044 [P] [US3] Define COLLAPSED_MINER_TRAITS static (Wounded: gain +3 ATK) in programs/field-enemies/src/traits.rs
- [x] T045 [P] [US3] Define SHARD_BEETLE_TRAITS static (BattleStart: gain 6 Shrapnel) in programs/field-enemies/src/traits.rs
- [x] T046 [P] [US3] Define TUNNEL_WARDEN_TRAITS static (FirstStrike: remove 3 Armor) in programs/field-enemies/src/traits.rs
- [x] T047 [P] [US3] Define BURROW_AMBUSHER_TRAITS static (BattleStart: deal 3 non-weapon damage) in programs/field-enemies/src/traits.rs
- [x] T048 [P] [US3] Define FROST_WISP_TRAITS static (FirstTurn if acts first: apply 2 Chill) in programs/field-enemies/src/traits.rs
- [x] T049 [P] [US3] Define POWDER_TICK_TRAITS static (Countdown(2): deal 6 non-weapon damage) in programs/field-enemies/src/traits.rs
- [x] T050 [P] [US3] Define COIN_SLUG_TRAITS static (BattleStart: gain Armor = Gold/10 cap 3) in programs/field-enemies/src/traits.rs
- [x] T051 [P] [US3] Define BLOOD_MOSQUITO_TRAITS static (OnHit: apply 1 Bleed) in programs/field-enemies/src/traits.rs
- [x] T052 [US3] Implement get_enemy_traits(archetype_id) returning trait array in programs/field-enemies/src/traits.rs
- [x] T053 [US3] Verify tests pass with `cargo test -p field-enemies`

**Checkpoint**: All 12 enemy traits defined and return correct effects

---

## Phase 6: User Story 4 - Biome-Weighted Enemy Selection (Priority: P2)

**Goal**: Biome A/B emphasize different enemy archetypes via weighted sampling

**Independent Test**: Generate 100 Biome A maps, verify emphasized enemies appear >20% more often

### Tests for User Story 4

- [x] T054 [P] [US4] Unit test for sample_archetype() with Biome A weights in programs/field-enemies/src/spawner.rs
- [x] T055 [P] [US4] Unit test for sample_archetype() with Biome B weights in programs/field-enemies/src/spawner.rs
- [x] T056 [P] [US4] Unit test verifying all 12 archetypes can spawn (no exclusion) in programs/field-enemies/src/spawner.rs

### Implementation for User Story 4

- [x] T057 [P] [US4] Define BIOME_A_WEIGHTS array (emphasize Tunnel Rat, Collapsed Miner, Shard Beetle, Coin Slug) in programs/field-enemies/src/constants.rs
- [x] T058 [P] [US4] Define BIOME_B_WEIGHTS array (emphasize Rust Mite, Frost Wisp, Blood Mosquito, Burrow Ambusher, Powder Tick) in programs/field-enemies/src/constants.rs
- [x] T059 [US4] Implement get_biome_weights(act) returning A or B weights in programs/field-enemies/src/spawner.rs
- [x] T060 [US4] Implement sample_archetype(rng, weights) using weighted random selection in programs/field-enemies/src/spawner.rs
- [x] T061 [US4] Update initialize_map_enemies to use biome-weighted archetype selection in programs/field-enemies/src/lib.rs
- [x] T062 [US4] Verify tests pass with `cargo test -p field-enemies`

**Checkpoint**: Biome-appropriate enemies spawn more frequently

---

## Phase 7: User Story 5 - Enemy Stat Scaling by Tier (Priority: P2)

**Goal**: All 36 enemy configurations (12 archetypes × 3 tiers) have correct stats matching GDD

**Independent Test**: Iterate all archetype/tier combinations, verify stats match spec table exactly

### Tests for User Story 5

- [x] T063 [P] [US5] Unit test for Tunnel Rat T1 stats (5/1/0/3/1) in programs/field-enemies/src/archetypes.rs
- [x] T064 [P] [US5] Unit test for Tunnel Warden T3 stats (14/4/8/4/3) in programs/field-enemies/src/archetypes.rs
- [x] T065 [P] [US5] Exhaustive test for all 36 stat configurations in programs/field-enemies/src/archetypes.rs

### Implementation for User Story 5

- [x] T066 [P] [US5] Define TUNNEL_RAT_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T067 [P] [US5] Define CAVE_BAT_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T068 [P] [US5] Define SPORE_SLIME_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T069 [P] [US5] Define RUST_MITE_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T070 [P] [US5] Define COLLAPSED_MINER_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T071 [P] [US5] Define SHARD_BEETLE_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T072 [P] [US5] Define TUNNEL_WARDEN_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T073 [P] [US5] Define BURROW_AMBUSHER_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T074 [P] [US5] Define FROST_WISP_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T075 [P] [US5] Define POWDER_TICK_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T076 [P] [US5] Define COIN_SLUG_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T077 [P] [US5] Define BLOOD_MOSQUITO_STATS: [EnemyStats; 3] array in programs/field-enemies/src/archetypes.rs
- [x] T078 [US5] Create ENEMY_STATS: [[EnemyStats; 3]; 12] master array in programs/field-enemies/src/archetypes.rs
- [x] T079 [US5] Implement get_enemy_stats(archetype_id, tier) lookup in programs/field-enemies/src/archetypes.rs
- [x] T080 [US5] Verify tests pass with `cargo test -p field-enemies`

**Checkpoint**: All 36 enemy configurations verified against GDD

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Quality assurance and documentation

- [x] T081 Run `cargo clippy -p field-enemies` and fix all warnings
- [x] T082 [P] Add IDL documentation comments to all public structs in programs/field-enemies/src/
- [x] T083 [P] Add IDL documentation comments to all instructions in programs/field-enemies/src/lib.rs
- [x] T084 Run `anchor build` and verify IDL generation
- [x] T085 Create integration test: spawn enemies, defeat one, verify Gold in tests/field-enemies/integration/
- [x] T086 Run quickstart.md validation scenarios
- [x] T087 Update AGENTS.md with field-enemies program information

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - US1, US2, US3 are all P1 - implement in order
  - US4, US5 are P2 - can start after all P1 stories complete
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

| Story | Priority | Can Start After | Dependencies                    |
| ----- | -------- | --------------- | ------------------------------- |
| US1   | P1       | Phase 2         | None                            |
| US2   | P1       | US1             | Needs EnemyArchetype from US1   |
| US3   | P1       | Phase 2         | Independent (trait definitions) |
| US4   | P2       | US2             | Uses spawner from US2           |
| US5   | P2       | US1             | Uses archetypes from US1        |

### Within Each User Story

1. Tests written FIRST, must FAIL
2. Implementation tasks in dependency order
3. Verify tests PASS before marking story complete

### Parallel Opportunities

**Phase 2 (Foundational)**:

```
T007, T008, T009 can run in parallel (different structs)
```

**Phase 3 (US1)**:

```
T014, T015, T016 can run in parallel (different test files)
T017, T018 can run in parallel (archetype definition)
```

**Phase 5 (US3) - Maximum Parallelism**:

```
T040-T051 can ALL run in parallel (12 independent trait definitions)
```

**Phase 7 (US5) - Maximum Parallelism**:

```
T066-T077 can ALL run in parallel (12 independent stat arrays)
```

---

## Implementation Strategy

### MVP First (US1 + US2 + US3)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: US1 (basic enemy encounter)
4. Complete Phase 4: US2 (tier distribution)
5. Complete Phase 5: US3 (enemy traits)
6. **STOP and VALIDATE**: Core field enemies system functional
7. Deploy/demo if ready

### Full Feature

1. Complete MVP (US1-US3)
2. Add Phase 6: US4 (biome weighting)
3. Add Phase 7: US5 (full stat tables)
4. Complete Phase 8: Polish

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Constitution requires TDD - tests MUST fail before implementation
- All 12 trait definitions (T040-T051) are independent and parallelizable
- All 12 stat arrays (T066-T077) are independent and parallelizable
- Verify `cargo clippy` passes with no warnings before completing
