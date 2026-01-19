import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PlayerInventory } from "../../target/types/player_inventory";
import { expect } from "chai";
import { Keypair, SystemProgram } from "@solana/web3.js";

describe("player-inventory", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.PlayerInventory as Program<PlayerInventory>;

  // Helper to derive inventory PDA
  const getInventoryPDA = (player: anchor.web3.PublicKey) => {
    return anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("inventory"), player.toBuffer()],
      program.programId,
    );
  };

  // Helper to create item ID as [u8; 8]
  const makeItemId = (id: string): number[] => {
    const buffer = Buffer.alloc(8, 0);
    buffer.write(id);
    return Array.from(buffer);
  };

  describe("Inventory Initialization", () => {
    it("initializes a new player inventory", async () => {
      const player = Keypair.generate();

      // Airdrop SOL to player
      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Fetch and verify inventory
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);

      expect(inventory.player.toString()).to.equal(player.publicKey.toString());
      expect(inventory.tool).to.be.null;
      expect(inventory.gearSlotCapacity).to.equal(4);
    });
  });

  describe("Equip Tool", () => {
    it("equips a tool in the tool slot", async () => {
      const player = Keypair.generate();

      // Airdrop SOL
      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      // Initialize inventory
      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Equip Twin Picks (T-SC-01)
      const itemId = makeItemId("T-SC-01");
      await program.methods
        .equipTool(itemId, { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify tool is equipped
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(inventory.tool).to.not.be.null;
      expect(Array.from(inventory.tool!.itemId)).to.deep.equal(itemId);
    });

    it("replaces existing tool when equipping new one", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Equip first tool
      const itemId1 = makeItemId("T-SC-01");
      await program.methods
        .equipTool(itemId1, { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Equip second tool (should replace)
      const itemId2 = makeItemId("T-FR-01");
      await program.methods
        .equipTool(itemId2, { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify new tool is equipped
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(Array.from(inventory.tool!.itemId)).to.deep.equal(itemId2);
    });

    it("fails to equip gear in tool slot", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Try to equip gear as tool (should fail)
      const itemId = makeItemId("G-ST-01");
      try {
        await program.methods
          .equipTool(itemId, { i: {} })
          .accounts({
            inventory: inventoryPda,
            player: player.publicKey,
          } as any)
          .signers([player])
          .rpc();
        expect.fail("Should have thrown WrongItemType error");
      } catch (error: any) {
        expect(error.toString()).to.include("WrongItemType");
      }
    });
  });

  describe("Equip Gear", () => {
    it("equips gear in available slot", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Equip Miner Helmet (G-ST-01)
      const itemId = makeItemId("G-ST-01");
      await program.methods
        .equipGear(itemId, { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify gear is equipped
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(inventory.gear[0]).to.not.be.null;
      expect(Array.from(inventory.gear[0]!.itemId)).to.deep.equal(itemId);
    });

    it("fails when all gear slots are full", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Equip 4 gear items (fills initial slots)
      const gearItems = ["G-ST-01", "G-ST-02", "G-SC-01", "G-SC-02"];
      for (const itemStr of gearItems) {
        const itemId = makeItemId(itemStr);
        await program.methods
          .equipGear(itemId, { i: {} })
          .accounts({
            inventory: inventoryPda,
            player: player.publicKey,
          } as any)
          .signers([player])
          .rpc();
      }

      // Try to equip 5th item (should fail)
      const itemId5 = makeItemId("G-FR-01");
      try {
        await program.methods
          .equipGear(itemId5, { i: {} })
          .accounts({
            inventory: inventoryPda,
            player: player.publicKey,
          } as any)
          .signers([player])
          .rpc();
        expect.fail("Should have thrown InventoryFull error");
      } catch (error: any) {
        expect(error.toString()).to.include("InventoryFull");
      }
    });
  });

  describe("Item Fusion", () => {
    it("fuses two identical Tier I items to Tier II", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Equip two identical gear items
      const itemId = makeItemId("G-ST-01");
      await program.methods
        .equipGear(itemId, { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      await program.methods
        .equipGear(itemId, { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Fuse items in slots 0 and 1
      await program.methods
        .fuseItems(0, 1)
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify fusion result
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(inventory.gear[0]).to.not.be.null;
      expect(inventory.gear[0]!.tier).to.deep.equal({ ii: {} });
      expect(inventory.gear[1]).to.be.null; // Second slot should be empty
    });

    it("fails to fuse different items", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Equip two different items
      await program.methods
        .equipGear(makeItemId("G-ST-01"), { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      await program.methods
        .equipGear(makeItemId("G-ST-02"), { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Try to fuse (should fail)
      try {
        await program.methods
          .fuseItems(0, 1)
          .accounts({
            inventory: inventoryPda,
            player: player.publicKey,
          } as any)
          .signers([player])
          .rpc();
        expect.fail("Should have thrown FusionMismatch error");
      } catch (error: any) {
        expect(error.toString()).to.include("FusionMismatch");
      }
    });
  });

  describe("Gear Slot Expansion", () => {
    it("expands gear slots from 4 to 6", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Expand slots
      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify expansion
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(inventory.gearSlotCapacity).to.equal(6);
    });

    it("expands gear slots from 6 to 8", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Expand twice
      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify expansion
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(inventory.gearSlotCapacity).to.equal(8);
    });

    it("fails to expand beyond max slots", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Expand to max
      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      await program.methods
        .expandGearSlots()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Try to expand again (should fail)
      try {
        await program.methods
          .expandGearSlots()
          .accounts({
            inventory: inventoryPda,
            player: player.publicKey,
          } as any)
          .signers([player])
          .rpc();
        expect.fail("Should have thrown AlreadyMaxSlots error");
      } catch (error: any) {
        expect(error.toString()).to.include("AlreadyMaxSlots");
      }
    });
  });

  describe("Tool Oil Application", () => {
    it("applies Tool Oil to equipped tool", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Equip tool first
      await program.methods
        .equipTool(makeItemId("T-SC-01"), { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Apply Tool Oil
      await program.methods
        .applyToolOil({ plusAtk: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify oil is applied
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(inventory.tool!.toolOilFlags & 0x01).to.equal(1); // +ATK flag
    });

    it("fails to apply same oil twice", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      await program.methods
        .equipTool(makeItemId("T-SC-01"), { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      await program.methods
        .applyToolOil({ plusAtk: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Try to apply same oil again (should fail)
      try {
        await program.methods
          .applyToolOil({ plusAtk: {} })
          .accounts({
            inventory: inventoryPda,
            player: player.publicKey,
          } as any)
          .signers([player])
          .rpc();
        expect.fail("Should have thrown ToolOilAlreadyApplied error");
      } catch (error: any) {
        expect(error.toString()).to.include("ToolOilAlreadyApplied");
      }
    });
  });

  describe("Unequip Gear", () => {
    it("unequips gear from slot", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      await program.methods
        .equipGear(makeItemId("G-ST-01"), { i: {} })
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Unequip
      await program.methods
        .unequipGear(0)
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
        } as any)
        .signers([player])
        .rpc();

      // Verify slot is empty
      const inventory =
        await program.account.playerInventory.fetch(inventoryPda);
      expect(inventory.gear[0]).to.be.null;
    });

    it("fails to unequip from empty slot", async () => {
      const player = Keypair.generate();

      const airdropSig = await provider.connection.requestAirdrop(
        player.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL,
      );
      await provider.connection.confirmTransaction(airdropSig);

      const [inventoryPda] = getInventoryPDA(player.publicKey);

      await program.methods
        .initializeInventory()
        .accounts({
          inventory: inventoryPda,
          player: player.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([player])
        .rpc();

      // Try to unequip from empty slot
      try {
        await program.methods
          .unequipGear(0)
          .accounts({
            inventory: inventoryPda,
            player: player.publicKey,
          } as any)
          .signers([player])
          .rpc();
        expect.fail("Should have thrown SlotEmpty error");
      } catch (error: any) {
        expect(error.toString()).to.include("SlotEmpty");
      }
    });
  });
});
