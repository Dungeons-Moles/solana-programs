/-
  State Machine Safety Proofs for Dungeons & Moles
  Properties SM-1 through SM-7 from SPEC.md
-/
import QEDGen.Solana
open QEDGen.Solana

-- ============================================================================
-- SM-1: Dead Players Blocked
-- ============================================================================

def move_player_sm (is_dead boss_ready : Bool) (moves : Nat) (signer ss : Pubkey) :
    Option Nat :=
  if h : signer = ss ∧ is_dead = false ∧ boss_ready = false ∧ moves > 0 then
    some (moves - 1)
  else none

def trigger_boss_sm (is_dead boss_ready : Bool) (signer ss : Pubkey) : Option Bool :=
  if h : signer = ss ∧ is_dead = false ∧ boss_ready = true then
    some false
  else none

theorem sm1_dead_blocks_move
    (boss_ready : Bool) (moves : Nat) (signer ss : Pubkey)
    (h_dead : is_dead = true) :
    move_player_sm is_dead boss_ready moves signer ss = none := by
  unfold move_player_sm; split
  · next h_cond => simp [h_dead] at h_cond
  · rfl

theorem sm1_dead_blocks_boss
    (boss_ready : Bool) (signer ss : Pubkey)
    (h_dead : is_dead = true) :
    trigger_boss_sm is_dead boss_ready signer ss = none := by
  unfold trigger_boss_sm; split
  · next h_cond => simp [h_dead] at h_cond
  · rfl

-- ============================================================================
-- SM-2: Boss Gate
-- ============================================================================

theorem sm2_boss_gate
    (is_dead boss_ready : Bool) (signer ss : Pubkey) (result : Bool)
    (h : trigger_boss_sm is_dead boss_ready signer ss = some result) :
    boss_ready = true := by
  unfold trigger_boss_sm at h
  split at h
  · next h_cond => exact h_cond.2.2
  · contradiction

-- ============================================================================
-- SM-3: One-Time POI
-- ============================================================================

def interact_poi_onetime (used : Bool) : Option Bool :=
  if h : used = false then some true else none

theorem sm3_onetime_no_reuse (h_used : used = true) :
    interact_poi_onetime used = none := by
  unfold interact_poi_onetime; split
  · next h_f => simp [h_used] at h_f
  · rfl

theorem sm3_onetime_becomes_used (used' : Bool)
    (h : interact_poi_onetime used = some used') :
    used' = true := by
  unfold interact_poi_onetime at h
  split at h
  · next => exact (Option.some.inj h).symm
  · contradiction

-- ============================================================================
-- SM-4: Session End Gate
-- ============================================================================

def end_session_sm (is_dead completed : Bool) : Option Unit :=
  if h : is_dead = true ∨ completed = true then some () else none

theorem sm4_session_end_gate
    (is_dead completed : Bool)
    (h : end_session_sm is_dead completed = some ()) :
    is_dead = true ∨ completed = true := by
  unfold end_session_sm at h
  split at h
  · assumption
  · contradiction

-- ============================================================================
-- SM-5: Duel No Double Entry
-- ============================================================================

def DUEL_ENTRY_LAM : Nat := 100000000

def enter_duel_sm (entry_lamports : Nat) : Option Nat :=
  if h : entry_lamports = 0 then some DUEL_ENTRY_LAM else none

theorem sm5_no_double_entry
    (entry_lamports result : Nat)
    (h : enter_duel_sm entry_lamports = some result) :
    entry_lamports = 0 := by
  unfold enter_duel_sm at h
  split at h
  · assumption
  · contradiction

theorem sm5_entry_blocks_reentry
    (entry_lamports result : Nat)
    (h : enter_duel_sm entry_lamports = some result) :
    result ≠ 0 := by
  unfold enter_duel_sm at h
  split at h
  · next => injection h with h_eq; subst h_eq; unfold DUEL_ENTRY_LAM; omega
  · contradiction

-- ============================================================================
-- SM-6: Gauntlet No Double Claim
-- ============================================================================

def claim_gauntlet_sm (paid : Bool) (finalized : Bool) : Option Bool :=
  if h : paid = false ∧ finalized = true then some true else none

theorem sm6_no_double_claim
    (paid finalized : Bool) (paid' : Bool)
    (h : claim_gauntlet_sm paid finalized = some paid') :
    paid = false := by
  unfold claim_gauntlet_sm at h
  split at h
  · next h_cond => exact h_cond.1
  · contradiction

theorem sm6_claim_sets_paid
    (paid finalized : Bool) (paid' : Bool)
    (h : claim_gauntlet_sm paid finalized = some paid') :
    paid' = true := by
  unfold claim_gauntlet_sm at h
  split at h
  · next => exact (Option.some.inj h).symm
  · contradiction

-- ============================================================================
-- SM-7: Gear Slot Expansion Monotonic
-- ============================================================================

def expand_gear_slots (capacity : Nat) : Option Nat :=
  if h : capacity < 12 then some (capacity + 2) else none

theorem sm7_gear_slot_monotonic
    (capacity capacity' : Nat)
    (h : expand_gear_slots capacity = some capacity') :
    capacity' > capacity := by
  unfold expand_gear_slots at h
  split at h
  · next h_lt => injection h with h_eq; omega
  · contradiction

theorem sm7_gear_slot_increment
    (capacity capacity' : Nat)
    (h : expand_gear_slots capacity = some capacity') :
    capacity' = capacity + 2 := by
  unfold expand_gear_slots at h
  split at h
  · next => exact (Option.some.inj h).symm
  · contradiction

theorem sm7_gear_slot_bounded
    (capacity capacity' : Nat)
    (h : expand_gear_slots capacity = some capacity') :
    capacity' <= 14 := by
  unfold expand_gear_slots at h
  split at h
  · next h_lt => injection h with h_eq; omega
  · contradiction
