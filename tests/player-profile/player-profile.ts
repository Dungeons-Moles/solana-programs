import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PlayerProfile } from "../../target/types/player_profile";
import { GameplayState } from "../../target/types/gameplay_state";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("player-profile", () => {
  const TREASURY = new anchor.web3.PublicKey(
    "5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8",
  );
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PlayerProfile as Program<PlayerProfile>;
  const gameplayProgram = anchor.workspace.GameplayState as Program<GameplayState>;

  const getGauntletConfigPDA = () =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("gauntlet_config")],
      gameplayProgram.programId,
    );
  const getGauntletPoolVaultPDA = () =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("gauntlet_pool_vault")],
      gameplayProgram.programId,
    );
  const getGauntletWeekPoolPDA = (week: number) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("gauntlet_week_pool"), Buffer.from([week])],
      gameplayProgram.programId,
    );
  const [GAUNTLET_POOL] = getGauntletPoolVaultPDA();
  let gauntletInitialized = false;

  // Helper to derive profile PDA
  const getProfilePDA = (owner: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("player"), owner.toBuffer()],
      program.programId,
    );
  };

  const ensureGauntletInitialized = async () => {
    if (gauntletInitialized) return;
    const [gauntletConfig] = getGauntletConfigPDA();
    const [gauntletPoolVault] = getGauntletPoolVaultPDA();
    const [gauntletWeek1] = getGauntletWeekPoolPDA(1);
    const [gauntletWeek2] = getGauntletWeekPoolPDA(2);
    const [gauntletWeek3] = getGauntletWeekPoolPDA(3);
    const [gauntletWeek4] = getGauntletWeekPoolPDA(4);
    const [gauntletWeek5] = getGauntletWeekPoolPDA(5);

    try {
      await gameplayProgram.methods
        .initializeGauntlet()
        .accounts({
          gauntletConfig,
          gauntletPoolVault,
          gauntletWeek1,
          gauntletWeek2,
          gauntletWeek3,
          gauntletWeek4,
          gauntletWeek5,
          admin: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([
          anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
            units: 1_400_000,
          }),
        ])
        .rpc();
    } catch (error: any) {
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }
    gauntletInitialized = true;
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
    it("rejects direct record_run_result mutation", async () => {
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

      try {
        await program.methods
          .recordRunResult(1, false)
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expect(error.toString()).to.include("DirectMutationDisabled");
      }

      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect(profile.totalRuns).to.equal(0);
      expect((profile as any).availableRuns).to.equal(20);
    });
  });

  describe("Purchase Runs", () => {
    before(async () => {
      await ensureGauntletInitialized();
    });

    it("rejects invalid gauntlet pool account", async () => {
      const user = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);
      await program.methods
        .initializeProfile("PurchaseRunsGuard")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      try {
        await program.methods
          .purchaseRuns()
          .accounts({
            playerProfile: profilePDA,
            owner: user.publicKey,
            treasury: TREASURY,
            gauntletPool: user.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Expected invalid gauntlet pool account to be rejected");
      } catch (error: any) {
        expect(error.toString()).to.satisfy((msg: string) =>
          msg.includes("InvalidGauntletPool") ||
          msg.includes("ConstraintAddress"),
        );
      }
    });

    it("accepts canonical gauntlet pool account", async () => {
      const user = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        user.publicKey,
        2 * LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [profilePDA] = getProfilePDA(user.publicKey);
      await program.methods
        .initializeProfile("PurchaseRunsCanonical")
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      await program.methods
        .purchaseRuns()
        .accounts({
          playerProfile: profilePDA,
          owner: user.publicKey,
          treasury: TREASURY,
          gauntletPool: GAUNTLET_POOL,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([user])
        .rpc();

      const profile = await program.account.playerProfile.fetch(profilePDA);
      expect((profile as any).availableRuns).to.equal(40);
    });
  });
});
