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

  describe("Initialize Profile", () => {
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
      // Field renamed: currentLevel -> highestLevelUnlocked (starts at 1)
      expect((profile as any).highestLevelUnlocked).to.equal(1);
      // availableRuns initial value is now 20 (not 40)
      expect((profile as any).availableRuns).to.equal(20);
      expect(profile.createdAt.toNumber()).to.be.greaterThan(0);
    });
  });

  describe("Reject Duplicate Profile", () => {
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

  describe("Update Profile Name", () => {
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

  describe("Reject Name Too Long", () => {
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
    it("records run completion", async () => {
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
      // availableRuns is NOT decremented by record_run_result anymore
      // It's decremented by consume_run at session start instead
      expect((profile as any).availableRuns).to.equal(20);
    });

    it("increments total_runs on completion", async () => {
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
      // availableRuns is NOT decremented by record_run_result anymore
      // It's decremented by consume_run at session start instead
      expect((profile as any).availableRuns).to.equal(20);
    });

    it("advances level on victory", async () => {
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

      // Verify starting level (field renamed: currentLevel -> highestLevelUnlocked, starts at 1)
      let profile = await program.account.playerProfile.fetch(profilePDA);
      expect((profile as any).highestLevelUnlocked).to.equal(1);

      // Record a victory at level 1
      await program.methods
        .recordRunResult(1, true)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify level advanced to 2
      profile = await program.account.playerProfile.fetch(profilePDA);
      expect((profile as any).highestLevelUnlocked).to.equal(2);
      // availableRuns is NOT decremented by record_run_result anymore
      expect((profile as any).availableRuns).to.equal(20);

      // Record another victory at level 2
      await program.methods
        .recordRunResult(2, true)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      profile = await program.account.playerProfile.fetch(profilePDA);
      expect((profile as any).highestLevelUnlocked).to.equal(3);
      // availableRuns is NOT decremented by record_run_result anymore
      expect((profile as any).availableRuns).to.equal(20);
    });

    it("does not advance level on defeat", async () => {
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

      // Advance to level 6 (start at 1, win 5 times to get to level 6)
      for (let i = 1; i <= 5; i++) {
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
      expect((profile as any).highestLevelUnlocked).to.equal(6);

      // Record a defeat at level 6
      await program.methods
        .recordRunResult(6, false)
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify level did NOT advance
      profile = await program.account.playerProfile.fetch(profilePDA);
      expect((profile as any).highestLevelUnlocked).to.equal(6);
      expect(profile.totalRuns).to.equal(6);
      // availableRuns is NOT decremented by record_run_result anymore
      // It's decremented by consume_run at session start instead
      expect((profile as any).availableRuns).to.equal(20);
    });
  });
});
