import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { MapGenerator } from "../../target/types/map_generator";
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

  describe("Initialize Map Config with Default Seeds", () => {
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

        // Verify default seeds (level i has seed i, LEVEL_COUNT = 40)
        for (let i = 0; i < 40; i++) {
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

  describe("RNG Produces Deterministic Results", () => {
    it("RNG produces deterministic results (verified via Rust tests)", async () => {
      // This test is primarily verified via Rust unit tests in rng.rs
      // Here we just verify the map-generator program is accessible
      await ensureConfigExists();

      const config = await program.account.mapConfig.fetch(configPDA);
      expect(config.seeds.length).to.equal(40); // LEVEL_COUNT = 40
      expect(config.version).to.equal(1);
    });
  });

  // Helper to derive generated map PDA
  const getGeneratedMapPDA = (session: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("generated_map"), session.toBuffer()],
      program.programId,
    );
  };

  describe("Generate Map", () => {
    it("generates a map for campaign level 1", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;
      // Create a mock session keypair (just needs to be a unique pubkey for PDA derivation)
      const session = Keypair.generate();
      const [generatedMapPDA] = getGeneratedMapPDA(session.publicKey);

      await program.methods
        .generateMap(1) // Level 1
        .accounts({
          payer: payer.publicKey,
          session: session.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      const generatedMap =
        await program.account.generatedMap.fetch(generatedMapPDA);

      // Verify map dimensions
      expect(generatedMap.width).to.equal(50);
      expect(generatedMap.height).to.equal(50);

      // Verify session reference
      expect(generatedMap.session.toString()).to.equal(
        session.publicKey.toString(),
      );

      // Verify seed was used (level 1 = index 0 = seed 1)
      expect(generatedMap.seed.toNumber()).to.equal(1);

      // Verify walkable count > 0
      expect(generatedMap.walkableCount).to.be.greaterThan(0);

      // Verify spawn point is within map bounds
      expect(generatedMap.spawnX).to.be.lessThan(50);
      expect(generatedMap.spawnY).to.be.lessThan(50);
      expect(generatedMap.spawnY).to.be.greaterThan(0); // spawn_y > 0 (mole den above)

      // Verify mole den is directly above spawn
      expect(generatedMap.moleDenX).to.equal(generatedMap.spawnX);
      expect(generatedMap.moleDenY).to.equal(generatedMap.spawnY - 1);

      // Verify enemy count is 0 (Phase 2)
      expect(generatedMap.enemyCount).to.be.greaterThan(0);
      expect(generatedMap.enemyCount).to.be.at.most(48);

      // Verify POIs placed (L1 Mole Den + randomized POIs)
      expect(generatedMap.poiCount).to.be.greaterThan(0);
      expect(generatedMap.pois[0].poiType).to.equal(1);
      expect(generatedMap.pois[0].isUsed).to.equal(false);
      expect(generatedMap.pois[0].x).to.equal(generatedMap.moleDenX);
      expect(generatedMap.pois[0].y).to.equal(generatedMap.moleDenY);

      const poiPositions = new Set<string>();
      for (let i = 0; i < generatedMap.poiCount; i++) {
        const poi = generatedMap.pois[i];
        poiPositions.add(`${poi.x},${poi.y}`);
      }

      for (let i = 0; i < generatedMap.enemyCount; i++) {
        const enemy = generatedMap.enemies[i];
        expect(enemy.x).to.be.lessThan(50);
        expect(enemy.y).to.be.lessThan(50);
        expect(enemy.archetypeId).to.be.lessThan(12);
        expect(enemy.tier).to.be.at.most(2);
        expect(
          enemy.x === generatedMap.spawnX && enemy.y === generatedMap.spawnY,
        ).to.equal(false);
        expect(
          enemy.x === generatedMap.moleDenX &&
            enemy.y === generatedMap.moleDenY,
        ).to.equal(false);
        expect(poiPositions.has(`${enemy.x},${enemy.y}`)).to.equal(false);
      }
    });

    it("generates different maps for different levels", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;

      // Generate map for level 2
      const session2 = Keypair.generate();
      const [generatedMapPDA2] = getGeneratedMapPDA(session2.publicKey);

      await program.methods
        .generateMap(2) // Level 2
        .accounts({
          payer: payer.publicKey,
          session: session2.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA2,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      // Generate map for level 3
      const session3 = Keypair.generate();
      const [generatedMapPDA3] = getGeneratedMapPDA(session3.publicKey);

      await program.methods
        .generateMap(3) // Level 3
        .accounts({
          payer: payer.publicKey,
          session: session3.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA3,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      const map2 = await program.account.generatedMap.fetch(generatedMapPDA2);
      const map3 = await program.account.generatedMap.fetch(generatedMapPDA3);

      // Different seeds should produce different maps
      expect(map2.seed.toNumber()).to.equal(2);
      expect(map3.seed.toNumber()).to.equal(3);

      // Packed tiles should be different (different mazes)
      expect(Buffer.from(map2.packedTiles).toString("hex")).to.not.equal(
        Buffer.from(map3.packedTiles).toString("hex"),
      );
    });

    it("produces deterministic maps for same level", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;

      // Generate two maps for the same level (but different sessions)
      const session1 = Keypair.generate();
      const session2 = Keypair.generate();
      const [generatedMapPDA1] = getGeneratedMapPDA(session1.publicKey);
      const [generatedMapPDA2] = getGeneratedMapPDA(session2.publicKey);

      await program.methods
        .generateMap(5) // Level 5
        .accounts({
          payer: payer.publicKey,
          session: session1.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA1,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      await program.methods
        .generateMap(5) // Level 5 again
        .accounts({
          payer: payer.publicKey,
          session: session2.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA2,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      const map1 = await program.account.generatedMap.fetch(generatedMapPDA1);
      const map2 = await program.account.generatedMap.fetch(generatedMapPDA2);

      // Same seed should produce same map structure
      expect(map1.seed.toNumber()).to.equal(map2.seed.toNumber());
      expect(Buffer.from(map1.packedTiles).toString("hex")).to.equal(
        Buffer.from(map2.packedTiles).toString("hex"),
      );
      expect(map1.spawnX).to.equal(map2.spawnX);
      expect(map1.spawnY).to.equal(map2.spawnY);
      expect(map1.walkableCount).to.equal(map2.walkableCount);
    });

    it("rejects invalid campaign level 0", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;
      const session = Keypair.generate();
      const [generatedMapPDA] = getGeneratedMapPDA(session.publicKey);

      try {
        await program.methods
          .generateMap(0) // Invalid level
          .accounts({
            payer: payer.publicKey,
            session: session.publicKey,
            mapConfig: configPDA,
            generatedMap: generatedMapPDA,
            systemProgram: SystemProgram.programId,
          } as any)
          .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
          .rpc();
        expect.fail("Should have thrown InvalidLevel error");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidLevel");
      }
    });

    it("rejects invalid campaign level 41", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;
      const session = Keypair.generate();
      const [generatedMapPDA] = getGeneratedMapPDA(session.publicKey);

      try {
        await program.methods
          .generateMap(41) // Invalid level (max is 40)
          .accounts({
            payer: payer.publicKey,
            session: session.publicKey,
            mapConfig: configPDA,
            generatedMap: generatedMapPDA,
            systemProgram: SystemProgram.programId,
          } as any)
          .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
          .rpc();
        expect.fail("Should have thrown InvalidLevel error");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidLevel");
      }
    });

    it("rejects duplicate map for same session", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;
      const session = Keypair.generate();
      const [generatedMapPDA] = getGeneratedMapPDA(session.publicKey);

      // First map generation should succeed
      await program.methods
        .generateMap(10)
        .accounts({
          payer: payer.publicKey,
          session: session.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      // Second map generation for same session should fail
      try {
        await program.methods
          .generateMap(10)
          .accounts({
            payer: payer.publicKey,
            session: session.publicKey,
            mapConfig: configPDA,
            generatedMap: generatedMapPDA,
            systemProgram: SystemProgram.programId,
          } as any)
          .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
          .rpc();
        expect.fail("Should have failed - map already exists");
      } catch (error: any) {
        // Account already initialized error
        expect(error.toString()).to.include("already in use");
      }
    });
  });

  describe("Map Tile Verification", () => {
    it("verifies spawn point and mole den are on floor tiles", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;
      const session = Keypair.generate();
      const [generatedMapPDA] = getGeneratedMapPDA(session.publicKey);

      await program.methods
        .generateMap(15)
        .accounts({
          payer: payer.publicKey,
          session: session.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      const map = await program.account.generatedMap.fetch(generatedMapPDA);

      // Helper to check if tile is walkable (floor) from packed tiles
      const isWalkable = (x: number, y: number): boolean => {
        const index = y * 50 + x;
        const byteIndex = Math.floor(index / 8);
        const bitIndex = index % 8;
        // 0 = floor (walkable), 1 = wall
        return ((map.packedTiles[byteIndex] >> bitIndex) & 1) === 0;
      };

      // Spawn point should be walkable
      expect(isWalkable(map.spawnX, map.spawnY)).to.be.true;

      // Mole den (above spawn) should also be walkable (floor tile for POI interaction)
      expect(isWalkable(map.moleDenX, map.moleDenY)).to.be.true;
    });

    it("verifies walkable count matches actual floor tiles", async () => {
      await ensureConfigExists();

      const payer = provider.wallet;
      const session = Keypair.generate();
      const [generatedMapPDA] = getGeneratedMapPDA(session.publicKey);

      await program.methods
        .generateMap(20)
        .accounts({
          payer: payer.publicKey,
          session: session.publicKey,
          mapConfig: configPDA,
          generatedMap: generatedMapPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .rpc();

      const map = await program.account.generatedMap.fetch(generatedMapPDA);

      // Count walkable tiles manually
      let walkableCount = 0;
      for (let y = 0; y < 50; y++) {
        for (let x = 0; x < 50; x++) {
          const index = y * 50 + x;
          const byteIndex = Math.floor(index / 8);
          const bitIndex = index % 8;
          if (((map.packedTiles[byteIndex] >> bitIndex) & 1) === 0) {
            walkableCount++;
          }
        }
      }

      expect(map.walkableCount).to.equal(walkableCount);
    });
  });
});
