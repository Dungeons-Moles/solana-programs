import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { CombatSystem } from "../../target/types/combat_system";
import { GameplayState } from "../../target/types/gameplay_state";
import { SessionManager } from "../../target/types/session_manager";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("combat-system status effects", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const combatProgram = anchor.workspace.CombatSystem as Program<CombatSystem>;
  const gameplayProgram = anchor.workspace
    .GameplayState as Program<GameplayState>;
  const sessionProgram = anchor.workspace
    .SessionManager as Program<SessionManager>;

  const getSessionPda = (player: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session"), player.toBuffer()],
      sessionProgram.programId,
    );
  };

  const getGameStatePda = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("game_state"), sessionPda.toBuffer()],
      gameplayProgram.programId,
    );
  };

  const getCombatStatePda = (gameStatePda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("combat_state"), gameStatePda.toBuffer()],
      combatProgram.programId,
    );
  };

  const getCounterPda = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session_counter")],
      sessionProgram.programId,
    );
  };

  let counterInitialized = false;
  const [counterPda] = getCounterPda();

  const ensureCounterExists = async () => {
    if (counterInitialized) return;
    const admin = provider.wallet;
    try {
      await sessionProgram.methods
        .initializeCounter()
        .accounts({
          sessionCounter: counterPda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (error: any) {
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }
    counterInitialized = true;
  };

  const setupCombat = async () => {
    await ensureCounterExists();

    const user = Keypair.generate();
    const airdropSig = await provider.connection.requestAirdrop(
      user.publicKey,
      5 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(airdropSig);

    const [sessionPda] = getSessionPda(user.publicKey);
    const [gameStatePda] = getGameStatePda(sessionPda);
    const [combatStatePda] = getCombatStatePda(gameStatePda);

    await sessionProgram.methods
      .startSession(1)
      .accounts({
        gameSession: sessionPda,
        sessionCounter: counterPda,
        player: user.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user])
      .rpc();

    await gameplayProgram.methods
      .initializeGameState(10, 10, 0, 0)
      .accounts({
        gameState: gameStatePda,
        gameSession: sessionPda,
        player: user.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user])
      .rpc();

    return { user, sessionPda, gameStatePda, combatStatePda };
  };

  const cleanup = async (
    user: Keypair,
    sessionPda: anchor.web3.PublicKey,
    gameStatePda: anchor.web3.PublicKey,
  ) => {
    try {
      await gameplayProgram.methods
        .closeGameState()
        .accounts({
          gameState: gameStatePda,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    } catch (error) {
      // Ignore if already closed
    }

    try {
      await sessionProgram.methods
        .endSession()
        .accounts({
          gameSession: sessionPda,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    } catch (error) {
      // Ignore if already closed
    }
  };

  describe("T034: Status effects in combat", () => {
    it("applies shrapnel retaliation damage", async () => {
      const { user, sessionPda, gameStatePda, combatStatePda } =
        await setupCombat();

      const playerStats = {
        hp: 10,
        maxHp: 10,
        atk: 3,
        arm: 0,
        spd: 2,
        dig: 1,
        strikes: 1,
      };

      const enemyStats = {
        hp: 4,
        maxHp: 4,
        atk: 1,
        arm: 1,
        spd: 1,
        dig: 1,
        strikes: 1,
      };

      await combatProgram.methods
        .initializeCombat(playerStats, enemyStats)
        .accounts({
          combatState: combatStatePda,
          gameState: gameStatePda,
          player: user.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      const playerEffects = [
        {
          trigger: { battleStart: {} },
          oncePerTurn: false,
          effectType: { applyShrapnel: {} },
          value: 2,
        },
      ];

      await combatProgram.methods
        .resolveCombat(playerEffects, [])
        .accounts({
          combatState: combatStatePda,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      const state =
        await combatProgram.account.combatState.fetch(combatStatePda);
      expect(state.playerWon).to.equal(true);
      expect(state.playerHp).to.equal(7);
      expect(state.playerStatus.shrapnel).to.equal(0);

      await cleanup(user, sessionPda, gameStatePda);
    });
  });
});
