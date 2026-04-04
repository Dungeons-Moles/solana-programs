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
  deriveDelegateAccounts,
} from "../shared/pda-helpers";
import {
  Transaction,
  TransactionInstruction,
  AccountMeta,
} from "@solana/web3.js";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const ER_RPC_URL =
  process.env.EXPO_PUBLIC_EPHEMERAL_PROVIDER_ENDPOINT || "http://127.0.0.1:7799";
const MAGIC_PROGRAM_ID = new PublicKey(
  "Magic11111111111111111111111111111111111111"
);
const MAGIC_CONTEXT_ID = new PublicKey(
  "MagicContext1111111111111111111111111111111"
);
const ER_VALIDATOR = new PublicKey(
  "mAGicPQYBMvcYveUZA5F5UNNwyHvfYh5xkLS2Fr1mev"
);
const COMPANY_TREASURY = new PublicKey(
  "5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8"
);
const STAGED_DEFENDER_POINTS = 15;
const TEST_EPOCH_DURATION_SECONDS = 5;

let connection: Connection;
let erConnection: Connection;
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

type GauntletEntrant = {
  user: Keypair;
  sessionSigner: Keypair;
  epochId: anchor.BN;
  epochIdBigInt: bigint;
  sessionPda: PublicKey;
  epochPoolPda: PublicKey;
  playerScorePda: PublicKey;
  rewardRecordPda: PublicKey;
  gauntletEchoesPda: PublicKey;
};

const sendBaseTx = async (
  label: string,
  ixs: TransactionInstruction[],
  signers: Keypair[]
): Promise<string> => {
  try {
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
  } catch (error) {
    throw new Error(`${label} failed: ${JSON.stringify(error)}`);
  }
};

const sendErTx = async (
  label: string,
  ixs: TransactionInstruction[],
  signer: Keypair
): Promise<string> => {
  try {
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
  } catch (error) {
    throw new Error(`${label} failed: ${JSON.stringify(error)}`);
  }
};

const buildGameplayInstruction = async (
  name: string,
  args: Record<string, unknown>,
  keys: AccountMeta[]
): Promise<TransactionInstruction> => {
  const data = await (programs.gameplayState as any).coder.instruction.encode(name, args);
  return new TransactionInstruction({
    programId: PROGRAM_IDS.gameplayState,
    keys,
    data,
  });
};

const waitForErAccount = async (account: PublicKey, label: string): Promise<void> => {
  for (let i = 0; i < 60; i++) {
    const info = await erConnection.getAccountInfo(account, "confirmed");
    if (info) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`${label} did not become visible on ER`);
};

const waitForCondition = async (
  label: string,
  predicate: () => Promise<boolean>,
  attempts = 80,
  intervalMs = 250
): Promise<void> => {
  for (let i = 0; i < attempts; i++) {
    if (await predicate()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, intervalMs));
  }
  throw new Error(`${label} did not become true in time`);
};

const decodeAccount = async (
  conn: Connection,
  account: PublicKey,
  accountName: string
): Promise<any> => {
  const info = await conn.getAccountInfo(account, "confirmed");
  if (!info) {
    throw new Error(`${accountName} missing on ${conn.rpcEndpoint}`);
  }
  return (programs.gameplayState as any).coder.accounts.decode(accountName, info.data);
};

const gauntletGlobalsExistOnBase = async (): Promise<boolean> => {
  const infos = await Promise.all([
    connection.getAccountInfo(gauntletConfigPda, "confirmed"),
    connection.getAccountInfo(gauntletPoolVaultPda, "confirmed"),
    connection.getAccountInfo(gauntletWeek1Pda, "confirmed"),
    connection.getAccountInfo(gauntletWeek2Pda, "confirmed"),
    connection.getAccountInfo(gauntletWeek3Pda, "confirmed"),
    connection.getAccountInfo(gauntletWeek4Pda, "confirmed"),
    connection.getAccountInfo(gauntletWeek5Pda, "confirmed"),
  ]);

  return infos.every(
    (info) => info?.owner.equals(PROGRAM_IDS.gameplayState) === true
  );
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

  if (await gauntletGlobalsExistOnBase()) {
    return;
  }

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
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
          units: 1_400_000,
        }),
      ])
      .rpc();
  } catch (error: any) {
    if (!String(error).includes("already in use")) throw error;
  }
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
    epochIdBigInt,
    sessionPda,
    epochPoolPda,
    playerScorePda,
    rewardRecordPda,
    gauntletEchoesPda,
  };
};

const delegateGauntletRewardAccountsForEntrant = async (
  entrant: GauntletEntrant
): Promise<void> => {
  const playerScoreDelegate = deriveDelegateAccounts(
    entrant.playerScorePda,
    PROGRAM_IDS.gameplayState
  );
  const rewardRecordDelegate = deriveDelegateAccounts(
    entrant.rewardRecordPda,
    PROGRAM_IDS.gameplayState
  );

  const delegateRewardsIx = await (programs.gameplayState.methods as any)
    .delegateGauntletRewardAccounts(entrant.epochId, ER_VALIDATOR)
    .accountsStrict({
      bufferGauntletPlayerScore: playerScoreDelegate.buffer,
      delegationRecordGauntletPlayerScore: playerScoreDelegate.delegationRecord,
      delegationMetadataGauntletPlayerScore: playerScoreDelegate.delegationMetadata,
      gauntletPlayerScore: entrant.playerScorePda,
      bufferGauntletRewardRecord: rewardRecordDelegate.buffer,
      delegationRecordGauntletRewardRecord: rewardRecordDelegate.delegationRecord,
      delegationMetadataGauntletRewardRecord: rewardRecordDelegate.delegationMetadata,
      gauntletRewardRecord: entrant.rewardRecordPda,
      payer: entrant.sessionSigner.publicKey,
      playerWallet: entrant.user.publicKey,
      ownerProgram: PROGRAM_IDS.gameplayState,
      delegationProgram: PROGRAM_IDS.delegation,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  await sendBaseTx("delegate-gauntlet-rewards", [delegateRewardsIx], [
    entrant.sessionSigner,
  ]);
};

const undelegateGauntletRewardAccountsForEntrant = async (
  entrant: GauntletEntrant
): Promise<void> => {
  const undelegateRewardIx = await (programs.gameplayState.methods as any)
    .undelegateGauntletRewardAccounts(entrant.epochId)
    .accounts({
      gauntletPlayerScore: entrant.playerScorePda,
      gauntletRewardRecord: entrant.rewardRecordPda,
      payer: entrant.sessionSigner.publicKey,
      playerWallet: entrant.user.publicKey,
      magicProgram: MAGIC_PROGRAM_ID,
      magicContext: MAGIC_CONTEXT_ID,
    } as any)
    .instruction();
  await sendErTx("undelegate-gauntlet-reward-accounts", [undelegateRewardIx], entrant.sessionSigner);
};

const payoutRewardForEntrantOnBase = async (entrant: GauntletEntrant): Promise<void> => {
  const payoutIx = await (programs.gameplayState.methods as any)
    .payoutGauntletReward(entrant.epochId)
    .accounts({
      gauntletRewardRecord: entrant.rewardRecordPda,
      gauntletPlayerScore: entrant.playerScorePda,
      gauntletPoolVault: gauntletPoolVaultPda,
      playerWallet: entrant.user.publicKey,
    } as any)
    .instruction();
  await sendBaseTx("payout-gauntlet-reward", [payoutIx], [entrant.sessionSigner]);
};

describe("Gauntlet Cranks E2E", function () {
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
    [gauntletConfigPda] = getGauntletConfigPda();
    [gauntletPoolVaultPda] = getGauntletPoolVaultPda();
    [gauntletWeek1Pda] = getGauntletWeekPoolPda(1);
    [gauntletWeek2Pda] = getGauntletWeekPoolPda(2);
    [gauntletWeek3Pda] = getGauntletWeekPoolPda(3);
    [gauntletWeek4Pda] = getGauntletWeekPoolPda(4);
    [gauntletWeek5Pda] = getGauntletWeekPoolPda(5);

    await ensureGlobalState();
  });

  it("delegates gauntlet accounts, schedules cranks, finalizes the epoch, and pays rewards on ER", async function () {
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

    const profileName = `crank-${user.publicKey.toBase58().slice(0, 6)}`;
    await programs.playerProfile.methods
      .initializeProfile(profileName)
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

    try {
      await (programs.gameplayState.methods as any)
        .stageGauntletDefenderPointsForTesting(epochId, new anchor.BN(STAGED_DEFENDER_POINTS))
        .accounts({
          gauntletEpochPool: epochPoolPda,
          player: user.publicKey,
          payer: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      await (programs.gameplayState.methods as any)
        .setGauntletEpochDurationForTesting(
          new anchor.BN(TEST_EPOCH_DURATION_SECONDS)
        )
        .accounts({
          gauntletConfig: gauntletConfigPda,
          payer: admin.publicKey,
        } as any)
        .rpc();
    } catch (error: any) {
      if (String(error).includes("TestOnlyInstructionDisabled")) {
        this.skip();
      }
      throw error;
    }

    await new Promise((resolve) =>
      setTimeout(resolve, (TEST_EPOCH_DURATION_SECONDS + 1) * 1000)
    );

    const configDelegate = deriveDelegateAccounts(
      gauntletConfigPda,
      PROGRAM_IDS.gameplayState
    );
    const vaultDelegate = deriveDelegateAccounts(
      gauntletPoolVaultPda,
      PROGRAM_IDS.gameplayState
    );
    const epochDelegate = deriveDelegateAccounts(
      epochPoolPda,
      PROGRAM_IDS.gameplayState
    );
    const playerScoreDelegate = deriveDelegateAccounts(
      playerScorePda,
      PROGRAM_IDS.gameplayState
    );
    const rewardRecordDelegate = deriveDelegateAccounts(
      rewardRecordPda,
      PROGRAM_IDS.gameplayState
    );

    const delegateGlobalsIx = await (programs.gameplayState.methods as any)
      .delegateGauntletGlobalAccounts(epochId, ER_VALIDATOR)
      .accountsStrict({
        bufferGauntletConfig: configDelegate.buffer,
        delegationRecordGauntletConfig: configDelegate.delegationRecord,
        delegationMetadataGauntletConfig: configDelegate.delegationMetadata,
        gauntletConfig: gauntletConfigPda,
        bufferGauntletPoolVault: vaultDelegate.buffer,
        delegationRecordGauntletPoolVault: vaultDelegate.delegationRecord,
        delegationMetadataGauntletPoolVault: vaultDelegate.delegationMetadata,
        gauntletPoolVault: gauntletPoolVaultPda,
        bufferGauntletEpochPool: epochDelegate.buffer,
        delegationRecordGauntletEpochPool: epochDelegate.delegationRecord,
        delegationMetadataGauntletEpochPool: epochDelegate.delegationMetadata,
        gauntletEpochPool: epochPoolPda,
        payer: sessionSigner.publicKey,
        ownerProgram: PROGRAM_IDS.gameplayState,
        delegationProgram: PROGRAM_IDS.delegation,
        systemProgram: SystemProgram.programId,
      } as any)
      .instruction();
    await sendBaseTx("delegate-gauntlet-globals", [delegateGlobalsIx], [sessionSigner]);

    const delegateRewardsIx = await (programs.gameplayState.methods as any)
      .delegateGauntletRewardAccounts(epochId, ER_VALIDATOR)
      .accountsStrict({
        bufferGauntletPlayerScore: playerScoreDelegate.buffer,
        delegationRecordGauntletPlayerScore: playerScoreDelegate.delegationRecord,
        delegationMetadataGauntletPlayerScore: playerScoreDelegate.delegationMetadata,
        gauntletPlayerScore: playerScorePda,
        bufferGauntletRewardRecord: rewardRecordDelegate.buffer,
        delegationRecordGauntletRewardRecord: rewardRecordDelegate.delegationRecord,
        delegationMetadataGauntletRewardRecord: rewardRecordDelegate.delegationMetadata,
        gauntletRewardRecord: rewardRecordPda,
        payer: sessionSigner.publicKey,
        playerWallet: user.publicKey,
        ownerProgram: PROGRAM_IDS.gameplayState,
        delegationProgram: PROGRAM_IDS.delegation,
        systemProgram: SystemProgram.programId,
      } as any)
      .instruction();
    await sendBaseTx("delegate-gauntlet-rewards", [delegateRewardsIx], [sessionSigner]);

    await waitForErAccount(gauntletConfigPda, "gauntlet_config");
    await waitForErAccount(gauntletPoolVaultPda, "gauntlet_pool_vault");
    await waitForErAccount(epochPoolPda, "gauntlet_epoch_pool");
    await waitForErAccount(playerScorePda, "gauntlet_player_score");
    await waitForErAccount(rewardRecordPda, "gauntlet_reward_record");

    const globalTaskId = Number(
      user.publicKey.toBuffer().readBigUInt64LE(0) % BigInt(Number.MAX_SAFE_INTEGER)
    );
    const rewardTaskId = globalTaskId + 1;

    const scheduleGlobalIx = await (programs.gameplayState.methods as any)
      .scheduleGauntletEpochCrank(epochId, {
        taskId: new anchor.BN(globalTaskId),
        executionIntervalMillis: new anchor.BN(250),
        iterations: new anchor.BN(8),
      })
      .accounts({
        magicProgram: MAGIC_PROGRAM_ID,
        payer: sessionSigner.publicKey,
        gauntletConfig: gauntletConfigPda,
        gauntletPoolVault: gauntletPoolVaultPda,
        gauntletEpochPool: epochPoolPda,
      } as any)
      .instruction();
    await sendErTx("schedule-gauntlet-epoch-crank", [scheduleGlobalIx], sessionSigner);

    const scheduleRewardIx = await buildGameplayInstruction(
      "scheduleGauntletPlayerRewardCrank",
      {
        epochId,
        args: {
          taskId: new anchor.BN(rewardTaskId),
          executionIntervalMillis: new anchor.BN(250),
          iterations: new anchor.BN(8),
        },
      },
      [
        { pubkey: MAGIC_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: sessionSigner.publicKey, isSigner: true, isWritable: true },
        { pubkey: rewardRecordPda, isSigner: false, isWritable: true },
        { pubkey: epochPoolPda, isSigner: false, isWritable: true },
        { pubkey: playerScorePda, isSigner: false, isWritable: true },
        { pubkey: user.publicKey, isSigner: false, isWritable: false },
      ]
    );
    await sendErTx("schedule-gauntlet-reward-crank", [scheduleRewardIx], sessionSigner);

    const vaultBefore = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const userBalanceBefore = await connection.getBalance(user.publicKey, "confirmed");

    await waitForCondition("scheduled gauntlet epoch finalization", async () => {
      try {
        const config = await decodeAccount(erConnection, gauntletConfigPda, "gauntletConfig");
        const pool = await decodeAccount(erConnection, epochPoolPda, "gauntletEpochPool");
        return (
          pool.finalized === true &&
          Number(config.currentEpochId) === Number(epochId.toString()) + 1
        );
      } catch {
        return false;
      }
    });

    const finalizedConfig = await decodeAccount(
      erConnection,
      gauntletConfigPda,
      "gauntletConfig"
    );
    const finalizedEpochPool = await decodeAccount(
      erConnection,
      epochPoolPda,
      "gauntletEpochPool"
    );
    expect(finalizedEpochPool.finalized).to.equal(true);
    expect(Number(finalizedConfig.currentEpochId)).to.equal(
      Number(epochId.toString()) + 1
    );

    await waitForCondition("scheduled gauntlet reward settlement", async () => {
      try {
        const pool = await decodeAccount(erConnection, epochPoolPda, "gauntletEpochPool");
        const score = await decodeAccount(
          erConnection,
          playerScorePda,
          "gauntletPlayerScore"
        );
        const reward = await decodeAccount(
          erConnection,
          rewardRecordPda,
          "gauntletRewardRecord"
        );
        return (
          pool.pendingDefenderPoints.length === 0 &&
          reward.settled === true &&
          Number(score.points) === STAGED_DEFENDER_POINTS &&
          Number(reward.finalPoints) === STAGED_DEFENDER_POINTS
        );
      } catch {
        return false;
      }
    });

    const settledEpochPool = await decodeAccount(
      erConnection,
      epochPoolPda,
      "gauntletEpochPool"
    );
    const settledPlayerScore = await decodeAccount(
      erConnection,
      playerScorePda,
      "gauntletPlayerScore"
    );
    const settledRewardRecord = await decodeAccount(
      erConnection,
      rewardRecordPda,
      "gauntletRewardRecord"
    );

    const undelegateGlobalsIx = await (programs.gameplayState.methods as any)
      .undelegateGauntletGlobalAccounts(epochId)
      .accounts({
        gauntletConfig: gauntletConfigPda,
        gauntletPoolVault: gauntletPoolVaultPda,
        gauntletEpochPool: epochPoolPda,
        payer: sessionSigner.publicKey,
        magicProgram: MAGIC_PROGRAM_ID,
        magicContext: MAGIC_CONTEXT_ID,
      } as any)
      .instruction();
    await sendErTx("undelegate-gauntlet-global-accounts", [undelegateGlobalsIx], sessionSigner);

    const undelegateRewardIx = await (programs.gameplayState.methods as any)
      .undelegateGauntletRewardAccounts(epochId)
      .accounts({
        gauntletPlayerScore: playerScorePda,
        gauntletRewardRecord: rewardRecordPda,
        payer: sessionSigner.publicKey,
        playerWallet: user.publicKey,
        magicProgram: MAGIC_PROGRAM_ID,
        magicContext: MAGIC_CONTEXT_ID,
      } as any)
      .instruction();
    await sendErTx("undelegate-gauntlet-reward-accounts", [undelegateRewardIx], sessionSigner);

    await waitForCondition("gauntlet reward accounts committed back to base", async () => {
      try {
        const [rewardInfo, scoreInfo, vaultInfo] = await Promise.all([
          connection.getAccountInfo(rewardRecordPda, "confirmed"),
          connection.getAccountInfo(playerScorePda, "confirmed"),
          connection.getAccountInfo(gauntletPoolVaultPda, "confirmed"),
        ]);
        return (
          rewardInfo?.owner.equals(PROGRAM_IDS.gameplayState) === true &&
          scoreInfo?.owner.equals(PROGRAM_IDS.gameplayState) === true &&
          vaultInfo?.owner.equals(PROGRAM_IDS.gameplayState) === true
        );
      } catch {
        return false;
      }
    });

    const payoutIx = await (programs.gameplayState.methods as any)
      .payoutGauntletReward(epochId)
      .accounts({
        gauntletRewardRecord: rewardRecordPda,
        gauntletPlayerScore: playerScorePda,
        gauntletPoolVault: gauntletPoolVaultPda,
        playerWallet: user.publicKey,
      } as any)
      .instruction();
    await sendBaseTx("payout-gauntlet-reward", [payoutIx], [sessionSigner]);

    const paidRewardRecord = await decodeAccount(
      connection,
      rewardRecordPda,
      "gauntletRewardRecord"
    );
    const paidPlayerScore = await decodeAccount(
      connection,
      playerScorePda,
      "gauntletPlayerScore"
    );
    const vaultAfter = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const userBalanceAfter = await connection.getBalance(user.publicKey, "confirmed");

    expect(settledEpochPool.pendingDefenderPoints.length).to.equal(0);
    expect(Number(settledPlayerScore.points)).to.equal(STAGED_DEFENDER_POINTS);
    expect(settledRewardRecord.settled).to.equal(true);
    expect(settledRewardRecord.paid).to.equal(false);
    expect(Number(settledRewardRecord.finalPoints)).to.equal(STAGED_DEFENDER_POINTS);
    expect(Number(settledRewardRecord.payoutLamports)).to.be.greaterThan(0);
    expect(paidRewardRecord.paid).to.equal(true);
    expect(paidPlayerScore.claimed).to.equal(true);
    expect(vaultAfter).to.equal(vaultBefore - Number(paidRewardRecord.payoutLamports));
    expect(userBalanceAfter).to.equal(
      userBalanceBefore + Number(paidRewardRecord.payoutLamports)
    );
  });

  it("settles multiple players on ER and preserves proportional payout math", async function () {
    const entrantA = await createGauntletEntrant("crank-a", 7);
    const entrantB = await createGauntletEntrant("crank-b", 5);

    expect(entrantA.epochId.toString()).to.equal(entrantB.epochId.toString());

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

    const configDelegate = deriveDelegateAccounts(
      gauntletConfigPda,
      PROGRAM_IDS.gameplayState
    );
    const vaultDelegate = deriveDelegateAccounts(
      gauntletPoolVaultPda,
      PROGRAM_IDS.gameplayState
    );
    const epochDelegate = deriveDelegateAccounts(
      entrantA.epochPoolPda,
      PROGRAM_IDS.gameplayState
    );

    const delegateGlobalsIx = await (programs.gameplayState.methods as any)
      .delegateGauntletGlobalAccounts(entrantA.epochId, ER_VALIDATOR)
      .accountsStrict({
        bufferGauntletConfig: configDelegate.buffer,
        delegationRecordGauntletConfig: configDelegate.delegationRecord,
        delegationMetadataGauntletConfig: configDelegate.delegationMetadata,
        gauntletConfig: gauntletConfigPda,
        bufferGauntletPoolVault: vaultDelegate.buffer,
        delegationRecordGauntletPoolVault: vaultDelegate.delegationRecord,
        delegationMetadataGauntletPoolVault: vaultDelegate.delegationMetadata,
        gauntletPoolVault: gauntletPoolVaultPda,
        bufferGauntletEpochPool: epochDelegate.buffer,
        delegationRecordGauntletEpochPool: epochDelegate.delegationRecord,
        delegationMetadataGauntletEpochPool: epochDelegate.delegationMetadata,
        gauntletEpochPool: entrantA.epochPoolPda,
        payer: entrantA.sessionSigner.publicKey,
        ownerProgram: PROGRAM_IDS.gameplayState,
        delegationProgram: PROGRAM_IDS.delegation,
        systemProgram: SystemProgram.programId,
      } as any)
      .instruction();
    await sendBaseTx("delegate-gauntlet-globals", [delegateGlobalsIx], [
      entrantA.sessionSigner,
    ]);

    await delegateGauntletRewardAccountsForEntrant(entrantA);
    await delegateGauntletRewardAccountsForEntrant(entrantB);

    await waitForErAccount(gauntletConfigPda, "gauntlet_config");
    await waitForErAccount(gauntletPoolVaultPda, "gauntlet_pool_vault");
    await waitForErAccount(entrantA.epochPoolPda, "gauntlet_epoch_pool");
    await waitForErAccount(entrantA.playerScorePda, "gauntlet_player_score_a");
    await waitForErAccount(entrantA.rewardRecordPda, "gauntlet_reward_record_a");
    await waitForErAccount(entrantB.playerScorePda, "gauntlet_player_score_b");
    await waitForErAccount(entrantB.rewardRecordPda, "gauntlet_reward_record_b");

    const globalTaskId = Number(
      entrantA.user.publicKey.toBuffer().readBigUInt64LE(0) %
        BigInt(Number.MAX_SAFE_INTEGER)
    );
    const rewardTaskIdA = globalTaskId + 1;
    const rewardTaskIdB = Number(
      entrantB.user.publicKey.toBuffer().readBigUInt64LE(0) %
        BigInt(Number.MAX_SAFE_INTEGER - 10)
    ) + 10;

    const scheduleGlobalIx = await (programs.gameplayState.methods as any)
      .scheduleGauntletEpochCrank(entrantA.epochId, {
        taskId: new anchor.BN(globalTaskId),
        executionIntervalMillis: new anchor.BN(250),
        iterations: new anchor.BN(8),
      })
      .accounts({
        magicProgram: MAGIC_PROGRAM_ID,
        payer: entrantA.sessionSigner.publicKey,
        gauntletConfig: gauntletConfigPda,
        gauntletPoolVault: gauntletPoolVaultPda,
        gauntletEpochPool: entrantA.epochPoolPda,
      } as any)
      .instruction();
    await sendErTx("schedule-gauntlet-epoch-crank", [scheduleGlobalIx], entrantA.sessionSigner);

    const scheduleRewardIxA = await buildGameplayInstruction(
      "scheduleGauntletPlayerRewardCrank",
      {
        epochId: entrantA.epochId,
        args: {
          taskId: new anchor.BN(rewardTaskIdA),
          executionIntervalMillis: new anchor.BN(250),
          iterations: new anchor.BN(8),
        },
      },
      [
        { pubkey: MAGIC_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: entrantA.sessionSigner.publicKey, isSigner: true, isWritable: true },
        { pubkey: entrantA.rewardRecordPda, isSigner: false, isWritable: true },
        { pubkey: entrantA.epochPoolPda, isSigner: false, isWritable: true },
        { pubkey: entrantA.playerScorePda, isSigner: false, isWritable: true },
        { pubkey: entrantA.user.publicKey, isSigner: false, isWritable: false },
      ]
    );
    await sendErTx("schedule-gauntlet-reward-crank-a", [scheduleRewardIxA], entrantA.sessionSigner);

    const scheduleRewardIxB = await buildGameplayInstruction(
      "scheduleGauntletPlayerRewardCrank",
      {
        epochId: entrantB.epochId,
        args: {
          taskId: new anchor.BN(rewardTaskIdB),
          executionIntervalMillis: new anchor.BN(250),
          iterations: new anchor.BN(8),
        },
      },
      [
        { pubkey: MAGIC_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: entrantB.sessionSigner.publicKey, isSigner: true, isWritable: true },
        { pubkey: entrantB.rewardRecordPda, isSigner: false, isWritable: true },
        { pubkey: entrantB.epochPoolPda, isSigner: false, isWritable: true },
        { pubkey: entrantB.playerScorePda, isSigner: false, isWritable: true },
        { pubkey: entrantB.user.publicKey, isSigner: false, isWritable: false },
      ]
    );
    await sendErTx("schedule-gauntlet-reward-crank-b", [scheduleRewardIxB], entrantB.sessionSigner);

    await waitForCondition("scheduled gauntlet epoch finalization", async () => {
      try {
        const config = await decodeAccount(erConnection, gauntletConfigPda, "gauntletConfig");
        const pool = await decodeAccount(
          erConnection,
          entrantA.epochPoolPda,
          "gauntletEpochPool"
        );
        return (
          pool.finalized === true &&
          Number(config.currentEpochId) === Number(entrantA.epochId.toString()) + 1
        );
      } catch {
        return false;
      }
    });

    await waitForCondition("scheduled multi-player reward settlement", async () => {
      try {
        const pool = await decodeAccount(
          erConnection,
          entrantA.epochPoolPda,
          "gauntletEpochPool"
        );
        const scoreA = await decodeAccount(
          erConnection,
          entrantA.playerScorePda,
          "gauntletPlayerScore"
        );
        const scoreB = await decodeAccount(
          erConnection,
          entrantB.playerScorePda,
          "gauntletPlayerScore"
        );
        const rewardA = await decodeAccount(
          erConnection,
          entrantA.rewardRecordPda,
          "gauntletRewardRecord"
        );
        const rewardB = await decodeAccount(
          erConnection,
          entrantB.rewardRecordPda,
          "gauntletRewardRecord"
        );
        return (
          pool.pendingDefenderPoints.length === 0 &&
          rewardA.settled === true &&
          rewardB.settled === true &&
          Number(scoreA.points) === 7 &&
          Number(scoreB.points) === 5 &&
          Number(rewardA.finalPoints) === 7 &&
          Number(rewardB.finalPoints) === 5
        );
      } catch {
        return false;
      }
    });

    const settledEpochPool = await decodeAccount(
      erConnection,
      entrantA.epochPoolPda,
      "gauntletEpochPool"
    );
    const settledRewardA = await decodeAccount(
      erConnection,
      entrantA.rewardRecordPda,
      "gauntletRewardRecord"
    );
    const settledRewardB = await decodeAccount(
      erConnection,
      entrantB.rewardRecordPda,
      "gauntletRewardRecord"
    );

    const totalPoolLamports = Number(settledEpochPool.totalPoolLamports);
    const totalPoints = Number(settledEpochPool.totalPoints);
    const expectedPayoutA = Math.floor((totalPoolLamports * 7) / totalPoints);
    const expectedPayoutB = Math.floor((totalPoolLamports * 5) / totalPoints);
    const expectedDust = totalPoolLamports - expectedPayoutA - expectedPayoutB;

    expect(settledEpochPool.pendingDefenderPoints.length).to.equal(0);
    expect(Number(settledRewardA.finalPoints)).to.equal(7);
    expect(Number(settledRewardB.finalPoints)).to.equal(5);
    expect(Number(settledRewardA.payoutLamports)).to.equal(expectedPayoutA);
    expect(Number(settledRewardB.payoutLamports)).to.equal(expectedPayoutB);

    const undelegateGlobalsIx = await (programs.gameplayState.methods as any)
      .undelegateGauntletGlobalAccounts(entrantA.epochId)
      .accounts({
        gauntletConfig: gauntletConfigPda,
        gauntletPoolVault: gauntletPoolVaultPda,
        gauntletEpochPool: entrantA.epochPoolPda,
        payer: entrantA.sessionSigner.publicKey,
        magicProgram: MAGIC_PROGRAM_ID,
        magicContext: MAGIC_CONTEXT_ID,
      } as any)
      .instruction();
    await sendErTx("undelegate-gauntlet-global-accounts", [undelegateGlobalsIx], entrantA.sessionSigner);

    await undelegateGauntletRewardAccountsForEntrant(entrantA);
    await undelegateGauntletRewardAccountsForEntrant(entrantB);

    await waitForCondition("multi-player gauntlet rewards committed back to base", async () => {
      try {
        const infos = await Promise.all([
          connection.getAccountInfo(entrantA.rewardRecordPda, "confirmed"),
          connection.getAccountInfo(entrantA.playerScorePda, "confirmed"),
          connection.getAccountInfo(entrantB.rewardRecordPda, "confirmed"),
          connection.getAccountInfo(entrantB.playerScorePda, "confirmed"),
          connection.getAccountInfo(gauntletPoolVaultPda, "confirmed"),
        ]);
        return infos.every(
          (info) => info?.owner.equals(PROGRAM_IDS.gameplayState) === true
        );
      } catch {
        return false;
      }
    });

    const vaultBefore = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const userBalanceBeforeA = await connection.getBalance(
      entrantA.user.publicKey,
      "confirmed"
    );
    const userBalanceBeforeB = await connection.getBalance(
      entrantB.user.publicKey,
      "confirmed"
    );

    await payoutRewardForEntrantOnBase(entrantA);
    await payoutRewardForEntrantOnBase(entrantB);

    const paidRewardA = await decodeAccount(
      connection,
      entrantA.rewardRecordPda,
      "gauntletRewardRecord"
    );
    const paidRewardB = await decodeAccount(
      connection,
      entrantB.rewardRecordPda,
      "gauntletRewardRecord"
    );
    const vaultAfter = await connection.getBalance(gauntletPoolVaultPda, "confirmed");
    const userBalanceAfterA = await connection.getBalance(
      entrantA.user.publicKey,
      "confirmed"
    );
    const userBalanceAfterB = await connection.getBalance(
      entrantB.user.publicKey,
      "confirmed"
    );
    const reservedVaultBalance = vaultBefore - totalPoolLamports;

    expect(paidRewardA.paid).to.equal(true);
    expect(paidRewardB.paid).to.equal(true);
    expect(vaultAfter).to.equal(vaultBefore - expectedPayoutA - expectedPayoutB);
    expect(vaultAfter - reservedVaultBalance).to.equal(expectedDust);
    expect(userBalanceAfterA).to.equal(userBalanceBeforeA + expectedPayoutA);
    expect(userBalanceAfterB).to.equal(userBalanceBeforeB + expectedPayoutB);
  });

});
