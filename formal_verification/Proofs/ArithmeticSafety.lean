/-
  Arithmetic Safety Proofs for Dungeons & Moles
  Properties AS-1 through AS-5 from SPEC.md
-/
import QEDGen.Solana
open QEDGen.Solana

-- ============================================================================
-- AS-1: Gold Bounds
-- ============================================================================

def modify_gold (gold : Nat) (delta : Int) : Option Nat :=
  if h : (gold : Int) + delta >= 0 ∧ (gold : Int) + delta <= 65535 then
    some ((gold : Int) + delta).toNat
  else none

theorem as1_gold_bounds
    (gold : Nat) (delta : Int) (gold' : Nat)
    (h : modify_gold gold delta = some gold') :
    gold' <= 65535 := by
  unfold modify_gold at h
  split at h
  · next h_bounds =>
    have h_eq := Option.some.inj h
    have := h_bounds.1; have := h_bounds.2
    omega
  · contradiction

-- ============================================================================
-- AS-2: HP Heal Bounds
-- ============================================================================

def heal_player (hp : Int) (amount : Nat) (max_hp : Int) : Option Int :=
  if h : min (hp + (amount : Int)) max_hp <= 32767 ∧
         min (hp + (amount : Int)) max_hp >= -32768 then
    some (min (hp + (amount : Int)) max_hp)
  else none

theorem as2_hp_heal_capped
    (hp : Int) (amount : Nat) (max_hp : Int) (hp' : Int)
    (h : heal_player hp amount max_hp = some hp') :
    hp' <= max_hp := by
  unfold heal_player at h
  split at h
  · next h_range =>
    have h_eq := Option.some.inj h; subst h_eq; omega
  · contradiction

theorem as2_hp_heal_monotonic
    (hp : Int) (amount : Nat) (max_hp : Int) (hp' : Int)
    (h : heal_player hp amount max_hp = some hp')
    (h_under_max : hp <= max_hp) :
    hp' >= hp := by
  unfold heal_player at h
  split at h
  · next h_range =>
    have h_eq := Option.some.inj h; subst h_eq; omega
  · contradiction

-- ============================================================================
-- AS-3: HP Bonus Bounds
-- ============================================================================

def add_hp_bonus (hp : Int) (bonus : Int) : Option Int :=
  if h : bonus > 0 ∧ hp + bonus <= 32767 ∧ hp + bonus >= -32768 then
    some (hp + bonus)
  else none

theorem as3_hp_bonus_positive
    (hp bonus : Int) (hp' : Int)
    (h : add_hp_bonus hp bonus = some hp') :
    bonus > 0 := by
  unfold add_hp_bonus at h
  split at h
  · next h_cond => exact h_cond.1
  · contradiction

theorem as3_hp_bonus_value
    (hp bonus : Int) (hp' : Int)
    (h : add_hp_bonus hp bonus = some hp') :
    hp' = hp + bonus := by
  unfold add_hp_bonus at h
  split at h
  · next => exact (Option.some.inj h).symm
  · contradiction

theorem as3_hp_bonus_in_range
    (hp bonus : Int) (hp' : Int)
    (h : add_hp_bonus hp bonus = some hp') :
    hp' <= 32767 ∧ hp' >= -32768 := by
  unfold add_hp_bonus at h
  split at h
  · next h_cond =>
    have h_eq := Option.some.inj h; subst h_eq
    exact ⟨h_cond.2.1, h_cond.2.2⟩
  · contradiction

-- ============================================================================
-- AS-4: Fee Overflow Safety
-- ============================================================================

def BPS_DENOM : Nat := 10000

theorem as4_fee_fits_u128
    (price fee_bps : Nat)
    (h_price : price <= U64_MAX)
    (h_bps : fee_bps <= BPS_DENOM) :
    price * fee_bps <= U128_MAX := by
  have h1 : price * fee_bps <= U64_MAX * BPS_DENOM := Nat.mul_le_mul h_price h_bps
  have h2 : U64_MAX * BPS_DENOM <= U128_MAX := by native_decide
  omega

-- ============================================================================
-- AS-5: Run Count Safety
-- ============================================================================

def RUNS_PER_PURCHASE : Nat := 20

def purchase_runs (available_runs : Nat) : Option Nat :=
  if h : available_runs + RUNS_PER_PURCHASE <= U32_MAX then
    some (available_runs + RUNS_PER_PURCHASE)
  else none

theorem as5_run_count_safe
    (available_runs new_runs : Nat)
    (h : purchase_runs available_runs = some new_runs) :
    new_runs <= U32_MAX := by
  unfold purchase_runs at h
  split at h
  · next h_bounds =>
    have h_eq := Option.some.inj h; omega
  · contradiction

theorem as5_run_count_increment
    (available_runs new_runs : Nat)
    (h : purchase_runs available_runs = some new_runs) :
    new_runs = available_runs + RUNS_PER_PURCHASE := by
  unfold purchase_runs at h
  split at h
  · next => exact (Option.some.inj h).symm
  · contradiction
