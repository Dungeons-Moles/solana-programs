# POI System IDL Contract

**Program ID**: TBD (will be generated)
**Version**: 0.1.0

## Instructions

### initialize_map_pois

Initialize POI state for a session. Called by map generator or session manager.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| payer | Signer, Mut | Pays for account rent |
| session | UncheckedAccount | GameSession PDA |
| map_pois | Account\<MapPois\>, Init | POI state account (PDA) |
| system_program | Program\<System\> | System program |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| act | u8 | Current act (1-4) |
| week | u8 | Current week (1-3) |
| seed | u64 | Deterministic seed for POI placement |

**Returns**: `Result<()>`

**Events**: `PoisInitialized { session, count, act }`

---

### get_poi_definition

View function to retrieve POI type definition.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| (none) | | Stateless view function |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_type | u8 | POI type ID (1-14) |

**Returns**: `Result<PoiInfo>`

**PoiInfo**:
```rust
pub struct PoiInfo {
    pub id: u8,
    pub name: String,
    pub emoji: [u8; 4],
    pub rarity: PoiRarity,
    pub use_type: UseType,
    pub active_condition: ActiveCondition,
    pub interaction_type: InteractionType,
}
```

---

### get_poi_at_position

Query POI at a map position.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| map_pois | Account\<MapPois\> | POI state account |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| x | u8 | X coordinate |
| y | u8 | Y coordinate |

**Returns**: `Result<Option<PoiInstanceInfo>>`

---

### interact_rest

Interact with rest POIs (L1 Mole Den, L5 Rest Alcove).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\>, Mut | Player's game state |
| map_pois | Account\<MapPois\>, Mut | POI state |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |

**Returns**: `Result<()>`

**Events**: `PoiInteracted { session, poi_type, x, y, interaction: "rest" }`

**Errors**:
- `NightOnlyPoi` - POI requires night phase
- `PoiAlreadyUsed` - One-time POI already used
- `InvalidPoiType` - POI is not a rest type

---

### interact_pick_item

Interact with item POIs (L2 Supply Cache, L3 Tool Crate, L12 Geode Vault, L13 Counter Cache).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\> | Player's game state (for act/week) |
| map_pois | Account\<MapPois\>, Mut | POI state |
| player_inventory | Account\<PlayerInventory\>, Mut | Player's inventory |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |
| offer_index | u8 | Which of the 3 offers to pick (0-2) |
| seed | u64 | Seed for item generation |

**Returns**: `Result<ItemOffer>` - The picked item

**Events**: `ItemPicked { session, poi_type, item_id, tier }`

**Errors**:
- `PoiAlreadyUsed` - One-time POI already used
- `InvalidOfferIndex` - offer_index >= 3
- `InventoryFull` - No space in inventory

---

### interact_tool_oil

Interact with Tool Oil Rack (L4).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| map_pois | Account\<MapPois\>, Mut | POI state |
| player_inventory | Account\<PlayerInventory\>, Mut | Player's inventory |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |
| modification | ToolOilModification | PlusAtk, PlusSpd, or PlusDig |

**Returns**: `Result<()>`

**Events**: `ToolOilApplied { session, modification }`

**Errors**:
- `NoToolEquipped` - Player has no tool
- `OilAlreadyApplied` - Modification already on this tool
- `InvalidPoiType` - POI is not Tool Oil Rack

---

### interact_survey_beacon

Interact with Survey Beacon (L6).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\> | Player position |
| map_pois | Account\<MapPois\>, Mut | POI state |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |

**Returns**: `Result<Vec<(u8, u8)>>` - Revealed tile coordinates (radius 13)

**Events**: `TilesRevealed { session, count, center_x, center_y }`

---

### interact_seismic_scanner

Interact with Seismic Scanner (L7).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\> | Player position |
| map_pois | Account\<MapPois\>, Mut | POI state |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |
| category | PoiCategory | Category to scan for |

**Returns**: `Result<Option<(u8, u8)>>` - Nearest POI position of category

**Events**: `PoiRevealed { session, poi_type, x, y }`

---

### interact_rail_waypoint

Interact with Rail Waypoint (L8).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\>, Mut | Player position |
| map_pois | Account\<MapPois\>, Mut | POI state |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index of current waypoint |
| destination_index | Option\<u8\> | Index of destination waypoint (None = just discover) |

**Returns**: `Result<()>`

**Events**:
- `WaypointDiscovered { session, x, y }` - First visit
- `FastTravelCompleted { session, from_x, from_y, to_x, to_y }` - Travel

**Errors**:
- `DestinationNotDiscovered` - Target waypoint not discovered
- `NoDestinationsAvailable` - No other discovered waypoints

---

### enter_shop

Enter Smuggler Hatch (L9) and generate offers.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\> | Act/week for rarity |
| map_pois | Account\<MapPois\>, Mut | POI state (holds ShopState) |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index of Smuggler Hatch |
| seed | u64 | Seed for offer generation |

**Returns**: `Result<[ItemOffer; 6]>` - Generated offers

**Events**: `ShopEntered { session, poi_index }`

---

### shop_purchase

Purchase item from Smuggler Hatch.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\>, Mut | Gold deduction |
| map_pois | Account\<MapPois\>, Mut | Shop state |
| player_inventory | Account\<PlayerInventory\>, Mut | Add item |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| offer_index | u8 | Which offer to purchase (0-5) |

**Returns**: `Result<()>`

**Events**: `ItemPurchased { session, item_id, price }`

**Errors**:
- `ShopNotActive` - Not in shop session
- `InsufficientGold` - Not enough gold
- `OfferAlreadyPurchased` - Item already bought
- `InventoryFull` - No space

---

### shop_reroll

Reroll Smuggler Hatch offers.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\>, Mut | Gold deduction |
| map_pois | Account\<MapPois\>, Mut | Shop state |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| seed | u64 | Seed for new offers |

**Returns**: `Result<[ItemOffer; 6]>` - New offers

**Events**: `ShopRerolled { session, cost, reroll_count }`

**Errors**:
- `ShopNotActive` - Not in shop session
- `InsufficientGold` - Not enough gold for reroll

---

### leave_shop

Exit Smuggler Hatch, clearing shop state.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| map_pois | Account\<MapPois\>, Mut | Clear shop state |

**Args**: None

**Returns**: `Result<()>`

**Events**: `ShopExited { session }`

---

### interact_rusty_anvil

Upgrade tool at Rusty Anvil (L10).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\>, Mut | Gold deduction |
| map_pois | Account\<MapPois\>, Mut | Mark used |
| player_inventory | Account\<PlayerInventory\>, Mut | Upgrade tool |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |

**Returns**: `Result<Tier>` - New tier

**Events**: `ToolUpgraded { session, item_id, old_tier, new_tier, cost }`

**Errors**:
- `PoiAlreadyUsed` - Already used this anvil
- `NoToolEquipped` - No tool to upgrade
- `AlreadyMaxTier` - Tool is Tier III
- `InsufficientGold` - Not enough gold

---

### interact_rune_kiln

Fuse items at Rune Kiln (L11).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| map_pois | Account\<MapPois\> | Verify POI type |
| player_inventory | Account\<PlayerInventory\>, Mut | Fuse items |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |
| gear_slot_1 | u8 | First gear slot index |
| gear_slot_2 | u8 | Second gear slot index |

**Returns**: `Result<Tier>` - Resulting tier

**Events**: `ItemsFused { session, item_id, result_tier }`

**Errors**:
- `ItemsNotIdentical` - Items don't match
- `AlreadyMaxTier` - Items are Tier III
- `InvalidGearSlot` - Slot index out of bounds

---

### interact_scrap_chute

Destroy gear at Scrap Chute (L14).

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player | Signer | Player authority |
| game_state | Account\<GameState\>, Mut | Gold deduction |
| map_pois | Account\<MapPois\>, Mut | Mark used |
| player_inventory | Account\<PlayerInventory\>, Mut | Remove gear |

**Args**:
| Name | Type | Description |
|------|------|-------------|
| poi_index | u8 | Index in MapPois.pois |
| gear_slot | u8 | Gear slot to destroy |

**Returns**: `Result<()>`

**Events**: `GearScrapped { session, item_id, cost }`

**Errors**:
- `PoiAlreadyUsed` - Already used this chute
- `InvalidGearSlot` - Slot empty or out of bounds
- `InsufficientGold` - Not enough gold

---

### get_spawn_config

View function to retrieve spawn configuration for an act.

**Accounts**: None (stateless)

**Args**:
| Name | Type | Description |
|------|------|-------------|
| act | u8 | Act number (1-4) |

**Returns**: `Result<SpawnConfigInfo>`

---

### close_map_pois

Close MapPois account, returning rent to payer.

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| authority | Signer | Session authority |
| map_pois | Account\<MapPois\>, Close | Account to close |
| rent_destination | AccountInfo, Mut | Receives rent |

**Args**: None

**Returns**: `Result<()>`

**Events**: `PoisClosed { session }`

## Events

| Event | Fields |
|-------|--------|
| PoisInitialized | session: Pubkey, count: u8, act: u8 |
| PoiInteracted | session: Pubkey, poi_type: u8, x: u8, y: u8, interaction: String |
| ItemPicked | session: Pubkey, poi_type: u8, item_id: [u8; 8], tier: Tier |
| ToolOilApplied | session: Pubkey, modification: ToolOilModification |
| TilesRevealed | session: Pubkey, count: u16, center_x: u8, center_y: u8 |
| PoiRevealed | session: Pubkey, poi_type: u8, x: u8, y: u8 |
| WaypointDiscovered | session: Pubkey, x: u8, y: u8 |
| FastTravelCompleted | session: Pubkey, from_x: u8, from_y: u8, to_x: u8, to_y: u8 |
| ShopEntered | session: Pubkey, poi_index: u8 |
| ItemPurchased | session: Pubkey, item_id: [u8; 8], price: u16 |
| ShopRerolled | session: Pubkey, cost: u16, reroll_count: u8 |
| ShopExited | session: Pubkey |
| ToolUpgraded | session: Pubkey, item_id: [u8; 8], old_tier: Tier, new_tier: Tier, cost: u16 |
| ItemsFused | session: Pubkey, item_id: [u8; 8], result_tier: Tier |
| GearScrapped | session: Pubkey, item_id: [u8; 8], cost: u16 |
| PoisClosed | session: Pubkey |

## Errors

| Code | Name | Message |
|------|------|---------|
| 6000 | InvalidAct | Act must be 1-4 |
| 6001 | InvalidPoiType | POI type must be 1-14 |
| 6002 | PoiNotFound | No POI at specified position |
| 6003 | PoiAlreadyUsed | One-time POI already used |
| 6004 | NightOnlyPoi | This POI can only be used at night |
| 6005 | NoToolEquipped | Player has no tool equipped |
| 6006 | OilAlreadyApplied | Tool oil modification already applied |
| 6007 | InsufficientGold | Not enough gold |
| 6008 | InventoryFull | No space in inventory |
| 6009 | ItemsNotIdentical | Items must be identical for fusion |
| 6010 | AlreadyMaxTier | Item is already maximum tier |
| 6011 | ShopNotActive | No active shop session |
| 6012 | OfferAlreadyPurchased | Offer already purchased |
| 6013 | DestinationNotDiscovered | Waypoint destination not discovered |
| 6014 | NoDestinationsAvailable | No other discovered waypoints |
| 6015 | InvalidOfferIndex | Offer index out of bounds |
| 6016 | InvalidGearSlot | Gear slot index out of bounds or empty |
