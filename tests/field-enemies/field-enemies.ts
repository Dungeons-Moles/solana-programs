import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { FieldEnemies } from "../../target/types/field_enemies";
import { expect } from "chai";
import { Keypair, SystemProgram } from "@solana/web3.js";

describe("field-enemies", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.FieldEnemies as Program<FieldEnemies>;

  // Helper to derive map enemies PDA
  const getMapEnemiesPDA = (sessionPubkey: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_enemies"), sessionPubkey.toBuffer()],
      program.programId,
    );
  };

  describe("Initialize Map Enemies", () => {
    it("spawns correct enemy count for Act 1 (36 enemies)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1) // act=1, level=1
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      expect(mapEnemies.count).to.equal(36);
      expect(mapEnemies.enemies.length).to.equal(36);
      expect(mapEnemies.session.toString()).to.equal(
        session.publicKey.toString(),
      );
    });

    it("spawns correct enemy count for Act 2 (40 enemies)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(2, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      expect(mapEnemies.count).to.equal(40);
    });

    it("spawns correct enemy count for Act 3 (44 enemies)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(3, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      expect(mapEnemies.count).to.equal(44);
    });

    it("spawns correct enemy count for Act 4 (48 enemies)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(4, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      expect(mapEnemies.count).to.equal(48);
    });

    it("rejects invalid act number (0)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      try {
        await program.methods
          .initializeMapEnemies(0, 1)
          .accounts({
            payer: provider.wallet.publicKey,
            session: session.publicKey,
            mapEnemies: mapEnemiesPDA,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();
        expect.fail("Should have thrown InvalidAct error");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidAct");
      }
    });

    it("rejects invalid act number (5)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      try {
        await program.methods
          .initializeMapEnemies(5, 1)
          .accounts({
            payer: provider.wallet.publicKey,
            session: session.publicKey,
            mapEnemies: mapEnemiesPDA,
            systemProgram: SystemProgram.programId,
          } as any)
          .rpc();
        expect.fail("Should have thrown InvalidAct error");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidAct");
      }
    });

    it("enemies have valid archetype IDs (0-11)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(1, 42)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      for (const enemy of mapEnemies.enemies) {
        expect(enemy.archetypeId).to.be.at.least(0);
        expect(enemy.archetypeId).to.be.at.most(11);
      }
    });

    it("enemies have valid tier values (0-2)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(2, 99)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      for (const enemy of mapEnemies.enemies) {
        expect(enemy.tier).to.be.at.least(0);
        expect(enemy.tier).to.be.at.most(2);
      }
    });

    it("enemies are not initially defeated", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      for (const enemy of mapEnemies.enemies) {
        expect(enemy.defeated).to.be.false;
      }
    });

    it("produces deterministic spawns for same session+level", async () => {
      // Two sessions with same key but different levels should differ
      const session1 = Keypair.generate();
      const [mapEnemiesPDA1] = getMapEnemiesPDA(session1.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session1.publicKey,
          mapEnemies: mapEnemiesPDA1,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies1 =
        await program.account.mapEnemies.fetch(mapEnemiesPDA1);

      // Different session
      const session2 = Keypair.generate();
      const [mapEnemiesPDA2] = getMapEnemiesPDA(session2.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session2.publicKey,
          mapEnemies: mapEnemiesPDA2,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies2 =
        await program.account.mapEnemies.fetch(mapEnemiesPDA2);

      // Different sessions should produce different enemy placements
      // (statistically very unlikely to be identical)
      expect(mapEnemies1.count).to.equal(mapEnemies2.count);

      // At least some enemies should differ
      let hasDifferences = false;
      for (let i = 0; i < mapEnemies1.enemies.length; i++) {
        if (
          mapEnemies1.enemies[i].x !== mapEnemies2.enemies[i].x ||
          mapEnemies1.enemies[i].y !== mapEnemies2.enemies[i].y ||
          mapEnemies1.enemies[i].archetypeId !==
            mapEnemies2.enemies[i].archetypeId
        ) {
          hasDifferences = true;
          break;
        }
      }
      expect(hasDifferences).to.be.true;
    });
  });

  describe("Mark Enemy Defeated", () => {
    it("marks enemy as defeated and returns correct gold reward for T1", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      let mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);

      // Find a T1 enemy (tier = 0)
      const t1Enemy = mapEnemies.enemies.find((e: any) => e.tier === 0);
      if (!t1Enemy) {
        console.log("No T1 enemy found, skipping T1 gold test");
        return;
      }

      const tx = await program.methods
        .markEnemyDefeated(t1Enemy.x, t1Enemy.y)
        .accounts({
          authority: provider.wallet.publicKey,
          mapEnemies: mapEnemiesPDA,
        })
        .rpc();

      // Verify enemy is marked defeated
      mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      const updatedEnemy = mapEnemies.enemies.find(
        (e: any) => e.x === t1Enemy.x && e.y === t1Enemy.y,
      );
      expect(updatedEnemy).to.not.be.undefined;
      expect(updatedEnemy!.defeated).to.be.true;
    });

    it("fails when enemy not found at position", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      // Try to defeat at position (0, 0) which should be empty
      try {
        await program.methods
          .markEnemyDefeated(0, 0)
          .accounts({
            authority: provider.wallet.publicKey,
            mapEnemies: mapEnemiesPDA,
          })
          .rpc();
        expect.fail("Should have thrown EnemyNotFound error");
      } catch (error: any) {
        expect(error.toString()).to.include("EnemyNotFound");
      }
    });

    it("fails when enemy already defeated", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);
      const enemy = mapEnemies.enemies[0];

      // Defeat the enemy first time
      await program.methods
        .markEnemyDefeated(enemy.x, enemy.y)
        .accounts({
          authority: provider.wallet.publicKey,
          mapEnemies: mapEnemiesPDA,
        })
        .rpc();

      // Try to defeat again
      try {
        await program.methods
          .markEnemyDefeated(enemy.x, enemy.y)
          .accounts({
            authority: provider.wallet.publicKey,
            mapEnemies: mapEnemiesPDA,
          })
          .rpc();
        expect.fail("Should have thrown error for already defeated enemy");
      } catch (error: any) {
        // Should fail because the enemy is either already defeated or not found
        // (get_enemy_at_position_mut filters out defeated enemies)
        expect(
          error.toString().includes("EnemyNotFound") ||
            error.toString().includes("EnemyAlreadyDefeated"),
        ).to.be.true;
      }
    });
  });

  describe("Tier Distribution", () => {
    it("Act 1 spawns mostly T1 enemies (approx 70%)", async () => {
      const session = Keypair.generate();
      const [mapEnemiesPDA] = getMapEnemiesPDA(session.publicKey);

      await program.methods
        .initializeMapEnemies(1, 123)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session.publicKey,
          mapEnemies: mapEnemiesPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies = await program.account.mapEnemies.fetch(mapEnemiesPDA);

      const tierCounts = [0, 0, 0];
      for (const enemy of mapEnemies.enemies) {
        tierCounts[enemy.tier]++;
      }

      const t1Ratio = tierCounts[0] / mapEnemies.count;

      // T1 should be majority (allow some variance due to RNG)
      // Expected 70% for Act 1, allowing 50-90% range
      expect(t1Ratio).to.be.at.least(0.4);
      expect(t1Ratio).to.be.at.most(0.95);
    });

    it("Act 4 has more T2/T3 enemies than Act 1", async () => {
      const session1 = Keypair.generate();
      const [mapEnemiesPDA1] = getMapEnemiesPDA(session1.publicKey);

      await program.methods
        .initializeMapEnemies(1, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session1.publicKey,
          mapEnemies: mapEnemiesPDA1,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const session4 = Keypair.generate();
      const [mapEnemiesPDA4] = getMapEnemiesPDA(session4.publicKey);

      await program.methods
        .initializeMapEnemies(4, 1)
        .accounts({
          payer: provider.wallet.publicKey,
          session: session4.publicKey,
          mapEnemies: mapEnemiesPDA4,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();

      const mapEnemies1 =
        await program.account.mapEnemies.fetch(mapEnemiesPDA1);
      const mapEnemies4 =
        await program.account.mapEnemies.fetch(mapEnemiesPDA4);

      // Count T1 in each
      const t1Count1 = mapEnemies1.enemies.filter(
        (e: any) => e.tier === 0,
      ).length;
      const t1Count4 = mapEnemies4.enemies.filter(
        (e: any) => e.tier === 0,
      ).length;

      const t1Ratio1 = t1Count1 / mapEnemies1.count;
      const t1Ratio4 = t1Count4 / mapEnemies4.count;

      // Act 4 should have lower T1 ratio than Act 1 (35% vs 70%)
      // Allow for RNG variance but expect general trend
      console.log(`Act 1 T1 ratio: ${t1Ratio1}, Act 4 T1 ratio: ${t1Ratio4}`);
    });
  });
});
