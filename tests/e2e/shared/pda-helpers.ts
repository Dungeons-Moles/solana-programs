import { PublicKey } from "@solana/web3.js";
import { PROGRAM_IDS } from "./setup";

// ── Session Manager PDAs ────────────────────────────────────────────────────
export function getSessionCounterPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("session_counter")],
    PROGRAM_IDS.sessionManager
  );
}

export function getSessionPda(
  player: PublicKey,
  campaignLevel: number
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("session"), player.toBuffer(), Buffer.from([campaignLevel])],
    PROGRAM_IDS.sessionManager
  );
}

export function getDuelSessionPda(player: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("duel_session"), player.toBuffer()],
    PROGRAM_IDS.sessionManager
  );
}

export function getSessionManagerAuthorityPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("session_manager_authority")],
    PROGRAM_IDS.sessionManager
  );
}

// ── Player Profile PDAs ─────────────────────────────────────────────────────
export function getPlayerProfilePda(owner: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("player"), owner.toBuffer()],
    PROGRAM_IDS.playerProfile
  );
}

// ── Map Generator PDAs ──────────────────────────────────────────────────────
export function getMapConfigPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("map_config")],
    PROGRAM_IDS.mapGenerator
  );
}

export function getGeneratedMapPda(
  sessionPda: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("generated_map"), sessionPda.toBuffer()],
    PROGRAM_IDS.mapGenerator
  );
}

// ── Gameplay State PDAs ─────────────────────────────────────────────────────
export function getGameStatePda(sessionPda: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("game_state"), sessionPda.toBuffer()],
    PROGRAM_IDS.gameplayState
  );
}

export function getMapEnemiesPda(sessionPda: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("map_enemies"), sessionPda.toBuffer()],
    PROGRAM_IDS.gameplayState
  );
}

export function getGameplayAuthorityPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("gameplay_authority")],
    PROGRAM_IDS.gameplayState
  );
}

export function getDuelVaultPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("duel_vault")],
    PROGRAM_IDS.gameplayState
  );
}

export function getDuelOpenQueuePda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("duel_open_queue")],
    PROGRAM_IDS.gameplayState
  );
}

export function getDuelEntryPda(sessionPda: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("duel_entry"), sessionPda.toBuffer()],
    PROGRAM_IDS.gameplayState
  );
}

export function getPitDraftQueuePda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pit_draft_queue")],
    PROGRAM_IDS.gameplayState
  );
}

export function getPitDraftVaultPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pit_draft_vault")],
    PROGRAM_IDS.gameplayState
  );
}

export function getGauntletConfigPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_config")],
    PROGRAM_IDS.gameplayState
  );
}

export function getGauntletPoolVaultPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_pool_vault")],
    PROGRAM_IDS.gameplayState
  );
}

export function getGauntletWeekPoolPda(week: number): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("gauntlet_week_pool"), Buffer.from([week])],
    PROGRAM_IDS.gameplayState
  );
}

// ── Player Inventory PDAs ───────────────────────────────────────────────────
export function getInventoryPda(sessionPda: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("inventory"), sessionPda.toBuffer()],
    PROGRAM_IDS.playerInventory
  );
}

// ── POI System PDAs ─────────────────────────────────────────────────────────
export function getMapPoisPda(sessionPda: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("map_pois"), sessionPda.toBuffer()],
    PROGRAM_IDS.poiSystem
  );
}

// ── NFT Marketplace PDAs ────────────────────────────────────────────────────
export function getMarketplaceConfigPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("marketplace_config")],
    PROGRAM_IDS.nftMarketplace
  );
}

export function getMintAuthorityPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("mint_authority")],
    PROGRAM_IDS.nftMarketplace
  );
}

export function getListingPda(assetPubkey: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("listing"), assetPubkey.toBuffer()],
    PROGRAM_IDS.nftMarketplace
  );
}

export function getQuestDefinitionPda(questId: number): [PublicKey, number] {
  const buf = Buffer.alloc(2);
  buf.writeUInt16LE(questId);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("quest_def"), buf],
    PROGRAM_IDS.nftMarketplace
  );
}

export function getQuestProgressPda(
  player: PublicKey,
  questId: number
): [PublicKey, number] {
  const buf = Buffer.alloc(2);
  buf.writeUInt16LE(questId);
  return PublicKey.findProgramAddressSync(
    [Buffer.from("quest_progress"), player.toBuffer(), buf],
    PROGRAM_IDS.nftMarketplace
  );
}

// ── Delegation PDAs ─────────────────────────────────────────────────────────
export function deriveDelegateAccounts(
  target: PublicKey,
  ownerProgram: PublicKey
): {
  buffer: PublicKey;
  delegationRecord: PublicKey;
  delegationMetadata: PublicKey;
} {
  const [buffer] = PublicKey.findProgramAddressSync(
    [Buffer.from("buffer"), target.toBuffer()],
    ownerProgram
  );
  const [delegationRecord] = PublicKey.findProgramAddressSync(
    [Buffer.from("delegation"), target.toBuffer()],
    PROGRAM_IDS.delegation
  );
  const [delegationMetadata] = PublicKey.findProgramAddressSync(
    [Buffer.from("delegation-metadata"), target.toBuffer()],
    PROGRAM_IDS.delegation
  );
  return { buffer, delegationRecord, delegationMetadata };
}
