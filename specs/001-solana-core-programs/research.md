# Research: Solana Core Programs

**Feature**: 001-solana-core-programs
**Date**: 2025-01-15

## R1: MagicBlock Ephemeral Rollup Integration

**Decision**: Use `ephemeral_rollups_sdk` crate with `#[ephemeral]` attribute for Anchor programs

**Rationale**:
- MagicBlock provides official SDK (`ephemeral_rollups_sdk`) for delegation/undelegation
- The `#[ephemeral]` attribute on Anchor programs enables automatic ephemeral rollup compatibility
- SDK provides three core functions: `delegate_account`, `commit_accounts`, `commit_and_undelegate_accounts`
- Delegation transfers PDA ownership to the delegation program, enabling ephemeral validators to process transactions

**Key Integration Patterns**:

1. **Delegation** (Base Layer → Ephemeral):
   ```rust
   delegate_account(
       &ctx.accounts.payer,
       &ctx.accounts.pda,
       &ctx.accounts.owner_program,
       pda_seeds,
       0,  // 0 = no time limit
       3_000,  // commit frequency in ms
   )
   ```

2. **Commit** (Ephemeral → Base Layer, keep delegated):
   ```rust
   commit_accounts(
       &ctx.accounts.payer,
       vec![&ctx.accounts.pda.to_account_info()],
       &ctx.accounts.magic_context,
       &ctx.accounts.magic_program,
   )
   ```

3. **Undelegate** (Ephemeral → Base Layer, return ownership):
   ```rust
   commit_and_undelegate_accounts(
       &ctx.accounts.payer,
       vec![&ctx.accounts.pda.to_account_info()],
       &ctx.accounts.magic_context,
       &ctx.accounts.magic_program,
   )
   ```

**Required Accounts for Delegation**:
- `payer`: Transaction fee payer
- `pda`: Account to delegate
- `owner_program`: Program that owns the PDA
- `delegation_buffer`, `delegation_record`, `delegation_metadata`: MagicBlock system accounts
- `delegation_program`: MagicBlock delegation program
- `system_program`: Solana system program

**Alternatives Considered**:
- Native Solana without rollups: Rejected due to latency requirements for real-time gameplay
- Custom rollup solution: Rejected due to complexity and MagicBlock's gaming focus

---

## R2: Anchor Program Structure

**Decision**: Use Anchor 0.30+ with standard program structure and `InitSpace` derive macro

**Rationale**:
- Anchor provides type-safe account handling with constraint macros
- `InitSpace` derive macro automatically calculates account space requirements
- Standard structure: `declare_id!`, `#[program]` module, `#[derive(Accounts)]` structs, `#[account]` data types
- Error handling via `#[error_code]` attribute and `error!()` macro

**Account Initialization Pattern**:
```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = signer,
        space = 8 + PlayerProfile::INIT_SPACE,
        seeds = [b"player", signer.key().as_ref()],
        bump
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

**PDA Derivation Strategy**:
- PlayerProfile: `[b"player", wallet_pubkey]`
- GameSession: `[b"session", player_pubkey]`
- MapConfig: `[b"map_config"]` (singleton)
- Treasury: `[b"treasury"]` (singleton)

**Alternatives Considered**:
- Native Solana programs: Rejected due to lack of type safety and higher security risk
- Seahorse (Python): Rejected due to less mature ecosystem

---

## R3: Deterministic Map Generation On-Chain

**Decision**: Implement seeded RNG using XorShift algorithm, matching TypeScript implementation

**Rationale**:
- The existing TypeScript implementation uses `SeededRNG` with XorShift algorithm
- Same algorithm must be used on-chain for client-server verification
- Map generation is a pure function: `seed → map`
- No external randomness sources (no Switchboard/VRF needed for determinism)

**XorShift Implementation** (to match TypeScript):
```rust
pub struct SeededRNG {
    state: u64,
}

impl SeededRNG {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }  // Ensure non-zero
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn next_int(&mut self, min: u64, max: u64) -> u64 {
        min + (self.next() % (max - min + 1))
    }
}
```

**Map Generation Outputs**:
- 50x50 tile grid (2500 tiles × 1 byte = 2.5 KB)
- POI positions (up to ~40 POIs × 8 bytes = 320 bytes)
- Enemy spawn positions (up to ~125 enemies × 12 bytes = 1.5 KB)
- Boss spawn position (8 bytes)
- Total: ~4.5 KB per map

**On-Chain vs Off-Chain Decision**:
- Full map generation: Off-chain (too compute-intensive for on-chain)
- Map verification: On-chain (verify hash of client-generated map)
- Seed-to-level mapping: On-chain (81 u64 seeds = 648 bytes)

**Alternatives Considered**:
- Full on-chain generation: Rejected due to compute unit limits
- Switchboard VRF: Rejected because determinism from fixed seeds is required

---

## R4: Payment Processing

**Decision**: Use native SOL transfer via System Program CPI

**Rationale**:
- Simple 0.05 SOL payment per tier unlock
- No need for SPL tokens at this stage
- Treasury account is a PDA owned by the program
- Withdrawal requires admin signature

**Transfer Pattern**:
```rust
let cpi_context = CpiContext::new(
    ctx.accounts.system_program.to_account_info(),
    Transfer {
        from: ctx.accounts.payer.to_account_info(),
        to: ctx.accounts.treasury.to_account_info(),
    },
);
transfer(cpi_context, TIER_UNLOCK_COST)?;
```

**Constants**:
- `TIER_UNLOCK_COST`: 50_000_000 lamports (0.05 SOL)
- `LEVELS_PER_TIER`: 40

**Alternatives Considered**:
- SPL token payments: Deferred to future feature for in-game currency
- Direct wallet-to-wallet: Rejected for accountability and on-chain tracking

---

## R5: Testing Strategy

**Decision**: Use Bankrun for fast local testing, Anchor test framework for integration

**Rationale**:
- Bankrun provides fast local validator simulation
- Anchor's built-in test framework handles TypeScript integration tests
- Constitution requires 80% test coverage minimum
- TDD approach: tests before implementation

**Test Categories**:
1. **Unit Tests** (Rust): Test individual functions, RNG determinism, account validation
2. **Integration Tests** (TypeScript): Full instruction flows, multi-account interactions
3. **Contract Tests**: Verify IDL matches implementation
4. **Determinism Tests**: Verify same seed produces same map across runs

**Test Dependencies**:
- `solana-program-test`: Local validator simulation
- `solana-banks-client`: Bankrun integration
- `@coral-xyz/anchor`: TypeScript test framework
- `chai`: Assertions

**Alternatives Considered**:
- Amman: Less active development
- Manual devnet testing only: Too slow for TDD workflow

---

## R6: Account Space Calculations

**Decision**: Use `InitSpace` derive macro with explicit field annotations

**PlayerProfile Space**:
```rust
#[account]
#[derive(InitSpace)]
pub struct PlayerProfile {
    pub owner: Pubkey,           // 32 bytes
    #[max_len(32)]
    pub name: String,            // 4 + 32 = 36 bytes
    pub total_runs: u32,         // 4 bytes
    pub current_level: u8,       // 1 byte
    pub unlocked_tier: u8,       // 1 byte
    pub created_at: i64,         // 8 bytes
    pub bump: u8,                // 1 byte
}
// Total: 8 (discriminator) + 32 + 36 + 4 + 1 + 1 + 8 + 1 = 91 bytes
```

**GameSession Space**:
```rust
#[account]
#[derive(InitSpace)]
pub struct GameSession {
    pub player: Pubkey,          // 32 bytes
    pub session_id: u64,         // 8 bytes
    pub started_at: i64,         // 8 bytes
    pub last_activity: i64,      // 8 bytes
    pub is_delegated: bool,      // 1 byte
    pub campaign_level: u8,      // 1 byte
    pub bump: u8,                // 1 byte
}
// Total: 8 + 32 + 8 + 8 + 8 + 1 + 1 + 1 = 67 bytes
```

**MapConfig Space**:
```rust
#[account]
#[derive(InitSpace)]
pub struct MapConfig {
    pub admin: Pubkey,           // 32 bytes
    pub seeds: [u64; 81],        // 648 bytes
    pub version: u8,             // 1 byte
    pub bump: u8,                // 1 byte
}
// Total: 8 + 32 + 648 + 1 + 1 = 690 bytes
```
