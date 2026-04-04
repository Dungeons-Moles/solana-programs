/-
  SOL Conservation Proofs for Dungeons & Moles
  Properties SC-1 through SC-3 from SPEC.md
-/
import QEDGen.Solana
open QEDGen.Solana

-- ============================================================================
-- Constants
-- ============================================================================

def BPS_DENOMINATOR : Nat := 10000
def DEFAULT_COMPANY_FEE_BPS : Nat := 300
def DEFAULT_GAUNTLET_FEE_BPS : Nat := 200
def RUN_PURCHASE_COST : Nat := 50000000
def DUEL_ENTRY_LAMPORTS : Nat := 100000000

-- ============================================================================
-- SC-1: Marketplace Fee Split Conservation
--
-- On-chain:  seller = price - floor(price*c/10000) - floor(price*g/10000)
-- Property:  seller + company_fee + gauntlet_fee = price
-- This is Nat subtraction identity: (a - b - c) + b + c = a when b+c <= a
-- ============================================================================

/-- Given fees don't exceed price, the split conserves total -/
theorem sc1_marketplace_fee_conservation
    (price cf gf : Nat)
    (h_le : cf + gf <= price) :
    (price - cf - gf) + cf + gf = price := by
  omega

/-- Default BPS are within denominator -/
theorem sc1_default_bps_valid :
    DEFAULT_COMPANY_FEE_BPS + DEFAULT_GAUNTLET_FEE_BPS <= BPS_DENOMINATOR := by
  unfold DEFAULT_COMPANY_FEE_BPS DEFAULT_GAUNTLET_FEE_BPS BPS_DENOMINATOR; omega

/-- Fees computed with valid BPS never exceed price -/
theorem sc1_fees_bounded (price bps : Nat) (h : bps <= BPS_DENOMINATOR) :
    price * bps / BPS_DENOMINATOR <= price := by
  unfold BPS_DENOMINATOR at *
  have : price * bps <= price * 10000 := Nat.mul_le_mul_left price h
  omega

-- ============================================================================
-- SC-2: Run Purchase Split Conservation
-- ============================================================================

theorem sc2_run_purchase_conservation :
    let half := RUN_PURCHASE_COST / 2
    half + (RUN_PURCHASE_COST - half) = RUN_PURCHASE_COST := by
  unfold RUN_PURCHASE_COST; omega

theorem sc2_run_purchase_equal_split :
    RUN_PURCHASE_COST / 2 = RUN_PURCHASE_COST - RUN_PURCHASE_COST / 2 := by
  unfold RUN_PURCHASE_COST; rfl

-- ============================================================================
-- SC-3: Duel Entry Deposit Conservation
-- ============================================================================

def enter_duel_vault (vault_balance entry_lamports : Nat) : Option (Nat × Nat) :=
  if h : entry_lamports = 0 then
    some (vault_balance + DUEL_ENTRY_LAMPORTS, DUEL_ENTRY_LAMPORTS)
  else none

theorem sc3_duel_entry_deposit
    (vault_balance entry_lamports new_balance new_entry : Nat)
    (h : enter_duel_vault vault_balance entry_lamports = some (new_balance, new_entry)) :
    new_balance = vault_balance + DUEL_ENTRY_LAMPORTS := by
  unfold enter_duel_vault at h
  split at h
  · next =>
    have h_eq := Option.some.inj h
    have := Prod.mk.inj h_eq
    omega
  · contradiction

theorem sc3_duel_entry_recorded
    (vault_balance entry_lamports new_balance new_entry : Nat)
    (h : enter_duel_vault vault_balance entry_lamports = some (new_balance, new_entry)) :
    new_entry = DUEL_ENTRY_LAMPORTS := by
  unfold enter_duel_vault at h
  split at h
  · next =>
    have h_eq := Option.some.inj h
    have := Prod.mk.inj h_eq
    omega
  · contradiction
