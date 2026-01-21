# Feature Specification: Item Offer Generation

**Feature Branch**: `008-item-offer-generation`
**Created**: 2026-01-20
**Status**: Draft
**Input**: User description: "Item Offer Generation"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Receive Item Offers at POIs (Priority: P1)

When a player interacts with a POI that offers items (Supply Cache, Tool Crate, Geode Vault, Counter Cache, or Smuggler Hatch), the system generates a set of item options for the player to choose from. Each offer respects the POI type's rarity constraints and is weighted toward tags that counter the current week's boss.

**Why this priority**: This is the core functionality - without item offer generation, item-granting POIs cannot function, blocking player progression and build crafting.

**Independent Test**: Can be fully tested by triggering any item-granting POI and verifying that the correct number of items appear with appropriate rarities and tag distributions.

**Acceptance Scenarios**:

1. **Given** a player in Act 1, Week 1 interacting with a Supply Cache, **When** the offer is generated, **Then** 3 Common Gear items are presented with increased weight for the Week 1 boss's weakness tags.
2. **Given** a player in Act 4 interacting with a Geode Vault, **When** the offer is generated, **Then** 3 items are presented where each has 90% chance of being Heroic and 10% chance of being Mythic (max 1 Mythic shown).
3. **Given** a player in Week 2 interacting with a Counter Cache, **When** the offer is generated, **Then** 3 items are presented drawn exclusively from the 2 weakness tags of the Week 2 boss.

---

### User Story 2 - Tag-Weighted Item Selection (Priority: P1)

The system weights item generation toward tags that counter the current week's boss. Base weight for each tag is 1.0, while boss weakness tags receive 1.4 weight. This makes counter-builds more likely without guaranteeing them.

**Why this priority**: Tag weighting is essential for the game's strategic depth - players need reasonable odds of finding boss counters to make informed build decisions.

**Independent Test**: Can be verified by generating a large sample of item offers and confirming that weakness-tagged items appear approximately 40% more frequently than baseline tags.

**Acceptance Scenarios**:

1. **Given** Week 1 boss has STONE and FROST weakness tags, **When** generating item offers during Week 1, **Then** STONE and FROST items each have 1.4x base weight while other tags have 1.0x weight.
2. **Given** a tag weighting system, **When** sampling items, **Then** the system normalizes weights and randomly selects a tag before selecting an item of the requested rarity from that tag.
3. **Given** the "final prep bias" option is enabled, **When** generating offers in Week 1 or Week 2, **Then** Week 3 final boss weakness tags receive an additional +0.1 weight bonus.

---

### User Story 3 - Act-Based Rarity Tables (Priority: P2)

Different acts have different rarity distributions for item offers. As players progress through acts, higher rarity items become more common in offers.

**Why this priority**: Rarity progression creates meaningful power curves across the campaign - essential for difficulty scaling but builds on core offer generation.

**Independent Test**: Can be verified by generating offers in each act and confirming rarity distributions match the defined tables.

**Acceptance Scenarios**:

1. **Given** a Supply Cache in Act 1, **When** generating 3 Gear options, **Then** each item is 100% Common.
2. **Given** a Supply Cache in Act 4, **When** generating 3 Gear options, **Then** each item has 65% Common / 35% Rare distribution.
3. **Given** a Tool Crate in Act 3, **When** generating 3 Tool options, **Then** each item has 60% Common / 30% Rare / 10% Heroic distribution.

---

### User Story 4 - Smuggler Hatch Shop Generation (Priority: P2)

The Smuggler Hatch POI generates a shop with 6 items (1 Tool + 5 Gear) with act-appropriate rarity weights. Players can reroll the shop for escalating Gold costs.

**Why this priority**: Shops provide player agency in build-crafting and Gold economy, but depend on core offer generation being functional.

**Independent Test**: Can be verified by interacting with a Smuggler Hatch and confirming 6 items appear with correct type split and rarity distributions per act.

**Acceptance Scenarios**:

1. **Given** a Smuggler Hatch in Act 2, **When** generating shop inventory, **Then** 1 Tool (65% Common / 30% Rare / 5% Heroic) and 5 Gear (55% Common / 38% Rare / 7% Heroic) are presented.
2. **Given** a player at a Smuggler Hatch, **When** they request a reroll, **Then** the first reroll costs 4 Gold, and each subsequent reroll increases by 2 Gold (6, 8, 10...).
3. **Given** Act 4 Smuggler Hatch, **When** generating Gear items, **Then** there is a 2% chance for Mythic rarity items to appear.

---

### User Story 5 - Week-Aware Tag Targeting (Priority: P3)

The system knows which week the player is in and applies the appropriate boss weakness tags for weighting. During Week 1, Week 1 boss weaknesses are used; during Week 2, Week 2 boss weaknesses are used; during Week 3, Week 3 final weaknesses are used.

**Why this priority**: Correct week targeting ensures strategic coherence - players find counters for upcoming bosses, not past ones.

**Independent Test**: Can be verified by generating offers in each week and confirming the correct boss weakness tags are applied.

**Acceptance Scenarios**:

1. **Given** a player in Week 1 of a Biome A stage, **When** generating offers, **Then** the Week 1 Biome A boss weakness tags are used for weighting.
2. **Given** a player transitioning from Week 1 to Week 2 (after defeating Week 1 boss), **When** generating offers, **Then** the system switches to Week 2 boss weakness tags.
3. **Given** a player in Week 3, **When** generating offers, **Then** the Week 3 final boss weakness tags are used exclusively.

---

### Edge Cases

- What happens when no items exist for a selected tag + rarity combination? The system falls back to any item of that rarity.
- What happens when a player has no Gold for shop rerolls? The reroll option is disabled but item display remains.
- What happens when generating Mythic items but the pool is exhausted? The system substitutes Heroic items.
- How does the system handle Counter Cache when boss weakness tags overlap with each other? Items from both tags are pooled together for selection.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST generate item offers based on POI type, with correct item counts (3 for caches/vaults, 6 for shops).
- **FR-002**: System MUST apply rarity tables based on current act (1-4) and POI type.
- **FR-003**: System MUST weight tag selection using base weight 1.0 for standard tags and 1.4 for current boss weakness tags.
- **FR-004**: System MUST normalize tag weights before random selection.
- **FR-005**: System MUST select items from the chosen tag that match the requested rarity.
- **FR-006**: System MUST track current week (1, 2, or 3) to determine which boss weakness tags to apply.
- **FR-007**: System MUST support the "final prep bias" option that adds +0.1 weight to Week 3 final weakness tags during Weeks 1-2.
- **FR-008**: System MUST generate Smuggler Hatch inventory with exactly 1 Tool and 5 Gear items.
- **FR-009**: System MUST track reroll count per Smuggler Hatch visit and calculate escalating Gold cost (4, 6, 8, 10...).
- **FR-010**: System MUST enforce Mythic rarity cap of 1 per offer where Mythic items are possible.
- **FR-011**: Counter Cache MUST draw items exclusively from the 2 weakness tags of the current week's boss.
- **FR-012**: System MUST use deterministic random generation seeded appropriately for verifiable on-chain results.
- **FR-013**: System MUST fall back to any item of the requested rarity when no items exist for the selected tag.

### Key Entities

- **ItemOffer**: A generated set of items presented to the player at a POI, containing item IDs, rarities, and prices (if applicable).
- **RarityTable**: Configuration defining probability distributions for item rarities by POI type and act.
- **TagWeight**: The calculated weight for each of the 8 item tags based on boss weaknesses and current week.
- **ShopInventory**: Extended ItemOffer for Smuggler Hatch including Tool/Gear split and reroll tracking.
- **BossWeakness**: Reference to the 2 weakness tags defined for each boss (from boss system).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Item offers generate correctly for all 5 item-granting POI types (Supply Cache, Tool Crate, Geode Vault, Counter Cache, Smuggler Hatch).
- **SC-002**: Boss weakness-tagged items appear approximately 40% more frequently than non-weakness tags when measured across 1000+ generated offers.
- **SC-003**: Rarity distributions match defined tables within 5% variance across 1000+ samples per act/POI combination.
- **SC-004**: Counter Cache offers contain only items from the current boss's 2 weakness tags (100% compliance).
- **SC-005**: Smuggler Hatch reroll costs follow the correct escalating formula (4 + 2*(rerollCount-1)).
- **SC-006**: No Mythic items appear where not allowed; maximum 1 Mythic per offer where allowed.
- **SC-007**: Item generation is deterministic - same seed produces identical offers.

## Assumptions

- The item system (004-item-system) provides a complete registry of all 80 items with their IDs, tags, types (Tool/Gear), and rarities.
- The boss system (006-boss-system) provides weakness tag data for all bosses accessible by week and biome.
- The POI system (007-poi-system) provides the interaction context including POI type and current game state (act, week, biome).
- Random number generation uses the session seed or a derived seed for deterministic results.
- The "final prep bias" feature is enabled by default but may be configurable.
