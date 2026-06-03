import { expect } from "chai";
import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  sendAndConfirmTransaction,
  SYSVAR_RENT_PUBKEY,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  createCollection,
  ruleSet,
} from "@metaplex-foundation/mpl-core";
import {
  generateSigner,
  publicKey,
} from "@metaplex-foundation/umi";
import {
  COMPANY_TREASURY,
  createUmiContext,
  loadWalletKeypair,
  PROGRAM_IDS,
} from "../shared/setup";
import {
  getGauntletPoolVaultPda,
  getListingPda,
  getMarketplaceConfigPda,
  getMintAuthorityPda,
  getPlayerProfilePda,
  getPlayerRelicPoolPda,
} from "../shared/pda-helpers";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const WS_URL = process.env.ANCHOR_PROVIDER_WS;
const SPL_NOOP_PROGRAM_ID = new PublicKey("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

const PLAYER_PROFILE_DISC = Buffer.from([82, 226, 99, 87, 164, 130, 181, 80]);
const MARKETPLACE_CONFIG_DISC = Buffer.from([169, 22, 247, 131, 182, 200, 81, 124]);
const LISTING_DISC = Buffer.from([218, 32, 50, 73, 43, 134, 26, 58]);

const IX = {
  initializeProfile: Buffer.from([32, 145, 77, 213, 58, 39, 251, 234]),
  updateProfileName: Buffer.from([96, 69, 10, 229, 192, 184, 200, 20]),
  initializeMarketplace: Buffer.from([47, 81, 64, 0, 96, 56, 105, 7]),
  mintSkin: Buffer.from([142, 213, 165, 190, 25, 244, 82, 176]),
  listNft: Buffer.from([88, 221, 93, 166, 63, 220, 106, 232]),
  cancelListing: Buffer.from([41, 183, 50, 232, 230, 233, 157, 70]),
  buyNft: Buffer.from([96, 0, 28, 190, 49, 107, 83, 222]),
};

type CuSample = {
  label: string;
  units: number | null;
};

const cuSamples: CuSample[] = [];

function meta(pubkey: PublicKey, isSigner = false, isWritable = false) {
  return { pubkey, isSigner, isWritable };
}

function quasarIx(
  programId: PublicKey,
  discriminator: Buffer,
  keys: TransactionInstruction["keys"],
  data: Buffer[] = [],
): TransactionInstruction {
  return new TransactionInstruction({
    programId,
    keys,
    data: Buffer.concat([discriminator, ...data]),
  });
}

function encodeString(value: string, prefixBytes = 1): Buffer {
  const bytes = Buffer.from(value, "utf8");
  if (prefixBytes === 1) {
    return Buffer.concat([Buffer.from([bytes.length]), bytes]);
  }
  if (prefixBytes === 2) {
    const length = Buffer.alloc(2);
    length.writeUInt16LE(bytes.length);
    return Buffer.concat([length, bytes]);
  }
  throw new Error(`Unsupported string prefix width: ${prefixBytes}`);
}

function encodeU16(value: number): Buffer {
  const out = Buffer.alloc(2);
  out.writeUInt16LE(value);
  return out;
}

function encodeU64(value: bigint | number): Buffer {
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(BigInt(value));
  return out;
}

async function airdrop(connection: Connection, pubkey: PublicKey, sol = 5): Promise<void> {
  const sig = await connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
  const latest = await connection.getLatestBlockhash("confirmed");
  await connection.confirmTransaction({ signature: sig, ...latest }, "confirmed");
}

function createConnection(): Connection {
  return new Connection(
    RPC_URL,
    WS_URL
      ? { commitment: "confirmed", wsEndpoint: WS_URL }
      : "confirmed",
  );
}

async function confirmedAccount(connection: Connection, address: PublicKey) {
  return connection.getAccountInfo(address, "confirmed");
}

async function sendTx(
  connection: Connection,
  label: string,
  payer: Keypair,
  instructions: TransactionInstruction[],
  extraSigners: Keypair[] = [],
): Promise<string> {
  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    ...instructions,
  );
  const signature = await sendAndConfirmTransaction(connection, tx, [payer, ...extraSigners], {
    commitment: "confirmed",
    skipPreflight: false,
  });
  const txMeta = await connection.getTransaction(signature, {
    commitment: "confirmed",
    maxSupportedTransactionVersion: 0,
  });
  const units = txMeta?.meta?.computeUnitsConsumed ?? null;
  cuSamples.push({ label, units });
  console.log(`[quasar-cu] ${label}=${units ?? "unavailable"} sig=${signature}`);
  return signature;
}

function decodeProfile(data: Buffer) {
  expect(data.subarray(0, 8).equals(PLAYER_PROFILE_DISC)).to.equal(true);
  const nameLen = Math.min(data[112] ?? 0, 32);
  return {
    owner: new PublicKey(data.subarray(8, 40)),
    totalRuns: data.readUInt32LE(40),
    highestLevelUnlocked: data[44],
    availableRuns: data.readUInt32LE(45),
    name: data.subarray(113, 113 + nameLen).toString("utf8"),
  };
}

function decodeMarketplaceConfig(data: Buffer) {
  expect(data.subarray(0, 8).equals(MARKETPLACE_CONFIG_DISC)).to.equal(true);
  return {
    authority: new PublicKey(data.subarray(8, 40)),
    skinsCollection: new PublicKey(data.subarray(40, 72)),
    itemsCollection: new PublicKey(data.subarray(72, 104)),
    companyTreasury: new PublicKey(data.subarray(104, 136)),
    gauntletPool: new PublicKey(data.subarray(136, 168)),
    companyFeeBps: data.readUInt16LE(168),
    gauntletFeeBps: data.readUInt16LE(170),
  };
}

function decodeListing(data: Buffer) {
  expect(data.subarray(0, 8).equals(LISTING_DISC)).to.equal(true);
  return {
    seller: new PublicKey(data.subarray(8, 40)),
    asset: new PublicKey(data.subarray(40, 72)),
    collection: new PublicKey(data.subarray(72, 104)),
    priceLamports: data.readBigUInt64LE(104),
  };
}

function initializeProfileIx(owner: PublicKey, profile: PublicKey, name: string) {
  return quasarIx(
    PROGRAM_IDS.playerProfile,
    IX.initializeProfile,
    [
      meta(owner, true, true),
      meta(profile, false, true),
      meta(SYSVAR_RENT_PUBKEY),
      meta(SystemProgram.programId),
    ],
    [encodeString(name)],
  );
}

function updateProfileNameIx(owner: PublicKey, profile: PublicKey, name: string) {
  return quasarIx(
    PROGRAM_IDS.playerProfile,
    IX.updateProfileName,
    [meta(owner, true), meta(profile, false, true)],
    [encodeString(name)],
  );
}

function initializeMarketplaceIx(
  authority: PublicKey,
  marketplaceConfig: PublicKey,
  gauntletPool: PublicKey,
  skinsCollection: PublicKey,
  itemsCollection: PublicKey,
) {
  return quasarIx(
    PROGRAM_IDS.nftMarketplace,
    IX.initializeMarketplace,
    [
      meta(authority, true, true),
      meta(marketplaceConfig, false, true),
      meta(gauntletPool),
      meta(SYSVAR_RENT_PUBKEY),
      meta(SystemProgram.programId),
    ],
    [skinsCollection.toBuffer(), itemsCollection.toBuffer()],
  );
}

function mintSkinIx(
  payer: PublicKey,
  asset: PublicKey,
  owner: PublicKey,
  collection: PublicKey,
  marketplaceConfig: PublicKey,
  mintAuthority: PublicKey,
) {
  return quasarIx(
    PROGRAM_IDS.nftMarketplace,
    IX.mintSkin,
    [
      meta(asset, true, true),
      meta(collection, false, true),
      meta(marketplaceConfig),
      meta(mintAuthority),
      meta(payer, true, true),
      meta(owner),
      meta(PROGRAM_IDS.mplCore),
      meta(SPL_NOOP_PROGRAM_ID),
      meta(SystemProgram.programId),
    ],
    [
      encodeU16(1),
      Buffer.from([0]),
      Buffer.from([0]),
      encodeString("Quasar Pilot Skin", 2),
      encodeString("https://arweave.net/quasar-pilot-skin", 2),
    ],
  );
}

function listNftIx(
  seller: PublicKey,
  listing: PublicKey,
  asset: PublicKey,
  collection: PublicKey,
  marketplaceConfig: PublicKey,
  mintAuthority: PublicKey,
  playerProfile: PublicKey,
  priceLamports: bigint,
) {
  return quasarIx(
    PROGRAM_IDS.nftMarketplace,
    IX.listNft,
    [
      meta(listing, false, true),
      meta(marketplaceConfig),
      meta(mintAuthority),
      meta(asset, false, true),
      meta(collection, false, true),
      meta(seller, true, true),
      meta(playerProfile),
      meta(PROGRAM_IDS.mplCore),
      meta(SYSVAR_RENT_PUBKEY),
      meta(SystemProgram.programId),
    ],
    [encodeU64(priceLamports)],
  );
}

function cancelListingIx(seller: PublicKey, listing: PublicKey, asset: PublicKey, collection: PublicKey) {
  return quasarIx(
    PROGRAM_IDS.nftMarketplace,
    IX.cancelListing,
    [
      meta(listing, false, true),
      meta(asset, false, true),
      meta(collection, false, true),
      meta(seller, true, true),
      meta(PROGRAM_IDS.mplCore),
      meta(SystemProgram.programId),
    ],
  );
}

function buyNftIx(
  buyer: PublicKey,
  seller: PublicKey,
  listing: PublicKey,
  asset: PublicKey,
  collection: PublicKey,
  marketplaceConfig: PublicKey,
  mintAuthority: PublicKey,
  buyerRelicPool: PublicKey,
  companyTreasury: PublicKey,
  gauntletPool: PublicKey,
) {
  return quasarIx(
    PROGRAM_IDS.nftMarketplace,
    IX.buyNft,
    [
      meta(listing, false, true),
      meta(marketplaceConfig),
      meta(mintAuthority),
      meta(asset, false, true),
      meta(PROGRAM_IDS.nftMarketplace),
      meta(collection, false, true),
      meta(buyer, true, true),
      meta(seller, false, true),
      meta(PROGRAM_IDS.nftMarketplace, false, true),
      meta(buyerRelicPool, false, true),
      meta(companyTreasury, false, true),
      meta(gauntletPool, false, true),
      meta(PROGRAM_IDS.mplCore),
      meta(PROGRAM_IDS.playerProfile),
      meta(SYSVAR_RENT_PUBKEY),
      meta(SystemProgram.programId),
    ],
  );
}

describe("Quasar pilot E2E", function () {
  this.timeout(180_000);

  let connection: Connection;
  let admin: Keypair;
  let seller: Keypair;
  let buyer: Keypair;
  let skinsCollection: PublicKey;
  const [marketplaceConfigPda] = getMarketplaceConfigPda();
  const [mintAuthorityPda] = getMintAuthorityPda();
  const [gauntletPoolPda] = getGauntletPoolVaultPda();

  before(async () => {
    connection = createConnection();
    admin = loadWalletKeypair();
    seller = Keypair.generate();
    buyer = Keypair.generate();
    await airdrop(connection, admin.publicKey, 10);
    await airdrop(connection, seller.publicKey, 10);
    await airdrop(connection, buyer.publicKey, 10);
  });

  after(() => {
    for (const sample of cuSamples) {
      console.log(`[quasar-cu-summary] ${sample.label}=${sample.units ?? "unavailable"}`);
    }
  });

  it("creates and updates a player profile using confirmed reads", async () => {
    const [profilePda] = getPlayerProfilePda(seller.publicKey);

    await sendTx(connection, "player_profile.initialize_profile", seller, [
      initializeProfileIx(seller.publicKey, profilePda, "Pilot Seller"),
    ]);

    const createdInfo = await confirmedAccount(connection, profilePda);
    expect(createdInfo).to.not.equal(null);
    let profile = decodeProfile(Buffer.from(createdInfo!.data));
    expect(profile.owner.toBase58()).to.equal(seller.publicKey.toBase58());
    expect(profile.availableRuns).to.equal(20);
    expect(profile.name).to.equal("Pilot Seller");

    await sendTx(connection, "player_profile.update_profile_name", seller, [
      updateProfileNameIx(seller.publicKey, profilePda, "Pilot Renamed"),
    ]);

    const updatedInfo = await confirmedAccount(connection, profilePda);
    expect(updatedInfo).to.not.equal(null);
    profile = decodeProfile(Buffer.from(updatedInfo!.data));
    expect(profile.name).to.equal("Pilot Renamed");
  });

  it("lists, cancels, re-lists, and buys a Metaplex Core skin through marketplace CPIs", async () => {
    const { umi } = createUmiContext(RPC_URL, admin);
    const collectionSigner = generateSigner(umi);
    await createCollection(umi, {
      collection: collectionSigner,
      name: "Quasar Pilot Skins",
      uri: "https://arweave.net/quasar-pilot-skins",
      updateAuthority: publicKey(mintAuthorityPda.toBase58()),
      plugins: [
        {
          type: "Royalties",
          basisPoints: 500,
          creators: [
            { address: publicKey(COMPANY_TREASURY.toBase58()), percentage: 60 },
            { address: publicKey(gauntletPoolPda.toBase58()), percentage: 40 },
          ],
          ruleSet: ruleSet("None"),
        },
      ],
    }).sendAndConfirm(umi);
    skinsCollection = new PublicKey(collectionSigner.publicKey.toString());

    const configInfo = await confirmedAccount(connection, marketplaceConfigPda);
    if (configInfo === null) {
      await sendTx(connection, "nft_marketplace.initialize_marketplace", admin, [
        initializeMarketplaceIx(
          admin.publicKey,
          marketplaceConfigPda,
          gauntletPoolPda,
          skinsCollection,
          skinsCollection,
        ),
      ]);
    }

    const finalConfigInfo = await confirmedAccount(connection, marketplaceConfigPda);
    expect(finalConfigInfo).to.not.equal(null);
    const config = decodeMarketplaceConfig(Buffer.from(finalConfigInfo!.data));
    skinsCollection = config.skinsCollection;
    expect(config.authority.toBase58()).to.equal(admin.publicKey.toBase58());
    expect(config.companyTreasury.toBase58()).to.equal(COMPANY_TREASURY.toBase58());
    expect(config.gauntletPool.toBase58()).to.equal(gauntletPoolPda.toBase58());

    const asset = Keypair.generate();
    await sendTx(connection, "nft_marketplace.mint_skin", admin, [
      mintSkinIx(
        admin.publicKey,
        asset.publicKey,
        seller.publicKey,
        skinsCollection,
        marketplaceConfigPda,
        mintAuthorityPda,
      ),
    ], [asset]);

    const assetInfo = await confirmedAccount(connection, asset.publicKey);
    expect(assetInfo).to.not.equal(null);
    expect(assetInfo!.owner.toBase58()).to.equal(PROGRAM_IDS.mplCore.toBase58());

    const [sellerProfilePda] = getPlayerProfilePda(seller.publicKey);
    const [listingPda] = getListingPda(asset.publicKey);
    const price = 1_000_000_000n;

    await sendTx(connection, "nft_marketplace.list_nft", seller, [
      listNftIx(
        seller.publicKey,
        listingPda,
        asset.publicKey,
        skinsCollection,
        marketplaceConfigPda,
        mintAuthorityPda,
        sellerProfilePda,
        price,
      ),
    ]);

    const listedInfo = await confirmedAccount(connection, listingPda);
    expect(listedInfo).to.not.equal(null);
    let listing = decodeListing(Buffer.from(listedInfo!.data));
    expect(listing.seller.toBase58()).to.equal(seller.publicKey.toBase58());
    expect(listing.asset.toBase58()).to.equal(asset.publicKey.toBase58());
    expect(listing.priceLamports).to.equal(price);

    await sendTx(connection, "nft_marketplace.cancel_listing", seller, [
      cancelListingIx(seller.publicKey, listingPda, asset.publicKey, skinsCollection),
    ]);
    expect(await confirmedAccount(connection, listingPda)).to.equal(null);

    await sendTx(connection, "nft_marketplace.list_nft.relist", seller, [
      listNftIx(
        seller.publicKey,
        listingPda,
        asset.publicKey,
        skinsCollection,
        marketplaceConfigPda,
        mintAuthorityPda,
        sellerProfilePda,
        price,
      ),
    ]);
    listing = decodeListing(Buffer.from((await confirmedAccount(connection, listingPda))!.data));
    expect(listing.priceLamports).to.equal(price);

    const [buyerRelicPoolPda] = getPlayerRelicPoolPda(buyer.publicKey);
    const sellerBefore = await connection.getBalance(seller.publicKey, "confirmed");
    await sendTx(connection, "nft_marketplace.buy_nft", buyer, [
      buyNftIx(
        buyer.publicKey,
        seller.publicKey,
        listingPda,
        asset.publicKey,
        skinsCollection,
        marketplaceConfigPda,
        mintAuthorityPda,
        buyerRelicPoolPda,
        COMPANY_TREASURY,
        gauntletPoolPda,
      ),
    ]);

    expect(await confirmedAccount(connection, listingPda)).to.equal(null);
    const sellerAfter = await connection.getBalance(seller.publicKey, "confirmed");
    expect(sellerAfter - sellerBefore).to.be.greaterThan(900_000_000);
  });
});
