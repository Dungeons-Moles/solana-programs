# Dungeons & Moles Verification Spec v1.0

A 7-program Solana game where players enter sessions (campaign/duel/gauntlet), explore procedural maps on Ephemeral Rollups via session keys, fight enemies/bosses, collect items, and trade NFTs on a marketplace. SOL flows through duel vaults, gauntlet pools, run purchases, and NFT sales.

## 0. Security Goals

1. **Session Isolation**: A session signer MUST NOT be able to modify another player's GameState, Inventory, or MapPois.
2. **Authority Gatekeeping**: Authorized instructions (`_authorized`, `_cpi`) MUST only execute when called via CPI from their designated program's PDA signer.
3. **SOL Conservation**: Every SOL transfer MUST be accounted for: `buyer_debit = seller_credit + company_fee + gauntlet_fee` (marketplace) and `player_debit = treasury_half + gauntlet_half` (run purchase).
4. **Arithmetic Safety**: All gold, HP, lamport, and fee calculations MUST NOT overflow or underflow their storage types.
5. **State Machine Safety**: Session lifecycle MUST be irreversible (started -> delegated -> playing -> ended), dead players MUST NOT act, one-time POIs MUST NOT be reused.
6. **CPI Correctness**: Every cross-program invocation MUST target the correct program ID with the correct PDA signer seeds.
7. **Duel Vault Conservation**: Duel entry lamports deposited to vault MUST equal payouts distributed (winner + fees), with no lamports lost.

## 1. State Model

### GameState (gameplay-state)
```
player         : Pubkey    -- session owner wallet
session_signer : Pubkey    -- session key for ER gameplay
session        : Pubkey    -- linked GameSession PDA
hp             : i16       -- current HP (0 <= hp <= max_hp)
gold           : u16       -- current gold (0-65535)
is_dead        : bool      -- death flag (blocks all gameplay)
completed      : bool      -- level completion flag
boss_fight_ready : bool    -- boss triggered
moves_remaining : u8       -- moves left in phase
phase          : Phase     -- Day1/Night1/.../Night3
week           : u8        -- current week (1-5)
run_mode       : RunMode   -- Campaign/Duel/Gauntlet
```

### GameSession (session-manager)
```
player         : Pubkey    -- player wallet
session_signer : Pubkey    -- session key
settled        : bool      -- run result recorded
campaign_level : u8        -- level being played
```

### PlayerProfile (player-profile)
```
owner          : Pubkey    -- wallet address
available_runs : u32       -- remaining runs
highest_level_unlocked : u8
equipped_skin  : Option<Pubkey>
```

### Listing (nft-marketplace)
```
seller         : Pubkey
asset          : Pubkey
price_lamports : u64
```

### MarketplaceConfig (nft-marketplace)
```
company_fee_bps  : u16    -- default 300 (3%)
gauntlet_fee_bps : u16    -- default 200 (2%)
```

### PlayerInventory (player-inventory)
```
session           : Pubkey
player            : Pubkey
tool              : Option<ItemInstance>
gear              : [Option<ItemInstance>; 12]
gear_slot_capacity : u8   -- 4/6/8/10/12
```

### DuelEntry (gameplay-state)
```
player          : Pubkey
entry_lamports  : u64     -- stake deposited
settled         : bool
outcome         : DuelRunOutcome
```

### Lifecycle Diagram
```
Session:  start -> delegate -> [gameplay on ER] -> undelegate -> end/abandon
Player:   alive -> (combat) -> dead  [irreversible within session]
POI:      unused -> used  [one-time POIs, irreversible]
Listing:  listed -> sold | cancelled
Duel:     entered -> settled [payout distributed]
```

## 2. Operations

### 2.1 move_player (gameplay-state)
**Signers**: session_signer (as `player` in context)
**Preconditions**: `!is_dead`, `!boss_fight_ready`, target adjacent, target in bounds, `moves_remaining >= move_cost`
**Effects**: (1) Deduct move cost, (2) update position, (3) resolve combat if enemy present, (4) advance phase if moves exhausted
**Postconditions**: `position = (target_x, target_y)`, `moves_remaining' = moves_remaining - cost`

### 2.2 trigger_boss_fight (gameplay-state)
**Signers**: session_signer
**Preconditions**: `!is_dead`, `boss_fight_ready`
**Effects**: Resolve boss combat, set `is_dead` or advance week
**Postconditions**: `boss_fight_ready = false` (implicitly, via week advance or death)

### 2.3 heal_player (gameplay-state, authorized)
**Signers**: poi_authority PDA (poi-system)
**Preconditions**: amount > 0
**Effects**: `hp' = min(hp + amount, max_hp)`
**Postconditions**: `hp' <= max_hp`, `hp' >= hp`

### 2.4 modify_gold_authorized (gameplay-state, authorized)
**Signers**: poi_authority PDA (poi-system)
**Preconditions**: `gold + delta >= 0`, `gold + delta <= 65535`
**Effects**: `gold' = gold + delta`
**Postconditions**: `0 <= gold' <= 65535`

### 2.5 buy_nft (nft-marketplace)
**Signers**: buyer
**Preconditions**: `buyer != seller`, listing exists, buyer has sufficient lamports
**Effects**: (1) Transfer seller_amount to seller, (2) transfer company_fee to treasury, (3) transfer gauntlet_fee to pool, (4) transfer NFT to buyer
**Postconditions**: `seller_amount + company_fee + gauntlet_fee = price_lamports`

### 2.6 purchase_runs (player-profile)
**Signers**: owner
**Preconditions**: owner has >= 0.05 SOL
**Effects**: (1) Transfer 0.025 SOL to treasury, (2) transfer 0.025 SOL to gauntlet pool, (3) `available_runs += 20`
**Postconditions**: `treasury_credit + gauntlet_credit = 0.05 SOL`

### 2.7 enter_duel (gameplay-state)
**Signers**: player (main wallet)
**Preconditions**: `!is_dead`, `!completed`, `run_mode == Duel`, `entry_lamports == 0`
**Effects**: Transfer DUEL_ENTRY_LAMPORTS to vault, record entry
**Postconditions**: `duel_entry.entry_lamports == DUEL_ENTRY_LAMPORTS`

### 2.8 start_session (session-manager)
**Signers**: player (wallet), session_signer
**Preconditions**: `available_runs > 0`, `campaign_level <= highest_level_unlocked`, level in [1,40]
**Effects**: Consume run, create session + child accounts, snapshot item pool
**Postconditions**: `available_runs' = available_runs - 1`, session created

### 2.9 end_session (session-manager)
**Signers**: session_signer
**Preconditions**: `game_state.is_dead || game_state.completed`
**Effects**: Record run result, close all child accounts, close session
**Postconditions**: Session account closed, profile updated with result

### 2.10 record_run_result_cpi (player-profile)
**Signers**: session_signer, session_manager_authority PDA
**Preconditions**: session.player == profile.owner, session owned by session-manager
**Effects**: Update profile (total_runs, highest_level_unlocked if victory)
**Postconditions**: If victory and level_completed >= highest_level_unlocked, unlock next level

### 2.11 equip_gear_authorized (player-inventory)
**Signers**: poi_authority PDA (poi-system)
**Preconditions**: valid item ID, item is Gear type, empty slot available
**Effects**: Place item in first empty gear slot, apply HP bonus if applicable
**Postconditions**: `gear[slot] = Some(item)`, HP adjusted

### 2.12 expand_gear_slots_authorized (player-inventory)
**Signers**: gameplay_authority PDA (gameplay-state)
**Preconditions**: gear_slot_capacity < 12
**Effects**: `gear_slot_capacity += 2`
**Postconditions**: `gear_slot_capacity' = gear_slot_capacity + 2`

### 2.13 claim_gauntlet_rewards (gameplay-state)
**Signers**: player
**Preconditions**: `!reward_record.paid`, epoch finalized, `player_wallet == player`
**Effects**: Compute payout from pool, transfer from vault to player
**Postconditions**: `reward_record.paid = true`

## 3. Formal Properties

### 3.1 Access Control (AC)

**AC-1 (Session Isolation)**: For all GameState `gs` and Signer `s`,
if `move_player(gs, s, ...)` succeeds then `s.key == gs.session_signer`.

**AC-2 (Boss Fight Auth)**: For all GameState `gs` and Signer `s`,
if `trigger_boss_fight(gs, s)` succeeds then `s.key == gs.session_signer`.

**AC-3 (Heal Auth)**: For all `heal_player(gs, signer)` that succeeds,
`signer` MUST be the poi_authority PDA derived from `[b"poi_authority"]` under POI_SYSTEM_PROGRAM_ID.

**AC-4 (Gold Auth)**: For all `modify_gold_authorized(gs, signer)` that succeeds,
`signer` MUST be the poi_authority PDA derived from `[b"poi_authority"]` under POI_SYSTEM_PROGRAM_ID.

**AC-5 (Equip Auth)**: For all `equip_gear_authorized(inv, signer)` that succeeds,
`signer` MUST be the poi_authority PDA from POI_SYSTEM_PROGRAM_ID.

**AC-6 (Slot Expansion Auth)**: For all `expand_gear_slots_authorized(inv, signer)` that succeeds,
`signer` MUST be the gameplay_authority PDA from GAMEPLAY_STATE_PROGRAM_ID.

**AC-7 (Record Result Auth)**: For all `record_run_result_cpi(profile, session, signer, authority)` that succeeds,
`authority` MUST be session_manager_authority PDA AND `session.player == profile.owner`.

**AC-8 (Relic Grant Auth)**: For all `grant_relic_ownership(pool, signer)` that succeeds,
`signer` MUST be mint_authority PDA from NFT_MARKETPLACE_PROGRAM_ID.

**AC-9 (Cancel Listing Auth)**: For all `cancel_listing(listing, signer)` that succeeds,
`signer.key == listing.seller`.

**AC-10 (End Session Auth)**: For all `end_session(session, signer)` that succeeds,
`signer.key == session.session_signer` AND session has `has_one = player`.

### 3.2 SOL Conservation (SC)

**SC-1 (Marketplace Fee Split)**: For all `buy_nft` with price `p`, company_fee_bps `c`, gauntlet_fee_bps `g`:
`company_fee = floor(p * c / 10000)` AND
`gauntlet_fee = floor(p * g / 10000)` AND
`seller_amount = p - company_fee - gauntlet_fee` AND
`seller_amount + company_fee + gauntlet_fee = p`.

**SC-2 (Run Purchase Split)**: For all `purchase_runs` with cost `C = 50_000_000`:
`treasury_amount = C / 2` AND `gauntlet_amount = C - C/2` AND
`treasury_amount + gauntlet_amount = C`.

**SC-3 (Duel Entry Deposit)**: For all `enter_duel` that succeeds:
`vault_balance' = vault_balance + DUEL_ENTRY_LAMPORTS`.

### 3.3 Arithmetic Safety (AS)

**AS-1 (Gold Bounds)**: For all `modify_gold_authorized(gs, delta)` that succeeds:
`0 <= gs.gold' <= 65535`.

**AS-2 (HP Heal Bounds)**: For all `heal_player(gs, amount)` that succeeds:
`gs.hp' <= max_hp` AND `gs.hp' >= gs.hp`.

**AS-3 (HP Bonus Bounds)**: For all `add_hp_bonus_authorized(gs, bonus)` that succeeds:
`bonus > 0` AND `gs.hp' = gs.hp + bonus` AND no i16 overflow.

**AS-4 (Fee Overflow Safety)**: For all `buy_nft` with `price <= u64::MAX`:
`price * company_fee_bps` MUST NOT overflow u64.

**AS-5 (Run Count Safety)**: For all `purchase_runs` that succeeds:
`available_runs' = available_runs + 20` AND no u32 overflow.

### 3.4 State Machine (SM)

**SM-1 (Dead Players Blocked)**: For all GameState `gs` where `gs.is_dead = true`:
`move_player(gs, ...)` MUST fail AND `trigger_boss_fight(gs, ...)` MUST fail.

**SM-2 (Boss Gate)**: For all `trigger_boss_fight(gs)` that succeeds:
`gs.boss_fight_ready = true` (precondition).

**SM-3 (One-Time POI)**: For all POI with `use_type = OneTime` and `poi.used = true`:
interaction MUST fail with `PoiAlreadyUsed`.

**SM-4 (Session End Gate)**: For all `end_session(session, game_state)` that succeeds:
`game_state.is_dead || game_state.completed`.

**SM-5 (Duel No Double Entry)**: For all `enter_duel(gs, duel_entry)` that succeeds:
`duel_entry.entry_lamports == 0` (precondition).

**SM-6 (Gauntlet No Double Claim)**: For all `claim_gauntlet_rewards(record)` that succeeds:
`!record.paid` (precondition).

**SM-7 (Gear Slot Expansion Monotonic)**: For all `expand_gear_slots_authorized(inv)` that succeeds:
`inv.gear_slot_capacity' > inv.gear_slot_capacity`.

### 3.5 CPI Correctness (CPI)

**CPI-1 (Heal targets gameplay-state)**: The CPI from poi-system to `heal_player` MUST target GAMEPLAY_STATE_PROGRAM_ID.

**CPI-2 (Equip targets player-inventory)**: The CPI from poi-system to `equip_gear_authorized` MUST target PLAYER_INVENTORY_PROGRAM_ID.

**CPI-3 (Record result targets player-profile)**: The CPI from session-manager to `record_run_result_cpi` MUST target PLAYER_PROFILE_PROGRAM_ID with session_manager_authority PDA signer.

**CPI-4 (Relic grant targets player-profile)**: The CPI from nft-marketplace to `grant_relic_ownership` MUST target PLAYER_PROFILE_PROGRAM_ID with mint_authority PDA signer.

**CPI-5 (Slot expansion targets player-inventory)**: The CPI from gameplay-state to `expand_gear_slots_authorized` MUST target PLAYER_INVENTORY_PROGRAM_ID with gameplay_authority PDA signer.

**CPI-6 (Gold modify targets gameplay-state)**: The CPI from poi-system to `modify_gold_authorized` MUST target GAMEPLAY_STATE_PROGRAM_ID with poi_authority PDA signer.

## 4. Trust Boundary

The following are axiomatic (not verified):
- **Solana runtime**: PDA derivation is correct, signer checks are enforced, account ownership is validated
- **Anchor framework**: `has_one`, `seeds`, `owner` constraints are correctly compiled to runtime checks
- **Metaplex Core**: NFT transfers/plugin operations behave as documented
- **MagicBlock ER**: Delegation/undelegation preserves account data integrity
- **System Program**: SOL transfers are atomic and correct

We verify the **program logic layer**: that given correct runtime enforcement, the business logic preserves the security properties above.

## 5. Verification Results

| Property | Status | Proof |
|---|---|---|
| AC-1 Session Isolation (move) | **Verified** | `Proofs/AccessControl.lean:ac1_session_isolation_move` |
| AC-2 Boss Fight Auth | **Verified** | `Proofs/AccessControl.lean:ac2_boss_fight_auth` |
| AC-3 Heal Auth | **Verified** | `Proofs/AccessControl.lean:ac3_heal_auth` |
| AC-4 Gold Auth | **Verified** | `Proofs/AccessControl.lean:ac4_gold_auth` |
| AC-5 Equip Auth | **Verified** | `Proofs/AccessControl.lean:ac5_equip_auth` |
| AC-6 Slot Expansion Auth | **Verified** | `Proofs/AccessControl.lean:ac6_slot_expansion_auth` |
| AC-7 Record Result Auth | **Verified** | `Proofs/AccessControl.lean:ac7_record_result_auth` |
| AC-8 Relic Grant Auth | **Verified** | `Proofs/AccessControl.lean:ac8_relic_grant_auth` |
| AC-9 Cancel Listing Auth | **Verified** | `Proofs/AccessControl.lean:ac9_cancel_listing_auth` |
| AC-10 End Session Auth | **Verified** | `Proofs/AccessControl.lean:ac10_end_session_auth` |
| SC-1 Marketplace Fee Split | **Verified** | `Proofs/SolConservation.lean:sc1_marketplace_fee_conservation` |
| SC-2 Run Purchase Split | **Verified** | `Proofs/SolConservation.lean:sc2_run_purchase_conservation` |
| SC-3 Duel Entry Deposit | **Verified** | `Proofs/SolConservation.lean:sc3_duel_entry_deposit` |
| AS-1 Gold Bounds | **Verified** | `Proofs/ArithmeticSafety.lean:as1_gold_bounds` |
| AS-2 HP Heal Bounds | **Verified** | `Proofs/ArithmeticSafety.lean:as2_hp_heal_capped` |
| AS-3 HP Bonus Bounds | **Verified** | `Proofs/ArithmeticSafety.lean:as3_hp_bonus_in_range` |
| AS-4 Fee Overflow Safety | **Verified** | `Proofs/ArithmeticSafety.lean:as4_fee_fits_u128` |
| AS-5 Run Count Safety | **Verified** | `Proofs/ArithmeticSafety.lean:as5_run_count_safe` |
| SM-1 Dead Players Blocked | **Verified** | `Proofs/StateMachine.lean:sm1_dead_blocks_move` |
| SM-2 Boss Gate | **Verified** | `Proofs/StateMachine.lean:sm2_boss_gate` |
| SM-3 One-Time POI | **Verified** | `Proofs/StateMachine.lean:sm3_onetime_no_reuse` |
| SM-4 Session End Gate | **Verified** | `Proofs/StateMachine.lean:sm4_session_end_gate` |
| SM-5 Duel No Double Entry | **Verified** | `Proofs/StateMachine.lean:sm5_no_double_entry` |
| SM-6 Gauntlet No Double Claim | **Verified** | `Proofs/StateMachine.lean:sm6_no_double_claim` |
| SM-7 Gear Slot Monotonic | **Verified** | `Proofs/StateMachine.lean:sm7_gear_slot_monotonic` |
| CPI-1 Heal targets gameplay-state | **Verified** | `Proofs/CpiCorrectness.lean:cpi1_heal_targets_gameplay_state` |
| CPI-2 Equip targets player-inventory | **Verified** | `Proofs/CpiCorrectness.lean:cpi2_equip_targets_player_inventory` |
| CPI-3 Record result targets player-profile | **Verified** | `Proofs/CpiCorrectness.lean:cpi3_record_result_targets_player_profile` |
| CPI-4 Relic grant targets player-profile | **Verified** | `Proofs/CpiCorrectness.lean:cpi4_relic_grant_targets_player_profile` |
| CPI-5 Slot expansion targets player-inventory | **Verified** | `Proofs/CpiCorrectness.lean:cpi5_expand_slots_targets_player_inventory` |
| CPI-6 Gold modify targets gameplay-state | **Verified** | `Proofs/CpiCorrectness.lean:cpi6_modify_gold_targets_gameplay_state` |
