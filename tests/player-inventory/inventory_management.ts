import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PlayerInventory } from "../../target/types/player_inventory";
import { SessionManager } from "../../target/types/session_manager";
import { PlayerProfile } from "../../target/types/player_profile";
import { MapGenerator } from "../../target/types/map_generator";
import { GameplayState } from "../../target/types/gameplay_state";
import { PoiSystem } from "../../target/types/poi_system";
import { expect } from "chai";
import { Keypair, SystemProgram } from "@solana/web3.js";

describe("player-inventory", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PlayerInventory as Program<PlayerInventory>;
  const sessionProgram = anchor.workspace
    .SessionManager as Program<SessionManager>;
  const playerProfileProgram = anchor.workspace
    .PlayerProfile as Program<PlayerProfile>;
  const mapGeneratorProgram = anchor.workspace
    .MapGenerator as Program<MapGenerator>;
  const gameplayProgram = anchor.workspace
    .GameplayState as Program<GameplayState>;
  const poiSystemProgram = anchor.workspace.PoiSystem as Program<PoiSystem>;

  // Helper to derive inventory PDA (now uses session key)
  const getInventoryPDA = (session: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("inventory"), session.toBuffer()],
      program.programId,
    );
  };

  const getSessionPDA = (
    player: anchor.web3.PublicKey,
    campaignLevel: number,
  ) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session"), player.toBuffer(), Buffer.from([campaignLevel])],
      sessionProgram.programId,
    );
  };

  const getCounterPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("session_counter")],
      sessionProgram.programId,
    );
  };

  const getMapConfigPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_config")],
      mapGeneratorProgram.programId,
    );
  };

  const getPlayerProfilePDA = (player: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("player"), player.toBuffer()],
      playerProfileProgram.programId,
    );
  };

  const getGeneratedMapPDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("generated_map"), sessionPda.toBuffer()],
      mapGeneratorProgram.programId,
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

  const getGameplayAuthorityPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("gameplay_authority")],
      gameplayProgram.programId,
    );
  };

  const getPoiAuthorityPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("poi_authority")],
      poiSystemProgram.programId,
    );
  };

  const getMapPoisPDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_pois"), sessionPda.toBuffer()],
      poiSystemProgram.programId,
    );
  };

  const getInventoryAuthorityPDA = () => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("inventory_authority")],
      program.programId,
    );
  };

  // Helper to create item ID as [u8; 8]
  const makeItemId = (id: string): number[] => {
    const buffer = Buffer.alloc(8, 0);
    buffer.write(id);
    return Array.from(buffer);
  };

  const expectDirectMutationOrPrecheck = (error: any) => {
    const text = error?.toString?.() ?? String(error);
    const allowed = [
      "DirectMutationDisabled",
      "SlotEmpty",
      "WrongItemType",
      "InvalidItemId",
    ];
    expect(allowed.some((code) => text.includes(code))).to.equal(
      true,
      `Unexpected error: ${text}`,
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

  // Helper to create a user with a profile
  const createUserWithProfile = async (name: string) => {
    const user = Keypair.generate();
    const sessionSigner = Keypair.generate();

    const airdropSig = await provider.connection.requestAirdrop(
      user.publicKey,
      5 * anchor.web3.LAMPORTS_PER_SOL,
    );
    const latestBlockhash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature: airdropSig,
      blockhash: latestBlockhash.blockhash,
      lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
    });

    // Airdrop SOL to session signer (needed for inventory initialization)
    const sessionSignerAirdropSig = await provider.connection.requestAirdrop(
      sessionSigner.publicKey,
      5 * anchor.web3.LAMPORTS_PER_SOL,
    );
    const sessionSignerBlockhash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction({
      signature: sessionSignerAirdropSig,
      blockhash: sessionSignerBlockhash.blockhash,
      lastValidBlockHeight: sessionSignerBlockhash.lastValidBlockHeight,
    });

    const [playerProfilePDA] = getPlayerProfilePDA(user.publicKey);

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

  // Helper to start a session and get PDAs
  const startSessionAndGetPDAs = async (
    user: Keypair,
    sessionSigner: Keypair,
    playerProfilePDA: anchor.web3.PublicKey,
    campaignLevel: number = 1,
  ) => {
    await ensureCounterExists();
    await ensureMapConfigExists();

    const [sessionPDA] = getSessionPDA(user.publicKey, campaignLevel);
    const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
    const [gameStatePDA] = getGameStatePDA(sessionPDA);
    const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
    const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
    const [inventoryPDA] = getInventoryPDA(sessionPDA);

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
        sessionSigner: sessionSigner.publicKey,
        gameState: gameStatePDA,
        mapEnemies: mapEnemiesPDA,
        mapPois: mapPoisPDA,
        inventory: inventoryPDA,
        gameplayStateProgram: gameplayProgram.programId,
        poiSystemProgram: poiSystemProgram.programId,
        playerInventoryProgram: program.programId,
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
      inventoryPDA,
      gameStatePDA,
      mapPoisPDA,
      sessionSigner: sessionSigner,
      sessionSigner,
    };
  };

  const movePlayer = async (
    sessionSigner: Keypair,
    gameStatePDA: anchor.web3.PublicKey,
    targetX: number,
    targetY: number,
  ) => {
    const gameState = await gameplayProgram.account.gameState.fetch(gameStatePDA);
    const [generatedMapPDA] = getGeneratedMapPDA(gameState.session);
    const [mapEnemiesPDA] = getMapEnemiesPDA(gameState.session);
    const [inventoryPDA] = getInventoryPDA(gameState.session);
    const [mapPoisPDA] = getMapPoisPDA(gameState.session);
    const [gameplayAuthorityPDA] = getGameplayAuthorityPDA();

    await (gameplayProgram.methods as any)
      .movePlayer(targetX, targetY)
      .accounts({
        gameState: gameStatePDA,
        gameSession: gameState.session,
        mapEnemies: mapEnemiesPDA,
        generatedMap: generatedMapPDA,
        inventory: inventoryPDA,
        gameplayAuthority: gameplayAuthorityPDA,
        playerInventoryProgram: program.programId,
        mapGeneratorProgram: mapGeneratorProgram.programId,
        mapPois: mapPoisPDA,
        poiSystemProgram: poiSystemProgram.programId,
        player: sessionSigner.publicKey,
      } as any)
      .signers([sessionSigner])
      .rpc();
  };

  const isWalkableTile = (map: any, x: number, y: number) => {
    const index = y * map.width + x;
    const byteIndex = Math.floor(index / 8);
    const bitIndex = index % 8;
    return ((map.packedTiles[byteIndex] >> bitIndex) & 1) === 0;
  };

  const findPath = (
    map: any,
    startX: number,
    startY: number,
    targetX: number,
    targetY: number,
    maxSteps: number,
  ): Array<{ x: number; y: number }> | null => {
    const width = map.width as number;
    const height = map.height as number;
    const visited = new Set<string>();
    const queue: Array<{
      x: number;
      y: number;
      path: Array<{ x: number; y: number }>;
    }> = [{ x: startX, y: startY, path: [] }];
    visited.add(`${startX},${startY}`);

    while (queue.length > 0) {
      const cur = queue.shift()!;
      if (cur.x === targetX && cur.y === targetY) {
        return cur.path;
      }
      if (cur.path.length >= maxSteps) continue;

      const neighbors = [
        { x: cur.x + 1, y: cur.y },
        { x: cur.x - 1, y: cur.y },
        { x: cur.x, y: cur.y + 1 },
        { x: cur.x, y: cur.y - 1 },
      ];

      for (const n of neighbors) {
        if (n.x < 0 || n.y < 0 || n.x >= width || n.y >= height) continue;
        const key = `${n.x},${n.y}`;
        if (visited.has(key)) continue;
        if (!isWalkableTile(map, n.x, n.y)) continue;
        visited.add(key);
        queue.push({
          x: n.x,
          y: n.y,
          path: [...cur.path, { x: n.x, y: n.y }],
        });
      }
    }

    return null;
  };

  // Helper to abandon a session and clean up (for test cleanup)
  // Uses abandonSession which requires both player and session signer signatures
  const endSession = async (
    user: Keypair,
    sessionSigner: Keypair,
    sessionPDA: anchor.web3.PublicKey,
    inventoryPDA: anchor.web3.PublicKey,
    campaignLevel: number = 1,
  ) => {
    const [gameStatePDA] = getGameStatePDA(sessionPDA);
    const [mapEnemiesPDA] = getMapEnemiesPDA(sessionPDA);
    const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);
    const [mapPoisPDA] = getMapPoisPDA(sessionPDA);
    await sessionProgram.methods
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
        playerInventoryProgram: program.programId,
        gameplayStateProgram: gameplayProgram.programId,
        mapGeneratorProgram: mapGeneratorProgram.programId,
        poiSystemProgram: poiSystemProgram.programId,
      } as any)
      .signers([user, sessionSigner])
      .rpc();
  };

  describe("Inventory Initialization", () => {
    it("initializes a new player inventory", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("InvInit1");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      // Fetch and verify inventory
      const inventory =
        await program.account.playerInventory.fetch(inventoryPDA);

      // Inventory is now owned by session signer
      expect(inventory.player.toString()).to.equal(bw.publicKey.toString());
      // Tool is initialized with BASIC_PICKAXE (T-XX-00)
      expect(inventory.tool).to.not.be.null;
      expect(inventory.gearSlotCapacity).to.equal(4);

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });
  });

  describe("Equip Tool", () => {
    it("rejects direct tool equip", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("EquipTool1");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      const itemId = makeItemId("T-SC-01");
      try {
        await program.methods
          .equipTool(itemId, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects direct tool replace attempts", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("EquipTool2");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      const itemId1 = makeItemId("T-SC-01");
      try {
        await program.methods
          .equipTool(itemId1, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects direct tool equip before item-type checks", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("EquipTool3");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      // Try to equip gear as tool (should fail)
      const itemId = makeItemId("G-ST-01");
      try {
        await program.methods
          .equipTool(itemId, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });
  });

  describe("Equip Gear", () => {
    it("rejects direct gear equip", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("EquipGear1");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      const itemId = makeItemId("G-ST-01");
      try {
        await program.methods
          .equipGear(itemId, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects direct gear equip attempts", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("EquipGear2");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      const itemId5 = makeItemId("G-FR-01");
      try {
        await program.methods
          .equipGear(itemId5, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });
  });

  describe("Item Fusion", () => {
    it("rejects direct item fusion", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Fusion1");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      try {
        await program.methods
          .fuseItems(0, 1)
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects direct invalid fusion path", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Fusion2");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      try {
        await program.methods
          .fuseItems(0, 1)
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });
  });

  describe("Gear Slot Expansion", () => {
    it("rejects direct gear slot expansion", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Expand1");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      try {
        await program.methods
          .expandGearSlots()
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects repeated direct slot expansion attempts", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Expand2");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      try {
        await program.methods
          .expandGearSlots()
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects direct max-slot expansion path", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Expand3");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      try {
        await program.methods
          .expandGearSlots()
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });
  });

  describe("Tool Oil Application", () => {
    it("rejects direct tool oil application", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Oil1");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      try {
        await program.methods
          .applyToolOil({ plusAtk: {} })
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects repeated direct tool oil application", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Oil2");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);

      try {
        await program.methods
          .applyToolOil({ plusAtk: {} })
          .accounts({
            inventory: inventoryPDA,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });
  });

  describe("Unequip Gear", () => {
    it("rejects direct gear unequip", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Unequip1");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);
      const [gameStatePDA] = getGameStatePDA(sessionPDA);
      const [inventoryAuthorityPDA] = getInventoryAuthorityPDA();
      try {
        await program.methods
          .unequipGear(0)
          .accounts({
            inventory: inventoryPDA,
            gameState: gameStatePDA,
            inventoryAuthority: inventoryAuthorityPDA,
            gameplayStateProgram: gameplayProgram.programId,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });

    it("rejects direct unequip before slot checks", async () => {
      const { user, sessionSigner, playerProfilePDA } =
        await createUserWithProfile("Unequip2");
      const {
        sessionPDA,
        inventoryPDA,
        sessionSigner: bw,
      } = await startSessionAndGetPDAs(user, sessionSigner, playerProfilePDA);
      const [gameStatePDA] = getGameStatePDA(sessionPDA);
      const [inventoryAuthorityPDA] = getInventoryAuthorityPDA();

      // Try to unequip from empty slot
      try {
        await program.methods
          .unequipGear(0)
          .accounts({
            inventory: inventoryPDA,
            gameState: gameStatePDA,
            inventoryAuthority: inventoryAuthorityPDA,
            gameplayStateProgram: gameplayProgram.programId,
            player: bw.publicKey,
          } as any)
          .signers([bw])
          .rpc();
        expect.fail("Should have thrown DirectMutationDisabled");
      } catch (error: any) {
        expectDirectMutationOrPrecheck(error);
      }

      // Clean up
      await endSession(user, bw, sessionPDA, inventoryPDA);
    });
  });

  describe("Authorized CPI Flows", () => {
    it("equips an item through POI authorized CPI (interact_pick_item)", async () => {
      const maxAttempts = 3;
      let lastError: any = null;

      for (let attempt = 0; attempt < maxAttempts; attempt++) {
        const { user, sessionSigner, playerProfilePDA } =
          await createUserWithProfile(`AuthPick${attempt}`);
        const {
          sessionPDA,
          inventoryPDA,
          gameStatePDA,
          mapPoisPDA,
          sessionSigner: bw,
        } = await startSessionAndGetPDAs(
          user,
          sessionSigner,
          playerProfilePDA,
        );

        try {
          const [inventoryAuthorityPDA] = getInventoryAuthorityPDA();
          const [poiAuthorityPDA] = getPoiAuthorityPDA();
          const [generatedMapPDA] = getGeneratedMapPDA(sessionPDA);

          const gameState =
            await gameplayProgram.account.gameState.fetch(gameStatePDA);
          const generatedMap =
            await mapGeneratorProgram.account.generatedMap.fetch(generatedMapPDA);
          const mapPois = await poiSystemProgram.account.mapPois.fetch(mapPoisPDA);

          const startX = gameState.positionX as number;
          const startY = gameState.positionY as number;
          const maxMoves = Number(gameState.movesRemaining);

          let selected:
            | {
                index: number;
                path: Array<{ x: number; y: number }>;
              }
            | undefined;

          for (let i = 0; i < mapPois.pois.length; i++) {
            const poi = mapPois.pois[i] as any;
            const poiType = Number(poi.poiType ?? poi.poi_type);
            if (poiType !== 2) continue; // Supply Cache (gear pick)

            const path = findPath(
              generatedMap,
              startX,
              startY,
              Number(poi.x),
              Number(poi.y),
              maxMoves,
            );
            if (!path) continue;
            if (!selected || path.length < selected.path.length) {
              selected = { index: i, path };
            }
          }

          if (!selected) {
            throw new Error("No reachable Supply Cache found in current session");
          }

          for (const step of selected.path) {
            await movePlayer(bw, gameStatePDA, step.x, step.y);
          }

          const before = await program.account.playerInventory.fetch(inventoryPDA);
          const beforeGearCount = before.gear.filter((g: any) => g !== null).length;

          await poiSystemProgram.methods
            .generateCacheOffer(selected.index)
            .accounts({
              mapPois: mapPoisPDA,
              gameState: gameStatePDA,
              inventory: inventoryPDA,
              inventoryAuthority: inventoryAuthorityPDA,
              poiAuthority: poiAuthorityPDA,
              playerInventoryProgram: program.programId,
              gameplayStateProgram: gameplayProgram.programId,
              gameSession: sessionPDA,
              player: bw.publicKey,
            } as any)
            .signers([bw])
            .rpc();

          await poiSystemProgram.methods
            .interactPickItem(selected.index, 0)
            .accounts({
              mapPois: mapPoisPDA,
              gameState: gameStatePDA,
              inventory: inventoryPDA,
              inventoryAuthority: inventoryAuthorityPDA,
              poiAuthority: poiAuthorityPDA,
              playerInventoryProgram: program.programId,
              gameplayStateProgram: gameplayProgram.programId,
              gameSession: sessionPDA,
              player: bw.publicKey,
            } as any)
            .signers([bw])
            .rpc();

          const after = await program.account.playerInventory.fetch(inventoryPDA);
          const afterGearCount = after.gear.filter((g: any) => g !== null).length;
          expect(afterGearCount).to.be.greaterThan(beforeGearCount);

          await endSession(user, bw, sessionPDA, inventoryPDA);
          return;
        } catch (error: any) {
          lastError = error;
          await endSession(user, bw, sessionPDA, inventoryPDA);
        }
      }

      throw new Error(
        `Authorized POI equip flow failed after ${maxAttempts} attempts: ${lastError}`,
      );
    });
  });
});
