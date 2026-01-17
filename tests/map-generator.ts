import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MapGenerator } from "../target/types/map_generator";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("map-generator", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.MapGenerator as Program<MapGenerator>;

  // Helper to derive map config PDA
  const getMapConfigPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_config")],
      program.programId,
    );
  };

  let configInitialized = false;
  const [configPDA] = getMapConfigPDA();

  const ensureConfigExists = async () => {
    if (configInitialized) return;
    const admin = provider.wallet;
    try {
      await program.methods
        .initializeMapConfig()
        .accounts({
          mapConfig: configPDA,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (error: any) {
      // Config might already exist from previous test run
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }
    configInitialized = true;
  };

  describe("T070: Initialize Map Config with Default Seeds", () => {
    it("initializes map config with default seeds", async () => {
      const admin = provider.wallet;

      try {
        await program.methods
          .initializeMapConfig()
          .accounts({
            mapConfig: configPDA,
            admin: admin.publicKey,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();

        const config = await program.account.mapConfig.fetch(configPDA);
        expect(config.admin.toString()).to.equal(admin.publicKey.toString());
        expect(config.version).to.equal(1);

        // Verify default seeds (level i has seed i)
        for (let i = 0; i <= 80; i++) {
          expect(config.seeds[i].toNumber()).to.equal(i);
        }
        configInitialized = true;
      } catch (error: any) {
        // Config might already exist from previous test run
        if (!error.toString().includes("already in use")) {
          throw error;
        }
        configInitialized = true;
      }
    });
  });

  describe("T071: Returns Correct Seed for Level", () => {
    it("returns correct seed for level", async () => {
      await ensureConfigExists();

      // Test getting seed for level 0
      await program.methods
        .getMapSeed(0)
        .accounts({
          mapConfig: configPDA,
        })
        .rpc();

      // Test getting seed for level 40
      await program.methods
        .getMapSeed(40)
        .accounts({
          mapConfig: configPDA,
        })
        .rpc();

      // Test getting seed for level 80 (max)
      await program.methods
        .getMapSeed(80)
        .accounts({
          mapConfig: configPDA,
        })
        .rpc();

      // Verify the config has correct seeds
      const config = await program.account.mapConfig.fetch(configPDA);
      expect(config.seeds[0].toNumber()).to.equal(0);
      expect(config.seeds[40].toNumber()).to.equal(40);
      expect(config.seeds[80].toNumber()).to.equal(80);
    });
  });

  describe("T072: Admin Can Update Seed Mapping", () => {
    it("admin can update seed mapping", async () => {
      await ensureConfigExists();

      const admin = provider.wallet;
      const newSeed = new anchor.BN(999999);

      await program.methods
        .updateMapConfig(5, newSeed)
        .accounts({
          mapConfig: configPDA,
          admin: admin.publicKey,
        })
        .rpc();

      const config = await program.account.mapConfig.fetch(configPDA);
      expect(config.seeds[5].toNumber()).to.equal(999999);

      // Reset it back
      await program.methods
        .updateMapConfig(5, new anchor.BN(5))
        .accounts({
          mapConfig: configPDA,
          admin: admin.publicKey,
        })
        .rpc();
    });
  });

  describe("T073: Batch Updates Multiple Seeds", () => {
    it("batch updates multiple seeds", async () => {
      await ensureConfigExists();

      const admin = provider.wallet;
      const updates = [
        { level: 10, seed: new anchor.BN(1010) },
        { level: 20, seed: new anchor.BN(2020) },
        { level: 30, seed: new anchor.BN(3030) },
      ];

      await program.methods
        .batchUpdateMapConfig(updates)
        .accounts({
          mapConfig: configPDA,
          admin: admin.publicKey,
        })
        .rpc();

      const config = await program.account.mapConfig.fetch(configPDA);
      expect(config.seeds[10].toNumber()).to.equal(1010);
      expect(config.seeds[20].toNumber()).to.equal(2020);
      expect(config.seeds[30].toNumber()).to.equal(3030);

      // Reset them back
      await program.methods
        .batchUpdateMapConfig([
          { level: 10, seed: new anchor.BN(10) },
          { level: 20, seed: new anchor.BN(20) },
          { level: 30, seed: new anchor.BN(30) },
        ])
        .accounts({
          mapConfig: configPDA,
          admin: admin.publicKey,
        })
        .rpc();
    });
  });

  describe("T074: Rejects Invalid Level Number", () => {
    it("rejects invalid level number", async () => {
      await ensureConfigExists();

      // Level 81 is invalid (max is 80)
      try {
        await program.methods
          .getMapSeed(81)
          .accounts({
            mapConfig: configPDA,
          })
          .rpc();
        expect.fail("Should have thrown InvalidLevel error");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidLevel");
      }

      // Try to update invalid level
      const admin = provider.wallet;
      try {
        await program.methods
          .updateMapConfig(100, new anchor.BN(12345))
          .accounts({
            mapConfig: configPDA,
            admin: admin.publicKey,
          })
          .rpc();
        expect.fail("Should have thrown InvalidLevel error");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidLevel");
      }
    });
  });

  describe("T075: Verifies Map Hash", () => {
    it("verifies map hash", async () => {
      await ensureConfigExists();

      // Generate a hash for level 0 (seed 0, but converted to 1 in RNG)
      // We need to match the on-chain computation
      // For testing, we'll just verify the instruction works with a known-bad hash
      const badHash = new Array(32).fill(0);

      try {
        await program.methods
          .verifyMapHash(0, badHash)
          .accounts({
            mapConfig: configPDA,
          })
          .rpc();
        expect.fail("Should have thrown InvalidMapHash error for bad hash");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidMapHash");
      }
    });
  });

  describe("T076: RNG Produces Deterministic Results", () => {
    it("RNG produces deterministic results (verified via Rust tests)", async () => {
      // This test is primarily verified via Rust unit tests in rng.rs
      // Here we just verify the map-generator program is accessible
      await ensureConfigExists();

      const config = await program.account.mapConfig.fetch(configPDA);
      expect(config.seeds.length).to.equal(81);
      expect(config.version).to.equal(1);
    });
  });

  describe("Transfer Admin", () => {
    it("transfers admin authority", async () => {
      await ensureConfigExists();

      const admin = provider.wallet;
      const newAdmin = Keypair.generate();

      // Transfer to new admin
      await program.methods
        .transferAdmin(newAdmin.publicKey)
        .accounts({
          mapConfig: configPDA,
          admin: admin.publicKey,
        })
        .rpc();

      let config = await program.account.mapConfig.fetch(configPDA);
      expect(config.admin.toString()).to.equal(newAdmin.publicKey.toString());

      // Transfer back (need to sign with new admin now)
      // First, fund the new admin
      const airdropSig = await provider.connection.requestAirdrop(
        newAdmin.publicKey,
        LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      await program.methods
        .transferAdmin(admin.publicKey)
        .accounts({
          mapConfig: configPDA,
          admin: newAdmin.publicKey,
        })
        .signers([newAdmin])
        .rpc();

      config = await program.account.mapConfig.fetch(configPDA);
      expect(config.admin.toString()).to.equal(admin.publicKey.toString());
    });

    it("rejects unauthorized admin operations", async () => {
      await ensureConfigExists();

      const nonAdmin = Keypair.generate();

      // Fund the non-admin
      const airdropSig = await provider.connection.requestAirdrop(
        nonAdmin.publicKey,
        LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      // Try to update as non-admin
      try {
        await program.methods
          .updateMapConfig(0, new anchor.BN(99999))
          .accounts({
            mapConfig: configPDA,
            admin: nonAdmin.publicKey,
          })
          .signers([nonAdmin])
          .rpc();
        expect.fail("Should have thrown Unauthorized error");
      } catch (error: any) {
        expect(error.toString()).to.include("Unauthorized");
      }
    });
  });
});
