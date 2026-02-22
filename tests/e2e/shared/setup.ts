import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import {
  createSignerFromKeypair,
  keypairIdentity,
} from "@metaplex-foundation/umi";
import { fromWeb3JsKeypair } from "@metaplex-foundation/umi-web3js-adapters";
import * as fs from "fs";
import * as path from "path";

// ── Program IDs ─────────────────────────────────────────────────────────────
export const PROGRAM_IDS = {
  playerProfile: new PublicKey("Ch3bbL1oQk2z5rX1jiun3KuSWZqnXZ1MnrfrtKj4MKun"),
  sessionManager: new PublicKey("6w1XVMSTRmZU9AWCKVvKohGAHSFMENhda7vqhKPQ8TPn"),
  mapGenerator: new PublicKey("GCy5GqvnJN99rgGtV6fMn8NtL9E7RoAyHDGzQv8me65j"),
  gameplayState: new PublicKey("C8hK4qsqsSYQeqyXuTPTUUS3T7N74WnZCuzvChTpK1Mo"),
  playerInventory: new PublicKey("APRnvp41jEYnT1EnrdBTim7bodqE6v2RSgzv1CG7Qv7u"),
  poiSystem: new PublicKey("KiT25b86BSAF8yErcWwyuuWNaoXMpNf859NjH41TpSj"),
  nftMarketplace: new PublicKey("ApUAEEKYsRMjxoMA65WV2xiG8xGwWzFhHjTMGGefcumK"),
  mplCore: new PublicKey("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"),
  delegation: new PublicKey("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh"),
};

export const COMPANY_TREASURY = new PublicKey(
  "5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8"
);

// ── IDL Loading ─────────────────────────────────────────────────────────────
const IDL_DIR = path.resolve(__dirname, "../../../target/idl");

export function loadIdl(name: string): any {
  const idlPath = path.join(IDL_DIR, `${name}.json`);
  return JSON.parse(fs.readFileSync(idlPath, "utf-8"));
}

export function loadProgram(
  idlName: string,
  provider: anchor.AnchorProvider
): Program {
  const idl = loadIdl(idlName);
  return new Program(idl, provider);
}

// ── Provider & Connection Setup ─────────────────────────────────────────────
export function createProvider(
  rpcUrl: string,
  wallet: anchor.Wallet
): anchor.AnchorProvider {
  const connection = new Connection(rpcUrl, "confirmed");
  return new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
}

export function walletFromKeypair(kp: Keypair): anchor.Wallet {
  return new anchor.Wallet(kp);
}

// ── Airdrop ─────────────────────────────────────────────────────────────────
export async function airdropAndConfirm(
  connection: Connection,
  pubkey: PublicKey,
  lamports: number = 2 * LAMPORTS_PER_SOL
): Promise<void> {
  const sig = await connection.requestAirdrop(pubkey, lamports);
  const latest = await connection.getLatestBlockhash("confirmed");
  await connection.confirmTransaction(
    { signature: sig, ...latest },
    "confirmed"
  );
}

// ── UMI Context ─────────────────────────────────────────────────────────────
export function createUmiContext(rpcUrl: string, walletKeypair: Keypair) {
  const umi = createUmi(rpcUrl);
  const umiKeypair = fromWeb3JsKeypair(walletKeypair);
  const signer = createSignerFromKeypair(umi, umiKeypair);
  umi.use(keypairIdentity(signer));
  return { umi, signer };
}

// ── Wallet Loading ──────────────────────────────────────────────────────────
export function loadWalletKeypair(
  walletPath?: string
): Keypair {
  const resolvedPath =
    walletPath ||
    process.env.ANCHOR_WALLET ||
    `${require("os").homedir()}/.config/solana/id.json`;
  const walletJson = JSON.parse(fs.readFileSync(resolvedPath, "utf-8"));
  return Keypair.fromSecretKey(Uint8Array.from(walletJson));
}

// ── All Programs Loader ─────────────────────────────────────────────────────
export interface AllPrograms {
  playerProfile: Program;
  sessionManager: Program;
  mapGenerator: Program;
  gameplayState: Program;
  playerInventory: Program;
  poiSystem: Program;
  nftMarketplace: Program;
}

export function loadAllPrograms(provider: anchor.AnchorProvider): AllPrograms {
  return {
    playerProfile: loadProgram("player_profile", provider),
    sessionManager: loadProgram("session_manager", provider),
    mapGenerator: loadProgram("map_generator", provider),
    gameplayState: loadProgram("gameplay_state", provider),
    playerInventory: loadProgram("player_inventory", provider),
    poiSystem: loadProgram("poi_system", provider),
    nftMarketplace: loadProgram("nft_marketplace", provider),
  };
}

// ── Re-exports ──────────────────────────────────────────────────────────────
export { anchor, Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Connection };
