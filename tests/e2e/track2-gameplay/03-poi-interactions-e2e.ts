import { expect } from "chai";
import {
  anchor,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Connection,
  loadAllPrograms,
  loadWalletKeypair,
  createProvider,
  walletFromKeypair,
  airdropAndConfirm,
  PROGRAM_IDS,
  AllPrograms,
} from "../shared/setup";
import {
  getSessionCounterPda,
  getSessionNoncesPda,
  getSessionPda,
  getSessionManagerAuthorityPda,
  getPlayerProfilePda,
  getMapConfigPda,
  getGeneratedMapPda,
  getGameStatePda,
  getMapEnemiesPda,
  getGameplayAuthorityPda,
  getDuelVaultPda,
  getDuelOpenQueuePda,
  getPitDraftQueuePda,
  getPitDraftVaultPda,
  getGauntletConfigPda,
  getGauntletPoolVaultPda,
  getGauntletWeekPoolPda,
  getGauntletEpochPoolPda,
  getGauntletPlayerScorePda,
  getInventoryPda,
  getMapPoisPda,
  getDuelSessionPda,
  getGauntletSessionPda,
  getPoiAuthorityPda,
  getInventoryAuthorityPda,
} from "../shared/pda-helpers";
import {
  Transaction,
  TransactionInstruction,
  ComputeBudgetProgram,
} from "@solana/web3.js";

// ── Constants ────────────────────────────────────────────────────────────────
const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "http://127.0.0.1:8899";
const MAP_WIDTH = 50;
const MAP_HEIGHT = 50;
const DAY_MOVES = 50;
const POI_CU_LIMIT = 400_000;
const SESSION_CU_LIMIT = 1_400_000;

// POI type IDs (L1-L14)
const POI = {
  MOLE_DEN: 1,
  SUPPLY_CACHE: 2,
  TOOL_CRATE: 3,
  TOOL_OIL_RACK: 4,
  REST_ALCOVE: 5,
  SURVEY_BEACON: 6,
  SEISMIC_SCANNER: 7,
  RAIL_WAYPOINT: 8,
  SMUGGLER_HATCH: 9,
  RUSTY_ANVIL: 10,
  RUNE_KILN: 11,
  GEODE_VAULT: 12,
  COUNTER_CACHE: 13,
  SCRAP_CHUTE: 14,
};

// ── Shared state ─────────────────────────────────────────────────────────────
let connection: Connection;
let provider: anchor.AnchorProvider;
let programs: AllPrograms;
let admin: Keypair;

// Global PDAs
let sessionCounterPda: PublicKey;
let mapConfigPda: PublicKey;
let gameplayAuthorityPda: PublicKey;
let sessionManagerAuthorityPda: PublicKey;
let poiAuthorityPda: PublicKey;
let inventoryAuthorityPda: PublicKey;
let duelVaultPda: PublicKey;
let duelOpenQueuePda: PublicKey;
let pitDraftQueuePda: PublicKey;
let pitDraftVaultPda: PublicKey;
let gauntletConfigPda: PublicKey;
let gauntletPoolVaultPda: PublicKey;
let gauntletWeek1Pda: PublicKey;
let gauntletWeek2Pda: PublicKey;
let gauntletWeek3Pda: PublicKey;
let gauntletWeek4Pda: PublicKey;
let gauntletWeek5Pda: PublicKey;

// ── Session context for each game mode ───────────────────────────────────────
interface SessionCtx {
  user: Keypair;
  sessionSigner: Keypair;
  playerProfilePda: PublicKey;
  sessionPda: PublicKey;
  gameStatePda: PublicKey;
  mapEnemiesPda: PublicKey;
  generatedMapPda: PublicKey;
  inventoryPda: PublicKey;
  mapPoisPda: PublicKey;
  campaignLevel: number;
}

interface PoiInstance {
  poiType: number;
  x: number;
  y: number;
  used: boolean;
  discovered: boolean;
  weekSpawned: number;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

const sendBaseTx = async (
  label: string,
  ixs: TransactionInstruction[],
  signers: Keypair[]
): Promise<string> => {
  const tx = new Transaction().add(...ixs);
  tx.feePayer = signers[0].publicKey;
  const bh = await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = bh.blockhash;
  tx.sign(...signers);
  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: true,
    maxRetries: 3,
  });
  await connection.confirmTransaction({ signature: sig, ...bh }, "confirmed");
  const status = await connection.getSignatureStatuses([sig], {
    searchTransactionHistory: true,
  });
  if (status.value[0]?.err) {
    throw new Error(`${label} failed: ${JSON.stringify(status.value[0].err)}`);
  }
  return sig;
};

/**
 * Check if a Solana transaction error is "ProgramFailedToComplete" (BPF stack overflow).
 * This is the specific error we're testing against with the Box<Account> fix.
 */
const isProgramFailedToComplete = (err: any): boolean => {
  const s = String(err);
  return (
    s.includes("ProgramFailedToComplete") ||
    s.includes("Program failed to complete") ||
    // InstructionError index 5 = ProgramFailedToComplete
    /InstructionError.*\[.*5\]/.test(s)
  );
};

/**
 * Decode GameState from raw account data.
 */
const fetchGameState = async (gameStatePda: PublicKey): Promise<any> => {
  const info = await connection.getAccountInfo(gameStatePda, "confirmed");
  if (!info) throw new Error("GameState account missing");
  return (programs.gameplayState as any).coder.accounts.decode(
    "gameState",
    info.data
  );
};

/**
 * Decode MapPois from raw account data.
 */
const fetchMapPois = async (mapPoisPda: PublicKey): Promise<any> => {
  const info = await connection.getAccountInfo(mapPoisPda, "confirmed");
  if (!info) throw new Error("MapPois account missing");
  return (programs.poiSystem as any).coder.accounts.decode(
    "mapPois",
    info.data
  );
};

/**
 * Decode PlayerInventory from raw account data.
 */
const fetchInventory = async (inventoryPda: PublicKey): Promise<any> => {
  const info = await connection.getAccountInfo(inventoryPda, "confirmed");
  if (!info) throw new Error("PlayerInventory account missing");
  return (programs.playerInventory as any).coder.accounts.decode(
    "playerInventory",
    info.data
  );
};

/**
 * Read packed_tiles from GeneratedMap raw account data.
 * Layout: 8 (disc) + 32 (session) + 1 (width) + 1 (height) + 8 (seed)
 *        + 1 (spawn_x) + 1 (spawn_y) + 1 (mole_den_x) + 1 (mole_den_y)
 *        + 2 (walkable_count) = offset 56, then 313 bytes of packed_tiles.
 */
const fetchPackedTiles = async (
  generatedMapPda: PublicKey
): Promise<Buffer> => {
  const info = await connection.getAccountInfo(generatedMapPda, "confirmed");
  if (!info) throw new Error("GeneratedMap account missing");
  const PACKED_TILES_OFFSET = 8 + 32 + 1 + 1 + 8 + 1 + 1 + 1 + 1 + 2;
  const PACKED_TILES_SIZE = 313;
  return info.data.subarray(
    PACKED_TILES_OFFSET,
    PACKED_TILES_OFFSET + PACKED_TILES_SIZE
  );
};

/**
 * Check if a tile is walkable (floor) in the bit-packed tile array.
 * 0 = floor (walkable), 1 = wall.
 */
const isWalkable = (
  packedTiles: Buffer,
  width: number,
  x: number,
  y: number
): boolean => {
  if (x < 0 || y < 0 || x >= width || y >= MAP_HEIGHT) return false;
  const index = y * width + x;
  const byteIndex = Math.floor(index / 8);
  const bitIndex = index % 8;
  return ((packedTiles[byteIndex] >> bitIndex) & 1) === 0;
};

/**
 * BFS pathfinding on the tile grid. Returns array of [x, y] steps
 * from start (exclusive) to target (inclusive), or null if no path.
 */
const findPath = (
  packedTiles: Buffer,
  width: number,
  height: number,
  fromX: number,
  fromY: number,
  toX: number,
  toY: number
): [number, number][] | null => {
  if (fromX === toX && fromY === toY) return [];

  const visited = new Set<number>();
  const parent = new Map<number, number>();
  const key = (x: number, y: number) => y * width + x;

  const startKey = key(fromX, fromY);
  const targetKey = key(toX, toY);
  visited.add(startKey);

  const queue: [number, number][] = [[fromX, fromY]];
  const dirs: [number, number][] = [
    [0, -1],
    [0, 1],
    [-1, 0],
    [1, 0],
  ];

  while (queue.length > 0) {
    const [cx, cy] = queue.shift()!;
    const ck = key(cx, cy);

    for (const [dx, dy] of dirs) {
      const nx = cx + dx;
      const ny = cy + dy;
      if (nx < 0 || ny < 0 || nx >= width || ny >= height) continue;
      const nk = key(nx, ny);
      if (visited.has(nk)) continue;
      if (!isWalkable(packedTiles, width, nx, ny)) continue;

      visited.add(nk);
      parent.set(nk, ck);

      if (nk === targetKey) {
        // Reconstruct path
        const path: [number, number][] = [];
        let cur = targetKey;
        while (cur !== startKey) {
          const px = cur % width;
          const py = Math.floor(cur / width);
          path.unshift([px, py]);
          cur = parent.get(cur)!;
        }
        return path;
      }

      queue.push([nx, ny]);
    }
  }

  return null; // No path found
};

/**
 * Navigate player to target (x, y) via movePlayer calls along BFS path.
 * Returns true if player reached the target, false if player died or no path.
 */
const navigatePlayerTo = async (
  ctx: SessionCtx,
  targetX: number,
  targetY: number,
  packedTiles: Buffer
): Promise<boolean> => {
  let gs = await fetchGameState(ctx.gameStatePda);
  let posX = Number(gs.positionX);
  let posY = Number(gs.positionY);

  if (posX === targetX && posY === targetY) return true;

  const path = findPath(
    packedTiles,
    MAP_WIDTH,
    MAP_HEIGHT,
    posX,
    posY,
    targetX,
    targetY
  );
  if (!path || path.length === 0) {
    console.log(
      `    No path from (${posX},${posY}) to (${targetX},${targetY})`
    );
    return false;
  }

  for (const [tx, ty] of path) {
    gs = await fetchGameState(ctx.gameStatePda);
    if (gs.isDead) {
      console.log(`    Player died during navigation at (${posX},${posY})`);
      return false;
    }

    try {
      const moveIx = await programs.gameplayState.methods
        .movePlayer(tx, ty)
        .accounts({
          gameState: ctx.gameStatePda,
          gameSession: ctx.sessionPda,
          mapEnemies: ctx.mapEnemiesPda,
          generatedMap: ctx.generatedMapPda,
          inventory: ctx.inventoryPda,
          gameplayAuthority: gameplayAuthorityPda,
          playerInventoryProgram: PROGRAM_IDS.playerInventory,
          mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
          mapPois: ctx.mapPoisPda,
          poiSystemProgram: PROGRAM_IDS.poiSystem,
          gameplayVrfState: null,
          player: ctx.sessionSigner.publicKey,
        } as any)
        .instruction();

      await sendBaseTx(
        `move(${tx},${ty})`,
        [
          ComputeBudgetProgram.setComputeUnitLimit({ units: POI_CU_LIMIT }),
          moveIx,
        ],
        [ctx.sessionSigner]
      );

      posX = tx;
      posY = ty;
    } catch (e: any) {
      // Check for death after failed move
      gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) {
        console.log(`    Player died during combat at move to (${tx},${ty})`);
        return false;
      }
      // Other move failures (e.g., phase transition blocked the move) - skip remaining path
      console.log(`    Move to (${tx},${ty}) failed: ${String(e).slice(0, 120)}`);
      return false;
    }
  }

  // Verify we arrived
  gs = await fetchGameState(ctx.gameStatePda);
  return Number(gs.positionX) === targetX && Number(gs.positionY) === targetY;
};

/**
 * Exhaust remaining day moves by ping-ponging between current position and
 * an adjacent walkable tile. This transitions the player to Night phase.
 */
const exhaustMovesToNight = async (
  ctx: SessionCtx,
  packedTiles: Buffer
): Promise<boolean> => {
  let gs = await fetchGameState(ctx.gameStatePda);
  if (gs.isDead) return false;
  if (gs.phase.night1 || gs.phase.night2 || gs.phase.night3) return true;

  let posX = Number(gs.positionX);
  let posY = Number(gs.positionY);

  // Find an adjacent walkable tile for ping-pong
  const dirs: [number, number][] = [
    [0, -1],
    [0, 1],
    [-1, 0],
    [1, 0],
  ];
  let altX = -1;
  let altY = -1;
  for (const [dx, dy] of dirs) {
    const nx = posX + dx;
    const ny = posY + dy;
    if (isWalkable(packedTiles, MAP_WIDTH, nx, ny)) {
      altX = nx;
      altY = ny;
      break;
    }
  }
  if (altX < 0) {
    console.log("    No adjacent walkable tile for ping-pong");
    return false;
  }

  // Ping-pong until night phase
  let maxAttempts = DAY_MOVES + 10;
  let atHome = true;
  for (let i = 0; i < maxAttempts; i++) {
    gs = await fetchGameState(ctx.gameStatePda);
    if (gs.isDead) return false;
    if (gs.phase.night1 || gs.phase.night2 || gs.phase.night3) return true;

    const [tx, ty] = atHome ? [altX, altY] : [posX, posY];
    try {
      const moveIx = await programs.gameplayState.methods
        .movePlayer(tx, ty)
        .accounts({
          gameState: ctx.gameStatePda,
          gameSession: ctx.sessionPda,
          mapEnemies: ctx.mapEnemiesPda,
          generatedMap: ctx.generatedMapPda,
          inventory: ctx.inventoryPda,
          gameplayAuthority: gameplayAuthorityPda,
          playerInventoryProgram: PROGRAM_IDS.playerInventory,
          mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
          mapPois: ctx.mapPoisPda,
          poiSystemProgram: PROGRAM_IDS.poiSystem,
          gameplayVrfState: null,
          player: ctx.sessionSigner.publicKey,
        } as any)
        .instruction();

      await sendBaseTx(
        `exhaust-move-${i}`,
        [
          ComputeBudgetProgram.setComputeUnitLimit({ units: POI_CU_LIMIT }),
          moveIx,
        ],
        [ctx.sessionSigner]
      );
      atHome = !atHome;
    } catch {
      // Move may fail due to phase transition or combat; continue
      gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) return false;
      if (gs.phase.night1 || gs.phase.night2 || gs.phase.night3) return true;
    }
  }

  gs = await fetchGameState(ctx.gameStatePda);
  return gs.phase.night1 || gs.phase.night2 || gs.phase.night3;
};

/**
 * Get all POI instances from the MapPois account, parsed into our interface.
 */
const getPoiInstances = async (
  mapPoisPda: PublicKey
): Promise<PoiInstance[]> => {
  const mp = await fetchMapPois(mapPoisPda);
  return mp.pois.map((p: any) => ({
    poiType: Number(p.poiType),
    x: Number(p.x),
    y: Number(p.y),
    used: Boolean(p.used),
    discovered: Boolean(p.discovered),
    weekSpawned: Number(p.weekSpawned),
  }));
};

/**
 * Find the index and instance of a POI by type. Returns null if not on map.
 */
const findPoi = (
  pois: PoiInstance[],
  poiType: number
): { index: number; poi: PoiInstance } | null => {
  for (let i = 0; i < pois.length; i++) {
    if (pois[i].poiType === poiType && !pois[i].used) {
      return { index: i, poi: pois[i] };
    }
  }
  return null;
};

/**
 * Find ALL POIs of a given type (for waypoints that need 2).
 */
const findAllPoisOfType = (
  pois: PoiInstance[],
  poiType: number
): { index: number; poi: PoiInstance }[] => {
  const results: { index: number; poi: PoiInstance }[] = [];
  for (let i = 0; i < pois.length; i++) {
    if (pois[i].poiType === poiType) {
      results.push({ index: i, poi: pois[i] });
    }
  }
  return results;
};

/**
 * Find the closest unused POI of a given type, based on Manhattan distance
 * from the player's current position. Minimizes navigation distance to reduce
 * combat encounters (which can kill the player).
 */
const findClosestPoi = (
  pois: PoiInstance[],
  poiType: number,
  playerX: number,
  playerY: number
): { index: number; poi: PoiInstance } | null => {
  let best: { index: number; poi: PoiInstance } | null = null;
  let bestDist = Infinity;
  for (let i = 0; i < pois.length; i++) {
    if (pois[i].poiType === poiType && !pois[i].used) {
      const dist =
        Math.abs(pois[i].x - playerX) + Math.abs(pois[i].y - playerY);
      if (dist < bestDist) {
        bestDist = dist;
        best = { index: i, poi: pois[i] };
      }
    }
  }
  return best;
};

// ── POI interaction builders ─────────────────────────────────────────────────

const buildInteractRest = async (
  ctx: SessionCtx,
  poiIndex: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactRest(poiIndex)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      generatedMap: ctx.generatedMapPda,
      poiAuthority: poiAuthorityPda,
      gameplayAuthority: gameplayAuthorityPda,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      gameplayVrfState: null,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildGenerateCacheOffer = async (
  ctx: SessionCtx,
  poiIndex: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .generateCacheOffer(poiIndex)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      inventoryAuthority: inventoryAuthorityPda,
      poiAuthority: poiAuthorityPda,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      gameSession: ctx.sessionPda,
      poiVrfState: null,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildInteractPickItem = async (
  ctx: SessionCtx,
  poiIndex: number,
  choiceIndex: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactPickItem(poiIndex, choiceIndex)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      inventoryAuthority: inventoryAuthorityPda,
      poiAuthority: poiAuthorityPda,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      gameSession: ctx.sessionPda,
      poiVrfState: null,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildGenerateOilOffer = async (
  ctx: SessionCtx,
  poiIndex: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .generateOilOffer(poiIndex)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      poiAuthority: poiAuthorityPda,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      poiVrfState: null,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildInteractToolOil = async (
  ctx: SessionCtx,
  poiIndex: number,
  modification: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactToolOil(poiIndex, 0, modification)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      poiAuthority: poiAuthorityPda,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      poiVrfState: null,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildEnterShop = async (
  ctx: SessionCtx,
  poiIndex: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .enterShop(poiIndex)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      gameSession: ctx.sessionPda,
      poiVrfState: null,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildLeaveShop = async (
  ctx: SessionCtx
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .leaveShop()
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameSession: ctx.sessionPda,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildInteractSurveyBeacon = async (
  ctx: SessionCtx,
  poiIndex: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactSurveyBeacon(poiIndex)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildInteractSeismicScanner = async (
  ctx: SessionCtx,
  poiIndex: number,
  category: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactSeismicScanner(poiIndex, category)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildDiscoverVisibleWaypoints = async (
  ctx: SessionCtx,
  visibilityRadius: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .discoverVisibleWaypoints(visibilityRadius)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildFastTravel = async (
  ctx: SessionCtx,
  fromPoiIndex: number,
  toPoiIndex: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .fastTravel(fromPoiIndex, toPoiIndex)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      poiAuthority: poiAuthorityPda,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildInteractRustyAnvil = async (
  ctx: SessionCtx,
  poiIndex: number,
  itemId: number[],
  currentTier: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactRustyAnvil(poiIndex, itemId, currentTier)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      poiAuthority: poiAuthorityPda,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildInteractRuneKiln = async (
  ctx: SessionCtx,
  poiIndex: number,
  item1Id: number[],
  item1Tier: number,
  item2Id: number[],
  item2Tier: number
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactRuneKiln(poiIndex, item1Id, item1Tier, item2Id, item2Tier)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      poiAuthority: poiAuthorityPda,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

const buildInteractScrapChute = async (
  ctx: SessionCtx,
  poiIndex: number,
  itemId: number[]
): Promise<TransactionInstruction> => {
  return programs.poiSystem.methods
    .interactScrapChute(poiIndex, itemId)
    .accounts({
      mapPois: ctx.mapPoisPda,
      gameState: ctx.gameStatePda,
      inventory: ctx.inventoryPda,
      inventoryAuthority: inventoryAuthorityPda,
      poiAuthority: poiAuthorityPda,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      player: ctx.sessionSigner.publicKey,
    } as any)
    .instruction();
};

// ── Session lifecycle helpers ────────────────────────────────────────────────

const createProfileAndCampaignSession = async (
  ctx: SessionCtx
): Promise<void> => {
  const name = `poi-camp-${ctx.user.publicKey.toBase58().slice(0, 6)}`;
  await programs.playerProfile.methods
    .initializeProfile(name)
    .accounts({
      playerProfile: ctx.playerProfilePda,
      owner: ctx.user.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([ctx.user])
    .rpc();

  const [sessionNoncesPda] = getSessionNoncesPda(ctx.user.publicKey);
  await programs.sessionManager.methods
    .startSession(ctx.campaignLevel)
    .accounts({
      sessionNonces: sessionNoncesPda,
      gameSession: ctx.sessionPda,
      sessionCounter: sessionCounterPda,
      playerProfile: ctx.playerProfilePda,
      player: ctx.user.publicKey,
      sessionSigner: ctx.sessionSigner.publicKey,
      mapConfig: mapConfigPda,
      generatedMap: ctx.generatedMapPda,
      gameState: ctx.gameStatePda,
      mapEnemies: ctx.mapEnemiesPda,
      mapPois: ctx.mapPoisPda,
      inventory: ctx.inventoryPda,
      mapVrfState: null,
      poiVrfState: null,
      gameplayVrfState: null,
      mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      poiSystemProgram: PROGRAM_IDS.poiSystem,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      systemProgram: SystemProgram.programId,
    } as any)
    .preInstructions([
      ComputeBudgetProgram.setComputeUnitLimit({ units: SESSION_CU_LIMIT }),
      ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
    ])
    .signers([ctx.user, ctx.sessionSigner])
    .rpc();
};

const createProfileAndDuelSession = async (
  ctx: SessionCtx
): Promise<void> => {
  const name = `poi-duel-${ctx.user.publicKey.toBase58().slice(0, 6)}`;
  await programs.playerProfile.methods
    .initializeProfile(name)
    .accounts({
      playerProfile: ctx.playerProfilePda,
      owner: ctx.user.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([ctx.user])
    .rpc();

  await programs.sessionManager.methods
    .startDuelSession()
    .accounts({
      gameSession: ctx.sessionPda,
      sessionCounter: sessionCounterPda,
      playerProfile: ctx.playerProfilePda,
      player: ctx.user.publicKey,
      sessionSigner: ctx.sessionSigner.publicKey,
      sessionManagerAuthority: sessionManagerAuthorityPda,
      mapConfig: mapConfigPda,
      generatedMap: ctx.generatedMapPda,
      gameState: ctx.gameStatePda,
      mapEnemies: ctx.mapEnemiesPda,
      mapPois: ctx.mapPoisPda,
      inventory: ctx.inventoryPda,
      mapVrfState: null,
      poiVrfState: null,
      gameplayVrfState: null,
      mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      poiSystemProgram: PROGRAM_IDS.poiSystem,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      systemProgram: SystemProgram.programId,
    } as any)
    .preInstructions([
      ComputeBudgetProgram.setComputeUnitLimit({ units: SESSION_CU_LIMIT }),
      ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
    ])
    .signers([ctx.user, ctx.sessionSigner])
    .rpc();
};

const createProfileAndGauntletSession = async (
  ctx: SessionCtx
): Promise<void> => {
  const name = `poi-gntlt-${ctx.user.publicKey.toBase58().slice(0, 6)}`;
  await programs.playerProfile.methods
    .initializeProfile(name)
    .accounts({
      playerProfile: ctx.playerProfilePda,
      owner: ctx.user.publicKey,
      systemProgram: SystemProgram.programId,
    } as any)
    .signers([ctx.user])
    .rpc();

  await programs.sessionManager.methods
    .startGauntletSession()
    .accounts({
      gameSession: ctx.sessionPda,
      sessionCounter: sessionCounterPda,
      playerProfile: ctx.playerProfilePda,
      player: ctx.user.publicKey,
      sessionSigner: ctx.sessionSigner.publicKey,
      mapConfig: mapConfigPda,
      generatedMap: ctx.generatedMapPda,
      gameState: ctx.gameStatePda,
      mapEnemies: ctx.mapEnemiesPda,
      mapPois: ctx.mapPoisPda,
      inventory: ctx.inventoryPda,
      mapVrfState: null,
      poiVrfState: null,
      gameplayVrfState: null,
      mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
      gameplayStateProgram: PROGRAM_IDS.gameplayState,
      poiSystemProgram: PROGRAM_IDS.poiSystem,
      playerInventoryProgram: PROGRAM_IDS.playerInventory,
      systemProgram: SystemProgram.programId,
    } as any)
    .preInstructions([
      ComputeBudgetProgram.setComputeUnitLimit({ units: SESSION_CU_LIMIT }),
      ComputeBudgetProgram.requestHeapFrame({ bytes: 256 * 1024 }),
    ])
    .signers([ctx.user, ctx.sessionSigner])
    .rpc();

  // Enter gauntlet (pay entry fee, create epoch accounts)
  const treasuryPk = new PublicKey(
    "5LvEA4tH5H5DtWCxa3FcauokxAycvafX9ruvcT2mEXt8"
  );
  await airdropAndConfirm(connection, treasuryPk, LAMPORTS_PER_SOL);

  const configInfo = await connection.getAccountInfo(
    gauntletConfigPda,
    "confirmed"
  );
  if (!configInfo) throw new Error("GauntletConfig not found");
  const gauntletConfig = (
    programs.gameplayState as any
  ).coder.accounts.decode("gauntletConfig", configInfo.data);
  const epochId = new anchor.BN(gauntletConfig.currentEpochId.toString());
  const epochIdBigInt = BigInt(gauntletConfig.currentEpochId.toString());

  const [epochPoolPda] = getGauntletEpochPoolPda(epochIdBigInt);
  const [playerScorePda] = getGauntletPlayerScorePda(
    epochIdBigInt,
    ctx.user.publicKey
  );

  const enterIx = await (programs.gameplayState.methods as any)
    .enterGauntlet(epochId)
    .accounts({
      gameState: ctx.gameStatePda,
      player: ctx.user.publicKey,
      gameplayVrfState: null,
      gauntletConfig: gauntletConfigPda,
      gauntletPoolVault: gauntletPoolVaultPda,
      companyTreasury: treasuryPk,
      gauntletEpochPool: epochPoolPda,
      gauntletPlayerScore: playerScorePda,
      systemProgram: SystemProgram.programId,
    } as any)
    .remainingAccounts([
      { pubkey: gauntletWeek1Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek2Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek3Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek4Pda, isSigner: false, isWritable: false },
      { pubkey: gauntletWeek5Pda, isSigner: false, isWritable: false },
    ])
    .instruction();

  await sendBaseTx(
    "enter-gauntlet",
    [
      ComputeBudgetProgram.setComputeUnitLimit({ units: SESSION_CU_LIMIT }),
      enterIx,
    ],
    [ctx.user]
  );
};

const endSession = async (ctx: SessionCtx): Promise<void> => {
  try {
    const endIx = await programs.sessionManager.methods
      .endSession(ctx.campaignLevel)
      .accounts({
        gameSession: ctx.sessionPda,
        gameState: ctx.gameStatePda,
        mapEnemies: ctx.mapEnemiesPda,
        generatedMap: ctx.generatedMapPda,
        mapPois: ctx.mapPoisPda,
        playerProfile: ctx.playerProfilePda,
        player: ctx.user.publicKey,
        sessionSigner: ctx.sessionSigner.publicKey,
        sessionManagerAuthority: sessionManagerAuthorityPda,
        inventory: ctx.inventoryPda,
        mapVrfState: null,
        poiVrfState: null,
        gameplayVrfState: null,
        playerInventoryProgram: PROGRAM_IDS.playerInventory,
        gameplayStateProgram: PROGRAM_IDS.gameplayState,
        playerProfileProgram: PROGRAM_IDS.playerProfile,
        mapGeneratorProgram: PROGRAM_IDS.mapGenerator,
        poiSystemProgram: PROGRAM_IDS.poiSystem,
      } as any)
      .instruction();

    await sendBaseTx("end-session", [endIx], [ctx.sessionSigner]);
  } catch (e: any) {
    console.log(`  endSession cleanup failed (non-critical): ${String(e).slice(0, 120)}`);
  }
};

// ── POI interaction test runner ──────────────────────────────────────────────

/**
 * Execute a POI interaction and assert it does NOT fail with ProgramFailedToComplete.
 * Returns true if the transaction succeeded, false if it failed with a non-BPF error.
 */
const executePoi = async (
  label: string,
  ixs: TransactionInstruction[],
  signer: Keypair
): Promise<boolean> => {
  try {
    await sendBaseTx(
      label,
      [
        ComputeBudgetProgram.setComputeUnitLimit({ units: POI_CU_LIMIT }),
        ...ixs,
      ],
      [signer]
    );
    return true;
  } catch (e: any) {
    if (isProgramFailedToComplete(e)) {
      throw new Error(
        `BPF STACK OVERFLOW (ProgramFailedToComplete) in ${label}: ${String(e)}`
      );
    }
    console.log(`    ${label} failed (non-BPF): ${String(e).slice(0, 150)}`);
    return false;
  }
};

/**
 * Run all POI interaction tests for a given session context.
 * Tests are ordered to maximize coverage:
 *   1. No-prereq, lightweight POIs first (L6, L7, L9) — minimizes risk of death
 *   2. Item acquisition POIs (L3, L2, L12, L13) — builds inventory
 *   3. Dependent POIs (L14, L4, L10) — need items/gold from earlier
 *   4. Heavy navigation POIs (L8, L11) — most likely to die en route
 *   5. Night-only POIs (L1, L5) — need phase transition
 * Uses findClosestPoi to pick the nearest POI of each type by Manhattan distance.
 */
const runPoiTests = (
  modeName: string,
  getCtx: () => SessionCtx,
  isL13Excluded: boolean
) => {
  let pois: PoiInstance[];
  let packedTiles: Buffer;
  let playerAlive = true;

  // Track acquired items for prerequisite POIs
  let hasToolId: number[] | null = null;
  let hasGearId: number[] | null = null;

  before(async function () {
    this.timeout(30_000);
    const ctx = getCtx();
    pois = await getPoiInstances(ctx.mapPoisPda);
    packedTiles = await fetchPackedTiles(ctx.generatedMapPda);
    console.log(
      `  ${modeName}: ${pois.length} POIs on map: [${pois.map((p) => `L${p.poiType}`).join(", ")}]`
    );
  });

  // Helper to get current player position
  const getPlayerPos = async (
    ctx: SessionCtx
  ): Promise<{ x: number; y: number; dead: boolean }> => {
    const gs = await fetchGameState(ctx.gameStatePda);
    return {
      x: Number(gs.positionX),
      y: Number(gs.positionY),
      dead: Boolean(gs.isDead),
    };
  };

  // ── L3 Tool Crate (acquire tool, guaranteed near spawn) ─────────────────
  it("L3 Tool Crate — acquire tool (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.TOOL_CRATE, pos.x, pos.y);
    if (!found) {
      console.log("    L3 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    // Step 1: generate_cache_offer
    const genIx = await buildGenerateCacheOffer(ctx, found.index);
    const genOk = await executePoi("L3-generate-offer", [genIx], ctx.sessionSigner);
    if (!genOk) return;

    // Step 2: interact_pick_item (pick first offer)
    const pickIx = await buildInteractPickItem(ctx, found.index, 0);
    const pickOk = await executePoi("L3-pick-item", [pickIx], ctx.sessionSigner);

    if (pickOk) {
      // Read inventory to remember the tool
      const inv = await fetchInventory(ctx.inventoryPda);
      if (inv.tool) {
        hasToolId = Array.from(inv.tool.itemId as number[]);
        console.log(`    Acquired tool: ${Buffer.from(inv.tool.itemId).toString("utf8").replace(/\0/g, "")}`);
      }
    }
  });

  // ── L2 Supply Cache (acquire gear, many instances near spawn) ──────────
  it("L2 Supply Cache — acquire gear (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.SUPPLY_CACHE, pos.x, pos.y);
    if (!found) {
      console.log("    L2 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    const genIx = await buildGenerateCacheOffer(ctx, found.index);
    const genOk = await executePoi("L2-generate-offer", [genIx], ctx.sessionSigner);
    if (!genOk) return;

    const pickIx = await buildInteractPickItem(ctx, found.index, 0);
    const pickOk = await executePoi("L2-pick-item", [pickIx], ctx.sessionSigner);

    if (pickOk) {
      const inv = await fetchInventory(ctx.inventoryPda);
      // Find first non-null gear slot
      for (const g of inv.gear) {
        if (g) {
          hasGearId = Array.from(g.itemId as number[]);
          console.log(`    Acquired gear: ${Buffer.from(g.itemId).toString("utf8").replace(/\0/g, "")}`);
          break;
        }
      }
    }
  });

  // ── L6 Survey Beacon (no prereqs, lightweight) ──────────────────────────
  it("L6 Survey Beacon — reveal tiles (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.SURVEY_BEACON, pos.x, pos.y);
    if (!found) {
      console.log("    L6 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    const ix = await buildInteractSurveyBeacon(ctx, found.index);
    await executePoi("L6-survey-beacon", [ix], ctx.sessionSigner);
  });

  // ── L7 Seismic Scanner (no prereqs, lightweight) ────────────────────────
  it("L7 Seismic Scanner — reveal nearest POI (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.SEISMIC_SCANNER, pos.x, pos.y);
    if (!found) {
      console.log("    L7 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    // category 0 = Items
    const ix = await buildInteractSeismicScanner(ctx, found.index, 0);
    await executePoi("L7-seismic-scanner", [ix], ctx.sessionSigner);
  });

  // ── L9 Smuggler Hatch (no prereqs, enter + leave) ──────────────────────
  it("L9 Smuggler Hatch — enter + leave shop (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.SMUGGLER_HATCH, pos.x, pos.y);
    if (!found) {
      console.log("    L9 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    const enterIx = await buildEnterShop(ctx, found.index);
    const enterOk = await executePoi("L9-enter-shop", [enterIx], ctx.sessionSigner);
    if (!enterOk) return;

    const leaveIx = await buildLeaveShop(ctx);
    await executePoi("L9-leave-shop", [leaveIx], ctx.sessionSigner);
  });

  // ── L12 Geode Vault (rare) ─────────────────────────────────────────────
  it("L12 Geode Vault — acquire heroic gear (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.GEODE_VAULT, pos.x, pos.y);
    if (!found) {
      console.log("    L12 not on map (rare), skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    const genIx = await buildGenerateCacheOffer(ctx, found.index);
    const genOk = await executePoi("L12-generate-offer", [genIx], ctx.sessionSigner);
    if (!genOk) return;

    const pickIx = await buildInteractPickItem(ctx, found.index, 0);
    await executePoi("L12-pick-item", [pickIx], ctx.sessionSigner);
  });

  // ── L13 Counter Cache (Campaign only) ──────────────────────────────────
  it("L13 Counter Cache — campaign only (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();

    if (isL13Excluded) {
      // Verify L13 is not on the map
      const found = findPoi(pois, POI.COUNTER_CACHE);
      expect(found).to.be.null;
      console.log("    L13 excluded from map (expected for Duel/Gauntlet)");
      return;
    }

    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.COUNTER_CACHE, pos.x, pos.y);
    if (!found) {
      console.log("    L13 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    const genIx = await buildGenerateCacheOffer(ctx, found.index);
    const genOk = await executePoi("L13-generate-offer", [genIx], ctx.sessionSigner);
    if (!genOk) return;

    const pickIx = await buildInteractPickItem(ctx, found.index, 0);
    await executePoi("L13-pick-item", [pickIx], ctx.sessionSigner);
  });

  // ── L14 Scrap Chute (needs gear) ───────────────────────────────────────
  it("L14 Scrap Chute — scrap gear (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    if (!hasGearId) {
      console.log("    L14 skipped — no gear in inventory");
      return this.skip();
    }

    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.SCRAP_CHUTE, pos.x, pos.y);
    if (!found) {
      console.log("    L14 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    // Re-check inventory for a gear item to scrap (might have changed)
    const inv = await fetchInventory(ctx.inventoryPda);
    let gearToScrap: number[] | null = null;
    for (const g of inv.gear) {
      if (g) {
        gearToScrap = Array.from(g.itemId as number[]);
        break;
      }
    }
    if (!gearToScrap) {
      console.log("    L14 skipped — no gear found in inventory");
      return this.skip();
    }

    const ix = await buildInteractScrapChute(ctx, found.index, gearToScrap);
    await executePoi("L14-scrap-chute", [ix], ctx.sessionSigner);
  });

  // ── L4 Tool Oil Rack (needs tool) ──────────────────────────────────────
  it("L4 Tool Oil Rack — modify tool (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    if (!hasToolId) {
      console.log("    L4 skipped — no tool in inventory");
      return this.skip();
    }

    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.TOOL_OIL_RACK, pos.x, pos.y);
    if (!found) {
      console.log("    L4 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    // Step 1: generate_oil_offer
    const genIx = await buildGenerateOilOffer(ctx, found.index);
    const genOk = await executePoi("L4-generate-oil-offer", [genIx], ctx.sessionSigner);
    if (!genOk) return;

    // Step 2: Read the generated oil offer and pick the first one
    const mp = await fetchMapPois(ctx.mapPoisPda);
    if (!mp.currentOilOffer) {
      console.log("    L4: No oil offer generated, skipping pick");
      return;
    }
    const oilFlag = mp.currentOilOffer.oils[0];

    const oilIx = await buildInteractToolOil(ctx, found.index, oilFlag);
    await executePoi("L4-tool-oil", [oilIx], ctx.sessionSigner);
  });

  // ── L10 Rusty Anvil (needs tool + gold) ────────────────────────────────
  it("L10 Rusty Anvil — upgrade tool (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    // Need a tool and 10 gold
    const inv = await fetchInventory(ctx.inventoryPda);
    if (!inv.tool) {
      console.log("    L10 skipped — no tool in inventory");
      return this.skip();
    }
    const gs = await fetchGameState(ctx.gameStatePda);
    if (Number(gs.gold) < 10) {
      console.log(`    L10 skipped — not enough gold (${gs.gold})`);
      return this.skip();
    }

    const found = findClosestPoi(pois, POI.RUSTY_ANVIL, pos.x, pos.y);
    if (!found) {
      console.log("    L10 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs2 = await fetchGameState(ctx.gameStatePda);
      if (gs2.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    // Re-fetch inventory after navigation (combat might have changed things)
    const inv2 = await fetchInventory(ctx.inventoryPda);
    if (!inv2.tool) {
      console.log("    L10 skipped — tool lost during navigation");
      return this.skip();
    }

    const toolId = Array.from(inv2.tool.itemId as number[]);
    const currentTier = 1; // Tier I

    const ix = await buildInteractRustyAnvil(ctx, found.index, toolId, currentTier);
    await executePoi("L10-rusty-anvil", [ix], ctx.sessionSigner);
  });

  // ── L8 Rail Waypoint (discover + fast travel) ──────────────────────────
  it("L8 Rail Waypoint — discover + fast travel (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const waypoints = findAllPoisOfType(pois, POI.RAIL_WAYPOINT);
    if (waypoints.length < 2) {
      console.log(`    L8: need 2 waypoints, found ${waypoints.length}, skipping`);
      return this.skip();
    }

    // Sort waypoints by distance from player, navigate to closest first
    waypoints.sort((a, b) => {
      const da = Math.abs(a.poi.x - pos.x) + Math.abs(a.poi.y - pos.y);
      const db = Math.abs(b.poi.x - pos.x) + Math.abs(b.poi.y - pos.y);
      return da - db;
    });

    // Navigate to first (closest) waypoint and discover
    const wp1 = waypoints[0];
    const arrived1 = await navigatePlayerTo(ctx, wp1.poi.x, wp1.poi.y, packedTiles);
    if (!arrived1) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    // Discover waypoints from current position
    const gs = await fetchGameState(ctx.gameStatePda);
    const isNight = gs.phase.night1 || gs.phase.night2 || gs.phase.night3;
    const visRadius = isNight ? 2 : 4;
    const discoverIx = await buildDiscoverVisibleWaypoints(ctx, visRadius);
    await executePoi("L8-discover-wp1", [discoverIx], ctx.sessionSigner);

    // Navigate to second waypoint and discover
    const wp2 = waypoints[1];
    const arrived2 = await navigatePlayerTo(ctx, wp2.poi.x, wp2.poi.y, packedTiles);
    if (!arrived2) {
      const gs2 = await fetchGameState(ctx.gameStatePda);
      if (gs2.isDead) { playerAlive = false; return this.skip(); }
      console.log("    L8: couldn't reach second waypoint, testing discover only");
      return;
    }

    const gs2 = await fetchGameState(ctx.gameStatePda);
    const isNight2 = gs2.phase.night1 || gs2.phase.night2 || gs2.phase.night3;
    const visRadius2 = isNight2 ? 2 : 4;
    const discoverIx2 = await buildDiscoverVisibleWaypoints(ctx, visRadius2);
    await executePoi("L8-discover-wp2", [discoverIx2], ctx.sessionSigner);

    // Check if both waypoints are discovered
    const updatedPois = await getPoiInstances(ctx.mapPoisPda);
    const discoveredWps = updatedPois.filter(
      (p) => p.poiType === POI.RAIL_WAYPOINT && p.discovered
    );
    if (discoveredWps.length < 2) {
      console.log(
        `    L8: only ${discoveredWps.length} waypoints discovered, skipping fast travel`
      );
      return;
    }

    // Fast travel from wp2 back to wp1
    const ftIx = await buildFastTravel(ctx, wp2.index, wp1.index);
    await executePoi("L8-fast-travel", [ftIx], ctx.sessionSigner);
  });

  // ── L11 Rune Kiln (needs 2 identical items) ────────────────────────────
  it("L11 Rune Kiln — fuse items (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();
    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.RUNE_KILN, pos.x, pos.y);
    if (!found) {
      console.log("    L11 not on map (rare), skipping");
      return this.skip();
    }

    // Need 2 identical gear items — check inventory
    const inv = await fetchInventory(ctx.inventoryPda);
    let fusePair: { itemId: number[]; tier: number } | null = null;
    const gearCounts = new Map<string, { itemId: number[]; tier: number; count: number }>();

    for (const g of inv.gear) {
      if (!g) continue;
      const tierVal =
        typeof g.tier === "object"
          ? Object.keys(g.tier)[0] === "i"
            ? 1
            : Object.keys(g.tier)[0] === "ii"
              ? 2
              : 3
          : Number(g.tier);
      const key = `${Buffer.from(g.itemId).toString("hex")}-${tierVal}`;
      const existing = gearCounts.get(key);
      if (existing) {
        existing.count++;
        if (existing.count >= 2) {
          fusePair = { itemId: Array.from(g.itemId as number[]), tier: tierVal };
          break;
        }
      } else {
        gearCounts.set(key, { itemId: Array.from(g.itemId as number[]), tier: tierVal, count: 1 });
      }
    }

    if (!fusePair) {
      console.log("    L11 skipped — no 2 identical gear items");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs = await fetchGameState(ctx.gameStatePda);
      if (gs.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    const ix = await buildInteractRuneKiln(
      ctx,
      found.index,
      fusePair.itemId,
      fusePair.tier,
      fusePair.itemId,
      fusePair.tier
    );
    await executePoi("L11-rune-kiln", [ix], ctx.sessionSigner);
  });

  // ── Night POIs (L1 Mole Den, L5 Rest Alcove) ──────────────────────────
  it("exhaust day moves to reach Night phase", async function () {
    this.timeout(300_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();

    const gs = await fetchGameState(ctx.gameStatePda);
    if (gs.isDead) { playerAlive = false; return this.skip(); }
    const isNight = gs.phase.night1 || gs.phase.night2 || gs.phase.night3;
    if (isNight) {
      console.log("    Already in night phase");
      return;
    }

    const reached = await exhaustMovesToNight(ctx, packedTiles);
    if (!reached) {
      const gs2 = await fetchGameState(ctx.gameStatePda);
      if (gs2.isDead) { playerAlive = false; return this.skip(); }
      console.log("    Could not reach night phase");
      return this.skip();
    }
    console.log("    Transitioned to night phase");
  });

  it("L1 Mole Den — night rest full heal (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();

    // Verify we're in night phase
    const gs = await fetchGameState(ctx.gameStatePda);
    if (gs.isDead) { playerAlive = false; return this.skip(); }
    const isNight = gs.phase.night1 || gs.phase.night2 || gs.phase.night3;
    if (!isNight) {
      console.log("    Not in night phase, skipping L1");
      return this.skip();
    }

    // L1 Mole Den is guaranteed on the map (Fixed rarity)
    // Find closest Mole Den from current position
    const posX = Number(gs.positionX);
    const posY = Number(gs.positionY);
    const allL1 = findAllPoisOfType(pois, POI.MOLE_DEN);
    if (allL1.length === 0) {
      console.log("    L1 not on map (unexpected), skipping");
      return this.skip();
    }
    allL1.sort((a, b) => {
      const da = Math.abs(a.poi.x - posX) + Math.abs(a.poi.y - posY);
      const db = Math.abs(b.poi.x - posX) + Math.abs(b.poi.y - posY);
      return da - db;
    });
    const found = allL1[0];

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs2 = await fetchGameState(ctx.gameStatePda);
      if (gs2.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    // Verify still in night (navigation might have consumed all night moves)
    const gs2 = await fetchGameState(ctx.gameStatePda);
    const stillNight = gs2.phase.night1 || gs2.phase.night2 || gs2.phase.night3;
    if (!stillNight) {
      console.log("    No longer in night phase after navigation, skipping L1");
      return this.skip();
    }

    const ix = await buildInteractRest(ctx, found.index);
    await executePoi("L1-mole-den", [ix], ctx.sessionSigner);
  });

  it("L5 Rest Alcove — night partial heal (no BPF crash)", async function () {
    this.timeout(120_000);
    if (!playerAlive) return this.skip();
    const ctx = getCtx();

    const gs = await fetchGameState(ctx.gameStatePda);
    if (gs.isDead) { playerAlive = false; return this.skip(); }

    // L1 (interact_rest with skip_to_day) may have pushed us back to day
    const isNight = gs.phase.night1 || gs.phase.night2 || gs.phase.night3;
    if (!isNight) {
      // Try exhausting moves again to reach the next night phase
      const reached = await exhaustMovesToNight(ctx, packedTiles);
      if (!reached) {
        const gs2 = await fetchGameState(ctx.gameStatePda);
        if (gs2.isDead) { playerAlive = false; return this.skip(); }
        console.log("    Could not reach night phase for L5");
        return this.skip();
      }
    }

    const pos = await getPlayerPos(ctx);
    if (pos.dead) { playerAlive = false; return this.skip(); }

    const found = findClosestPoi(pois, POI.REST_ALCOVE, pos.x, pos.y);
    if (!found) {
      console.log("    L5 not on map, skipping");
      return this.skip();
    }

    const arrived = await navigatePlayerTo(ctx, found.poi.x, found.poi.y, packedTiles);
    if (!arrived) {
      const gs2 = await fetchGameState(ctx.gameStatePda);
      if (gs2.isDead) { playerAlive = false; return this.skip(); }
      return this.skip();
    }

    const gs3 = await fetchGameState(ctx.gameStatePda);
    const stillNight = gs3.phase.night1 || gs3.phase.night2 || gs3.phase.night3;
    if (!stillNight) {
      console.log("    No longer in night phase after navigation, skipping L5");
      return this.skip();
    }

    const ix = await buildInteractRest(ctx, found.index);
    await executePoi("L5-rest-alcove", [ix], ctx.sessionSigner);
  });
};

// ── Global Setup ─────────────────────────────────────────────────────────────
before(async function () {
  this.timeout(60_000);

  admin = loadWalletKeypair();
  const wallet = walletFromKeypair(admin);
  connection = new Connection(RPC_URL, "confirmed");
  provider = createProvider(RPC_URL, wallet);
  anchor.setProvider(provider);
  programs = loadAllPrograms(provider);

  // Global PDAs
  [sessionCounterPda] = getSessionCounterPda();
  [mapConfigPda] = getMapConfigPda();
  [gameplayAuthorityPda] = getGameplayAuthorityPda();
  [sessionManagerAuthorityPda] = getSessionManagerAuthorityPda();
  [poiAuthorityPda] = getPoiAuthorityPda();
  [inventoryAuthorityPda] = getInventoryAuthorityPda();
  [duelVaultPda] = getDuelVaultPda();
  [duelOpenQueuePda] = getDuelOpenQueuePda();
  [pitDraftQueuePda] = getPitDraftQueuePda();
  [pitDraftVaultPda] = getPitDraftVaultPda();
  [gauntletConfigPda] = getGauntletConfigPda();
  [gauntletPoolVaultPda] = getGauntletPoolVaultPda();
  [gauntletWeek1Pda] = getGauntletWeekPoolPda(1);
  [gauntletWeek2Pda] = getGauntletWeekPoolPda(2);
  [gauntletWeek3Pda] = getGauntletWeekPoolPda(3);
  [gauntletWeek4Pda] = getGauntletWeekPoolPda(4);
  [gauntletWeek5Pda] = getGauntletWeekPoolPda(5);
});

// ═════════════════════════════════════════════════════════════════════════════
// 1. Initialize global state (idempotent)
// ═════════════════════════════════════════════════════════════════════════════
describe("POI E2E: Initialize global state", function () {
  this.timeout(60_000);

  it("initializes all global accounts", async () => {
    try {
      await programs.sessionManager.methods
        .initializeCounter()
        .accounts({
          sessionCounter: sessionCounterPda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (e: any) {
      if (!String(e).includes("already in use")) throw e;
    }

    try {
      await programs.mapGenerator.methods
        .initializeMapConfig()
        .accounts({
          mapConfig: mapConfigPda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (e: any) {
      if (!String(e).includes("already in use")) throw e;
    }

    try {
      await programs.gameplayState.methods
        .initializeDuels()
        .accounts({
          duelVault: duelVaultPda,
          duelOpenQueue: duelOpenQueuePda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (e: any) {
      if (!String(e).includes("already in use")) throw e;
    }

    try {
      await programs.gameplayState.methods
        .initializePitDraft()
        .accounts({
          pitDraftQueue: pitDraftQueuePda,
          pitDraftVault: pitDraftVaultPda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .rpc();
    } catch (e: any) {
      if (!String(e).includes("already in use")) throw e;
    }

    try {
      await programs.gameplayState.methods
        .initializeGauntlet()
        .accounts({
          gauntletConfig: gauntletConfigPda,
          gauntletPoolVault: gauntletPoolVaultPda,
          gauntletWeek1: gauntletWeek1Pda,
          gauntletWeek2: gauntletWeek2Pda,
          gauntletWeek3: gauntletWeek3Pda,
          gauntletWeek4: gauntletWeek4Pda,
          gauntletWeek5: gauntletWeek5Pda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        } as any)
        .preInstructions([
          ComputeBudgetProgram.setComputeUnitLimit({ units: SESSION_CU_LIMIT }),
        ])
        .rpc();
    } catch (e: any) {
      if (!String(e).includes("already in use")) throw e;
    }

    const info = await connection.getAccountInfo(sessionCounterPda, "confirmed");
    expect(info).to.not.be.null;
  });
});

// ═════════════════════════════════════════════════════════════════════════════
// 2. Context Struct Smoke Test (no navigation required)
//    Calls each POI instruction from spawn point. Business logic errors are
//    expected and fine — we only fail if ProgramFailedToComplete (BPF stack
//    overflow), which would mean the Box<Account> fix is broken.
// ═════════════════════════════════════════════════════════════════════════════
describe("POI E2E: Box<Account> Smoke Test — Campaign", function () {
  this.timeout(120_000);

  let ctx: SessionCtx;
  let pois: PoiInstance[];

  before(async function () {
    this.timeout(60_000);
    const user = Keypair.generate();
    const sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(
      connection,
      sessionSigner.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    const campaignLevel = 1;
    const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    const [sessionPda] = getSessionPda(user.publicKey, campaignLevel);
    const [gameStatePda] = getGameStatePda(sessionPda);
    const [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    const [generatedMapPda] = getGeneratedMapPda(sessionPda);
    const [inventoryPda] = getInventoryPda(sessionPda);
    const [mapPoisPda] = getMapPoisPda(sessionPda);

    ctx = {
      user,
      sessionSigner,
      playerProfilePda,
      sessionPda,
      gameStatePda,
      mapEnemiesPda,
      generatedMapPda,
      inventoryPda,
      mapPoisPda,
      campaignLevel,
    };

    await createProfileAndCampaignSession(ctx);
    pois = await getPoiInstances(ctx.mapPoisPda);
    console.log(
      `  Smoke test POIs: [${pois.map((p) => `L${p.poiType}`).join(", ")}]`
    );
  });

  const findAnyPoiOfType = (type: number): number | null => {
    const idx = pois.findIndex((p) => p.poiType === type);
    return idx >= 0 ? idx : null;
  };

  it("InteractRest (L1/L5) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.MOLE_DEN) ?? findAnyPoiOfType(POI.REST_ALCOVE);
    if (idx === null) return this.skip();
    const ix = await buildInteractRest(ctx, idx);
    await executePoi("smoke-InteractRest", [ix], ctx.sessionSigner);
  });

  it("GenerateCacheOffer (L2/L3/L12/L13) — no BPF stack overflow", async function () {
    const idx =
      findAnyPoiOfType(POI.TOOL_CRATE) ??
      findAnyPoiOfType(POI.SUPPLY_CACHE) ??
      findAnyPoiOfType(POI.GEODE_VAULT) ??
      findAnyPoiOfType(POI.COUNTER_CACHE);
    if (idx === null) return this.skip();
    const ix = await buildGenerateCacheOffer(ctx, idx);
    await executePoi("smoke-GenerateCacheOffer", [ix], ctx.sessionSigner);
  });

  it("InteractPickItem (L2/L3/L12/L13) — no BPF stack overflow", async function () {
    const idx =
      findAnyPoiOfType(POI.TOOL_CRATE) ??
      findAnyPoiOfType(POI.SUPPLY_CACHE) ??
      findAnyPoiOfType(POI.GEODE_VAULT) ??
      findAnyPoiOfType(POI.COUNTER_CACHE);
    if (idx === null) return this.skip();
    const ix = await buildInteractPickItem(ctx, idx, 0);
    await executePoi("smoke-InteractPickItem", [ix], ctx.sessionSigner);
  });

  it("GenerateOilOffer (L4) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.TOOL_OIL_RACK);
    if (idx === null) return this.skip();
    const ix = await buildGenerateOilOffer(ctx, idx);
    await executePoi("smoke-GenerateOilOffer", [ix], ctx.sessionSigner);
  });

  it("InteractToolOil (L4) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.TOOL_OIL_RACK);
    if (idx === null) return this.skip();
    const ix = await buildInteractToolOil(ctx, idx, 0);
    await executePoi("smoke-InteractToolOil", [ix], ctx.sessionSigner);
  });

  it("InteractSurveyBeacon (L6) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SURVEY_BEACON);
    if (idx === null) return this.skip();
    const ix = await buildInteractSurveyBeacon(ctx, idx);
    await executePoi("smoke-InteractSurveyBeacon", [ix], ctx.sessionSigner);
  });

  it("InteractSeismicScanner (L7) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SEISMIC_SCANNER);
    if (idx === null) return this.skip();
    const ix = await buildInteractSeismicScanner(ctx, idx, 0);
    await executePoi("smoke-InteractSeismicScanner", [ix], ctx.sessionSigner);
  });

  it("DiscoverVisibleWaypoints (L8) — no BPF stack overflow", async function () {
    const ix = await buildDiscoverVisibleWaypoints(ctx, 4);
    await executePoi("smoke-DiscoverWaypoints", [ix], ctx.sessionSigner);
  });

  it("FastTravel (L8) — no BPF stack overflow", async function () {
    const waypoints = pois
      .map((p, i) => ({ poiType: p.poiType, index: i }))
      .filter((p) => p.poiType === POI.RAIL_WAYPOINT);
    if (waypoints.length < 2) return this.skip();
    const ix = await buildFastTravel(ctx, waypoints[0].index, waypoints[1].index);
    await executePoi("smoke-FastTravel", [ix], ctx.sessionSigner);
  });

  it("EnterShop (L9) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SMUGGLER_HATCH);
    if (idx === null) return this.skip();
    const ix = await buildEnterShop(ctx, idx);
    await executePoi("smoke-EnterShop", [ix], ctx.sessionSigner);
  });

  it("LeaveShop (L9) — no BPF stack overflow", async function () {
    const ix = await buildLeaveShop(ctx);
    await executePoi("smoke-LeaveShop", [ix], ctx.sessionSigner);
  });

  it("InteractRustyAnvil (L10) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.RUSTY_ANVIL);
    if (idx === null) return this.skip();
    // dummy item ID and tier — will fail with business logic error, not BPF overflow
    const ix = await buildInteractRustyAnvil(ctx, idx, [0, 0, 0, 0, 0, 0], 1);
    await executePoi("smoke-InteractRustyAnvil", [ix], ctx.sessionSigner);
  });

  it("InteractRuneKiln (L11) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.RUNE_KILN);
    if (idx === null) return this.skip();
    const dummyId = [0, 0, 0, 0, 0, 0];
    const ix = await buildInteractRuneKiln(ctx, idx, dummyId, 1, dummyId, 1);
    await executePoi("smoke-InteractRuneKiln", [ix], ctx.sessionSigner);
  });

  it("InteractScrapChute (L14) — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SCRAP_CHUTE);
    if (idx === null) return this.skip();
    const ix = await buildInteractScrapChute(ctx, idx, [0, 0, 0, 0, 0, 0]);
    await executePoi("smoke-InteractScrapChute", [ix], ctx.sessionSigner);
  });

  after(async function () {
    this.timeout(30_000);
    if (ctx) await endSession(ctx);
  });
});

// Repeat smoke test for Gauntlet mode (different account layout / higher enemy scaling)
describe("POI E2E: Box<Account> Smoke Test — Gauntlet", function () {
  this.timeout(180_000);

  let ctx: SessionCtx;
  let pois: PoiInstance[];

  before(async function () {
    this.timeout(120_000);
    const user = Keypair.generate();
    const sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(
      connection,
      sessionSigner.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    const campaignLevel = 20;
    const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    const [sessionPda] = getGauntletSessionPda(user.publicKey);
    const [gameStatePda] = getGameStatePda(sessionPda);
    const [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    const [generatedMapPda] = getGeneratedMapPda(sessionPda);
    const [inventoryPda] = getInventoryPda(sessionPda);
    const [mapPoisPda] = getMapPoisPda(sessionPda);

    ctx = {
      user,
      sessionSigner,
      playerProfilePda,
      sessionPda,
      gameStatePda,
      mapEnemiesPda,
      generatedMapPda,
      inventoryPda,
      mapPoisPda,
      campaignLevel,
    };

    await createProfileAndGauntletSession(ctx);
    pois = await getPoiInstances(ctx.mapPoisPda);
    console.log(
      `  Gauntlet smoke test POIs: [${pois.map((p) => `L${p.poiType}`).join(", ")}]`
    );
  });

  const findAnyPoiOfType = (type: number): number | null => {
    const idx = pois.findIndex((p) => p.poiType === type);
    return idx >= 0 ? idx : null;
  };

  it("InteractRest — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.MOLE_DEN) ?? findAnyPoiOfType(POI.REST_ALCOVE);
    if (idx === null) return this.skip();
    const ix = await buildInteractRest(ctx, idx);
    await executePoi("gauntlet-InteractRest", [ix], ctx.sessionSigner);
  });

  it("GenerateCacheOffer — no BPF stack overflow", async function () {
    const idx =
      findAnyPoiOfType(POI.TOOL_CRATE) ??
      findAnyPoiOfType(POI.SUPPLY_CACHE);
    if (idx === null) return this.skip();
    const ix = await buildGenerateCacheOffer(ctx, idx);
    await executePoi("gauntlet-GenerateCacheOffer", [ix], ctx.sessionSigner);
  });

  it("InteractPickItem — no BPF stack overflow", async function () {
    const idx =
      findAnyPoiOfType(POI.TOOL_CRATE) ??
      findAnyPoiOfType(POI.SUPPLY_CACHE);
    if (idx === null) return this.skip();
    const ix = await buildInteractPickItem(ctx, idx, 0);
    await executePoi("gauntlet-InteractPickItem", [ix], ctx.sessionSigner);
  });

  it("GenerateOilOffer — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.TOOL_OIL_RACK);
    if (idx === null) return this.skip();
    const ix = await buildGenerateOilOffer(ctx, idx);
    await executePoi("gauntlet-GenerateOilOffer", [ix], ctx.sessionSigner);
  });

  it("InteractToolOil — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.TOOL_OIL_RACK);
    if (idx === null) return this.skip();
    const ix = await buildInteractToolOil(ctx, idx, 0);
    await executePoi("gauntlet-InteractToolOil", [ix], ctx.sessionSigner);
  });

  it("InteractSurveyBeacon — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SURVEY_BEACON);
    if (idx === null) return this.skip();
    const ix = await buildInteractSurveyBeacon(ctx, idx);
    await executePoi("gauntlet-InteractSurveyBeacon", [ix], ctx.sessionSigner);
  });

  it("InteractSeismicScanner — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SEISMIC_SCANNER);
    if (idx === null) return this.skip();
    const ix = await buildInteractSeismicScanner(ctx, idx, 0);
    await executePoi("gauntlet-InteractSeismicScanner", [ix], ctx.sessionSigner);
  });

  it("DiscoverVisibleWaypoints — no BPF stack overflow", async function () {
    const ix = await buildDiscoverVisibleWaypoints(ctx, 4);
    await executePoi("gauntlet-DiscoverWaypoints", [ix], ctx.sessionSigner);
  });

  it("FastTravel — no BPF stack overflow", async function () {
    const waypoints = pois
      .map((p, i) => ({ poiType: p.poiType, index: i }))
      .filter((p) => p.poiType === POI.RAIL_WAYPOINT);
    if (waypoints.length < 2) return this.skip();
    const ix = await buildFastTravel(ctx, waypoints[0].index, waypoints[1].index);
    await executePoi("gauntlet-FastTravel", [ix], ctx.sessionSigner);
  });

  it("EnterShop — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SMUGGLER_HATCH);
    if (idx === null) return this.skip();
    const ix = await buildEnterShop(ctx, idx);
    await executePoi("gauntlet-EnterShop", [ix], ctx.sessionSigner);
  });

  it("LeaveShop — no BPF stack overflow", async function () {
    const ix = await buildLeaveShop(ctx);
    await executePoi("gauntlet-LeaveShop", [ix], ctx.sessionSigner);
  });

  it("InteractRustyAnvil — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.RUSTY_ANVIL);
    if (idx === null) return this.skip();
    const ix = await buildInteractRustyAnvil(ctx, idx, [0, 0, 0, 0, 0, 0], 1);
    await executePoi("gauntlet-InteractRustyAnvil", [ix], ctx.sessionSigner);
  });

  it("InteractRuneKiln — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.RUNE_KILN);
    if (idx === null) return this.skip();
    const dummyId = [0, 0, 0, 0, 0, 0];
    const ix = await buildInteractRuneKiln(ctx, idx, dummyId, 1, dummyId, 1);
    await executePoi("gauntlet-InteractRuneKiln", [ix], ctx.sessionSigner);
  });

  it("InteractScrapChute — no BPF stack overflow", async function () {
    const idx = findAnyPoiOfType(POI.SCRAP_CHUTE);
    if (idx === null) return this.skip();
    const ix = await buildInteractScrapChute(ctx, idx, [0, 0, 0, 0, 0, 0]);
    await executePoi("gauntlet-InteractScrapChute", [ix], ctx.sessionSigner);
  });

  after(async function () {
    this.timeout(30_000);
    if (ctx) await endSession(ctx);
  });
});

// ═════════════════════════════════════════════════════════════════════════════
// 3. Campaign Mode — all 14 POIs (with navigation)
// ═════════════════════════════════════════════════════════════════════════════
describe("POI E2E: Campaign Mode", function () {
  this.timeout(600_000);

  let ctx: SessionCtx;

  before(async function () {
    this.timeout(60_000);
    const user = Keypair.generate();
    const sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(
      connection,
      sessionSigner.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    const campaignLevel = 1;
    const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    const [sessionPda] = getSessionPda(user.publicKey, campaignLevel);
    const [gameStatePda] = getGameStatePda(sessionPda);
    const [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    const [generatedMapPda] = getGeneratedMapPda(sessionPda);
    const [inventoryPda] = getInventoryPda(sessionPda);
    const [mapPoisPda] = getMapPoisPda(sessionPda);

    ctx = {
      user,
      sessionSigner,
      playerProfilePda,
      sessionPda,
      gameStatePda,
      mapEnemiesPda,
      generatedMapPda,
      inventoryPda,
      mapPoisPda,
      campaignLevel,
    };

    await createProfileAndCampaignSession(ctx);
    console.log("  Campaign session started");
  });

  runPoiTests("Campaign", () => ctx, false);

  after(async function () {
    this.timeout(30_000);
    if (ctx) await endSession(ctx);
  });
});

// ═════════════════════════════════════════════════════════════════════════════
// 4. Duel Mode — 13 POIs (with navigation)
// ═════════════════════════════════════════════════════════════════════════════
describe("POI E2E: Duel Mode", function () {
  this.timeout(600_000);

  let ctx: SessionCtx;

  before(async function () {
    this.timeout(60_000);
    const user = Keypair.generate();
    const sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(
      connection,
      sessionSigner.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    const campaignLevel = 20; // DUEL_CAMPAIGN_LEVEL
    const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    const [sessionPda] = getDuelSessionPda(user.publicKey);
    const [gameStatePda] = getGameStatePda(sessionPda);
    const [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    const [generatedMapPda] = getGeneratedMapPda(sessionPda);
    const [inventoryPda] = getInventoryPda(sessionPda);
    const [mapPoisPda] = getMapPoisPda(sessionPda);

    ctx = {
      user,
      sessionSigner,
      playerProfilePda,
      sessionPda,
      gameStatePda,
      mapEnemiesPda,
      generatedMapPda,
      inventoryPda,
      mapPoisPda,
      campaignLevel,
    };

    await createProfileAndDuelSession(ctx);
    console.log("  Duel session started");
  });

  runPoiTests("Duel", () => ctx, true);

  after(async function () {
    this.timeout(30_000);
    if (ctx) await endSession(ctx);
  });
});

// ═════════════════════════════════════════════════════════════════════════════
// 5. Gauntlet Mode — 13 POIs (with navigation)
// ═════════════════════════════════════════════════════════════════════════════
describe("POI E2E: Gauntlet Mode", function () {
  this.timeout(600_000);

  let ctx: SessionCtx;

  before(async function () {
    this.timeout(120_000);
    const user = Keypair.generate();
    const sessionSigner = Keypair.generate();
    await airdropAndConfirm(connection, user.publicKey, 10 * LAMPORTS_PER_SOL);
    await airdropAndConfirm(
      connection,
      sessionSigner.publicKey,
      10 * LAMPORTS_PER_SOL
    );

    const campaignLevel = 20; // GAUNTLET_CAMPAIGN_LEVEL
    const [playerProfilePda] = getPlayerProfilePda(user.publicKey);
    const [sessionPda] = getGauntletSessionPda(user.publicKey);
    const [gameStatePda] = getGameStatePda(sessionPda);
    const [mapEnemiesPda] = getMapEnemiesPda(sessionPda);
    const [generatedMapPda] = getGeneratedMapPda(sessionPda);
    const [inventoryPda] = getInventoryPda(sessionPda);
    const [mapPoisPda] = getMapPoisPda(sessionPda);

    ctx = {
      user,
      sessionSigner,
      playerProfilePda,
      sessionPda,
      gameStatePda,
      mapEnemiesPda,
      generatedMapPda,
      inventoryPda,
      mapPoisPda,
      campaignLevel,
    };

    await createProfileAndGauntletSession(ctx);
    console.log("  Gauntlet session started (with enterGauntlet)");
  });

  runPoiTests("Gauntlet", () => ctx, true);

  after(async function () {
    this.timeout(30_000);
    if (ctx) await endSession(ctx);
  });
});
