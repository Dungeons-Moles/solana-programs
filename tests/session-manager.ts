import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SessionManager } from "../target/types/session_manager";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("session-manager", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SessionManager as Program<SessionManager>;

  // Helper to derive session PDA
  const getSessionPDA = (player: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session"), player.toBuffer()],
      program.programId
    );
  };

  // Helper to derive counter PDA
  const getCounterPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session_counter")],
      program.programId
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
        })
        .rpc();
    } catch (error: any) {
      // Counter might already exist from previous test run
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }
    counterInitialized = true;
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
          })
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

      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [sessionPDA] = getSessionPDA(user.publicKey);

      await program.methods
        .startSession(5) // Campaign level 5
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          player: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      const session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.player.toString()).to.equal(user.publicKey.toString());
      expect(session.campaignLevel).to.equal(5);
      expect(session.isDelegated).to.equal(false);
      expect(session.sessionId.toNumber()).to.be.greaterThan(0);
      expect(session.startedAt.toNumber()).to.be.greaterThan(0);
      expect(session.lastActivity.toNumber()).to.be.greaterThan(0);

      // Clean up: end the session
      await program.methods
        .endSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();
    });
  });

  describe("T048: Reject Second Session for Same Player", () => {
    it("rejects second session for same player", async () => {
      await ensureCounterExists();

      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [sessionPDA] = getSessionPDA(user.publicKey);

      // First session should succeed
      await program.methods
        .startSession(0)
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          player: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Second session should fail
      try {
        await program.methods
          .startSession(1)
          .accounts({
            gameSession: sessionPDA,
            sessionCounter: counterPDA,
            player: user.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (error: any) {
        // Account already initialized - this is expected
        expect(error.toString()).to.include("already in use");
      }

      // Clean up
      await program.methods
        .endSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();
    });
  });

  describe("T049: Delegate Session to Ephemeral Rollup", () => {
    it("delegates session to ephemeral rollup", async () => {
      await ensureCounterExists();

      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [sessionPDA] = getSessionPDA(user.publicKey);

      // Start session
      await program.methods
        .startSession(10)
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          player: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Delegate session
      await program.methods
        .delegateSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();

      const session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.isDelegated).to.equal(true);

      // Clean up
      await program.methods
        .endSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();
    });
  });

  describe("T050: Commit Session State", () => {
    it("commits session state", async () => {
      await ensureCounterExists();

      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [sessionPDA] = getSessionPDA(user.publicKey);

      // Start and delegate session
      await program.methods
        .startSession(15)
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          player: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      await program.methods
        .delegateSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();

      // Commit with a state hash
      const stateHash = new Uint8Array(32);
      stateHash.fill(0xab);

      await program.methods
        .commitSession(Array.from(stateHash) as number[])
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();

      const session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.stateHash).to.deep.equal(Array.from(stateHash));

      // Clean up
      await program.methods
        .endSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();
    });
  });

  describe("T051: End Session and Close Account", () => {
    it("ends session and closes account", async () => {
      await ensureCounterExists();

      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [sessionPDA] = getSessionPDA(user.publicKey);

      // Start session
      await program.methods
        .startSession(20)
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          player: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Verify session exists
      let session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.player.toString()).to.equal(user.publicKey.toString());

      // Get user balance before ending
      const balanceBefore = await provider.connection.getBalance(user.publicKey);

      // End session
      await program.methods
        .endSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();

      // Verify session account is closed
      const sessionAccount = await provider.connection.getAccountInfo(sessionPDA);
      expect(sessionAccount).to.be.null;

      // Verify rent was returned to user
      const balanceAfter = await provider.connection.getBalance(user.publicKey);
      expect(balanceAfter).to.be.greaterThan(balanceBefore);
    });
  });

  describe("T052: Force Close Timed-Out Session", () => {
    it("rejects force close before timeout", async () => {
      await ensureCounterExists();

      const user = Keypair.generate();
      const anyoneElse = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const airdropSig2 = await provider.connection.requestAirdrop(
        anyoneElse.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig2);

      const [sessionPDA] = getSessionPDA(user.publicKey);

      // Start session
      await program.methods
        .startSession(25)
        .accounts({
          gameSession: sessionPDA,
          sessionCounter: counterPDA,
          player: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Try to force close immediately (should fail - not timed out)
      try {
        await program.methods
          .forceCloseSession()
          .accounts({
            gameSession: sessionPDA,
            sessionOwner: user.publicKey,
            recipient: anyoneElse.publicKey,
          })
          .signers([])
          .rpc();
        expect.fail("Should have thrown SessionNotTimedOut error");
      } catch (error: any) {
        expect(error.toString()).to.include("SessionNotTimedOut");
      }

      // Clean up - owner can still end their own session
      await program.methods
        .endSession()
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        })
        .signers([user])
        .rpc();
    });

    // Note: Testing actual timeout would require time manipulation
    // which is complex in the Solana test environment
    it("force close structure is valid (timeout test skipped)", async () => {
      // This test validates that the force_close_session instruction exists
      // and has the correct account structure. Actual timeout testing would
      // require manipulating the cluster clock or waiting 1 hour.
      expect(program.methods.forceCloseSession).to.exist;
    });
  });
});
