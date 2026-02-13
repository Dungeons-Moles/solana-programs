import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import idlSessionManager from "../target/idl/session_manager.json";
import idlMapGenerator from "../target/idl/map_generator.json";
import idlGameplayState from "../target/idl/gameplay_state.json";

async function main() {
  console.log("Starting initialization...");

  // Configure client to use the provider.
  // This reads from ANCHOR_PROVIDER_URL (default: http://localhost:8899)
  // and ANCHOR_WALLET (default: ~/.config/solana/id.json)
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  console.log("Provider set:", provider.connection.rpcEndpoint);
  console.log("Wallet:", provider.wallet.publicKey.toBase58());

  const sessionManager = new Program(idlSessionManager as any, provider);
  const mapGenerator = new Program(idlMapGenerator as any, provider);
  const gameplayState = new Program(idlGameplayState as any, provider);

  console.log("Initializing Session Counter...");
  const [counterPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("session_counter")],
    sessionManager.programId,
  );

  try {
    const tx = await sessionManager.methods
      .initializeCounter()
      .accounts({
        sessionCounter: counterPda,
        admin: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .rpc();
    console.log("Success! Session counter initialized. Sig:", tx);
  } catch (e: any) {
    if (e.toString().includes("already in use")) {
      console.log("Session counter already initialized.");
    } else {
      console.error("Failed to initialize session counter:", e);
    }
  }

  console.log("Initializing Map Config...");
  const [mapConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("map_config")],
    mapGenerator.programId,
  );

  try {
    const tx = await mapGenerator.methods
      .initializeMapConfig()
      .accounts({
        mapConfig: mapConfigPda,
        admin: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .rpc();
    console.log("Success! Map config initialized. Sig:", tx);
  } catch (e: any) {
    if (e.toString().includes("already in use")) {
      console.log("Map config already initialized.");
    } else {
      console.error("Failed to initialize map config:", e);
    }
  }

  console.log("Initializing Duels (vault)...");
  const [duelVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("duel_vault")],
    gameplayState.programId,
  );
  const [duelOpenQueuePda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("duel_open_queue")],
    gameplayState.programId,
  );

  try {
    const tx = await gameplayState.methods
      .initializeDuels()
      .accounts({
        duelVault: duelVaultPda,
        duelOpenQueue: duelOpenQueuePda,
        admin: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .rpc();
    console.log("Success! Duels vault initialized. Sig:", tx);
  } catch (e: any) {
    if (e.toString().includes("already in use")) {
      console.log("Duels vault already initialized.");
    } else {
      console.error("Failed to initialize duels vault:", e);
    }
  }

  console.log("Initializing Pit Draft...");
  const [pitDraftQueuePda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("pit_draft_queue")],
    gameplayState.programId,
  );
  const [pitDraftVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("pit_draft_vault")],
    gameplayState.programId,
  );

  try {
    const tx = await gameplayState.methods
      .initializePitDraft()
      .accounts({
        pitDraftQueue: pitDraftQueuePda,
        pitDraftVault: pitDraftVaultPda,
        admin: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .rpc();
    console.log("Success! Pit Draft initialized. Sig:", tx);
  } catch (e: any) {
    if (e.toString().includes("already in use")) {
      console.log("Pit Draft already initialized.");
    } else {
      console.error("Failed to initialize pit draft:", e);
    }
  }

  console.log("Initializing Gauntlet...");
  const [gauntletConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_config")],
    gameplayState.programId,
  );
  const [gauntletPoolVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_pool_vault")],
    gameplayState.programId,
  );
  const [gauntletWeek1Pda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_week_pool"), Buffer.from([1])],
    gameplayState.programId,
  );
  const [gauntletWeek2Pda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_week_pool"), Buffer.from([2])],
    gameplayState.programId,
  );
  const [gauntletWeek3Pda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_week_pool"), Buffer.from([3])],
    gameplayState.programId,
  );
  const [gauntletWeek4Pda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_week_pool"), Buffer.from([4])],
    gameplayState.programId,
  );
  const [gauntletWeek5Pda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_week_pool"), Buffer.from([5])],
    gameplayState.programId,
  );

  try {
    const tx = await gameplayState.methods
      .initializeGauntlet()
      .accounts({
        gauntletConfig: gauntletConfigPda,
        gauntletPoolVault: gauntletPoolVaultPda,
        gauntletWeek1: gauntletWeek1Pda,
        gauntletWeek2: gauntletWeek2Pda,
        gauntletWeek3: gauntletWeek3Pda,
        gauntletWeek4: gauntletWeek4Pda,
        gauntletWeek5: gauntletWeek5Pda,
        admin: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .preInstructions([
        anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
          units: 1_400_000,
        }),
      ])
      .rpc();
    console.log("Success! Gauntlet initialized. Sig:", tx);
  } catch (e: any) {
    if (e.toString().includes("already in use")) {
      console.log("Gauntlet already initialized.");
    } else {
      console.error("Failed to initialize gauntlet:", e);
    }
  }
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });
