# Contract: poi-system

**Program ID**: (existing)

## Instructions

### interact_poi (MODIFIED)

Interacts with a POI, validating player is on the correct tile.

```rust
pub fn interact_poi(
    ctx: Context<InteractPoi>,
    poi_index: u8,
) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| map_pois | Account (mut) | POIs on map |
| game_state | Account | Player state (for position) |
| game_session | Account | Parent session (for active_item_pool) |
| player_inventory | Account (mut) | For item pickup |
| player | Signer | Session owner or burner |

**Logic Changes**:

1. **Position Validation** (NEW):

   ```rust
   let poi = map_pois.pois.get(poi_index as usize)?;
   require!(
       game_state.position_x == poi.x && game_state.position_y == poi.y,
       PoiError::PlayerNotOnPoiTile
   );
   ```

2. **Item Pool Filtering** (NEW):
   - When generating shop/cache offers, filter by `game_session.active_item_pool`
   - Only items with bit set in pool can appear as offers

---

### open_shop (MODIFIED)

Opens shop interface at Smuggler Hatch POI.

**Logic Changes**:

- Filter shop offers by `game_session.active_item_pool`
- Reroll also respects pool filter

---

### generate_cache_offer (MODIFIED)

Generates item offer for cache POIs.

**Logic Changes**:

- Only select items where `is_bit_set(active_item_pool, item_index)` is true

---

## New Error

```rust
pub enum PoiError {
    // ... existing ...

    /// Player must be standing on the POI tile to interact
    #[msg("Player is not on the POI tile")]
    PlayerNotOnPoiTile,
}
```

---

## Events

No new events - existing events sufficient.
