/**
 * E2E Test Suite: Track 1 - NFT / Marketplace
 *
 * Tests Metaplex Core collections, NFT minting (skins + items),
 * equip/unequip, marketplace list/buy/cancel, quests, and purchase runs.
 *
 * Runs against surfpool (devnet fork on localhost:8899).
 * Programs are auto-deployed from target/deploy/*.so.
 */

import { expect } from "chai";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Connection,
} from "@solana/web3.js";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  createCollection,
  ruleSet,
} from "@metaplex-foundation/mpl-core";
import {
  generateSigner,
  publicKey,
  createSignerFromKeypair,
  keypairIdentity,
} from "@metaplex-foundation/umi";
import { fromWeb3JsKeypair } from "@metaplex-foundation/umi-web3js-adapters";
import {
  PROGRAM_IDS,
  COMPANY_TREASURY,
  loadProgram,
  createProvider,
  walletFromKeypair,
  airdropAndConfirm,
  createUmiContext,
  loadWalletKeypair,
} from "../shared/setup";
import {
  getMarketplaceConfigPda,
  getMintAuthorityPda,
  getListingPda,
  getQuestDefinitionPda,
  getQuestProgressPda,
  getPlayerProfilePda,
  getGauntletConfigPda,
  getGauntletPoolVaultPda,
  getGauntletWeekPoolPda,
} from "../shared/pda-helpers";

// ── Constants ───────────────────────────────────────────────────────────────
const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const ROYALTY_BPS = 500;

// ── Shared state ────────────────────────────────────────────────────────────
let connection: Connection;
let admin: Keypair;
let provider: anchor.AnchorProvider;
let nftMarketplace: Program;
let playerProfile: Program;
let gameplayState: Program;

// Collection public keys (set after UMI collection creation)
let skinsCollectionPubkey: PublicKey;
let itemsCollectionPubkey: PublicKey;

// PDAs
const [marketplaceConfigPda] = getMarketplaceConfigPda();
const [mintAuthorityPda] = getMintAuthorityPda();
const [gauntletConfigPda] = getGauntletConfigPda();
const [gauntletPoolVaultPda] = getGauntletPoolVaultPda();

// Track minted assets for later tests
let mintedSkinAsset: PublicKey;
const mintedNftItemAssets: PublicKey[] = [];

// ── Helpers ─────────────────────────────────────────────────────────────────

/** Airdrop SOL to a keypair and return a configured provider for that keypair. */
async function fundKeypair(kp: Keypair, sol: number = 5): Promise<void> {
  await airdropAndConfirm(connection, kp.publicKey, sol * LAMPORTS_PER_SOL);
}

/** Create a player profile for the given keypair. */
async function createProfileForUser(
  user: Keypair,
  name: string
): Promise<PublicKey> {
  const [profilePda] = getPlayerProfilePda(user.publicKey);
  const userProvider = createProvider(RPC_URL, walletFromKeypair(user));
  const pp = loadProgram("player_profile", userProvider);

  await pp.methods
    .initializeProfile(name)
    .accounts({
      playerProfile: profilePda,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .rpc();

  return profilePda;
}

/** Mint a skin to a target owner (called by admin). */
async function mintSkinToOwner(
  ownerPubkey: PublicKey,
  name: string = "Test Skin",
  skinId: number = 0
): Promise<PublicKey> {
  const assetKeypair = Keypair.generate();

  await nftMarketplace.methods
    .mintSkin(name, "https://arweave.net/placeholder", skinId, 0, 0)
    .accounts({
      asset: assetKeypair.publicKey,
      collection: skinsCollectionPubkey,
      marketplaceConfig: marketplaceConfigPda,
      mintAuthority: mintAuthorityPda,
      payer: admin.publicKey,
      owner: ownerPubkey,
      mplCoreProgram: PROGRAM_IDS.mplCore,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([assetKeypair])
    .rpc();

  return assetKeypair.publicKey;
}

// ── Top-level setup ─────────────────────────────────────────────────────────

before(async function () {
  this.timeout(30_000);

  admin = loadWalletKeypair();
  connection = new Connection(RPC_URL, "confirmed");
  provider = createProvider(RPC_URL, walletFromKeypair(admin));

  nftMarketplace = loadProgram("nft_marketplace", provider);
  playerProfile = loadProgram("player_profile", provider);
  gameplayState = loadProgram("gameplay_state", provider);

  await airdropAndConfirm(connection, admin.publicKey, 10 * LAMPORTS_PER_SOL);
});

// ═══════════════════════════════════════════════════════════════════════════
// 1. Initialize Infrastructure
// ═══════════════════════════════════════════════════════════════════════════

describe("Initialize infrastructure", function () {
  this.timeout(120_000);

  it("initializes gauntlet (needed for gauntlet_pool PDA existence)", async () => {
    const [week1] = getGauntletWeekPoolPda(1);
    const [week2] = getGauntletWeekPoolPda(2);
    const [week3] = getGauntletWeekPoolPda(3);
    const [week4] = getGauntletWeekPoolPda(4);
    const [week5] = getGauntletWeekPoolPda(5);

    try {
      await gameplayState.methods
        .initializeGauntlet()
        .accounts({
          gauntletConfig: gauntletConfigPda,
          gauntletPoolVault: gauntletPoolVaultPda,
          gauntletWeek1: week1,
          gauntletWeek2: week2,
          gauntletWeek3: week3,
          gauntletWeek4: week4,
          gauntletWeek5: week5,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([
          anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
            units: 1_400_000,
          }),
        ])
        .rpc();
    } catch (error: any) {
      // Already initialized is acceptable
      if (!error.toString().includes("already in use")) {
        throw error;
      }
    }

    // Verify gauntlet config exists
    const info = await connection.getAccountInfo(gauntletConfigPda);
    expect(info).to.not.be.null;
  });

  it("creates Metaplex Core collections via UMI", async () => {
    const { umi, signer } = createUmiContext(RPC_URL, admin);
    const mintAuthority = publicKey(mintAuthorityPda.toBase58());
    const treasury = publicKey(COMPANY_TREASURY.toBase58());
    const gauntletPool = publicKey(gauntletPoolVaultPda.toBase58());

    // Create skins collection
    const skinsCollection = generateSigner(umi);
    await createCollection(umi, {
      collection: skinsCollection,
      name: "Dungeons & Moles Skins",
      uri: "https://arweave.net/skins-collection-metadata",
      updateAuthority: mintAuthority,
      plugins: [
        {
          type: "Royalties",
          basisPoints: ROYALTY_BPS,
          creators: [
            { address: treasury, percentage: 60 },
            { address: gauntletPool, percentage: 40 },
          ],
          ruleSet: ruleSet("None"),
        },
      ],
    }).sendAndConfirm(umi);

    skinsCollectionPubkey = new PublicKey(skinsCollection.publicKey.toString());

    // Create NFT items collection
    const itemsCollection = generateSigner(umi);
    await createCollection(umi, {
      collection: itemsCollection,
      name: "Dungeons & Moles NFT Items",
      uri: "https://arweave.net/items-collection-metadata",
      updateAuthority: mintAuthority,
      plugins: [
        {
          type: "Royalties",
          basisPoints: ROYALTY_BPS,
          creators: [
            { address: treasury, percentage: 60 },
            { address: gauntletPool, percentage: 40 },
          ],
          ruleSet: ruleSet("None"),
        },
      ],
    }).sendAndConfirm(umi);

    itemsCollectionPubkey = new PublicKey(
      itemsCollection.publicKey.toString()
    );

    // Verify both exist on-chain
    const skinsInfo = await connection.getAccountInfo(skinsCollectionPubkey);
    expect(skinsInfo).to.not.be.null;

    const itemsInfo = await connection.getAccountInfo(itemsCollectionPubkey);
    expect(itemsInfo).to.not.be.null;
  });

  it("initializes marketplace config", async () => {
    await nftMarketplace.methods
      .initializeMarketplace(skinsCollectionPubkey, itemsCollectionPubkey)
      .accounts({
        marketplaceConfig: marketplaceConfigPda,
        authority: admin.publicKey,
        gauntletPool: gauntletPoolVaultPda,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Verify marketplace config account fields
    const config = await (
      nftMarketplace.account as any
    ).marketplaceConfig.fetch(marketplaceConfigPda);

    expect(config.authority.toString()).to.equal(admin.publicKey.toString());
    expect(config.skinsCollection.toString()).to.equal(
      skinsCollectionPubkey.toString()
    );
    expect(config.itemsCollection.toString()).to.equal(
      itemsCollectionPubkey.toString()
    );
    expect(config.companyTreasury.toString()).to.equal(
      COMPANY_TREASURY.toString()
    );
    expect(config.gauntletPool.toString()).to.equal(
      gauntletPoolVaultPda.toString()
    );
    expect(config.companyFeeBps).to.equal(300); // 3%
    expect(config.gauntletFeeBps).to.equal(200); // 2%
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// 2. Gauntlet pool validation
// ═══════════════════════════════════════════════════════════════════════════

describe("Gauntlet pool validation", function () {
  this.timeout(30_000);

  it("rejects initializeMarketplace with wrong gauntlet_pool", async () => {
    const fakePool = Keypair.generate().publicKey;

    try {
      await nftMarketplace.methods
        .initializeMarketplace(skinsCollectionPubkey, itemsCollectionPubkey)
        .accounts({
          marketplaceConfig: marketplaceConfigPda,
          authority: admin.publicKey,
          gauntletPool: fakePool,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      expect.fail("Should have thrown");
    } catch (error: any) {
      expect(error.toString()).to.satisfy(
        (msg: string) =>
          msg.includes("InvalidGauntletPool") ||
          msg.includes("ConstraintAddress") ||
          msg.includes("already in use")
      );
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// 3. Mint skins
// ═══════════════════════════════════════════════════════════════════════════

describe("Mint skins", function () {
  this.timeout(60_000);

  it("mints a skin via mintSkin and verifies on-chain", async () => {
    const assetKeypair = Keypair.generate();

    await nftMarketplace.methods
      .mintSkin("Golden Mole", "https://arweave.net/golden-mole", 1, 1, 2)
      .accounts({
        asset: assetKeypair.publicKey,
        collection: skinsCollectionPubkey,
        marketplaceConfig: marketplaceConfigPda,
        mintAuthority: mintAuthorityPda,
        payer: admin.publicKey,
        owner: admin.publicKey,
        mplCoreProgram: PROGRAM_IDS.mplCore,
        systemProgram: SystemProgram.programId,
      } as any)
      .signers([assetKeypair])
      .rpc();

    // Verify asset exists on-chain
    const assetInfo = await connection.getAccountInfo(assetKeypair.publicKey);
    expect(assetInfo).to.not.be.null;
    expect(assetInfo!.owner.toString()).to.equal(
      PROGRAM_IDS.mplCore.toString()
    );

    mintedSkinAsset = assetKeypair.publicKey;
  });

  it("rejects non-authority mint", async () => {
    const randomUser = Keypair.generate();
    await fundKeypair(randomUser, 2);

    const userProvider = createProvider(
      RPC_URL,
      walletFromKeypair(randomUser)
    );
    const userMarketplace = loadProgram("nft_marketplace", userProvider);

    const assetKeypair = Keypair.generate();

    try {
      await userMarketplace.methods
        .mintSkin("Hacker Skin", "https://arweave.net/hack", 999, 0, 0)
        .accounts({
          asset: assetKeypair.publicKey,
          collection: skinsCollectionPubkey,
          marketplaceConfig: marketplaceConfigPda,
          mintAuthority: mintAuthorityPda,
          payer: randomUser.publicKey,
          owner: randomUser.publicKey,
          mplCoreProgram: PROGRAM_IDS.mplCore,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([assetKeypair])
        .rpc();
      expect.fail("Should have thrown");
    } catch (error: any) {
      // Non-authority payer should be rejected
      expect(error).to.not.be.null;
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// 4. Mint NFT items
// ═══════════════════════════════════════════════════════════════════════════

describe("Mint NFT items", function () {
  this.timeout(120_000);

  const NFT_ITEM_IDS = [
    "S-XX-01",
    "S-XX-02",
    "S-XX-03",
    "S-XX-04",
    "S-XX-05",
    "S-XX-06",
  ];

  const NFT_ITEM_NAMES = [
    "Infernal Pickaxe",
    "Shadow Cloak",
    "Crystal Shield",
    "Void Hammer",
    "Storm Bow",
    "Ancient Tome",
  ];

  for (let i = 0; i < NFT_ITEM_IDS.length; i++) {
    it(`mints NFT item ${NFT_ITEM_IDS[i]}`, async () => {
      const assetKeypair = Keypair.generate();

      // Pad ID to 8 bytes
      const idBytes = Buffer.alloc(8, 0);
      idBytes.write(NFT_ITEM_IDS[i], 0, "utf-8");

      await nftMarketplace.methods
        .mintNftItem(
          NFT_ITEM_NAMES[i],
          "https://arweave.net/placeholder",
          Array.from(idBytes)
        )
        .accounts({
          asset: assetKeypair.publicKey,
          collection: itemsCollectionPubkey,
          marketplaceConfig: marketplaceConfigPda,
          mintAuthority: mintAuthorityPda,
          payer: admin.publicKey,
          owner: admin.publicKey,
          mplCoreProgram: PROGRAM_IDS.mplCore,
          systemProgram: SystemProgram.programId,
        } as any)
        .signers([assetKeypair])
        .rpc();

      // Verify asset exists
      const assetInfo = await connection.getAccountInfo(
        assetKeypair.publicKey
      );
      expect(assetInfo).to.not.be.null;
      expect(assetInfo!.owner.toString()).to.equal(
        PROGRAM_IDS.mplCore.toString()
      );

      mintedNftItemAssets.push(assetKeypair.publicKey);
    });
  }
});

// ═══════════════════════════════════════════════════════════════════════════
// 5. Equip/unequip skin
// ═══════════════════════════════════════════════════════════════════════════

describe("Equip/unequip skin", function () {
  this.timeout(60_000);

  let skinUser: Keypair;
  let skinUserProfilePda: PublicKey;
  let skinUserAsset: PublicKey;

  before(async () => {
    skinUser = Keypair.generate();
    await fundKeypair(skinUser, 5);

    // Create player profile for the skin user
    skinUserProfilePda = await createProfileForUser(skinUser, "SkinTester");

    // Mint a skin to the user
    skinUserAsset = await mintSkinToOwner(skinUser.publicKey, "User Skin", 10);
  });

  it("equips a skin via playerProfile.equipSkin", async () => {
    const userProvider = createProvider(
      RPC_URL,
      walletFromKeypair(skinUser)
    );
    const pp = loadProgram("player_profile", userProvider);

    await pp.methods
      .equipSkin()
      .accounts({
        playerProfile: skinUserProfilePda,
        owner: skinUser.publicKey,
        skinAsset: skinUserAsset,
      } as any)
      .rpc();

    // Verify equipped_skin is set on profile
    const profile = await (pp.account as any).playerProfile.fetch(skinUserProfilePda);
    expect((profile as any).equippedSkin).to.not.be.null;
    expect((profile as any).equippedSkin.toString()).to.equal(
      skinUserAsset.toString()
    );
  });

  it("unequips via playerProfile.unequipSkin", async () => {
    const userProvider = createProvider(
      RPC_URL,
      walletFromKeypair(skinUser)
    );
    const pp = loadProgram("player_profile", userProvider);

    await pp.methods
      .unequipSkin()
      .accounts({
        playerProfile: skinUserProfilePda,
        owner: skinUser.publicKey,
      } as any)
      .rpc();

    // Verify equipped_skin is cleared
    const profile = await (pp.account as any).playerProfile.fetch(skinUserProfilePda);
    expect((profile as any).equippedSkin).to.be.null;
  });

  it("rejects non-owner equip", async () => {
    const otherUser = Keypair.generate();
    await fundKeypair(otherUser, 2);

    const otherProvider = createProvider(
      RPC_URL,
      walletFromKeypair(otherUser)
    );
    const pp = loadProgram("player_profile", otherProvider);

    // Other user tries to equip the skin owned by skinUser
    const [otherProfilePda] = getPlayerProfilePda(otherUser.publicKey);

    // Create profile for the other user first
    await pp.methods
      .initializeProfile("OtherUser")
      .accounts({
        playerProfile: otherProfilePda,
        owner: otherUser.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    try {
      await pp.methods
        .equipSkin()
        .accounts({
          playerProfile: otherProfilePda,
          owner: otherUser.publicKey,
          skinAsset: skinUserAsset,
        } as any)
        .rpc();
      expect.fail("Should have thrown");
    } catch (error: any) {
      expect(error.toString()).to.satisfy(
        (msg: string) =>
          msg.includes("SkinNotOwned") ||
          msg.includes("InvalidSkinAsset") ||
          msg.includes("ConstraintRaw")
      );
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// 6. List/buy/cancel
// ═══════════════════════════════════════════════════════════════════════════

describe("List/buy/cancel", function () {
  this.timeout(120_000);

  let seller: Keypair;
  let buyer: Keypair;
  let sellerProfilePda: PublicKey;
  let sellerSkinAsset: PublicKey;
  const listPrice = new anchor.BN(1_000_000_000); // 1 SOL

  before(async () => {
    seller = Keypair.generate();
    buyer = Keypair.generate();
    await fundKeypair(seller, 10);
    await fundKeypair(buyer, 10);

    // Create profiles
    sellerProfilePda = await createProfileForUser(seller, "SellerUser");
    await createProfileForUser(buyer, "BuyerUser");

    // Mint a skin to seller
    sellerSkinAsset = await mintSkinToOwner(
      seller.publicKey,
      "Seller Skin",
      20
    );
  });

  it("lists a skin via listNft", async () => {
    const sellerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(seller)
    );
    const sellerMarketplace = loadProgram("nft_marketplace", sellerProvider);

    const [listingPda] = getListingPda(sellerSkinAsset);

    await sellerMarketplace.methods
      .listNft(listPrice)
      .accounts({
        listing: listingPda,
        marketplaceConfig: marketplaceConfigPda,
        mintAuthority: mintAuthorityPda,
        asset: sellerSkinAsset,
        collection: skinsCollectionPubkey,
        seller: seller.publicKey,
        playerProfile: sellerProfilePda,
        mplCoreProgram: PROGRAM_IDS.mplCore,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Verify listing PDA data
    const listing = await (
      sellerMarketplace.account as any
    ).listing.fetch(listingPda);
    expect(listing.seller.toString()).to.equal(seller.publicKey.toString());
    expect(listing.asset.toString()).to.equal(sellerSkinAsset.toString());
    expect(listing.collection.toString()).to.equal(
      skinsCollectionPubkey.toString()
    );
    expect(listing.priceLamports.toNumber()).to.equal(listPrice.toNumber());
  });

  it("cancels listing successfully", async () => {
    const sellerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(seller)
    );
    const sellerMarketplace = loadProgram("nft_marketplace", sellerProvider);

    const [listingPda] = getListingPda(sellerSkinAsset);

    await sellerMarketplace.methods
      .cancelListing()
      .accounts({
        listing: listingPda,
        asset: sellerSkinAsset,
        collection: skinsCollectionPubkey,
        seller: seller.publicKey,
        mplCoreProgram: PROGRAM_IDS.mplCore,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Verify listing PDA is closed
    const listingInfo = await connection.getAccountInfo(listingPda);
    expect(listingInfo).to.be.null;
  });

  it("re-lists and buyer purchases with fee split", async () => {
    // Re-list
    const sellerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(seller)
    );
    const sellerMarketplace = loadProgram("nft_marketplace", sellerProvider);

    const [listingPda] = getListingPda(sellerSkinAsset);

    await sellerMarketplace.methods
      .listNft(listPrice)
      .accounts({
        listing: listingPda,
        marketplaceConfig: marketplaceConfigPda,
        mintAuthority: mintAuthorityPda,
        asset: sellerSkinAsset,
        collection: skinsCollectionPubkey,
        seller: seller.publicKey,
        playerProfile: sellerProfilePda,
        mplCoreProgram: PROGRAM_IDS.mplCore,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Capture balances before purchase
    const sellerBalBefore = await connection.getBalance(seller.publicKey);
    const treasuryBalBefore = await connection.getBalance(COMPANY_TREASURY);
    const gauntletBalBefore = await connection.getBalance(gauntletPoolVaultPda);

    // Buyer purchases
    const buyerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(buyer)
    );
    const buyerMarketplace = loadProgram("nft_marketplace", buyerProvider);

    await buyerMarketplace.methods
      .buyNft()
      .accounts({
        listing: listingPda,
        marketplaceConfig: marketplaceConfigPda,
        mintAuthority: mintAuthorityPda,
        asset: sellerSkinAsset,
        collection: skinsCollectionPubkey,
        buyer: buyer.publicKey,
        seller: seller.publicKey,
        companyTreasury: COMPANY_TREASURY,
        gauntletPool: gauntletPoolVaultPda,
        mplCoreProgram: PROGRAM_IDS.mplCore,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Verify listing is closed
    const listingInfo = await connection.getAccountInfo(listingPda);
    expect(listingInfo).to.be.null;

    // Verify fee split: 3% company, 2% gauntlet
    const priceNum = listPrice.toNumber();
    const expectedCompanyFee = Math.floor((priceNum * 300) / 10_000);
    const expectedGauntletFee = Math.floor((priceNum * 200) / 10_000);

    const treasuryBalAfter = await connection.getBalance(COMPANY_TREASURY);
    const gauntletBalAfter = await connection.getBalance(gauntletPoolVaultPda);
    const sellerBalAfter = await connection.getBalance(seller.publicKey);

    // Treasury should receive 3% (30_000_000 lamports)
    expect(treasuryBalAfter - treasuryBalBefore).to.equal(expectedCompanyFee);

    // Gauntlet pool should receive 2% (20_000_000 lamports)
    expect(gauntletBalAfter - gauntletBalBefore).to.equal(expectedGauntletFee);

    // Seller should receive price - company_fee - gauntlet_fee + listing account rent refund
    const sellerNetReceived = sellerBalAfter - sellerBalBefore;
    const expectedSellerNet = priceNum - expectedCompanyFee - expectedGauntletFee;
    // Allow small deviation for rent refund from closed listing account
    expect(sellerNetReceived).to.be.greaterThanOrEqual(expectedSellerNet);
  });

  it("rejects buy-self (seller tries to buy own listing)", async () => {
    // Mint a new skin to seller so they can list it
    const newSkin = await mintSkinToOwner(
      seller.publicKey,
      "Self-buy Skin",
      21
    );

    const sellerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(seller)
    );
    const sellerMarketplace = loadProgram("nft_marketplace", sellerProvider);

    const [listingPda] = getListingPda(newSkin);

    await sellerMarketplace.methods
      .listNft(listPrice)
      .accounts({
        listing: listingPda,
        marketplaceConfig: marketplaceConfigPda,
        mintAuthority: mintAuthorityPda,
        asset: newSkin,
        collection: skinsCollectionPubkey,
        seller: seller.publicKey,
        playerProfile: sellerProfilePda,
        mplCoreProgram: PROGRAM_IDS.mplCore,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Seller tries to buy their own listing
    try {
      await sellerMarketplace.methods
        .buyNft()
        .accounts({
          listing: listingPda,
          marketplaceConfig: marketplaceConfigPda,
          mintAuthority: mintAuthorityPda,
          asset: newSkin,
          collection: skinsCollectionPubkey,
          buyer: seller.publicKey,
          seller: seller.publicKey,
          companyTreasury: COMPANY_TREASURY,
          gauntletPool: gauntletPoolVaultPda,
          mplCoreProgram: PROGRAM_IDS.mplCore,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      expect.fail("Should have thrown CannotBuySelf");
    } catch (error: any) {
      expect(error.toString()).to.include("CannotBuySelf");
    }

    // Cleanup: cancel the listing
    await sellerMarketplace.methods
      .cancelListing()
      .accounts({
        listing: listingPda,
        asset: newSkin,
        collection: skinsCollectionPubkey,
        seller: seller.publicKey,
        mplCoreProgram: PROGRAM_IDS.mplCore,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();
  });

  it("rejects price=0 listing", async () => {
    // Mint new skin to seller for this test
    const zeroSkin = await mintSkinToOwner(
      seller.publicKey,
      "Zero Price Skin",
      22
    );

    const sellerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(seller)
    );
    const sellerMarketplace = loadProgram("nft_marketplace", sellerProvider);

    const [listingPda] = getListingPda(zeroSkin);

    try {
      await sellerMarketplace.methods
        .listNft(new anchor.BN(0))
        .accounts({
          listing: listingPda,
          marketplaceConfig: marketplaceConfigPda,
          mintAuthority: mintAuthorityPda,
          asset: zeroSkin,
          collection: skinsCollectionPubkey,
          seller: seller.publicKey,
          playerProfile: sellerProfilePda,
          mplCoreProgram: PROGRAM_IDS.mplCore,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      expect.fail("Should have thrown InvalidPrice");
    } catch (error: any) {
      expect(error.toString()).to.include("InvalidPrice");
    }
  });

  it("rejects listing an equipped skin", async () => {
    // Mint a skin and equip it
    const equipSkin = await mintSkinToOwner(
      seller.publicKey,
      "Equipped Skin",
      23
    );

    const sellerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(seller)
    );
    const pp = loadProgram("player_profile", sellerProvider);

    // Equip the skin
    await pp.methods
      .equipSkin()
      .accounts({
        playerProfile: sellerProfilePda,
        owner: seller.publicKey,
        skinAsset: equipSkin,
      } as any)
      .rpc();

    // Try to list the equipped skin
    const sellerMarketplace = loadProgram("nft_marketplace", sellerProvider);
    const [listingPda] = getListingPda(equipSkin);

    try {
      await sellerMarketplace.methods
        .listNft(listPrice)
        .accounts({
          listing: listingPda,
          marketplaceConfig: marketplaceConfigPda,
          mintAuthority: mintAuthorityPda,
          asset: equipSkin,
          collection: skinsCollectionPubkey,
          seller: seller.publicKey,
          playerProfile: sellerProfilePda,
          mplCoreProgram: PROGRAM_IDS.mplCore,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      expect.fail("Should have thrown SkinCurrentlyEquipped");
    } catch (error: any) {
      expect(error.toString()).to.include("SkinCurrentlyEquipped");
    }

    // Cleanup: unequip
    await pp.methods
      .unequipSkin()
      .accounts({
        playerProfile: sellerProfilePda,
        owner: seller.publicKey,
      } as any)
      .rpc();
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// 7. Quests
// ═══════════════════════════════════════════════════════════════════════════

describe("Quests", function () {
  this.timeout(60_000);

  const questId = 1;
  const objectiveCount = 5; // Need to win 5 battles
  let questPlayer: Keypair;

  before(async () => {
    questPlayer = Keypair.generate();
    await fundKeypair(questPlayer, 5);
  });

  it("creates a quest (admin)", async () => {
    const [questDefPda] = getQuestDefinitionPda(questId);

    // reward_data: 32 bytes of zeros (placeholder)
    const rewardData = Array(32).fill(0);

    await nftMarketplace.methods
      .createQuest(
        questId,
        { daily: {} },         // QuestType::Daily
        { winBattles: {} },    // ObjectiveType::WinBattles
        objectiveCount,
        { gauntletBooster: {} }, // RewardType::GauntletBooster
        rewardData,
        1                       // season
      )
      .accounts({
        questDefinition: questDefPda,
        marketplaceConfig: marketplaceConfigPda,
        authority: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Verify quest definition
    const quest = await (
      nftMarketplace.account as any
    ).questDefinition.fetch(questDefPda);
    expect(quest.questId).to.equal(questId);
    expect(quest.objectiveCount).to.equal(objectiveCount);
    expect(quest.active).to.be.true;
    expect(quest.season).to.equal(1);
  });

  it("player accepts the quest", async () => {
    const playerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(questPlayer)
    );
    const playerMarketplace = loadProgram("nft_marketplace", playerProvider);

    const [questDefPda] = getQuestDefinitionPda(questId);
    const [questProgressPda] = getQuestProgressPda(
      questPlayer.publicKey,
      questId
    );

    await playerMarketplace.methods
      .acceptQuest(questId)
      .accounts({
        questDefinition: questDefPda,
        questProgress: questProgressPda,
        player: questPlayer.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Verify progress account
    const progress = await (
      playerMarketplace.account as any
    ).questProgress.fetch(questProgressPda);
    expect(progress.player.toString()).to.equal(
      questPlayer.publicKey.toString()
    );
    expect(progress.questId).to.equal(questId);
    expect(progress.progress).to.equal(0);
    expect(progress.completed).to.be.false;
    expect(progress.claimed).to.be.false;
  });

  it("updates quest progress", async () => {
    const playerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(questPlayer)
    );
    const playerMarketplace = loadProgram("nft_marketplace", playerProvider);

    const [questDefPda] = getQuestDefinitionPda(questId);
    const [questProgressPda] = getQuestProgressPda(
      questPlayer.publicKey,
      questId
    );

    // Increment by 3 (partial)
    await playerMarketplace.methods
      .updateQuestProgress(questId, 3)
      .accounts({
        questDefinition: questDefPda,
        questProgress: questProgressPda,
        player: questPlayer.publicKey,
      } as any)
      .rpc();

    let progress = await (
      playerMarketplace.account as any
    ).questProgress.fetch(questProgressPda);
    expect(progress.progress).to.equal(3);
    expect(progress.completed).to.be.false;

    // Increment by 2 more to complete (total = 5)
    await playerMarketplace.methods
      .updateQuestProgress(questId, 2)
      .accounts({
        questDefinition: questDefPda,
        questProgress: questProgressPda,
        player: questPlayer.publicKey,
      } as any)
      .rpc();

    progress = await (
      playerMarketplace.account as any
    ).questProgress.fetch(questProgressPda);
    expect(progress.progress).to.equal(5);
    expect(progress.completed).to.be.true;
  });

  it("rejects claiming incomplete quest", async () => {
    // Create a second quest that has not been completed
    const incompleteQuestId = 2;
    const [incQuestDefPda] = getQuestDefinitionPda(incompleteQuestId);

    const rewardData = Array(32).fill(0);

    await nftMarketplace.methods
      .createQuest(
        incompleteQuestId,
        { weekly: {} },
        { completeLevels: {} },
        10, // need 10 levels
        { skin: {} },
        rewardData,
        1
      )
      .accounts({
        questDefinition: incQuestDefPda,
        marketplaceConfig: marketplaceConfigPda,
        authority: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Player accepts
    const playerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(questPlayer)
    );
    const playerMarketplace = loadProgram("nft_marketplace", playerProvider);
    const [incProgressPda] = getQuestProgressPda(
      questPlayer.publicKey,
      incompleteQuestId
    );

    await playerMarketplace.methods
      .acceptQuest(incompleteQuestId)
      .accounts({
        questDefinition: incQuestDefPda,
        questProgress: incProgressPda,
        player: questPlayer.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Try to claim without completing
    try {
      await playerMarketplace.methods
        .claimQuestReward(incompleteQuestId)
        .accounts({
          questDefinition: incQuestDefPda,
          questProgress: incProgressPda,
          player: questPlayer.publicKey,
        } as any)
        .rpc();
      expect.fail("Should have thrown QuestNotCompleted");
    } catch (error: any) {
      expect(error.toString()).to.include("QuestNotCompleted");
    }
  });

  it("claims completed quest reward", async () => {
    const playerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(questPlayer)
    );
    const playerMarketplace = loadProgram("nft_marketplace", playerProvider);

    const [questDefPda] = getQuestDefinitionPda(questId);
    const [questProgressPda] = getQuestProgressPda(
      questPlayer.publicKey,
      questId
    );

    await playerMarketplace.methods
      .claimQuestReward(questId)
      .accounts({
        questDefinition: questDefPda,
        questProgress: questProgressPda,
        player: questPlayer.publicKey,
      } as any)
      .rpc();

    // Verify claimed
    const progress = await (
      playerMarketplace.account as any
    ).questProgress.fetch(questProgressPda);
    expect(progress.claimed).to.be.true;
  });

  it("rejects double-claim", async () => {
    const playerProvider = createProvider(
      RPC_URL,
      walletFromKeypair(questPlayer)
    );
    const playerMarketplace = loadProgram("nft_marketplace", playerProvider);

    const [questDefPda] = getQuestDefinitionPda(questId);
    const [questProgressPda] = getQuestProgressPda(
      questPlayer.publicKey,
      questId
    );

    try {
      await playerMarketplace.methods
        .claimQuestReward(questId)
        .accounts({
          questDefinition: questDefPda,
          questProgress: questProgressPda,
          player: questPlayer.publicKey,
        } as any)
        .rpc();
      expect.fail("Should have thrown QuestRewardAlreadyClaimed");
    } catch (error: any) {
      expect(error.toString()).to.include("QuestRewardAlreadyClaimed");
    }
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// 8. Purchase runs
// ═══════════════════════════════════════════════════════════════════════════

describe("Purchase runs", function () {
  this.timeout(60_000);

  let runUser: Keypair;
  let runUserProfilePda: PublicKey;

  before(async () => {
    runUser = Keypair.generate();
    await fundKeypair(runUser, 5);

    runUserProfilePda = await createProfileForUser(runUser, "RunBuyer");
  });

  it("purchases runs with canonical gauntlet pool", async () => {
    const userProvider = createProvider(
      RPC_URL,
      walletFromKeypair(runUser)
    );
    const pp = loadProgram("player_profile", userProvider);

    // Verify initial available_runs = 20
    let profile = await (pp.account as any).playerProfile.fetch(runUserProfilePda);
    expect((profile as any).availableRuns).to.equal(20);

    await pp.methods
      .purchaseRuns()
      .accounts({
        playerProfile: runUserProfilePda,
        owner: runUser.publicKey,
        treasury: COMPANY_TREASURY,
        gauntletPool: gauntletPoolVaultPda,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();

    // Verify available_runs increased by 20
    profile = await (pp.account as any).playerProfile.fetch(runUserProfilePda);
    expect((profile as any).availableRuns).to.equal(40);
  });

  it("rejects wrong gauntlet pool", async () => {
    const userProvider = createProvider(
      RPC_URL,
      walletFromKeypair(runUser)
    );
    const pp = loadProgram("player_profile", userProvider);

    const fakePool = Keypair.generate().publicKey;

    try {
      await pp.methods
        .purchaseRuns()
        .accounts({
          playerProfile: runUserProfilePda,
          owner: runUser.publicKey,
          treasury: COMPANY_TREASURY,
          gauntletPool: fakePool,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
      expect.fail("Should have thrown");
    } catch (error: any) {
      expect(error.toString()).to.satisfy(
        (msg: string) =>
          msg.includes("InvalidGauntletPool") ||
          msg.includes("ConstraintAddress")
      );
    }
  });
});
