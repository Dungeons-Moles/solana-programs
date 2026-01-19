import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PlayerProfile } from "../../target/types/player_profile";
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
      program.programId,
    );
  };

  describe("T020: Initialize Profile", () => {
    it("initializes player profile with name", async () => {
      const user = Keypair.generate();

      // Airdrop SOL to user
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL,
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
        } as any)
        .signers([user])
        .rpc();

      // Fetch the created profile
      const profile = await program.account.playerProfile.fetch(profilePDA);

      expect(profile.owner.toString()).to.equal(user.publicKey.toString());
      expect(profile.name).to.equal(testName);
      expect(profile.totalRuns).to.equal(0);
      expect(profile.currentLevel).to.equal(0);
      expect((profile as any).availableRuns).to.equal(40);
      expect(profile.createdAt.toNumber()).to.be.greaterThan(0);
    });
  });

  describe("T021: Reject Duplicate Profile", () => {
    it("rejects duplicate profile creation", async () => {
      const user = Keypair.generate();

      // Airdrop SOL to user
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL,
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
        } as any)
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
          } as any)
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
        2 * LAMPORTS_PER_SOL,
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
        } as any)
        .signers([user])
        .rpc();

      // Update name
      const newName = "UpdatedName";
      await program.methods
        .updateProfileName(newName)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        } as any)
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
        2 * LAMPORTS_PER_SOL,
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
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown an error");
      } catch (error: any) {
        expect(error.toString()).to.include("NameTooLong");
      }
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
        2 * LAMPORTS_PER_SOL,
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
        } as any)
        .signers([user])
        .rpc();

      // Record a run (level 0, defeat)
      await program.methods
        .recordRunResult(0, false)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.totalRuns).to.equal(1);
      expect((profile as any).availableRuns).to.equal(39);
    });

    it("T095: increments total_runs on completion", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL,
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
        } as any)
        .signers([user])
        .rpc();

      // Record multiple runs
      for (let i = 0; i < 5; i++) {
        await program.methods
          .recordRunResult(i, i % 2 === 0) // alternating wins/losses
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
      }

      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.totalRuns).to.equal(5);
      expect((profile as any).availableRuns).to.equal(35);
    });

    it("T096: advances level on victory", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL,
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
        } as any)
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
        } as any)
        .signers([user])
        .rpc();

      // Verify level advanced
      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(1);
      expect((profile as any).availableRuns).to.equal(39);

      // Record another victory
      await program.methods
        .recordRunResult(1, true)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(2);
      expect((profile as any).availableRuns).to.equal(38);
    });

    it("T097: does not advance level on defeat", async () => {
      const user = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL,
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
        } as any)
        .signers([user])
        .rpc();

      // Advance to level 5
      for (let i = 0; i < 5; i++) {
        await program.methods
          .recordRunResult(i, true)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          } as any)
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
        } as any)
        .signers([user])
        .rpc();

      // Verify level did NOT advance
      profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.currentLevel).to.equal(5);
      expect(profile.totalRuns).to.equal(6);
      expect((profile as any).availableRuns).to.equal(34);
    });
  });
});
