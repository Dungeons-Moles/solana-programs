# Quasar VRF Pinocchio Refresh Report - 2026-06-17

## Decision

**Decision category: Reopen ER-core.**

Reopen only through the next scoped batch: **Batch 5 - ER-core Quasar
migration pilot**. Batch 4 does not migrate `session-manager`,
`map-generator`, `player-inventory`, `poi-system`, or `gameplay-state`.

MagicBlock PR #96 is enough to prove the Quasar adapter mechanics: scoped VRF request bytes, PDA contract, signed CPI shape, callback signer verification, delegation, callback state mutation, and commit/undelegate all work from a throwaway Quasar program when run against MagicBlock's serviceable local ER queue.

On 2026-06-23 MagicBlock clarified that the local VRF setup services `Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT`. That makes the earlier strict-local `5hBR...` expectation invalid for local harnesses. Production/devnet D&M VRF remains pinned to `5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc`; the local `Sc9M...` queue is only the local oracle stand-in.

This report does not approve speculative migration of `session-manager`, `map-generator`, `player-inventory`, `poi-system`, or `gameplay-state` inside Batch 4. It clears the adapter/localnet blocker so the next batch can plan and test ER-core migration deliberately.

## Sources Reviewed

- MagicBlock PR #96: `magicblock-engine-examples/pinocchio-roll-dice`
- `ephemeral-rollups-pinocchio` 0.15.5 local source:
  - `src/vrf/consts.rs`
  - `src/vrf/pda.rs`
  - `src/vrf/types.rs`
  - `src/vrf/instruction.rs`
- MagicBlock packaged local dumps from `@magicblock-labs/ephemeral-validator@0.12.3`

## Implemented Spike Surface

- Added `er-compat` feature split:
  - default `anchor-shim` keeps the existing Anchor ER shim.
  - `quasar-vrf` builds the no_std Quasar VRF adapter without Anchor.
- Added `crates/er-compat/src/quasar_vrf.rs`:
  - pinned VRF program id `Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz`
  - pinned D&M ER queue `5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc`
  - explicit local-only queue constant `Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT`
  - scoped request discriminators `10` and `11`
  - `[b"identity"]` callback-program PDA request signer
  - `[b"identity", callback_program_id]` scoped VRF callback signer verifier
  - manual `RequestRandomness` serialization matching MagicBlock Pinocchio output
  - Quasar `AccountView` to Pinocchio `AccountView` bridge for delegation/commit helpers
- Added throwaway `programs/quasar-vrf-probe/`:
  - initialize, delegate, request VRF, callback, commit/undelegate
  - one-pointer SBF entrypoint compatibility shim for MagicBlock ER
  - feature-gated `local-vrf-queue` mode for the MagicBlock local queue only
- Added `tests/quasar-vrf-probe/vrf-request-localnet.ts` and `run-localnet.sh`.

## D&M Source Files Used

- `CLAUDE.md`
- `.specs/features/batch-4-quasar-vrf-adapter/spec.md`
- `.specs/features/batch-4-quasar-vrf-adapter/tasks.md`
- `crates/er-compat/src/lib.rs`
- `crates/er-compat/src/quasar_vrf.rs`
- `programs/quasar-vrf-probe/Cargo.toml`
- `programs/quasar-vrf-probe/src/lib.rs`
- `programs/quasar-vrf-probe/src/state.rs`
- `tests/quasar-vrf-probe/run-localnet.sh`
- `tests/quasar-vrf-probe/vrf-request-localnet.ts`
- `target/quasar-vrf-probe-localnet/vrf-oracle-er.log`
- `Tasks/Batch 3/T03b - Quasar VRF adapter spike.md`
- `Tasks/Batch 3/T04 - Go-NoGo report.md`
- `Tasks/Batch 4/T06 - ER localnet proof.md`
- `Tasks/Batch 4/T07 - Security review.md`

## Verification

Passing adapter contract tests:

```bash
cargo test -p er-compat --no-default-features --features quasar-vrf
```

Result: 9/9 tests passed. These compare constants, scoped/high-priority request bytes, empty metas/args, PDA derivations, account order, callback decoding, and scoped identity behavior against `ephemeral-rollups-pinocchio`.

Close recheck output on 2026-06-23:

```text
running 9 tests
...
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Passing Quasar probe build checks:

```bash
cargo check -p quasar-vrf-probe --features local-vrf-queue
```

Result: passed with Quasar macro `unexpected cfg(solana)` warnings.

Close recheck output on 2026-06-23:

```text
warning: `quasar-vrf-probe` (lib) generated 11 warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
```

```bash
cargo build-sbf --manifest-path programs/quasar-vrf-probe/Cargo.toml --features local-vrf-queue --sbf-out-dir target/deploy
```

Result: passed on 2026-06-17 and reran successfully on 2026-06-23.

Close recheck output on 2026-06-23:

```text
Finished `release` profile [optimized] target(s) in 6.25s
```

Size:

```text
target/deploy/quasar_vrf_probe.so 88984 bytes (2026-06-23 local-vrf-queue rebuild)
```

MagicBlock upstream control:

```bash
systemd-run --user --pipe --wait --collect --property=LimitNOFILE=1000000 \
  --working-directory=/tmp/magicblock-engine-examples \
  /usr/bin/env PATH=/tmp/dm-mb-tools/node_modules/.bin:/tmp/dm-agave-3.1.9/active_release/bin:$PATH \
  EXACT_MATCH=1 SKIP_REGULAR_TESTS=1 SKIP_TEE_TESTS=1 FAIL_FAST=1 \
  /tmp/magicblock-engine-examples/scripts/test-locally.sh pinocchio-roll-dice
```

Result: passed 1/1. Built SBF sizes:

```text
roll_dice.so 36840 bytes
roll_dice_delegated.so 73888 bytes
```

Quasar local-queue lifecycle control:

```bash
systemd-run --user --pipe --wait --collect --property=LimitNOFILE=1000000 \
  --working-directory=/home/ailton/Work/dungeons-and-moles/solana-programs \
  /usr/bin/env PATCH_LOCAL_5HBR_QUEUE=0 \
  VRF_EPHEMERAL_QUEUE=Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT \
  CLIENT_EPHEMERAL_PROVIDER_ENDPOINT=http://127.0.0.1:7799 \
  bash tests/quasar-vrf-probe/run-localnet.sh
```

Result: passed 1/1 on 2026-06-17 and again on 2026-06-23 after MagicBlock confirmed `Sc9M...` as the local ER oracle queue.

Close recheck output on 2026-06-23:

```text
[quasar-vrf-probe] initialize_probe sig=62TLTNZoJpDkR6gFNh72ZJFo6zpeneYwYqdKaMsyvDvXeVi3573DTZ1xedH5g7Nw5eHEVdKM9qCBYTitpr94xig cu=1996
[quasar-vrf-probe] delegate_probe sig=3FmSqHxSs7zHxT6j27c3o9ELToaaRu1xoPf9vAMJAABrp55CZAsEA8PGpfSTM4da84mnHmTAK34PsPSxe112Pgj3 cu=27532
[quasar-vrf-probe] ping sig=wFwmjSCVBGwtzxPHRmvcTQpeyUu77FuYBXUjtBFZ4iGuVgfgMAwcfWnqkKf5zCYABuRQ9FcXRmxyuuGns952VH9 cu=293
[quasar-vrf-probe] request_vrf sig=2qEkp6vxGz8kWLeJ9ZPkESigyWWL1m9UYhmBsFppaFmwzSGeipvvFJM597nkpQjkgbGcfyTRP3Z3RofqcJQXSXz2 cu=19453
[quasar-vrf-probe] undelegate_probe sig=2jV64r2zNpL9xk7G5aktvbgEyYyoyXJarWayyDACooGd7bg6rEmAyUpXuRBDSG5stR2bc6zatfQaFokscwnJuktY cu=23853
1 passing (4s)
Finished with result: success
```

Observed CU:

```text
initialize_probe 1996 CU
delegate_probe 27532 CU
ping 293 CU
request_vrf 19453 CU
undelegate_probe 23853 CU
```

Oracle evidence:

```text
[2026-06-23T09:08:27Z INFO  vrf_oracle::oracle::processor] Processing queue: Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT, with len: 53
[2026-06-23T09:08:31Z INFO  vrf_oracle::oracle::processor] Processing queue: Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT, with len: 54
```

The TS test observed `fulfilled_count >= before + 1`, `callback_verified = true`, a die roll in `1..=6`, and successful commit/undelegate back to base.

## Local Queue Clarification

Earlier strict D&M queue command:

```bash
systemd-run --user --pipe --wait --collect --property=LimitNOFILE=1000000 \
  --working-directory=/home/ailton/Work/dungeons-and-moles/solana-programs \
  /usr/bin/env bash tests/quasar-vrf-probe/run-localnet.sh
```

Historical result: blocked. The Quasar program initializes, delegates, pings on ER, and submits the VRF request to `5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc`, but the local ER oracle never processes that queue and the callback is not observed within 60s.

Observed CU before timeout:

```text
initialize_probe 1996 CU
delegate_probe 27554 CU
ping 293 CU
request_vrf 18460 CU
```

Attempts to make the local `5hBR...` account serviceable:

- Duplicate `--account 5hBR... patched.json` with `mb-test-validator` did not override the packaged dump.
- Direct `solana-test-validator` startup with one patched `5hBR...` dump still did not make the ER oracle process `5hBR...`.
- The packaged local oracle processes `Sc9M...` on ER and `GKE6...` on base; it does not process `5hBR...` in this local topology.

MagicBlock later confirmed the local behavior: local VRF oracle setup uses `Sc9M...`. Therefore the strict local `5hBR...` timeout is not an adapter blocker. It remains true that production/devnet D&M VRF must use `5hBR...`.

## What Works Now

- MagicBlock's Pinocchio/no_std VRF reference works locally with the current tooling.
- A Quasar program can use the adapter to issue scoped MagicBlock VRF requests.
- Quasar callback dispatch works on MagicBlock ER after replacing the two-argument Quasar SBF entrypoint with a one-pointer compatibility entrypoint.
- The callback signer verifies against the scoped `[b"identity", callback_program_id]` PDA.
- Delegation and commit/undelegate work through the Pinocchio helpers from Quasar account views.
- Raw delegated account handling must use unchecked/manual byte access in the throwaway probe; typed owner-checked Quasar accounts do not survive the delegation owner swap.

## What Does Not Work Yet

- Batch 4 does not yet migrate any ER-core production program.
- The next ER-core migration batch still needs program-specific E2E coverage across delegation, session-key signing, VRF request/fulfillment, undelegate, settlement, and frontend sync.
- Devnet/mainnet proof must use `5hBR...`; local harnesses use `Sc9M...`.

## Security Notes

- Default adapter helpers still pin `ER_ORACLE_QUEUE`.
- The explicit queue helper is documented for local harness use; production code should not call it.
- The deprecated global VRF identity is retained only as a comparison constant; callbacks verify scoped identity.
- Callback discriminator bytes are explicit in the fixture.
- The adapter does not transmute Quasar accounts into Anchor accounts.
- Unsafe account conversion is limited to the Quasar `AccountView` -> Pinocchio `AccountView` bridge needed for MagicBlock's Pinocchio helpers.
- Production use still requires a dedicated review before importing this adapter into ER-core programs.

## Recommendation

Close Batch 4 as adapter/localnet proof unblocked. Plan **Batch 5 - ER-core Quasar migration pilot** next; do not sneak production ER program edits into Batch 4.

Next narrow task:

1. Convert one ER-core program in Batch 5, starting with the smallest VRF/delegation surface.
2. Keep production/default helpers pinned to `5hBR...`; use a feature-gated `Sc9M...` local harness only for local oracle tests.
3. Add E2E coverage for the full session lifecycle before frontend sync.
