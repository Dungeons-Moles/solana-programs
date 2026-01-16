import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PlayerProfile } from "../target/types/player_profile";
import { SessionManager } from "../target/types/session_manager";
import { MapGenerator } from "../target/types/map_generator";

module.exports = async function (provider: anchor.AnchorProvider) {
  anchor.setProvider(provider);

  const playerProfileProgram = anchor.workspace
    .PlayerProfile as Program<PlayerProfile>;
  const sessionManagerProgram = anchor.workspace
    .SessionManager as Program<SessionManager>;
  const mapGeneratorProgram = anchor.workspace
    .MapGenerator as Program<MapGenerator>;

  const admin = provider.wallet.publicKey;

  const [treasuryPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("treasury")],
    playerProfileProgram.programId
  );
  const [counterPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("session_counter")],
    sessionManagerProgram.programId
  );
  const [mapConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("map_config")],
    mapGeneratorProgram.programId
  );

  await initializeTreasury(playerProfileProgram, treasuryPda, admin);
  await initializeSessionCounter(sessionManagerProgram, counterPda, admin);
  await initializeMapConfig(mapGeneratorProgram, mapConfigPda, admin);
};

async function initializeTreasury(
  program: Program<PlayerProfile>,
  treasury: anchor.web3.PublicKey,
  admin: anchor.web3.PublicKey
) {
  try {
    await program.methods
      .initializeTreasury()
      .accounts({
        treasury,
        admin,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("Treasury initialized:", treasury.toBase58());
  } catch (error: any) {
    if (!error.toString().includes("already in use")) {
      throw error;
    }
    console.log("Treasury already exists:", treasury.toBase58());
  }
}

async function initializeSessionCounter(
  program: Program<SessionManager>,
  sessionCounter: anchor.web3.PublicKey,
  admin: anchor.web3.PublicKey
) {
  try {
    await program.methods
      .initializeCounter()
      .accounts({
        sessionCounter,
        admin,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("Session counter initialized:", sessionCounter.toBase58());
  } catch (error: any) {
    if (!error.toString().includes("already in use")) {
      throw error;
    }
    console.log("Session counter already exists:", sessionCounter.toBase58());
  }
}

async function initializeMapConfig(
  program: Program<MapGenerator>,
  mapConfig: anchor.web3.PublicKey,
  admin: anchor.web3.PublicKey
) {
  try {
    await program.methods
      .initializeMapConfig()
      .accounts({
        mapConfig,
        admin,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();
    console.log("Map config initialized:", mapConfig.toBase58());
  } catch (error: any) {
    if (!error.toString().includes("already in use")) {
      throw error;
    }
    console.log("Map config already exists:", mapConfig.toBase58());
  }
}
