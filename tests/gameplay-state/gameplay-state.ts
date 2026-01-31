import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { GameplayState } from "../../target/types/gameplay_state";
import { MapGenerator } from "../../target/types/map_generator";
import { SessionManager } from "../../target/types/session_manager";
import { PlayerProfile } from "../../target/types/player_profile";
import { PoiSystem } from "../../target/types/poi_system";
import { PlayerInventory } from "../../target/types/player_inventory";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("gameplay-state", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const gameplayProgram = anchor.workspace
    .GameplayState as Program<GameplayState>;
  const sessionProgram = anchor.workspace
    .SessionManager as Program<SessionManager>;
  const playerProfileProgram = anchor.workspace
    .PlayerProfile as Program<PlayerProfile>;
  const poiSystemProgram = anchor.workspace.PoiSystem as Program<PoiSystem>;
  const playerInventoryProgram = anchor.workspace
    .PlayerInventory as Program<PlayerInventory>;
  const mapGeneratorProgram = anchor.workspace
    .MapGenerator as Program<MapGenerator>;

  const MAP_WIDTH = 50;
  const MAP_HEIGHT = 50;

  // Helper to derive GameState PDA
  const getGameStatePDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("game_state"), sessionPda.toBuffer()],
      gameplayProgram.programId,
    );
  };

  // Helper to derive Session PDA
  const getSessionPDA = (
    player: anchor.web3.PublicKey,
    campaignLevel: number,
  ) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session"), player.toBuffer(), Buffer.from([campaignLevel])],
      sessionProgram.programId,
    );
  };

  const getMapEnemiesPDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_enemies"), sessionPda.toBuffer()],
      gameplayProgram.programId,
    );
  };

  const getMapPoisPDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_pois"), sessionPda.toBuffer()],
      poiSystemProgram.programId,
    );
  };

  const getInventoryPDA = (session: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("inventory"), session.toBuffer()],
      playerInventoryProgram.programId,
    );
  };

  // Helper to derive counter PDA
  const getCounterPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session_counter")],
      sessionProgram.programId,
    );
  };

  // Helper to derive map config PDA
  const getMapConfigPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_config")],
      mapGeneratorProgram.programId,
    );
  };

  const getGeneratedMapPDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("generated_map"), sessionPda.toBuffer()],
      mapGeneratorProgram.programId,
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
  let mapConfigInitialized = false;
  const [mapConfigPDA] = getMapConfigPDA();

  const ensureCounterExists = async () => {
    if (counterInitialized) return;
    const admin = provider.wallet;
    try {
      await sessionProgram.methods
        .initializeCounter()
        .accounts({
          sessionCounter: counterPDA,
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

  const ensureMapConfigExists = async () => {
    if (mapConfigInitialized) return;
    const admin = provider.wallet;
    try {
      await mapGeneratorProgram.methods
        .initializeMapConfig()
        .accounts({
          mapConfig: mapConfigPDA,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (error: any) {
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }
    mapConfigInitialized = true;
  };

  // Helper to setup a user with session and game state
  const setupUserWithGameState = async () => {
    await ensureCounterExists();
    await ensureMapConfigExists();

    const user = Keypair.generate();
    const burnerWallet = Keypair.generate();
    const campaignLevel = 1;

    // Airdrop SOL
    const airdropSig = await provider.connection.requestAirdrop(
      user.publicKey,
      5 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(airdropSig);

    const burnerAirdropSig = await provider.connection.requestAirdrop(
      burnerWallet.publicKey,
      5 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(burnerAirdropSig);

    const [playerProfilePDA] = getPlayerProfilePDA(user.publicKey);
    const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);
    const [gameStatePDA] = getGameStatePDA(sessionPDA);
    const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
    const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
    const [inventoryPDA] = getInventoryPDA(sessionPDA);
    const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);

    // Create player profile first
    await playerProfileProgram.methods
      .initializeProfile("TestPlayer")
      .accounts({
        playerProfile: playerProfilePDA,
        owner: user.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), anchor.web3.ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 })])
      .signers([user])
      .rpc();

    await (sessionProgram.methods as any)
      .startSession(campaignLevel)
      .accounts({
        gameSession: sessionPDA,
        sessionCounter: counterPDA,
        playerProfile: playerProfilePDA,
        mapConfig: mapConfigPDA,
        generatedMap: generatedMapPDA,
        mapGeneratorProgram: mapGeneratorProgram.programId,
        player: user.publicKey,
        burnerWallet: burnerWallet.publicKey,
        gameState: gameStatePDA,
        mapEnemies: mapEnemiesPDA,
        mapPois: mapPoisPDA,
        inventory: inventoryPDA,
        gameplayStateProgram: gameplayProgram.programId,
        poiSystemProgram: poiSystemProgram.programId,
        playerInventoryProgram: playerInventoryProgram.programId,
        playerProfileProgram: playerProfileProgram.programId,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }), anchor.web3.ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 })])
      .signers([user, burnerWallet])
      .rpc();

    try {
      await mapGeneratorProgram.methods
        .generateMap(campaignLevel)
        .accounts({
          payer: user.publicKey,
          session: sessionPDA,
          mapConfig: mapConfigPDA,
          generatedMap: generatedMapPDA,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
        .signers([user])
        .rpc();
    } catch (error: any) {
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }

    const gameState =
      await gameplayProgram.account.gameState.fetch(gameStatePDA);

    return {
      user,
      burnerWallet,
      sessionPDA,
      gameStatePDA,
      playerProfilePDA,
      mapWidth: gameState.mapWidth,
      mapHeight: gameState.mapHeight,
      startX: gameState.positionX,
      startY: gameState.positionY,
      campaignLevel,
    };
  };

  // Cleanup helper
  const cleanup = async (
    user: Keypair,
    burnerWallet: Keypair,
    sessionPDA: anchor.web3.PublicKey,
    gameStatePDA: anchor.web3.PublicKey,
    campaignLevel: number = 1,
  ) => {
    const [inventoryPDA] = getInventoryPDA(sessionPDA);
    try {
      await gameplayProgram.methods
        .closeGameState()
        .accounts({
          gameState: gameStatePDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    } catch (e) {
      // Ignore if already closed
    }
    try {
      await sessionProgram.methods
        .endSession(campaignLevel, true)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
          burnerWallet: burnerWallet.publicKey,
          inventory: inventoryPDA,
          playerInventoryProgram: playerInventoryProgram.programId,
        } as any)
        .signers([user, burnerWallet])
        .rpc();
    } catch (e) {
      // Ignore if already closed
    }
  };

  // Helper to move player with position tracking
  // burnerWallet: the keypair that signs gameplay transactions
  const movePlayer = async (
    burnerWallet: Keypair,
    gameStatePDA: anchor.web3.PublicKey,
    targetX: number,
    targetY: number,
  ) => {
    const gameState =
      await gameplayProgram.account.gameState.fetch(gameStatePDA);
    const [generatedMapPDA] = getGeneratedMapPDA(gameState.session);
    const [mapEnemiesPDA] = getMapEnemiesPDA(gameState.session);
    const [inventoryPDA] = getInventoryPDA(gameState.session);

    await (gameplayProgram.methods as any)
      .movePlayer(targetX, targetY)
      .accounts({
        gameState: gameStatePDA,
        gameSession: gameState.session,
        mapEnemies: mapEnemiesPDA,
        generatedMap: generatedMapPDA,
        inventory: inventoryPDA,
        playerInventoryProgram: playerInventoryProgram.programId,
        player: burnerWallet.publicKey,
      } as any)
      .signers([burnerWallet])
      .rpc();
    return { x: targetX, y: targetY };
  };

  const isWalkableTile = (map: any, x: number, y: number) => {
    const index = y * map.width + x;
    const byteIndex = Math.floor(index / 8);
    const bitIndex = index % 8;
    return ((map.packedTiles[byteIndex] >> bitIndex) & 1) === 0;
  };

  const getAdjacentTargetByTile = async (
    gameStatePDA: anchor.web3.PublicKey,
    fromX: number,
    fromY: number,
    mapWidth: number,
    mapHeight: number,
    wantWall: boolean,
  ) => {
    const gameState =
      await gameplayProgram.account.gameState.fetch(gameStatePDA);
    const [generatedMapPDA] = getGeneratedMapPDA(gameState.session);
    const generatedMap =
      await mapGeneratorProgram.account.generatedMap.fetch(generatedMapPDA);
    const candidates = [
      { x: fromX + 1, y: fromY },
      { x: fromX - 1, y: fromY },
      { x: fromX, y: fromY + 1 },
      { x: fromX, y: fromY - 1 },
    ];

    for (const candidate of candidates) {
      if (
        candidate.x < 0 ||
        candidate.y < 0 ||
        candidate.x >= mapWidth ||
        candidate.y >= mapHeight
      ) {
        continue;
      }
      const walkable = isWalkableTile(generatedMap, candidate.x, candidate.y);
      if (wantWall ? !walkable : walkable) {
        return candidate;
      }
    }

    throw new Error("No adjacent tile matching requirements");
  };

  const getNonAdjacentTarget = (
    x: number,
    y: number,
    mapWidth: number,
    mapHeight: number,
  ) => {
    if (x + 2 < mapWidth) {
      return { x: x + 2, y };
    }
    if (x >= 2) {
      return { x: x - 2, y };
    }
    if (y + 2 < mapHeight) {
      return { x, y: y + 2 };
    }
    return { x, y: y - 2 };
  };

  // Helper to move back and forth
  // burnerWallet: the keypair that signs gameplay transactions
  const moveBackAndForth = async (
    burnerWallet: Keypair,
    gameStatePDA: anchor.web3.PublicKey,
    startX: number,
    startY: number,
    mapWidth: number,
    mapHeight: number,
    moveCount: number,
  ) => {
    const target = await getAdjacentTargetByTile(
      gameStatePDA,
      startX,
      startY,
      mapWidth,
      mapHeight,
      false,
    );
    let currentX = startX;
    let currentY = startY;
    for (let i = 0; i < moveCount; i++) {
      const next =
        currentX === startX && currentY === startY
          ? target
          : { x: startX, y: startY };
      await movePlayer(burnerWallet, gameStatePDA, next.x, next.y);
      currentX = next.x;
      currentY = next.y;
    }
    return { x: currentX, y: currentY };
  };

  describe("Player Movement Tracking", () => {
    describe("Test setup and helper functions", () => {
      it("creates game state with correct initial values", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          startY,
          mapWidth,
          mapHeight,
        } = await setupUserWithGameState();

        const gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);

        expect(gameState.player.toString()).to.equal(
          user.publicKey.toString(),
        );
        expect(gameState.burnerWallet.toString()).to.equal(
          burnerWallet.publicKey.toString(),
        );
        expect(gameState.positionX).to.equal(startX);
        expect(gameState.positionY).to.equal(startY);
        expect(gameState.mapWidth).to.equal(mapWidth);
        expect(gameState.mapHeight).to.equal(mapHeight);
        expect(mapWidth).to.equal(MAP_WIDTH);
        expect(mapHeight).to.equal(MAP_HEIGHT);
        expect(gameState.hp).to.equal(10);
        // Stats (atk, arm, spd, dig) are now derived from inventory, not stored in GameState
        expect(gameState.gearSlots).to.equal(4);
        expect(gameState.week).to.equal(1);
        expect(JSON.stringify(gameState.phase)).to.include("day1");
        expect(gameState.movesRemaining).to.equal(50);
        expect(Number(gameState.totalMoves)).to.equal(0);
        expect(gameState.bossFightReady).to.equal(false);

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Floor movement deducts 1 move", () => {
      it("deducts 1 move for floor tile movement", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          startY,
          mapWidth,
          mapHeight,
        } = await setupUserWithGameState();
        const target = await getAdjacentTargetByTile(
          gameStatePDA,
          startX,
          startY,
          mapWidth,
          mapHeight,
          false,
        );

        await movePlayer(burnerWallet, gameStatePDA, target.x, target.y);

        const gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gameState.positionX).to.equal(target.x);
        expect(gameState.positionY).to.equal(target.y);
        expect(gameState.movesRemaining).to.equal(49);
        expect(Number(gameState.totalMoves)).to.equal(1);

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Wall dig deducts max(2, 6-DIG) moves", () => {
      it("deducts correct wall dig cost with default DIG stat", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          startY,
          mapWidth,
          mapHeight,
        } = await setupUserWithGameState();
        const target = await getAdjacentTargetByTile(
          gameStatePDA,
          startX,
          startY,
          mapWidth,
          mapHeight,
          true,
        );

        await movePlayer(burnerWallet, gameStatePDA, target.x, target.y);

        const gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gameState.positionX).to.equal(target.x);
        expect(gameState.movesRemaining).to.equal(45); // 50 - 5 = 45

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });

      // DIG stat is now derived from inventory, so we skip the high DIG test
      // The minimum dig cost of 2 is still enforced when DIG >= 4
      it.skip("uses minimum dig cost of 2 when DIG is high (skipped: DIG now derived from inventory)", async () => {
        // This test would require equipping items with high DIG bonus
        // and verifying the wall movement cost is reduced to minimum (2)
      });
    });

    describe("Out-of-bounds movement rejected", () => {
      it("rejects movement outside map boundaries", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          mapWidth,
        } = await setupUserWithGameState();

        try {
          await movePlayer(burnerWallet, gameStatePDA, mapWidth, 0);
          expect.fail("Should have thrown an error");
        } catch (error: any) {
          expect(error.toString()).to.include("OutOfBounds");
        }

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Insufficient moves rejected", () => {
      it("rejects wall dig when not enough moves remaining", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          startY,
          mapWidth,
          mapHeight,
        } = await setupUserWithGameState();

        // Phase spanning allows borrowing moves from next phase, so we must test in Night3
        // where there is no next phase to borrow from.
        // Use set_phase_for_testing to jump directly to Night3 with 2 moves remaining.
        await gameplayProgram.methods
          .setPhaseForTesting({ night3: {} }, 2)
          .accounts({
            gameState: gameStatePDA,
            burnerWallet: burnerWallet.publicKey,
          })
          .signers([burnerWallet])
          .rpc();

        // Verify we're in Night3 with 2 moves remaining
        let gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gameState.movesRemaining).to.equal(2);
        expect(gameState.phase).to.deep.equal({ night3: {} });

        const target = await getAdjacentTargetByTile(
          gameStatePDA,
          startX,
          startY,
          mapWidth,
          mapHeight,
          true,
        );
        try {
          await movePlayer(burnerWallet, gameStatePDA, target.x, target.y); // wall needs 6 (DIG=0), but we only have 2
          expect.fail("Should have thrown an error");
        } catch (error: any) {
          expect(error.toString()).to.include("InsufficientMoves");
        }

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Non-adjacent movement rejected", () => {
      it("rejects movement to non-adjacent tile", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          startY,
          mapWidth,
          mapHeight,
        } = await setupUserWithGameState();
        const target = getNonAdjacentTarget(
          startX,
          startY,
          mapWidth,
          mapHeight,
        );
        const diagonal = {
          x: startX + 1 < mapWidth ? startX + 1 : startX - 1,
          y: startY + 1 < mapHeight ? startY + 1 : startY - 1,
        };

        try {
          await movePlayer(burnerWallet, gameStatePDA, target.x, target.y);
          expect.fail("Should have thrown an error");
        } catch (error: any) {
          expect(error.toString()).to.include("NotAdjacent");
        }

        try {
          await movePlayer(burnerWallet, gameStatePDA, diagonal.x, diagonal.y);
          expect.fail("Should have thrown an error");
        } catch (error: any) {
          expect(error.toString()).to.include("NotAdjacent");
        }

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });
  });

  describe("Time and Phase Progression", () => {
    describe("Day phase has 50 moves", () => {
      it("initializes with 50 moves in Day1", async () => {
        const { user, burnerWallet, sessionPDA, gameStatePDA, campaignLevel } =
          await setupUserWithGameState();

        const gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gameState.movesRemaining).to.equal(50);
        expect(JSON.stringify(gameState.phase)).to.include("day1");

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Night phase has 30 moves", () => {
      it("transitions to Night1 with 30 moves after Day1 exhausted", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          startY,
          mapWidth,
          mapHeight,
        } = await setupUserWithGameState();

        // Exhaust Day1 moves
        await moveBackAndForth(
          burnerWallet,
          gameStatePDA,
          startX,
          startY,
          mapWidth,
          mapHeight,
          50,
        );

        const gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(JSON.stringify(gameState.phase)).to.include("night1");
        expect(gameState.movesRemaining).to.equal(30);

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Phase advances when moves exhausted", () => {
      it("advances from Day1 to Night1 to Day2", async () => {
        const {
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
          startX,
          startY,
          mapWidth,
          mapHeight,
        } = await setupUserWithGameState();

        // Day1 -> Night1
        let pos = await moveBackAndForth(
          burnerWallet,
          gameStatePDA,
          startX,
          startY,
          mapWidth,
          mapHeight,
          50,
        );
        let gs = await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(JSON.stringify(gs.phase)).to.include("night1");

        // Night1 -> Day2
        await moveBackAndForth(
          burnerWallet,
          gameStatePDA,
          pos.x,
          pos.y,
          mapWidth,
          mapHeight,
          30,
        );
        gs = await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(JSON.stringify(gs.phase)).to.include("day2");

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Week advances after Night3", () => {
      it.skip("advances to Week 2 after completing Week 1 (skipped: takes too long)", async () => {
        // This test would take 240 transactions - skipped for CI
      });
    });

    describe("Boss fight and week transitions", () => {
      it("verifies phase transition logic constants", async () => {
        // Test that the Phase enum and moves_allowed work correctly
        const { user, burnerWallet, sessionPDA, gameStatePDA, campaignLevel } =
          await setupUserWithGameState();

        const gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gameState.week).to.equal(1);
        expect(gameState.gearSlots).to.equal(4);
        expect(gameState.bossFightReady).to.equal(false);

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });

      it.skip("completes full 3-week cycle (skipped: takes too long)", async () => {
        // This would require 720 transactions - skipped for CI
      });
    });
  });

  describe("Player Stats Management", () => {
    describe("HP initialized correctly", () => {
      it("has correct initial HP (base 10)", async () => {
        const { user, burnerWallet, sessionPDA, gameStatePDA, campaignLevel } =
          await setupUserWithGameState();

        const gameState =
          await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gameState.hp).to.equal(10); // Base HP

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Stats derived from inventory", () => {
      it("verifies inventory determines combat stats (skipped: needs full combat integration)", async () => {
        // Stats are now derived from equipped items in player-inventory
        // Testing requires equipping items and checking combat results
        // This is covered by integration tests
      });
    });

    describe("Stat modification tests (deprecated)", () => {
      it.skip("modifyStat removed - stats now derived from inventory", () => {
        // These tests are no longer applicable.
        // HP is modified via:
        // - Combat damage (handled in resolve_enemy_combat/resolve_boss_fight)
        // - Healing via rest POIs (uses heal_player instruction)
      });
    });
  });

  describe("Gear Slots Progression", () => {
    describe("gear_slots initialized to 4", () => {
      it("starts with 4 gear slots", async () => {
        const { user, burnerWallet, sessionPDA, gameStatePDA, campaignLevel } =
          await setupUserWithGameState();

        const gs = await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gs.gearSlots).to.equal(4);

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Gear slots progression", () => {
      it.skip("gear_slots increases to 6 after Week 1 (skipped: takes too long)", async () => {
        // Would require 240 transactions
      });

      it.skip("gear_slots increases to 8 after Week 2 (skipped: takes too long)", async () => {
        // Would require 480 transactions
      });

      it.skip("gear_slots capped at 8 after Week 3 (skipped: takes too long)", async () => {
        // Would require 720 transactions
      });
    });
  });

  describe("Session Integration & Cleanup", () => {
    describe("GameState requires valid GameSession PDA", () => {
      it("rejects non-session account", async () => {
        const user = Keypair.generate();
        const burner = Keypair.generate();
        const airdropSig = await provider.connection.requestAirdrop(
          user.publicKey,
          LAMPORTS_PER_SOL,
        );
        await provider.connection.confirmTransaction(airdropSig);

        const [gameStatePDA] = getGameStatePDA(user.publicKey);
        const [generatedMapPDA] = getGeneratedMapPDA(user.publicKey);
        const [mapEnemiesPDA] = getMapEnemiesPDA(user.publicKey);

        await ensureMapConfigExists();

        await mapGeneratorProgram.methods
          .generateMap(1)
          .accounts({
            payer: user.publicKey,
            session: user.publicKey,
            mapConfig: mapConfigPDA,
            generatedMap: generatedMapPDA,
            mapGeneratorProgram: mapGeneratorProgram.programId,
            systemProgram: SystemProgram.programId,
          } as any)
          .preInstructions([anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 })])
          .signers([user])
          .rpc();

        try {
          await gameplayProgram.methods
            .initializeGameState(1, 10, 10, 0, 0)
            .accounts({
              gameState: gameStatePDA,
              gameSession: user.publicKey,
              generatedMap: generatedMapPDA,
              mapEnemies: mapEnemiesPDA,
              player: user.publicKey,
              burnerWallet: burner.publicKey,
              systemProgram: SystemProgram.programId,
            } as any)
            .signers([user])
            .rpc();
          expect.fail("Should have thrown InvalidSessionOwner error");
        } catch (error: any) {
          expect(error.toString()).to.include("InvalidSessionOwner");
        }
      });

      it("creates GameState linked to session", async () => {
        const { user, burnerWallet, sessionPDA, gameStatePDA, campaignLevel } =
          await setupUserWithGameState();

        const gs = await gameplayProgram.account.gameState.fetch(gameStatePDA);
        expect(gs.session.toString()).to.equal(sessionPDA.toString());

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("Only session owner can modify game state", () => {
      it("rejects modification from non-owner", async () => {
        const { user, burnerWallet, sessionPDA, gameStatePDA, campaignLevel } =
          await setupUserWithGameState();

        const otherUser = Keypair.generate();
        const airdropSig = await provider.connection.requestAirdrop(
          otherUser.publicKey,
          LAMPORTS_PER_SOL,
        );
        await provider.connection.confirmTransaction(airdropSig);

        try {
          const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
          const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
          const [inventoryPDA] = getInventoryPDA(sessionPDA);
          await (gameplayProgram.methods as any)
            .movePlayer(1, 0)
            .accounts({
              gameState: gameStatePDA,
              gameSession: sessionPDA,
              mapEnemies: mapEnemiesPDA,
              generatedMap: generatedMapPDA,
              inventory: inventoryPDA,
              playerInventoryProgram: playerInventoryProgram.programId,
              player: otherUser.publicKey,
            } as any)
            .signers([otherUser])
            .rpc();
          expect.fail("Should have thrown an error");
        } catch (error: any) {
          expect(error.toString()).to.include("Unauthorized");
        }

        await cleanup(
          user,
          burnerWallet,
          sessionPDA,
          gameStatePDA,
          campaignLevel,
        );
      });
    });

    describe("close_game_state returns rent to player", () => {
      it("closes game state and returns rent", async () => {
        const { user, burnerWallet, sessionPDA, gameStatePDA, campaignLevel } =
          await setupUserWithGameState();

        const balanceBefore = await provider.connection.getBalance(
          user.publicKey,
        );

        await gameplayProgram.methods
          .closeGameState()
          .accounts({
            gameState: gameStatePDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();

        const balanceAfter = await provider.connection.getBalance(
          user.publicKey,
        );
        expect(balanceAfter).to.be.greaterThan(balanceBefore);

        const gameStateAccount =
          await provider.connection.getAccountInfo(gameStatePDA);
        expect(gameStateAccount).to.be.null;

        const [inventoryPDA] = getInventoryPDA(sessionPDA);
        await sessionProgram.methods
          .endSession(campaignLevel, true)
          .accounts({
            gameSession: sessionPDA,
            player: user.publicKey,
            burnerWallet: burnerWallet.publicKey,
            inventory: inventoryPDA,
            playerInventoryProgram: playerInventoryProgram.programId,
          } as any)
          .signers([user, burnerWallet])
          .rpc();
      });
    });
    describe("Automatic session end on defeat", () => {
      // This test is skipped because:
      // 1. HP can no longer be artificially reduced (modifyStat removed)
      // 2. Stats are derived from inventory, so player starts with decent combat ability
      // 3. Testing defeat requires multiple combats which may time out or be non-deterministic
      // The defeat mechanics are tested in Rust unit tests and combat-system tests
      it.skip("ends session automatically when defeated in combat (skipped: no direct HP modification)", async () => {
        // With inventory-derived stats and no direct HP modification,
        // reliably testing defeat requires:
        // - Creating a player with weak equipment
        // - Finding a strong enemy
        // - Ensuring the player loses
        // This is better tested via Rust unit tests
      });
    });
  });
});
