# Tasks: Combat System & Status Effects

**Input**: Design documents from `/specs/003-combat-system/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Test-Driven Development is required per constitution (II). Tests are written first.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4, US5)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md, the project structure is:

- **Program**: `programs/combat-system/src/`
- **Tests**: `tests/combat-system/` (unit in Rust, integration in TypeScript)

---

## Phase 1: Setup (Project Initialization)

**Purpose**: Create the combat-system program structure

- [x] T001 Create program directory structure at programs/combat-system/
- [x] T002 Initialize Cargo.toml for combat-system program with Anchor 0.31.1 dependencies
- [x] T003 Add combat_system to Anchor.toml programs configuration
- [x] T004 [P] Create constants.rs with combat constants (SUDDEN_DEATH_TURN=25, MAX_TURNS=50, MIN_STRIKES=1) in programs/combat-system/src/constants.rs
- [x] T005 [P] Create errors.rs with CombatSystemError enum in programs/combat-system/src/errors.rs

---

## Phase 2: Foundational (Core Types & State)

**Purpose**: Define all types and state structures that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 [P] Create StatusEffects struct (chill, shrapnel, rust, bleed as u8) in programs/combat-system/src/state.rs
- [x] T007 [P] Create CombatantInput struct for initializing combatants in programs/combat-system/src/state.rs
- [x] T008 [P] Create TriggerType enum (BattleStart, FirstTurn, TurnStart, EveryOtherTurn, OnHit, Exposed, Wounded) in programs/combat-system/src/state.rs
- [x] T009 [P] Create EffectType enum (DealDamage, DealNonWeaponDamage, Heal, GainArmor, etc.) in programs/combat-system/src/state.rs
- [x] T010 [P] Create ItemEffect struct (trigger, once_per_turn, effect_type, value) in programs/combat-system/src/state.rs
- [x] T011 [P] Create ResolutionType enum for combat outcomes in programs/combat-system/src/state.rs
- [x] T012 Create CombatState account struct with all fields per data-model.md in programs/combat-system/src/state.rs
- [x] T013 Create lib.rs with declare_id! and module declarations in programs/combat-system/src/lib.rs
- [x] T014 Create all event structs (CombatInitialized, TurnExecuted, StatusApplied, CombatResult) in programs/combat-system/src/lib.rs

**Checkpoint**: Foundation ready - all types defined, user story implementation can begin

---

## Phase 3: User Story 1 - Basic Auto-Combat Resolution (Priority: P1) 🎯 MVP

**Goal**: Combat resolves turn-by-turn based on stats, with SPD determining turn order, damage calculation, sudden death, and failsafe

**Independent Test**: Initialize combat with player (ATK 3, SPD 2) vs enemy (HP 6, ARM 1, SPD 1), verify player wins with correct damage per turn

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T015 [P] [US1] Unit test for damage calculation (max(0, ATK - ARM)) in programs/combat-system/src/engine.rs (as #[cfg(test)] module)
- [x] T016 [P] [US1] Unit test for SPD-based turn order (higher SPD acts first, ties go to enemy) in programs/combat-system/src/engine.rs
- [x] T017 [P] [US1] Unit test for sudden death (Turn 25+ adds +1 ATK per turn) in programs/combat-system/src/engine.rs
- [x] T018 [P] [US1] Unit test for failsafe (Turn 50, HP% comparison, ties favor enemy) in programs/combat-system/src/engine.rs
- [x] T019 [US1] Integration test for basic combat resolution in tests/combat-system/basic_combat.ts

### Implementation for User Story 1

- [x] T020 [US1] Implement calculate_weapon_damage(atk, arm) -> i16 in programs/combat-system/src/engine.rs
- [x] T021 [US1] Implement determine_turn_order(player_spd, enemy_spd) -> (first, second) in programs/combat-system/src/engine.rs
- [x] T022 [US1] Implement check_sudden_death(turn) -> i16 bonus in programs/combat-system/src/engine.rs
- [x] T023 [US1] Implement check_failsafe(turn, player_hp, player_max_hp, enemy_hp, enemy_max_hp) -> Option<bool> in programs/combat-system/src/engine.rs
- [x] T024 [US1] Implement execute_strike(attacker_atk, defender_arm, defender_hp) -> (new_hp, damage_dealt) in programs/combat-system/src/engine.rs
- [x] T025 [US1] Implement InitializeCombat accounts struct and handler in programs/combat-system/src/lib.rs
- [x] T026 [US1] Implement ResolveCombat accounts struct in programs/combat-system/src/lib.rs
- [x] T027 [US1] Implement resolve_combat instruction handler with turn loop in programs/combat-system/src/lib.rs
- [x] T028 [US1] Implement CloseCombat accounts struct and handler in programs/combat-system/src/lib.rs
- [x] T029 [US1] Emit CombatInitialized and CombatResult events

**Checkpoint**: Basic combat works - player vs enemy resolves to winner based on stats alone

---

## Phase 4: User Story 2 - Status Effect Application (Priority: P2)

**Goal**: Chill, Shrapnel, Rust, and Bleed modify combat behavior according to their rules

**Independent Test**: Apply 2 Chill to enemy, verify strikes reduced by 2 (min 1), verify 1 stack removed at turn end

### Tests for User Story 2

- [x] T030 [P] [US2] Unit test for Chill (reduces strikes, min 1, decays 1/turn) in programs/combat-system/src/effects.rs
- [x] T031 [P] [US2] Unit test for Shrapnel (damage on hit, clears at turn end) in programs/combat-system/src/effects.rs
- [x] T032 [P] [US2] Unit test for Rust (armor decay, persists) in programs/combat-system/src/effects.rs
- [x] T033 [P] [US2] Unit test for Bleed (DoT at turn end, decays 1/turn) in programs/combat-system/src/effects.rs
- [x] T034 [US2] Integration test for status effects in combat in tests/combat-system/status_effects.ts

### Implementation for User Story 2

- [x] T035 [US2] Implement apply_chill_to_strikes(base_strikes, chill_stacks) -> u8 in programs/combat-system/src/effects.rs
- [x] T036 [US2] Implement process_shrapnel_retaliation(shrapnel_stacks, attacker_hp) -> i16 in programs/combat-system/src/effects.rs
- [x] T037 [US2] Implement process_rust_decay(rust_stacks, current_arm) -> i16 in programs/combat-system/src/effects.rs
- [x] T038 [US2] Implement process_bleed_damage(bleed_stacks, current_hp) -> i16 in programs/combat-system/src/effects.rs
- [x] T039 [US2] Implement decay_status_effects(status: &mut StatusEffects) in programs/combat-system/src/effects.rs
- [x] T040 [US2] Integrate status effect processing into resolve_combat turn loop in programs/combat-system/src/lib.rs
- [x] T041 [US2] Emit StatusApplied events when effects are applied

**Checkpoint**: Combat with status effects works - all 4 effects function correctly

---

## Phase 5: User Story 3 - Combat Trigger System (Priority: P3)

**Goal**: Item effects activate at correct combat phases (BattleStart, TurnStart, FirstTurn, EveryOtherTurn, Exposed, Wounded, OnHit)

**Independent Test**: Pass BattleStart effect (+5 ARM), verify player gains 5 ARM before Turn 1

### Tests for User Story 3

- [x] T042 [P] [US3] Unit test for BattleStart trigger (before Turn 1) in programs/combat-system/src/triggers.rs
- [x] T043 [P] [US3] Unit test for FirstTurn trigger (only Turn 1) in programs/combat-system/src/triggers.rs
- [x] T044 [P] [US3] Unit test for TurnStart trigger (every turn) in programs/combat-system/src/triggers.rs
- [x] T045 [P] [US3] Unit test for EveryOtherTurn trigger (turns 2,4,6...) in programs/combat-system/src/triggers.rs
- [x] T046 [P] [US3] Unit test for Exposed condition (ARM = 0) in programs/combat-system/src/triggers.rs
- [x] T047 [P] [US3] Unit test for Wounded condition (HP < 50% max) in programs/combat-system/src/triggers.rs
- [x] T048 [US3] Integration test for trigger system in tests/combat-system/triggers.ts

### Implementation for User Story 3

- [x] T049 [US3] Implement should_trigger(trigger_type, turn, is_first_turn) -> bool in programs/combat-system/src/triggers.rs
- [x] T050 [US3] Implement check_exposed(arm) -> bool in programs/combat-system/src/triggers.rs
- [x] T051 [US3] Implement check_wounded(hp, max_hp) -> bool in programs/combat-system/src/triggers.rs
- [x] T052 [US3] Implement apply_effect(effect_type, value, target_stats, target_status) in programs/combat-system/src/triggers.rs
- [x] T053 [US3] Implement process_triggers_for_phase(effects, phase, turn, stats, status) in programs/combat-system/src/triggers.rs
- [x] T054 [US3] Integrate trigger processing into resolve_combat at correct phases in programs/combat-system/src/lib.rs
- [x] T055 [US3] Add once_per_turn tracking flags to combat loop in programs/combat-system/src/lib.rs

**Checkpoint**: Triggers work - effects fire at correct combat phases

---

## Phase 6: User Story 4 - Multi-Strike Combat (Priority: P4)

**Goal**: Combatants with multiple strikes execute them independently, respecting once-per-turn limits

**Independent Test**: Player with 2 strikes and OnHit effect, verify 2 damage instances but only 1 trigger

### Tests for User Story 4

- [x] T056 [P] [US4] Unit test for multi-strike damage (each strike resolves independently) in programs/combat-system/src/engine.rs
- [x] T057 [P] [US4] Unit test for once-per-turn limits across strikes in programs/combat-system/src/engine.rs
- [x] T058 [P] [US4] Unit test for Chill interaction with multi-strike in programs/combat-system/src/engine.rs
- [x] T059 [US4] Integration test for multi-strike combat in tests/combat-system/multi_strike.ts

### Implementation for User Story 4

- [x] T060 [US4] Implement execute_strikes(num_strikes, atk, defender_arm, defender_hp, on_hit_effects, triggered_flags) in programs/combat-system/src/engine.rs
- [x] T061 [US4] Refactor turn execution to use strike loop instead of single attack in programs/combat-system/src/lib.rs
- [x] T062 [US4] Implement once_per_turn flag reset at turn end in programs/combat-system/src/lib.rs
- [x] T063 [US4] Integrate Chill effect into strike count calculation in programs/combat-system/src/lib.rs

**Checkpoint**: Multi-strike works - multiple strikes resolve correctly with proper trigger limits

---

## Phase 7: User Story 5 - Deterministic Combat Resolution (Priority: P5)

**Goal**: Identical inputs always produce identical outputs, enabling replay verification

**Independent Test**: Run same combat configuration 100 times, verify all outcomes identical

### Tests for User Story 5

- [x] T064 [P] [US5] Unit test for deterministic damage sequences in programs/combat-system/src/engine.rs
- [x] T065 [P] [US5] Unit test for deterministic effect ordering in programs/combat-system/src/triggers.rs
- [x] T066 [US5] Integration test running same combat 100 times verifying identical results in tests/combat-system/determinism.ts

### Implementation for User Story 5

- [x] T067 [US5] Audit all combat functions for any sources of non-determinism in programs/combat-system/src/
- [x] T068 [US5] Document explicit effect ordering in code comments in programs/combat-system/src/lib.rs
- [x] T069 [US5] Add assertion that no randomness is used (no rand crate, no clock-based logic) in programs/combat-system/src/lib.rs

**Checkpoint**: Determinism verified - combat is fully reproducible

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Final validation, documentation, and cleanup

- [x] T070 [P] Run cargo clippy on combat-system with no warnings
- [x] T071 [P] Run cargo fmt on all combat-system source files
- [x] T072 [P] Verify anchor build -p combat_system succeeds
- [x] T073 Run all unit tests with cargo test -p combat-system
- [x] T074 Run all integration tests with anchor test
- [x] T075 Validate against quickstart.md examples
- [x] T076 [P] Add IDL documentation comments to all instructions in programs/combat-system/src/lib.rs
- [x] T077 Update Anchor.toml with combat_system program ID after first deploy

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phases 3-7)**: All depend on Foundational phase completion
  - US1 (Basic Combat) should complete first as other stories build on it
  - US2-US5 can proceed after US1 if team capacity allows
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundation only - core combat loop
- **User Story 2 (P2)**: Foundation + US1 combat loop to integrate into
- **User Story 3 (P3)**: Foundation + US1 combat loop to integrate into
- **User Story 4 (P4)**: Foundation + US1 + US2 (Chill affects strikes)
- **User Story 5 (P5)**: All previous stories (validates entire system)

### Parallel Opportunities by Phase

**Phase 2 (Foundational)**:

```
T006, T007, T008, T009, T010, T011 can all run in parallel
```

**Phase 3 (US1 Tests)**:

```
T015, T016, T017, T018 can all run in parallel
```

**Phase 4 (US2 Tests)**:

```
T030, T031, T032, T033 can all run in parallel
```

**Phase 5 (US3 Tests)**:

```
T042, T043, T044, T045, T046, T047 can all run in parallel
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL)
3. Complete Phase 3: User Story 1 (Basic Combat)
4. **STOP and VALIDATE**: Test basic combat independently
5. Deploy/demo - combat works with stats only, no effects

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → **MVP: Basic combat works**
3. Add User Story 2 → Status effects work
4. Add User Story 3 → Triggers work (items can have effects)
5. Add User Story 4 → Multi-strike works
6. Add User Story 5 → Determinism verified
7. Polish → Production ready

---

## Notes

- TDD required per constitution - write failing tests before implementation
- All arithmetic must use checked operations (checked_add, checked_sub, etc.)
- No randomness allowed - combat must be deterministic
- Emit events for indexer compatibility
- Effects passed as input parameters (items not implemented yet)
- [P] tasks can run in parallel within their phase
