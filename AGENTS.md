# solana-programs Development Guidelines

## Active Technologies

- Rust 2021 edition (Solana BPF target) + Anchor 0.32.0 across all programs/crates
- Solana SDK/CLI 2.3+ and `solana-program` account/PDA patterns
- TypeScript integration tests via `ts-mocha` + `@coral-xyz/anchor` 0.32.0
- Shared gameplay crates: `combat-system`, `field-enemies`, `boss-system`
- MagicBlock integration is stubbed (SDK pending toolchain update)

## Commands

anchor build
anchor test
cargo test
cargo clippy
yarn test

## Code Style

Rust 1.75+ (Solana BPF target): Follow standard conventions

<!-- MANUAL ADDITIONS START -->

## Forbidden Actions (MANDATORY)

### NEVER run `cargo clean` without explicit user permission

`cargo clean` destroys the deploy keypairs in `target/deploy/`. These keypairs determine the on-chain program addresses. Once lost, those addresses are gone forever — you cannot redeploy to the same address. On mainnet this would be catastrophic. Always use `anchor clean` instead if you need to clear build artifacts (it preserves keypairs). If you truly need `cargo clean`, ask the user first and explain the consequences.

## Session & Signing Rules (MANDATORY)

These rules are non-negotiable. All code — programs and frontend — must follow them strictly. If existing code violates these rules, it must be refactored.

### Rule 1: One wallet signature per session entry

The player's wallet signs **exactly one transaction** to enter a game session. Every other in-session transaction (movement, combat, POI interaction, session closure) is signed by the **session key** in the background — no wallet popups.

The only exception is **abandon session**, which requires the wallet signature as a safety measure.

**Implication:** Any instruction that runs during a session (start to end) must accept the session signer, not the player wallet. Entry fees, echo draws, and any other setup must be bundled into the single entry transaction or handled by session-key-authorized instructions.

### Rule 2: All in-session gameplay happens on the Ephemeral Rollup

Everything between delegation and undelegation runs on the ER via session keys. No base-layer transactions during active gameplay.

Base-layer wallet transactions are only for **out-of-session** actions:
- Starting/entering a session (the single wallet-signed entry tx)
- Equipping skins
- Buying sessions / top-ups
- Marketplace trades (list, buy, cancel)
- Managing the item pool

**Implication:** Settlement, point crediting, echo insertion, and any other post-game bookkeeping that touches global/shared accounts must either (a) be deferred to session end and signed by the session key, or (b) be handled by a PDA authority so the session key can invoke it via CPI. Never require the player wallet mid-session or at session teardown.

### Rule 2.1: All VRF must be on the Ephemeral Rollup (MANDATORY)

This rule is non-negotiable across programs and frontend integration.

- **Localnet only:** `*Rng` flows are allowed for local deterministic testing.
- **Devnet/Mainnet:** all session-critical randomness must use `*Vrf` flows and on-chain fulfillment/callbacks.
- **VRF oracle queue:** All programs must use the ER oracle queue (`5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc`). Never use the base-layer `DEFAULT_QUEUE` (`Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh`) in any VRF instruction (`oracle_queue` accounts must be `UncheckedAccount` — not constrained to a specific address).
- **VRF timing:** `init_*_vrf_state` and `request_*_vrf` MUST run on the Ephemeral Rollup **after** delegation, never on the base chain. The frontend must send these transactions via the routed ER connection (`sendRoutedErTransaction`).
- Programs must not rely on client-generated randomness on devnet/mainnet for gameplay-critical decisions.
- Frontend must gate gameplay until required VRF state accounts are fulfilled on ER.

### Rule 3: E2E test coverage for on-chain changes

Every on-chain change that affects instruction signatures, account layouts, or session lifecycle **must** have a corresponding E2E test in `tests/e2e/`. Tests live alongside the programs in this repo and exercise the full flow: account initialization → session start → delegation → ER gameplay → undelegation → settlement → session end.

**Implication:** Do not merge on-chain changes without an E2E test that exercises the new or modified instruction. Use the existing helpers in `tests/e2e/shared/` (PDA derivation, delegation, base-layer send). If a new instruction is added, add a test step that calls it with the correct signer and validates the resulting account state.

### Rule 4: Frontend sync after verified on-chain changes

The frontend app at `../app/` must be updated to match any on-chain changes — but **only after** the on-chain code compiles, passes `cargo test --workspace`, and the E2E tests are updated. Never update the frontend speculatively before verifying the programs work.

Changes to propagate:
- Instruction argument changes → update transaction builders in `src/services/solana/`
- Account struct changes (added/removed accounts, signer changes) → update the corresponding builder accounts
- New instructions → add builder function + integrate into the appropriate hook in `src/hooks/`
- Removed accounts from session start → update `useSessionManager.ts`

<!-- MANUAL ADDITIONS END -->
