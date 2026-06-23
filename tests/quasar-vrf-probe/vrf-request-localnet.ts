import { expect } from "chai";
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { loadWalletKeypair } from "../e2e/shared/setup";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const ER_RPC_URL =
  process.env.EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";
const PROGRAM_ID = new PublicKey(
  "5s8adsH9jmPrSZRSRyDu5vBXp3X5ySm5SXg673Yvrm94",
);
const VRF_PROGRAM_ID = new PublicKey(
  "Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz",
);
const ER_ORACLE_QUEUE = new PublicKey(
  process.env.VRF_EPHEMERAL_QUEUE ||
    "5hBR571xnXppuCPveTrctfTU7tJLSN94nq7kv7FRK5Tc",
);
const DELEGATION_PROGRAM = new PublicKey(
  "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh",
);
const MAGIC_CONTEXT = new PublicKey(
  "MagicContext1111111111111111111111111111111",
);
const MAGIC_PROGRAM = new PublicKey(
  "Magic11111111111111111111111111111111111111",
);
const VALIDATOR = new PublicKey(
  process.env.VALIDATOR || "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev",
);

const VRF_PROBE_DISC = Buffer.from([219, 84, 237, 45, 14, 99, 103, 11]);
const IX = {
  initializeProbe: Buffer.from([1, 0, 0, 0, 0, 0, 0, 0]),
  requestVrf: Buffer.from([2, 0, 0, 0, 0, 0, 0, 0]),
  delegateProbe: Buffer.from([4, 0, 0, 0, 0, 0, 0, 0]),
  undelegateProbe: Buffer.from([5, 0, 0, 0, 0, 0, 0, 0]),
  requestVrfFixed: Buffer.from([6, 0, 0, 0, 0, 0, 0, 0]),
  ping: Buffer.from([7, 0, 0, 0, 0, 0, 0, 0]),
};

type ProbeState = {
  owner: PublicKey;
  requestCount: bigint;
  fulfilledCount: bigint;
  lastRequestSeed: Buffer;
  lastRandomness: Buffer;
  lastDieRoll: number;
  callbackVerified: boolean;
  bump: number;
};

function meta(pubkey: PublicKey, isSigner = false, isWritable = false) {
  return { pubkey, isSigner, isWritable };
}

function quasarIx(
  discriminator: Buffer,
  keys: TransactionInstruction["keys"],
  data: Buffer[] = [],
): TransactionInstruction {
  return new TransactionInstruction({
    programId: PROGRAM_ID,
    keys,
    data: Buffer.concat([discriminator, ...data]),
  });
}

function decodeProbe(data: Buffer): ProbeState {
  expect(data.subarray(0, 8).equals(VRF_PROBE_DISC)).to.equal(true);
  return {
    owner: new PublicKey(data.subarray(8, 40)),
    requestCount: data.readBigUInt64LE(40),
    fulfilledCount: data.readBigUInt64LE(48),
    lastRequestSeed: data.subarray(56, 88),
    lastRandomness: data.subarray(88, 120),
    lastDieRoll: data[120],
    callbackVerified: data[121] !== 0,
    bump: data[122],
  };
}

async function airdrop(
  connection: Connection,
  pubkey: PublicKey,
  sol = 2,
): Promise<void> {
  const balance = await connection.getBalance(pubkey, "confirmed");
  if (balance >= sol * LAMPORTS_PER_SOL) {
    return;
  }
  const sig = await connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
  const latest = await connection.getLatestBlockhash("confirmed");
  await connection.confirmTransaction(
    { signature: sig, ...latest },
    "confirmed",
  );
}

async function sendTx(
  connection: Connection,
  payer: Keypair,
  label: string,
  instructions: TransactionInstruction[],
  skipPreflight = false,
): Promise<string> {
  const latest = await connection.getLatestBlockhash("confirmed");
  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    ...instructions,
  );
  tx.feePayer = payer.publicKey;
  tx.recentBlockhash = latest.blockhash;
  tx.sign(payer);

  if (process.env.LOG_SIMULATION === "1") {
    try {
      const simulation = await connection.simulateTransaction(tx, [payer]);
      if (simulation.value.err || process.env.LOG_SIMULATION === "1") {
        console.log(
          `[quasar-vrf-probe] ${label} simulation err=${JSON.stringify(
            simulation.value.err,
          )}`,
        );
        for (const log of simulation.value.logs ?? []) {
          console.log(`[quasar-vrf-probe] ${label} sim-log ${log}`);
        }
      }
    } catch (error) {
      console.log(
        `[quasar-vrf-probe] ${label} simulation unavailable: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
  }

  let sig: string;
  try {
    sig = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight,
      preflightCommitment: "confirmed",
    });
  } catch (error) {
    const maybeLogs = (error as { logs?: string[] })?.logs;
    for (const log of maybeLogs ?? []) {
      console.log(`[quasar-vrf-probe] ${label} send-log ${log}`);
    }
    throw new Error(
      `${label} send failed: ${
        error instanceof Error ? error.message : JSON.stringify(error)
      }`,
    );
  }
  const confirmation = await connection.confirmTransaction(
    { signature: sig, ...latest },
    "confirmed",
  );
  const meta = await connection.getTransaction(sig, {
    commitment: "confirmed",
    maxSupportedTransactionVersion: 0,
  });
  console.log(
    `[quasar-vrf-probe] ${label} sig=${sig} cu=${meta?.meta?.computeUnitsConsumed ?? "unavailable"}`,
  );
  const err = confirmation.value.err ?? meta?.meta?.err;
  if (err) {
    for (const log of meta?.meta?.logMessages ?? []) {
      console.log(`[quasar-vrf-probe] ${label} log ${log}`);
    }
    throw new Error(`${label} failed: ${JSON.stringify(err)}`);
  }
  return sig;
}

async function waitForFulfillment(
  connection: Connection,
  probePda: PublicKey,
  minFulfilledCount: bigint,
): Promise<ProbeState> {
  const timeoutMs = Number(process.env.VRF_CALLBACK_TIMEOUT_MS || 15_000);
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const info = await connection.getAccountInfo(probePda, "confirmed");
    if (info) {
      const state = decodeProbe(info.data);
      if (state.fulfilledCount >= minFulfilledCount && state.callbackVerified) {
        return state;
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`VRF callback not observed within ${timeoutMs}ms`);
}

async function waitForBaseCommit(
  connection: Connection,
  probePda: PublicKey,
  minFulfilledCount: bigint,
): Promise<ProbeState> {
  const timeoutMs = Number(process.env.UNDELEGATE_TIMEOUT_MS || 20_000);
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const info = await connection.getAccountInfo(probePda, "confirmed");
    if (info?.owner.equals(PROGRAM_ID)) {
      const state = decodeProbe(info.data);
      if (state.fulfilledCount >= minFulfilledCount && state.callbackVerified) {
        return state;
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Base commit not observed within ${timeoutMs}ms`);
}

describe("quasar-vrf-probe localnet", () => {
  const baseConnection = new Connection(RPC_URL, "confirmed");
  const erConnection = new Connection(ER_RPC_URL, "confirmed");
  const payer = loadWalletKeypair();
  const [probePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vrf_probe"), payer.publicKey.toBuffer()],
    PROGRAM_ID,
  );
  const [programIdentity] = PublicKey.findProgramAddressSync(
    [Buffer.from("identity")],
    PROGRAM_ID,
  );
  const [bufferPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("buffer"), probePda.toBuffer()],
    PROGRAM_ID,
  );
  const [delegationRecord] = PublicKey.findProgramAddressSync(
    [Buffer.from("delegation"), probePda.toBuffer()],
    DELEGATION_PROGRAM,
  );
  const [delegationMetadata] = PublicKey.findProgramAddressSync(
    [Buffer.from("delegation-metadata"), probePda.toBuffer()],
    DELEGATION_PROGRAM,
  );

  it("delegates, requests MagicBlock scoped VRF, observes callback, and undelegates", async () => {
    await airdrop(baseConnection, payer.publicKey);

    const existing = await baseConnection.getAccountInfo(probePda, "confirmed");
    if (!existing) {
      await sendTx(baseConnection, payer, "initialize_probe", [
        quasarIx(IX.initializeProbe, [
          meta(payer.publicKey, true, true),
          meta(probePda, false, true),
          meta(SYSVAR_RENT_PUBKEY),
          meta(SystemProgram.programId),
        ]),
      ]);
    }

    const beforeInfo = await baseConnection.getAccountInfo(
      probePda,
      "confirmed",
    );
    expect(beforeInfo, "probe account must exist").to.not.equal(null);
    const before = decodeProbe(beforeInfo!.data);

    await sendTx(baseConnection, payer, "delegate_probe", [
      quasarIx(IX.delegateProbe, [
        meta(payer.publicKey, true, true),
        meta(probePda, false, true),
        meta(PROGRAM_ID),
        meta(bufferPda, false, true),
        meta(delegationRecord, false, true),
        meta(delegationMetadata, false, true),
        meta(SystemProgram.programId),
        meta(DELEGATION_PROGRAM),
        meta(VALIDATOR),
      ]),
    ]);

    const delegatedInfo = await baseConnection.getAccountInfo(
      probePda,
      "confirmed",
    );
    expect(delegatedInfo?.owner.equals(DELEGATION_PROGRAM)).to.equal(true);

    const callerSeed = Buffer.alloc(32, 7);
    await sendTx(erConnection, payer, "ping", [quasarIx(IX.ping, [])], true);

    await sendTx(
      erConnection,
      payer,
      "request_vrf",
      [
        quasarIx(IX.requestVrfFixed, [
          meta(payer.publicKey, true, true),
          meta(probePda, false, true),
          meta(ER_ORACLE_QUEUE, false, true),
          meta(programIdentity),
          meta(VRF_PROGRAM_ID),
          meta(SYSVAR_SLOT_HASHES_PUBKEY),
          meta(SystemProgram.programId),
        ]),
      ],
      true,
    );

    const requestedInfo = await erConnection.getAccountInfo(
      probePda,
      "confirmed",
    );
    expect(requestedInfo, "probe account must remain readable").to.not.equal(
      null,
    );
    const requested = decodeProbe(requestedInfo!.data);
    expect(requested.requestCount >= before.requestCount + 1n).to.equal(true);
    expect(requested.lastRequestSeed.equals(callerSeed)).to.equal(true);

    const fulfilled = await waitForFulfillment(
      erConnection,
      probePda,
      before.fulfilledCount + 1n,
    );
    expect(fulfilled.owner.equals(payer.publicKey)).to.equal(true);
    expect(fulfilled.lastDieRoll).to.be.within(1, 6);

    await sendTx(
      erConnection,
      payer,
      "undelegate_probe",
      [
        quasarIx(IX.undelegateProbe, [
          meta(payer.publicKey, true),
          meta(probePda, false, true),
          meta(MAGIC_CONTEXT, false, true),
          meta(MAGIC_PROGRAM),
        ]),
      ],
      true,
    );

    const committed = await waitForBaseCommit(
      baseConnection,
      probePda,
      before.fulfilledCount + 1n,
    );
    expect(committed.lastDieRoll).to.equal(fulfilled.lastDieRoll);
  });
});
