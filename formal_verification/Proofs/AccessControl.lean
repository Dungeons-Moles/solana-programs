/-
  Access Control Proofs for Dungeons & Moles
  Properties AC-1 through AC-10 from SPEC.md
-/
import QEDGen.Solana
open QEDGen.Solana

-- ============================================================================
-- State Models
-- ============================================================================

structure GameState where
  player : Pubkey
  session_signer : Pubkey
  session : Pubkey
  is_dead : Bool
  completed : Bool
  boss_fight_ready : Bool
  hp : Int
  gold : Nat
  moves_remaining : Nat

structure GameSession where
  player : Pubkey
  session_signer : Pubkey
  settled : Bool
  campaign_level : Nat

structure PlayerProfile where
  owner : Pubkey
  available_runs : Nat
  highest_level_unlocked : Nat

structure Listing where
  seller : Pubkey
  asset : Pubkey
  price_lamports : Nat

structure PlayerInventory where
  session : Pubkey
  player : Pubkey
  gear_slot_capacity : Nat

structure PlayerRelicPool where
  owner : Pubkey
  count : Nat

-- ============================================================================
-- AC-1: Session Isolation (move_player)
-- ============================================================================

def move_player_transition
    (gs : GameState) (signer : Pubkey) : Option GameState :=
  if h : signer = gs.session_signer ∧ gs.is_dead = false ∧
         gs.boss_fight_ready = false ∧ gs.moves_remaining > 0 then
    some { gs with moves_remaining := gs.moves_remaining - 1 }
  else none

theorem ac1_session_isolation_move
    (gs : GameState) (signer : Pubkey) (gs' : GameState)
    (h : move_player_transition gs signer = some gs') :
    signer = gs.session_signer := by
  unfold move_player_transition at h
  split at h
  · next h_cond => exact h_cond.1
  · contradiction

-- ============================================================================
-- AC-2: Boss Fight Auth
-- ============================================================================

def trigger_boss_fight_transition
    (gs : GameState) (signer : Pubkey) : Option GameState :=
  if h : signer = gs.session_signer ∧ gs.is_dead = false ∧ gs.boss_fight_ready = true then
    some { gs with boss_fight_ready := false }
  else none

theorem ac2_boss_fight_auth
    (gs : GameState) (signer : Pubkey) (gs' : GameState)
    (h : trigger_boss_fight_transition gs signer = some gs') :
    signer = gs.session_signer := by
  unfold trigger_boss_fight_transition at h
  split at h
  · next h_cond => exact h_cond.1
  · contradiction

-- ============================================================================
-- AC-3: Heal Auth
-- ============================================================================

def heal_player_transition
    (gs : GameState) (signer poi_authority_pda : Pubkey)
    (amount : Nat) (max_hp : Int) : Option GameState :=
  if signer = poi_authority_pda then
    some { gs with hp := min (gs.hp + amount) max_hp }
  else none

theorem ac3_heal_auth
    (gs : GameState) (signer poi_authority_pda : Pubkey) (amount : Nat) (max_hp : Int)
    (gs' : GameState)
    (h : heal_player_transition gs signer poi_authority_pda amount max_hp = some gs') :
    signer = poi_authority_pda := by
  unfold heal_player_transition at h
  split at h
  · assumption
  · contradiction

-- ============================================================================
-- AC-4: Gold Auth
-- ============================================================================

def modify_gold_transition
    (gs : GameState) (signer poi_authority_pda : Pubkey)
    (delta : Int) : Option GameState :=
  if h : signer = poi_authority_pda ∧
         (gs.gold : Int) + delta >= 0 ∧ (gs.gold : Int) + delta <= 65535 then
    some { gs with gold := ((gs.gold : Int) + delta).toNat }
  else none

theorem ac4_gold_auth
    (gs : GameState) (signer poi_authority_pda : Pubkey) (delta : Int)
    (gs' : GameState)
    (h : modify_gold_transition gs signer poi_authority_pda delta = some gs') :
    signer = poi_authority_pda := by
  unfold modify_gold_transition at h
  split at h
  · next h_cond => exact h_cond.1
  · contradiction

-- ============================================================================
-- AC-5: Equip Auth
-- ============================================================================

def equip_gear_transition
    (inv : PlayerInventory) (signer poi_authority_pda : Pubkey) : Option PlayerInventory :=
  if signer = poi_authority_pda then some inv else none

theorem ac5_equip_auth
    (inv : PlayerInventory) (signer poi_authority_pda : Pubkey) (inv' : PlayerInventory)
    (h : equip_gear_transition inv signer poi_authority_pda = some inv') :
    signer = poi_authority_pda := by
  unfold equip_gear_transition at h
  split at h
  · assumption
  · contradiction

-- ============================================================================
-- AC-6: Slot Expansion Auth
-- ============================================================================

def expand_gear_slots_transition
    (inv : PlayerInventory) (signer gameplay_authority_pda : Pubkey) :
    Option PlayerInventory :=
  if h : signer = gameplay_authority_pda ∧ inv.gear_slot_capacity < 12 then
    some { inv with gear_slot_capacity := inv.gear_slot_capacity + 2 }
  else none

theorem ac6_slot_expansion_auth
    (inv : PlayerInventory) (signer gameplay_authority_pda : Pubkey)
    (inv' : PlayerInventory)
    (h : expand_gear_slots_transition inv signer gameplay_authority_pda = some inv') :
    signer = gameplay_authority_pda := by
  unfold expand_gear_slots_transition at h
  split at h
  · next h_cond => exact h_cond.1
  · contradiction

-- ============================================================================
-- AC-7: Record Result Auth
-- ============================================================================

def record_run_result_transition
    (profile : PlayerProfile) (session : GameSession)
    (session_signer authority_signer session_manager_authority_pda : Pubkey)
    (level_completed : Nat) (victory : Bool) : Option PlayerProfile :=
  if h : authority_signer = session_manager_authority_pda ∧
         session.player = profile.owner ∧
         session.session_signer = session_signer ∧
         session.campaign_level = level_completed then
    let new_level := if victory && level_completed >= profile.highest_level_unlocked
      then level_completed + 1
      else profile.highest_level_unlocked
    some { profile with highest_level_unlocked := new_level }
  else none

theorem ac7_record_result_auth
    (profile : PlayerProfile) (session : GameSession)
    (session_signer authority_signer session_manager_authority_pda : Pubkey)
    (level_completed : Nat) (victory : Bool) (profile' : PlayerProfile)
    (h : record_run_result_transition profile session session_signer authority_signer
         session_manager_authority_pda level_completed victory = some profile') :
    authority_signer = session_manager_authority_pda ∧
    session.player = profile.owner := by
  unfold record_run_result_transition at h
  split at h
  · next h_cond => exact ⟨h_cond.1, h_cond.2.1⟩
  · contradiction

-- ============================================================================
-- AC-8: Relic Grant Auth
-- ============================================================================

def grant_relic_transition
    (pool : PlayerRelicPool) (signer mint_authority_pda : Pubkey) : Option PlayerRelicPool :=
  if signer = mint_authority_pda then
    some { pool with count := pool.count + 1 }
  else none

theorem ac8_relic_grant_auth
    (pool : PlayerRelicPool) (signer mint_authority_pda : Pubkey) (pool' : PlayerRelicPool)
    (h : grant_relic_transition pool signer mint_authority_pda = some pool') :
    signer = mint_authority_pda := by
  unfold grant_relic_transition at h
  split at h
  · assumption
  · contradiction

-- ============================================================================
-- AC-9: Cancel Listing Auth
-- ============================================================================

def cancel_listing_transition (listing : Listing) (signer : Pubkey) : Option Unit :=
  if signer = listing.seller then some () else none

theorem ac9_cancel_listing_auth
    (listing : Listing) (signer : Pubkey)
    (h : cancel_listing_transition listing signer ≠ none) :
    signer = listing.seller := by
  unfold cancel_listing_transition at h
  split at h
  · assumption
  · exact absurd rfl h

-- ============================================================================
-- AC-10: End Session Auth
-- ============================================================================

def end_session_transition
    (session : GameSession) (game_state : GameState) (signer : Pubkey) :
    Option Unit :=
  if h : signer = session.session_signer ∧
         (game_state.is_dead = true ∨ game_state.completed = true) then
    some ()
  else none

theorem ac10_end_session_auth
    (session : GameSession) (game_state : GameState) (signer : Pubkey)
    (h : end_session_transition session game_state signer = some ()) :
    signer = session.session_signer := by
  unfold end_session_transition at h
  split at h
  · next h_cond => exact h_cond.1
  · contradiction
