# POI System IDL Extensions: Item Offer Generation

**Feature**: 008-item-offer-generation
**Date**: 2026-01-20

This document describes the Anchor IDL extensions to the poi-system program for item offer generation.

## New Instructions

### generate_cache_offer

Generates a 3-item offer for Supply Cache (L2), Tool Crate (L3), Geode Vault (L12), or Counter Cache (L13) POIs.

```rust
pub fn generate_cache_offer(ctx: Context<GenerateCacheOffer>) -> Result<()>
```

**Accounts**:
```rust
#[derive(Accounts)]
pub struct GenerateCacheOffer<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        seeds = [b"session", player.key().as_ref()],
        bump = session.bump,
    )]
    pub session: Account<'info, Session>,

    #[account(
        mut,
        seeds = [b"map_pois", session.key().as_ref()],
        bump = map_pois.bump,
        has_one = session,
    )]
    pub map_pois: Account<'info, MapPois>,

    #[account(
        seeds = [b"game_state", session.key().as_ref()],
        bump = game_state.bump,
    )]
    pub game_state: Account<'info, GameState>,
}
```

**Arguments**: None (POI index is determined by current interaction context)

**Events Emitted**:
```rust
#[event]
pub struct CacheOfferGenerated {
    pub session: Pubkey,
    pub poi_index: u8,
    pub poi_type: u8,
    pub items: [(u64, u8); 3],  // (item_id as u64, rarity)
}
```

**Errors**:
- `InvalidPoiType` - POI is not an item-granting cache
- `PoiAlreadyUsed` - One-time POI has been used
- `NoActiveInteraction` - No POI interaction in progress

---

### generate_shop_inventory

Generates a 6-item shop inventory for Smuggler Hatch (L9) POI.

```rust
pub fn generate_shop_inventory(ctx: Context<GenerateShopInventory>) -> Result<()>
```

**Accounts**:
```rust
#[derive(Accounts)]
pub struct GenerateShopInventory<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        seeds = [b"session", player.key().as_ref()],
        bump = session.bump,
    )]
    pub session: Account<'info, Session>,

    #[account(
        mut,
        seeds = [b"map_pois", session.key().as_ref()],
        bump = map_pois.bump,
        has_one = session,
    )]
    pub map_pois: Account<'info, MapPois>,

    #[account(
        seeds = [b"game_state", session.key().as_ref()],
        bump = game_state.bump,
    )]
    pub game_state: Account<'info, GameState>,
}
```

**Arguments**: None

**Events Emitted**:
```rust
#[event]
pub struct ShopInventoryGenerated {
    pub session: Pubkey,
    pub poi_index: u8,
    pub items: [(u64, u8, u8, u16); 6],  // (item_id, rarity, item_type, price)
    pub reroll_count: u8,
}
```

**Errors**:
- `InvalidPoiType` - POI is not a Smuggler Hatch
- `ShopAlreadyActive` - Shop inventory already generated for this visit

---

### reroll_shop

Rerolls the Smuggler Hatch shop inventory for escalating Gold cost.

```rust
pub fn reroll_shop(ctx: Context<RerollShop>) -> Result<()>
```

**Accounts**:
```rust
#[derive(Accounts)]
pub struct RerollShop<'info> {
    #[account(mut)]
    pub player: Signer<'info>,

    #[account(
        seeds = [b"session", player.key().as_ref()],
        bump = session.bump,
    )]
    pub session: Account<'info, Session>,

    #[account(
        mut,
        seeds = [b"map_pois", session.key().as_ref()],
        bump = map_pois.bump,
        has_one = session,
    )]
    pub map_pois: Account<'info, MapPois>,

    #[account(
        mut,
        seeds = [b"game_state", session.key().as_ref()],
        bump = game_state.bump,
    )]
    pub game_state: Account<'info, GameState>,
}
```

**Arguments**: None

**Events Emitted**:
```rust
#[event]
pub struct ShopRerolled {
    pub session: Pubkey,
    pub poi_index: u8,
    pub new_items: [(u64, u8, u8, u16); 6],
    pub reroll_count: u8,
    pub gold_spent: u16,
    pub remaining_gold: u16,
}
```

**Errors**:
- `NoActiveShop` - No shop inventory to reroll
- `InsufficientGold` - Player cannot afford reroll cost

---

## Extended Account Structures

### MapPois (Extended)

```rust
#[account]
pub struct MapPois {
    // Existing fields...
    pub session: Pubkey,
    pub bump: u8,
    pub count: u8,
    pub act: u8,
    pub week: u8,
    pub seed: u64,
    pub pois: Vec<PoiInstance>,

    // New fields for offer generation
    pub current_offer: Option<CacheOffer>,
    pub shop_state: Option<ShopState>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CacheOffer {
    pub poi_index: u8,
    pub items: [OfferItem; 3],
    pub generated_at_seed: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct OfferItem {
    pub item_id: [u8; 8],
    pub rarity: u8,  // 0=Common, 1=Rare, 2=Heroic, 3=Mythic
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ShopState {
    pub active: bool,
    pub poi_index: u8,
    pub reroll_count: u8,
    pub inventory: [ShopItem; 6],
    pub rng_state: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct ShopItem {
    pub item_id: [u8; 8],
    pub rarity: u8,
    pub item_type: u8,  // 0=Gear, 1=Tool
    pub price: u16,
}
```

---

## New Error Codes

```rust
#[error_code]
pub enum OfferError {
    #[msg("POI type does not support item offers")]
    InvalidPoiType,

    #[msg("This POI has already been used")]
    PoiAlreadyUsed,

    #[msg("No POI interaction is currently active")]
    NoActiveInteraction,

    #[msg("Shop inventory already generated for this visit")]
    ShopAlreadyActive,

    #[msg("No active shop to reroll")]
    NoActiveShop,

    #[msg("Insufficient gold for reroll")]
    InsufficientGold,

    #[msg("No items available for the selected tag and rarity")]
    NoItemsAvailable,

    #[msg("Invalid offer context")]
    InvalidOfferContext,
}
```

---

## Integration Notes

### Instruction Flow

1. Player moves to POI tile
2. Client calls `interact_poi` (existing instruction)
3. For item-granting POIs, client then calls:
   - `generate_cache_offer` for L2, L3, L12, L13
   - `generate_shop_inventory` for L9
4. Client displays offer to player
5. Player selects item (separate `pick_item` instruction)
6. For shops, player may call `reroll_shop` before picking

### Determinism Guarantee

All offer generation uses:
- Session seed from `MapPois.seed`
- POI index for uniqueness
- Call counter for rerolls

Same inputs always produce identical offers.

### Account Size Considerations

Extended `MapPois` account size:
- Existing: ~1000 bytes (50 POIs)
- CacheOffer: 26 bytes (1 + 3×(8+1))
- ShopState: 62 bytes (1+1+1+6×(8+1+1+2)+8)
- Total increase: ~90 bytes

Within Anchor account limits.
