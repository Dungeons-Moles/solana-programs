/**
 * Creates the two Metaplex Core collections and initializes the NFT marketplace.
 *
 * Steps:
 *   1. Create "Dungeons & Moles Skins" collection
 *   2. Create "Dungeons & Moles NFT Items" collection
 *   3. Initialize marketplace config with both collection addresses
 *
 * Both collections use:
 *   - Royalty plugin: 500 bps (5%), split 60% company treasury / 40% gauntlet pool
 *   - Update authority: nft-marketplace mint_authority PDA
 *
 * Usage:
 *   ANCHOR_PROVIDER_URL=https://api.devnet.solana.com \
 *   ANCHOR_WALLET=~/.config/solana/id.json \
 *   yarn ts-node scripts/init-collections.ts
 *
 * Prerequisites:
 *   yarn add @metaplex-foundation/mpl-core @metaplex-foundation/umi \
 *     @metaplex-foundation/umi-bundle-defaults @metaplex-foundation/umi-web3js-adapters
 */

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
import * as anchor from "@coral-xyz/anchor";
import * as fs from "fs";
import * as os from "os";

// ── Constants ────────────────────────────────────────────────────────────
const NFT_MARKETPLACE_PROGRAM_ID = new anchor.web3.PublicKey(
  "8gZC4WcbiC3ZSGEYMvruFRPcY1JJyZnLHFJSQnigGEEw"
);

const COMPANY_TREASURY = new anchor.web3.PublicKey(
  "5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8"
);

// Derive the gameplay-state gauntlet_pool_vault PDA
const GAMEPLAY_STATE_PROGRAM_ID = new anchor.web3.PublicKey(
  "5VAaGSSoBP4UEt3RL2EXvDwpeDxAXMndsJn7QX96nc4n"
);

const [gauntletPoolVault] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("gauntlet_pool_vault")],
  GAMEPLAY_STATE_PROGRAM_ID
);

// Derive the mint_authority PDA from nft-marketplace
const [mintAuthorityPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("mint_authority")],
  NFT_MARKETPLACE_PROGRAM_ID
);

// Derive the marketplace_config PDA
const [marketplaceConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("marketplace_config")],
  NFT_MARKETPLACE_PROGRAM_ID
);

// Royalties: 500 bps = 5% total
const ROYALTY_BPS = 500;

async function main() {
  console.log("=== Dungeons & Moles Collection Init + Marketplace Setup ===\n");

  // Load wallet from env or default path
  const walletPath =
    process.env.ANCHOR_WALLET ||
    `${os.homedir()}/.config/solana/id.json`;
  const rpcUrl =
    process.env.ANCHOR_PROVIDER_URL || "https://api.devnet.solana.com";

  console.log("RPC:", rpcUrl);
  console.log("Wallet:", walletPath);
  console.log("NFT Marketplace Program:", NFT_MARKETPLACE_PROGRAM_ID.toBase58());
  console.log("Mint Authority PDA:", mintAuthorityPda.toBase58());
  console.log("Company Treasury:", COMPANY_TREASURY.toBase58());
  console.log("Gauntlet Pool:", gauntletPoolVault.toBase58());
  console.log();

  // Load keypair
  const walletJson = JSON.parse(fs.readFileSync(walletPath, "utf-8"));
  const keypair = anchor.web3.Keypair.fromSecretKey(
    Uint8Array.from(walletJson)
  );

  // Set up Umi (for collection creation)
  const umi = createUmi(rpcUrl);
  const umiKeypair = fromWeb3JsKeypair(keypair);
  const signer = createSignerFromKeypair(umi, umiKeypair);
  umi.use(keypairIdentity(signer));

  const mintAuthority = publicKey(mintAuthorityPda.toBase58());
  const treasury = publicKey(COMPANY_TREASURY.toBase58());
  const gauntletPool = publicKey(gauntletPoolVault.toBase58());

  // ── 1. Create Skins Collection ─────────────────────────────────────
  console.log("Creating Skins Collection...");
  const skinsCollection = generateSigner(umi);

  try {
    await createCollection(umi, {
      collection: skinsCollection,
      name: "Dungeons & Moles Skins",
      uri: "https://arweave.net/skins-collection-metadata", // TODO: replace with actual URI
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

    console.log(
      "  Skins Collection created:",
      skinsCollection.publicKey.toString()
    );
  } catch (e: any) {
    console.error("  Failed to create skins collection:", e.message || e);
    process.exit(1);
  }

  // ── 2. Create NFT Items Collection ────────────────────────────────
  console.log("Creating NFT Items Collection...");
  const itemsCollection = generateSigner(umi);

  try {
    await createCollection(umi, {
      collection: itemsCollection,
      name: "Dungeons & Moles NFT Items",
      uri: "https://arweave.net/items-collection-metadata", // TODO: replace with actual URI
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

    console.log(
      "  NFT Items Collection created:",
      itemsCollection.publicKey.toString()
    );
  } catch (e: any) {
    console.error("  Failed to create NFT items collection:", e.message || e);
    process.exit(1);
  }

  // ── 3. Initialize Marketplace ─────────────────────────────────────
  console.log("\nInitializing Marketplace...");

  const connection = new anchor.web3.Connection(rpcUrl, "confirmed");
  const wallet = new anchor.Wallet(keypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });

  // Load IDL
  const idlPath = `${__dirname}/../target/idl/nft_marketplace.json`;
  const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
  const program = new anchor.Program(idl, provider);

  try {
    const skinsCollectionPubkey = new anchor.web3.PublicKey(
      skinsCollection.publicKey.toString()
    );
    const itemsCollectionPubkey = new anchor.web3.PublicKey(
      itemsCollection.publicKey.toString()
    );

    await program.methods
      .initializeMarketplace(skinsCollectionPubkey, itemsCollectionPubkey)
      .accounts({
        marketplaceConfig: marketplaceConfigPda,
        authority: keypair.publicKey,
        gauntletPool: gauntletPoolVault,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("  Marketplace initialized!");
    console.log("  Config PDA:", marketplaceConfigPda.toBase58());
  } catch (e: any) {
    if (e.message?.includes("already in use")) {
      console.log("  Marketplace already initialized (config account exists).");
    } else {
      console.error("  Failed to initialize marketplace:", e.message || e);
      process.exit(1);
    }
  }

  // ── Summary ────────────────────────────────────────────────────────
  console.log("\n=== Summary ===");
  console.log("Skins Collection:", skinsCollection.publicKey.toString());
  console.log("NFT Items Collection:", itemsCollection.publicKey.toString());
  console.log("Marketplace Config:", marketplaceConfigPda.toBase58());
  console.log("\nYou can now mint skins and NFT items using: anchor run mint-nft");
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });
