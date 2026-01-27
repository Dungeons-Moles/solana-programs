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
  const sessionProgram = anchor.workspace.SessionManager as Program<SessionManager>;
  const playerProfileProgram = anchor.workspace.PlayerProfile as Program<PlayerProfile>;
  const mapGeneratorProgram = anchor.workspace.MapGenerator as Program<MapGenerator>;
  const gameplayProgram = anchor.workspace.GameplayState as Program<GameplayState>;
  const poiSystemProgram = anchor.workspace.PoiSystem as Program<PoiSystem>;

  // Helper to derive inventory PDA (now uses session key)
  const getInventoryPDA = (session: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("inventory"), session.toBuffer()],
      program.programId,
    );
  };

  const getSessionPDA = (player: anchor.web3.PublicKey, campaignLevel: number) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("session"),
        player.toBuffer(),
        Buffer.from([campaignLevel]),
      ],
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

  const getMapPoisPDA = (sessionPda: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("map_pois"), sessionPda.toBuffer()],
      poiSystemProgram.programId,
    );
  };

  // Helper to create item ID as [u8; 8]
  const makeItemId = (id: string): number[] => {
    const buffer = Buffer.alloc(8, 0);
    buffer.write(id);
    return Array.from(buffer);
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
    const burnerWallet = Keypair.generate();

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

    return { user, burnerWallet, playerProfilePDA };
  };

  // Helper to start a session and get PDAs
  const startSessionAndGetPDAs = async (user: Keypair, burnerWallet: Keypair, playerProfilePDA: anchor.web3.PublicKey, campaignLevel: number = 1) => {
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
        burnerWallet: burnerWallet.publicKey,
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
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
        anchor.web3.ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([user])
      .rpc();

    return { sessionPDA, inventoryPDA, gameStatePDA, mapPoisPDA };
  };

  // Helper to end a session and clean up
  const endSession = async (user: Keypair, sessionPDA: anchor.web3.PublicKey, inventoryPDA: anchor.web3.PublicKey, campaignLevel: number = 1) => {
    await sessionProgram.methods
      .endSession(campaignLevel, true)
      .accounts({
        gameSession: sessionPDA,
        player: user.publicKey,
        inventory: inventoryPDA,
        playerInventoryProgram: program.programId,
      } as any)
      .signers([user])
      .rpc();
  };

  describe("Inventory Initialization", () => {
    it("initializes a new player inventory", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("InvInit1");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Fetch and verify inventory
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);

      expect(inventory.player.toString()).to.equal(user.publicKey.toString());
      // Tool is initialized with BASIC_PICKAXE (T-XX-00)
      expect(inventory.tool).to.not.be.null;
      expect(inventory.gearSlotCapacity).to.equal(4);

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });
  });

  describe("Equip Tool", () => {
    it("equips a tool in the tool slot", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("EquipTool1");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Equip Twin Picks (T-SC-01)
      const itemId = makeItemId("T-SC-01");
      await program.methods
        .equipTool(itemId, { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify tool is equipped
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(inventory.tool).to.not.be.null;
      expect(Array.from(inventory.tool!.itemId)).to.deep.equal(itemId);

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("replaces existing tool when equipping new one", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("EquipTool2");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Equip first tool
      const itemId1 = makeItemId("T-SC-01");
      await program.methods
        .equipTool(itemId1, { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Equip second tool (should replace)
      const itemId2 = makeItemId("T-FR-01");
      await program.methods
        .equipTool(itemId2, { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify new tool is equipped
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(Array.from(inventory.tool!.itemId)).to.deep.equal(itemId2);

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("fails to equip gear in tool slot", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("EquipTool3");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Try to equip gear as tool (should fail)
      const itemId = makeItemId("G-ST-01");
      try {
        await program.methods
          .equipTool(itemId, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown WrongItemType error");
      } catch (error: any) {
        expect(error.toString()).to.include("WrongItemType");
      }

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });
  });

  describe("Equip Gear", () => {
    it("equips gear in available slot", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("EquipGear1");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Equip Miner Helmet (G-ST-01)
      const itemId = makeItemId("G-ST-01");
      await program.methods
        .equipGear(itemId, { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify gear is equipped
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(inventory.gear[0]).to.not.be.null;
      expect(Array.from(inventory.gear[0]!.itemId)).to.deep.equal(itemId);

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("fails when all gear slots are full", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("EquipGear2");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Equip 4 gear items (fills initial slots)
      const gearItems = ["G-ST-01", "G-ST-02", "G-SC-01", "G-SC-02"];
      for (const itemStr of gearItems) {
        const itemId = makeItemId(itemStr);
        await program.methods
          .equipGear(itemId, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
      }

      // Try to equip 5th item (should fail)
      const itemId5 = makeItemId("G-FR-01");
      try {
        await program.methods
          .equipGear(itemId5, { i: {} })
          .accounts({
            inventory: inventoryPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown InventoryFull error");
      } catch (error: any) {
        expect(error.toString()).to.include("InventoryFull");
      }

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });
  });

  describe("Item Fusion", () => {
    it("fuses two identical Tier I items to Tier II", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Fusion1");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Equip two identical gear items
      const itemId = makeItemId("G-ST-01");
      await program.methods
        .equipGear(itemId, { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      await program.methods
        .equipGear(itemId, { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Fuse items in slots 0 and 1
      await program.methods
        .fuseItems(0, 1)
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify fusion result
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(inventory.gear[0]).to.not.be.null;
      expect(inventory.gear[0]!.tier).to.deep.equal({ ii: {} });
      expect(inventory.gear[1]).to.be.null; // Second slot should be empty

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("fails to fuse different items", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Fusion2");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Equip two different items
      await program.methods
        .equipGear(makeItemId("G-ST-01"), { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      await program.methods
        .equipGear(makeItemId("G-ST-02"), { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Try to fuse (should fail)
      try {
        await program.methods
          .fuseItems(0, 1)
          .accounts({
            inventory: inventoryPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown FusionMismatch error");
      } catch (error: any) {
        expect(error.toString()).to.include("FusionMismatch");
      }

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });
  });

  describe("Gear Slot Expansion", () => {
    it("expands gear slots from 4 to 6", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Expand1");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Expand slots
      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify expansion
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(inventory.gearSlotCapacity).to.equal(6);

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("expands gear slots from 6 to 8", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Expand2");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Expand twice
      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify expansion
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(inventory.gearSlotCapacity).to.equal(8);

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("fails to expand beyond max slots", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Expand3");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Expand to max
      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Try to expand again (should fail)
      try {
        await program.methods
          .expandGearSlots()
          .accounts({
            inventory: inventoryPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown AlreadyMaxSlots error");
      } catch (error: any) {
        expect(error.toString()).to.include("AlreadyMaxSlots");
      }

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });
  });

  describe("Tool Oil Application", () => {
    it("applies Tool Oil to equipped tool", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Oil1");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Equip tool first
      await program.methods
        .equipTool(makeItemId("T-SC-01"), { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Apply Tool Oil
      await program.methods
        .applyToolOil({ plusAtk: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify oil is applied
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(inventory.tool!.toolOilFlags & 0x01).to.equal(1); // +ATK flag

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("fails to apply same oil twice", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Oil2");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      await program.methods
        .equipTool(makeItemId("T-SC-01"), { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      await program.methods
        .applyToolOil({ plusAtk: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Try to apply same oil again (should fail)
      try {
        await program.methods
          .applyToolOil({ plusAtk: {} })
          .accounts({
            inventory: inventoryPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown ToolOilAlreadyApplied error");
      } catch (error: any) {
        expect(error.toString()).to.include("ToolOilAlreadyApplied");
      }

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });
  });

  describe("Unequip Gear", () => {
    it("unequips gear from slot", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Unequip1");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      await program.methods
        .equipGear(makeItemId("G-ST-01"), { i: {} })
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Unequip
      await program.methods
        .unequipGear(0)
        .accounts({
          inventory: inventoryPDA,
          player: user.publicKey,
        } as any)
        .signers([user])
        .rpc();

      // Verify slot is empty
      const inventory = await program.account.playerInventory.fetch(inventoryPDA);
      expect(inventory.gear[0]).to.be.null;

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });

    it("fails to unequip from empty slot", async () => {
      const { user, burnerWallet, playerProfilePDA } = await createUserWithProfile("Unequip2");
      const { sessionPDA, inventoryPDA } = await startSessionAndGetPDAs(user, burnerWallet, playerProfilePDA);

      // Try to unequip from empty slot
      try {
        await program.methods
          .unequipGear(0)
          .accounts({
            inventory: inventoryPDA,
            player: user.publicKey,
          } as any)
          .signers([user])
          .rpc();
        expect.fail("Should have thrown SlotEmpty error");
      } catch (error: any) {
        expect(error.toString()).to.include("SlotEmpty");
      }

      // Clean up
      await endSession(user, sessionPDA, inventoryPDA);
    });
  });
});
