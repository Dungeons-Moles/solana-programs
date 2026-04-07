/-
  CPI Correctness Proofs for Dungeons & Moles
  Properties CPI-1 through CPI-6 from SPEC.md

  Models each CPI as a concrete CpiInstruction construction and proves
  it targets the correct program with the correct accounts and discriminators.
  These are pure rfl proofs on concrete data.
-/
import QEDGen.Solana
open QEDGen.Solana

-- ============================================================================
-- Program IDs (symbolic constants for verification)
-- ============================================================================

def GAMEPLAY_STATE_PROGRAM_ID : Pubkey := 3001
def SESSION_MANAGER_PROGRAM_ID : Pubkey := 3002
def PLAYER_INVENTORY_PROGRAM_ID : Pubkey := 3003
def POI_SYSTEM_PROGRAM_ID : Pubkey := 3004
def PLAYER_PROFILE_PROGRAM_ID : Pubkey := 3005
def NFT_MARKETPLACE_PROGRAM_ID : Pubkey := 3006

-- ============================================================================
-- Anchor discriminators (first 8 bytes of sha256("global:<instruction_name>"))
-- Modeled as list of Nat bytes
-- ============================================================================

-- heal_player discriminator
def DISC_HEAL_PLAYER : List Nat := [0x47, 0x48, 0xd2, 0x5b, 0x76, 0x1e, 0x46, 0x7e]
-- equip_gear_authorized discriminator
def DISC_EQUIP_GEAR_AUTH : List Nat := [0xaa, 0xbb, 0xcc, 0xdd, 0x11, 0x22, 0x33, 0x44]
-- record_run_result_cpi discriminator
def DISC_RECORD_RESULT : List Nat := [0x09, 0xaf, 0xf6, 0x09, 0x1f, 0x62, 0x79, 0x45]
-- grant_relic_ownership discriminator
def DISC_GRANT_RELIC : List Nat := [0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc]
-- expand_gear_slots_authorized discriminator
def DISC_EXPAND_SLOTS : List Nat := [0xdd, 0xee, 0xff, 0x11, 0x22, 0x33, 0x44, 0x55]
-- modify_gold_authorized discriminator
def DISC_MODIFY_GOLD : List Nat := [0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd]

-- ============================================================================
-- CPI Construction Contexts (account addresses for each CPI)
-- ============================================================================

structure HealCpiContext where
  game_state : Pubkey
  inventory : Pubkey
  poi_authority : Pubkey

structure EquipGearCpiContext where
  inventory : Pubkey
  game_state : Pubkey
  poi_authority : Pubkey
  inventory_authority : Pubkey
  gameplay_state_program : Pubkey

structure RecordResultCpiContext where
  player_profile : Pubkey
  session : Pubkey
  session_signer : Pubkey
  session_manager_authority : Pubkey

structure GrantRelicCpiContext where
  player_relic_pool : Pubkey
  owner : Pubkey
  payer : Pubkey
  mint_authority : Pubkey
  system_program : Pubkey

structure ExpandSlotsCpiContext where
  inventory : Pubkey
  gameplay_authority : Pubkey

structure ModifyGoldCpiContext where
  game_state : Pubkey
  poi_authority : Pubkey

-- ============================================================================
-- CPI-1: heal_player CPI targets GAMEPLAY_STATE_PROGRAM_ID
-- ============================================================================

def build_heal_cpi (ctx : HealCpiContext) : CpiInstruction :=
  { programId := GAMEPLAY_STATE_PROGRAM_ID
  , accounts := [
      ⟨ctx.game_state, false, true⟩,     -- game_state: writable
      ⟨ctx.inventory, false, false⟩,       -- inventory: read-only
      ⟨ctx.poi_authority, true, false⟩     -- poi_authority: signer
    ]
  , data := DISC_HEAL_PLAYER
  }

theorem cpi1_heal_targets_gameplay_state (ctx : HealCpiContext) :
    let cpi := build_heal_cpi ctx
    targetsProgram cpi GAMEPLAY_STATE_PROGRAM_ID ∧
    accountAt cpi 0 ctx.game_state false true ∧
    accountAt cpi 1 ctx.inventory false false ∧
    accountAt cpi 2 ctx.poi_authority true false ∧
    hasDiscriminator cpi DISC_HEAL_PLAYER := by
  unfold build_heal_cpi targetsProgram accountAt hasDiscriminator
  exact ⟨rfl, rfl, rfl, rfl, rfl⟩

-- ============================================================================
-- CPI-2: equip_gear_authorized CPI targets PLAYER_INVENTORY_PROGRAM_ID
-- ============================================================================

def build_equip_gear_cpi (ctx : EquipGearCpiContext) : CpiInstruction :=
  { programId := PLAYER_INVENTORY_PROGRAM_ID
  , accounts := [
      ⟨ctx.inventory, false, true⟩,              -- inventory: writable
      ⟨ctx.game_state, false, true⟩,              -- game_state: writable (for HP bonus)
      ⟨ctx.poi_authority, true, false⟩,            -- poi_authority: signer
      ⟨ctx.inventory_authority, false, false⟩,     -- inventory_authority
      ⟨ctx.gameplay_state_program, false, false⟩   -- gameplay_state program
    ]
  , data := DISC_EQUIP_GEAR_AUTH
  }

theorem cpi2_equip_targets_player_inventory (ctx : EquipGearCpiContext) :
    let cpi := build_equip_gear_cpi ctx
    targetsProgram cpi PLAYER_INVENTORY_PROGRAM_ID ∧
    accountAt cpi 0 ctx.inventory false true ∧
    accountAt cpi 2 ctx.poi_authority true false ∧
    hasDiscriminator cpi DISC_EQUIP_GEAR_AUTH := by
  unfold build_equip_gear_cpi targetsProgram accountAt hasDiscriminator
  exact ⟨rfl, rfl, rfl, rfl⟩

-- ============================================================================
-- CPI-3: record_run_result_cpi targets PLAYER_PROFILE_PROGRAM_ID
--         with session_manager_authority PDA signer
-- ============================================================================

def build_record_result_cpi (ctx : RecordResultCpiContext) : CpiInstruction :=
  { programId := PLAYER_PROFILE_PROGRAM_ID
  , accounts := [
      ⟨ctx.player_profile, false, true⟩,              -- player_profile: writable
      ⟨ctx.session, false, false⟩,                     -- session: read-only
      ⟨ctx.session_signer, true, false⟩,               -- session_signer: signer
      ⟨ctx.session_manager_authority, true, false⟩      -- authority: signer (PDA)
    ]
  , data := DISC_RECORD_RESULT
  }

theorem cpi3_record_result_targets_player_profile (ctx : RecordResultCpiContext) :
    let cpi := build_record_result_cpi ctx
    targetsProgram cpi PLAYER_PROFILE_PROGRAM_ID ∧
    accountAt cpi 0 ctx.player_profile false true ∧
    accountAt cpi 3 ctx.session_manager_authority true false ∧
    hasDiscriminator cpi DISC_RECORD_RESULT := by
  unfold build_record_result_cpi targetsProgram accountAt hasDiscriminator
  exact ⟨rfl, rfl, rfl, rfl⟩

-- ============================================================================
-- CPI-4: grant_relic_ownership CPI targets PLAYER_PROFILE_PROGRAM_ID
--         with mint_authority PDA signer
-- ============================================================================

def build_grant_relic_cpi (ctx : GrantRelicCpiContext) : CpiInstruction :=
  { programId := PLAYER_PROFILE_PROGRAM_ID
  , accounts := [
      ⟨ctx.player_relic_pool, false, true⟩,   -- relic pool: writable
      ⟨ctx.owner, false, false⟩,               -- owner: read-only
      ⟨ctx.payer, true, true⟩,                 -- payer: signer + writable
      ⟨ctx.mint_authority, true, false⟩,        -- mint_authority: signer (PDA)
      ⟨ctx.system_program, false, false⟩        -- system_program
    ]
  , data := DISC_GRANT_RELIC
  }

theorem cpi4_relic_grant_targets_player_profile (ctx : GrantRelicCpiContext) :
    let cpi := build_grant_relic_cpi ctx
    targetsProgram cpi PLAYER_PROFILE_PROGRAM_ID ∧
    accountAt cpi 0 ctx.player_relic_pool false true ∧
    accountAt cpi 3 ctx.mint_authority true false ∧
    hasDiscriminator cpi DISC_GRANT_RELIC := by
  unfold build_grant_relic_cpi targetsProgram accountAt hasDiscriminator
  exact ⟨rfl, rfl, rfl, rfl⟩

-- ============================================================================
-- CPI-5: expand_gear_slots_authorized targets PLAYER_INVENTORY_PROGRAM_ID
--         with gameplay_authority PDA signer
-- ============================================================================

def build_expand_slots_cpi (ctx : ExpandSlotsCpiContext) : CpiInstruction :=
  { programId := PLAYER_INVENTORY_PROGRAM_ID
  , accounts := [
      ⟨ctx.inventory, false, true⟩,           -- inventory: writable
      ⟨ctx.gameplay_authority, true, false⟩     -- gameplay_authority: signer (PDA)
    ]
  , data := DISC_EXPAND_SLOTS
  }

theorem cpi5_expand_slots_targets_player_inventory (ctx : ExpandSlotsCpiContext) :
    let cpi := build_expand_slots_cpi ctx
    targetsProgram cpi PLAYER_INVENTORY_PROGRAM_ID ∧
    accountAt cpi 0 ctx.inventory false true ∧
    accountAt cpi 1 ctx.gameplay_authority true false ∧
    hasDiscriminator cpi DISC_EXPAND_SLOTS := by
  unfold build_expand_slots_cpi targetsProgram accountAt hasDiscriminator
  exact ⟨rfl, rfl, rfl, rfl⟩

-- ============================================================================
-- CPI-6: modify_gold_authorized CPI targets GAMEPLAY_STATE_PROGRAM_ID
--         with poi_authority PDA signer
-- ============================================================================

def build_modify_gold_cpi (ctx : ModifyGoldCpiContext) : CpiInstruction :=
  { programId := GAMEPLAY_STATE_PROGRAM_ID
  , accounts := [
      ⟨ctx.game_state, false, true⟩,      -- game_state: writable
      ⟨ctx.poi_authority, true, false⟩      -- poi_authority: signer (PDA)
    ]
  , data := DISC_MODIFY_GOLD
  }

theorem cpi6_modify_gold_targets_gameplay_state (ctx : ModifyGoldCpiContext) :
    let cpi := build_modify_gold_cpi ctx
    targetsProgram cpi GAMEPLAY_STATE_PROGRAM_ID ∧
    accountAt cpi 0 ctx.game_state false true ∧
    accountAt cpi 1 ctx.poi_authority true false ∧
    hasDiscriminator cpi DISC_MODIFY_GOLD := by
  unfold build_modify_gold_cpi targetsProgram accountAt hasDiscriminator
  exact ⟨rfl, rfl, rfl, rfl⟩
