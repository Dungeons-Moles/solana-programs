# Research: POI System

**Feature**: 007-poi-system
**Date**: 2026-01-20
**Status**: Complete

## Research Questions

### 1. POI Definition Storage Architecture

**Decision**: Compile-time constants with static arrays (same as items, field-enemies, bosses)

**Rationale**:
- 14 POI definitions are immutable game data from GDD
- Compile-time constants are free to access (no account reads)
- Follows established pattern from `player-inventory` (80 items), `field-enemies` (12 archetypes), `boss-system` (24 bosses)
- Program size increase is minimal (~1KB for all POI data)

**Alternatives Considered**:
- **On-chain config accounts**: Rejected - adds PDAs, rent costs, no runtime modification needed
- **External data loading**: Rejected - complicates build, no benefit for static data

### 2. POI Runtime State Architecture

**Decision**: MapPois PDA per session, similar to MapEnemies pattern

**Rationale**:
- POI instances need position (x, y), type reference, and used/unused state
- Session-scoped state allows cleanup when session ends
- Follows MapEnemies pattern from field-enemies program
- Maximum ~50 POIs per map fits in single account

**Data Structure**:
```rust
#[account]
pub struct MapPois {
    pub session: Pubkey,
    pub bump: u8,
    pub pois: Vec<PoiInstance>,
    pub count: u8,
}

pub struct PoiInstance {
    pub poi_type: u8,       // L1-L14 (index into definitions)
    pub x: u8,
    pub y: u8,
    pub used: bool,
    pub week_spawned: u8,   // For Counter Cache week tracking
}
```

**Space Calculation**:
- 8 (discriminator) + 32 (session) + 1 (bump) + 4 (vec len) + 50 * 5 (instances) + 1 (count)
- Total: 296 bytes (well under account limits)

### 3. Gold Tracking Location

**Decision**: Extend gameplay-state with gold field

**Rationale**:
- Gold is core game currency needed for shop, upgrades, and Scrap Chute
- GameState already tracks player resources (HP, stats, moves)
- Adding `gold: u16` is minimal change
- Allows POI interactions to modify gold via CPI to gameplay-state

**Alternative Considered**:
- **Separate gold account**: Rejected - unnecessary complexity, gold is session-scoped

### 4. Shop State Management

**Decision**: Separate ShopState struct stored inline in MapPois for active shop sessions

**Rationale**:
- Shop state (current offers, reroll count) must persist within a POI visit
- Not all POIs need shop state - only Smuggler Hatch (L9)
- Can store as optional field or separate account

**Implementation**:
```rust
pub struct ShopState {
    pub poi_index: u8,           // Which POI this shop is for
    pub offers: [ItemOffer; 6],  // 1 Tool + 5 Gear
    pub reroll_count: u8,        // Times rerolled this visit
    pub active: bool,            // Currently shopping
}
```

Store in MapPois as optional field - when player enters Smuggler Hatch, populate. Clear when leaving.

### 5. Item Offer Generation Integration

**Decision**: Reuse player-inventory offers module with POI-specific extensions

**Rationale**:
- `player-inventory/src/offers.rs` already implements:
  - Rarity tables by act
  - Tag weighting for boss weaknesses
  - Deterministic selection via seed
  - Price calculation
- Need to extend for:
  - Counter Cache (weakness-only filtering) - already done
  - Geode Vault (higher rarity table) - already done
  - Smuggler Hatch (separate gear/tool tables)

**Dependencies**:
- Import `generate_item_offer` from player-inventory (via CPI or lib import)
- Boss weakness tags from boss-system `get_boss_weaknesses`

### 6. Rarity Table Reconciliation

**Decision**: Use GDD tables (more granular than spec)

**Rationale**:
- Spec had simplified rarity tables (FR-005: 100% Common Act 1, 65%/35% Act 4)
- GDD Section 12 has more nuanced progression:
  - L2 Supply Cache: 100% → 85/15 → 75/25 → 65/35
  - L3 Tool Crate: 85/15 → 70/25/5 → 60/30/10 → 50/35/15
  - L9 Smuggler Hatch: Separate gear/tool tables
- Use GDD values for implementation fidelity

**Corrected Tables**:
```rust
const SUPPLY_CACHE_RARITY: [(u8, u8, u8, u8); 4] = [
    (100, 0, 0, 0),   // Act 1
    (85, 15, 0, 0),   // Act 2
    (75, 25, 0, 0),   // Act 3
    (65, 35, 0, 0),   // Act 4
];

const TOOL_CRATE_RARITY: [(u8, u8, u8, u8); 4] = [
    (85, 15, 0, 0),   // Act 1
    (70, 25, 5, 0),   // Act 2
    (60, 30, 10, 0),  // Act 3
    (50, 35, 15, 0),  // Act 4
];

const SMUGGLER_GEAR_RARITY: [(u8, u8, u8, u8); 4] = [
    (70, 27, 3, 0),   // Act 1
    (55, 38, 7, 0),   // Act 2
    (45, 42, 13, 0),  // Act 3
    (35, 45, 18, 2),  // Act 4
];

const SMUGGLER_TOOL_RARITY: [(u8, u8, u8, u8); 4] = [
    (80, 20, 0, 0),   // Act 1
    (65, 30, 5, 0),   // Act 2
    (55, 35, 10, 0),  // Act 3
    (45, 40, 15, 0),  // Act 4
];
```

### 7. Night-Only POI Enforcement

**Decision**: Check phase from GameState during interaction validation

**Rationale**:
- L1 (Mole Den) and L5 (Rest Alcove) are night-only
- GameState has `phase` field with `is_night()` method
- Interaction instruction checks phase before allowing action

**Implementation**:
```rust
pub fn interact_with_poi(ctx: Context<InteractWithPoi>, poi_index: u8) -> Result<()> {
    let poi = &ctx.accounts.map_pois.pois[poi_index as usize];
    let def = get_poi_definition(poi.poi_type);

    if def.night_only && !ctx.accounts.game_state.phase.is_night() {
        return Err(PoiSystemError::NightOnlyPoi.into());
    }
    // ...
}
```

### 8. Tool Oil State Tracking

**Decision**: Track oil applications in ItemInstance.tool_oil_flags (already exists)

**Rationale**:
- `player-inventory/src/state.rs` defines `ItemInstance.tool_oil_flags: u8`
- Bitflags: 0x01=+ATK, 0x02=+SPD, 0x04=+DIG
- Tool Oil Rack (L4) checks if modification already applied to current tool
- L4 is "repeatable per tool" - can use multiple times with different tools

### 9. Rail Waypoint Discovery

**Decision**: Track discovered waypoints in MapPois

**Rationale**:
- Rail Waypoints (L8) must be discovered before fast travel
- Add `discovered: bool` field to PoiInstance
- L8 instances start as `discovered: false`
- First interaction marks as discovered; subsequent interactions allow travel

**Extension to PoiInstance**:
```rust
pub struct PoiInstance {
    pub poi_type: u8,
    pub x: u8,
    pub y: u8,
    pub used: bool,          // One-time POIs
    pub discovered: bool,    // For Rail Waypoints
    pub week_spawned: u8,    // For Counter Cache
}
```

### 10. Spawn Configuration

**Decision**: Compile-time constants for spawn counts and guarantees per act

**Rationale**:
- Spawn configuration is immutable game data
- Used by map-generator (future) to place POIs
- Following same pattern as POI definitions

**Implementation**:
```rust
pub struct ActSpawnConfig {
    pub baseline: [(u8, u8); 5],    // (poi_type, count) pairs
    pub guarantees: &'static [SpawnGuarantee],
}

pub struct SpawnGuarantee {
    pub poi_type: u8,
    pub count: u8,
    pub week: Option<u8>,           // Week-specific (for Counter Cache)
    pub chance_percent: u8,          // 100 = guaranteed
    pub placement: PlacementRule,
}

pub enum PlacementRule {
    Random,
    AdjacentToStart,                 // L1 Mole Den
}
```

### 11. Program Dependencies

**Decision**: Cross-program imports (lib) rather than CPI for read-only data

**Rationale**:
- POI system needs:
  - Item definitions from player-inventory (for offers)
  - Boss weaknesses from boss-system (for tag weighting)
  - Phase/gold from gameplay-state (for validation)
- Read-only data can be imported directly (no CPI overhead)
- State mutations (gold changes) require CPI to gameplay-state

**Cargo.toml Dependencies**:
```toml
[dependencies]
player-inventory = { path = "../player-inventory", features = ["cpi"] }
boss-system = { path = "../boss-system", features = ["cpi"] }
gameplay-state = { path = "../gameplay-state", features = ["cpi"] }
```

### 12. Scrap Chute Gold Costs

**Decision**: Per-act cost table as compile-time constant

**Rationale**:
- GDD specifies: Act 1-2: 8 Gold, Act 3: 10 Gold, Act 4: 12 Gold
- Simple lookup table

**Implementation**:
```rust
const SCRAP_CHUTE_COST: [u16; 4] = [8, 8, 10, 12];  // Acts 1-4
```

## Dependencies Confirmed

| Dependency | Status | Notes |
|------------|--------|-------|
| player-inventory items | EXISTS | 80 items with tags, rarities, effects |
| player-inventory offers | EXISTS | generate_item_offer, rarity tables, tag weighting |
| boss-system weaknesses | EXISTS | get_boss_weaknesses returns [ItemTag; 2] |
| gameplay-state Phase | EXISTS | Phase enum with is_night() method |
| gameplay-state GameState | NEEDS GOLD | Add gold: u16 field |
| field-enemies MapEnemies | EXISTS | Pattern for session-scoped state |

## New Types Required

| Type | Purpose | Location |
|------|---------|----------|
| PoiDefinition | Static POI data | poi-system/pois.rs |
| PoiInstance | Runtime POI on map | poi-system/state.rs |
| MapPois | Session POI state | poi-system/state.rs |
| ShopState | Smuggler Hatch state | poi-system/state.rs |
| ActSpawnConfig | Spawn configuration | poi-system/spawn.rs |
| PoiSystemError | Custom errors | poi-system/errors.rs |

## Open Items

1. **GameState gold field**: Need to add `gold: u16` to gameplay-state. This is a minor extension task.
   - **Resolution**: Will create a task to extend gameplay-state before POI implementation.

All other research questions resolved.
