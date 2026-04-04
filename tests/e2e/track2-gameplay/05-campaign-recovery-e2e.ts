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
  getGameplayAuthorityPda,
  getInventoryPda,
  getMapPoisPda,
  getSessionDiscoveryPda,
  deriveDelegateAccounts,
} from "../shared/pda-helpers";
import { Transaction, TransactionInstruction, ComputeBudgetProgram } from "@solana/web3.js";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const ER_RPC_URL =
  process.env.EXPO_PUBLIC_EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";

const DELEGATION_PROGRAM_ID = PROGRAM_IDS.delegation;
const ER_VALIDATOR = new PublicKey("mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev");
const CAMPAIGN_LEVEL = 1;

type CampaignContext = {
  user: Keypair;
  sessionSigner: Keypair;
  playerProfilePda: PublicKey;
  sessionPda: PublicKey;
  gameStatePda: PublicKey;
  generatedMapPda: PublicKey;
  inventoryPda: PublicKey;
  mapPoisPda: PublicKey;
  sessionDiscoveryPda: PublicKey;
};

let connection: Connection;
let erConnection: Connection;
let provider: anchor.AnchorProvider;
let programs: AllPrograms;
let admin: Keypair;

let sessionCounterPda: PublicKey;
let mapConfigPda: PublicKey;
let gameplayAuthorityPda: PublicKey;
let sessionManagerAuthorityPda: PublicKey;

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

const sendErTx = async (
  label: string,
  ixs: TransactionInstruction[],
  signer: Keypair,
): Promise<string> => {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = signer.publicKey;
  const bh = await erConnection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(signer);
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
  for (let i = 0; i < 60; i += 1) {
    const info = await connection.getAccountInfo(account, "confirmed");
    if (info?.owner.equals(owner)) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  const info = await connection.getAccountInfo(account, "confirmed");
  throw new Error(
    `${label} owner did not restore (current=${info?.owner.toBase58() ?? "missing"})`,
  );
};

const waitForErVisibility = async (account: PublicKey, label: string): Promise<void> => {
  for (let i = 0; i < 60; i += 1) {
    const info = await erConnection.getAccountInfo(account, "confirmed");
    if (info) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`${label} account did not become visible on ER`);
};

const fetchGameState = async (conn: Connection, gameStatePda: PublicKey): Promise<any> => {
  const info = await conn.getAccountInfo(gameStatePda, "confirmed");
  if (!info) {
    throw new Error(`gameState missing on ${conn.rpcEndpoint}`);
  }
  return (programs.gameplayState as any).coder.accounts.decode("gameState", info.data);
};

const ensureGlobalState = async (): Promise<void> => {
  try {
    await programs.sessionManager.methods
      .initializeCounter()
      .accounts({
        sessionCounter: sessionCounterPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();
  } catch (error: any) {
    if (!String(error).includes("already in use")) throw error;
  }

  try {
    await programs.mapGenerator.methods
      .initializeMapConfig()
      .accounts({
        mapConfig: mapConfigPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();
  } catch (error: any) {
    if (!String(error).includes("already in use")) throw error;
  }
};

const setupCampaignSession = async (namePrefix: string): Promise<CampaignContext> => {
  const user = Keypair.generate();
  const sessionSigner = Keypair.generate();

  await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
  await airdropAndConfirm(connection, sessionSigner.publicKey, 5 * LAMPORTS_PER_SOL);

  const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
  const [sessionPda] = getSessionPda(user.publicKey, CAMPAIGN_LEVEL);
  const [gameStatePda] = getGameStatePda(sessionPda);
  const [generatedMapPda] = getGeneratedMapPda(sessionPda);
  const [inventoryPda] = getInventoryPda(sessionPda);
  const [mapPoisPda] = getMapPoisPda(sessionPda);
  const [sessionDiscoveryPda] = getSessionDiscoveryPda(sessionPda);
  const [sessionNoncesPda] = getSessionNoncesPda(user.publicKey);

  await programs.playerProfile.methods
    .initializeProfile(`${namePrefix}-${user.publicKey.toBase58().slice(0, 6)}`)
    .accounts({
      playerProfile: playerProfilePda,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([user])
    .rpc();

  await programs.sessionManager.methods
    .startSession(CAMPAIGN_LEVEL)
    .accounts({
      sessionNonces: sessionNoncesPda,
      gameSession: sessionPda,
      sessionCounter: sessionCounterPda,
      playerProfile: playerProfilePda,
      playerRelicPool: null,
      player: user.publicKey,
      sessionSigner: sessionSigner.publicKey,
      mapConfig: mapConfigPda,
      generatedMap: generatedMapPda,
      sessionDiscovery: sessionDiscoveryPda,
      gameState: gameStatePda,
      mapPois: mapPoisPda,
      inventory: inventoryPda,
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
    .signers([user, sessionSigner])
    .rpc();

  return {
    user,
    sessionSigner,
    playerProfilePda,
    sessionPda,
    gameStatePda,
    generatedMapPda,
    inventoryPda,
    mapPoisPda,
    sessionDiscoveryPda,
  };
};

const delegateAllAccounts = async (
  ctx: CampaignContext,
  options?: { includeSession?: boolean },
): Promise<void> => {
  const includeSession = options?.includeSession ?? true;
  const gsDelegate = deriveDelegateAccounts(ctx.gameStatePda, PROGRAM_IDS.gameplayState);
  const gmDelegate = deriveDelegateAccounts(ctx.generatedMapPda, PROGRAM_IDS.mapGenerator);
  const invDelegate = deriveDelegateAccounts(ctx.inventoryPda, PROGRAM_IDS.playerInventory);
  const poisDelegate = deriveDelegateAccounts(ctx.mapPoisPda, PROGRAM_IDS.poiSystem);
  const [sessionNoncesPda] = getSessionNoncesPda(ctx.user.publicKey);

  const delegateGameplayIx = await programs.gameplayState.methods
    .delegateGameplayAccounts(ER_VALIDATOR)
    .accountsStrict({
      bufferGameState: gsDelegate.buffer,
      delegationRecordGameState: gsDelegate.delegationRecord,
      delegationMetadataGameState: gsDelegate.delegationMetadata,
      gameState: ctx.gameStatePda,
      gameSession: ctx.sessionPda,
      player: ctx.sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.gameplayState,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-gameplay", [delegateGameplayIx], [ctx.sessionSigner]);

  const delegateMapIx = await programs.mapGenerator.methods
    .delegateGeneratedMap(ER_VALIDATOR)
    .accountsStrict({
      bufferGeneratedMap: gmDelegate.buffer,
      delegationRecordGeneratedMap: gmDelegate.delegationRecord,
      delegationMetadataGeneratedMap: gmDelegate.delegationMetadata,
      generatedMap: ctx.generatedMapPda,
      session: ctx.sessionPda,
      player: ctx.sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.mapGenerator,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-map", [delegateMapIx], [ctx.sessionSigner]);

  const delegateInvIx = await programs.playerInventory.methods
    .delegateInventory(ER_VALIDATOR)
    .accountsStrict({
      bufferInventory: invDelegate.buffer,
      delegationRecordInventory: invDelegate.delegationRecord,
      delegationMetadataInventory: invDelegate.delegationMetadata,
      inventory: ctx.inventoryPda,
      session: ctx.sessionPda,
      player: ctx.sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.playerInventory,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-inventory", [delegateInvIx], [ctx.sessionSigner]);

  const delegatePoisIx = await programs.poiSystem.methods
    .delegateMapPois(ER_VALIDATOR)
    .accountsStrict({
      bufferMapPois: poisDelegate.buffer,
      delegationRecordMapPois: poisDelegate.delegationRecord,
      delegationMetadataMapPois: poisDelegate.delegationMetadata,
      mapPois: ctx.mapPoisPda,
      gameSession: ctx.sessionPda,
      player: ctx.sessionSigner.publicKey,
      ownerProgram: PROGRAM_IDS.poiSystem,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-pois", [delegatePoisIx], [ctx.sessionSigner]);

  if (includeSession) {
    const delegateSessionIx = await programs.sessionManager.methods
      .delegateSession(CAMPAIGN_LEVEL, ER_VALIDATOR)
      .accounts({
        gameSession: ctx.sessionPda,
        player: ctx.user.publicKey,
        sessionSigner: ctx.sessionSigner.publicKey,
        sessionNonces: sessionNoncesPda,
      } as any)
      .instruction();
    const tx = new Transaction().add(delegateSessionIx);
    tx.feePayer = ctx.sessionSigner.publicKey;
    const bh = await connection.getLatestBlockhash("confirmed");
    tx.recentBlockhash = bh.blockhash;
    tx.sign(ctx.sessionSigner, ctx.user);
    const sig = await connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
  }

  await waitForErVisibility(ctx.gameStatePda, "game_state");
  await waitForErVisibility(ctx.generatedMapPda, "generated_map");
  await waitForErVisibility(ctx.inventoryPda, "inventory");
  await waitForErVisibility(ctx.mapPoisPda, "map_pois");
  if (includeSession) {
    await waitForErVisibility(ctx.sessionPda, "session");
  }
};

const undelegateAllPerProgram = async (
  ctx: CampaignContext,
  options?: { includeSession?: boolean },
): Promise<void> => {
  const includeSession = options?.includeSession ?? true;
  const undelegateGameplayIx = await programs.gameplayState.methods
    .undelegateGameplayAccounts()
    .accounts({
      gameState: ctx.gameStatePda,
      gameSession: ctx.sessionPda,
      sessionSigner: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-gameplay", [undelegateGameplayIx], ctx.sessionSigner);
  await waitForBaseOwner(ctx.gameStatePda, PROGRAM_IDS.gameplayState, "game_state");

  const undelegateMapIx = await programs.mapGenerator.methods
    .undelegateGeneratedMap()
    .accounts({
      generatedMap: ctx.generatedMapPda,
      session: ctx.sessionPda,
      sessionSigner: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-map", [undelegateMapIx], ctx.sessionSigner);
  await waitForBaseOwner(ctx.generatedMapPda, PROGRAM_IDS.mapGenerator, "generated_map");

  const undelegateInvIx = await programs.playerInventory.methods
    .undelegateInventory()
    .accounts({
      inventory: ctx.inventoryPda,
      session: ctx.sessionPda,
      sessionSigner: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-inventory", [undelegateInvIx], ctx.sessionSigner);
  await waitForBaseOwner(ctx.inventoryPda, PROGRAM_IDS.playerInventory, "inventory");

  const undelegatePoisIx = await programs.poiSystem.methods
    .undelegateMapPois()
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameSession: ctx.sessionPda,
      sessionSigner: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
  await sendErTx("undelegate-pois", [undelegatePoisIx], ctx.sessionSigner);
  await waitForBaseOwner(ctx.mapPoisPda, PROGRAM_IDS.poiSystem, "map_pois");

  if (includeSession) {
    const stateHash = Array.from({ length: 32 }, (_, i) => i);
    const undelegateSessionIx = await programs.sessionManager.methods
      .undelegateSession(CAMPAIGN_LEVEL, stateHash)
      .accounts({
        gameSession: ctx.sessionPda,
        player: ctx.user.publicKey,
        sessionSigner: ctx.sessionSigner.publicKey,
      } as any)
      .instruction();
    await sendErTx("undelegate-session", [undelegateSessionIx], ctx.sessionSigner);
    await waitForBaseOwner(ctx.sessionPda, PROGRAM_IDS.sessionManager, "session");
  }
};

describe("Campaign Recovery E2E", function () {
  this.timeout(240_000);

  before(async function () {
    admin = loadWalletKeypair();
    const wallet = walletFromKeypair(admin);
    connection = new Connection(RPC_URL, "confirmed");
    erConnection = new Connection(ER_RPC_URL, "confirmed");
    provider = createProvider(RPC_URL, wallet);
    anchor.setProvider(provider);
    programs = loadAllPrograms(provider);

    [sessionCounterPda] = getSessionCounterPda();
    [mapConfigPda] = getMapConfigPda();
    [gameplayAuthorityPda] = getGameplayAuthorityPda();
    [sessionManagerAuthorityPda] = getSessionManagerAuthorityPda();

    await ensureGlobalState();
  });

  it("reaches boss-ready campaign state on ER and ends cleanly after synthetic completion", async function () {
    const ctx = await setupCampaignSession("campaign-progress");

    await delegateAllAccounts(ctx, { includeSession: true });

    const skipToEowIx = await (programs.gameplayState.methods as any)
      .skipToEow()
      .accounts({
        gameState: ctx.gameStatePda,
        player: ctx.sessionSigner.publicKey,
      } as any)
      .instruction();
    await sendErTx("skip-to-eow", [skipToEowIx], ctx.sessionSigner);

    const bossReadyState = await fetchGameState(erConnection, ctx.gameStatePda);
    expect(bossReadyState.phase).to.deep.equal({ night3: {} });
    expect(Number(bossReadyState.movesRemaining)).to.equal(0);
    expect(bossReadyState.bossFightReady).to.equal(true);
    expect(bossReadyState.completed).to.equal(false);

    const setCompletedIx = await (programs.gameplayState.methods as any)
      .testSetCompleted()
      .accounts({
        gameState: ctx.gameStatePda,
        sessionSigner: ctx.sessionSigner.publicKey,
      } as any)
      .instruction();
    await sendErTx("test-set-completed", [setCompletedIx], ctx.sessionSigner);

    const completedState = await fetchGameState(erConnection, ctx.gameStatePda);
    expect(completedState.completed).to.equal(true);
    expect(completedState.bossFightReady).to.equal(false);

    await undelegateAllPerProgram(ctx, { includeSession: true });

    const baseState = await fetchGameState(connection, ctx.gameStatePda);
    const sessionInfo = await connection.getAccountInfo(ctx.sessionPda, "confirmed");
    expect(baseState.completed).to.equal(true);
    expect(baseState.bossFightReady).to.equal(false);
    expect(sessionInfo?.owner.equals(PROGRAM_IDS.sessionManager)).to.equal(true);
  });

  it("can delegate, undelegate, and re-delegate the same campaign runtime accounts", async function () {
    const ctx = await setupCampaignSession("campaign-retry");

    await delegateAllAccounts(ctx, { includeSession: true });
    await undelegateAllPerProgram(ctx, { includeSession: true });

    await delegateAllAccounts(ctx, { includeSession: false });

    const gameStateInfo = await erConnection.getAccountInfo(ctx.gameStatePda, "confirmed");
    expect(gameStateInfo).to.not.be.null;

    await undelegateAllPerProgram(ctx, { includeSession: false });

    const restoredGameStateInfo = await connection.getAccountInfo(ctx.gameStatePda, "confirmed");
    const sessionInfo = await connection.getAccountInfo(ctx.sessionPda, "confirmed");
    expect(restoredGameStateInfo?.owner.equals(PROGRAM_IDS.gameplayState)).to.equal(true);
    expect(sessionInfo?.owner.equals(PROGRAM_IDS.sessionManager)).to.equal(true);
  });
});
