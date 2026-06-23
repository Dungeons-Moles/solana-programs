#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

MB_TOOLS_BIN="${MB_TOOLS_BIN:-/tmp/dm-mb-tools/node_modules/.bin}"
AGAVE_BIN="${AGAVE_BIN:-/tmp/dm-agave-3.1.9/active_release/bin}"
export PATH="$MB_TOOLS_BIN:$AGAVE_BIN:$PATH"

LOG_DIR="${LOG_DIR:-$ROOT_DIR/target/quasar-vrf-probe-localnet}"
WALLET_KP="${WALLET_KP:-$HOME/.config/solana/id.json}"
VALIDATOR="${VALIDATOR:-mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev}"
PROGRAM_KEYPAIR="$ROOT_DIR/target/deploy/quasar_vrf_probe-keypair.json"
PROGRAM_SO="$ROOT_DIR/target/deploy/quasar_vrf_probe.so"
ER_ORACLE_QUEUE="5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc"
LOCAL_SERVICEABLE_ER_QUEUE="Sc9MJUngNbQXSXGP3F67KvKwVnhaYn6kcioxXNVowYT"

mkdir -p "$LOG_DIR"

PIDS=()

cleanup() {
  local pid
  for pid in "${PIDS[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill -- "-$pid" 2>/dev/null || kill "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

require_bin() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required binary: $1"
    exit 1
  fi
}

wait_for_tcp() {
  local name="$1"
  local host="$2"
  local port="$3"
  local pid="$4"
  local timeout="${5:-60}"
  local i
  for ((i = 1; i <= timeout; i++)); do
    if (echo >"/dev/tcp/$host/$port") 2>/dev/null; then
      sleep 1
      echo "$name is ready on $host:$port"
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "$name exited before binding $host:$port"
      return 1
    fi
    sleep 1
  done
  echo "$name did not bind $host:$port within ${timeout}s"
  return 1
}

wait_for_base_slot() {
  local pid="$1"
  local timeout="${SOLANA_READY_TIMEOUT:-90}"
  local i slot
  for ((i = 1; i <= timeout; i++)); do
    if ! kill -0 "$pid" 2>/dev/null; then
      echo "mb-test-validator exited before producing slots"
      return 1
    fi
    slot="$(
      curl -s --max-time 1 -X POST -H "content-type: application/json" \
        -d '{"jsonrpc":"2.0","method":"getSlot","params":[{"commitment":"processed"}],"id":1}' \
        http://127.0.0.1:8899 2>/dev/null \
        | sed -nE 's/.*"result":([0-9]+).*/\1/p'
    )"
    if [[ "$slot" =~ ^[0-9]+$ ]] && [ "$slot" -gt 0 ]; then
      echo "base validator is ready at slot $slot"
      return 0
    fi
    sleep 1
  done
  echo "base validator did not produce slots within ${timeout}s"
  return 1
}

print_log_tail() {
  local file="$1"
  local lines="${2:-120}"
  if [ -f "$file" ]; then
    echo "----- $file -----"
    tail -n "$lines" "$file"
    echo "----- end $file -----"
  fi
}

require_bin mb-test-validator
require_bin ephemeral-validator
require_bin query-filtering-service
require_bin vrf-oracle
require_bin solana
require_bin solana-keygen
require_bin curl
require_bin node

if [ ! -f "$PROGRAM_KEYPAIR" ] || [ ! -f "$PROGRAM_SO" ]; then
  echo "missing probe build artifacts under target/deploy"
  exit 1
fi

WALLET_PUBKEY="$(solana-keygen pubkey "$WALLET_KP")"

echo "Using MagicBlock tools from: $MB_TOOLS_BIN"
echo "Using Agave tools from: $AGAVE_BIN"
echo "Logs: $LOG_DIR"
echo "Preloading quasar_vrf_probe: $(solana-keygen pubkey "$PROGRAM_KEYPAIR")"

MB_TOOLS_NODE_MODULES="$(cd "$MB_TOOLS_BIN/.." && pwd)"
LOCAL_DUMPS_DIR="$MB_TOOLS_NODE_MODULES/@magicblock-labs/ephemeral-validator/bin/local-dumps"
BASE_VALIDATOR_CMD=(mb-test-validator)
BASE_VALIDATOR_ARGS=(
  --reset
  --ledger "$LOG_DIR/base-ledger"
  --upgradeable-program "$PROGRAM_KEYPAIR" "$PROGRAM_SO" "$WALLET_PUBKEY"
)

if [ "${PATCH_LOCAL_5HBR_QUEUE:-1}" = "1" ]; then
  PATCHED_QUEUE_DUMP="$LOG_DIR/5hBR-local-serviceable-queue.json"
  node - "$LOCAL_DUMPS_DIR/$LOCAL_SERVICEABLE_ER_QUEUE.json" "$PATCHED_QUEUE_DUMP" "$LOCAL_SERVICEABLE_ER_QUEUE" "$ER_ORACLE_QUEUE" <<'NODE'
const fs = require("fs");
const [source, dest, fromPubkey, toPubkey] = process.argv.slice(2);
const raw = fs.readFileSync(source, "utf8");
const patched = raw.replace(
  `"pubkey": "${fromPubkey}"`,
  `"pubkey": "${toPubkey}"`,
);
if (patched === raw) {
  throw new Error(`source dump did not contain pubkey ${fromPubkey}`);
}
fs.writeFileSync(dest, patched);
NODE
  BASE_VALIDATOR_CMD=(solana-test-validator)
  BASE_VALIDATOR_ARGS=(
    --reset
    --ledger "$LOG_DIR/base-ledger"
    --bpf-program DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh "$LOCAL_DUMPS_DIR/DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh.so"
    --bpf-program noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV "$LOCAL_DUMPS_DIR/noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV.so"
    --bpf-program Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz "$LOCAL_DUMPS_DIR/Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz.so"
    --bpf-program ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1 "$LOCAL_DUMPS_DIR/ACLseoPoyC3cBqoUtkbjZ4aDrkurZW86v19pXz2XQnp1.so"
    --bpf-program SPLxh1LVZzEkX99H6rqYizhytLWPZVV296zyYDPagv2 "$LOCAL_DUMPS_DIR/SPLxh1LVZzEkX99H6rqYizhytLWPZVV296zyYDPagv2.so"
    --bpf-program EnhkomtzKms55jXi3ijn9XsMKYpMT4BJjmbuDQmPo3YS "$LOCAL_DUMPS_DIR/EnhkomtzKms55jXi3ijn9XsMKYpMT4BJjmbuDQmPo3YS.so"
    --bpf-program DmnRGfyyftzacFb1XadYhWF6vWqXwtQk5tbr6XgR3BA1 "$LOCAL_DUMPS_DIR/DmnRGfyyftzacFb1XadYhWF6vWqXwtQk5tbr6XgR3BA1.so"
    --bpf-program KeyspM2ssCJbqUhQ4k7sveSiY4WjnYsrXkC8oDbwde5 "$LOCAL_DUMPS_DIR/KeyspM2ssCJbqUhQ4k7sveSiY4WjnYsrXkC8oDbwde5.so"
    --account mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev "$LOCAL_DUMPS_DIR/mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev.json"
    --account EpJnX7ueXk7fKojBymqmVuCuwyhDQsYcLVL1XMsBbvDX "$LOCAL_DUMPS_DIR/EpJnX7ueXk7fKojBymqmVuCuwyhDQsYcLVL1XMsBbvDX.json"
    --account 7JrkjmZPprHwtuvtuGTXp9hwfGYFAQLnLeFM52kqAgXg "$LOCAL_DUMPS_DIR/7JrkjmZPprHwtuvtuGTXp9hwfGYFAQLnLeFM52kqAgXg.json"
    --account Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh "$LOCAL_DUMPS_DIR/Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh.json"
    --account "$ER_ORACLE_QUEUE" "$PATCHED_QUEUE_DUMP"
    --account F72HqCR8nwYsVyeVd38pgKkjXmXFzVAM8rjZZsXWbdE "$LOCAL_DUMPS_DIR/F72HqCR8nwYsVyeVd38pgKkjXmXFzVAM8rjZZsXWbdE.json"
    --account paywJiVATrVDLYLmowJqzG6MsaCt77L8WyTnBb2754t "$LOCAL_DUMPS_DIR/paywJiVATrVDLYLmowJqzG6MsaCt77L8WyTnBb2754t.json"
    --account CXMc1eCiEp9YXjanBNB6HUvbWCmxeVmhcR3bPXw8exJA "$LOCAL_DUMPS_DIR/CXMc1eCiEp9YXjanBNB6HUvbWCmxeVmhcR3bPXw8exJA.json"
    --account GKE6d7iv8kCBrsxr78W3xVdjGLLLJnxsGiuzrsZCGEvb "$LOCAL_DUMPS_DIR/GKE6d7iv8kCBrsxr78W3xVdjGLLLJnxsGiuzrsZCGEvb.json"
    --account FRqXJqfCi3o6gF3Yqnkx1gKA3YnbRDJbBs6hKpme3NHJ "$LOCAL_DUMPS_DIR/FRqXJqfCi3o6gF3Yqnkx1gKA3YnbRDJbBs6hKpme3NHJ.json"
    --account "$LOCAL_SERVICEABLE_ER_QUEUE" "$LOCAL_DUMPS_DIR/$LOCAL_SERVICEABLE_ER_QUEUE.json"
    --account 9yvg9551MmE8mhWd88jAPLE3noTXHoopYG1BDhmtkCeR "$LOCAL_DUMPS_DIR/9yvg9551MmE8mhWd88jAPLE3noTXHoopYG1BDhmtkCeR.json"
    --account 7L9eCRv52UpGVePGj9P1zop8kzmh4SpYzYn6YhoAKHBg "$LOCAL_DUMPS_DIR/7L9eCRv52UpGVePGj9P1zop8kzmh4SpYzYn6YhoAKHBg.json"
    --upgradeable-program "$PROGRAM_KEYPAIR" "$PROGRAM_SO" "$WALLET_PUBKEY"
  )
  echo "Patched local $ER_ORACLE_QUEUE queue dump from $LOCAL_SERVICEABLE_ER_QUEUE for oracle serviceability"
fi

setsid "${BASE_VALIDATOR_CMD[@]}" \
  "${BASE_VALIDATOR_ARGS[@]}" \
  >"$LOG_DIR/base-validator.log" 2>&1 < /dev/null &
BASE_PID=$!
PIDS+=("$BASE_PID")

if ! wait_for_base_slot "$BASE_PID"; then
  print_log_tail "$LOG_DIR/base-validator.log"
  exit 1
fi

KEEP_SOL="${ER_FEE_PAYER_KEEP_SOL:-1000}"
INCINERATOR="1nc1nerator11111111111111111111111111111111"
BAL_LAMPORTS="$(solana balance --lamports --url http://127.0.0.1:8899 --keypair "$WALLET_KP" 2>/dev/null | awk '{print $1}')"
KEEP_LAMPORTS=$((KEEP_SOL * 1000000000))
if [[ "$BAL_LAMPORTS" =~ ^[0-9]+$ ]] && [ "$BAL_LAMPORTS" -gt "$KEEP_LAMPORTS" ]; then
  SEND_SOL=$(((BAL_LAMPORTS - KEEP_LAMPORTS) / 1000000000))
  echo "Capping base fee-payer balance, keeping about $KEEP_SOL SOL"
  solana transfer "$INCINERATOR" "$SEND_SOL" \
    --allow-unfunded-recipient \
    --url http://127.0.0.1:8899 \
    --keypair "$WALLET_KP" \
    --fee-payer "$WALLET_KP" \
    >/dev/null
fi

setsid ephemeral-validator \
  --no-tui \
  --lifecycle ephemeral \
  --remotes http://127.0.0.1:8899 \
  --remotes ws://127.0.0.1:8900 \
  --listen 127.0.0.1:7799 \
  --storage "$LOG_DIR/er-storage" \
  --reset \
  >"$LOG_DIR/ephemeral-validator.log" 2>&1 < /dev/null &
ER_PID=$!
PIDS+=("$ER_PID")

if ! wait_for_tcp "ephemeral-validator" 127.0.0.1 7799 "$ER_PID" 60; then
  print_log_tail "$LOG_DIR/ephemeral-validator.log"
  exit 1
fi

setsid query-filtering-service \
  --listen-addr 127.0.0.1:6699 \
  --listen-addr-ws 127.0.0.1:6700 \
  --ephemeral-url http://127.0.0.1:7799 \
  --ephemeral-url-ws ws://127.0.0.1:7800 \
  --token-expiry-days 180 \
  --add-cors-headers \
  >"$LOG_DIR/query-filtering-service.log" 2>&1 < /dev/null &
QFS_PID=$!
PIDS+=("$QFS_PID")

if ! wait_for_tcp "query-filtering-service" 127.0.0.1 6699 "$QFS_PID" 60; then
  print_log_tail "$LOG_DIR/query-filtering-service.log"
  exit 1
fi

setsid env \
  VRF_ORACLE_SKIP_PREFLIGHT=true \
  RPC_URL=http://127.0.0.1:8899 \
  WEBSOCKET_URL=ws://127.0.0.1:8900 \
  RUST_LOG=info \
  vrf-oracle \
  >"$LOG_DIR/vrf-oracle-base.log" 2>&1 < /dev/null &
VRF_BASE_PID=$!
PIDS+=("$VRF_BASE_PID")
sleep 2
if ! kill -0 "$VRF_BASE_PID" 2>/dev/null; then
  print_log_tail "$LOG_DIR/vrf-oracle-base.log"
  exit 1
fi

setsid env \
  VRF_ORACLE_SKIP_PREFLIGHT=true \
  RPC_URL=http://127.0.0.1:7799 \
  WEBSOCKET_URL=ws://127.0.0.1:7800 \
  RUST_LOG=info \
  vrf-oracle \
  >"$LOG_DIR/vrf-oracle-er.log" 2>&1 < /dev/null &
VRF_ER_PID=$!
PIDS+=("$VRF_ER_PID")
sleep 2
if ! kill -0 "$VRF_ER_PID" 2>/dev/null; then
  print_log_tail "$LOG_DIR/vrf-oracle-er.log"
  exit 1
fi

echo "Local MagicBlock stack is ready. Running Quasar VRF probe..."

ANCHOR_PROVIDER_URL=http://127.0.0.1:8899 \
ANCHOR_WALLET="$WALLET_KP" \
EPHEMERAL_PROVIDER_ENDPOINT="${CLIENT_EPHEMERAL_PROVIDER_ENDPOINT:-http://127.0.0.1:7799}" \
VALIDATOR="$VALIDATOR" \
VRF_CALLBACK_TIMEOUT_MS="${VRF_CALLBACK_TIMEOUT_MS:-60000}" \
UNDELEGATE_TIMEOUT_MS="${UNDELEGATE_TIMEOUT_MS:-60000}" \
./node_modules/.bin/ts-mocha -p ./tsconfig.json -t 1000000 --exit \
  tests/quasar-vrf-probe/vrf-request-localnet.ts
