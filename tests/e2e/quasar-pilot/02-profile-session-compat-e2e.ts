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
import { createCollection, ruleSet } from "@metaplex-foundation/mpl-core";
import { generateSigner, publicKey } from "@metaplex-foundation/umi";
import {
  COMPANY_TREASURY,
  createProvider,
  createUmiContext,
  loadAllPrograms,
  loadWalletKeypair,
  PROGRAM_IDS,
  walletFromKeypair,
  anchor,
} from "../shared/setup";
import {
  getGeneratedMapPda,
  getGameStatePda,
  getGauntletPoolVaultPda,
  getInventoryPda,
  getMapConfigPda,
  getMapPoisPda,
  getMarketplaceConfigPda,
  getMintAuthorityPda,
  getPlayerProfilePda,
  getPlayerRelicPoolPda,
  getRelicAssetPda,
  getSessionCounterPda,
  getSessionDiscoveryPda,
  getSessionNoncesPda,
  getSessionPda,
} from "../shared/pda-helpers";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const WS_URL = process.env.ANCHOR_PROVIDER_WS;
const SPL_NOOP_PROGRAM_ID = new PublicKey("noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV");

const PLAYER_PROFILE_DISC = Buffer.from([82, 226, 99, 87, 164, 130, 181, 80]);
const PLAYER_RELIC_POOL_DISC = Buffer.from([1, 105, 67, 203, 111, 254, 159, 128]);
const MARKETPLACE_CONFIG_DISC = Buffer.from([169, 22, 247, 131, 182, 200, 81, 124]);

const IX = {
  initializeProfile: Buffer.from([32, 145, 77, 213, 58, 39, 251, 234]),
  initializeMarketplace: Buffer.from([47, 81, 64, 0, 96, 56, 105, 7]),
  mintNftItem: Buffer.from([225, 105, 7, 236, 107, 78, 104, 144]),
  setRelicActive: Buffer.from([59, 219, 164, 158, 201, 20, 227, 95]),
};

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

async function airdrop(connection: Connection, pubkey: PublicKey, sol = 5): Promise<void> {
  const sig = await connection.requestAirdrop(pubkey, sol * LAMPORTS_PER_SOL);
  const latest = await connection.getLatestBlockhash("confirmed");
  await connection.confirmTransaction({ signature: sig, ...latest }, "confirmed");
}

function createConnection(): Connection {
  return new Connection(
    RPC_URL,
    WS_URL ? { commitment: "confirmed", wsEndpoint: WS_URL } : "confirmed",
  );
}

async function sendTx(
  connection: Connection,
  payer: Keypair,
  instructions: TransactionInstruction[],
  extraSigners: Keypair[] = [],
): Promise<void> {
  const tx = new Transaction().add(
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    ...instructions,
  );
  await sendAndConfirmTransaction(connection, tx, [payer, ...extraSigners], {
    commitment: "confirmed",
    skipPreflight: false,
  });
}

function decodeProfile(data: Buffer) {
  expect(data.subarray(0, 8).equals(PLAYER_PROFILE_DISC)).to.equal(true);
  return {
    owner: new PublicKey(data.subarray(8, 40)),
    availableRuns: data.readUInt32LE(45),
    activeItemPool: Buffer.from(data.subarray(68, 78)),
  };
}

function decodeRelicPool(data: Buffer) {
  expect(data.subarray(0, 8).equals(PLAYER_RELIC_POOL_DISC)).to.equal(true);
  const itemId = Buffer.from(data.subarray(42, 50));
  return {
    owner: new PublicKey(data.subarray(8, 40)),
    count: data[40],
    firstOwnedCount: data.readUInt16LE(50),
    firstInActivePool: data[52] !== 0,
    firstItemId: itemId,
  };
}

function decodeMarketplaceConfig(data: Buffer) {
  expect(data.subarray(0, 8).equals(MARKETPLACE_CONFIG_DISC)).to.equal(true);
  return {
    itemsCollection: new PublicKey(data.subarray(72, 104)),
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

function mintNftItemIx(
  payer: PublicKey,
  asset: PublicKey,
  collection: PublicKey,
  marketplaceConfig: PublicKey,
  mintAuthority: PublicKey,
  relicAsset: PublicKey,
  playerRelicPool: PublicKey,
  owner: PublicKey,
  itemId: Buffer,
) {
  return quasarIx(
    PROGRAM_IDS.nftMarketplace,
    IX.mintNftItem,
    [
      meta(asset, true, true),
      meta(collection, false, true),
      meta(marketplaceConfig),
      meta(mintAuthority),
      meta(payer, true, true),
      meta(relicAsset, false, true),
      meta(playerRelicPool, false, true),
      meta(owner),
      meta(PROGRAM_IDS.mplCore),
      meta(PROGRAM_IDS.playerProfile),
      meta(SPL_NOOP_PROGRAM_ID),
      meta(SYSVAR_RENT_PUBKEY),
      meta(SystemProgram.programId),
    ],
    [
      itemId,
      encodeString("Compat Relic", 2),
      encodeString("https://arweave.net/compat-relic", 2),
    ],
  );
}

function setRelicActiveIx(
  owner: PublicKey,
  playerRelicPool: PublicKey,
  itemId: Buffer,
  active: boolean,
) {
  return quasarIx(
    PROGRAM_IDS.playerProfile,
    IX.setRelicActive,
    [meta(owner, true), meta(playerRelicPool, false, true)],
    [itemId, Buffer.from([active ? 1 : 0])],
  );
}

describe("Quasar profile downstream compatibility", function () {
  this.timeout(180_000);

  let connection: Connection;
  let admin: Keypair;
  const [sessionCounterPda] = getSessionCounterPda();
  const [mapConfigPda] = getMapConfigPda();
  const [marketplaceConfigPda] = getMarketplaceConfigPda();
  const [mintAuthorityPda] = getMintAuthorityPda();
  const [gauntletPoolPda] = getGauntletPoolVaultPda();

  before(async () => {
    connection = createConnection();
    admin = loadWalletKeypair();
    await airdrop(connection, admin.publicKey, 10);
  });

  async function ensureSessionCounter(programs: ReturnType<typeof loadAllPrograms>) {
    const info = await connection.getAccountInfo(sessionCounterPda, "confirmed");
    if (info !== null) {
      return;
    }
    await programs.sessionManager.methods
      .initializeCounter()
      .accounts({
        sessionCounter: sessionCounterPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();
  }

  async function ensureMapConfig(programs: ReturnType<typeof loadAllPrograms>) {
    const info = await connection.getAccountInfo(mapConfigPda, "confirmed");
    if (info !== null) {
      return;
    }
    await programs.mapGenerator.methods
      .initializeMapConfig()
      .accounts({
        mapConfig: mapConfigPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      } as any)
      .rpc();
  }

  async function ensureMarketplaceConfig(): Promise<PublicKey> {
    const info = await connection.getAccountInfo(marketplaceConfigPda, "confirmed");
    if (info !== null) {
      return decodeMarketplaceConfig(Buffer.from(info.data)).itemsCollection;
    }

    const { umi } = createUmiContext(RPC_URL, admin);
    const collectionSigner = generateSigner(umi);
    await createCollection(umi, {
      collection: collectionSigner,
      name: "Compat Item Collection",
      uri: "https://arweave.net/compat-items",
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
    const collection = new PublicKey(collectionSigner.publicKey.toString());

    await sendTx(connection, admin, [
      initializeMarketplaceIx(
        admin.publicKey,
        marketplaceConfigPda,
        gauntletPoolPda,
        collection,
        collection,
      ),
    ]);

    return collection;
  }

  it("starts an Anchor campaign session from Quasar profile and relic pool bytes", async () => {
    const provider = createProvider(RPC_URL, walletFromKeypair(admin));
    anchor.setProvider(provider);
    const programs = loadAllPrograms(provider);
    await ensureSessionCounter(programs);
    await ensureMapConfig(programs);
    const itemsCollection = await ensureMarketplaceConfig();

    const player = Keypair.generate();
    const sessionSigner = Keypair.generate();
    await airdrop(connection, player.publicKey, 5);
    await airdrop(connection, sessionSigner.publicKey, 5);

    const [playerProfilePda] = getPlayerProfilePda(player.publicKey);
    const [playerRelicPoolPda] = getPlayerRelicPoolPda(player.publicKey);
    const relicAsset = Keypair.generate();
    const [relicAssetPda] = getRelicAssetPda(relicAsset.publicKey);
    const relicItemId = Buffer.from("T08A0001", "utf8");

    await sendTx(connection, player, [
      initializeProfileIx(player.publicKey, playerProfilePda, "Compat Pilot"),
    ]);

    await sendTx(connection, admin, [
      mintNftItemIx(
        admin.publicKey,
        relicAsset.publicKey,
        itemsCollection,
        marketplaceConfigPda,
        mintAuthorityPda,
        relicAssetPda,
        playerRelicPoolPda,
        player.publicKey,
        relicItemId,
      ),
    ], [relicAsset]);

    await sendTx(connection, player, [
      setRelicActiveIx(player.publicKey, playerRelicPoolPda, relicItemId, true),
    ]);

    const profileBeforeInfo = await connection.getAccountInfo(playerProfilePda, "confirmed");
    expect(profileBeforeInfo).to.not.equal(null);
    const profileBefore = decodeProfile(Buffer.from(profileBeforeInfo!.data));
    expect(profileBefore.owner.toBase58()).to.equal(player.publicKey.toBase58());

    const relicPoolBeforeInfo = await connection.getAccountInfo(playerRelicPoolPda, "confirmed");
    expect(relicPoolBeforeInfo).to.not.equal(null);
    const relicPoolBefore = decodeRelicPool(Buffer.from(relicPoolBeforeInfo!.data));
    expect(relicPoolBefore.owner.toBase58()).to.equal(player.publicKey.toBase58());
    expect(relicPoolBefore.count).to.equal(1);
    expect(relicPoolBefore.firstOwnedCount).to.equal(1);
    expect(relicPoolBefore.firstInActivePool).to.equal(true);
    expect(relicPoolBefore.firstItemId.equals(relicItemId)).to.equal(true);

    const campaignLevel = 1;
    const [sessionNoncesPda] = getSessionNoncesPda(player.publicKey);
    const [sessionPda] = getSessionPda(player.publicKey, campaignLevel);
    const [gameStatePda] = getGameStatePda(sessionPda);
    const [generatedMapPda] = getGeneratedMapPda(sessionPda);
    const [inventoryPda] = getInventoryPda(sessionPda);
    const [mapPoisPda] = getMapPoisPda(sessionPda);
    const [sessionDiscoveryPda] = getSessionDiscoveryPda(sessionPda);

    await (programs.sessionManager.methods as any)
      .startSession(campaignLevel)
      .accounts({
        sessionNonces: sessionNoncesPda,
        gameSession: sessionPda,
        sessionCounter: sessionCounterPda,
        playerProfile: playerProfilePda,
        playerRelicPool: playerRelicPoolPda,
        player: player.publicKey,
        sessionSigner: sessionSigner.publicKey,
        generatedMap: generatedMapPda,
        sessionDiscovery: sessionDiscoveryPda,
        gameState: gameStatePda,
        mapPois: mapPoisPda,
        inventory: inventoryPda,
        mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
        gameplayStateProgram: PROGRAM_IDS.gameplayState,
        poiSystemProgram: PROGRAM_IDS.poiSystem,
        playerInventoryProgram: PROGRAM_IDS.playerInventory,
        playerProfileProgram: PROGRAM_IDS.playerProfile,
        systemProgram: SystemProgram.programId,
      } as any)
      .remainingAccounts([
        { pubkey: relicAsset.publicKey, isSigner: false, isWritable: false },
        { pubkey: relicAssetPda, isSigner: false, isWritable: false },
      ])
      .preInstructions([
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
      ])
      .signers([player, sessionSigner])
      .rpc();

    const profileAfterInfo = await connection.getAccountInfo(playerProfilePda, "confirmed");
    expect(profileAfterInfo).to.not.equal(null);
    const profileAfter = decodeProfile(Buffer.from(profileAfterInfo!.data));
    expect(profileAfter.availableRuns).to.equal(profileBefore.availableRuns - 1);
    expect(profileAfter.activeItemPool.equals(profileBefore.activeItemPool)).to.equal(true);

    const gameSession = await (programs.sessionManager.account as any).gameSession.fetch(sessionPda);
    expect(gameSession.player.toBase58()).to.equal(player.publicKey.toBase58());
    expect(gameSession.campaignLevel).to.equal(campaignLevel);
    expect(Buffer.from(gameSession.activeItemPool).equals(profileBefore.activeItemPool)).to.equal(true);
    expect(gameSession.activeRelicCount).to.equal(1);
    expect(Buffer.from(gameSession.activeRelics[0].itemId).equals(relicItemId)).to.equal(true);
  });
});
