import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import { SessionManager } from "../../target/types/session_manager";
import { PlayerProfile } from "../../target/types/player_profile";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("session-manager", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SessionManager as Program<SessionManager>;
  const playerProfileProgram = anchor.workspace
    .PlayerProfile as Program<PlayerProfile>;

  // Helper to derive session PDA (now includes campaignLevel)
  const getSessionPDA = (
    player: anchor.web3.PublicKey,
    campaignLevel: number,
  ) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session"), player.toBuffer(), Buffer.from([campaignLevel])],
      program.programId,
    );
  };

  // Helper to derive counter PDA
  const getCounterPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session_counter")],
      program.programId,
    );
  };

  // Helper to derive PlayerProfile PDA
  const getPlayerProfilePDA = (player: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("player"), player.toBuffer()],
      playerProfileProgram.programId,
    );
  };

  let counterInitialized = false;
  const [counterPDA] = getCounterPDA();

  const ensureCounterExists = async () => {
    if (counterInitialized) return;
    const admin = provider.wallet;
    try {
      await program.methods
        .initializeCounter()
        .accounts({
          sessionCounter: counterPDA,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (error: any) {
      // Counter might already exist from previous test run
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }
    counterInitialized = true;
  };

  // Helper to create user with profile
  const createUserWithProfile = async (name: string = "TestPlayer") => {
    const user = Keypair.generate();
    const burnerWallet = Keypair.generate();

    // Airdrop SOL
    const airdropSig = await provider.connection.requestAirdrop(
      user.publicKey,
      2 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(airdropSig);

    const [playerProfilePDA] = getPlayerProfilePDA(user.publicKey);

    // Create player profile
    await playerProfileProgram.methods
      .initializeProfile(name)
      .accounts({
        playerProfile: playerProfilePDA,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([user])
      .rpc();

    return { user, burnerWallet, playerProfilePDA };
  };

  describe("T046: Initialize Session Counter", () => {
    it("initializes session counter", async () => {
      const admin = provider.wallet;

      try {
        await program.methods
          .initializeCounter()
          .accounts({
            sessionCounter: counterPDA,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();

        const counter = await program.account.sessionCounter.fetch(counterPDA);
        expect(counter.count.toNumber()).to.equal(0);
        counterInitialized = true;
      } catch (error: any) {
        // Counter might already exist from previous test run
        if (!error.toString().includes("already in use")) {
          throw error;
        }
        counterInitialized = true;
      }
    });
  });

  describe("T047: Start New Game Session", () => {
    it("starts new game session", async () => {
      await ensureCounterExists();

      const { user, burnerWallet, playerProfilePDA } =
        await createUserWithProfile("SessionTest1");
      const campaignLevel = 1; // Player starts with level 1 unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      await program.methods
        .startSession(campaignLevel, new BN(0))
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          playerProfile: playerProfilePDA,
          player: user.publicKey,
          burnerWallet: burnerWallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      const session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.player.toString()).to.equal(user.publicKey.toString());
      expect(session.campaignLevel).to.equal(campaignLevel);
      expect(session.isDelegated).to.equal(false);
      expect(session.sessionId.toNumber()).to.be.greaterThan(0);
      expect(session.startedAt.toNumber()).to.be.greaterThan(0);
      expect(session.lastActivity.toNumber()).to.be.greaterThan(0);

      // Clean up: end the session
      await program.methods
        .endSession(campaignLevel, true)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    });
  });

  describe("T048: Reject Second Session for Same Player at Same Level", () => {
    it("rejects second session for same player at same level", async () => {
      await ensureCounterExists();

      const { user, burnerWallet, playerProfilePDA } =
        await createUserWithProfile("SessionTest2");
      const campaignLevel = 1; // Must be >= 1 and <= highest_level_unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // First session should succeed
      await program.methods
        .startSession(campaignLevel, new BN(0))
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          playerProfile: playerProfilePDA,
          player: user.publicKey,
          burnerWallet: burnerWallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      // Second session at same level should fail
      try {
        await program.methods
          .startSession(campaignLevel, new BN(0))
          .accounts({
            gameSession: sessionPDA,
            sessionCounter: counterPDA,
            playerProfile: playerProfilePDA,
            player: user.publicKey,
            burnerWallet: burnerWallet.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (error: any) {
        // Account already initialized - this is expected
        expect(error.toString()).to.include("already in use");
      }

      // Clean up
      await program.methods
        .endSession(campaignLevel, true)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    });
  });

  describe("T049: Delegate Session to Ephemeral Rollup", () => {
    it("delegates session to ephemeral rollup", async () => {
      await ensureCounterExists();

      const { user, burnerWallet, playerProfilePDA } =
        await createUserWithProfile("SessionTest3");
      const campaignLevel = 1; // Player starts with level 1 unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // Start session
      await program.methods
        .startSession(campaignLevel, new BN(0))
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          playerProfile: playerProfilePDA,
          player: user.publicKey,
          burnerWallet: burnerWallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      // Delegate session (campaignLevel is now first param)
      await program.methods
        .delegateSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      const session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.isDelegated).to.equal(true);

      // Clean up
      await program.methods
        .endSession(campaignLevel, true)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    });
  });

  describe("T050: Commit Session State", () => {
    it("commits session state", async () => {
      await ensureCounterExists();

      const { user, burnerWallet, playerProfilePDA } =
        await createUserWithProfile("SessionTest4");
      const campaignLevel = 1; // Player starts with level 1 unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // Start and delegate session
      await program.methods
        .startSession(campaignLevel, new BN(0))
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          playerProfile: playerProfilePDA,
          player: user.publicKey,
          burnerWallet: burnerWallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      await program.methods
        .delegateSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Commit with a state hash (campaignLevel is now first param)
      const stateHash = new Uint8Array(32);
      stateHash.fill(0xab);

      await program.methods
        .commitSession(campaignLevel, Array.from(stateHash) as number[])
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      const session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.stateHash).to.deep.equal(Array.from(stateHash));

      // Clean up
      await program.methods
        .endSession(campaignLevel, true)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    });
  });

  describe("T051: End Session and Close Account", () => {
    it("ends session and closes account", async () => {
      await ensureCounterExists();

      const { user, burnerWallet, playerProfilePDA } =
        await createUserWithProfile("SessionTest5");
      const campaignLevel = 1; // Player starts with level 1 unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // Start session
      await program.methods
        .startSession(campaignLevel, new BN(0))
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          playerProfile: playerProfilePDA,
          player: user.publicKey,
          burnerWallet: burnerWallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      // Verify session exists
      let session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.player.toString()).to.equal(user.publicKey.toString());

      // Get user balance before ending
      const balanceBefore = await provider.connection.getBalance(
        user.publicKey,
      );

      // End session (campaignLevel is now first param, victory bool added)
      await program.methods
        .endSession(campaignLevel, true)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify session account is closed
      const sessionAccount =
        await provider.connection.getAccountInfo(sessionPDA);
      expect(sessionAccount).to.be.null;

      // Verify rent was returned to user
      const balanceAfter = await provider.connection.getBalance(user.publicKey);
      expect(balanceAfter).to.be.greaterThan(balanceBefore);
    });
  });

  describe("T052: Force Close Session", () => {
    it("allows force close without timeout", async () => {
      await ensureCounterExists();

      const { user, burnerWallet, playerProfilePDA } =
        await createUserWithProfile("SessionTest6");
      const anyoneElse = Keypair.generate();
      const campaignLevel = 1; // Player starts with level 1 unlocked

      const airdropSig2 = await provider.connection.requestAirdrop(
        anyoneElse.publicKey,
        2 * LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig2);

      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // Start session
      await program.methods
        .startSession(campaignLevel, new BN(0))
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          playerProfile: playerProfilePDA,
          player: user.publicKey,
          burnerWallet: burnerWallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      // Force close immediately (campaignLevel is now first param)
      await program.methods
        .forceCloseSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          sessionOwner: user.publicKey,
          recipient: anyoneElse.publicKey,
        } as any)
        .signers([])
        .rpc();

      const sessionAccount =
        await provider.connection.getAccountInfo(sessionPDA);
      expect(sessionAccount).to.be.null;
    });

    it("force close structure is valid", async () => {
      expect(program.methods.forceCloseSession).to.exist;
    });
  });
});
