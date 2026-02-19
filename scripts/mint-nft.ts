/**
 * Mints a skin or NFT item to a specified wallet address.
 *
 * Calls your nft-marketplace Anchor program which does the CPI to Metaplex Core on-chain.
 *
 * Usage:
 *   # Mint a skin
 *   MINT_TYPE=skin MINT_NAME="Golden Mole" anchor run mint-nft
 *
 *   # Mint a skin to a specific wallet
 *   MINT_TYPE=skin MINT_NAME="Golden Mole" OWNER=<pubkey> anchor run mint-nft
 *
 *   # Mint an NFT item
 *   MINT_TYPE=item MINT_NAME="Infernal Pickaxe" NFT_ITEM_ID="S-XX-01" anchor run mint-nft
 *
 * Environment variables:
 *   MINT_TYPE     - "skin" or "item" (required)
 *   MINT_NAME     - Name for the NFT (required)
 *   MINT_URI      - Metadata URI (default: placeholder)
 *   OWNER         - Wallet to receive the NFT (default: your wallet)
 *   NFT_ITEM_ID   - Item ID for NFT items, e.g. "S-XX-01" (required for items)
 */

import * as anchor from "@coral-xyz/anchor";
import * as fs from "fs";
import * as os from "os";

// ── Constants ────────────────────────────────────────────────────────────
const NFT_MARKETPLACE_PROGRAM_ID = new anchor.web3.PublicKey(
  "8gZC4WcbiC3ZSGEYMvruFRPcY1JJyZnLHFJSQnigGEEw"
);

const MPL_CORE_PROGRAM_ID = new anchor.web3.PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
);

// Derive PDAs
const [marketplaceConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("marketplace_config")],
  NFT_MARKETPLACE_PROGRAM_ID
);

const [mintAuthorityPda] = anchor.web3.PublicKey.findProgramAddressSync(
  [Buffer.from("mint_authority")],
  NFT_MARKETPLACE_PROGRAM_ID
);

async function main() {
  // ── Parse env ─────────────────────────────────────────────────────
  const mintType = process.env.MINT_TYPE;
  if (!mintType || !["skin", "item"].includes(mintType)) {
    console.error("Error: MINT_TYPE must be 'skin' or 'item'");
    console.error("Example: MINT_TYPE=skin MINT_NAME=\"Test Skin\" anchor run mint-nft");
    process.exit(1);
  }

  const mintName = process.env.MINT_NAME;
  if (!mintName) {
    console.error("Error: MINT_NAME is required");
    process.exit(1);
  }

  const mintUri = process.env.MINT_URI || "https://arweave.net/placeholder";

  const walletPath =
    process.env.ANCHOR_WALLET ||
    `${os.homedir()}/.config/solana/id.json`;
  const rpcUrl =
    process.env.ANCHOR_PROVIDER_URL || "https://api.devnet.solana.com";

  // Load keypair
  const walletJson = JSON.parse(fs.readFileSync(walletPath, "utf-8"));
  const keypair = anchor.web3.Keypair.fromSecretKey(
    Uint8Array.from(walletJson)
  );

  const ownerPubkey = process.env.OWNER
    ? new anchor.web3.PublicKey(process.env.OWNER)
    : keypair.publicKey;

  console.log(`=== Mint NFT (${mintType}) ===\n`);
  console.log("RPC:", rpcUrl);
  console.log("Payer:", keypair.publicKey.toBase58());
  console.log("Owner:", ownerPubkey.toBase58());
  console.log("Name:", mintName);
  console.log("URI:", mintUri);
  console.log();

  // Set up Anchor
  const connection = new anchor.web3.Connection(rpcUrl, "confirmed");
  const wallet = new anchor.Wallet(keypair);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });

  const idlPath = `${__dirname}/../target/idl/nft_marketplace.json`;
  const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
  const program = new anchor.Program(idl, provider);

  // Fetch marketplace config to get collection addresses
  const config = await (program.account as any).marketplaceConfig.fetch(
    marketplaceConfigPda
  );

  // Generate new keypair for the asset account
  const assetKeypair = anchor.web3.Keypair.generate();

  if (mintType === "skin") {
    console.log("Collection:", config.skinsCollection.toBase58());
    console.log("\nMinting skin...");

    // _skin_id, _season, _rarity are unused on-chain (prefixed with _)
    await program.methods
      .mintSkin(mintName, mintUri, 0, 0, 0)
      .accounts({
        asset: assetKeypair.publicKey,
        collection: config.skinsCollection,
        marketplaceConfig: marketplaceConfigPda,
        mintAuthority: mintAuthorityPda,
        payer: keypair.publicKey,
        owner: ownerPubkey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([assetKeypair])
      .rpc();

    console.log("  Skin minted!");
    console.log("  Asset:", assetKeypair.publicKey.toBase58());
  } else {
    // item
    const nftItemId = process.env.NFT_ITEM_ID;
    if (!nftItemId) {
      console.error("Error: NFT_ITEM_ID is required for item mints");
      console.error("Valid IDs: S-XX-01, S-XX-02, S-XX-03, S-XX-04, S-XX-05, S-XX-06");
      process.exit(1);
    }

    // Pad to 8 bytes (null-terminated)
    const idBytes = Buffer.alloc(8, 0);
    idBytes.write(nftItemId, 0, "utf-8");

    console.log("Collection:", config.itemsCollection.toBase58());
    console.log("NFT Item ID:", nftItemId);
    console.log("\nMinting NFT item...");

    await program.methods
      .mintNftItem(mintName, mintUri, Array.from(idBytes))
      .accounts({
        asset: assetKeypair.publicKey,
        collection: config.itemsCollection,
        marketplaceConfig: marketplaceConfigPda,
        mintAuthority: mintAuthorityPda,
        payer: keypair.publicKey,
        owner: ownerPubkey,
        mplCoreProgram: MPL_CORE_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([assetKeypair])
      .rpc();

    console.log("  NFT item minted!");
    console.log("  Asset:", assetKeypair.publicKey.toBase58());
  }

  console.log("\nDone! The NFT is now owned by:", ownerPubkey.toBase58());
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.error("Error:", e.message || e);
    process.exit(1);
  });
