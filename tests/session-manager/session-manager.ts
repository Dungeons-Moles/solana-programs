import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SessionManager } from "../../target/types/session_manager";
import { MapGenerator } from "../../target/types/map_generator";
import { PlayerProfile } from "../../target/types/player_profile";
import { GameplayState } from "../../target/types/gameplay_state";
import { PoiSystem } from "../../target/types/poi_system";
import { PlayerInventory } from "../../target/types/player_inventory";
import { expect } from "chai";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram } from "@solana/web3.js";

describe("session-manager", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SessionManager as Program<SessionManager>;
  const playerProfileProgram = anchor.workspace
    .PlayerProfile as Program<PlayerProfile>;
  const gameplayProgram = anchor.workspace
    .GameplayState as Program<GameplayState>;
  const poiSystemProgram = anchor.workspace.PoiSystem as Program<PoiSystem>;
  const playerInventoryProgram = anchor.workspace
    .PlayerInventory as Program<PlayerInventory>;
  const mapGeneratorProgram = anchor.workspace
    .MapGenerator as Program<MapGenerator>;

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

  const getDuelSessionPDA = (player: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("duel_session"), player.toBuffer()],
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

  const getGameStatePDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("game_state"), sessionPda.toBuffer()],
      gameplayProgram.programId,
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

  const getSessionManagerAuthorityPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session_manager_authority")],
      program.programId,
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

  // Helper to create user with profile
  const createUserWithProfile = async (name: string = "TestPlayer") => {
    const user = Keypair.generate();
    const sessionSigner = Keypair.generate();

    // Airdrop SOL
    const airdropSig = await provider.connection.requestAirdrop(
      user.publicKey,
      2 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(airdropSig);

    const sessionSignerAirdropSig = await provider.connection.requestAirdrop(
      sessionSigner.publicKey,
      2 * LAMPORTS_PER_SOL,
    );
    await provider.connection.confirmTransaction(sessionSignerAirdropSig);

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

    return { user, sessionSigner, playerProfilePDA };
  };

  const startSession = async (params: {
    user: Keypair;
    sessionSigner: Keypair;
    playerProfilePDA: anchor.web3.PublicKey;
    campaignLevel: number;
  }) => {
    const { user, sessionSigner, playerProfilePDA, campaignLevel } = params;

    await ensureMapConfigExists();

    const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);
    const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
    const [gameStatePDA] = getGameStatePDA(sessionPDA);
    const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
    const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
    const [inventoryPDA] = getInventoryPDA(sessionPDA);

    await (program.methods as any)
      .startSession(campaignLevel)
      .accounts({
        gameSession: sessionPDA,
        sessionCounter: counterPDA,
        playerProfile: playerProfilePDA,
        mapConfig: mapConfigPDA,
        generatedMap: generatedMapPDA,
        mapGeneratorProgram: mapGeneratorProgram.programId,
        player: user.publicKey,
        sessionSigner: sessionSigner.publicKey,
        gameState: gameStatePDA,
        mapEnemies: mapEnemiesPDA,
        mapPois: mapPoisPDA,
        inventory: inventoryPDA,
        gameplayStateProgram: gameplayProgram.programId,
        poiSystemProgram: poiSystemProgram.programId,
        playerInventoryProgram: playerInventoryProgram.programId,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
          units: 1400000,
        }),
        anchor.web3.ComputeBudgetProgram.requestHeapFrame({
          bytes: 256 * 1024,
        }),
      ])
      .signers([user, sessionSigner])
      .rpc();

    return {
      sessionPDA,
      gameStatePDA,
      mapEnemiesPDA,
      mapPoisPDA,
      inventoryPDA,
    };
  };

  const startDuelSession = async (params: {
    user: Keypair;
    sessionSigner: Keypair;
    playerProfilePDA: anchor.web3.PublicKey;
  }) => {
    const { user, sessionSigner, playerProfilePDA } = params;
    await ensureMapConfigExists();

    const [sessionPDA] = getDuelSessionPDA(user.publicKey);
    const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
    const [gameStatePDA] = getGameStatePDA(sessionPDA);
    const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
    const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
    const [inventoryPDA] = getInventoryPDA(sessionPDA);
    const [sessionManagerAuthorityPDA] = getSessionManagerAuthorityPDA();

    await (program.methods as any)
      .startDuelSession()
      .accounts({
        gameSession: sessionPDA,
        sessionCounter: counterPDA,
        playerProfile: playerProfilePDA,
        mapConfig: mapConfigPDA,
        generatedMap: generatedMapPDA,
        mapGeneratorProgram: mapGeneratorProgram.programId,
        player: user.publicKey,
        sessionSigner: sessionSigner.publicKey,
        sessionManagerAuthority: sessionManagerAuthorityPDA,
        gameState: gameStatePDA,
        mapEnemies: mapEnemiesPDA,
        mapPois: mapPoisPDA,
        inventory: inventoryPDA,
        gameplayStateProgram: gameplayProgram.programId,
        poiSystemProgram: poiSystemProgram.programId,
        playerInventoryProgram: playerInventoryProgram.programId,
        systemProgram: SystemProgram.programId,
      } as any)
      .preInstructions([
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
          units: 1_400_000,
        }),
        anchor.web3.ComputeBudgetProgram.requestHeapFrame({
          bytes: 256 * 1024,
        }),
      ])
      .signers([user, sessionSigner])
      .rpc();
  };

  describe("Initialize Session Counter", () => {
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

  describe("Start New Game Session", () => {
    it("starts new game session", async () => {
      await ensureCounterExists();

      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("SessionTest1");
      const campaignLevel = 1; // Player starts with level 1 unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      await startSession({
        user,
        sessionSigner,
        playerProfilePDA,
        campaignLevel,
      });

      const session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.player.toString()).to.equal(user.publicKey.toString());
      expect(session.campaignLevel).to.equal(campaignLevel);
      expect(session.isDelegated).to.equal(false);
      expect(session.sessionId.toNumber()).to.be.greaterThan(0);
      expect(session.startedAt.toNumber()).to.be.greaterThan(0);
      expect(session.lastActivity.toNumber()).to.be.greaterThan(0);

      // Clean up: end the session (also closes all sub-accounts via CPI)
      const [inventoryPDA] = getInventoryPDA(sessionPDA);
      const [gameStatePDA] = getGameStatePDA(sessionPDA);
      const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
      const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
      const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
      await program.methods
        .abandonSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          gameState: gameStatePDA,
          mapEnemies: mapEnemiesPDA,
          generatedMap: generatedMapPDA,
          mapPois: mapPoisPDA,
          player: user.publicKey,
          sessionSigner: sessionSigner.publicKey,
          inventory: inventoryPDA,
          playerInventoryProgram: playerInventoryProgram.programId,
          gameplayStateProgram: gameplayProgram.programId,
          mapGeneratorProgram: mapGeneratorProgram.programId,
          poiSystemProgram: poiSystemProgram.programId,
        } as any)
        .signers([user, sessionSigner])
        .rpc();
    });

    it("initializes bundled accounts", async () => {
      await ensureCounterExists();

      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("SessionBundleInit");
      const campaignLevel = 1;

      const {
        sessionPDA,
        gameStatePDA,
        mapEnemiesPDA,
        mapPoisPDA,
        inventoryPDA,
      } = await startSession({
        user,
        sessionSigner,
        playerProfilePDA,
        campaignLevel,
      });

      const gameState =
        await gameplayProgram.account.gameState.fetch(gameStatePDA);
      expect(gameState.session.toString()).to.equal(sessionPDA.toString());

      const mapEnemies =
        await gameplayProgram.account.mapEnemies.fetch(mapEnemiesPDA);
      expect(mapEnemies.session.toString()).to.equal(sessionPDA.toString());

      const mapPois = await poiSystemProgram.account.mapPois.fetch(mapPoisPDA);
      expect(mapPois.session.toString()).to.equal(sessionPDA.toString());

      const inventory =
        await playerInventoryProgram.account.playerInventory.fetch(
          inventoryPDA,
        );
      expect(inventory.player.toString()).to.equal(
        sessionSigner.publicKey.toString(),
      );

      // abandonSession now closes all sub-accounts (game_state, map_enemies, generated_map, map_pois, inventory)
      const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
      await program.methods
        .abandonSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          gameState: gameStatePDA,
          mapEnemies: mapEnemiesPDA,
          generatedMap: generatedMapPDA,
          mapPois: mapPoisPDA,
          player: user.publicKey,
          sessionSigner: sessionSigner.publicKey,
          inventory: inventoryPDA,
          playerInventoryProgram: playerInventoryProgram.programId,
          gameplayStateProgram: gameplayProgram.programId,
          mapGeneratorProgram: mapGeneratorProgram.programId,
          poiSystemProgram: poiSystemProgram.programId,
        } as any)
        .signers([user, sessionSigner])
        .rpc();
    });
  });

  describe("Reject Second Session for Same Player at Same Level", () => {
    it("rejects second session for same player at same level", async () => {
      await ensureCounterExists();

      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("SessionTest2");
      const campaignLevel = 1; // Must be >= 1 and <= highest_level_unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // First session should succeed
      await startSession({
        user,
        sessionSigner,
        playerProfilePDA,
        campaignLevel,
      });

      // Second session at same level should fail
      try {
        await startSession({
          user,
          sessionSigner,
          playerProfilePDA,
          campaignLevel,
        });
        expect.fail("Should have thrown an error");
      } catch (error: any) {
        // Account already initialized - this is expected
        expect(error.toString()).to.include("already in use");
      }

      // Clean up
      const [inventoryPDA] = getInventoryPDA(sessionPDA);
      const [gameStatePDA] = getGameStatePDA(sessionPDA);
      const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
      const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
      const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
      await program.methods
        .abandonSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          gameState: gameStatePDA,
          mapEnemies: mapEnemiesPDA,
          generatedMap: generatedMapPDA,
          mapPois: mapPoisPDA,
          player: user.publicKey,
          sessionSigner: sessionSigner.publicKey,
          inventory: inventoryPDA,
          playerInventoryProgram: playerInventoryProgram.programId,
          gameplayStateProgram: gameplayProgram.programId,
          mapGeneratorProgram: mapGeneratorProgram.programId,
          poiSystemProgram: poiSystemProgram.programId,
        } as any)
        .signers([user, sessionSigner])
        .rpc();
    });
  });

  describe("Delegate Session to Ephemeral Rollup", () => {
    it.skip("rejects delegation when campaign level does not match session (requires MagicBlock runtime accounts)", async () => {
      await ensureCounterExists();

      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("SessionTest3");
      const campaignLevel = 1; // Player starts with level 1 unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // Start bundled session
      await startSession({
        user,
        sessionSigner,
        playerProfilePDA,
        campaignLevel,
      });

      try {
        await program.methods
          .delegateSession(campaignLevel + 1)
          .accounts({
            gameSession: sessionPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown InvalidCampaignLevel");
      } catch (error: any) {
        expect(error.toString()).to.include("InvalidCampaignLevel");
      }

      // Clean up
      const [inventoryPDA] = getInventoryPDA(sessionPDA);
      const [gameStatePDA] = getGameStatePDA(sessionPDA);
      const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
      const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
      const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
      await program.methods
        .abandonSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          gameState: gameStatePDA,
          mapEnemies: mapEnemiesPDA,
          generatedMap: generatedMapPDA,
          mapPois: mapPoisPDA,
          player: user.publicKey,
          sessionSigner: sessionSigner.publicKey,
          inventory: inventoryPDA,
          playerInventoryProgram: playerInventoryProgram.programId,
          gameplayStateProgram: gameplayProgram.programId,
          mapGeneratorProgram: mapGeneratorProgram.programId,
          poiSystemProgram: poiSystemProgram.programId,
        } as any)
        .signers([user, sessionSigner])
        .rpc();
    });

    it.skip("delegates session to ephemeral rollup (requires MagicBlock runtime accounts)", async () => {
      await ensureCounterExists();

      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("SessionTest3Delegation");
      const campaignLevel = 1;
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      await startSession({
        user,
        sessionSigner,
        playerProfilePDA,
        campaignLevel,
      });

      await program.methods
        .delegateSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();
    });
  });

  describe("Duel Seed Security", () => {
    it("always derives duel seed on-chain", async () => {
      await ensureCounterExists();

      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("DuelSeedDerived");

      await startDuelSession({
        user,
        sessionSigner,
        playerProfilePDA,
      });

      const duelCampaignLevel = 20;
      const [sessionPDA] = getDuelSessionPDA(user.publicKey);
      const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
      const generatedMap = await mapGeneratorProgram.account.generatedMap.fetch(generatedMapPDA);
      expect(Number(generatedMap.seed)).to.be.greaterThan(0);

      const [inventoryPDA] = getInventoryPDA(sessionPDA);
      const [gameStatePDA] = getGameStatePDA(sessionPDA);
      const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
      const [mapPoisPDA] = getMapPoisPDA(sessionPDA);

      await program.methods
        .abandonSession(duelCampaignLevel)
        .accounts({
          gameSession: sessionPDA,
          gameState: gameStatePDA,
          mapEnemies: mapEnemiesPDA,
          generatedMap: generatedMapPDA,
          mapPois: mapPoisPDA,
          player: user.publicKey,
          sessionSigner: sessionSigner.publicKey,
          inventory: inventoryPDA,
          playerInventoryProgram: playerInventoryProgram.programId,
          gameplayStateProgram: gameplayProgram.programId,
          mapGeneratorProgram: mapGeneratorProgram.programId,
          poiSystemProgram: poiSystemProgram.programId,
        } as any)
        .signers([user, sessionSigner])
        .rpc();
    });
  });

  describe("End Session and Close Account", () => {
    it("ends session and closes account", async () => {
      await ensureCounterExists();

      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("SessionTest5");
      const campaignLevel = 1; // Player starts with level 1 unlocked
      const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);

      // Start bundled session
      await startSession({
        user,
        sessionSigner,
        playerProfilePDA,
        campaignLevel,
      });

      // Verify session exists
      let session = await program.account.gameSession.fetch(sessionPDA);
      expect(session.player.toString()).to.equal(user.publicKey.toString());

      // Get user balance before ending
      const balanceBefore = await provider.connection.getBalance(
        user.publicKey,
      );
      const [inventoryPDA] = getInventoryPDA(sessionPDA);
      const [gameStatePDA] = getGameStatePDA(sessionPDA);
      const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
      const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
      const [mapPoisPDA] = getMapPoisPDA(sessionPDA);

      // End session (closes all sub-accounts via CPI, returning rent)
      await program.methods
        .abandonSession(campaignLevel)
        .accounts({
          gameSession: sessionPDA,
          gameState: gameStatePDA,
          mapEnemies: mapEnemiesPDA,
          generatedMap: generatedMapPDA,
          mapPois: mapPoisPDA,
          player: user.publicKey,
          sessionSigner: sessionSigner.publicKey,
          inventory: inventoryPDA,
          playerInventoryProgram: playerInventoryProgram.programId,
          gameplayStateProgram: gameplayProgram.programId,
          mapGeneratorProgram: mapGeneratorProgram.programId,
          poiSystemProgram: poiSystemProgram.programId,
        } as any)
        .signers([user, sessionSigner])
        .rpc();

      // Verify session account is closed
      const sessionAccount =
        await provider.connection.getAccountInfo(sessionPDA);
      expect(sessionAccount).to.be.null;

      // Verify inventory account is closed
      const inventoryAccount =
        await provider.connection.getAccountInfo(inventoryPDA);
      expect(inventoryAccount).to.be.null;

      // Verify rent was returned to user (from both session and inventory)
      const balanceAfter = await provider.connection.getBalance(user.publicKey);
      expect(balanceAfter).to.be.greaterThan(balanceBefore);
    });
  });
});
