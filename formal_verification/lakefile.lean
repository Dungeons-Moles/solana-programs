import Lake
open Lake DSL

package dungeonsAndMolesProofs

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.15.0"
require qedgenSupport from
  "/home/ailton/.cache/qedgen-solana-skills/validation-workspace/lean_solana"

@[default_target]
lean_lib Proofs where
  roots := #[`Proofs]
  moreLeanArgs := #["-DwarningAsError=false"]
