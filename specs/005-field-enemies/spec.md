# Feature Specification: Field Enemies

**Feature Branch**: `005-field-enemies`  
**Created**: 2026-01-19  
**Status**: Draft  
**Input**: User description: "field enemies according to @specs/gdd.md"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Player Encounters Field Enemies (Priority: P1)

Players exploring the dungeon map encounter field enemies placed on tiles. When stepping onto an enemy tile, auto-combat initiates and resolves based on the combat system rules.

**Why this priority**: Field enemies are the primary source of Gold income and the main obstacle during exploration. Without enemies, the core gameplay loop cannot function.

**Independent Test**: Can be fully tested by spawning a single enemy type on a map, having the player step onto it, and verifying combat resolves correctly with Gold awarded.

**Acceptance Scenarios**:

1. **Given** a player on the map with a Tunnel Rat (T1) on an adjacent tile, **When** the player moves onto that tile, **Then** auto-combat initiates and the enemy's stats (5 HP, 1 ATK, 0 ARM, 3 SPD, 1 DIG) and trait ("On Hit: steal 1 Gold") are applied correctly.
2. **Given** combat ends with player victory, **When** the battle concludes, **Then** the player receives Gold based on enemy tier (T1=2, T2=4, T3=6).
3. **Given** combat ends with player defeat, **When** the player's HP reaches 0, **Then** the run ends.

---

### User Story 2 - Enemy Tier Distribution Per Act (Priority: P1)

The game spawns enemies according to tier distribution rules that vary by act, ensuring difficulty progression throughout the campaign.

**Why this priority**: Tier distribution directly controls difficulty scaling. Without it, early acts could be too hard or late acts too easy.

**Independent Test**: Can be tested by generating multiple maps for each act and verifying the tier ratio matches the expected distribution within acceptable variance.

**Acceptance Scenarios**:

1. **Given** the player starts a run in Act 1, **When** enemies are spawned, **Then** approximately 70% are T1, 25% are T2, and 5% are T3.
2. **Given** the player starts a run in Act 4, **When** enemies are spawned, **Then** approximately 35% are T1, 45% are T2, and 20% are T3.
3. **Given** any act, **When** enemy spawn is calculated, **Then** the total enemy count matches act targets (Act 1: 36, Act 2: 40, Act 3: 44, Act 4: 48).

---

### User Story 3 - Enemy Trait Execution During Combat (Priority: P1)

Each enemy archetype has a unique trait that triggers according to combat system rules (Battle Start, On Hit, Turn Start, etc.).

**Why this priority**: Traits create tactical variety and make each enemy type feel distinct. They are core to the combat system's depth.

**Independent Test**: Can be tested by fighting each enemy type and verifying their trait triggers at the correct time with correct effects.

**Acceptance Scenarios**:

1. **Given** a battle with a Shard Beetle, **When** battle starts, **Then** the beetle gains 6 Shrapnel.
2. **Given** a battle with a Blood Mosquito and the mosquito hits the player, **When** the hit resolves, **Then** 1 Bleed is applied to the player (once per turn).
3. **Given** a battle with a Powder Tick, **When** 2 turns pass (Countdown reaches 0), **Then** 6 non-weapon damage is dealt to both the player and the Powder Tick.

---

### User Story 4 - Biome-Weighted Enemy Selection (Priority: P2)

Enemy archetypes are weighted based on the current biome, making certain enemies more common in specific acts.

**Why this priority**: Biome weighting creates thematic consistency and allows players to prepare for common enemies in each act.

**Independent Test**: Can be tested by generating multiple maps for each biome and verifying the expected enemy types appear more frequently.

**Acceptance Scenarios**:

1. **Given** a run in Biome A (Acts 1 & 3), **When** enemies are spawned, **Then** Tunnel Rat, Collapsed Miner, Shard Beetle, and Coin Slug appear more frequently than other archetypes.
2. **Given** a run in Biome B (Acts 2 & 4), **When** enemies are spawned, **Then** Rust Mite Swarm, Frost Wisp, Blood Mosquito, Burrow Ambusher, and Powder Tick appear more frequently.
3. **Given** any biome, **When** enemies are spawned, **Then** all 12 enemy archetypes can still appear (weighting, not exclusion).

---

### User Story 5 - Enemy Stat Scaling by Tier (Priority: P2)

Higher tier enemies have proportionally better stats (HP/ATK/ARM/SPD/DIG) following the defined stat tables.

**Why this priority**: Stat scaling ensures higher tier enemies are appropriately challenging and rewarding.

**Independent Test**: Can be tested by spawning each enemy at each tier and verifying stats match the GDD tables.

**Acceptance Scenarios**:

1. **Given** a Tunnel Warden at T1, **When** combat begins, **Then** its stats are 8 HP, 2 ATK, 4 ARM, 2 SPD, 2 DIG.
2. **Given** a Tunnel Warden at T3, **When** combat begins, **Then** its stats are 14 HP, 4 ATK, 8 ARM, 4 SPD, 3 DIG.
3. **Given** any enemy archetype and tier, **When** stats are loaded, **Then** they exactly match the GDD specification.

---

### Edge Cases

- What happens when player has 0 Gold and Tunnel Rat trait triggers ("steal 1 Gold")? Player Gold remains at 0.
- How does Countdown work for Powder Tick if combat ends before detonation? The bomb does not trigger; countdown resets for next battle.
- What happens when Collapsed Miner becomes Wounded on Turn 1? It gains +3 ATK immediately when HP drops below 50%.
- What if Coin Slug trait calculates armor from Gold during battle that changes mid-fight? Armor is calculated at Battle Start only, using initial Gold value.

## Requirements _(mandatory)_

### Functional Requirements

- **FR-001**: System MUST support 12 distinct enemy archetypes: Tunnel Rat, Cave Bat, Spore Slime, Rust Mite Swarm, Collapsed Miner, Shard Beetle, Tunnel Warden, Burrow Ambusher, Frost Wisp, Powder Tick, Coin Slug, Blood Mosquito.
- **FR-002**: System MUST support 3 tiers (T1, T2, T3) for each enemy archetype with distinct stat values (HP/ATK/ARM/SPD/DIG).
- **FR-003**: System MUST apply correct Gold rewards upon enemy defeat: T1=2 Gold, T2=4 Gold, T3=6 Gold.
- **FR-004**: System MUST spawn enemies according to tier distribution by act:
  - Act 1: T1 70% / T2 25% / T3 5%
  - Act 2: T1 55% / T2 35% / T3 10%
  - Act 3: T1 45% / T2 40% / T3 15%
  - Act 4: T1 35% / T2 45% / T3 20%
- **FR-005**: System MUST spawn target enemy counts per act: Act 1=36, Act 2=40, Act 3=44, Act 4=48.
- **FR-006**: System MUST weight enemy archetype selection by biome:
  - Biome A (Acts 1, 3): emphasize Tunnel Rat, Collapsed Miner, Shard Beetle, Coin Slug
  - Biome B (Acts 2, 4): emphasize Rust Mite Swarm, Frost Wisp, Blood Mosquito, Burrow Ambusher, Powder Tick
- **FR-007**: System MUST execute each enemy's unique trait at the correct trigger time (Battle Start, On Hit, Turn Start, Every Other Turn, Wounded, Countdown).
- **FR-008**: System MUST apply enemy trait effects according to combat system rules (status effects, damage types, once-per-turn limits).
- **FR-009**: System MUST store enemy definitions in data tables keyed by archetype ID and tier.
- **FR-010**: System MUST use deterministic seeded randomness for enemy placement and tier selection to ensure reproducible runs.

### Key Entities

- **FieldEnemy**: Represents an enemy instance with archetype, tier, current stats (HP/ATK/ARM/SPD/DIG), and trait definition. Placed on map tiles during generation.
- **EnemyArchetype**: Template defining base stats per tier, visual identifier (emoji), trait behavior, and biome affinity weights.
- **EnemyTier**: Enum (T1, T2, T3) determining stat scaling and Gold reward.
- **EnemyTrait**: Behavior definition specifying trigger condition, effect, and any limitations (e.g., once-per-turn).

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: All 12 enemy archetypes with 3 tiers each (36 total enemy configurations) are correctly defined and spawn in-game.
- **SC-002**: Enemy tier distribution per act is within 5% of target values over 100 generated maps.
- **SC-003**: All enemy traits trigger correctly in 100% of test scenarios covering each trigger type.
- **SC-004**: Gold rewards are correctly applied in 100% of combat victory scenarios.
- **SC-005**: Biome-weighted enemy selection shows statistically significant preference (>20% higher spawn rate) for emphasized archetypes.
- **SC-006**: Enemy spawn counts per act are within 10% of target values (36/40/44/48).

## Assumptions

- The combat system (auto-battle, turn order, damage calculation, status effects) is already implemented or will be implemented as a prerequisite.
- Map generation system can place entities on tiles.
- The seeded random number generator is available for deterministic enemy selection.
- Status effects (Chill, Shrapnel, Rust, Bleed) and damage types (weapon, non-weapon) are defined per the combat system specification.

## Reference

Enemy stat table (HP/ATK/ARM/SPD/DIG per tier):

| Enemy           | Emoji | T1         | T2         | T3         | Trait                                                         |
| --------------- | ----- | ---------- | ---------- | ---------- | ------------------------------------------------------------- |
| Tunnel Rat      | 🐀    | 5/1/0/3/1  | 7/2/0/4/1  | 9/3/1/5/2  | On Hit (once/turn): steal 1 Gold                              |
| Cave Bat        | 🦇    | 6/1/0/3/1  | 8/2/0/4/1  | 10/3/0/5/2 | Every other turn: restore 1 HP                                |
| Spore Slime     | 🟢    | 8/1/2/0/1  | 11/2/3/0/1 | 14/3/4/0/2 | Battle Start: apply 2 Chill to you                            |
| Rust Mite Swarm | 🐜    | 6/1/0/3/2  | 9/2/0/4/2  | 12/3/0/5/3 | On Hit (once/turn): apply 1 Rust                              |
| Collapsed Miner | 🧟    | 10/2/0/1/3 | 14/3/0/2/3 | 18/4/1/3/4 | Wounded: gain +3 ATK (this battle)                            |
| Shard Beetle    | 🪲    | 9/1/3/1/2  | 12/2/4/1/2 | 15/3/5/2/3 | Battle Start: gain 6 Shrapnel                                 |
| Tunnel Warden   | 🦀    | 8/2/4/2/2  | 11/3/6/3/2 | 14/4/8/4/3 | First strike each turn: remove 3 Armor from you before damage |
| Burrow Ambusher | 🦂    | 6/3/0/4/2  | 9/4/0/5/2  | 12/5/0/6/3 | Battle Start: deal 3 damage ignoring Armor                    |
| Frost Wisp      | 🧊    | 7/1/0/4/1  | 10/2/0/5/1 | 13/3/0/6/2 | If it acts first on Turn 1: apply 2 Chill                     |
| Powder Tick     | 🧨    | 7/1/0/2/1  | 10/2/0/3/1 | 13/3/0/4/2 | Countdown(2): deal 6 damage to you and itself (non-weapon)    |
| Coin Slug       | 🐌🪙  | 7/1/2/1/1  | 10/2/3/1/1 | 13/3/4/2/2 | Battle Start: gain Armor equal to floor(your Gold/10) (cap 3) |
| Blood Mosquito  | 🦟    | 6/1/0/3/1  | 9/2/0/4/1  | 12/3/0/5/2 | On Hit (once/turn): apply 1 Bleed                             |
