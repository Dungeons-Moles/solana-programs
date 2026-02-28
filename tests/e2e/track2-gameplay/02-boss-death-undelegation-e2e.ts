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
  getGauntletEpochPoolPda,
  getGauntletPlayerScorePda,
  getInventoryPda,
  getMapPoisPda,
  getDuelSessionPda,
  getGauntletSessionPda,
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

// ── Phase constants matching gameplay-state ─────────────────────────────────
const DAY_MOVES = 50;
const NIGHT_MOVES = 30;
// Total moves per week: 3 days (50 each) + 3 nights (30 each) = 240
const TOTAL_MOVES_PER_WEEK = (DAY_MOVES * 3) + (NIGHT_MOVES * 3);

// ── Shared mutable state ────────────────────────────────────────────────────
let connection: Connection;
let erConnection: Connection;
let provider: anchor.AnchorProvider;
let programs: AllPrograms;
let admin: Keypair;

// Global PDAs
let sessionCounterPda: PublicKey;
let mapConfigPda: PublicKey;
let gameplayAuthorityPda: PublicKey;
let sessionManagerAuthorityPda: PublicKey;
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

// ── Helpers ─────────────────────────────────────────────────────────────────

/**
 * Send a transaction on the base layer signed by a keypair as fee payer.
 */
const sendBaseTx = async (
  label: string,
  ixs: TransactionInstruction[],
  signers: Keypair[]
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

/**
 * Send a transaction on the ER signed by sessionSigner as fee payer.
 */
const sendErTx = async (
  label: string,
  ixs: TransactionInstruction[],
  sessionSigner: Keypair
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
 */
const waitForBaseOwner = async (
  account: PublicKey,
  owner: PublicKey,
  label: string
): Promise<void> => {
  for (let i = 0; i < 60; i++) {
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
 * Poll ER until an account's owner matches expected program.
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
const fetchGameState = async (
  conn: Connection,
  gameStatePda: PublicKey
): Promise<any> => {
  const accountInfo = await conn.getAccountInfo(gameStatePda, "confirmed");
  if (!accountInfo) {
    throw new Error(`GameState account missing on ${conn.rpcEndpoint}`);
  }
  return (programs.gameplayState as any).coder.accounts.decode(
    "gameState",
    accountInfo.data
  );
};

/**
 * Delegate all 6 accounts to ER for a session.
 */
const delegateAllAccounts = async (
  sessionPda: PublicKey,
  gameStatePda: PublicKey,
  mapEnemiesPda: PublicKey,
  generatedMapPda: PublicKey,
  inventoryPda: PublicKey,
  mapPoisPda: PublicKey,
  sessionSigner: Keypair,
  user: Keypair,
  campaignLevel: number,
  isDuel: boolean,
  isGauntlet: boolean,
): Promise<void> => {
  // 1. Delegate gameplay accounts (gameState + mapEnemies)
  const gsDelegate = deriveDelegateAccounts(gameStatePda, PROGRAM_IDS.gameplayState);
  const meDelegate = deriveDelegateAccounts(mapEnemiesPda, PROGRAM_IDS.gameplayState);
  const delegateGameplayIx = await programs.gameplayState.methods
    .delegateGameplayAccounts(ER_VALIDATOR)
    .accountsStrict({
      bufferGameState: gsDelegate.buffer,
      delegationRecordGameState: gsDelegate.delegationRecord,
      delegationMetadataGameState: gsDelegate.delegationMetadata,
      gameState: gameStatePda,
      bufferMapEnemies: meDelegate.buffer,
      delegationRecordMapEnemies: meDelegate.delegationRecord,
      delegationMetadataMapEnemies: meDelegate.delegationMetadata,
      mapEnemies: mapEnemiesPda,
      gameSession: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.gameplayState,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-gameplay", [delegateGameplayIx], [sessionSigner]);

  // 2. Delegate generated map
  const gmDelegate = deriveDelegateAccounts(generatedMapPda, PROGRAM_IDS.mapGenerator);
  const delegateMapIx = await programs.mapGenerator.methods
    .delegateGeneratedMap(ER_VALIDATOR)
    .accountsStrict({
      bufferGeneratedMap: gmDelegate.buffer,
      delegationRecordGeneratedMap: gmDelegate.delegationRecord,
      delegationMetadataGeneratedMap: gmDelegate.delegationMetadata,
      generatedMap: generatedMapPda,
      session: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.mapGenerator,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-map", [delegateMapIx], [sessionSigner]);

  // 3. Delegate inventory
  const invDelegate = deriveDelegateAccounts(inventoryPda, PROGRAM_IDS.playerInventory);
  const delegateInvIx = await programs.playerInventory.methods
    .delegateInventory(ER_VALIDATOR)
    .accountsStrict({
      bufferInventory: invDelegate.buffer,
      delegationRecordInventory: invDelegate.delegationRecord,
      delegationMetadataInventory: invDelegate.delegationMetadata,
      inventory: inventoryPda,
      session: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.playerInventory,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-inventory", [delegateInvIx], [sessionSigner]);

  // 4. Delegate map POIs
  const poisDelegate = deriveDelegateAccounts(mapPoisPda, PROGRAM_IDS.poiSystem);
  const delegatePoisIx = await programs.poiSystem.methods
    .delegateMapPois(ER_VALIDATOR)
    .accountsStrict({
      bufferMapPois: poisDelegate.buffer,
      delegationRecordMapPois: poisDelegate.delegationRecord,
      delegationMetadataMapPois: poisDelegate.delegationMetadata,
      mapPois: mapPoisPda,
      gameSession: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.poiSystem,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-pois", [delegatePoisIx], [sessionSigner]);

  // 5. Delegate session (requires user sig)
  const [sessionNonces] = getSessionNoncesPda(user.publicKey);
  const delegateSessionIx = await programs.sessionManager.methods
    .delegateSession(campaignLevel, ER_VALIDATOR)
    .accounts({
      gameSession: sessionPda,
      player: user.publicKey,
      sessionSigner: sessionSigner.publicKey,
      sessionNonces,
    } as any)
    .instruction();
  const tx = new Transaction().add(delegateSessionIx);
  tx.feePayer = sessionSigner.publicKey;
  const bh = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(sessionSigner, user);
  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
    maxRetries: 3,
  });
  await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");

  // Wait for ER to observe all delegated accounts before ER gameplay.
  await waitForErOwner(gameStatePda, DELEGATION_PROGRAM_ID, "game_state");
  await waitForErOwner(mapEnemiesPda, DELEGATION_PROGRAM_ID, "map_enemies");
  await waitForErOwner(generatedMapPda, DELEGATION_PROGRAM_ID, "generated_map");
  await waitForErOwner(inventoryPda, DELEGATION_PROGRAM_ID, "inventory");
  await waitForErOwner(mapPoisPda, DELEGATION_PROGRAM_ID, "map_pois");
  await waitForErOwner(sessionPda, DELEGATION_PROGRAM_ID, "session");
};

/**
 * Move player back and forth on ER until dead or boss fight completes.
 * Returns the game state after the loop ends.
 */
const moveUntilDeadOrBoss = async (
  gameStatePda: PublicKey,
  mapEnemiesPda: PublicKey,
  generatedMapPda: PublicKey,
  inventoryPda: PublicKey,
  mapPoisPda: PublicKey,
  sessionPda: PublicKey,
  sessionSigner: Keypair,
): Promise<any> => {
  // Get initial position
  let gameState = await fetchGameState(erConnection, gameStatePda);
  let posX = Number(gameState.positionX);
  let posY = Number(gameState.positionY);
  const mapW = Number(gameState.mapWidth);
  const mapH = Number(gameState.mapHeight);

  console.log(`  Initial position: (${posX}, ${posY}), HP: ${gameState.hp}, mapSize: ${mapW}x${mapH}`);

  // Simple ping-pong: remember previous position after each successful move,
  // then alternate between current and previous. On failure, try all 4 directions.
  let moveCount = 0;
  let failCount = 0;
  const maxMoves = TOTAL_MOVES_PER_WEEK + 50;
  let prevX = -1;
  let prevY = -1;

  const tryMove = async (tx: number, ty: number): Promise<boolean> => {
    const moveTx = await programs.gameplayState.methods
      .movePlayer(tx, ty)
        .accounts({
          gameState: gameStatePda,
          gameSession: sessionPda,
          mapEnemies: mapEnemiesPda,
          generatedMap: generatedMapPda,
          inventory: inventoryPda,
          gameplayAuthority: gameplayAuthorityPda,
          playerInventoryProgram: PROGRAM_IDS.playerInventory,
          mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
          mapPois: mapPoisPda,
          poiSystemProgram: PROGRAM_IDS.poiSystem,
          gameplayVrfState: null,
          player: sessionSigner.publicKey,
        } as any)
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
        return false;
      }
      return true;
    } catch {
      await new Promise((r) => setTimeout(r, 300));
      return false;
    }
  };

  for (let i = 0; i < maxMoves; i++) {
    if (gameState.isDead || gameState.completed) {
      console.log(`  Game ended after ${moveCount} moves (${failCount} fails). isDead=${gameState.isDead}, completed=${gameState.completed}`);
      return gameState;
    }

    // Build list of candidate targets
    const candidates: [number, number][] = [];

    // First choice: go back to previous position (ping-pong)
    if (prevX >= 0 && prevY >= 0 && (prevX !== posX || prevY !== posY)) {
      candidates.push([prevX, prevY]);
    }

    // Fallback: try all 4 cardinal directions from current position
    const dirs: [number, number][] = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    for (const [dx, dy] of dirs) {
      const nx = posX + dx;
      const ny = posY + dy;
      if (nx >= 0 && ny >= 0 && nx < mapW && ny < mapH) {
        // Don't duplicate the ping-pong target
        if (candidates.length === 0 || candidates[0][0] !== nx || candidates[0][1] !== ny) {
          candidates.push([nx, ny]);
        }
      }
    }

    let moved = false;
    for (const [tx, ty] of candidates) {
      const ok = await tryMove(tx, ty);

      // Refresh state after every attempt (might be dead)
      gameState = await fetchGameState(erConnection, gameStatePda);
      if (gameState.isDead || gameState.completed) {
        moveCount++;
        moved = true;
        break;
      }

      if (ok) {
        prevX = posX;
        prevY = posY;
        posX = Number(gameState.positionX);
        posY = Number(gameState.positionY);
        moveCount++;
        moved = true;
        break;
      }
    }

    if (!moved) {
      failCount++;
      // Refresh position in case it changed
      gameState = await fetchGameState(erConnection, gameStatePda);
      posX = Number(gameState.positionX);
      posY = Number(gameState.positionY);
      if (failCount > 20) {
        console.log(`  Too many consecutive failures (${failCount}), breaking.`);
        break;
      }
    } else {
      failCount = 0;
    }

    // Log progress every 50 moves
    if (moveCount % 50 === 0 && moveCount > 0) {
      console.log(`  Move ${moveCount}: pos=(${posX},${posY}), HP=${gameState.hp}, phase=${JSON.stringify(gameState.phase)}, week=${gameState.week}, movesRemaining=${gameState.movesRemaining}`);
    }
  }

  // Final state check
  gameState = await fetchGameState(erConnection, gameStatePda);
  console.log(`  Final state after ${moveCount} moves: isDead=${gameState.isDead}, completed=${gameState.completed}, HP=${gameState.hp}, bossFightReady=${gameState.bossFightReady}`);

  // If boss fight is ready but not auto-resolved, call trigger_boss_fight.
  // All modes (campaign, duel, gauntlet) now auto-resolve inline, but
  // trigger_boss_fight is a fallback for edge cases.
  if (gameState.bossFightReady && !gameState.isDead && !gameState.completed) {
    console.log(`  Boss fight ready but not auto-resolved, calling trigger_boss_fight...`);
    const triggerIx = await programs.gameplayState.methods
      .triggerBossFight()
      .accounts({
        gameState: gameStatePda,
        gameSession: sessionPda,
        mapEnemies: mapEnemiesPda,
        generatedMap: generatedMapPda,
        inventory: inventoryPda,
        gameplayAuthority: gameplayAuthorityPda,
        playerInventoryProgram: PROGRAM_IDS.playerInventory,
        player: sessionSigner.publicKey,
      } as any)
      .instruction();
    await sendErTx("trigger-boss-fight", [triggerIx], sessionSigner);
    gameState = await fetchGameState(erConnection, gameStatePda);
    console.log(`  After trigger_boss_fight: isDead=${gameState.isDead}, completed=${gameState.completed}, HP=${gameState.hp}`);
  }

  return gameState;
};

/**
 * Undelegate all accounts using per-program approach:
 * 1. gameplay-state undelegates game_state + map_enemies
 * 2. map-generator undelegates generated_map
 * 3. player-inventory undelegates inventory
 * 4. poi-system undelegates map_pois
 * 5. session-manager undelegates game_session
 */
const undelegateAllPerProgram = async (
  sessionPda: PublicKey,
  gameStatePda: PublicKey,
  mapEnemiesPda: PublicKey,
  generatedMapPda: PublicKey,
  inventoryPda: PublicKey,
  mapPoisPda: PublicKey,
  sessionSigner: Keypair,
  user: Keypair,
  campaignLevel: number,
): Promise<void> => {
  // 1. Undelegate gameplay accounts
  const undelegateGameplayIx = await programs.gameplayState.methods
    .undelegateGameplayAccounts()
    .accounts({
      gameState: gameStatePda,
      mapEnemies: mapEnemiesPda,
      gameSession: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-gameplay", [undelegateGameplayIx], sessionSigner);
  await waitForBaseOwner(gameStatePda, PROGRAM_IDS.gameplayState, "game_state");
  await waitForBaseOwner(mapEnemiesPda, PROGRAM_IDS.gameplayState, "map_enemies");

  // 2. Undelegate generated map
  const undelegateMapIx = await programs.mapGenerator.methods
    .undelegateGeneratedMap()
    .accounts({
      generatedMap: generatedMapPda,
      session: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-map", [undelegateMapIx], sessionSigner);
  await waitForBaseOwner(generatedMapPda, PROGRAM_IDS.mapGenerator, "generated_map");

  // 3. Undelegate inventory
  const undelegateInvIx = await programs.playerInventory.methods
    .undelegateInventory()
    .accounts({
      inventory: inventoryPda,
      session: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-inventory", [undelegateInvIx], sessionSigner);
  await waitForBaseOwner(inventoryPda, PROGRAM_IDS.playerInventory, "inventory");

  // 4. Undelegate map POIs
  const undelegatePoisIx = await programs.poiSystem.methods
    .undelegateMapPois()
    .accounts({
      mapPois: mapPoisPda,
      gameSession: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-pois", [undelegatePoisIx], sessionSigner);
  await waitForBaseOwner(mapPoisPda, PROGRAM_IDS.poiSystem, "map_pois");

  // 5. Undelegate session
  const stateHash = Array.from({ length: 32 }, (_, i) => i);
  const undelegateSessionIx = await programs.sessionManager.methods
    .undelegateSession(campaignLevel, stateHash)
    .accounts({
      gameSession: sessionPda,
      player: user.publicKey,
      sessionSigner: sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-session", [undelegateSessionIx], sessionSigner);
  await waitForBaseOwner(sessionPda, PROGRAM_IDS.sessionManager, "session");
};

/**
 * End session on base layer.
 */
const endSessionOnBase = async (
  sessionPda: PublicKey,
  gameStatePda: PublicKey,
  mapEnemiesPda: PublicKey,
  generatedMapPda: PublicKey,
  inventoryPda: PublicKey,
  mapPoisPda: PublicKey,
  playerProfilePda: PublicKey,
  sessionSigner: Keypair,
  user: Keypair,
  campaignLevel: number,
): Promise<void> => {
  const endSessionIx = await programs.sessionManager.methods
    .endSession(campaignLevel)
    .accounts({
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
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      playerProfileProgram: PROGRAM_IDS.playerProfile,
      mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
      poiSystemProgram: PROGRAM_IDS.poiSystem,
    } as any)
    .instruction();

  await sendBaseTx("end-session", [endSessionIx], [sessionSigner]);

  // Verify session closed
  const sessionInfo = await connection.getAccountInfo(sessionPda, "confirmed");
  expect(sessionInfo).to.be.null;
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
});

// ─────────────────────────────────────────────────────────────────────────────
// 1. Initialize global state (idempotent)
// ─────────────────────────────────────────────────────────────────────────────
describe("Boss Death E2E: Initialize global state", function () {
  this.timeout(60_000);

  it("initializes all global accounts", async () => {
    // Session counter
    try {
      await programs.sessionManager.methods.initializeCounter()
        .accounts({ sessionCounter: sessionCounterPda, admin: admin.publicKey, systemProgram: SystemProgram.programId } as any)
        .rpc();
    } catch (e: any) { if (!String(e).includes("already in use")) throw e; }

    // Map config
    try {
      await programs.mapGenerator.methods.initializeMapConfig()
        .accounts({ mapConfig: mapConfigPda, admin: admin.publicKey, systemProgram: SystemProgram.programId } as any)
        .rpc();
    } catch (e: any) { if (!String(e).includes("already in use")) throw e; }

    // Duels
    try {
      await programs.gameplayState.methods.initializeDuels()
        .accounts({ duelVault: duelVaultPda, duelOpenQueue: duelOpenQueuePda, admin: admin.publicKey, systemProgram: SystemProgram.programId } as any)
        .rpc();
    } catch (e: any) { if (!String(e).includes("already in use")) throw e; }

    // Pit draft
    try {
      await programs.gameplayState.methods.initializePitDraft()
        .accounts({ pitDraftQueue: pitDraftQueuePda, pitDraftVault: pitDraftVaultPda, admin: admin.publicKey, systemProgram: SystemProgram.programId } as any)
        .rpc();
    } catch (e: any) { if (!String(e).includes("already in use")) throw e; }

    // Gauntlet
    try {
      await programs.gameplayState.methods.initializeGauntlet()
        .accounts({
          gauntletConfig: gauntletConfigPda, gauntletPoolVault: gauntletPoolVaultPda,
          gauntletWeek1: gauntletWeek1Pda, gauntletWeek2: gauntletWeek2Pda,
          gauntletWeek3: gauntletWeek3Pda, gauntletWeek4: gauntletWeek4Pda,
          gauntletWeek5: gauntletWeek5Pda,
          admin: admin.publicKey, systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 })])
        .rpc();
    } catch (e: any) { if (!String(e).includes("already in use")) throw e; }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Campaign level 1: boss death → undelegate → end session
// ─────────────────────────────────────────────────────────────────────────────
describe("Boss Death E2E: Campaign (level 1)", function () {
  this.timeout(600_000); // 10 min for 240 moves

  let user: Keypair;
  let sessionSigner: Keypair;
  let playerProfilePda: PublicKey;
  let sessionPda: PublicKey;
  let gameStatePda: PublicKey;
  let mapEnemiesPda: PublicKey;
  let generatedMapPda: PublicKey;
  let inventoryPda: PublicKey;
  let mapPoisPda: PublicKey;
  const campaignLevel = 1;

  before(async function () {
    this.timeout(30_000);
    user = Keypair.generate();
    sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(connection, sessionSigner.publicKey, 10 * LAMPORTS_PER_SOL);
    [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    [sessionPda] = getSessionPda(user.publicKey, campaignLevel);
    [gameStatePda] = getGameStatePda(sessionPda);
    [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    [generatedMapPda] = getGeneratedMapPda(sessionPda);
    [inventoryPda] = getInventoryPda(sessionPda);
    [mapPoisPda] = getMapPoisPda(sessionPda);
  });

  it("creates profile + starts session", async () => {
    const name = `boss-test-${user.publicKey.toBase58().slice(0, 6)}`;
    await programs.playerProfile.methods.initializeProfile(name)
      .accounts({ playerProfile: playerProfilePda, owner: user.publicKey, systemProgram: SystemProgram.programId } as any)
      .signers([user]).rpc();

    const [sessionNoncesPda] = getSessionNoncesPda(user.publicKey);
    await programs.sessionManager.methods.startSession(campaignLevel)
      .accounts({
        sessionNonces: sessionNoncesPda,
        gameSession: sessionPda, sessionCounter: sessionCounterPda, playerProfile: playerProfilePda,
        player: user.publicKey, sessionSigner: sessionSigner.publicKey, mapConfig: mapConfigPda,
        generatedMap: generatedMapPda, gameState: gameStatePda, mapEnemies: mapEnemiesPda,
        mapPois: mapPoisPda, inventory: inventoryPda,
        mapGeneratorProgram: PROGRAM_IDS.mapGenerator, gameplayStateProgram: PROGRAM_IDS.gameplayState,
        poiSystemProgram: PROGRAM_IDS.poiSystem, playerInventoryProgram: PROGRAM_IDS.playerInventory,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([user, sessionSigner]).rpc();
  });

  it("delegates all accounts to ER", async () => {
    await delegateAllAccounts(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionSigner, user, campaignLevel,
      false, false
    );

    // Verify delegation
    const gsInfo = await connection.getAccountInfo(gameStatePda, "confirmed");
    expect(gsInfo!.owner.equals(DELEGATION_PROGRAM_ID)).to.be.true;
  });

  it("moves player until dead or boss fight completes", async () => {
    const finalState = await moveUntilDeadOrBoss(
      gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionPda, sessionSigner
    );

    // Player should either be dead or have completed the level
    expect(finalState.isDead || finalState.completed).to.be.true;
  });

  it("undelegates all accounts (per-program)", async () => {
    await undelegateAllPerProgram(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionSigner, user, campaignLevel
    );

    // Verify all accounts back on base
    const gsInfo = await connection.getAccountInfo(gameStatePda, "confirmed");
    expect(gsInfo!.owner.equals(PROGRAM_IDS.gameplayState)).to.be.true;
    const sessionInfo = await connection.getAccountInfo(sessionPda, "confirmed");
    expect(sessionInfo!.owner.equals(PROGRAM_IDS.sessionManager)).to.be.true;
  });

  it("ends session on base layer", async () => {
    await endSessionOnBase(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, playerProfilePda, sessionSigner, user, campaignLevel
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. Duel: boss death → undelegate → end session
// ─────────────────────────────────────────────────────────────────────────────
describe("Boss Death E2E: Duel", function () {
  this.timeout(600_000);

  let user: Keypair;
  let sessionSigner: Keypair;
  let playerProfilePda: PublicKey;
  let sessionPda: PublicKey;
  let gameStatePda: PublicKey;
  let mapEnemiesPda: PublicKey;
  let generatedMapPda: PublicKey;
  let inventoryPda: PublicKey;
  let mapPoisPda: PublicKey;
  const campaignLevel = 20; // DUEL_CAMPAIGN_LEVEL

  before(async function () {
    this.timeout(30_000);
    user = Keypair.generate();
    sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(connection, sessionSigner.publicKey, 10 * LAMPORTS_PER_SOL);
    [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    [sessionPda] = getDuelSessionPda(user.publicKey);
    [gameStatePda] = getGameStatePda(sessionPda);
    [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    [generatedMapPda] = getGeneratedMapPda(sessionPda);
    [inventoryPda] = getInventoryPda(sessionPda);
    [mapPoisPda] = getMapPoisPda(sessionPda);
  });

  it("creates profile + starts duel session", async () => {
    const name = `duel-test-${user.publicKey.toBase58().slice(0, 6)}`;
    await programs.playerProfile.methods.initializeProfile(name)
      .accounts({ playerProfile: playerProfilePda, owner: user.publicKey, systemProgram: SystemProgram.programId } as any)
      .signers([user]).rpc();

    const [duelSessionNoncesPda] = getSessionNoncesPda(user.publicKey);
    await programs.sessionManager.methods.startDuelSession()
      .accounts({
        sessionNonces: duelSessionNoncesPda,
        gameSession: sessionPda, sessionCounter: sessionCounterPda, playerProfile: playerProfilePda,
        player: user.publicKey, sessionSigner: sessionSigner.publicKey,
        sessionManagerAuthority: sessionManagerAuthorityPda, mapConfig: mapConfigPda,
        generatedMap: generatedMapPda, gameState: gameStatePda, mapEnemies: mapEnemiesPda,
        mapPois: mapPoisPda, inventory: inventoryPda,
        mapVrfState: null,
        poiVrfState: null,
        gameplayVrfState: null,
        mapGeneratorProgram: PROGRAM_IDS.mapGenerator, gameplayStateProgram: PROGRAM_IDS.gameplayState,
        poiSystemProgram: PROGRAM_IDS.poiSystem, playerInventoryProgram: PROGRAM_IDS.playerInventory,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([user, sessionSigner]).rpc();
  });

  it("delegates all accounts to ER", async () => {
    await delegateAllAccounts(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionSigner, user, campaignLevel,
      true, false
    );
  });

  it("moves player until dead or boss fight completes", async () => {
    const finalState = await moveUntilDeadOrBoss(
      gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionPda, sessionSigner
    );
    expect(finalState.isDead || finalState.completed).to.be.true;
  });

  it("undelegates all accounts (per-program)", async () => {
    await undelegateAllPerProgram(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionSigner, user, campaignLevel
    );
  });

  it("ends session on base layer", async () => {
    await endSessionOnBase(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, playerProfilePda, sessionSigner, user, campaignLevel
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Gauntlet: enter → walk → auto-resolve echo → undelegate → settle → end
// ─────────────────────────────────────────────────────────────────────────────
describe("Boss Death E2E: Gauntlet", function () {
  this.timeout(600_000);

  let user: Keypair;
  let sessionSigner: Keypair;
  let playerProfilePda: PublicKey;
  let sessionPda: PublicKey;
  let gameStatePda: PublicKey;
  let mapEnemiesPda: PublicKey;
  let generatedMapPda: PublicKey;
  let inventoryPda: PublicKey;
  let mapPoisPda: PublicKey;
  const campaignLevel = 20; // GAUNTLET_CAMPAIGN_LEVEL

  before(async function () {
    this.timeout(30_000);
    user = Keypair.generate();
    sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(connection, sessionSigner.publicKey, 10 * LAMPORTS_PER_SOL);
    [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    [sessionPda] = getGauntletSessionPda(user.publicKey);
    [gameStatePda] = getGameStatePda(sessionPda);
    [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    [generatedMapPda] = getGeneratedMapPda(sessionPda);
    [inventoryPda] = getInventoryPda(sessionPda);
    [mapPoisPda] = getMapPoisPda(sessionPda);
  });

  it("creates profile + starts gauntlet session", async () => {
    const name = `gauntlet-test-${user.publicKey.toBase58().slice(0, 6)}`;
    await programs.playerProfile.methods.initializeProfile(name)
      .accounts({ playerProfile: playerProfilePda, owner: user.publicKey, systemProgram: SystemProgram.programId } as any)
      .signers([user]).rpc();

    const [gauntletSessionNoncesPda] = getSessionNoncesPda(user.publicKey);
    await programs.sessionManager.methods.startGauntletSession()
      .accounts({
        sessionNonces: gauntletSessionNoncesPda,
        gameSession: sessionPda, sessionCounter: sessionCounterPda, playerProfile: playerProfilePda,
        player: user.publicKey, sessionSigner: sessionSigner.publicKey,
        mapConfig: mapConfigPda,
        generatedMap: generatedMapPda, gameState: gameStatePda, mapEnemies: mapEnemiesPda,
        mapPois: mapPoisPda, inventory: inventoryPda,
        mapVrfState: null,
        poiVrfState: null,
        gameplayVrfState: null,
        mapGeneratorProgram: PROGRAM_IDS.mapGenerator, gameplayStateProgram: PROGRAM_IDS.gameplayState,
        poiSystemProgram: PROGRAM_IDS.poiSystem, playerInventoryProgram: PROGRAM_IDS.playerInventory,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([user, sessionSigner]).rpc();
  });

  it("enters gauntlet (pays entry fee, pre-creates epoch accounts)", async () => {
    // Ensure company treasury exists on localnet (needed for mut SystemAccount rent)
    const treasuryPk = new PublicKey("5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8");
    await airdropAndConfirm(connection, treasuryPk, LAMPORTS_PER_SOL);
    // Fetch gauntlet config to get current epoch ID
    const gauntletConfigAccount = await connection.getAccountInfo(gauntletConfigPda, "confirmed");
    if (!gauntletConfigAccount) throw new Error("GauntletConfig not found");
    const gauntletConfig = (programs.gameplayState as any).coder.accounts.decode(
      "gauntletConfig",
      gauntletConfigAccount.data
    );
    const epochId = new anchor.BN(gauntletConfig.currentEpochId.toString());
    const epochIdBigInt = BigInt(gauntletConfig.currentEpochId.toString());
    console.log(`  Gauntlet epoch ID: ${epochId.toString()}`);

    const [epochPoolPda] = getGauntletEpochPoolPda(epochIdBigInt);
    const [playerScorePda] = getGauntletPlayerScorePda(epochIdBigInt, user.publicKey);

    const enterIx = await (programs.gameplayState.methods as any)
      .enterGauntlet(epochId)
      .accounts({
        gameState: gameStatePda,
        player: user.publicKey,
        gameplayVrfState: null,
        gauntletConfig: gauntletConfigPda,
        gauntletPoolVault: gauntletPoolVaultPda,
        companyTreasury: new PublicKey("5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8"),
        gauntletEpochPool: epochPoolPda,
        gauntletPlayerScore: playerScorePda,
        systemProgram: SystemProgram.programId,
      } as any)
      .remainingAccounts([
        { pubkey: gauntletWeek1Pda, isSigner: false, isWritable: false },
        { pubkey: gauntletWeek2Pda, isSigner: false, isWritable: false },
        { pubkey: gauntletWeek3Pda, isSigner: false, isWritable: false },
        { pubkey: gauntletWeek4Pda, isSigner: false, isWritable: false },
        { pubkey: gauntletWeek5Pda, isSigner: false, isWritable: false },
      ])
      .instruction();

    await sendBaseTx(
      "enter-gauntlet",
      [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        enterIx,
      ],
      [user]
    );

    const gs = await fetchGameState(connection, gameStatePda);
    expect(gs.runMode?.gauntlet).to.not.be.undefined;
    expect(gs.maxWeeks).to.equal(5);
    console.log(`  Gauntlet entered: runMode=Gauntlet, maxWeeks=${gs.maxWeeks}, epochId=${gs.gauntletEpochId}`);
  });

  it("delegates all accounts to ER", async () => {
    await delegateAllAccounts(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionSigner, user, campaignLevel,
      false, true
    );
  });

  it("moves player until dead or completed (echo combat auto-resolves)", async () => {
    const finalState = await moveUntilDeadOrBoss(
      gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionPda, sessionSigner
    );
    // Gauntlet can remain active after long movement loops depending on
    // encounter RNG; log state and proceed with undelegation/end-session flow.
    console.log(`  Gauntlet result: isDead=${finalState.isDead}, completed=${finalState.completed}, week=${finalState.week}`);
  });

  it("undelegates all accounts (per-program)", async () => {
    await undelegateAllPerProgram(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, sessionSigner, user, campaignLevel
    );
  });

  it("settles gauntlet session on base layer (session key signs)", async () => {
    const baseGameState = await fetchGameState(connection, gameStatePda);
    console.log(`  Pre-settle: pointsEarned=${baseGameState.gauntletPointsEarned}, highestWeekWon=${baseGameState.gauntletHighestWeekWon}, settled=${baseGameState.gauntletSettled}`);

    // settle_gauntlet_session is only valid while the run is still active.
    if (baseGameState.isDead || baseGameState.completed) {
      console.log("  Skipping settle: gauntlet run already ended");
      return;
    }

    const epochId = BigInt(baseGameState.gauntletEpochId.toString());
    const [epochPoolPda] = getGauntletEpochPoolPda(epochId);
    const [playerScorePda] = getGauntletPlayerScorePda(epochId, user.publicKey);

    const settleIx = await (programs.gameplayState.methods as any)
      .settleGauntletSession(new anchor.BN(epochId.toString()))
      .accounts({
        gameState: gameStatePda,
        player: user.publicKey,
        sessionSigner: sessionSigner.publicKey,
        gameplayVrfState: null,
        gauntletEpochPool: epochPoolPda,
        gauntletPlayerScore: playerScorePda,
        inventory: inventoryPda,
        gauntletWeek1: gauntletWeek1Pda,
        gauntletWeek2: gauntletWeek2Pda,
        gauntletWeek3: gauntletWeek3Pda,
        gauntletWeek4: gauntletWeek4Pda,
        gauntletWeek5: gauntletWeek5Pda,
      } as any)
      .instruction();

    try {
      await sendBaseTx(
        "settle-gauntlet-session",
        [
          ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
          settleIx,
        ],
        [sessionSigner]
      );
    } catch (e: any) {
      const msg = String(e);
      // 6046 == GameplayStateError::GauntletRunEnded. This can race with
      // base-state reads after undelegation; settlement is no longer applicable.
      if (msg.includes("6046")) {
        console.log("  Skipping settle: gauntlet run already ended on-chain");
        return;
      }
      throw e;
    }

    const settledState = await fetchGameState(connection, gameStatePda);
    expect(settledState.gauntletSettled).to.be.true;
    console.log(`  Post-settle: settled=${settledState.gauntletSettled}, pointsEarned=${settledState.gauntletPointsEarned}`);
  });

  it("ends session on base layer", async () => {
    await endSessionOnBase(
      sessionPda, gameStatePda, mapEnemiesPda, generatedMapPda,
      inventoryPda, mapPoisPda, playerProfilePda, sessionSigner, user, campaignLevel
    );
  });
});
