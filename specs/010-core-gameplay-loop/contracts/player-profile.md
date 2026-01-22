# Contract: player-profile

**Program ID**: `29DPbP1zuCCRg63PiShMjxAmZos97BR5TmhpijUYQzze`

## Instructions

### initialize_profile

Creates a new player profile with starter items and runs.

```rust
pub fn initialize_profile(ctx: Context<InitializeProfile>, name: String) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player_profile | Account (init) | PDA: ["player", owner] |
| owner | Signer (mut) | Wallet creating profile |
| system_program | Program | System program |

**Logic**:

1. Validate name length <= 32
2. Set `unlocked_items = STARTER_ITEMS_BITMASK`
3. Set `active_item_pool = STARTER_ITEMS_BITMASK`
4. Set `available_runs = 20`
5. Set `highest_level_unlocked = 1`
6. Emit `ProfileCreated` event

---

### purchase_runs (NEW)

Allows player to purchase 20 additional runs for 0.001 SOL.

```rust
pub fn purchase_runs(ctx: Context<PurchaseRuns>) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player_profile | Account (mut) | Player's profile |
| owner | Signer (mut) | Profile owner, pays SOL |
| treasury | AccountInfo (mut) | Treasury wallet receiving SOL |
| system_program | Program | System program |

**Logic**:

1. Transfer 1,000,000 lamports from owner to treasury
2. Add 20 to `available_runs` (checked_add)
3. Emit `RunsPurchased` event

**Errors**:

- `InsufficientPayment`: Owner has < 0.001 SOL

---

### record_run_result (MODIFIED)

Records a completed run and handles level/item unlocks.

```rust
pub fn record_run_result(
    ctx: Context<RecordRunResult>,
    level_completed: u8,
    victory: bool,
) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player_profile | Account (mut) | Player's profile |
| owner | Signer | Profile owner |

**Logic**:

1. Decrement `available_runs` (checked_sub)
2. Increment `total_runs` (checked_add)
3. If `victory == true` AND `level_completed == highest_level_unlocked`:
   a. Increment `highest_level_unlocked` (if < 40)
   b. Select random locked item (PRNG)
   c. Set bit in `unlocked_items`
   d. Set bit in `active_item_pool`
   e. Emit `ItemUnlocked` event
4. Emit `RunCompleted` event

---

### update_active_pool (NEW)

Updates the player's active item pool for future sessions.

```rust
pub fn update_active_pool(
    ctx: Context<UpdateActivePool>,
    new_pool: [u8; 10],
) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| player_profile | Account (mut) | Player's profile |
| owner | Signer | Profile owner |

**Logic**:

1. Validate `count_bits(new_pool) >= 40`
2. Validate `is_subset(new_pool, unlocked_items)`
3. Set `active_item_pool = new_pool`
4. Emit `ActivePoolUpdated` event

**Errors**:

- `ActivePoolTooSmall`: Pool has < 40 items
- `ItemNotUnlocked`: Pool contains item not in unlocked_items

---

## Events

### ProfileCreated

```rust
#[event]
pub struct ProfileCreated {
    pub owner: Pubkey,
    pub timestamp: i64,
}
```

### RunsPurchased (NEW)

```rust
#[event]
pub struct RunsPurchased {
    pub owner: Pubkey,
    pub runs_added: u32,
    pub new_total: u32,
    pub timestamp: i64,
}
```

### ItemUnlocked (NEW)

```rust
#[event]
pub struct ItemUnlocked {
    pub owner: Pubkey,
    pub item_index: u8,
    pub level_completed: u8,
    pub timestamp: i64,
}
```

### ActivePoolUpdated (NEW)

```rust
#[event]
pub struct ActivePoolUpdated {
    pub owner: Pubkey,
    pub item_count: u8,
    pub timestamp: i64,
}
```

### RunCompleted

```rust
#[event]
pub struct RunCompleted {
    pub owner: Pubkey,
    pub total_runs: u32,
    pub available_runs: u32,
    pub level_reached: u8,
    pub victory: bool,
    pub timestamp: i64,
}
```
