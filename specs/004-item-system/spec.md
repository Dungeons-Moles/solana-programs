# Feature Specification: Item System

**Feature Branch**: `004-item-system`
**Created**: 2026-01-19
**Status**: Draft
**Input**: User description: "the item system"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Item Data Registry (Priority: P1)

The system stores definitions for all 80 items (10 per tag across 8 tags), including their names, types (Tool/Gear), tags, rarities, and combat effects. Game systems can look up any item by its ID to retrieve its properties.

**Why this priority**: Foundation for all item functionality. Without item definitions, no other item feature can work.

**Independent Test**: Can be fully tested by querying item definitions and verifying all 80 items return correct properties (name, tag, rarity, effects).

**Acceptance Scenarios**:

1. **Given** the item registry is initialized, **When** querying item ID "T-ST-01" (Bulwark Shovel), **Then** the system returns: type=Tool, tag=STONE, rarity=Common, ATK bonus=1/2/3, ARM bonus=4/6/8
2. **Given** the item registry, **When** querying all items with tag FROST, **Then** exactly 10 items are returned
3. **Given** the item registry, **When** querying item with invalid ID "X-XX-99", **Then** the system returns an error indicating item not found

---

### User Story 2 - Player Inventory Management (Priority: P2)

Players have an inventory with a Tool slot (exactly 1 equipped tool) and Gear slots (4 initially, expanding to 6 after Week 1 boss, 8 after Week 2 boss). Players can view their inventory, equip items, and see which items are currently equipped.

**Why this priority**: Players need to hold and manage items before they can use them in combat or upgrade them.

**Independent Test**: Can be tested by giving a player items, having them equip a tool and gear, and verifying the inventory state is correct.

**Acceptance Scenarios**:

1. **Given** a new player with empty inventory and 4 gear slots, **When** they receive item "T-SC-01" (Twin Picks), **Then** it is equipped in the Tool slot
2. **Given** a player with 4 gear slots and 4 equipped gear items, **When** they try to equip a 5th gear item, **Then** the system rejects with "inventory full" error
3. **Given** a player who defeats the Week 1 boss, **When** the boss is defeated, **Then** their gear slots increase from 4 to 6
4. **Given** a player with a Tool equipped, **When** they equip a different Tool, **Then** the old Tool is replaced (only 1 Tool at a time)

---

### User Story 3 - Item Tier Upgrades (Priority: P3)

Players can upgrade items from Tier I to Tier II to Tier III by fusing two identical items at a Rune Kiln POI. Higher tiers have the same effects but with increased numeric values.

**Why this priority**: Upgrades add progression depth and give players meaningful choices about which items to invest in.

**Independent Test**: Can be tested by giving a player two identical Tier I items, fusing them, and verifying the result is one Tier II item with doubled effect values.

**Acceptance Scenarios**:

1. **Given** a player with two "G-ST-01" (Miner Helmet) Tier I items, **When** they fuse at a Rune Kiln, **Then** they receive one "G-ST-01" Tier II with +6 ARM (instead of +3)
2. **Given** a player with two "G-ST-01" Tier II items, **When** they fuse, **Then** they receive one "G-ST-01" Tier III with +9 ARM
3. **Given** a player with one "G-ST-01" Tier I and one "G-ST-02" Tier I (different items), **When** they attempt to fuse, **Then** the system rejects with "items must match" error
4. **Given** a player with two "G-ST-01" Tier III items, **When** they attempt to fuse, **Then** the system rejects with "already max tier" error

---

### User Story 4 - Itemset Bonuses (Priority: P4)

When a player has all required items of an itemset equipped, they receive a bonus effect. There are 12 itemsets, each requiring 3-4 specific items.

**Why this priority**: Itemsets create build diversity and strategic depth. They encourage players to collect specific combinations.

**Independent Test**: Can be tested by equipping the required items for "Union Standard" set and verifying the Battle Start bonus (+4 Armor, +1 DIG) activates.

**Acceptance Scenarios**:

1. **Given** a player with G-ST-01, G-ST-02, and G-SC-01 equipped (Union Standard set), **When** combat begins, **Then** they gain +4 Armor and +1 DIG as a Battle Start effect
2. **Given** a player with only G-ST-01 and G-ST-02 (missing G-SC-01), **When** combat begins, **Then** no set bonus activates
3. **Given** a player with all 4 shard items (Shard Circuit set), **When** they have the set active, **Then** shard effects trigger every turn instead of every other turn

---

### User Story 5 - Item Offer Generation (Priority: P5)

When a player visits a POI that offers items (Supply Cache, Tool Crate, Geode Vault, Smuggler Hatch), the system generates item offers weighted by: the POI's rarity table for the current act, and the current week's boss weakness tags (40% boost to weakness tags).

**Why this priority**: Loot shaping makes runs feel less random and helps players find counters to upcoming bosses.

**Independent Test**: Can be tested by simulating 1000 Supply Cache offers during Week 1 and verifying weakness-tagged items appear ~40% more often than baseline.

**Acceptance Scenarios**:

1. **Given** a player at a Supply Cache in Act 1, **When** offers are generated, **Then** all 3 items are Common Gear (Act 1 table: 100% Common)
2. **Given** a player in Week 1 facing a boss with STONE+FROST weaknesses, **When** generating offers, **Then** STONE and FROST tagged items each have 1.4x base weight
3. **Given** a player at a Geode Vault in Act 4, **When** offers are generated, **Then** there is a 10% chance one item is Mythic (max 1 Mythic shown)
4. **Given** a player at a Counter Cache, **When** offers are generated, **Then** all items are drawn only from the current boss's 2 weakness tags

---

### User Story 6 - Combat Effect Resolution (Priority: P6)

When combat begins, the system converts equipped items into combat effects (ItemEffect arrays) that the combat system can process. This includes Tool ATK/strikes, Gear stat bonuses, and triggered effects.

**Why this priority**: This is the integration point between items and combat. Without it, items have no gameplay effect.

**Independent Test**: Can be tested by equipping a player with items, starting combat, and verifying the combat system receives correct ItemEffect arrays.

**Acceptance Scenarios**:

1. **Given** a player with "T-FR-01" (Rime Pike) equipped, **When** combat effects are generated, **Then** the output includes: +2 ATK stat bonus, OnHit trigger applying 1 Chill
2. **Given** a player with "G-ST-03" (Spiked Bracers) equipped, **When** combat effects are generated, **Then** the output includes: BattleStart trigger granting 2 Shrapnel
3. **Given** a player with an active itemset bonus, **When** combat effects are generated, **Then** the set bonus effect is included in the ItemEffect array
4. **Given** a Tier II item with "1/2/3" values, **When** combat effects are generated, **Then** the middle value (2) is used for all numeric effects

---

### Edge Cases

- What happens when a player's inventory is full and they're forced to receive an item? (Item is discarded or player must choose one to discard)
- What happens if a player unequips an item that's part of an active set? (Set bonus deactivates immediately)
- How are "once per tool" modifiers from Tool Oil Rack tracked? (Flag stored on the item instance)
- What happens if an item references a stat the combatant doesn't have? (Effect is skipped gracefully)
- How are itemset requirements checked if an item has multiple tags? (Each item has exactly one tag)

## Requirements *(mandatory)*

### Functional Requirements

#### Item Registry

- **FR-001**: System MUST store definitions for all 80 items with: ID, name, type (Tool/Gear), tag, rarity, and tier-scaled effect values
- **FR-002**: System MUST support 8 item tags: STONE, SCOUT, GREED, BLAST, FROST, RUST, BLOOD, TEMPO
- **FR-003**: System MUST support 4 rarities: Common, Rare, Heroic, Mythic
- **FR-004**: System MUST support 3 tiers (I, II, III) with numeric scaling per tier
- **FR-005**: System MUST allow querying items by ID, tag, type, and rarity

#### Inventory

- **FR-006**: System MUST enforce exactly 1 Tool slot per player
- **FR-007**: System MUST enforce gear slot limits: 4 initially, 6 after Week 1 boss, 8 after Week 2 boss (max 8)
- **FR-008**: System MUST track each item instance's tier (I, II, or III)
- **FR-009**: System MUST prevent equipping more gear than available slots

#### Upgrades

- **FR-010**: System MUST allow fusing two identical items (same ID + same tier) to produce one item at next tier
- **FR-011**: System MUST reject fusing items that don't match (different IDs or different tiers)
- **FR-012**: System MUST reject fusing Tier III items (already max)
- **FR-013**: System MUST scale effect values by tier: Tier I = first value, Tier II = second value, Tier III = third value

#### Itemsets

- **FR-014**: System MUST store definitions for all 12 itemsets with: required item IDs and bonus effect
- **FR-015**: System MUST detect when all required items of a set are equipped
- **FR-016**: System MUST include set bonus effects in combat when set is active
- **FR-017**: System MUST deactivate set bonus immediately when a required item is unequipped

#### Offer Generation

- **FR-018**: System MUST generate item offers based on POI type and current act's rarity table
- **FR-019**: System MUST weight item tags by current week's boss weaknesses (1.4x for weakness tags)
- **FR-020**: System MUST support Counter Cache offers that draw only from boss weakness tags
- **FR-021**: System MUST respect Mythic item caps (max 1 Mythic per offer in eligible POIs)

#### Combat Integration

- **FR-022**: System MUST convert equipped items to ItemEffect arrays for combat system
- **FR-023**: System MUST include stat bonuses (ATK, ARM, SPD, DIG, HP) from items
- **FR-024**: System MUST include triggered effects with correct trigger types
- **FR-025**: System MUST include strike count from Tool
- **FR-026**: System MUST include active itemset bonuses in combat effects

### Key Entities

- **ItemDefinition**: Static data for an item type. Has ID, name, type (Tool/Gear), tag, rarity, base effect values (as I/II/III triplet).
- **ItemInstance**: A specific item owned by a player. References ItemDefinition ID, has current tier (I, II, III), and optional modification flags (e.g., Tool Oil applied).
- **PlayerInventory**: Player's equipped items. Has Tool slot (0-1 ItemInstance), Gear slots (0-8 ItemInstance), and current slot capacity.
- **ItemsetDefinition**: Static data for a set bonus. Has ID, name, required item IDs (3-4), and bonus ItemEffect.
- **ItemOffer**: A generated offer at a POI. Has item definition ID, tier (always I for fresh items), and price (if applicable).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 80 item definitions are queryable and return correct properties
- **SC-002**: All 12 itemset bonuses activate correctly when requirements are met
- **SC-003**: Item tier upgrades correctly scale all numeric effect values
- **SC-004**: Inventory slot limits are enforced correctly at each game phase
- **SC-005**: Item offers at POIs show statistically significant bias toward boss weakness tags (within 5% of expected 1.4x weight over 1000 samples)
- **SC-006**: Combat receives correct ItemEffect arrays that produce expected combat outcomes

## Assumptions

- Item definitions are static data that does not change during gameplay
- Items are not tradeable between players (single-player progression)
- Item prices at shops follow the GDD pricing table (Common Gear 8g, Rare Gear 14g, etc.)
- The Rune Kiln POI fusion is the only way to upgrade item tiers
- Tool Oil Rack modifications (+1 ATK/SPD/DIG) are tracked as flags on item instances
- Boss weakness tags are determined by the current week's scheduled boss

## Dependencies

- Combat System (provides ItemEffect processing)
- Gameplay State Tracking (provides player stats, week/phase, boss schedule)
- Player Profile (provides base stats that items modify)

## Out of Scope

- POI interaction logic (handled by POI feature)
- Shop UI/UX and gold transactions (handled by POI feature)
- Boss definitions and weakness assignments (handled by Boss feature)
- Item drop rates from enemies (enemies drop gold only per GDD)
