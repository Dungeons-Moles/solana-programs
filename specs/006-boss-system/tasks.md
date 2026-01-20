# Tasks: Boss System

**Input**: Design documents from `/specs/006-boss-system/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/boss_system.idl.json

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure:

- **Program code**: `programs/boss-system/src/`
- **Tests**: `programs/boss-system/tests/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Initialize the boss-system program within the Anchor workspace

- [x] T001 Create program directory structure at programs/boss-system/
- [x] T002 Initialize Cargo.toml with anchor-lang, solana-program dependencies in programs/boss-system/Cargo.toml
- [x] T003 Add boss-system to workspace members in Anchor.toml
- [x] T004 [P] Create error enum with InvalidStage, BossNotFound in programs/boss-system/src/errors.rs
- [x] T005 [P] Create lib.rs with declare_id! and program module stub in programs/boss-system/src/lib.rs

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and enums that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 Define Biome enum (A, B) in programs/boss-system/src/lib.rs
- [x] T007 Define Week enum (One, Two, Three) in programs/boss-system/src/lib.rs
- [x] T008 Define BossStats struct (hp, atk, arm, spd, dig) in programs/boss-system/src/lib.rs
- [x] T009 Import ItemTag enum from player-inventory or define locally in programs/boss-system/src/lib.rs
- [x] T010 Import TriggerType, EffectType from combat-system or define locally in programs/boss-system/src/lib.rs
- [x] T011 Define BossTrait struct (trigger, effect_type, value, once_per_turn) in programs/boss-system/src/traits.rs
- [x] T012 Define SpecialMechanic enum (None, Countdown, Reflection, Phase, Conditional) in programs/boss-system/src/traits.rs
- [x] T013 Define Condition enum (PlayerDigLessThan, PlayerExposed, ActsFirstTurn1, ReflectionDepleted) in programs/boss-system/src/traits.rs
- [x] T014 Define BossDefinition struct with all fields in programs/boss-system/src/bosses.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Boss Definition Registry (Priority: P1) 🎯 MVP

**Goal**: Complete registry of all 24 boss definitions with stats, traits, and weakness tags

**Independent Test**: Query any boss by ID and verify stats match GDD specifications

### Implementation for User Story 1

- [x] T015 [P] [US1] Define Week 1 Biome A boss traits (Broodmother, Obsidian Golem, Gas Anomaly, Mad Miner, Shard Colossus) in programs/boss-system/src/traits.rs
- [x] T016 [P] [US1] Define Week 2 Biome A boss traits (Drill Sergeant, Crystal Mimic, Rust Regent, Powder Keg Baron, Greedkeeper) in programs/boss-system/src/traits.rs
- [x] T017 [P] [US1] Define Week 3 Biome A final boss traits (Eldritch Mole, Gilded Devourer) in programs/boss-system/src/traits.rs
- [x] T018 [P] [US1] Define Week 3 Biome B final boss traits (Frostbound Leviathan, Rusted Chronomancer) in programs/boss-system/src/traits.rs
- [x] T019 [US1] Create BOSSES const array with all 5 Week 1 Biome A boss definitions in programs/boss-system/src/bosses.rs
- [x] T020 [US1] Add all 5 Week 2 Biome A boss definitions to BOSSES array in programs/boss-system/src/bosses.rs
- [x] T021 [US1] Add 2 Week 3 Biome A final boss definitions to BOSSES array in programs/boss-system/src/bosses.rs
- [x] T022 [US1] Add all 5 Week 1 Biome B variant boss definitions (+1 SPD) to BOSSES array in programs/boss-system/src/bosses.rs
- [x] T023 [US1] Add all 5 Week 2 Biome B variant boss definitions (+1 SPD) to BOSSES array in programs/boss-system/src/bosses.rs
- [x] T024 [US1] Add 2 Week 3 Biome B final boss definitions to BOSSES array in programs/boss-system/src/bosses.rs
- [x] T025 [US1] Implement get_boss(id: &[u8; 12]) -> Option<&BossDefinition> function in programs/boss-system/src/bosses.rs
- [x] T026 [US1] Implement get_bosses_by_week(biome: Biome, week: Week) -> Vec<&BossDefinition> function in programs/boss-system/src/bosses.rs
- [x] T027 [US1] Implement getBossForStage instruction (view function) in programs/boss-system/src/lib.rs
- [x] T028 [US1] Add unit tests for all 24 boss lookups verifying stats match GDD in programs/boss-system/tests/boss_lookup.rs

**Checkpoint**: User Story 1 complete - all 24 bosses queryable by ID

---

## Phase 4: User Story 2 - Boss Selection by Stage (Priority: P1)

**Goal**: Deterministically select which boss appears at each stage (1-80) based on act, week, and stage number

**Independent Test**: Given any stage number, determine exactly which boss appears for each weekly fight

### Implementation for User Story 2

- [x] T029 [P] [US2] Implement calculate_act(stage: u8) -> u8 function (0-3) in programs/boss-system/src/selection.rs
- [x] T030 [P] [US2] Implement calculate_stage_in_act(stage: u8) -> u8 function (1-20) in programs/boss-system/src/selection.rs
- [x] T031 [P] [US2] Implement get_biome_for_act(act: u8) -> Biome function in programs/boss-system/src/selection.rs
- [x] T032 [US2] Implement select_week1_boss_index(stage_in_act: u8) -> u8 function (cycles 0-4) in programs/boss-system/src/selection.rs
- [x] T033 [US2] Implement select_week2_boss_index(stage_in_act: u8) -> u8 function (offset by 2) in programs/boss-system/src/selection.rs
- [x] T034 [US2] Implement select_week3_boss_index(stage_in_act: u8) -> u8 function (odd/even) in programs/boss-system/src/selection.rs
- [x] T035 [US2] Implement select_boss(stage: u8, week: Week) -> &BossDefinition main function in programs/boss-system/src/selection.rs
- [x] T036 [US2] Add unit tests for stage 1-20 (Act 1 Biome A) boss selection in programs/boss-system/tests/boss_selection.rs
- [x] T037 [US2] Add unit tests for stage 21-40 (Act 2 Biome B) boss selection in programs/boss-system/tests/boss_selection.rs
- [x] T038 [US2] Add unit tests for stage 41-60 (Act 3 Biome A+) boss selection in programs/boss-system/tests/boss_selection.rs
- [x] T039 [US2] Add unit tests for stage 61-80 (Act 4 Biome B+) boss selection in programs/boss-system/tests/boss_selection.rs
- [x] T040 [US2] Add unit tests for Week 3 final alternation (odd=Final1, even=Final2) in programs/boss-system/tests/boss_selection.rs

**Checkpoint**: User Story 2 complete - any stage maps to correct boss deterministically

---

## Phase 5: User Story 3 - Boss Stat Scaling (Priority: P2)

**Goal**: Scale boss stats based on act and stage progression with tier and act baseline modifiers

**Independent Test**: Verify stat calculations for any boss at any stage match GDD formulas

### Implementation for User Story 3

- [x] T041 [P] [US3] Implement calculate_tier(stage_in_act: u8) -> u8 function (0-3) in programs/boss-system/src/scaling.rs
- [x] T042 [P] [US3] Implement scale_week1_stats(base: &BossStats, tier: u8) -> BossStats in programs/boss-system/src/scaling.rs
- [x] T043 [P] [US3] Implement scale_week2_stats(base: &BossStats, tier: u8) -> BossStats in programs/boss-system/src/scaling.rs
- [x] T044 [P] [US3] Implement scale_week3_stats(base: &BossStats, tier: u8) -> BossStats in programs/boss-system/src/scaling.rs
- [x] T045 [US3] Implement apply_act_baseline(stats: &mut BossStats, act: u8, week: Week) function in programs/boss-system/src/scaling.rs
- [x] T046 [US3] Implement scale_boss(def: &BossDefinition, stage: u8, week: Week) -> ScaledBossStats main function in programs/boss-system/src/scaling.rs
- [x] T047 [US3] Implement getScaledBossStats instruction (view function) in programs/boss-system/src/lib.rs
- [x] T048 [US3] Add unit tests for tier calculation across all stages in programs/boss-system/tests/boss_scaling.rs
- [x] T049 [US3] Add unit tests for Week 1 scaling formulas (+2 HP×tier, +1 ARM×tier) in programs/boss-system/tests/boss_scaling.rs
- [x] T050 [US3] Add unit tests for Week 2 scaling formulas (+3 HP×tier, +1 ARM×tier, +1 ATK at tier>=2) in programs/boss-system/tests/boss_scaling.rs
- [x] T051 [US3] Add unit tests for Week 3 scaling formulas (+4 HP×tier, +1 ARM×tier, +1 ATK at tier>=1) in programs/boss-system/tests/boss_scaling.rs
- [x] T052 [US3] Add unit tests for Act 3/4 baseline bonuses in programs/boss-system/tests/boss_scaling.rs

**Checkpoint**: User Story 3 complete - stats scale correctly for all 80 stages

---

## Phase 6: User Story 4 - Boss Trait Execution (Priority: P2)

**Goal**: Boss traits trigger during combat at correct moments with correct effects

**Independent Test**: Execute boss combat and verify each trait triggers at the correct moment

### Implementation for User Story 4

- [x] T053 [P] [US4] Define ItemEffect conversion for standard traits (BattleStart, TurnStart, OnHit) in programs/boss-system/src/traits.rs
- [x] T054 [P] [US4] Define ItemEffect conversion for EveryOtherTurn traits in programs/boss-system/src/traits.rs
- [x] T055 [P] [US4] Define ItemEffect conversion for Wounded and FirstTurn traits in programs/boss-system/src/traits.rs
- [x] T056 [US4] Implement trait_to_item_effect(trait: &BossTrait) -> ItemEffect function in programs/boss-system/src/traits.rs
- [x] T057 [US4] Implement get_boss_effects(def: &BossDefinition) -> Vec<ItemEffect> function in programs/boss-system/src/traits.rs
- [x] T058 [US4] Implement getBossEffects instruction (view function) in programs/boss-system/src/lib.rs
- [x] T059 [US4] Add CountdownState struct for tracking Powder Keg Baron countdown in programs/boss-system/src/traits.rs
- [x] T060 [US4] Implement countdown trait handling (decrement each turn, fire at 0) in programs/boss-system/src/traits.rs
- [x] T061 [US4] Add ReflectionState struct for Crystal Mimic in programs/boss-system/src/traits.rs
- [x] T062 [US4] Implement reflection mechanic (intercept status, decrement stacks) in programs/boss-system/src/traits.rs
- [x] T063 [US4] Add unit tests for Broodmother traits (3 strikes, Chill every other turn) in programs/boss-system/tests/boss_traits.rs
- [x] T064 [US4] Add unit tests for Obsidian Golem traits (Turn Start +4 ARM, non-weapon removes ARM) in programs/boss-system/tests/boss_traits.rs
- [x] T065 [US4] Add unit tests for countdown trait execution in programs/boss-system/tests/boss_traits.rs
- [x] T066 [US4] Add unit tests for reflection trait execution in programs/boss-system/tests/boss_traits.rs

**Checkpoint**: User Story 4 complete - all standard traits execute correctly

---

## Phase 7: User Story 5 - Boss Phase Mechanics (Priority: P2)

**Goal**: Week 3 finals have phase-based mechanics at HP thresholds (75%, 50%, 25%)

**Independent Test**: Run Week 3 final combat and verify phase transitions at correct HP thresholds

### Implementation for User Story 5

- [x] T067 [P] [US5] Define PhaseState struct (phase_1_triggered, phase_2_triggered, phase_3_triggered) in programs/boss-system/src/traits.rs
- [x] T068 [P] [US5] Define Phase enum (One, Two, Three) in programs/boss-system/src/traits.rs
- [x] T069 [US5] Implement check_phase_transition(hp: i16, max_hp: u16, state: &mut PhaseState) -> Option<Phase> in programs/boss-system/src/traits.rs
- [x] T070 [US5] Define phase effects for Eldritch Mole (75%: +10 ARM, 50%: 2 strikes, 25%: 2 Bleed/turn) in programs/boss-system/src/traits.rs
- [x] T071 [US5] Define phase effects for Gilded Devourer (Wounded: 3 Bleed) in programs/boss-system/src/traits.rs
- [x] T072 [US5] Define phase effects for Frostbound Leviathan (Exposed: clear Chill +2 SPD) in programs/boss-system/src/traits.rs
- [x] T073 [US5] Define phase effects for Rusted Chronomancer (Wounded: 4 Bleed) in programs/boss-system/src/traits.rs
- [x] T074 [US5] Implement get_phase_effects(boss_id: &[u8; 12], phase: Phase) -> Vec<ItemEffect> in programs/boss-system/src/traits.rs
- [x] T075 [US5] Add unit tests for phase threshold detection (75%, 50%, 25%) in programs/boss-system/tests/boss_phases.rs
- [x] T076 [US5] Add unit tests for Eldritch Mole phase effects in programs/boss-system/tests/boss_phases.rs
- [x] T077 [US5] Add unit tests for Deep Dig modifier (player DIG > boss reduces Phase 1 armor) in programs/boss-system/tests/boss_phases.rs
- [x] T078 [US5] Add unit tests verifying each phase triggers exactly once in programs/boss-system/tests/boss_phases.rs

**Checkpoint**: User Story 5 complete - Week 3 finals have working phase mechanics

---

## Phase 8: User Story 6 - Boss Weakness Integration (Priority: P3)

**Goal**: Provide boss weakness tags for item offer generation weighting

**Independent Test**: Query any boss and retrieve correct 2 weakness tags

### Implementation for User Story 6

- [x] T079 [US6] Implement get_boss_weaknesses(stage: u8, week: Week) -> [ItemTag; 2] function in programs/boss-system/src/bosses.rs
- [x] T080 [US6] Implement getBossWeaknesses instruction (view function) in programs/boss-system/src/lib.rs
- [x] T081 [US6] Add unit tests for all 14 unique boss weakness tag pairs in programs/boss-system/tests/boss_weaknesses.rs
- [x] T082 [US6] Add unit tests verifying weakness retrieval for each week type in programs/boss-system/tests/boss_weaknesses.rs

**Checkpoint**: User Story 6 complete - weakness tags available for item system integration

---

## Phase 9: Combat Integration

**Goal**: Convert boss data to combat-system compatible format

**Independent Test**: Initialize boss combat with scaled stats and effects

### Implementation for Combat Integration

- [x] T083 [P] Implement to_combatant_input(scaled: &ScaledBossStats, strikes: u8) -> CombatantInput in programs/boss-system/src/lib.rs
- [x] T084 Implement getBossCombatInput instruction (view function) in programs/boss-system/src/lib.rs
- [x] T085 Add integration test for boss combat initialization in programs/boss-system/tests/integration/boss_combat.rs
- [x] T086 Add integration test for boss trait effects in combat in programs/boss-system/tests/integration/boss_combat.rs
- [x] T087 Add integration test for phase transitions during combat in programs/boss-system/tests/integration/boss_combat.rs

**Checkpoint**: Combat integration complete - bosses can be used in combat system

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Final validation and cleanup

- [x] T088 [P] Verify all 24 bosses have correct traits per GDD FR-024 through FR-037
- [x] T089 [P] Run cargo clippy on boss-system program and fix warnings
- [x] T090 [P] Run cargo fmt on all boss-system source files
- [x] T091 Generate IDL with anchor build -p boss_system
- [x] T092 Validate generated IDL matches specs/006-boss-system/contracts/boss_system.idl.json
- [x] T093 Run all unit tests with cargo test -p boss-system
- [x] T094 Run quickstart.md validation scenarios
- [x] T095 Update CLAUDE.md if any new patterns established

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Story 1 (Phase 3)**: Depends on Foundational - defines all 24 bosses
- **User Story 2 (Phase 4)**: Depends on US1 (needs boss definitions to select)
- **User Story 3 (Phase 5)**: Depends on US1 (needs base stats to scale)
- **User Story 4 (Phase 6)**: Depends on US1 (needs trait definitions)
- **User Story 5 (Phase 7)**: Depends on US4 (extends trait system with phases)
- **User Story 6 (Phase 8)**: Depends on US1 (needs weakness tags from definitions)
- **Combat Integration (Phase 9)**: Depends on US3 + US4 (needs scaling + traits)
- **Polish (Phase 10)**: Depends on all phases complete

### User Story Dependencies

```
US1 (Boss Registry)     ──┬──► US2 (Selection) ──────────────────────┐
                          │                                          │
                          ├──► US3 (Scaling) ────────────────────────┤
                          │                                          │
                          ├──► US4 (Traits) ──► US5 (Phases) ────────┼──► Combat Integration
                          │                                          │
                          └──► US6 (Weaknesses) ─────────────────────┘
```

### Parallel Opportunities

**Within Phase 1 (Setup)**:

- T004 and T005 can run in parallel (different files)

**Within Phase 2 (Foundational)**:

- T006-T010 must be sequential (same file lib.rs)
- T011-T014 can run in parallel with T006-T010 (different files)

**Within Phase 3 (US1 - Boss Registry)**:

- T015, T016, T017, T018 can run in parallel (all in traits.rs but independent sections)
- T019-T024 must be sequential (building BOSSES array)

**Within Phase 4 (US2 - Selection)**:

- T029, T030, T031 can run in parallel (independent helper functions)
- T036-T040 can run in parallel (different test scenarios)

**Within Phase 5 (US3 - Scaling)**:

- T041-T044 can run in parallel (independent scaling functions)
- T048-T052 can run in parallel (different test scenarios)

**Within Phase 6 (US4 - Traits)**:

- T053, T054, T055 can run in parallel (independent conversions)
- T063-T066 can run in parallel (different test scenarios)

**Within Phase 7 (US5 - Phases)**:

- T067, T068 can run in parallel (different structs)
- T070-T073 can run in parallel (different boss phase definitions)
- T075-T078 can run in parallel (different test scenarios)

---

## Parallel Example: User Story 1 Boss Definitions

```bash
# Launch all trait definitions in parallel:
Task: "Define Week 1 Biome A boss traits in programs/boss-system/src/traits.rs"
Task: "Define Week 2 Biome A boss traits in programs/boss-system/src/traits.rs"
Task: "Define Week 3 Biome A final boss traits in programs/boss-system/src/traits.rs"
Task: "Define Week 3 Biome B final boss traits in programs/boss-system/src/traits.rs"
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational
3. Complete Phase 3: User Story 1 (Boss Registry)
4. Complete Phase 4: User Story 2 (Boss Selection)
5. **STOP and VALIDATE**: Can select any boss for any stage
6. Deploy/demo basic boss lookup functionality

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 (Boss Registry) → 24 bosses defined → Demo
3. Add US2 (Selection) → Stage mapping works → Demo
4. Add US3 (Scaling) → Stats scale by act/tier → Demo
5. Add US4 (Traits) → Basic traits work → Demo
6. Add US5 (Phases) → Week 3 finals complete → Demo
7. Add US6 (Weaknesses) → Item integration ready → Demo
8. Combat Integration → Full boss combat → Release

### Task Summary

| Phase               | Tasks  | Parallelizable |
| ------------------- | ------ | -------------- |
| Setup               | 5      | 2              |
| Foundational        | 9      | 4              |
| US1 - Boss Registry | 14     | 6              |
| US2 - Selection     | 12     | 8              |
| US3 - Scaling       | 12     | 9              |
| US4 - Traits        | 14     | 7              |
| US5 - Phases        | 12     | 8              |
| US6 - Weaknesses    | 4      | 0              |
| Combat Integration  | 5      | 1              |
| Polish              | 8      | 3              |
| **Total**           | **95** | **48**         |

---

## Notes

- [P] tasks = different files or independent sections, no dependencies
- [US#] label maps task to specific user story for traceability
- US1 is MVP - defines all 24 bosses
- US1 + US2 together provide basic boss lookup by stage
- Constitution requires TDD - write tests that fail before implementation
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
