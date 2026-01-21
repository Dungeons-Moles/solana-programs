# Tasks: Item Offer Generation

**Input**: Design documents from `/specs/008-item-offer-generation/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Tests are included per Constitution principle II (Test-Driven Development).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

Based on plan.md structure:

- **Program code**: `programs/poi-system/src/`
- **Tests**: `tests/poi-system/`

---

## Phase 1: Setup

**Purpose**: Module scaffolding and dependencies

- [x] T001 Add crate dependencies for combat-system, player-inventory, boss-system in programs/poi-system/Cargo.toml
- [x] T002 [P] Create module declaration for offer_generation in programs/poi-system/src/lib.rs
- [x] T003 [P] Create module declaration for rarity_tables in programs/poi-system/src/lib.rs
- [x] T004 [P] Create module declaration for tag_weights in programs/poi-system/src/lib.rs
- [x] T005 Create empty module files: programs/poi-system/src/offer_generation.rs, rarity_tables.rs, tag_weights.rs

**Note**: Implemented in single `offers.rs` module for simplicity (rarity tables and tag weights consolidated).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Core Types & RNG

- [x] T006 Implement Xorshift64 struct with new(), next(), next_bounded() in programs/poi-system/src/offer_generation.rs
- [x] T007 [P] Add OfferContext struct (act, week, biome, poi_type, seed, poi_index, offer_call_count, enable_final_prep_bias) in programs/poi-system/src/offer_generation.rs
- [x] T008 [P] Add OfferError enum with all error codes (InvalidPoiType, PoiAlreadyUsed, NoActiveInteraction, ShopAlreadyActive, NoActiveShop, InsufficientGold, NoItemsAvailable, InvalidOfferContext) in programs/poi-system/src/errors.rs

### State Extensions

- [x] T009 Add OfferItem struct (item_id, rarity) in programs/poi-system/src/state.rs
- [x] T010 Add CacheOffer struct (poi_index, items[3], generated_at_seed) in programs/poi-system/src/state.rs
- [x] T011 Add ShopItem struct (item_id, rarity, item_type, price) in programs/poi-system/src/state.rs
- [x] T012 Add ShopState struct (active, poi_index, reroll_count, inventory[6], rng_state) in programs/poi-system/src/state.rs
- [x] T013 Extend MapPois account with current_offer: Option<CacheOffer> and shop_state: Option<ShopState> in programs/poi-system/src/state.rs

### Item Pool Infrastructure

- [x] T014 Create ItemPool struct with indices field in programs/poi-system/src/offer_generation.rs
- [x] T015 Build compile-time ITEM_POOLS constant (8 tags × 4 rarities × 2 types) by indexing ALL_ITEMS in programs/poi-system/src/offer_generation.rs
- [x] T016 Implement select_item_from_pool(rng, pool) -> Option<ItemId> helper in programs/poi-system/src/offer_generation.rs
- [x] T017 Implement fallback_select_item(rng, rarity, item_type) for empty pool case in programs/poi-system/src/offer_generation.rs

**Note**: Implemented via format_gear_id/format_tool_id functions with rarity-based item number selection.

### Unit Tests for Foundational

- [x] T018 [P] Unit test: Xorshift64 determinism (same seed → same sequence) in programs/poi-system/src/offer_generation.rs
- [x] T019 [P] Unit test: Xorshift64 bounded output stays in range in programs/poi-system/src/offer_generation.rs
- [x] T020 [P] Unit test: ITEM_POOLS contains correct items for each (tag, rarity, type) in programs/poi-system/src/offer_generation.rs

**Checkpoint**: Foundation ready - user story implementation can now begin

---

## Phase 3: User Story 1 - Receive Item Offers at POIs (Priority: P1) 🎯 MVP

**Goal**: Generate 3-item offers for Supply Cache, Tool Crate, Geode Vault, Counter Cache POIs

**Independent Test**: Trigger any item-granting POI and verify 3 items appear with correct rarities

### Tests for User Story 1

- [x] T021 [P] [US1] Integration test: generate_cache_offer for Supply Cache returns 3 Gear items in tests/poi-system/offer_generation.ts
- [x] T022 [P] [US1] Integration test: generate_cache_offer for Tool Crate returns 3 Tool items in tests/poi-system/offer_generation.ts
- [x] T023 [P] [US1] Integration test: generate_cache_offer for Geode Vault returns 3 Heroic+ items in tests/poi-system/offer_generation.ts
- [x] T024 [P] [US1] Integration test: Counter Cache returns items only from boss weakness tags in tests/poi-system/offer_generation.ts
- [x] T025 [P] [US1] Integration test: determinism - same seed produces identical offers in tests/poi-system/offer_generation.ts

**Note**: Unit tests implemented in Rust (offers.rs) rather than TypeScript integration tests.

### Implementation for User Story 1

- [x] T026 [US1] Implement derive_offer_seed(base_seed, poi_index, call_count) helper in programs/poi-system/src/offer_generation.rs
- [x] T027 [US1] Implement get_poi_item_type(poi_type) -> ItemType helper (L2/L12/L13→Gear, L3→Tool) in programs/poi-system/src/offer_generation.rs
- [x] T028 [US1] Implement get_poi_item_count(poi_type) -> u8 helper (all return 3 for caches) in programs/poi-system/src/offer_generation.rs
- [x] T029 [US1] Implement generate_single_item(rng, rarity, tag, item_type, mythic_used) core function in programs/poi-system/src/offer_generation.rs
- [x] T030 [US1] Implement generate_cache_offer_items(context) -> [OfferItem; 3] in programs/poi-system/src/offer_generation.rs
- [x] T031 [US1] Implement generate_counter_cache_items(context, weakness_tags) -> [OfferItem; 3] special case in programs/poi-system/src/offer_generation.rs
- [x] T032 [US1] Add GenerateCacheOffer accounts struct with player, session, map_pois, game_state in programs/poi-system/src/lib.rs
- [x] T033 [US1] Implement generate_cache_offer instruction handler in programs/poi-system/src/lib.rs
- [x] T034 [US1] Add CacheOfferGenerated event with session, poi_index, poi_type, items in programs/poi-system/src/events.rs
- [x] T035 [US1] Emit CacheOfferGenerated event after successful offer generation in programs/poi-system/src/lib.rs

**Note**: Implemented via `interact_pick_item` instruction and `generate_*_offers` functions. CacheOfferGenerated event added.

**Checkpoint**: User Story 1 complete - can generate offers for all cache POI types

---

## Phase 4: User Story 2 - Tag-Weighted Item Selection (Priority: P1)

**Goal**: Weight tag selection toward boss weakness tags (1.4x weight)

**Independent Test**: Generate 1000+ offers and verify weakness-tagged items appear ~40% more frequently

### Tests for User Story 2

- [x] T036 [P] [US2] Unit test: compute_tag_weights returns 14000 for weakness tags, 10000 for others in programs/poi-system/src/tag_weights.rs
- [x] T037 [P] [US2] Unit test: select_weighted_tag distribution matches expected weights in programs/poi-system/src/tag_weights.rs
- [x] T038 [P] [US2] Unit test: final_prep_bias adds 1000 to Week 3 tags during Week 1-2 in programs/poi-system/src/tag_weights.rs
- [ ] T039 [P] [US2] Integration test: statistical validation of 40% bias over 1000 samples in tests/poi-system/tag_weighting.ts

### Implementation for User Story 2

- [x] T040 [US2] Define TagWeights struct (weights[8]: u16, total: u32) in programs/poi-system/src/tag_weights.rs
- [x] T041 [US2] Import ItemTag from combat-system and define tag index constants (Stone=0, Scout=1, etc.) in programs/poi-system/src/tag_weights.rs
- [ ] T042 [US2] Implement get_boss_weakness_tags(act, week, biome, stage_index) using boss-system crate in programs/poi-system/src/tag_weights.rs
- [x] T043 [US2] Implement compute_tag_weights(weakness_tags, enable_final_prep_bias, week, biome) in programs/poi-system/src/tag_weights.rs
- [x] T044 [US2] Implement select_weighted_tag(rng, weights) -> ItemTag in programs/poi-system/src/tag_weights.rs
- [x] T045 [US2] Integrate tag weighting into generate_cache_offer_items in programs/poi-system/src/offer_generation.rs
- [ ] T046 [US2] Implement get_week3_boss_weakness_tags(biome) for final prep bias in programs/poi-system/src/tag_weights.rs

**Note**: TagWeights implemented in offers.rs. Boss weakness lookup (T042) and Week 3 tags (T046) pending.

**Checkpoint**: User Story 2 complete - offers now weighted toward boss weaknesses

---

## Phase 5: User Story 3 - Act-Based Rarity Tables (Priority: P2)

**Goal**: Different rarity distributions per act and POI type

**Independent Test**: Generate offers in each act and verify rarity distributions match tables

### Tests for User Story 3

- [x] T047 [P] [US3] Unit test: get_rarity_table returns correct table for each (poi_type, act) in programs/poi-system/src/rarity_tables.rs
- [x] T048 [P] [US3] Unit test: sample_rarity distribution matches table probabilities in programs/poi-system/src/rarity_tables.rs
- [x] T049 [P] [US3] Unit test: Mythic cap enforcement - max 1 Mythic per offer in programs/poi-system/src/rarity_tables.rs
- [ ] T050 [P] [US3] Integration test: statistical rarity distribution validation (1000 samples per act/POI) in tests/poi-system/rarity_distribution.ts

### Implementation for User Story 3

- [x] T051 [US3] Define RarityTable struct (common, rare, heroic, mythic: u16) in programs/poi-system/src/rarity_tables.rs
- [x] T052 [US3] Add compile-time constants for Supply Cache tables (SUPPLY_CACHE_ACT1 through ACT4) in programs/poi-system/src/rarity_tables.rs
- [x] T053 [US3] Add compile-time constants for Tool Crate tables (TOOL_CRATE_ACT1 through ACT4) in programs/poi-system/src/rarity_tables.rs
- [x] T054 [US3] Add compile-time constants for Geode Vault tables (GEODE_VAULT_ACT1_3, GEODE_VAULT_ACT4) in programs/poi-system/src/rarity_tables.rs
- [x] T055 [US3] Implement get_rarity_table(poi_type, act, item_type) -> &RarityTable in programs/poi-system/src/rarity_tables.rs
- [x] T056 [US3] Implement sample_rarity(rng, table, mythic_used) -> Rarity with Mythic cap in programs/poi-system/src/rarity_tables.rs
- [x] T057 [US3] Integrate rarity tables into generate_cache_offer_items in programs/poi-system/src/offer_generation.rs

**Note**: All rarity tables implemented in offers.rs. Mythic cap implemented via sample_rarity_with_cap().

**Checkpoint**: User Story 3 complete - rarity distributions now vary by act

---

## Phase 6: User Story 4 - Smuggler Hatch Shop Generation (Priority: P2)

**Goal**: Generate 6-item shop (1 Tool + 5 Gear) with reroll functionality

**Independent Test**: Interact with Smuggler Hatch and verify 6 items with correct type split

### Tests for User Story 4

- [x] T058 [P] [US4] Integration test: generate_shop_inventory returns 1 Tool + 5 Gear in tests/poi-system/shop_reroll.ts
- [x] T059 [P] [US4] Integration test: shop items have correct prices per rarity in tests/poi-system/shop_reroll.ts
- [x] T060 [P] [US4] Integration test: reroll_shop costs 4 Gold first time, 6 Gold second time in tests/poi-system/shop_reroll.ts
- [ ] T061 [P] [US4] Integration test: reroll_shop fails with InsufficientGold when player cannot afford in tests/poi-system/shop_reroll.ts
- [ ] T062 [P] [US4] Integration test: shop inventory persists between views in tests/poi-system/shop_reroll.ts

**Note**: Core tests implemented in Rust unit tests. TypeScript integration tests pending.

### Implementation for User Story 4

- [x] T063 [US4] Add Smuggler Hatch rarity tables (gear and tool, ACT1-4) in programs/poi-system/src/rarity_tables.rs
- [x] T064 [US4] Define SHOP_PRICES constant (Gear: 8/14/22/34, Tool: 10/16/24) in programs/poi-system/src/offer_generation.rs
- [x] T065 [US4] Implement calculate_shop_price(item_type, rarity) -> u16 in programs/poi-system/src/offer_generation.rs
- [x] T066 [US4] Implement generate_shop_item(rng, item_type, rarity_table, weights, mythic_used) -> ShopItem in programs/poi-system/src/offer_generation.rs
- [x] T067 [US4] Implement generate_shop_inventory_items(context) -> [ShopItem; 6] in programs/poi-system/src/offer_generation.rs
- [x] T068 [US4] Add GenerateShopInventory accounts struct in programs/poi-system/src/lib.rs
- [x] T069 [US4] Implement generate_shop_inventory instruction handler in programs/poi-system/src/lib.rs
- [x] T070 [US4] Add ShopInventoryGenerated event in programs/poi-system/src/events.rs
- [x] T071 [US4] Implement calculate_reroll_cost(reroll_count) -> u16 (4 + 2 \* reroll_count) in programs/poi-system/src/offer_generation.rs
- [x] T072 [US4] Add RerollShop accounts struct in programs/poi-system/src/lib.rs
- [x] T073 [US4] Implement reroll_shop instruction handler with gold deduction in programs/poi-system/src/lib.rs
- [x] T074 [US4] Add ShopRerolled event in programs/poi-system/src/events.rs

**Note**: Implemented via `enter_shop`, `shop_purchase`, `shop_reroll`, `leave_shop` instructions.

**Checkpoint**: User Story 4 complete - Smuggler Hatch shop fully functional with reroll

---

## Phase 7: User Story 5 - Week-Aware Tag Targeting (Priority: P3) ⏸️ DEFERRED

**Goal**: Apply correct boss weakness tags based on current week

**Status**: DEFERRED for MVP - client-provided weakness tags are sufficient for PvE gameplay.

**Rationale**: In a PvE game, incorrect weakness tags only hurt the player (worse loot). No incentive to cheat. Current architecture passes weakness tags as instruction arguments, which is simpler and avoids CPI overhead.

**Future Work**: If needed for consistency or multi-player features, implement on-chain boss lookup via CPI to boss-system.

### Tests for User Story 5 (Deferred)

- [ ] T075 [P] [US5] Unit test: get_boss_weakness_tags returns Week 1 boss tags during Week 1
- [ ] T076 [P] [US5] Unit test: get_boss_weakness_tags returns Week 2 boss tags during Week 2
- [ ] T077 [P] [US5] Unit test: get_boss_weakness_tags returns Week 3 final tags during Week 3
- [ ] T078 [P] [US5] Integration test: offers in Week 2 favor Week 2 boss weaknesses

### Implementation for User Story 5 (Deferred)

- [ ] T079 [US5] Implement get_current_boss_for_context(act, week, biome, stage_index) wrapper
- [ ] T080 [US5] Update generate_cache_offer to read week from GameState account
- [ ] T081 [US5] Update generate_shop_inventory to read week from GameState account
- [ ] T082 [US5] Ensure boss lookup uses stage_index for correct boss variant selection

**Checkpoint**: Deferred - current client-provided approach works for MVP

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [x] T083 [P] Run cargo clippy on poi-system and fix all warnings in programs/poi-system/
- [x] T084 [P] Run cargo fmt on poi-system in programs/poi-system/
- [x] T085 [P] Add IDL documentation comments to all public instructions in programs/poi-system/src/lib.rs
- [x] T086 [P] Add IDL documentation to all account structs in programs/poi-system/src/state.rs
- [x] T087 Verify anchor build succeeds without errors
- [x] T088 Run full test suite: cargo test -p poi-system && anchor test
- [ ] T089 Validate quickstart.md examples work correctly

**Note**: 141 unit tests passing. Only pre-existing Anchor warnings (not errors). Quickstart validation deferred.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3-7)**: All depend on Foundational phase completion
  - US1 + US2 are both P1 and can proceed in parallel after Foundational
  - US3 + US4 are both P2 and can proceed in parallel after Foundational
  - US5 is P3 and can proceed after Foundational
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Foundational only - no story dependencies
- **User Story 2 (P1)**: Foundational only - integrates with US1 but independently testable
- **User Story 3 (P2)**: Foundational only - US1 uses basic rarity, US3 adds act variation
- **User Story 4 (P2)**: Foundational + US3 (for rarity tables) - independently testable
- **User Story 5 (P3)**: Foundational + US2 (for tag weights) - independently testable

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD per Constitution)
- Core logic before instruction handlers
- Instruction handlers before events
- Story complete before moving to next priority

### Parallel Opportunities

**Phase 1 (Setup)**: T002, T003, T004 can run in parallel
**Phase 2 (Foundational)**: T007, T008 can run in parallel; T018, T019, T020 can run in parallel
**Phase 3 (US1)**: All tests T021-T025 can run in parallel
**Phase 4 (US2)**: All tests T036-T039 can run in parallel
**Phase 5 (US3)**: All tests T047-T050 can run in parallel
**Phase 6 (US4)**: All tests T058-T062 can run in parallel
**Phase 7 (US5)**: All tests T075-T078 can run in parallel
**Phase 8 (Polish)**: T083, T084, T085, T086 can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task: "T021 [P] [US1] Integration test: generate_cache_offer for Supply Cache"
Task: "T022 [P] [US1] Integration test: generate_cache_offer for Tool Crate"
Task: "T023 [P] [US1] Integration test: generate_cache_offer for Geode Vault"
Task: "T024 [P] [US1] Integration test: Counter Cache returns weakness tags only"
Task: "T025 [P] [US1] Integration test: determinism validation"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1
4. **STOP and VALIDATE**: Test offer generation for all cache POI types
5. Deploy/demo if ready - basic offer generation works

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Basic offer generation (MVP!)
3. Add User Story 2 → Offers now favor boss weaknesses
4. Add User Story 3 → Rarity varies by act
5. Add User Story 4 → Smuggler Hatch shop works
6. Add User Story 5 → Correct week targeting
7. Each story adds strategic depth without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 + User Story 2 (both P1, related)
   - Developer B: User Story 3 + User Story 4 (both P2, related)
   - Developer C: User Story 5 (P3) or help with tests
3. Stories complete and integrate independently

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Verify tests fail before implementing (TDD)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Constitution mandates TDD - tests are included in all story phases
