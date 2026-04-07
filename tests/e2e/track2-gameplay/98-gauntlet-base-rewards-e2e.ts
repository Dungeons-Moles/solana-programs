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
  getGauntletSessionPda,
  getSessionDiscoveryPda,
  getPlayerProfilePda,
  getMapConfigPda,
  getGeneratedMapPda,
  getGameStatePda,
  getGauntletConfigPda,
  getGauntletPoolVaultPda,
  getGauntletWeekPoolPda,
  getGauntletEpochPoolPda,
  getGauntletPlayerScorePda,
  getGauntletRewardRecordPda,
  getGauntletEchoesPda,
  getInventoryPda,
  getMapPoisPda,
} from "../shared/pda-helpers";
import { Connection as Web3Connection } from "@solana/web3.js";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const COMPANY_TREASURY = new PublicKey(
  "5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8"
);
const TEST_EPOCH_DURATION_SECONDS = 5;

type GauntletEntrant = {
  user: Keypair;
  sessionSigner: Keypair;
  epochId: anchor.BN;
  epochPoolPda: PublicKey;
  playerScorePda: PublicKey;
  rewardRecordPda: PublicKey;
};

let connection: Connection;
let provider: anchor.AnchorProvider;
let programs: AllPrograms;
let admin: Keypair;

let sessionCounterPda: PublicKey;
let mapConfigPda: PublicKey;
let gauntletConfigPda: PublicKey;
let gauntletPoolVaultPda: PublicKey;
let gauntletWeek1Pda: PublicKey;
let gauntletWeek2Pda: PublicKey;
let gauntletWeek3Pda: PublicKey;
let gauntletWeek4Pda: PublicKey;
let gauntletWeek5Pda: PublicKey;

const decodeAccount = async (
  conn: Web3Connection,
  account: PublicKey,
  accountName: string
): Promise<any> => {
  const info = await conn.getAccountInfo(account, "confirmed");
  if (!info) {
    throw new Error(`${accountName} missing on ${conn.rpcEndpoint}`);
  }
  return (programs.gameplayState as any).coder.accounts.decode(accountName, info.data);
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

  const treasuryBalance = await connection.getBalance(COMPANY_TREASURY, "confirmed");
  if (treasuryBalance < 1_000_000) {
    await airdropAndConfirm(connection, COMPANY_TREASURY, LAMPORTS_PER_SOL);
  }

  const gauntletInfo = await connection.getAccountInfo(gauntletConfigPda, "confirmed");
  if (gauntletInfo) {
    return;
  }

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
      anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
        units: 1_400_000,
      }),
    ])
    .rpc();
};

const setGauntletEpochDurationForTesting = async (seconds: number): Promise<void> => {
  await (programs.gameplayState.methods as any)
    .setGauntletEpochDurationForTesting(new anchor.BN(seconds))
    .accounts({
      gauntletConfig: gauntletConfigPda,
      payer: admin.publicKey,
    } as any)
    .rpc();
};

const createGauntletEntrant = async (
  namePrefix: string,
  stagedDefenderPoints: number
): Promise<GauntletEntrant> => {
  const user = Keypair.generate();
  const sessionSigner = Keypair.generate();

  await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
  await airdropAndConfirm(connection, sessionSigner.publicKey, 10 * LAMPORTS_PER_SOL);

  const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
  const [sessionPda] = getGauntletSessionPda(user.publicKey);
  const [sessionNoncesPda] = getSessionNoncesPda(user.publicKey);
  const [sessionDiscoveryPda] = getSessionDiscoveryPda(sessionPda);
  const [gameStatePda] = getGameStatePda(sessionPda);
  const [generatedMapPda] = getGeneratedMapPda(sessionPda);
  const [inventoryPda] = getInventoryPda(sessionPda);
  const [mapPoisPda] = getMapPoisPda(sessionPda);

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
    .startGauntletSession()
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
      anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
        units: 1_400_000,
      }),
      anchor.web3.ComputeBudgetProgram.requestHeapFrame({
        bytes: 256 * 1024,
      }),
    ])
    .signers([user, sessionSigner])
    .rpc();

  const gauntletConfig = await (programs.gameplayState.account as any).gauntletConfig.fetch(
    gauntletConfigPda
  );
  const epochId = new anchor.BN(gauntletConfig.currentEpochId.toString());
  const epochIdBigInt = BigInt(gauntletConfig.currentEpochId.toString());
  const [epochPoolPda] = getGauntletEpochPoolPda(epochIdBigInt);
  const [playerScorePda] = getGauntletPlayerScorePda(epochIdBigInt, user.publicKey);
  const [rewardRecordPda] = getGauntletRewardRecordPda(epochIdBigInt, user.publicKey);
  const [gauntletEchoesPda] = getGauntletEchoesPda(sessionPda);

  await (programs.gameplayState.methods as any)
    .enterGauntlet(epochId)
    .accounts({
      gameState: gameStatePda,
      player: user.publicKey,
      gauntletConfig: gauntletConfigPda,
      gauntletPoolVault: gauntletPoolVaultPda,
      companyTreasury: COMPANY_TREASURY,
      gauntletEpochPool: epochPoolPda,
      gauntletPlayerScore: playerScorePda,
      gauntletRewardRecord: rewardRecordPda,
      gauntletEchoes: gauntletEchoesPda,
      systemProgram: SystemProgram.programId,
    } as any)
    .remainingAccounts([
      { pubkey: gauntletWeek1Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek2Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek3Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek4Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek5Pda, isSigner: false, isWritable: false },
    ])
    .signers([user])
    .rpc();

  await (programs.gameplayState.methods as any)
    .stageGauntletDefenderPointsForTesting(epochId, new anchor.BN(stagedDefenderPoints))
    .accounts({
      gauntletEpochPool: epochPoolPda,
      player: user.publicKey,
      payer: admin.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .rpc();

  return {
    user,
    sessionSigner,
    epochId,
    epochPoolPda,
    playerScorePda,
    rewardRecordPda,
  };
};

const finalizeEpochOnBase = async (epochId: anchor.BN, epochPoolPda: PublicKey): Promise<void> => {
  await (programs.gameplayState.methods as any)
    .finalizeGauntletEpoch(epochId)
    .accounts({
      gauntletConfig: gauntletConfigPda,
      gauntletPoolVault: gauntletPoolVaultPda,
      gauntletEpochPool: epochPoolPda,
      payer: admin.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .rpc();
};

const settleRewardOnBase = async (entrant: GauntletEntrant): Promise<void> => {
  await (programs.gameplayState.methods as any)
    .settleGauntletRewardForPlayer(entrant.epochId)
    .accounts({
      gauntletEpochPool: entrant.epochPoolPda,
      gauntletPlayerScore: entrant.playerScorePda,
      gauntletRewardRecord: entrant.rewardRecordPda,
      player: entrant.user.publicKey,
      payer: entrant.sessionSigner.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([entrant.sessionSigner])
    .rpc();
};

const payoutRewardOnBase = async (entrant: GauntletEntrant): Promise<void> => {
  const ix = await (programs.gameplayState.methods as any)
    .payoutGauntletReward(entrant.epochId)
    .accounts({
      gauntletRewardRecord: entrant.rewardRecordPda,
      gauntletPlayerScore: entrant.playerScorePda,
      gauntletPoolVault: gauntletPoolVaultPda,
      playerWallet: entrant.user.publicKey,
    } as any)
    .instruction();

  const tx = new anchor.web3.Transaction().add(ix);
  tx.feePayer = entrant.sessionSigner.publicKey;
  const bh = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(entrant.sessionSigner);
  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
    maxRetries: 3,
  });
  await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
  const status = await connection.getSignatureStatuses([sig], {
    searchTransactionHistory: true,
  });
  if (status.value[0]?.err) {
    throw new Error(`payout-gauntlet-reward failed: ${JSON.stringify(status.value[0].err)}`);
  }
};

describe("Gauntlet Base Rewards E2E", function () {
  this.timeout(240_000);

  before(async function () {
    admin = loadWalletKeypair();
    const wallet = walletFromKeypair(admin);
    connection = new Connection(RPC_URL, "confirmed");
    provider = createProvider(RPC_URL, wallet);
    anchor.setProvider(provider);
    programs = loadAllPrograms(provider);

    [sessionCounterPda] = getSessionCounterPda();
    [mapConfigPda] = getMapConfigPda();
    [gauntletConfigPda] = getGauntletConfigPda();
    [gauntletPoolVaultPda] = getGauntletPoolVaultPda();
    [gauntletWeek1Pda] = getGauntletWeekPoolPda(1);
    [gauntletWeek2Pda] = getGauntletWeekPoolPda(2);
    [gauntletWeek3Pda] = getGauntletWeekPoolPda(3);
    [gauntletWeek4Pda] = getGauntletWeekPoolPda(4);
    [gauntletWeek5Pda] = getGauntletWeekPoolPda(5);

    await ensureGlobalState();
  });

  it("splits rewards proportionally across players and leaves dust in the vault", async function () {
    const entrantA = await createGauntletEntrant("split-a", 7);
    const entrantB = await createGauntletEntrant("split-b", 5);

    expect(entrantA.epochId.toString()).to.equal(entrantB.epochId.toString());

    await setGauntletEpochDurationForTesting(TEST_EPOCH_DURATION_SECONDS);
    await new Promise((resolve) =>
      setTimeout(resolve, (TEST_EPOCH_DURATION_SECONDS + 1) * 1000)
    );

    await finalizeEpochOnBase(entrantA.epochId, entrantA.epochPoolPda);

    const finalizedEpochPool = await decodeAccount(
      connection,
      entrantA.epochPoolPda,
      "gauntletEpochPool"
    );
    const totalPoolLamports = Number(finalizedEpochPool.totalPoolLamports);
    const totalPoints = Number(finalizedEpochPool.totalPoints);
    expect(totalPoints).to.equal(12);

    await settleRewardOnBase(entrantA);
    await settleRewardOnBase(entrantB);

    const rewardA = await decodeAccount(
      connection,
      entrantA.rewardRecordPda,
      "gauntletRewardRecord"
    );
    const rewardB = await decodeAccount(
      connection,
      entrantB.rewardRecordPda,
      "gauntletRewardRecord"
    );

    const expectedA = Math.floor((totalPoolLamports * 7) / totalPoints);
    const expectedB = Math.floor((totalPoolLamports * 5) / totalPoints);
    const expectedDust = totalPoolLamports - expectedA - expectedB;

    expect(Number(rewardA.finalPoints)).to.equal(7);
    expect(Number(rewardB.finalPoints)).to.equal(5);
    expect(Number(rewardA.payoutLamports)).to.equal(expectedA);
    expect(Number(rewardB.payoutLamports)).to.equal(expectedB);

    const vaultBefore = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    await payoutRewardOnBase(entrantA);
    await payoutRewardOnBase(entrantB);
    const vaultAfter = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const reservedVaultBalance = vaultBefore - totalPoolLamports;

    expect(vaultAfter).to.equal(vaultBefore - expectedA - expectedB);
    expect(vaultAfter - reservedVaultBalance).to.equal(expectedDust);
  });

  it("supports manual settlement fallback and idempotent payout on base", async function () {
    const entrant = await createGauntletEntrant("manual", 11);

    try {
      await setGauntletEpochDurationForTesting(TEST_EPOCH_DURATION_SECONDS);
    } catch (error: any) {
      if (String(error).includes("TestOnlyInstructionDisabled")) {
        this.skip();
      }
      throw error;
    }

    await new Promise((resolve) =>
      setTimeout(resolve, (TEST_EPOCH_DURATION_SECONDS + 1) * 1000)
    );

    await finalizeEpochOnBase(entrant.epochId, entrant.epochPoolPda);
    await settleRewardOnBase(entrant);

    const settledReward = await decodeAccount(
      connection,
      entrant.rewardRecordPda,
      "gauntletRewardRecord"
    );
    const settledScore = await decodeAccount(
      connection,
      entrant.playerScorePda,
      "gauntletPlayerScore"
    );
    expect(settledReward.settled).to.equal(true);
    expect(settledReward.paid).to.equal(false);
    expect(Number(settledScore.points)).to.equal(11);

    await settleRewardOnBase(entrant);
    const settledRewardAgain = await decodeAccount(
      connection,
      entrant.rewardRecordPda,
      "gauntletRewardRecord"
    );
    expect(Number(settledRewardAgain.payoutLamports)).to.equal(
      Number(settledReward.payoutLamports)
    );

    const vaultBefore = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const userBefore = await connection.getBalance(entrant.user.publicKey, "confirmed");

    await payoutRewardOnBase(entrant);

    const paidReward = await decodeAccount(
      connection,
      entrant.rewardRecordPda,
      "gauntletRewardRecord"
    );
    const paidScore = await decodeAccount(
      connection,
      entrant.playerScorePda,
      "gauntletPlayerScore"
    );
    const vaultAfterFirst = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const userAfterFirst = await connection.getBalance(entrant.user.publicKey, "confirmed");

    expect(paidReward.paid).to.equal(true);
    expect(paidScore.claimed).to.equal(true);
    expect(vaultAfterFirst).to.equal(vaultBefore - Number(paidReward.payoutLamports));
    expect(userAfterFirst).to.equal(userBefore + Number(paidReward.payoutLamports));

    await payoutRewardOnBase(entrant);

    const paidRewardAgain = await decodeAccount(
      connection,
      entrant.rewardRecordPda,
      "gauntletRewardRecord"
    );
    const vaultAfterSecond = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const userAfterSecond = await connection.getBalance(entrant.user.publicKey, "confirmed");

    expect(paidRewardAgain.paid).to.equal(true);
    expect(vaultAfterSecond).to.equal(vaultAfterFirst);
    expect(userAfterSecond).to.equal(userAfterFirst);
  });
});
