# Feature Specification: POI System

**Feature Branch**: `007-poi-system`
**Created**: 2026-01-20
**Status**: Draft
**Input**: User description: "for POIs"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - POI Definition Registry (Priority: P1)

The game needs a complete registry of all 14 POI types with their properties (rarity, use type, active conditions, emoji). Each POI type defines how players interact with exploration locations.

**Why this priority**: Without POI definitions, no POI interactions can occur. This is the foundational data layer.

**Independent Test**: Can be tested by querying any POI type by ID and verifying properties match GDD specifications.

**Acceptance Scenarios**:

1. **Given** a POI ID like "L1", **When** querying the POI registry, **Then** returns Mole Den with rarity=Fixed, use=Repeatable, active=Night-only
2. **Given** any of the 14 POI IDs, **When** queried, **Then** returns complete definition including interaction type
3. **Given** an invalid POI ID, **When** queried, **Then** returns an error indicating POI not found

---

### User Story 2 - Item Offer Generation (Priority: P1)

POIs that offer items (Supply Cache, Tool Crate, Geode Vault, Smuggler Hatch, Counter Cache) must generate weighted item selections based on act, rarity tables, and boss weakness tags.

**Why this priority**: Item acquisition is core to gameplay progression and build strategy.

**Independent Test**: Given a POI type and act, can generate deterministic item offers with correct rarity distribution and tag weighting.

**Acceptance Scenarios**:

1. **Given** a Supply Cache in Act 1, **When** generating offers, **Then** produces 3 Common Gear items with tag weighting toward current boss weaknesses
2. **Given** a Geode Vault in Act 4, **When** generating offers, **Then** 90% Heroic / 10% Mythic distribution with max 1 Mythic shown
3. **Given** a Counter Cache, **When** generating offers, **Then** all 3 items are from the 2 weakness tags of the current week boss
4. **Given** a Smuggler Hatch, **When** generating offers, **Then** produces 1 Tool + 5 Gear with act-appropriate rarity weights

---

### User Story 3 - POI Interaction Execution (Priority: P1)

Players interact with POIs to perform actions (pick items, heal, upgrade, fast travel). Each POI type has specific interaction logic and state changes.

**Why this priority**: Interaction execution delivers the actual gameplay value of POIs.

**Independent Test**: Can execute each POI interaction and verify correct state changes occur.

**Acceptance Scenarios**:

1. **Given** player at Mole Den during Night, **When** interacting, **Then** time skips to Day and player HP is restored to max
2. **Given** player at Rest Alcove (one-time), **When** interacting, **Then** time skips to Day, player heals 10 HP, and POI is marked used
3. **Given** player at Tool Oil Rack with tool not yet modified, **When** selecting +ATK, **Then** tool gains +1 ATK and oil type is marked applied
4. **Given** player at Rusty Anvil with Tier I tool and 8 Gold, **When** upgrading, **Then** tool becomes Tier II and 8 Gold is deducted

---

### User Story 4 - Shop and Pricing (Priority: P2)

The Smuggler Hatch provides a shop interface with gold-based purchases and a reroll mechanic. Items have fixed prices by rarity.

**Why this priority**: Shopping adds economic depth but basic item acquisition works without it.

**Independent Test**: Can browse shop, purchase items, and reroll with correct gold costs.

**Acceptance Scenarios**:

1. **Given** Smuggler Hatch with 6 items displayed, **When** purchasing Common Gear, **Then** deducts 8 Gold and adds item to inventory
2. **Given** player with 4 Gold at Smuggler Hatch, **When** rerolling first time, **Then** deducts 4 Gold and generates new 6-item selection
3. **Given** player has rerolled twice already, **When** rerolling third time, **Then** costs 8 Gold (4 + 2 + 2)
4. **Given** player selects Heroic Tool, **When** purchasing, **Then** deducts 24 Gold

---

### User Story 5 - Item Upgrade POIs (Priority: P2)

Rusty Anvil and Rune Kiln allow players to upgrade item tiers. Rusty Anvil costs Gold for tools; Rune Kiln fuses identical items for free.

**Why this priority**: Upgrade mechanics are important for late-game builds but early game works without them.

**Independent Test**: Can upgrade items at each POI with correct costs and results.

**Acceptance Scenarios**:

1. **Given** player at Rusty Anvil with Tier I tool, **When** upgrading, **Then** costs 8 Gold and tool becomes Tier II
2. **Given** player at Rusty Anvil with Tier II tool, **When** upgrading, **Then** costs 16 Gold and tool becomes Tier III
3. **Given** player at Rune Kiln with two identical Tier I gear, **When** fusing, **Then** produces one Tier II gear at no Gold cost
4. **Given** player at Rune Kiln with non-matching items, **When** attempting fusion, **Then** error indicates items must match

---

### User Story 6 - Fast Travel (Priority: P2)

Rail Waypoints allow fast travel between discovered waypoints, providing map mobility.

**Why this priority**: Fast travel is convenience feature; exploration works without it.

**Independent Test**: Can discover waypoints and travel between them.

**Acceptance Scenarios**:

1. **Given** player discovers first Rail Waypoint, **When** interacting, **Then** waypoint is marked discovered
2. **Given** two discovered waypoints, **When** selecting destination at one, **Then** player teleports to the other
3. **Given** only one discovered waypoint, **When** attempting travel, **Then** message indicates no destinations available

---

### User Story 7 - Map Reveal POIs (Priority: P3)

Survey Beacon and Seismic Scanner reveal map information. Survey Beacon reveals tiles in radius; Seismic Scanner reveals nearest POI of chosen category.

**Why this priority**: Map reveal enhances exploration but is not required for core gameplay.

**Independent Test**: Can use each reveal POI and verify correct tiles/POIs are revealed.

**Acceptance Scenarios**:

1. **Given** player at Survey Beacon, **When** activating, **Then** all tiles within radius 13 are revealed
2. **Given** player at Seismic Scanner, **When** selecting "Shop" category, **Then** nearest Smuggler Hatch location is revealed
3. **Given** Survey Beacon already used, **When** attempting reuse, **Then** error indicates POI is depleted

---

### User Story 8 - POI Spawn Configuration (Priority: P3)

The system must support configurable POI spawn counts and guarantees per act, including baseline spawns and guaranteed placements.

**Why this priority**: Spawn configuration is needed for map generation but POI interactions work independently.

**Independent Test**: Can retrieve spawn configuration for any act and verify counts match GDD.

**Acceptance Scenarios**:

1. **Given** Act 1 configuration, **When** querying baseline spawns, **Then** returns L2 x10, L3 x2, L4 x2, L6 x1, L10 x1
2. **Given** Act 1 configuration, **When** querying guarantees, **Then** includes L1 adjacent, L8 x2, L9 x1, L11 x1, L12 x1
3. **Given** Act 4 configuration, **When** querying Counter Cache guarantee, **Then** L13 (Week 3) has 20% chance

---

### Edge Cases

- What happens when player doesn't have enough Gold for purchase/upgrade? Transaction fails with insufficient funds error.
- What happens when player's inventory is full at item POI? Cannot pick item; must discard or skip.
- What happens when player tries to use Night-only POI during Day? Interaction blocked with time restriction message.
- What happens when player tries to apply Tool Oil already applied to current tool? Error indicates modification already applied.
- How does Scrap Chute work with empty inventory? Cannot use; requires at least one Gear item.

## Requirements *(mandatory)*

### Functional Requirements

**POI Registry**
- **FR-001**: System MUST store 14 POI type definitions as compile-time constants
- **FR-002**: Each POI definition MUST include: ID (L1-L14), name, emoji, rarity, use type (one-time/repeatable), active condition (anytime/night-only), and interaction type
- **FR-003**: System MUST provide lookup function to retrieve POI type by ID
- **FR-004**: System MUST provide function to list POIs by rarity category

**Item Offer Generation**
- **FR-005**: Supply Cache (L2) MUST generate 3 Gear items with act-specific rarity table (Act 1: 100% Common, Act 4: 65% Common / 35% Rare)
- **FR-006**: Tool Crate (L3) MUST generate 3 Tool items with act-specific rarity table including Heroic at Act 2+
- **FR-007**: Geode Vault (L12) MUST generate 3 Heroic items (Act 1-3) or 90% Heroic / 10% Mythic (Act 4, max 1 Mythic)
- **FR-008**: Smuggler Hatch (L9) MUST generate 1 Tool + 5 Gear with separate rarity tables per item type
- **FR-009**: Counter Cache (L13) MUST generate 3 items exclusively from current week boss's 2 weakness tags
- **FR-010**: All item generation MUST apply tag weighting (1.4x for boss weakness tags)
- **FR-011**: Item generation MUST be deterministic given a seed value

**POI Interactions**
- **FR-012**: Mole Den (L1) MUST skip to Day and restore all HP; repeatable; night-only
- **FR-013**: Rest Alcove (L5) MUST skip to Day and heal 10 HP; one-time; night-only
- **FR-014**: Tool Oil Rack (L4) MUST apply +1 ATK, +1 SPD, or +1 DIG to current tool; each modification once per tool
- **FR-015**: Survey Beacon (L6) MUST reveal all tiles within radius 13; one-time
- **FR-016**: Seismic Scanner (L7) MUST reveal nearest POI of selected category; one-time
- **FR-017**: Rail Waypoint (L8) MUST enable fast travel to other discovered waypoints; repeatable
- **FR-018**: Scrap Chute (L14) MUST destroy 1 Gear item; costs Gold by act (8/8/10/12); one-time

**Shop Mechanics**
- **FR-019**: Smuggler Hatch prices MUST be: Common Gear 8, Rare Gear 14, Heroic Gear 22, Mythic Gear 34
- **FR-020**: Smuggler Hatch Tool prices MUST be: Common 10, Rare 16, Heroic 24
- **FR-021**: Reroll cost MUST start at 4 Gold and increase by 2 each reroll (4, 6, 8, 10...)
- **FR-022**: Shop state (items, reroll count) MUST persist until player leaves POI

**Upgrade Mechanics**
- **FR-023**: Rusty Anvil (L10) MUST upgrade Tool tier; I→II costs 8 Gold, II→III costs 16 Gold; one-time
- **FR-024**: Rune Kiln (L11) MUST fuse 2 identical items into 1 higher tier; no Gold cost; repeatable

**POI State**
- **FR-025**: One-time POIs MUST track used/unused state per map instance
- **FR-026**: Repeatable POIs MUST allow multiple interactions
- **FR-027**: Night-only POIs MUST reject interaction during Day phase

**Spawn Configuration**
- **FR-028**: System MUST store baseline spawn counts per act (e.g., Act 1: L2 x10, L3 x2)
- **FR-029**: System MUST store guaranteed spawns per act (e.g., L1 adjacent, L8 x2)
- **FR-030**: Counter Cache (L13) guarantees MUST vary by act: 100% W1, 100% W2, 50%/100%/30%/20% W3 by act

### Key Entities

- **PoiDefinition**: Static POI type data including ID, name, emoji, rarity, use type, active condition, interaction type
- **PoiInstance**: Runtime POI on the map with position, type reference, used flag, and interaction-specific state
- **ItemOffer**: Generated item selection at a POI (item ID, tier, price if applicable)
- **ShopState**: Smuggler Hatch state including current offers and reroll count
- **SpawnConfig**: Per-act configuration for POI spawn counts and guarantees

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: All 14 POI types can be queried and return correct properties
- **SC-002**: Item generation produces correct rarity distribution (within 5% of target over 1000 samples)
- **SC-003**: Tag weighting increases weakness-tagged item selection by approximately 40%
- **SC-004**: All POI interactions complete and update state correctly
- **SC-005**: Shop purchases deduct correct Gold amounts
- **SC-006**: Upgrade mechanics produce correct tier results at correct costs
- **SC-007**: Fast travel correctly teleports between discovered waypoints
- **SC-008**: Spawn configuration returns correct counts for all 4 acts

## Assumptions

- Item system (004) is implemented and provides item definitions, tags, and inventory management
- Boss system (006) is implemented and provides weakness tags for current week boss
- GameState tracks player Gold, HP, current time (Day/Night), and position
- Map system will consume spawn configuration for POI placement
- Deterministic seed for item generation comes from game state (session + action count)
