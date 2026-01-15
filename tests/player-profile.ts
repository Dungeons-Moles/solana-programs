import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PlayerProfile } from "../target/types/player_profile";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("player-profile", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PlayerProfile as Program<PlayerProfile>;

  // Helper to derive profile PDA
  const getProfilePDA = (owner: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("player"), owner.toBuffer()],
      program.programId
    );
  };

  // Helper to derive treasury PDA
  const getTreasuryPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("treasury")],
      program.programId
    );
  };

  describe("T020: Initialize Profile", () => {
    it("initializes player profile with name", async () => {
      const user = Keypair.generate();

      // Airdrop SOL to user
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);
      const testName = "TestPlayer";

      await program.methods
        .initializeProfile(testName)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Fetch the created profile
      const profile = await program.account.playerProfile.fetch(profilePDA);

      expect(profile.owner.toString()).to.equal(user.publicKey.toString());
      expect(profile.name).to.equal(testName);
      expect(profile.totalRuns).to.equal(0);
      expect(profile.currentLevel).to.equal(0);
      expect(profile.unlockedTier).to.equal(0);
      expect(profile.createdAt.toNumber()).to.be.greaterThan(0);
    });
  });

  describe("T021: Reject Duplicate Profile", () => {
    it("rejects duplicate profile creation", async () => {
      const user = Keypair.generate();

      // Airdrop SOL to user
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // First creation should succeed
      await program.methods
        .initializeProfile("FirstProfile")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Second creation should fail
      try {
        await program.methods
          .initializeProfile("SecondProfile")
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (error: any) {
        // Account already initialized - this is expected
        expect(error.toString()).to.include("already in use");
      }
    });
  });

  describe("T022: Update Profile Name", () => {
    it("updates profile name", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile
      await program.methods
        .initializeProfile("OriginalName")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Update name
      const newName = "UpdatedName";
      await program.methods
        .updateProfileName(newName)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        })
        .signers([user])
        .rpc();

      // Verify update
      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.name).to.equal(newName);
    });
  });

  describe("T023: Reject Name Too Long", () => {
    it("rejects name longer than 32 characters", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);
      const longName = "A".repeat(33); // 33 chars, exceeds 32 limit

      try {
        await program.methods
          .initializeProfile(longName)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (error: any) {
        expect(error.toString()).to.include("NameTooLong");
      }
    });
  });

  describe("Treasury Operations", () => {
    // Shared treasury initialization for this test suite
    let treasuryInitialized = false;
    const [treasuryPDA] = getTreasuryPDA();

    const ensureTreasuryExists = async () => {
      if (treasuryInitialized) return;
      const admin = provider.wallet;
      try {
        await program.methods
          .initializeTreasury()
          .accounts({
            treasury: treasuryPDA,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
      } catch (error: any) {
        // Treasury might already exist from previous test run
        if (!error.toString().includes("already in use")) {
          throw error;
        }
      }
      treasuryInitialized = true;
    };

    it("T031: initializes treasury account", async () => {
      const admin = provider.wallet;

      try {
        await program.methods
          .initializeTreasury()
          .accounts({
            treasury: treasuryPDA,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        const treasury = await program.account.treasury.fetch(treasuryPDA);
        expect(treasury.admin.toString()).to.equal(admin.publicKey.toString());
        expect(treasury.totalCollected.toNumber()).to.equal(0);
        treasuryInitialized = true;
      } catch (error: any) {
        // Treasury might already exist from previous test run
        if (!error.toString().includes("already in use")) {
          throw error;
        }
        treasuryInitialized = true;
      }
    });

    it("T032: unlocks tier with 0.05 SOL payment", async () => {
      await ensureTreasuryExists();

      const user = Keypair.generate();
      const TIER_UNLOCK_COST = 50_000_000; // 0.05 SOL

      // Airdrop enough SOL for tier unlock + rent
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        3 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile first
      await program.methods
        .initializeProfile("TierTestPlayer")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Advance player to level 39 (tier boundary) via record_run_result
      // Each victory advances level by 1, so we need 39 victories
      for (let i = 0; i < 39; i++) {
        await program.methods
          .recordRunResult(i, true)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          })
          .signers([user])
          .rpc();
      }

      // Verify we're at level 39
      let profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(39);
      expect(profile.unlockedTier).to.equal(0);

      // Now unlock tier 1
      await program.methods
        .unlockCampaignTier()
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          treasury: treasuryPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Verify tier is now 1
      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.unlockedTier).to.equal(1);
    });

    it("T033: rejects unlock before tier boundary", async () => {
      await ensureTreasuryExists();

      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        3 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile (starts at level 0, tier 0)
      await program.methods
        .initializeProfile("EarlyUnlockTest")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Advance to level 20 (not at tier boundary 39)
      for (let i = 0; i < 20; i++) {
        await program.methods
          .recordRunResult(i, true)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          })
          .signers([user])
          .rpc();
      }

      // Verify we're at level 20
      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(20);

      // Try to unlock tier - should fail
      try {
        await program.methods
          .unlockCampaignTier()
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
            treasury: treasuryPDA,
            systemProgram: SystemProgram.programId,
          })
          .signers([user])
          .rpc();
        expect.fail("Should have thrown TierNotReached error");
      } catch (error: any) {
        expect(error.toString()).to.include("TierNotReached");
      }
    });

    it("T034: transfers SOL to treasury", async () => {
      await ensureTreasuryExists();

      const user = Keypair.generate();
      const TIER_UNLOCK_COST = 50_000_000; // 0.05 SOL

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        3 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile
      await program.methods
        .initializeProfile("TreasuryTransferTest")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Advance to level 39
      for (let i = 0; i < 39; i++) {
        await program.methods
          .recordRunResult(i, true)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          })
          .signers([user])
          .rpc();
      }

      // Record treasury balance before unlock
      const treasuryBefore = await provider.connection.getBalance(treasuryPDA);
      const treasuryAccountBefore = await program.account.treasury.fetch(treasuryPDA);
      const totalCollectedBefore = treasuryAccountBefore.totalCollected.toNumber();

      // Unlock tier
      await program.methods
        .unlockCampaignTier()
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          treasury: treasuryPDA,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Verify treasury received the SOL
      const treasuryAfter = await provider.connection.getBalance(treasuryPDA);
      expect(treasuryAfter - treasuryBefore).to.equal(TIER_UNLOCK_COST);

      // Verify total_collected increased
      const treasuryAccountAfter = await program.account.treasury.fetch(treasuryPDA);
      expect(treasuryAccountAfter.totalCollected.toNumber() - totalCollectedBefore).to.equal(TIER_UNLOCK_COST);
    });

    it("T035: admin can withdraw from treasury", async () => {
      await ensureTreasuryExists();

      const admin = provider.wallet;
      const recipient = Keypair.generate();
      const withdrawAmount = 10_000_000; // 0.01 SOL

      // Get treasury balance before
      const treasuryBalanceBefore = await provider.connection.getBalance(treasuryPDA);

      // Ensure treasury has enough balance (it should from previous tests)
      if (treasuryBalanceBefore < withdrawAmount) {
        // Skip if not enough balance
        console.log("Skipping withdrawal test - insufficient treasury balance");
        return;
      }

      // Get recipient balance before
      const recipientBalanceBefore = await provider.connection.getBalance(recipient.publicKey);

      // Withdraw
      await program.methods
        .withdrawTreasury(new anchor.BN(withdrawAmount))
        .accounts({
          treasury: treasuryPDA,
          admin: admin.publicKey,
          recipient: recipient.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Verify treasury balance decreased
      const treasuryBalanceAfter = await provider.connection.getBalance(treasuryPDA);
      expect(treasuryBalanceBefore - treasuryBalanceAfter).to.equal(withdrawAmount);

      // Verify recipient balance increased
      const recipientBalanceAfter = await provider.connection.getBalance(recipient.publicKey);
      expect(recipientBalanceAfter - recipientBalanceBefore).to.equal(withdrawAmount);
    });
  });

  // ==========================================================================
  // Phase 7: Run Completion Tests (US5)
  // ==========================================================================

  describe("Run Completion (US5)", () => {
    it("T094: records run completion", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile
      await program.methods
        .initializeProfile("RunCompletionTest")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Record a run (level 0, defeat)
      await program.methods
        .recordRunResult(0, false)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        })
        .signers([user])
        .rpc();

      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.totalRuns).to.equal(1);
    });

    it("T095: increments total_runs on completion", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile
      await program.methods
        .initializeProfile("RunCountTest")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Record multiple runs
      for (let i = 0; i < 5; i++) {
        await program.methods
          .recordRunResult(i, i % 2 === 0) // alternating wins/losses
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          })
          .signers([user])
          .rpc();
      }

      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.totalRuns).to.equal(5);
    });

    it("T096: advances level on victory", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile
      await program.methods
        .initializeProfile("LevelAdvanceTest")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Verify starting level
      let profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(0);

      // Record a victory
      await program.methods
        .recordRunResult(0, true)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        })
        .signers([user])
        .rpc();

      // Verify level advanced
      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(1);

      // Record another victory
      await program.methods
        .recordRunResult(1, true)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        })
        .signers([user])
        .rpc();

      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(2);
    });

    it("T097: does not advance level on defeat", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile
      await program.methods
        .initializeProfile("NoAdvanceOnDefeat")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Advance to level 5
      for (let i = 0; i < 5; i++) {
        await program.methods
          .recordRunResult(i, true)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          })
          .signers([user])
          .rpc();
      }

      let profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(5);

      // Record a defeat
      await program.methods
        .recordRunResult(5, false)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        })
        .signers([user])
        .rpc();

      // Verify level did NOT advance
      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(5);
      expect(profile.totalRuns).to.equal(6);
    });

    it("T098: respects tier boundary on level advance", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);

      // Create profile
      await program.methods
        .initializeProfile("TierBoundaryTest")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([user])
        .rpc();

      // Advance to level 38 (one before tier 0 boundary at 39)
      for (let i = 0; i < 38; i++) {
        await program.methods
          .recordRunResult(i, true)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          })
          .signers([user])
          .rpc();
      }

      let profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(38);
      expect(profile.unlockedTier).to.equal(0);

      // Win to reach level 39 (tier boundary)
      await program.methods
        .recordRunResult(38, true)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        })
        .signers([user])
        .rpc();

      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(39);

      // Try to win again - should NOT advance past level 39 without unlocking tier 1
      // (current max = (0+1)*40 - 1 = 39)
      await program.methods
        .recordRunResult(39, true)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        })
        .signers([user])
        .rpc();

      profile = await program.account.playerProfile.fetch(profilePDA);
      // Should still be at 39 (capped by tier)
      expect(profile.currentLevel).to.equal(39);
      // But run count should have increased
      expect(profile.totalRuns).to.equal(40);
    });
  });
});
