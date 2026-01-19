# Tasks: Item System

**Input**: Design documents from `/specs/004-item-system/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Test-Driven Development is required per constitution (II). Tests are written first.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4, US5, US6)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md, the project structure is:

- **Program**: `programs/player-inventory/src/`
- **Item Data**: Compile-time constants in program code
- **Tests**: `tests/player-inventory/` (unit in Rust, integration in TypeScript)

---

## Phase 1: Setup (Project Initialization)

**Purpose**: Create the player-inventory program structure

- [x] T001 Create program directory structure at programs/player-inventory/
- [x] T002 Initialize Cargo.toml for player-inventory program with Anchor 0.31.1 dependencies
- [x] T003 Add player_inventory to Anchor.toml programs configuration
- [x] T004 [P] Create constants.rs with inventory constants (INITIAL_GEAR_SLOTS=4, MAX_GEAR_SLOTS=8) in programs/player-inventory/src/constants.rs
- [x] T005 [P] Create errors.rs with InventoryError enum (InventoryFull, InvalidItemId, etc.) in programs/player-inventory/src/errors.rs

---

## Phase 2: Foundational (Core Types & Enums)

**Purpose**: Define all types and enums that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T006 [P] Create ItemType enum (Tool, Gear) in programs/player-inventory/src/state.rs
- [x] T007 [P] Create ItemTag enum (Stone, Scout, Greed, Blast, Frost, Rust, Blood, Tempo) in programs/player-inventory/src/state.rs
- [x] T008 [P] Create Rarity enum (Common, Rare, Heroic, Mythic) in programs/player-inventory/src/state.rs
- [x] T009 [P] Create Tier enum (I, II, III) in programs/player-inventory/src/state.rs
- [x] T010 [P] Create TriggerType enum (matching combat-system) in programs/player-inventory/src/state.rs
- [x] T011 [P] Create EffectType enum (matching combat-system) in programs/player-inventory/src/state.rs
- [x] T012 [P] Create EffectDefinition struct (trigger, effect_type, once_per_turn, values[3]) in programs/player-inventory/src/state.rs
- [x] T013 [P] Create ItemEffect struct (matching combat-system format) in programs/player-inventory/src/state.rs
- [x] T014 Create lib.rs with declare_id! and module declarations in programs/player-inventory/src/lib.rs

**Checkpoint**: Foundation ready - all enums and base types defined

---

## Phase 3: User Story 1 - Item Data Registry (Priority: P1) 🎯 MVP

**Goal**: Store 80 item definitions as compile-time constants, queryable by ID, tag, type, and rarity

**Independent Test**: Query item "T-ST-01" and verify it returns Bulwark Shovel with correct properties

### Tests for User Story 1

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [x] T015 [P] [US1] Unit test for get_item("T-ST-01") returns correct ItemDefinition in programs/player-inventory/src/items.rs
- [x] T016 [P] [US1] Unit test for get_items_by_tag(Stone) returns 10 items in programs/player-inventory/src/items.rs
- [x] T017 [P] [US1] Unit test for get_items_by_rarity(Heroic) returns correct count in programs/player-inventory/src/items.rs
- [x] T018 [P] [US1] Unit test for get_item("X-XX-99") returns None in programs/player-inventory/src/items.rs
- [x] T019 [US1] Integration test verifying all 80 items are defined in tests/player-inventory/item_queries.ts

### Implementation for User Story 1

- [x] T020 [P] [US1] Create ItemDefinition struct (id, name, item_type, tag, rarity, effects) in programs/player-inventory/src/items.rs
- [x] T021 [US1] Define STONE tag items (T-ST-01, T-ST-02, G-ST-01 through G-ST-08) in programs/player-inventory/src/items.rs
- [x] T022 [P] [US1] Define SCOUT tag items (T-SC-01, T-SC-02, G-SC-01 through G-SC-08) in programs/player-inventory/src/items.rs
- [x] T023 [P] [US1] Define GREED tag items (T-GR-01, T-GR-02, G-GR-01 through G-GR-08) in programs/player-inventory/src/items.rs
- [x] T024 [P] [US1] Define BLAST tag items (T-BL-01, T-BL-02, G-BL-01 through G-BL-08) in programs/player-inventory/src/items.rs
- [x] T025 [P] [US1] Define FROST tag items (T-FR-01, T-FR-02, G-FR-01 through G-FR-08) in programs/player-inventory/src/items.rs
- [x] T026 [P] [US1] Define RUST tag items (T-RU-01, T-RU-02, G-RU-01 through G-RU-08) in programs/player-inventory/src/items.rs
- [x] T027 [P] [US1] Define BLOOD tag items (T-BO-01, T-BO-02, G-BO-01 through G-BO-08) in programs/player-inventory/src/items.rs
- [x] T028 [P] [US1] Define TEMPO tag items (T-TE-01, T-TE-02, G-TE-01 through G-TE-08) in programs/player-inventory/src/items.rs
- [x] T029 [US1] Create ITEMS constant array containing all 80 items in programs/player-inventory/src/items.rs
- [x] T030 [US1] Implement get_item(id) -> Option<&ItemDefinition> in programs/player-inventory/src/items.rs
- [x] T031 [US1] Implement get_items_by_tag(tag) -> Vec<&ItemDefinition> in programs/player-inventory/src/items.rs
- [x] T032 [US1] Implement get_items_by_rarity(rarity) -> Vec<&ItemDefinition> in programs/player-inventory/src/items.rs

**Checkpoint**: All 80 items queryable by ID, tag, type, and rarity

---

## Phase 4: User Story 2 - Player Inventory Management (Priority: P2)

**Goal**: Player inventory with Tool slot (1) and Gear slots (4-8), equip/unequip items

**Independent Test**: Initialize inventory, equip tool and 4 gear items, verify state is correct

### Tests for User Story 2

- [x] T033 [P] [US2] Unit test for inventory initialization with 4 gear slots in programs/player-inventory/src/state.rs
- [x] T034 [P] [US2] Unit test for equip_tool replaces existing tool in programs/player-inventory/src/state.rs
- [x] T035 [P] [US2] Unit test for equip_gear fails when slots full in programs/player-inventory/src/state.rs
- [x] T036 [P] [US2] Unit test for unequip_gear removes item from slot in programs/player-inventory/src/state.rs
- [x] T037 [US2] Integration test for full inventory management flow in tests/player-inventory/inventory_management.ts

### Implementation for User Story 2

- [x] T038 [P] [US2] Create ItemInstance struct (item_id, tier, tool_oil_flags) in programs/player-inventory/src/state.rs
- [x] T039 [US2] Create PlayerInventory account struct (player, tool, gear[8], gear_slot_capacity, bump) in programs/player-inventory/src/state.rs
- [x] T040 [US2] Implement InitializeInventory accounts struct and handler in programs/player-inventory/src/lib.rs
- [x] T041 [US2] Implement EquipTool accounts struct and handler in programs/player-inventory/src/lib.rs
- [x] T042 [US2] Implement EquipGear accounts struct and handler in programs/player-inventory/src/lib.rs
- [x] T043 [US2] Implement UnequipGear accounts struct and handler in programs/player-inventory/src/lib.rs
- [x] T044 [US2] Emit InventoryInitialized, ItemEquipped, ItemUnequipped events in programs/player-inventory/src/lib.rs

**Checkpoint**: Inventory management works - equip/unequip tools and gear

---

## Phase 5: User Story 3 - Item Tier Upgrades (Priority: P3)

**Goal**: Fuse two identical items at same tier to produce one item at next tier

**Independent Test**: Give player two Tier I Miner Helmets, fuse them, verify result is one Tier II Miner Helmet

### Tests for User Story 3

- [x] T045 [P] [US3] Unit test for valid fusion (same ID, same tier) produces next tier in programs/player-inventory/src/fusion.rs
- [x] T046 [P] [US3] Unit test for fusion rejects mismatched item IDs in programs/player-inventory/src/fusion.rs
- [x] T047 [P] [US3] Unit test for fusion rejects mismatched tiers in programs/player-inventory/src/fusion.rs
- [x] T048 [P] [US3] Unit test for fusion rejects Tier III items in programs/player-inventory/src/fusion.rs
- [x] T049 [US3] Integration test for fusion flow in tests/player-inventory/fusion.ts

### Implementation for User Story 3

- [x] T050 [US3] Implement validate_fusion(item_a, item_b) -> Result<()> in programs/player-inventory/src/fusion.rs
- [x] T051 [US3] Implement execute_fusion(tier) -> Tier in programs/player-inventory/src/fusion.rs
- [x] T052 [US3] Implement FuseItems accounts struct and handler in programs/player-inventory/src/lib.rs
- [x] T053 [US3] Emit ItemFused event with old/new tier in programs/player-inventory/src/lib.rs

**Checkpoint**: Tier upgrades work - fuse identical items to upgrade

---

## Phase 6: User Story 4 - Itemset Bonuses (Priority: P4)

**Goal**: Detect active itemsets and include bonus effects in combat

**Independent Test**: Equip Union Standard set items (G-ST-01, G-ST-02, G-SC-01), verify set bonus detected

### Tests for User Story 4

- [x] T054 [P] [US4] Unit test for Union Standard set detection in programs/player-inventory/src/itemsets.rs
- [x] T055 [P] [US4] Unit test for partial set (2 of 3 items) returns no bonus in programs/player-inventory/src/itemsets.rs
- [x] T056 [P] [US4] Unit test for multiple active sets in programs/player-inventory/src/itemsets.rs
- [x] T057 [US4] Integration test for itemset activation in tests/player-inventory/itemsets.ts

### Implementation for User Story 4

- [x] T058 [P] [US4] Create ItemsetDefinition struct (id, name, emoji, required_items, bonus_effect) in programs/player-inventory/src/itemsets.rs
- [x] T059 [US4] Define all 12 itemsets as ITEMSETS constant in programs/player-inventory/src/itemsets.rs
- [x] T060 [US4] Implement inventory.has_item(id) helper in programs/player-inventory/src/state.rs
- [x] T061 [US4] Implement get_active_itemsets(inventory) -> Vec<&ItemsetDefinition> in programs/player-inventory/src/itemsets.rs

**Checkpoint**: Itemsets work - detect when all required items equipped

---

## Phase 7: User Story 5 - Item Offer Generation (Priority: P5)

**Goal**: Generate weighted item offers for POIs based on act rarity table and boss weaknesses

**Independent Test**: Generate 1000 Supply Cache offers in Week 1, verify STONE/FROST items appear ~40% more often

### Tests for User Story 5

- [x] T062 [P] [US5] Unit test for rarity table lookup by POI and act in programs/player-inventory/src/offers.rs
- [x] T063 [P] [US5] Unit test for tag weighting (1.4x for weakness tags) in programs/player-inventory/src/offers.rs
- [x] T064 [P] [US5] Unit test for Counter Cache draws only from weakness tags in programs/player-inventory/src/offers.rs
- [x] T065 [P] [US5] Unit test for Mythic cap (max 1 per offer) in programs/player-inventory/src/offers.rs
- [x] T066 [US5] Integration test for offer generation statistics in tests/player-inventory/offers.ts

### Implementation for User Story 5

- [x] T067 [P] [US5] Create PoiType enum in programs/player-inventory/src/offers.rs
- [x] T068 [P] [US5] Create ItemOffer struct (item_id, tier, price) in programs/player-inventory/src/offers.rs
- [x] T069 [US5] Implement get_rarity_from_table(poi_type, act, seed) -> Rarity in programs/player-inventory/src/offers.rs
- [x] T070 [US5] Implement calculate_tag_weights(boss_weaknesses) -> HashMap<ItemTag, f32> in programs/player-inventory/src/offers.rs
- [x] T071 [US5] Implement weighted_select(candidates, weights, seed) -> &ItemDefinition in programs/player-inventory/src/offers.rs
- [x] T072 [US5] Implement generate_item_offer(poi_type, act, week, boss_weaknesses, seed) -> ItemOffer in programs/player-inventory/src/offers.rs
- [x] T073 [US5] Implement GenerateItemOffer instruction in programs/player-inventory/src/lib.rs

**Checkpoint**: Offer generation works - weighted by boss weaknesses

---

## Phase 8: User Story 6 - Combat Effect Resolution (Priority: P6)

**Goal**: Convert equipped items and itemsets to ItemEffect arrays for combat system

**Independent Test**: Equip Rime Pike tool, verify combat effects include +2 ATK and OnHit Chill

### Tests for User Story 6

- [x] T074 [P] [US6] Unit test for tool effects with tier scaling in programs/player-inventory/src/effects.rs
- [x] T075 [P] [US6] Unit test for gear effects accumulation in programs/player-inventory/src/effects.rs
- [x] T076 [P] [US6] Unit test for itemset bonus inclusion in programs/player-inventory/src/effects.rs
- [x] T077 [P] [US6] Unit test for Tool Oil modifications (+ATK/+SPD/+DIG) in programs/player-inventory/src/effects.rs
- [x] T078 [US6] Integration test for combat integration in tests/player-inventory/combat_integration.ts

### Implementation for User Story 6

- [x] T079 [US6] Implement effect_definition_to_item_effect(def, tier) -> ItemEffect in programs/player-inventory/src/effects.rs
- [x] T080 [US6] Implement generate_tool_effects(tool) -> Vec<ItemEffect> in programs/player-inventory/src/effects.rs
- [x] T081 [US6] Implement generate_gear_effects(gear_slots) -> Vec<ItemEffect> in programs/player-inventory/src/effects.rs
- [x] T082 [US6] Implement generate_itemset_effects(inventory) -> Vec<ItemEffect> in programs/player-inventory/src/effects.rs
- [x] T083 [US6] Implement generate_combat_effects(inventory) -> Vec<ItemEffect> in programs/player-inventory/src/effects.rs
- [x] T084 [US6] Implement GetCombatEffects instruction in programs/player-inventory/src/lib.rs

**Checkpoint**: Combat integration works - items produce correct effects

---

## Phase 9: Additional Features

**Purpose**: Gear slot expansion and Tool Oil modifications

- [x] T085 [P] Create ToolOilModification enum (PlusAtk, PlusSpd, PlusDig) in programs/player-inventory/src/state.rs
- [x] T086 Implement ExpandGearSlots accounts struct and handler in programs/player-inventory/src/lib.rs
- [x] T087 Implement ApplyToolOil accounts struct and handler in programs/player-inventory/src/lib.rs
- [x] T088 [P] Emit GearSlotsExpanded and ToolOilApplied events in programs/player-inventory/src/lib.rs

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Final validation, documentation, and cleanup

- [x] T089 [P] Run cargo clippy on player-inventory with no warnings
- [x] T090 [P] Run cargo fmt on all player-inventory source files
- [x] T091 [P] Verify anchor build -p player_inventory succeeds
- [x] T092 Run all unit tests with cargo test -p player-inventory
- [ ] T093 Run all integration tests with anchor test
- [x] T094 Validate against quickstart.md examples
- [x] T095 [P] Add IDL documentation comments to all instructions in programs/player-inventory/src/lib.rs
- [x] T096 Update Anchor.toml with player_inventory program ID after first deploy

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phases 3-8)**: All depend on Foundational phase completion
  - US1 (Item Registry) should complete first as other stories need items
  - US2-US6 can proceed after US1 if team capacity allows
- **Additional Features (Phase 9)**: Depends on US2 (Inventory)
- **Polish (Phase 10)**: Depends on all stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundation only - item definitions
- **User Story 2 (P2)**: Foundation + US1 (needs item lookup)
- **User Story 3 (P3)**: Foundation + US2 (needs inventory)
- **User Story 4 (P4)**: Foundation + US1 + US2 (needs items and inventory)
- **User Story 5 (P5)**: Foundation + US1 (needs items for generation)
- **User Story 6 (P6)**: Foundation + US1 + US2 + US4 (needs all)

### Parallel Opportunities by Phase

**Phase 2 (Foundational)**:

```
T006, T007, T008, T009, T010, T011, T012, T013 can all run in parallel
```

**Phase 3 (US1 - Item definitions)**:

```
T021, T022, T023, T024, T025, T026, T027, T028 can all run in parallel (different tags)
```

**Phase 4 (US2 Tests)**:

```
T033, T034, T035, T036 can all run in parallel
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL)
3. Complete Phase 3: User Story 1 (Item Registry)
4. **STOP and VALIDATE**: Query items, verify all 80 defined
5. Deploy/demo - item lookup works

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add User Story 1 → **MVP: Items queryable**
3. Add User Story 2 → Inventory management works
4. Add User Story 3 → Tier upgrades work
5. Add User Story 4 → Itemsets detected
6. Add User Story 5 → Offer generation works
7. Add User Story 6 → Combat integration complete
8. Polish → Production ready

---

## Notes

- TDD required per constitution - write failing tests before implementation
- All 80 items from GDD Section 9 must be defined
- All 12 itemsets from GDD Section 10 must be defined
- Item definitions are compile-time constants (no account reads)
- Combat effects use same format as combat-system (ItemEffect)
- [P] tasks can run in parallel within their phase
