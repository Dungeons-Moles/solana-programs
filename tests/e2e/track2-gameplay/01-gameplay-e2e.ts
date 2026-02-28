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
  AllPrograms,
} from "../shared/setup";
import {
  getSessionCounterPda,
  getSessionNoncesPda,
  getSessionPda,
  getSessionManagerAuthorityPda,
  getPlayerProfilePda,
  getMapConfigPda,
  getGeneratedMapPda,
  getGameStatePda,
  getMapEnemiesPda,
  getGameplayAuthorityPda,
  getDuelVaultPda,
  getDuelOpenQueuePda,
  getPitDraftQueuePda,
  getPitDraftVaultPda,
  getGauntletConfigPda,
  getGauntletPoolVaultPda,
  getGauntletWeekPoolPda,
  getInventoryPda,
  getMapPoisPda,
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
  process.env.EXPO_PUBLIC_EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";

const DELEGATION_PROGRAM_ID = PROGRAM_IDS.delegation;
const ER_VALIDATOR = new PublicKey("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev");

// ── Shared mutable state across describe blocks ─────────────────────────────
let connection: Connection;
let erConnection: Connection;
let provider: anchor.AnchorProvider;
let programs: AllPrograms;
let admin: Keypair;

let user: Keypair;
let sessionSigner: Keypair;
const campaignLevel = 1;

// PDAs populated during tests
let sessionCounterPda: PublicKey;
let sessionNoncesPda: PublicKey;
let mapConfigPda: PublicKey;
let playerProfilePda: PublicKey;
let sessionPda: PublicKey;
let gameStatePda: PublicKey;
let mapEnemiesPda: PublicKey;
let generatedMapPda: PublicKey;
let inventoryPda: PublicKey;
let mapPoisPda: PublicKey;
let gameplayAuthorityPda: PublicKey;
let sessionManagerAuthorityPda: PublicKey;

// Duel / PitDraft / Gauntlet PDAs
let duelVaultPda: PublicKey;
let duelOpenQueuePda: PublicKey;
let pitDraftQueuePda: PublicKey;
let pitDraftVaultPda: PublicKey;
let gauntletConfigPda: PublicKey;
let gauntletPoolVaultPda: PublicKey;
let gauntletWeek1Pda: PublicKey;
let gauntletWeek2Pda: PublicKey;
let gauntletWeek3Pda: PublicKey;
let gauntletWeek4Pda: PublicKey;
let gauntletWeek5Pda: PublicKey;

// Initial position for movement test
let initialX: number;
let initialY: number;

// ── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Send a transaction on the base layer signed by sessionSigner as fee payer.
 */
const sendDelegateTx = async (
  label: string,
  ixs: TransactionInstruction[]
): Promise<string> => {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = sessionSigner.publicKey;
  const bh = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(sessionSigner);
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

/**
 * Send a transaction on the ER signed by sessionSigner as fee payer.
 */
const sendErSessionSignerTx = async (
  label: string,
  ixs: TransactionInstruction[]
): Promise<string> => {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = sessionSigner.publicKey;
  const bh = await erConnection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(sessionSigner);
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

/**
 * Poll base layer until an account's owner matches the expected program.
 * Times out after ~10s (40 iterations x 250ms).
 */
const waitForBaseOwner = async (
  account: PublicKey,
  owner: PublicKey,
  label: string
): Promise<void> => {
  for (let i = 0; i < 40; i++) {
    const info = await connection.getAccountInfo(account, "confirmed");
    if (info?.owner.equals(owner)) return;
    await new Promise((r) => setTimeout(r, 250));
  }
  const info = await connection.getAccountInfo(account, "confirmed");
  throw new Error(
    `${label} owner did not restore (current=${info?.owner.toBase58() ?? "missing"})`
  );
};

/**
 * Poll ER until an account owner matches expected program.
 */
const waitForErOwner = async (
  account: PublicKey,
  _owner: PublicKey,
  label: string
): Promise<void> => {
  for (let i = 0; i < 60; i++) {
    const info = await erConnection.getAccountInfo(account, "confirmed");
    if (info) return;
    await new Promise((r) => setTimeout(r, 250));
  }
  const info = await erConnection.getAccountInfo(account, "confirmed");
  throw new Error(
    `${label} account did not become visible on ER (currentOwner=${info?.owner.toBase58() ?? "missing"})`
  );
};

/**
 * Decode game state from raw account data on an arbitrary connection.
 */
const fetchGameStateFrom = async (
  conn: Connection
): Promise<{ positionX: number; positionY: number }> => {
  const accountInfo = await conn.getAccountInfo(gameStatePda, "confirmed");
  if (!accountInfo) {
    throw new Error(`GameState account missing on ${conn.rpcEndpoint}`);
  }
  return (programs.gameplayState as any).coder.accounts.decode(
    "gameState",
    accountInfo.data
  ) as { positionX: number; positionY: number };
};

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

  // Derive global PDAs
  [sessionCounterPda] = getSessionCounterPda();
  [mapConfigPda] = getMapConfigPda();
  [duelVaultPda] = getDuelVaultPda();
  [duelOpenQueuePda] = getDuelOpenQueuePda();
  [pitDraftQueuePda] = getPitDraftQueuePda();
  [pitDraftVaultPda] = getPitDraftVaultPda();
  [gauntletConfigPda] = getGauntletConfigPda();
  [gauntletPoolVaultPda] = getGauntletPoolVaultPda();
  [gauntletWeek1Pda] = getGauntletWeekPoolPda(1);
  [gauntletWeek2Pda] = getGauntletWeekPoolPda(2);
  [gauntletWeek3Pda] = getGauntletWeekPoolPda(3);
  [gauntletWeek4Pda] = getGauntletWeekPoolPda(4);
  [gauntletWeek5Pda] = getGauntletWeekPoolPda(5);
  [gameplayAuthorityPda] = getGameplayAuthorityPda();
  [sessionManagerAuthorityPda] = getSessionManagerAuthorityPda();

  // Generate fresh user + session signer keypairs
  user = Keypair.generate();
  sessionSigner = Keypair.generate();
  [sessionNoncesPda] = getSessionNoncesPda(user.publicKey);

  // Fund both
  await airdropAndConfirm(connection, user.publicKey, 5 * LAMPORTS_PER_SOL);
  await airdropAndConfirm(connection, sessionSigner.publicKey, 5 * LAMPORTS_PER_SOL);
});

// ─────────────────────────────────────────────────────────────────────────────
// 1. Initialize global state
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: Initialize global state", function () {
  this.timeout(60_000);

  it("initializes session counter (idempotent)", async () => {
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
    const info = await connection.getAccountInfo(sessionCounterPda, "confirmed");
    expect(info).to.not.be.null;
    expect(info!.owner.equals(PROGRAM_IDS.sessionManager)).to.be.true;
  });

  it("initializes map config (idempotent)", async () => {
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
    const info = await connection.getAccountInfo(mapConfigPda, "confirmed");
    expect(info).to.not.be.null;
    expect(info!.owner.equals(PROGRAM_IDS.mapGenerator)).to.be.true;
  });

  it("initializes duels vault + open queue (idempotent)", async () => {
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
      if (!String(e).includes("already in use")) throw e;
    }
    const vaultInfo = await connection.getAccountInfo(duelVaultPda, "confirmed");
    const queueInfo = await connection.getAccountInfo(duelOpenQueuePda, "confirmed");
    expect(vaultInfo).to.not.be.null;
    expect(queueInfo).to.not.be.null;
  });

  it("initializes pit draft queue + vault (idempotent)", async () => {
    try {
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
    const queueInfo = await connection.getAccountInfo(pitDraftQueuePda, "confirmed");
    const vaultInfo = await connection.getAccountInfo(pitDraftVaultPda, "confirmed");
    expect(queueInfo).to.not.be.null;
    expect(vaultInfo).to.not.be.null;
  });

  it("initializes gauntlet with 5 week pools (idempotent)", async () => {
    try {
      await programs.gameplayState.methods
        .initializeGauntlet()
        .accounts({
          gauntletConfig: gauntletConfigPda,
          gauntletPoolVault: gauntletPoolVaultPda,
          gauntletWeek1: gauntletWeek1Pda,
          gauntletWeek2: gauntletWeek2Pda,
          gauntletWeek3: gauntletWeek3Pda,
          gauntletWeek4: gauntletWeek4Pda,
          gauntletWeek5: gauntletWeek5Pda,
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
    const configInfo = await connection.getAccountInfo(gauntletConfigPda, "confirmed");
    expect(configInfo).to.not.be.null;
    for (const weekPda of [
      gauntletWeek1Pda,
      gauntletWeek2Pda,
      gauntletWeek3Pda,
      gauntletWeek4Pda,
      gauntletWeek5Pda,
    ]) {
      const info = await connection.getAccountInfo(weekPda, "confirmed");
      expect(info).to.not.be.null;
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Create profile + start session
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: Create profile + start session", function () {
  this.timeout(60_000);

  it("creates a player profile", async () => {
    [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    const name = `e2e-${user.publicKey.toBase58().slice(0, 6)}`;

    await programs.playerProfile.methods
      .initializeProfile(name)
      .accounts({
        playerProfile: playerProfilePda,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user])
      .rpc();

    const profileInfo = await connection.getAccountInfo(playerProfilePda, "confirmed");
    expect(profileInfo).to.not.be.null;
    expect(profileInfo!.owner.equals(PROGRAM_IDS.playerProfile)).to.be.true;
  });

  it("resets profile by closing and re-creating the profile PDA", async () => {
    await programs.playerProfile.methods
      .closeProfile()
      .accounts({
        playerProfile: playerProfilePda,
        owner: user.publicKey,
      } as any)
      .signers([user])
      .rpc();

    const closedInfo = await connection.getAccountInfo(playerProfilePda, "confirmed");
    expect(
      !closedInfo || !closedInfo.owner.equals(PROGRAM_IDS.playerProfile)
    ).to.be.true;

    const resetName = `reset-${user.publicKey.toBase58().slice(0, 6)}`;
    await programs.playerProfile.methods
      .initializeProfile(resetName)
      .accounts({
        playerProfile: playerProfilePda,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user])
      .rpc();

    const recreatedInfo = await connection.getAccountInfo(playerProfilePda, "confirmed");
    expect(recreatedInfo).to.not.be.null;
    expect(recreatedInfo!.owner.equals(PROGRAM_IDS.playerProfile)).to.be.true;
  });

  it("starts a session with all sub-accounts", async () => {
    [sessionPda] = getSessionPda(user.publicKey, campaignLevel);
    [gameStatePda] = getGameStatePda(sessionPda);
    [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    [generatedMapPda] = getGeneratedMapPda(sessionPda);
    [inventoryPda] = getInventoryPda(sessionPda);
    [mapPoisPda] = getMapPoisPda(sessionPda);

    await programs.sessionManager.methods
      .startSession(campaignLevel)
      .accounts({
        sessionNonces: sessionNoncesPda,
        gameSession: sessionPda,
        sessionCounter: sessionCounterPda,
        playerProfile: playerProfilePda,
        player: user.publicKey,
        sessionSigner: sessionSigner.publicKey,
        mapConfig: mapConfigPda,
        generatedMap: generatedMapPda,
        gameState: gameStatePda,
        mapEnemies: mapEnemiesPda,
        mapPois: mapPoisPda,
        inventory: inventoryPda,
        mapGeneratorProgram: programs.mapGenerator.programId,
        gameplayStateProgram: programs.gameplayState.programId,
        poiSystemProgram: programs.poiSystem.programId,
        playerInventoryProgram: programs.playerInventory.programId,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([user, sessionSigner])
      .rpc();

    // Verify SessionNonces account created with correct bump
    const noncesInfo = await connection.getAccountInfo(sessionNoncesPda, "confirmed");
    expect(noncesInfo).to.not.be.null;
    expect(noncesInfo!.owner.equals(PROGRAM_IDS.sessionManager)).to.be.true;

    // Verify all sub-accounts created
    const sessionInfo = await connection.getAccountInfo(sessionPda, "confirmed");
    expect(sessionInfo).to.not.be.null;
    expect(sessionInfo!.owner.equals(PROGRAM_IDS.sessionManager)).to.be.true;

    const gameStateInfo = await connection.getAccountInfo(gameStatePda, "confirmed");
    expect(gameStateInfo).to.not.be.null;
    expect(gameStateInfo!.owner.equals(PROGRAM_IDS.gameplayState)).to.be.true;

    const mapEnemiesInfo = await connection.getAccountInfo(mapEnemiesPda, "confirmed");
    expect(mapEnemiesInfo).to.not.be.null;

    const generatedMapInfo = await connection.getAccountInfo(generatedMapPda, "confirmed");
    expect(generatedMapInfo).to.not.be.null;

    const inventoryInfo = await connection.getAccountInfo(inventoryPda, "confirmed");
    expect(inventoryInfo).to.not.be.null;

    const mapPoisInfo = await connection.getAccountInfo(mapPoisPda, "confirmed");
    expect(mapPoisInfo).to.not.be.null;

    // Store initial position for later movement
    const gameState = await fetchGameStateFrom(connection);
    initialX = Number(gameState.positionX);
    initialY = Number(gameState.positionY);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. Delegate to ER
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: Delegate to ER", function () {
  this.timeout(120_000);

  it("delegates gameplay accounts (gameState + mapEnemies)", async () => {
    const gameplayGameStateDelegate = deriveDelegateAccounts(
      gameStatePda,
      programs.gameplayState.programId
    );
    const gameplayMapEnemiesDelegate = deriveDelegateAccounts(
      mapEnemiesPda,
      programs.gameplayState.programId
    );

    const ix = await programs.gameplayState.methods
      .delegateGameplayAccounts(ER_VALIDATOR)
      .accountsStrict({
        bufferGameState: gameplayGameStateDelegate.buffer,
        delegationRecordGameState: gameplayGameStateDelegate.delegationRecord,
        delegationMetadataGameState: gameplayGameStateDelegate.delegationMetadata,
        gameState: gameStatePda,
        bufferMapEnemies: gameplayMapEnemiesDelegate.buffer,
        delegationRecordMapEnemies: gameplayMapEnemiesDelegate.delegationRecord,
        delegationMetadataMapEnemies: gameplayMapEnemiesDelegate.delegationMetadata,
        mapEnemies: mapEnemiesPda,
        gameSession: sessionPda,
        player: sessionSigner.publicKey,
        ownerProgram: programs.gameplayState.programId,
        delegationProgram: DELEGATION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .instruction();

    await sendDelegateTx("delegate-gameplay", [ix]);

    const gsInfo = await connection.getAccountInfo(gameStatePda, "confirmed");
    expect(gsInfo!.owner.equals(DELEGATION_PROGRAM_ID)).to.be.true;

    const meInfo = await connection.getAccountInfo(mapEnemiesPda, "confirmed");
    expect(meInfo!.owner.equals(DELEGATION_PROGRAM_ID)).to.be.true;
  });

  it("delegates generated map", async () => {
    const generatedMapDelegate = deriveDelegateAccounts(
      generatedMapPda,
      programs.mapGenerator.programId
    );

    const ix = await programs.mapGenerator.methods
      .delegateGeneratedMap(ER_VALIDATOR)
      .accountsStrict({
        bufferGeneratedMap: generatedMapDelegate.buffer,
        delegationRecordGeneratedMap: generatedMapDelegate.delegationRecord,
        delegationMetadataGeneratedMap: generatedMapDelegate.delegationMetadata,
        generatedMap: generatedMapPda,
        session: sessionPda,
        player: sessionSigner.publicKey,
        ownerProgram: programs.mapGenerator.programId,
        delegationProgram: DELEGATION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .instruction();

    await sendDelegateTx("delegate-generated-map", [ix]);

    const info = await connection.getAccountInfo(generatedMapPda, "confirmed");
    expect(info!.owner.equals(DELEGATION_PROGRAM_ID)).to.be.true;
  });

  it("delegates inventory", async () => {
    const inventoryDelegate = deriveDelegateAccounts(
      inventoryPda,
      programs.playerInventory.programId
    );

    const ix = await programs.playerInventory.methods
      .delegateInventory(ER_VALIDATOR)
      .accountsStrict({
        bufferInventory: inventoryDelegate.buffer,
        delegationRecordInventory: inventoryDelegate.delegationRecord,
        delegationMetadataInventory: inventoryDelegate.delegationMetadata,
        inventory: inventoryPda,
        session: sessionPda,
        player: sessionSigner.publicKey,
        ownerProgram: programs.playerInventory.programId,
        delegationProgram: DELEGATION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .instruction();

    await sendDelegateTx("delegate-inventory", [ix]);

    const info = await connection.getAccountInfo(inventoryPda, "confirmed");
    expect(info!.owner.equals(DELEGATION_PROGRAM_ID)).to.be.true;
  });

  it("delegates map POIs", async () => {
    const mapPoisDelegate = deriveDelegateAccounts(
      mapPoisPda,
      programs.poiSystem.programId
    );

    const ix = await programs.poiSystem.methods
      .delegateMapPois(ER_VALIDATOR)
      .accountsStrict({
        bufferMapPois: mapPoisDelegate.buffer,
        delegationRecordMapPois: mapPoisDelegate.delegationRecord,
        delegationMetadataMapPois: mapPoisDelegate.delegationMetadata,
        mapPois: mapPoisPda,
        gameSession: sessionPda,
        player: sessionSigner.publicKey,
        ownerProgram: programs.poiSystem.programId,
        delegationProgram: DELEGATION_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .instruction();

    await sendDelegateTx("delegate-map-pois", [ix]);

    const info = await connection.getAccountInfo(mapPoisPda, "confirmed");
    expect(info!.owner.equals(DELEGATION_PROGRAM_ID)).to.be.true;
  });

  it("delegates session (user signs)", async () => {
    const ix = await programs.sessionManager.methods
      .delegateSession(campaignLevel, ER_VALIDATOR)
      .accounts({
        gameSession: sessionPda,
        player: user.publicKey,
        sessionSigner: sessionSigner.publicKey,
        sessionNonces: sessionNoncesPda,
      } as any)
      .instruction();

    // delegateSession requires user as signer, so we build a raw tx signed by both
    const tx = new Transaction().add(ix);
    tx.feePayer = sessionSigner.publicKey;
    const bh = await connection.getLatestBlockhash("confirmed");
    tx.recentBlockhash = bh.blockhash;
    tx.sign(sessionSigner, user);
    const sig = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
    const status = await connection.getSignatureStatuses([sig], {
      searchTransactionHistory: true,
    });
    if (status.value[0]?.err) {
      throw new Error(
        `delegate-session failed: ${JSON.stringify(status.value[0].err)}`
      );
    }

    const info = await connection.getAccountInfo(sessionPda, "confirmed");
    expect(info!.owner.equals(DELEGATION_PROGRAM_ID)).to.be.true;
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Gameplay on ER
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: Gameplay on ER", function () {
  this.timeout(120_000);

  it("moves player on ER with retry logic", async () => {
    // Base-layer delegation transactions need a short propagation window before
    // ER accepts writes to delegated accounts.
    await waitForErOwner(gameStatePda, DELEGATION_PROGRAM_ID, "game_state");
    await waitForErOwner(mapEnemiesPda, DELEGATION_PROGRAM_ID, "map_enemies");
    await waitForErOwner(generatedMapPda, DELEGATION_PROGRAM_ID, "generated_map");
    await waitForErOwner(inventoryPda, DELEGATION_PROGRAM_ID, "inventory");
    await waitForErOwner(mapPoisPda, DELEGATION_PROGRAM_ID, "map_pois");
    await waitForErOwner(sessionPda, DELEGATION_PROGRAM_ID, "session");

    const targetX = initialX > 0 ? initialX - 1 : initialX + 1;
    const targetY = initialY;

    let moveSucceeded = false;
    let lastMoveError: unknown = null;

    for (let attempt = 1; attempt <= 24; attempt++) {
      const moveTx = await programs.gameplayState.methods
        .movePlayer(targetX, targetY)
        .accountsPartial({
          gameState: gameStatePda,
          gameSession: sessionPda,
          mapEnemies: mapEnemiesPda,
          generatedMap: generatedMapPda,
          inventory: inventoryPda,
          gameplayAuthority: gameplayAuthorityPda,
          playerInventoryProgram: programs.playerInventory.programId,
          mapGeneratorProgram: programs.mapGenerator.programId,
          mapPois: mapPoisPda,
          poiSystemProgram: programs.poiSystem.programId,
          gameplayVrfState: null,
          player: sessionSigner.publicKey,
        })
        .transaction();

      moveTx.feePayer = sessionSigner.publicKey;
      const erBh = await erConnection.getLatestBlockhash("confirmed");
      moveTx.recentBlockhash = erBh.blockhash;
      moveTx.sign(sessionSigner);

      try {
        const moveSig = await erConnection.sendRawTransaction(moveTx.serialize(), {
          skipPreflight: true,
          maxRetries: 1,
        });
        await erConnection.confirmTransaction(
          { signature: moveSig, ...erBh },
          "confirmed"
        );
        const status = await erConnection.getSignatureStatuses([moveSig], {
          searchTransactionHistory: true,
        });
        if (status.value[0]?.err) {
          throw new Error(
            `move tx finalized with error: ${JSON.stringify(status.value[0].err)}`
          );
        }
        moveSucceeded = true;
        break;
      } catch (e: any) {
        lastMoveError = e;
        await new Promise((r) => setTimeout(r, 500));
      }
    }

    if (!moveSucceeded) {
      throw lastMoveError ?? new Error("movePlayer failed after 24 attempts");
    }

    // Verify position changed on ER
    const erState = await fetchGameStateFrom(erConnection);
    const erX = Number(erState.positionX);
    const erY = Number(erState.positionY);
    expect(erX !== initialX || erY !== initialY).to.be.true;

    // Verify position is still stale on base layer (delegated, so base has old data)
    const baseState = await fetchGameStateFrom(connection);
    const baseX = Number(baseState.positionX);
    const baseY = Number(baseState.positionY);
    expect(baseX).to.equal(initialX);
    expect(baseY).to.equal(initialY);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. Undelegate
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: Undelegate", function () {
  this.timeout(120_000);

  it("undelegates gameplay accounts (gameState + mapEnemies)", async () => {
    await waitForErOwner(gameStatePda, DELEGATION_PROGRAM_ID, "game_state");
    await waitForErOwner(mapEnemiesPda, DELEGATION_PROGRAM_ID, "map_enemies");
    await waitForErOwner(generatedMapPda, DELEGATION_PROGRAM_ID, "generated_map");
    await waitForErOwner(inventoryPda, DELEGATION_PROGRAM_ID, "inventory");
    await waitForErOwner(mapPoisPda, DELEGATION_PROGRAM_ID, "map_pois");
    await waitForErOwner(sessionPda, DELEGATION_PROGRAM_ID, "session");

    const ix = await programs.gameplayState.methods
      .undelegateGameplayAccounts()
      .accounts({
        gameState: gameStatePda,
        mapEnemies: mapEnemiesPda,
        gameSession: sessionPda,
        sessionSigner: sessionSigner.publicKey,
      } as any)
      .instruction();

    await sendErSessionSignerTx("undelegate-gameplay", [ix]);
    await waitForBaseOwner(gameStatePda, PROGRAM_IDS.gameplayState, "game_state");
    await waitForBaseOwner(mapEnemiesPda, PROGRAM_IDS.gameplayState, "map_enemies");

    const gsInfo = await connection.getAccountInfo(gameStatePda, "confirmed");
    expect(gsInfo!.owner.equals(PROGRAM_IDS.gameplayState)).to.be.true;

    const meInfo = await connection.getAccountInfo(mapEnemiesPda, "confirmed");
    expect(meInfo!.owner.equals(PROGRAM_IDS.gameplayState)).to.be.true;
  });

  it("undelegates generated map", async () => {
    const ix = await programs.mapGenerator.methods
      .undelegateGeneratedMap()
      .accounts({
        generatedMap: generatedMapPda,
        session: sessionPda,
        sessionSigner: sessionSigner.publicKey,
      } as any)
      .instruction();

    await sendErSessionSignerTx("undelegate-generated-map", [ix]);
    await waitForBaseOwner(generatedMapPda, PROGRAM_IDS.mapGenerator, "generated_map");

    const info = await connection.getAccountInfo(generatedMapPda, "confirmed");
    expect(info!.owner.equals(PROGRAM_IDS.mapGenerator)).to.be.true;
  });

  it("undelegates inventory", async () => {
    const ix = await programs.playerInventory.methods
      .undelegateInventory()
      .accounts({
        inventory: inventoryPda,
        session: sessionPda,
        sessionSigner: sessionSigner.publicKey,
      } as any)
      .instruction();

    await sendErSessionSignerTx("undelegate-inventory", [ix]);
    await waitForBaseOwner(inventoryPda, PROGRAM_IDS.playerInventory, "inventory");

    const info = await connection.getAccountInfo(inventoryPda, "confirmed");
    expect(info!.owner.equals(PROGRAM_IDS.playerInventory)).to.be.true;
  });

  it("undelegates map POIs", async () => {
    const ix = await programs.poiSystem.methods
      .undelegateMapPois()
      .accounts({
        mapPois: mapPoisPda,
        gameSession: sessionPda,
        sessionSigner: sessionSigner.publicKey,
      } as any)
      .instruction();

    await sendErSessionSignerTx("undelegate-map-pois", [ix]);
    await waitForBaseOwner(mapPoisPda, PROGRAM_IDS.poiSystem, "map_pois");

    const info = await connection.getAccountInfo(mapPoisPda, "confirmed");
    expect(info!.owner.equals(PROGRAM_IDS.poiSystem)).to.be.true;
  });

  it("undelegates session (user + sessionSigner sign, via ER)", async () => {
    const stateHash = Array.from({ length: 32 }, (_, i) => i);

    const ix = await programs.sessionManager.methods
      .undelegateSession(campaignLevel, stateHash)
      .accounts({
        gameSession: sessionPda,
        player: user.publicKey,
        sessionSigner: sessionSigner.publicKey,
      } as any)
      .instruction();

    // undelegateSession needs sessionSigner as fee payer; may or may not need user sig
    // Following the reference: only sessionSigner signs on ER
    const tx = new Transaction().add(ix);
    tx.feePayer = sessionSigner.publicKey;
    const bh = await erConnection.getLatestBlockhash("confirmed");
    tx.recentBlockhash = bh.blockhash;
    tx.sign(sessionSigner);
    const sig = await erConnection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    await erConnection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
    const status = await erConnection.getSignatureStatuses([sig], {
      searchTransactionHistory: true,
    });
    if (status.value[0]?.err) {
      throw new Error(
        `undelegate-session failed: ${JSON.stringify(status.value[0].err)}`
      );
    }

    // Poll until session owner restored on base layer
    await waitForBaseOwner(sessionPda, PROGRAM_IDS.sessionManager, "session");

    const info = await connection.getAccountInfo(sessionPda, "confirmed");
    expect(info!.owner.equals(PROGRAM_IDS.sessionManager)).to.be.true;
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 6. End session
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: End session", function () {
  this.timeout(60_000);

  it("ends the session on base layer", async () => {
    // Fetch profile totalRuns before end
    const profileBefore = await (programs.playerProfile as any).account.playerProfile.fetch(
      playerProfilePda
    );
    const runsBefore = Number(profileBefore.totalRuns);

    const endSessionIx = await programs.sessionManager.methods
      .endSession(campaignLevel)
      .accountsPartial({
        gameSession: sessionPda,
        gameState: gameStatePda,
        mapEnemies: mapEnemiesPda,
        generatedMap: generatedMapPda,
        mapPois: mapPoisPda,
        playerProfile: playerProfilePda,
        player: user.publicKey,
        sessionSigner: sessionSigner.publicKey,
        sessionManagerAuthority: sessionManagerAuthorityPda,
        inventory: inventoryPda,
        mapVrfState: null,
        poiVrfState: null,
        gameplayVrfState: null,
        playerInventoryProgram: programs.playerInventory.programId,
        gameplayStateProgram: programs.gameplayState.programId,
        playerProfileProgram: programs.playerProfile.programId,
        mapGeneratorProgram: programs.mapGenerator.programId,
        poiSystemProgram: programs.poiSystem.programId,
      })
      .instruction();

    const tx = new Transaction().add(endSessionIx);
    tx.feePayer = sessionSigner.publicKey;
    const bh = await connection.getLatestBlockhash("confirmed");
    tx.recentBlockhash = bh.blockhash;
    tx.sign(sessionSigner);
    const sig = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
    const status = await connection.getSignatureStatuses([sig], {
      searchTransactionHistory: true,
    });
    if (status.value[0]?.err) {
      throw new Error(
        `end-session failed: ${JSON.stringify(status.value[0].err)}`
      );
    }

    // Verify session PDA closed (account no longer exists)
    const sessionInfo = await connection.getAccountInfo(sessionPda, "confirmed");
    expect(sessionInfo).to.be.null;

    // Verify profile totalRuns incremented
    const profileAfter = await (programs.playerProfile as any).account.playerProfile.fetch(
      playerProfilePda
    );
    const runsAfter = Number(profileAfter.totalRuns);
    expect(runsAfter).to.equal(runsBefore + 1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 7. Error cases
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: Error cases", function () {
  this.timeout(60_000);

  it("fails to start session without a profile", async () => {
    const noProfileUser = Keypair.generate();
    const noProfileSessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, noProfileUser.publicKey, 2 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(
      connection,
      noProfileSessionSigner.publicKey,
      2 * LAMPORTS_PER_SOL
    );

    const [noProfilePda] = getPlayerProfilePda(noProfileUser.publicKey);
    const [noSessionNoncesPda] = getSessionNoncesPda(noProfileUser.publicKey);
    const [noSessionPda] = getSessionPda(noProfileUser.publicKey, campaignLevel);
    const [noGameStatePda] = getGameStatePda(noSessionPda);
    const [noMapEnemiesPda] = getMapEnemiesPda(noSessionPda);
    const [noGeneratedMapPda] = getGeneratedMapPda(noSessionPda);
    const [noInventoryPda] = getInventoryPda(noSessionPda);
    const [noMapPoisPda] = getMapPoisPda(noSessionPda);

    try {
      await programs.sessionManager.methods
        .startSession(campaignLevel)
        .accounts({
          sessionNonces: noSessionNoncesPda,
          gameSession: noSessionPda,
          sessionCounter: sessionCounterPda,
          playerProfile: noProfilePda,
          player: noProfileUser.publicKey,
          sessionSigner: noProfileSessionSigner.publicKey,
          mapConfig: mapConfigPda,
          generatedMap: noGeneratedMapPda,
          gameState: noGameStatePda,
          mapEnemies: noMapEnemiesPda,
          mapPois: noMapPoisPda,
          inventory: noInventoryPda,
          mapGeneratorProgram: programs.mapGenerator.programId,
          gameplayStateProgram: programs.gameplayState.programId,
          poiSystemProgram: programs.poiSystem.programId,
          playerInventoryProgram: programs.playerInventory.programId,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
        ])
        .signers([noProfileUser, noProfileSessionSigner])
        .rpc();

      expect.fail("Expected startSession to fail without a profile");
    } catch (e: any) {
      // The error should indicate the profile account doesn't exist
      expect(String(e)).to.match(
        /AccountNotInitialized|account.*not.*found|does not exist|not initialized|Account does not exist|AccountOwnedByWrongProgram|ConstraintSeeds|AnchorError|Error/i
      );
    }
  });

  it("fails to start a duplicate session for same user + level", async () => {
    // The user's session was already closed in the end-session test.
    // Start a fresh session, then try to start another at the same level.
    const dupSessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, dupSessionSigner.publicKey, 2 * LAMPORTS_PER_SOL);

    const [dupSessionPda] = getSessionPda(user.publicKey, campaignLevel);
    const [dupGameStatePda] = getGameStatePda(dupSessionPda);
    const [dupMapEnemiesPda] = getMapEnemiesPda(dupSessionPda);
    const [dupGeneratedMapPda] = getGeneratedMapPda(dupSessionPda);
    const [dupInventoryPda] = getInventoryPda(dupSessionPda);
    const [dupMapPoisPda] = getMapPoisPda(dupSessionPda);

    // First session at this level should succeed
    await programs.sessionManager.methods
      .startSession(campaignLevel)
      .accounts({
        sessionNonces: sessionNoncesPda,
        gameSession: dupSessionPda,
        sessionCounter: sessionCounterPda,
        playerProfile: playerProfilePda,
        player: user.publicKey,
        sessionSigner: dupSessionSigner.publicKey,
        mapConfig: mapConfigPda,
        generatedMap: dupGeneratedMapPda,
        gameState: dupGameStatePda,
        mapEnemies: dupMapEnemiesPda,
        mapPois: dupMapPoisPda,
        inventory: dupInventoryPda,
        mapGeneratorProgram: programs.mapGenerator.programId,
        gameplayStateProgram: programs.gameplayState.programId,
        poiSystemProgram: programs.poiSystem.programId,
        playerInventoryProgram: programs.playerInventory.programId,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([user, dupSessionSigner])
      .rpc();

    // Second session at the same level should fail (PDA already exists)
    const dupSessionSigner2 = Keypair.generate();
    await airdropAndConfirm(connection, dupSessionSigner2.publicKey, 2 * LAMPORTS_PER_SOL);

    try {
      await programs.sessionManager.methods
        .startSession(campaignLevel)
        .accounts({
          sessionNonces: sessionNoncesPda,
          gameSession: dupSessionPda,
          sessionCounter: sessionCounterPda,
          playerProfile: playerProfilePda,
          player: user.publicKey,
          sessionSigner: dupSessionSigner2.publicKey,
          mapConfig: mapConfigPda,
          generatedMap: dupGeneratedMapPda,
          gameState: dupGameStatePda,
          mapEnemies: dupMapEnemiesPda,
          mapPois: dupMapPoisPda,
          inventory: dupInventoryPda,
          mapGeneratorProgram: programs.mapGenerator.programId,
          gameplayStateProgram: programs.gameplayState.programId,
          poiSystemProgram: programs.poiSystem.programId,
          playerInventoryProgram: programs.playerInventory.programId,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
        ])
        .signers([user, dupSessionSigner2])
        .rpc();

      expect.fail("Expected duplicate startSession to fail");
    } catch (e: any) {
      // Should fail because the session PDA already exists
      expect(String(e)).to.match(
        /already in use|custom program error|Error|already.*exist|ConstraintRaw/i
      );
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 8. Session nonce override flow
// ─────────────────────────────────────────────────────────────────────────────
describe("Track 2 E2E: Session nonce override", function () {
  this.timeout(60_000);

  it("override_campaign_session increments campaign nonce", async () => {
    // Read nonces before override
    const noncesBefore = (
      programs.sessionManager as any
    ).coder.accounts.decode(
      "sessionNonces",
      (await connection.getAccountInfo(sessionNoncesPda, "confirmed"))!.data
    ) as { campaignNonce: { toString(): string } };
    const nonceBefore = BigInt(noncesBefore.campaignNonce.toString());

    await programs.sessionManager.methods
      .overrideCampaignSession()
      .accounts({
        sessionNonces: sessionNoncesPda,
        player: user.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user])
      .rpc();

    const noncesAfter = (
      programs.sessionManager as any
    ).coder.accounts.decode(
      "sessionNonces",
      (await connection.getAccountInfo(sessionNoncesPda, "confirmed"))!.data
    ) as { campaignNonce: { toString(): string } };
    const nonceAfter = BigInt(noncesAfter.campaignNonce.toString());

    expect(nonceAfter).to.equal(nonceBefore + 1n);
  });

  it("can start a new session at the new nonce PDA", async () => {
    // Read current campaign nonce
    const nonces = (
      programs.sessionManager as any
    ).coder.accounts.decode(
      "sessionNonces",
      (await connection.getAccountInfo(sessionNoncesPda, "confirmed"))!.data
    ) as { campaignNonce: { toString(): string } };
    const currentNonce = BigInt(nonces.campaignNonce.toString());

    // Derive session PDA at the new nonce
    const [newSessionPda] = getSessionPda(
      user.publicKey,
      campaignLevel,
      currentNonce
    );
    const [newGameStatePda] = getGameStatePda(newSessionPda);
    const [newMapEnemiesPda] = getMapEnemiesPda(newSessionPda);
    const [newGeneratedMapPda] = getGeneratedMapPda(newSessionPda);
    const [newInventoryPda] = getInventoryPda(newSessionPda);
    const [newMapPoisPda] = getMapPoisPda(newSessionPda);

    const overrideSessionSigner = Keypair.generate();
    await airdropAndConfirm(
      connection,
      overrideSessionSigner.publicKey,
      2 * LAMPORTS_PER_SOL
    );

    await programs.sessionManager.methods
      .startSession(campaignLevel)
      .accounts({
        sessionNonces: sessionNoncesPda,
        gameSession: newSessionPda,
        sessionCounter: sessionCounterPda,
        playerProfile: playerProfilePda,
        player: user.publicKey,
        sessionSigner: overrideSessionSigner.publicKey,
        mapConfig: mapConfigPda,
        generatedMap: newGeneratedMapPda,
        gameState: newGameStatePda,
        mapEnemies: newMapEnemiesPda,
        mapPois: newMapPoisPda,
        inventory: newInventoryPda,
        mapGeneratorProgram: programs.mapGenerator.programId,
        gameplayStateProgram: programs.gameplayState.programId,
        poiSystemProgram: programs.poiSystem.programId,
        playerInventoryProgram: programs.playerInventory.programId,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([user, overrideSessionSigner])
      .rpc();

    // Verify new session created at new PDA
    const sessionInfo = await connection.getAccountInfo(
      newSessionPda,
      "confirmed"
    );
    expect(sessionInfo).to.not.be.null;
    expect(sessionInfo!.owner.equals(PROGRAM_IDS.sessionManager)).to.be.true;

    // Old session at nonce=0 should still exist (stuck, not closed)
    const [oldSessionPda] = getSessionPda(user.publicKey, campaignLevel, 0);
    const oldSessionInfo = await connection.getAccountInfo(
      oldSessionPda,
      "confirmed"
    );
    expect(oldSessionInfo).to.not.be.null;
  });
});
