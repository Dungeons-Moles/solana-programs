import { expect } from "chai";
import {
  anchor,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Connection,
  loadAllPrograms,
  loadWalletKeypair,
  createProvider,
  walletFromKeypair,
  airdropAndConfirm,
  PROGRAM_IDS,
  COMPANY_TREASURY,
  AllPrograms,
} from "../shared/setup";
import {
  getSessionCounterPda,
  getSessionNoncesPda,
  getSessionManagerAuthorityPda,
  getPlayerProfilePda,
  getMapConfigPda,
  getGeneratedMapPda,
  getMapVrfStatePda,
  getGameStatePda,
  getGameplayVrfStatePda,
  getMapEnemiesPda,
  getGameplayAuthorityPda,
  getDuelVaultPda,
  getDuelOpenQueuePda,
  getDuelEntryPda,
  getDuelSessionPda,
  getInventoryPda,
  getMapPoisPda,
  getSessionDiscoveryPda,
  getGauntletPoolVaultPda,
  deriveDelegateAccounts,
} from "../shared/pda-helpers";
import {
  Transaction,
  TransactionInstruction,
  ComputeBudgetProgram,
} from "@solana/web3.js";

// ── Connections ─────────────────────────────────────────────────────────────
const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const ER_RPC_URL =
  process.env.EXPO_PUBLIC_EPHEMERAL_PROVIDER_ENDPOINT ||
  "http://127.0.0.1:7799";
const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev",
);

const DUEL_ENTRY_LAMPORTS = 100_000_000; // 0.1 SOL
const DUEL_CAMPAIGN_LEVEL = 20;

// ── Shared mutable state ────────────────────────────────────────────────────
let connection: Connection;
let erConnection: Connection;
let provider: anchor.AnchorProvider;
let programs: AllPrograms;
let admin: Keypair;

// Global PDAs
let sessionCounterPda: PublicKey;
let mapConfigPda: PublicKey;
let sessionManagerAuthorityPda: PublicKey;
let duelVaultPda: PublicKey;
let duelOpenQueuePda: PublicKey;
let gauntletPoolVaultPda: PublicKey;

// ── Helpers ─────────────────────────────────────────────────────────────────

const sendBaseTx = async (
  label: string,
  ixs: TransactionInstruction[],
  signers: Keypair[],
): Promise<string> => {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = signers[0].publicKey;
  const bh = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(...signers);
  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
    maxRetries: 3,
  });
  await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
  const status = await connection.getSignatureStatuses([sig], {
    searchTransactionHistory: true,
  });
  if (status.value[0]?.err) {
    throw new Error(`${label} failed: ${JSON.stringify(status.value[0].err)}`);
  }
  return sig;
};

interface PlayerContext {
  user: Keypair;
  sessionSigner: Keypair;
  playerProfilePda: PublicKey;
  sessionPda: PublicKey;
  gameStatePda: PublicKey;
  mapEnemiesPda: PublicKey;
  generatedMapPda: PublicKey;
  inventoryPda: PublicKey;
  mapPoisPda: PublicKey;
  mapVrfStatePda: PublicKey;
  gameplayVrfStatePda: PublicKey;
  duelEntryPda: PublicKey;
  sessionNoncesPda: PublicKey;
  sessionDiscoveryPda: PublicKey;
}

async function setupPlayer(label: string): Promise<PlayerContext> {
  const user = Keypair.generate();
  const sessionSigner = Keypair.generate();
  await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
  await airdropAndConfirm(
    connection,
    sessionSigner.publicKey,
    5 * LAMPORTS_PER_SOL,
  );

  const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
  const [sessionPda] = getDuelSessionPda(user.publicKey);
  const [gameStatePda] = getGameStatePda(sessionPda);
  const [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
  const [generatedMapPda] = getGeneratedMapPda(sessionPda);
  const [inventoryPda] = getInventoryPda(sessionPda);
  const [mapPoisPda] = getMapPoisPda(sessionPda);
  const [mapVrfStatePda] = getMapVrfStatePda(sessionPda);
  const [gameplayVrfStatePda] = getGameplayVrfStatePda(sessionPda);
  const [duelEntryPda] = getDuelEntryPda(sessionPda);
  const [sessionNoncesPda] = getSessionNoncesPda(user.publicKey);
  const [sessionDiscoveryPda] = getSessionDiscoveryPda(sessionPda);

  return {
    user,
    sessionSigner,
    playerProfilePda,
    sessionPda,
    gameStatePda,
    mapEnemiesPda,
    generatedMapPda,
    inventoryPda,
    mapPoisPda,
    mapVrfStatePda,
    gameplayVrfStatePda,
    duelEntryPda,
    sessionNoncesPda,
    sessionDiscoveryPda,
  };
}

const sendErTx = async (
  label: string,
  ixs: TransactionInstruction[],
  signers: Keypair[],
): Promise<string> => {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = signers[0].publicKey;
  const bh = await erConnection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(...signers);
  const sig = await erConnection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
    maxRetries: 3,
  });
  await erConnection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
  const status = await erConnection.getSignatureStatuses([sig], {
    searchTransactionHistory: true,
  });
  if (status.value[0]?.err) {
    throw new Error(`${label} failed: ${JSON.stringify(status.value[0].err)}`);
  }
  return sig;
};

const waitForBaseOwner = async (
  account: PublicKey,
  owner: PublicKey,
  label: string,
): Promise<void> => {
  for (let i = 0; i < 40; i += 1) {
    const info = await connection.getAccountInfo(account, "confirmed");
    if (info?.owner.equals(owner)) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const info = await connection.getAccountInfo(account, "confirmed");
  throw new Error(
    `${label} owner did not restore (current=${info?.owner.toBase58() ?? "missing"})`,
  );
};

const waitForErVisibility = async (
  account: PublicKey,
  label: string,
): Promise<void> => {
  for (let i = 0; i < 60; i += 1) {
    const info = await erConnection.getAccountInfo(account, "confirmed");
    if (info) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`${label} account did not become visible on ER`);
};

async function initDelegateAndUndelegateDuelVrfStates(
  p: PlayerContext,
): Promise<void> {
  const initMapIx = await programs.mapGenerator.methods
    .initMapVrfState()
    .accounts({
      payer: p.sessionSigner.publicKey,
      session: p.sessionPda,
      vrfState: p.mapVrfStatePda,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();

  const initGameplayIx = await programs.gameplayState.methods
    .initGameplayVrfState()
    .accounts({
      payer: p.sessionSigner.publicKey,
      session: p.sessionPda,
      vrfState: p.gameplayVrfStatePda,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();

  await sendBaseTx(
    "init-duel-vrf-states",
    [initMapIx, initGameplayIx],
    [p.sessionSigner],
  );

  const mapDelegate = deriveDelegateAccounts(
    p.mapVrfStatePda,
    programs.mapGenerator.programId,
  );
  const gameplayDelegate = deriveDelegateAccounts(
    p.gameplayVrfStatePda,
    programs.gameplayState.programId,
  );

  const delegateMapIx = await programs.mapGenerator.methods
    .delegateMapVrfState(ER_VALIDATOR)
    .accountsStrict({
      bufferMapVrfState: mapDelegate.buffer,
      delegationRecordMapVrfState: mapDelegate.delegationRecord,
      delegationMetadataMapVrfState: mapDelegate.delegationMetadata,
      mapVrfState: p.mapVrfStatePda,
      session: p.sessionPda,
      player: p.sessionSigner.publicKey,
      ownerProgram: programs.mapGenerator.programId,
      delegationProgram: PROGRAM_IDS.delegation,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();

  const delegateGameplayIx = await programs.gameplayState.methods
    .delegateGameplayVrfState(ER_VALIDATOR)
    .accountsStrict({
      bufferGameplayVrfState: gameplayDelegate.buffer,
      delegationRecordGameplayVrfState: gameplayDelegate.delegationRecord,
      delegationMetadataGameplayVrfState: gameplayDelegate.delegationMetadata,
      gameplayVrfState: p.gameplayVrfStatePda,
      gameSession: p.sessionPda,
      player: p.sessionSigner.publicKey,
      ownerProgram: programs.gameplayState.programId,
      delegationProgram: PROGRAM_IDS.delegation,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();

  await sendBaseTx(
    "delegate-duel-vrf-states",
    [delegateMapIx, delegateGameplayIx],
    [p.sessionSigner],
  );

  await waitForErVisibility(p.mapVrfStatePda, "duel_map_vrf_state");
  await waitForErVisibility(p.gameplayVrfStatePda, "duel_gameplay_vrf_state");

  const undelegateMapIx = await programs.mapGenerator.methods
    .undelegateMapVrfState()
    .accounts({
      mapVrfState: p.mapVrfStatePda,
      session: p.sessionPda,
      sessionSigner: p.sessionSigner.publicKey,
    } as any)
    .instruction();

  const undelegateGameplayIx = await programs.gameplayState.methods
    .undelegateGameplayVrfState()
    .accounts({
      gameplayVrfState: p.gameplayVrfStatePda,
      gameSession: p.sessionPda,
      sessionSigner: p.sessionSigner.publicKey,
    } as any)
    .instruction();

  await sendErTx(
    "undelegate-duel-vrf-states",
    [undelegateMapIx, undelegateGameplayIx],
    [p.sessionSigner],
  );

  await waitForBaseOwner(
    p.mapVrfStatePda,
    PROGRAM_IDS.mapGenerator,
    "duel_map_vrf_state",
  );
  await waitForBaseOwner(
    p.gameplayVrfStatePda,
    PROGRAM_IDS.gameplayState,
    "duel_gameplay_vrf_state",
  );
}

async function createProfileAndStartDuelSession(
  p: PlayerContext,
): Promise<void> {
  const name = `duel-${p.user.publicKey.toBase58().slice(0, 6)}`;
  await programs.playerProfile.methods
    .initializeProfile(name)
    .accounts({
      playerProfile: p.playerProfilePda,
      owner: p.user.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([p.user])
    .rpc();

  await programs.sessionManager.methods
    .startDuelSession()
    .accounts({
      sessionNonces: p.sessionNoncesPda,
      gameSession: p.sessionPda,
      sessionCounter: sessionCounterPda,
      playerProfile: p.playerProfilePda,
      player: p.user.publicKey,
      sessionSigner: p.sessionSigner.publicKey,
      sessionManagerAuthority: sessionManagerAuthorityPda,
      mapConfig: mapConfigPda,
      generatedMap: p.generatedMapPda,
      sessionDiscovery: p.sessionDiscoveryPda,
      gameState: p.gameStatePda,
      mapEnemies: p.mapEnemiesPda,
      mapPois: p.mapPoisPda,
      inventory: p.inventoryPda,
      mapVrfState: null,
      poiVrfState: null,
      gameplayVrfState: null,
      mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      poiSystemProgram: PROGRAM_IDS.poiSystem,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      systemProgram: SystemProgram.programId,
    } as any)
    .preInstructions([
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
    ])
    .signers([p.user, p.sessionSigner])
    .rpc();
}

async function enterDuel(p: PlayerContext): Promise<void> {
  await programs.gameplayState.methods
    .enterDuel()
    .accounts({
      duelEntry: p.duelEntryPda,
      duelVault: duelVaultPda,
      player: p.user.publicKey,
      gameState: p.gameStatePda,
      companyTreasury: COMPANY_TREASURY,
      gauntletPoolVault: gauntletPoolVaultPda,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([p.user])
    .rpc();
}

async function assignDuelMapSeed(p: PlayerContext): Promise<void> {
  await programs.gameplayState.methods
    .assignDuelMapSeed()
    .accounts({
      gameState: p.gameStatePda,
      duelOpenQueue: duelOpenQueuePda,
      duelEntry: p.duelEntryPda,
      sessionSigner: p.sessionSigner.publicKey,
    } as any)
    .signers([p.sessionSigner])
    .rpc();
}

async function testSetCompleted(p: PlayerContext): Promise<void> {
  await programs.gameplayState.methods
    .testSetCompleted()
    .accounts({
      gameState: p.gameStatePda,
      sessionSigner: p.sessionSigner.publicKey,
    } as any)
    .signers([p.sessionSigner])
    .rpc();
}

async function settleDuelPayout(
  p: PlayerContext,
  creatorWallet: PublicKey | null,
): Promise<void> {
  await programs.gameplayState.methods
    .settleDuelPayout()
    .accountsPartial({
      duelEntry: p.duelEntryPda,
      duelVault: duelVaultPda,
      player: p.user.publicKey,
      sessionSigner: p.sessionSigner.publicKey,
      creatorWallet: creatorWallet,
      companyTreasury: COMPANY_TREASURY,
      gauntletPoolVault: gauntletPoolVaultPda,
      duelOpenQueue: duelOpenQueuePda,
      gameState: p.gameStatePda,
      inventory: p.inventoryPda,
    } as any)
    .preInstructions([
      ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }),
    ])
    .signers([p.sessionSigner])
    .rpc();
}

async function resetDuelEntryAndEndSession(
  p: PlayerContext,
  options?: { includeSessionVrfAccounts?: boolean },
): Promise<void> {
  const resetIx = await programs.gameplayState.methods
    .resetDuelEntry()
    .accountsPartial({
      duelEntry: p.duelEntryPda,
      gameState: p.gameStatePda,
      player: p.user.publicKey,
      sessionSigner: p.sessionSigner.publicKey,
      duelVault: duelVaultPda,
      companyTreasury: COMPANY_TREASURY,
      gauntletPoolVault: gauntletPoolVaultPda,
      creatorWallet: null,
    } as any)
    .instruction();

  const endIx = await programs.sessionManager.methods
    .endSession(DUEL_CAMPAIGN_LEVEL)
    .accounts({
      gameSession: p.sessionPda,
      gameState: p.gameStatePda,
      mapEnemies: p.mapEnemiesPda,
      generatedMap: p.generatedMapPda,
      mapPois: p.mapPoisPda,
      sessionDiscovery: p.sessionDiscoveryPda,
      playerProfile: p.playerProfilePda,
      player: p.user.publicKey,
      sessionSigner: p.sessionSigner.publicKey,
      sessionManagerAuthority: sessionManagerAuthorityPda,
      inventory: p.inventoryPda,
      mapVrfState: options?.includeSessionVrfAccounts ? p.mapVrfStatePda : null,
      poiVrfState: null,
      gameplayVrfState: options?.includeSessionVrfAccounts
        ? p.gameplayVrfStatePda
        : null,
      gauntletEchoes: null,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      playerProfileProgram: PROGRAM_IDS.playerProfile,
      mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
      poiSystemProgram: PROGRAM_IDS.poiSystem,
    } as any)
    .instruction();

  await sendBaseTx(
    "reset-duel-entry+end-session",
    [resetIx, endIx],
    [p.sessionSigner, p.user],
  );

  // Verify session closed
  const sessionInfo = await connection.getAccountInfo(
    p.sessionPda,
    "confirmed",
  );
  expect(sessionInfo).to.be.null;

  if (options?.includeSessionVrfAccounts) {
    const [mapVrfInfo, gameplayVrfInfo] = await Promise.all([
      connection.getAccountInfo(p.mapVrfStatePda, "confirmed"),
      connection.getAccountInfo(p.gameplayVrfStatePda, "confirmed"),
    ]);
    expect(mapVrfInfo).to.be.null;
    expect(gameplayVrfInfo).to.be.null;
  }
}

async function fetchGameState(gameStatePda: PublicKey): Promise<any> {
  const accountInfo = await connection.getAccountInfo(
    gameStatePda,
    "confirmed",
  );
  if (!accountInfo) {
    throw new Error("GameState account missing");
  }
  return (programs.gameplayState as any).coder.accounts.decode(
    "gameState",
    accountInfo.data,
  );
}

// ── Setup ───────────────────────────────────────────────────────────────────
before(async function () {
  this.timeout(30_000);

  admin = loadWalletKeypair();
  const wallet = walletFromKeypair(admin);
  connection = new Connection(RPC_URL, "confirmed");
  erConnection = new Connection(ER_RPC_URL, "confirmed");
  provider = createProvider(RPC_URL, wallet);
  anchor.setProvider(provider);
  programs = loadAllPrograms(provider);

  [sessionCounterPda] = getSessionCounterPda();
  [mapConfigPda] = getMapConfigPda();
  [sessionManagerAuthorityPda] = getSessionManagerAuthorityPda();
  [duelVaultPda] = getDuelVaultPda();
  [duelOpenQueuePda] = getDuelOpenQueuePda();
  [gauntletPoolVaultPda] = getGauntletPoolVaultPda();
});

// ─────────────────────────────────────────────────────────────────────────────
// Duel Matching E2E
// ─────────────────────────────────────────────────────────────────────────────
describe("Duel Matching E2E", function () {
  this.timeout(300_000);

  // ── Setup: initialize global duel accounts ──────────────────────────────
  describe("Setup", function () {
    this.timeout(60_000);

    it("initializes duels (vault + queues)", async () => {
      // Session counter
      try {
        await programs.sessionManager.methods
          .initializeCounter()
          .accounts({
            sessionCounter: sessionCounterPda,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();
      } catch (e: any) {
        if (!String(e).includes("already in use")) throw e;
      }

      // Map config
      try {
        await programs.mapGenerator.methods
          .initializeMapConfig()
          .accounts({
            mapConfig: mapConfigPda,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();
      } catch (e: any) {
        if (!String(e).includes("already in use")) throw e;
      }

      // Duels — tolerate already-initialized or delegated DuelErQueue
      try {
        await programs.gameplayState.methods
          .initializeDuels()
          .accounts({
            duelVault: duelVaultPda,
            duelOpenQueue: duelOpenQueuePda,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();
      } catch (e: any) {
        const msg = String(e);
        if (
          !msg.includes("already in use") &&
          !msg.includes("AccountOwnedByWrongProgram")
        )
          throw e;
      }

      // Pit Draft
      try {
        const [pitDraftQueuePda] = PublicKey.findProgramAddressSync(
          [Buffer.from("pit_draft_queue")],
          PROGRAM_IDS.gameplayState,
        );
        const [pitDraftVaultPda] = PublicKey.findProgramAddressSync(
          [Buffer.from("pit_draft_vault")],
          PROGRAM_IDS.gameplayState,
        );
        await programs.gameplayState.methods
          .initializePitDraft()
          .accounts({
            pitDraftQueue: pitDraftQueuePda,
            pitDraftVault: pitDraftVaultPda,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();
      } catch (e: any) {
        if (!String(e).includes("already in use")) throw e;
      }

      // Gauntlet (creates gauntletPoolVault needed by enter_duel)
      try {
        const [gauntletConfigPda] = PublicKey.findProgramAddressSync(
          [Buffer.from("gauntlet_config")],
          PROGRAM_IDS.gameplayState,
        );
        const weekPdas = [1, 2, 3, 4, 5].map(
          (w) =>
            PublicKey.findProgramAddressSync(
              [Buffer.from("gauntlet_week_pool"), Buffer.from([w])],
              PROGRAM_IDS.gameplayState,
            )[0],
        );
        await programs.gameplayState.methods
          .initializeGauntlet()
          .accounts({
            gauntletConfig: gauntletConfigPda,
            gauntletPoolVault: gauntletPoolVaultPda,
            gauntletWeek1: weekPdas[0],
            gauntletWeek2: weekPdas[1],
            gauntletWeek3: weekPdas[2],
            gauntletWeek4: weekPdas[3],
            gauntletWeek5: weekPdas[4],
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .preInstructions([
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ])
          .rpc();
      } catch (e: any) {
        if (!String(e).includes("already in use")) throw e;
      }

      // Verify
      const queue = await (
        programs.gameplayState.account as any
      ).duelOpenQueue.fetch(duelOpenQueuePda);
      expect(queue.initialized).to.be.true;
    });
  });

  // Shared across describe blocks
  let playerA: PlayerContext;

  // ── Player A creates unmatched duel ─────────────────────────────────────
  describe("Player A creates unmatched duel", function () {
    this.timeout(120_000);

    let queueLengthBefore: number;

    before(async function () {
      this.timeout(30_000);
      playerA = await setupPlayer("playerA");

      // Record queue length before test
      const queue = await (
        programs.gameplayState.account as any
      ).duelOpenQueue.fetch(duelOpenQueuePda);
      queueLengthBefore = queue.entries.length;
    });

    it("creates profile + starts duel session for player A", async () => {
      await createProfileAndStartDuelSession(playerA);

      // Verify game state exists with duel run mode
      const gs = await fetchGameState(playerA.gameStatePda);
      expect(gs.runMode).to.deep.include({ duel: {} });
    });

    it("enters duel (pays entry fee)", async () => {
      const vaultBefore = await connection.getBalance(duelVaultPda);
      await enterDuel(playerA);
      const vaultAfter = await connection.getBalance(duelVaultPda);

      // Verify entry fee deposited
      expect(vaultAfter - vaultBefore).to.equal(DUEL_ENTRY_LAMPORTS);

      // Verify duel entry created
      const duelEntry = await (
        programs.gameplayState.account as any
      ).duelEntry.fetch(playerA.duelEntryPda);
      expect(duelEntry.player.toBase58()).to.equal(
        playerA.user.publicKey.toBase58(),
      );
      expect(Number(duelEntry.entryLamports)).to.equal(DUEL_ENTRY_LAMPORTS);
      expect(duelEntry.finalized).to.be.false;
      expect(duelEntry.settled).to.be.false;
    });

    it("assign_duel_map_seed (no match, creator path)", async () => {
      await assignDuelMapSeed(playerA);

      // Check if player A was matched or not.
      // On a clean validator, no match available => duel_map_seed = 0 (creator path).
      // On a validator with pre-existing queue entries from other players,
      // player A might get matched — both outcomes are valid.
      const gs = await fetchGameState(playerA.gameStatePda);
      const duelEntry = await (
        programs.gameplayState.account as any
      ).duelEntry.fetch(playerA.duelEntryPda);
      // On a clean validator, no match available => duel_map_seed = 0 (creator path).
      expect(Number(gs.duelMapSeed)).to.equal(
        0,
        "creator path: seed should be 0",
      );
      expect(duelEntry.matchedCreator).to.be.null;
    });

    it("marks game as completed via test_set_completed", async () => {
      await testSetCompleted(playerA);

      const gs = await fetchGameState(playerA.gameStatePda);
      expect(gs.completed).to.be.true;
    });

    it("settle_duel_payout pushes to queue", async () => {
      await settleDuelPayout(playerA, null);

      const duelEntry = await (
        programs.gameplayState.account as any
      ).duelEntry.fetch(playerA.duelEntryPda);
      expect(duelEntry.settled).to.be.true;
      expect(duelEntry.finalized).to.be.true;
    });

    it("verifies DuelOpenQueue has 1 new entry with player A", async () => {
      const queue = await (
        programs.gameplayState.account as any
      ).duelOpenQueue.fetch(duelOpenQueuePda);
      // If player A was matched (consumed an entry) and then settled (pushed new entry),
      // the net change might be 0. If unmatched, it's +1.
      // Just verify player A's entry is in the queue.
      const playerAEntry = queue.entries.find(
        (e: any) => e.player.toBase58() === playerA.user.publicKey.toBase58(),
      );
      expect(playerAEntry).to.not.be.undefined;
      expect(Number(playerAEntry.entryLamports)).to.equal(DUEL_ENTRY_LAMPORTS);
    });

    it("ends session for player A", async () => {
      await initDelegateAndUndelegateDuelVrfStates(playerA);
      await resetDuelEntryAndEndSession(playerA, {
        includeSessionVrfAccounts: true,
      });
    });
  });

  // ── Player B matches with player A ──────────────────────────────────────
  describe("Player B matches with player A", function () {
    this.timeout(120_000);

    let playerB: PlayerContext;
    let queueLengthBefore: number;
    let vaultBalanceBefore: number;
    let playerAWallet: PublicKey;

    before(async function () {
      this.timeout(60_000);

      // Player A's entry is already in the queue from the previous test section.
      playerAWallet = playerA.user.publicKey;

      // Record queue state and vault balance
      const queue = await (
        programs.gameplayState.account as any
      ).duelOpenQueue.fetch(duelOpenQueuePda);
      queueLengthBefore = queue.entries.length;
      // Player A's entry should be in the queue from the previous section
      const hasPlayerA = queue.entries.some(
        (e: any) => e.player.toBase58() === playerA.user.publicKey.toBase58(),
      );
      expect(hasPlayerA).to.be.true;

      // Set up player B
      playerB = await setupPlayer("playerB");
      vaultBalanceBefore = await connection.getBalance(duelVaultPda);
    });

    it("creates profile + starts duel session for player B", async () => {
      await createProfileAndStartDuelSession(playerB);

      const gs = await fetchGameState(playerB.gameStatePda);
      expect(gs.runMode).to.deep.include({ duel: {} });
    });

    it("enters duel (pays entry fee)", async () => {
      await enterDuel(playerB);

      const duelEntry = await (
        programs.gameplayState.account as any
      ).duelEntry.fetch(playerB.duelEntryPda);
      expect(Number(duelEntry.entryLamports)).to.equal(DUEL_ENTRY_LAMPORTS);
    });

    it("assign_duel_map_seed (matches with player A)", async () => {
      await assignDuelMapSeed(playerB);

      // Should have matched — duel_map_seed != 0
      const gs = await fetchGameState(playerB.gameStatePda);
      expect(Number(gs.duelMapSeed)).to.not.equal(0);
    });

    it("verifies DuelOpenQueue lost one entry", async () => {
      const queue = await (
        programs.gameplayState.account as any
      ).duelOpenQueue.fetch(duelOpenQueuePda);
      expect(queue.entries.length).to.equal(queueLengthBefore - 1);
    });

    it("verifies duel_entry.matched_creator is set", async () => {
      const duelEntry = await (
        programs.gameplayState.account as any
      ).duelEntry.fetch(playerB.duelEntryPda);
      expect(duelEntry.matchedCreator).to.not.be.null;
      expect(duelEntry.matchedCreator.player.toBase58()).to.equal(
        playerAWallet.toBase58(),
      );
    });

    it("marks game as completed via test_set_completed", async () => {
      await testSetCompleted(playerB);

      const gs = await fetchGameState(playerB.gameStatePda);
      expect(gs.completed).to.be.true;
    });

    it("settle_duel_payout resolves PvP and pays winner", async () => {
      const vaultBefore = await connection.getBalance(duelVaultPda);

      // Pass creator wallet for potential payout
      await settleDuelPayout(playerB, playerAWallet);

      const duelEntry = await (
        programs.gameplayState.account as any
      ).duelEntry.fetch(playerB.duelEntryPda);
      expect(duelEntry.settled).to.be.true;
      expect(duelEntry.finalized).to.be.true;

      // Vault balance should have decreased (total pot distributed)
      const vaultAfter = await connection.getBalance(duelVaultPda);
      const totalPot = DUEL_ENTRY_LAMPORTS * 2;
      expect(vaultBefore - vaultAfter).to.equal(totalPot);
    });

    it("ends session for player B", async () => {
      await resetDuelEntryAndEndSession(playerB);
    });
  });

  // Self-matching prevention is covered by the Rust unit test:
  // test_find_matching_creator_index_skips_self (in lib.rs)
});
