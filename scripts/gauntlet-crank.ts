import * as anchor from "@anchor-lang/core";
import { Program } from "@anchor-lang/core";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import gameplayIdl from "../target/idl/gameplay_state.json";

type EpochPoolRow = {
  publicKey: PublicKey;
  account: {
    epochId: bigint | number;
    finalized: boolean;
    pendingDefenderPoints: Array<{
      player: PublicKey;
      points: bigint | number;
    }>;
  };
};

type ScoreRow = {
  publicKey: PublicKey;
  account: {
    epochId: bigint | number;
    player: PublicKey;
    points: bigint | number;
    claimed: boolean;
  };
};

type RewardRow = {
  publicKey: PublicKey;
  account: {
    epochId: bigint | number;
    player: PublicKey;
    finalPoints: bigint | number;
    payoutLamports: bigint | number;
    settled: boolean;
    paid: boolean;
  };
};

const INTERVAL_MS = Number(process.env.GAUNTLET_CRANK_INTERVAL_MS ?? "5000");
const RUN_ONCE = process.env.GAUNTLET_CRANK_ONCE === "1";

function deriveEpochPoolPda(programId: PublicKey, epochId: bigint): PublicKey {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(epochId);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_epoch_pool"), buf],
    programId,
  )[0];
}

function derivePlayerScorePda(
  programId: PublicKey,
  epochId: bigint,
  player: PublicKey,
): PublicKey {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(epochId);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_player_score"), buf, player.toBuffer()],
    programId,
  )[0];
}

function deriveRewardRecordPda(
  programId: PublicKey,
  epochId: bigint,
  player: PublicKey,
): PublicKey {
  const buf = Buffer.alloc(8);
  buf.writeBigUInt64LE(epochId);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_reward_record"), buf, player.toBuffer()],
    programId,
  )[0];
}

async function maybeFinalizeCurrentEpoch(
  program: Program,
  payer: PublicKey,
  currentEpochId: bigint,
): Promise<void> {
  const [gauntletConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_config")],
    program.programId,
  );
  const [gauntletPoolVault] = PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_pool_vault")],
    program.programId,
  );
  const gauntletEpochPool = deriveEpochPoolPda(
    program.programId,
    currentEpochId,
  );

  try {
    await (program.methods as any)
      .finalizeGauntletEpoch(new anchor.BN(currentEpochId.toString()))
      .accounts({
        gauntletConfig,
        gauntletPoolVault,
        gauntletEpochPool,
        payer,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();
    console.log(
      `[gauntlet-crank] finalize attempted for epoch ${currentEpochId.toString()}`,
    );
  } catch (error) {
    console.error(
      `[gauntlet-crank] finalize failed for epoch ${currentEpochId.toString()}:`,
      error,
    );
  }
}

async function settlePendingDefenderPoints(
  program: Program,
  payer: PublicKey,
  epochPool: EpochPoolRow,
): Promise<void> {
  const epochId = BigInt(epochPool.account.epochId.toString());
  for (const pending of epochPool.account.pendingDefenderPoints) {
    const player = new PublicKey(pending.player);
    const gauntletPlayerScore = derivePlayerScorePda(
      program.programId,
      epochId,
      player,
    );
    try {
      await (program.methods as any)
        .settleGauntletDefenderPoints(new anchor.BN(epochId.toString()))
        .accounts({
          gauntletEpochPool: epochPool.publicKey,
          gauntletPlayerScore,
          player,
          payer,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      console.log(
        `[gauntlet-crank] settled defender points for ${player.toBase58()} on epoch ${epochId.toString()}`,
      );
    } catch (error) {
      console.error(
        `[gauntlet-crank] defender settle failed for ${player.toBase58()} on epoch ${epochId.toString()}:`,
        error,
      );
    }
  }
}

async function settleFinalizedRewards(
  program: Program,
  payer: PublicKey,
  epochPool: EpochPoolRow,
  scores: ScoreRow[],
  rewards: RewardRow[],
): Promise<void> {
  const epochId = BigInt(epochPool.account.epochId.toString());
  const players = new Map<string, PublicKey>();

  for (const score of scores) {
    if (BigInt(score.account.epochId.toString()) === epochId) {
      players.set(score.account.player.toBase58(), score.account.player);
    }
  }
  for (const pending of epochPool.account.pendingDefenderPoints) {
    players.set(pending.player.toBase58(), pending.player);
  }

  for (const player of players.values()) {
    const existingReward = rewards.find(
      (row) =>
        BigInt(row.account.epochId.toString()) === epochId &&
        row.account.player.toBase58() === player.toBase58(),
    );
    if (existingReward?.account.settled) continue;

    const gauntletPlayerScore = derivePlayerScorePda(
      program.programId,
      epochId,
      player,
    );
    const gauntletRewardRecord = deriveRewardRecordPda(
      program.programId,
      epochId,
      player,
    );

    try {
      await (program.methods as any)
        .settleGauntletRewardForPlayer(new anchor.BN(epochId.toString()))
        .accounts({
          gauntletEpochPool: epochPool.publicKey,
          gauntletPlayerScore,
          gauntletRewardRecord,
          player,
          payer,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      console.log(
        `[gauntlet-crank] settled payout for ${player.toBase58()} on epoch ${epochId.toString()}`,
      );
    } catch (error) {
      console.error(
        `[gauntlet-crank] reward settle failed for ${player.toBase58()} on epoch ${epochId.toString()}:`,
        error,
      );
    }
  }
}

async function payoutSettledRewards(
  program: Program,
  rewards: RewardRow[],
): Promise<void> {
  const [gauntletPoolVault] = PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_pool_vault")],
    program.programId,
  );

  for (const reward of rewards) {
    if (!reward.account.settled || reward.account.paid) continue;

    const epochId = BigInt(reward.account.epochId.toString());
    const player = reward.account.player;
    const gauntletPlayerScore = derivePlayerScorePda(
      program.programId,
      epochId,
      player,
    );
    try {
      await (program.methods as any)
        .payoutGauntletReward(new anchor.BN(epochId.toString()))
        .accounts({
          gauntletRewardRecord: reward.publicKey,
          gauntletPlayerScore,
          gauntletPoolVault,
          playerWallet: player,
        } as any)
        .rpc();
      console.log(
        `[gauntlet-crank] paid ${player.toBase58()} for epoch ${epochId.toString()}`,
      );
    } catch (error) {
      console.error(
        `[gauntlet-crank] payout failed for ${player.toBase58()}:`,
        error,
      );
    }
  }
}

async function runIteration(program: Program, payer: PublicKey): Promise<void> {
  const [gauntletConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_config")],
    program.programId,
  );
  const gauntletConfig = await (
    program.account as {
      gauntletConfig: {
        fetch: (
          address: PublicKey,
        ) => Promise<{ currentEpochId: bigint | number }>;
      };
      gauntletEpochPool: { all: () => Promise<EpochPoolRow[]> };
      gauntletPlayerScore: { all: () => Promise<ScoreRow[]> };
      gauntletRewardRecord: { all: () => Promise<RewardRow[]> };
    }
  ).gauntletConfig.fetch(gauntletConfigPda);

  await maybeFinalizeCurrentEpoch(
    program,
    payer,
    BigInt(gauntletConfig.currentEpochId.toString()),
  );

  const epochPools = await (
    program.account as {
      gauntletEpochPool: { all: () => Promise<EpochPoolRow[]> };
    }
  ).gauntletEpochPool.all();

  for (const epochPool of epochPools) {
    await settlePendingDefenderPoints(program, payer, epochPool);
  }

  const scores = await (
    program.account as {
      gauntletPlayerScore: { all: () => Promise<ScoreRow[]> };
    }
  ).gauntletPlayerScore.all();
  const rewardsBeforeSettle = await (
    program.account as {
      gauntletRewardRecord: { all: () => Promise<RewardRow[]> };
    }
  ).gauntletRewardRecord.all();

  for (const epochPool of epochPools) {
    if (!epochPool.account.finalized) continue;
    await settleFinalizedRewards(
      program,
      payer,
      epochPool,
      scores,
      rewardsBeforeSettle,
    );
  }

  const rewardsAfterSettle = await (
    program.account as {
      gauntletRewardRecord: { all: () => Promise<RewardRow[]> };
    }
  ).gauntletRewardRecord.all();
  await payoutSettledRewards(program, rewardsAfterSettle);
}

async function main(): Promise<void> {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = new Program(gameplayIdl as any, provider);

  console.log("[gauntlet-crank] rpc:", provider.connection.rpcEndpoint);
  console.log("[gauntlet-crank] payer:", provider.wallet.publicKey.toBase58());

  do {
    try {
      await runIteration(program, provider.wallet.publicKey);
    } catch (error) {
      console.error("[gauntlet-crank] iteration failed:", error);
    }

    if (!RUN_ONCE) {
      await new Promise((resolve) => setTimeout(resolve, INTERVAL_MS));
    }
  } while (!RUN_ONCE);
}

main().catch((error) => {
  console.error("[gauntlet-crank] fatal:", error);
  process.exit(1);
});
