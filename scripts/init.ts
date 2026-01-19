import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import idlSessionManager from "../target/idl/session_manager.json";
import idlMapGenerator from "../target/idl/map_generator.json";

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
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });
