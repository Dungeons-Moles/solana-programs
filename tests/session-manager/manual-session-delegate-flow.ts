import * as anchor from "@coral-xyz/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  Connection,
  SendTransactionError,
} from "@solana/web3.js";
import fs from "fs";
import os from "os";
import path from "path";

async function main() {
  const walletPath = process.env.ANCHOR_WALLET ?? path.join(os.homedir(), ".config/solana/id.json");
  const walletSecret = Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf8")));
  const wallet = new anchor.Wallet(Keypair.fromSecretKey(walletSecret));
  const connection = new Connection(process.env.ANCHOR_PROVIDER_URL ?? "http://127.0.0.1:8899", "confirmed");
  const erConnection = new Connection(
    process.env.EXPO_PUBLIC_EPHEMERAL_PROVIDER_ENDPOINT ?? "http://127.0.0.1:7799",
    "confirmed"
  );
  const DELEGATION_PROGRAM_ID = new PublicKey("DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh");
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });
  anchor.setProvider(provider);

  const sessionManager: any = anchor.workspace.SessionManager;
  const playerProfile: any = anchor.workspace.PlayerProfile;
  const gameplayState: any = anchor.workspace.GameplayState;
  const mapGenerator: any = anchor.workspace.MapGenerator;
  const playerInventory: any = anchor.workspace.PlayerInventory;
  const poiSystem: any = anchor.workspace.PoiSystem;

  const getSessionCounterPda = () =>
    PublicKey.findProgramAddressSync([Buffer.from("session_counter")], sessionManager.programId);
  const getMapConfigPda = () =>
    PublicKey.findProgramAddressSync([Buffer.from("map_config")], mapGenerator.programId);
  const getPlayerProfilePda = (owner: PublicKey) =>
    PublicKey.findProgramAddressSync([Buffer.from("player"), owner.toBuffer()], playerProfile.programId);
  const getSessionPda = (owner: PublicKey, campaignLevel: number) =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("session"), owner.toBuffer(), Buffer.from([campaignLevel])],
      sessionManager.programId
    );
  const getGameStatePda = (sessionPda: PublicKey) =>
    PublicKey.findProgramAddressSync([Buffer.from("game_state"), sessionPda.toBuffer()], gameplayState.programId);
  const getMapEnemiesPda = (sessionPda: PublicKey) =>
    PublicKey.findProgramAddressSync([Buffer.from("map_enemies"), sessionPda.toBuffer()], gameplayState.programId);
  const getGeneratedMapPda = (sessionPda: PublicKey) =>
    PublicKey.findProgramAddressSync([Buffer.from("generated_map"), sessionPda.toBuffer()], mapGenerator.programId);
  const getInventoryPda = (sessionPda: PublicKey) =>
    PublicKey.findProgramAddressSync([Buffer.from("inventory"), sessionPda.toBuffer()], playerInventory.programId);
  const getMapPoisPda = (sessionPda: PublicKey) =>
    PublicKey.findProgramAddressSync([Buffer.from("map_pois"), sessionPda.toBuffer()], poiSystem.programId);
  const deriveDelegateAccounts = (target: PublicKey, ownerProgram: PublicKey) => {
    const [buffer] = PublicKey.findProgramAddressSync([Buffer.from("buffer"), target.toBuffer()], ownerProgram);
    const [delegationRecord] = PublicKey.findProgramAddressSync(
      [Buffer.from("delegation"), target.toBuffer()],
      DELEGATION_PROGRAM_ID
    );
    const [delegationMetadata] = PublicKey.findProgramAddressSync(
      [Buffer.from("delegation-metadata"), target.toBuffer()],
      DELEGATION_PROGRAM_ID
    );
    return { buffer, delegationRecord, delegationMetadata };
  };

  const [sessionCounterPda] = getSessionCounterPda();
  const [mapConfigPda] = getMapConfigPda();

  try {
    await sessionManager.methods
      .initializeCounter()
      .accounts({
        sessionCounter: sessionCounterPda,
        admin: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("[flow] initialized session counter");
  } catch (e: any) {
    if (!String(e).includes("already in use")) throw e;
  }

  try {
    await mapGenerator.methods
      .initializeMapConfig()
      .accounts({
        mapConfig: mapConfigPda,
        admin: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("[flow] initialized map config");
  } catch (e: any) {
    if (!String(e).includes("already in use")) throw e;
  }

  const user = Keypair.generate();
  const sessionSigner = Keypair.generate();
  const name = `flow-${user.publicKey.toBase58().slice(0, 6)}`;
  const campaignLevel = 1;

  for (const kp of [user, sessionSigner]) {
    const sig = await provider.connection.requestAirdrop(kp.publicKey, 2 * LAMPORTS_PER_SOL);
    const latest = await provider.connection.getLatestBlockhash("confirmed");
    await provider.connection.confirmTransaction({ signature: sig, ...latest }, "confirmed");
  }
  console.log("[flow] funded new user + session signer", {
    user: user.publicKey.toBase58(),
    sessionSigner: sessionSigner.publicKey.toBase58(),
  });

  const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
  await playerProfile.methods
    .initializeProfile(name)
    .accounts({
      playerProfile: playerProfilePda,
      owner: user.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([user])
    .rpc();
  console.log("[flow] profile created");

  const [sessionPda] = getSessionPda(user.publicKey, campaignLevel);
  const [gameStatePda] = getGameStatePda(sessionPda);
  const [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
  const [generatedMapPda] = getGeneratedMapPda(sessionPda);
  const [inventoryPda] = getInventoryPda(sessionPda);
  const [mapPoisPda] = getMapPoisPda(sessionPda);
  const [gameplayAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("gameplay_authority")],
    gameplayState.programId
  );
  const gameplayGameStateDelegate = deriveDelegateAccounts(gameStatePda, gameplayState.programId);
  const gameplayMapEnemiesDelegate = deriveDelegateAccounts(mapEnemiesPda, gameplayState.programId);
  const generatedMapDelegate = deriveDelegateAccounts(generatedMapPda, mapGenerator.programId);
  const inventoryDelegate = deriveDelegateAccounts(inventoryPda, playerInventory.programId);
  const mapPoisDelegate = deriveDelegateAccounts(mapPoisPda, poiSystem.programId);
  const fetchGameStateFrom = async (conn: Connection) => {
    const accountInfo = await conn.getAccountInfo(gameStatePda, "confirmed");
    if (!accountInfo) {
      throw new Error(`GameState account missing on ${conn.rpcEndpoint}`);
    }
    return gameplayState.coder.accounts.decode("gameState", accountInfo.data) as {
      positionX: number;
      positionY: number;
    };
  };

  await sessionManager.methods
    .startSession(campaignLevel)
    .accounts({
      gameSession: sessionPda,
      sessionCounter: sessionCounterPda,
      playerProfile: playerProfilePda,
      player: user.publicKey,
      sessionSigner: sessionSigner.publicKey,
      mapConfig: mapConfigPda,
      generatedMap: generatedMapPda,
      gameState: gameStatePda,
      mapEnemies: mapEnemiesPda,
      mapPois: mapPoisPda,
      inventory: inventoryPda,
      mapGeneratorProgram: mapGenerator.programId,
      gameplayStateProgram: gameplayState.programId,
      poiSystemProgram: poiSystem.programId,
      playerInventoryProgram: playerInventory.programId,
      systemProgram: SystemProgram.programId,
    })
    .preInstructions([
      anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      anchor.web3.ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
    ])
    .signers([user, sessionSigner])
    .rpc();
  console.log("[flow] session started");
  const initialGameState = await gameplayState.account.gameState.fetch(gameStatePda);
  console.log("[flow] initial position", {
    x: Number(initialGameState.positionX),
    y: Number(initialGameState.positionY),
  });
  const initialX = Number(initialGameState.positionX);
  const initialY = Number(initialGameState.positionY);
  const targetX = initialX > 0 ? initialX - 1 : initialX + 1;
  const targetY = initialY;

  const delegateGameplayIx = await gameplayState.methods
    .delegateGameplayAccounts()
    .accountsStrict({
      bufferGameState: gameplayGameStateDelegate.buffer,
      delegationRecordGameState: gameplayGameStateDelegate.delegationRecord,
      delegationMetadataGameState: gameplayGameStateDelegate.delegationMetadata,
      gameState: gameStatePda,
      bufferMapEnemies: gameplayMapEnemiesDelegate.buffer,
      delegationRecordMapEnemies: gameplayMapEnemiesDelegate.delegationRecord,
      delegationMetadataMapEnemies: gameplayMapEnemiesDelegate.delegationMetadata,
      mapEnemies: mapEnemiesPda,
      gameSession: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: gameplayState.programId,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  const delegateGeneratedMapIx = await mapGenerator.methods
    .delegateGeneratedMap()
    .accountsStrict({
      bufferGeneratedMap: generatedMapDelegate.buffer,
      delegationRecordGeneratedMap: generatedMapDelegate.delegationRecord,
      delegationMetadataGeneratedMap: generatedMapDelegate.delegationMetadata,
      generatedMap: generatedMapPda,
      session: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: mapGenerator.programId,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  const delegateInventoryIx = await playerInventory.methods
    .delegateInventory()
    .accountsStrict({
      bufferInventory: inventoryDelegate.buffer,
      delegationRecordInventory: inventoryDelegate.delegationRecord,
      delegationMetadataInventory: inventoryDelegate.delegationMetadata,
      inventory: inventoryPda,
      session: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: playerInventory.programId,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  const delegateMapPoisIx = await poiSystem.methods
    .delegateMapPois()
    .accountsStrict({
      bufferMapPois: mapPoisDelegate.buffer,
      delegationRecordMapPois: mapPoisDelegate.delegationRecord,
      delegationMetadataMapPois: mapPoisDelegate.delegationMetadata,
      mapPois: mapPoisPda,
      gameSession: sessionPda,
      player: sessionSigner.publicKey,
      ownerProgram: poiSystem.programId,
      delegationProgram: DELEGATION_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    } as any)
    .instruction();
  const delegateSessionIx = await sessionManager.methods
    .delegateSession(campaignLevel)
    .accounts({
      gameSession: sessionPda,
      player: user.publicKey,
      sessionSigner: sessionSigner.publicKey,
    })
    .instruction();

  const sendDelegateTx = async (label: string, ixs: anchor.web3.TransactionInstruction[]) => {
    const tx = new Transaction().add(...ixs);
    tx.feePayer = sessionSigner.publicKey;
    const bh = await provider.connection.getLatestBlockhash("confirmed");
    tx.recentBlockhash = bh.blockhash;
    tx.sign(sessionSigner);
    const size = tx.serialize().length;
    console.log(`[flow] ${label} serialized size: ${size} bytes`);
    const sig = await provider.connection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    await provider.connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
    const status = await provider.connection.getSignatureStatuses([sig], {
      searchTransactionHistory: true,
    });
    const err = status.value[0]?.err;
    if (err) {
      throw new Error(`${label} failed: ${JSON.stringify(err)}`);
    }
    console.log(`[flow] ${label} ok: ${sig}`);
  };

  await sendDelegateTx("delegate-gameplay", [delegateGameplayIx]);
  await sendDelegateTx("delegate-generated-map", [delegateGeneratedMapIx]);
  await sendDelegateTx("delegate-inventory", [delegateInventoryIx]);
  await sendDelegateTx("delegate-map-pois", [delegateMapPoisIx]);
  await sendDelegateTx("delegate-session", [delegateSessionIx]);
  console.log("[flow] success with 5 delegate txs");

  const sessionBaseInfo = await connection.getAccountInfo(sessionPda, "confirmed");
  const sessionErInfo = await erConnection.getAccountInfo(sessionPda, "confirmed");
  const decodedBaseSession = sessionBaseInfo?.data
    ? (sessionManager.coder.accounts.decode("gameSession", sessionBaseInfo.data) as { isDelegated: boolean })
    : null;
  const decodedErSession = sessionErInfo?.data
    ? (sessionManager.coder.accounts.decode("gameSession", sessionErInfo.data) as { isDelegated: boolean })
    : null;
  console.log("[flow] session delegated state", {
    baseOwner: sessionBaseInfo?.owner.toBase58() ?? null,
    erOwner: sessionErInfo?.owner.toBase58() ?? null,
    baseIsDelegated: decodedBaseSession?.isDelegated ?? null,
    erIsDelegated: decodedErSession?.isDelegated ?? null,
  });

  const writableAccounts = [
    { label: "game_state", key: gameStatePda },
    { label: "game_session", key: sessionPda },
    { label: "map_enemies", key: mapEnemiesPda },
    { label: "generated_map", key: generatedMapPda },
    { label: "inventory", key: inventoryPda },
    { label: "map_pois", key: mapPoisPda },
    { label: "player(session_signer)", key: sessionSigner.publicKey },
  ];
  for (const acc of writableAccounts) {
    const baseInfo = await connection.getAccountInfo(acc.key, "confirmed");
    const erInfo = await erConnection.getAccountInfo(acc.key, "confirmed");
    console.log("[flow] owner check", {
      account: acc.label,
      key: acc.key.toBase58(),
      baseOwner: baseInfo?.owner.toBase58() ?? null,
      erOwner: erInfo?.owner.toBase58() ?? null,
      baseDelegated: baseInfo?.owner.equals(DELEGATION_PROGRAM_ID) ?? false,
      erDelegated: erInfo?.owner.equals(DELEGATION_PROGRAM_ID) ?? false,
      erExists: !!erInfo,
    });
  }

  let moveSucceeded = false;
  let lastMoveError: unknown = null;
  for (let attempt = 1; attempt <= 12; attempt += 1) {
    const moveTx = await gameplayState.methods
      .movePlayer(targetX, targetY)
      .accounts({
        gameState: gameStatePda,
        gameSession: sessionPda,
        mapEnemies: mapEnemiesPda,
        generatedMap: generatedMapPda,
        inventory: inventoryPda,
        gameplayAuthority: gameplayAuthorityPda,
        playerInventoryProgram: playerInventory.programId,
        mapGeneratorProgram: mapGenerator.programId,
        mapPois: mapPoisPda,
        poiSystemProgram: poiSystem.programId,
        player: sessionSigner.publicKey,
      })
      .transaction();
    moveTx.feePayer = sessionSigner.publicKey;
    const erBh = await erConnection.getLatestBlockhash("confirmed");
    moveTx.recentBlockhash = erBh.blockhash;
    moveTx.sign(sessionSigner);
    if (attempt === 1) {
      console.log("[flow] move tx serialized size:", moveTx.serialize().length, "bytes");
    }
    try {
      const moveSig = await erConnection.sendRawTransaction(moveTx.serialize(), {
        skipPreflight: true,
        maxRetries: 1,
      });
      await erConnection.confirmTransaction({ signature: moveSig, ...erBh }, "confirmed");
      const status = await erConnection.getSignatureStatuses([moveSig], {
        searchTransactionHistory: true,
      });
      const statusErr = status.value[0]?.err;
      if (statusErr) {
        throw new Error(
          `move tx finalized with error: ${
            typeof statusErr === "string" ? statusErr : JSON.stringify(statusErr)
          }`
        );
      }
      console.log("[flow] move tx success:", { attempt, moveSig });
      moveSucceeded = true;
      break;
    } catch (e: any) {
      lastMoveError = e;
      console.error("[flow] move tx failed:", {
        attempt,
        message: e?.message ?? String(e),
      });
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  if (!moveSucceeded) {
    const diagnosticIx = await gameplayState.methods
      .movePlayer(targetX, targetY)
      .accounts({
        gameState: gameStatePda,
        gameSession: sessionPda,
        mapEnemies: mapEnemiesPda,
        generatedMap: generatedMapPda,
        inventory: inventoryPda,
        gameplayAuthority: gameplayAuthorityPda,
        playerInventoryProgram: playerInventory.programId,
        mapGeneratorProgram: mapGenerator.programId,
        mapPois: mapPoisPda,
        poiSystemProgram: poiSystem.programId,
        player: sessionSigner.publicKey,
      })
      .instruction();
    console.log(
      "[flow] move account metas:",
      diagnosticIx.keys.map((k: any, i: number) => ({
        i,
        pubkey: k.pubkey.toBase58(),
        isSigner: k.isSigner,
        isWritable: k.isWritable,
      }))
    );
    for (let idx = 0; idx < diagnosticIx.keys.length; idx += 1) {
      const originalMeta = diagnosticIx.keys[idx];
      if (!originalMeta?.isWritable) continue;
      const ix = await gameplayState.methods
        .movePlayer(targetX, targetY)
        .accounts({
          gameState: gameStatePda,
          gameSession: sessionPda,
          mapEnemies: mapEnemiesPda,
          generatedMap: generatedMapPda,
          inventory: inventoryPda,
          gameplayAuthority: gameplayAuthorityPda,
          playerInventoryProgram: playerInventory.programId,
          mapGeneratorProgram: mapGenerator.programId,
          mapPois: mapPoisPda,
          poiSystemProgram: poiSystem.programId,
          player: sessionSigner.publicKey,
        })
        .instruction();
      ix.keys[idx].isWritable = false;
      const tx = new Transaction().add(ix);
      tx.feePayer = sessionSigner.publicKey;
      const bh = await erConnection.getLatestBlockhash("confirmed");
      tx.recentBlockhash = bh.blockhash;
      tx.sign(sessionSigner);
      try {
        const sig = await erConnection.sendRawTransaction(tx.serialize(), {
          skipPreflight: false,
          maxRetries: 1,
        });
        await erConnection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
        console.log("[flow] diagnostic writable override success", {
          idx,
          pubkey: originalMeta.pubkey.toBase58(),
          sig,
        });
      } catch (diagErr: any) {
        console.log("[flow] diagnostic writable override failed", {
          idx,
          pubkey: originalMeta.pubkey.toBase58(),
          message: diagErr?.message ?? String(diagErr),
        });
      }
    }
    try {
      const walletPaidIx = await gameplayState.methods
        .movePlayer(targetX, targetY)
        .accounts({
          gameState: gameStatePda,
          gameSession: sessionPda,
          mapEnemies: mapEnemiesPda,
          generatedMap: generatedMapPda,
          inventory: inventoryPda,
          gameplayAuthority: gameplayAuthorityPda,
          playerInventoryProgram: playerInventory.programId,
          mapGeneratorProgram: mapGenerator.programId,
          mapPois: mapPoisPda,
          poiSystemProgram: poiSystem.programId,
          player: sessionSigner.publicKey,
        })
        .instruction();
      const walletPaidTx = new Transaction().add(walletPaidIx);
      walletPaidTx.feePayer = user.publicKey;
      const bh = await erConnection.getLatestBlockhash("confirmed");
      walletPaidTx.recentBlockhash = bh.blockhash;
      walletPaidTx.sign(user, sessionSigner);
      const sig = await erConnection.sendRawTransaction(walletPaidTx.serialize(), {
        skipPreflight: false,
        maxRetries: 1,
      });
      await erConnection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
      console.log("[flow] diagnostic: wallet-as-fee-payer move success", sig);
    } catch (walletPayerErr: any) {
      console.error(
        "[flow] diagnostic: wallet-as-fee-payer move failed",
        walletPayerErr?.message ?? String(walletPayerErr)
      );
    }
    if (lastMoveError instanceof SendTransactionError) {
      try {
        const logs = await lastMoveError.getLogs(erConnection);
        console.error("[flow] move tx logs:", logs);
      } catch (logErr) {
        console.error("[flow] move tx logs unavailable:", String(logErr));
      }
    }
    throw lastMoveError;
  }

  const updatedGameStateEr = await fetchGameStateFrom(erConnection);
  const updatedGameStateBase = await gameplayState.account.gameState.fetch(gameStatePda);
  console.log("[flow] updated position (ER)", {
    x: Number(updatedGameStateEr.positionX),
    y: Number(updatedGameStateEr.positionY),
  });
  console.log("[flow] updated position (base)", {
    x: Number(updatedGameStateBase.positionX),
    y: Number(updatedGameStateBase.positionY),
  });
  if (
    Number(updatedGameStateEr.positionX) === Number(initialGameState.positionX) &&
    Number(updatedGameStateEr.positionY) === Number(initialGameState.positionY)
  ) {
    throw new Error(
      "[flow] move transaction reported success but ER position did not change"
    );
  }

  const fallbackStateHash = Array.from({ length: 32 }, (_, i) => i);
  const sendErSessionSignerTx = async (
    label: string,
    ixs: anchor.web3.TransactionInstruction[]
  ): Promise<string> => {
    const tx = new Transaction().add(...ixs);
    tx.feePayer = sessionSigner.publicKey;
    const bh = await erConnection.getLatestBlockhash("confirmed");
    tx.recentBlockhash = bh.blockhash;
    tx.sign(sessionSigner);
    const sig = await erConnection.sendRawTransaction(tx.serialize(), {
      skipPreflight: true,
      maxRetries: 3,
    });
    await erConnection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
    const status = await erConnection.getSignatureStatuses([sig], { searchTransactionHistory: true });
    if (status.value[0]?.err) {
      const txMeta = await erConnection.getTransaction(sig, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      });
      console.error(`[flow] ${label} logs:`, txMeta?.meta?.logMessages ?? null);
      throw new Error(`[flow] ${label} failed: ${JSON.stringify(status.value[0].err)}`);
    }
    console.log(`[flow] ${label} ok:`, sig);
    return sig;
  };
  const waitForBaseOwner = async (
    account: PublicKey,
    owner: PublicKey,
    label: string
  ): Promise<void> => {
    for (let i = 0; i < 40; i += 1) {
      const info = await connection.getAccountInfo(account, "confirmed");
      if (info?.owner.equals(owner)) {
        console.log(`[flow] ${label} owner restored:`, owner.toBase58());
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
    const info = await connection.getAccountInfo(account, "confirmed");
    throw new Error(
      `[flow] ${label} owner did not restore (current=${info?.owner.toBase58() ?? "missing"})`
    );
  };

  const undelegateGameplayIx = await gameplayState.methods
    .undelegateGameplayAccounts()
    .accounts({
      gameState: gameStatePda,
      mapEnemies: mapEnemiesPda,
      gameSession: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    })
    .instruction();
  await sendErSessionSignerTx("undelegate-gameplay", [undelegateGameplayIx]);
  await waitForBaseOwner(gameStatePda, gameplayState.programId, "game_state");
  await waitForBaseOwner(mapEnemiesPda, gameplayState.programId, "map_enemies");

  const undelegateGeneratedMapIx = await mapGenerator.methods
    .undelegateGeneratedMap()
    .accounts({
      generatedMap: generatedMapPda,
      session: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    })
    .instruction();
  await sendErSessionSignerTx("undelegate-generated-map", [undelegateGeneratedMapIx]);
  await waitForBaseOwner(generatedMapPda, mapGenerator.programId, "generated_map");

  const undelegateInventoryIx = await playerInventory.methods
    .undelegateInventory()
    .accounts({
      inventory: inventoryPda,
      session: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    })
    .instruction();
  await sendErSessionSignerTx("undelegate-inventory", [undelegateInventoryIx]);
  await waitForBaseOwner(inventoryPda, playerInventory.programId, "inventory");

  const undelegateMapPoisIx = await poiSystem.methods
    .undelegateMapPois()
    .accounts({
      mapPois: mapPoisPda,
      gameSession: sessionPda,
      sessionSigner: sessionSigner.publicKey,
    })
    .instruction();
  await sendErSessionSignerTx("undelegate-map-pois", [undelegateMapPoisIx]);
  await waitForBaseOwner(mapPoisPda, poiSystem.programId, "map_pois");

  const undelegateIx = await sessionManager.methods
    .undelegateSession(campaignLevel, fallbackStateHash)
    .accounts({
      gameSession: sessionPda,
      player: user.publicKey,
      sessionSigner: sessionSigner.publicKey,
    })
    .instruction();
  const undelegateTx = new Transaction().add(undelegateIx);
  undelegateTx.feePayer = sessionSigner.publicKey;
  const undelegateBh = await erConnection.getLatestBlockhash("confirmed");
  undelegateTx.recentBlockhash = undelegateBh.blockhash;
  undelegateTx.sign(sessionSigner);
  const undelegateSig = await erConnection.sendRawTransaction(undelegateTx.serialize(), {
    skipPreflight: true,
    maxRetries: 3,
  });
  await erConnection.confirmTransaction({ signature: undelegateSig, ...undelegateBh }, "confirmed");
  const undelegateStatus = await erConnection.getSignatureStatuses([undelegateSig], {
    searchTransactionHistory: true,
  });
  if (undelegateStatus.value[0]?.err) {
    const tx = await erConnection.getTransaction(undelegateSig, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    console.error("[flow] undelegate-session logs:", tx?.meta?.logMessages ?? null);
    throw new Error(
      `[flow] undelegate-session failed: ${JSON.stringify(undelegateStatus.value[0].err)}`
    );
  }
  console.log("[flow] undelegate-session sent:", undelegateSig);

  let baseOwnerRestored = false;
  for (let i = 0; i < 30; i += 1) {
    const info = await connection.getAccountInfo(sessionPda, "confirmed");
    if (info?.owner.equals(sessionManager.programId)) {
      baseOwnerRestored = true;
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  console.log("[flow] session owner restored on base:", baseOwnerRestored);
  if (!baseOwnerRestored) {
    throw new Error("[flow] session owner not restored after undelegate");
  }

  const endSessionIx = await sessionManager.methods
    .endSession(campaignLevel)
    .accounts({
      gameSession: sessionPda,
      gameState: gameStatePda,
      mapEnemies: mapEnemiesPda,
      generatedMap: generatedMapPda,
      mapPois: mapPoisPda,
      playerProfile: playerProfilePda,
      player: user.publicKey,
      sessionSigner: sessionSigner.publicKey,
      sessionManagerAuthority: PublicKey.findProgramAddressSync(
        [Buffer.from("session_manager_authority")],
        sessionManager.programId
      )[0],
      inventory: inventoryPda,
      playerInventoryProgram: playerInventory.programId,
      gameplayStateProgram: gameplayState.programId,
      playerProfileProgram: playerProfile.programId,
      mapGeneratorProgram: mapGenerator.programId,
      poiSystemProgram: poiSystem.programId,
    })
    .instruction();
  const endTx = new Transaction().add(endSessionIx);
  endTx.feePayer = sessionSigner.publicKey;
  const endBh = await connection.getLatestBlockhash("confirmed");
  endTx.recentBlockhash = endBh.blockhash;
  endTx.sign(sessionSigner);
  const endSig = await connection.sendRawTransaction(endTx.serialize(), {
    skipPreflight: true,
    maxRetries: 3,
  });
  await connection.confirmTransaction({ signature: endSig, ...endBh }, "confirmed");
  const endStatus = await connection.getSignatureStatuses([endSig], {
    searchTransactionHistory: true,
  });
  if (endStatus.value[0]?.err) {
    const tx = await connection.getTransaction(endSig, {
      commitment: "confirmed",
      maxSupportedTransactionVersion: 0,
    });
    console.error("[flow] end-session logs:", tx?.meta?.logMessages ?? null);
    throw new Error(`[flow] end-session failed: ${JSON.stringify(endStatus.value[0].err)}`);
  }
  console.log("[flow] end-session success:", endSig);

  const postCloseSessionInfo = await connection.getAccountInfo(sessionPda, "confirmed");
  console.log("[flow] post-close session exists:", !!postCloseSessionInfo);
  if (postCloseSessionInfo) {
    throw new Error("[flow] session account still exists after end-session");
  }
}

main().catch((e) => {
  console.error("[flow] fatal error:", e);
  process.exit(1);
});
