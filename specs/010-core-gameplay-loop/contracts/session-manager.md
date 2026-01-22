# Contract: session-manager

**Program ID**: `FcMT7MzBLVQGaMATEMws3fjsL2Q77QSHmoEPdowTMxJa`

## Instructions

### start_session (MODIFIED)

Creates a new game session for a specific level with SOL transfer to burner wallet.

```rust
pub fn start_session(
    ctx: Context<StartSession>,
    campaign_level: u8,
    burner_lamports: u64,
) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| game_session | Account (init) | PDA: ["session", player, level] |
| session_counter | Account (mut) | Global counter |
| player_profile | Account | Player's profile (for validation) |
| player | Signer (mut) | Main wallet |
| burner_wallet | AccountInfo (mut) | Burner wallet for gameplay |
| system_program | Program | System program |

**PDA Seeds Change**:

```rust
// OLD: seeds = [b"session", player.key().as_ref()]
// NEW: seeds = [b"session", player.key().as_ref(), &[campaign_level]]
```

**Logic**:

1. Validate `player_profile.available_runs > 0`
2. Validate `campaign_level <= player_profile.highest_level_unlocked`
3. Validate `campaign_level >= 1 && campaign_level <= 40`
4. Increment session counter
5. Initialize session fields
6. Copy `player_profile.active_item_pool` to `session.active_item_pool`
7. Store `burner_wallet` pubkey in session
8. Transfer `burner_lamports` from player to burner_wallet
9. Emit `SessionStarted` event

**Errors**:

- `NoAvailableRuns`: Player has 0 runs
- `LevelNotUnlocked`: Level > highest_level_unlocked
- `InvalidCampaignLevel`: Level out of range 1-40

---

### end_session (MODIFIED)

Ends session and triggers profile update via CPI.

```rust
pub fn end_session(
    ctx: Context<EndSession>,
    victory: bool,
) -> Result<()>
```

**Accounts**:
| Name | Type | Description |
|------|------|-------------|
| game_session | Account (mut, close) | Session to close |
| player_profile | Account (mut) | Player's profile |
| player | Signer (mut) | Session owner |
| player_profile_program | Program | player-profile program for CPI |

**Logic**:

1. CPI to `player_profile::record_run_result(level, victory)`
2. Close session account (rent to player)
3. Emit `SessionEnded` event

---

### delegate_session

Delegates session to MagicBlock ephemeral rollup. (No changes)

---

### commit_session

Commits state from ephemeral rollup. (No changes)

---

### force_close_session

Force closes abandoned session. (No changes)

---

## Events

### SessionStarted (MODIFIED)

```rust
#[event]
pub struct SessionStarted {
    pub player: Pubkey,
    pub session_id: u64,
    pub campaign_level: u8,
    pub burner_wallet: Pubkey,
    pub burner_lamports: u64,
    pub timestamp: i64,
}
```

### SessionEnded (MODIFIED)

```rust
#[event]
pub struct SessionEnded {
    pub player: Pubkey,
    pub session_id: u64,
    pub campaign_level: u8,
    pub victory: bool,
    pub final_state_hash: [u8; 32],
    pub timestamp: i64,
}
```

### SessionDelegated

(No changes)
