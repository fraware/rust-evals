/-
  Obligation sketch: `clap-rs__clap_2075` — conflict error usage lines.

  When two flags are mutually exclusive, each single-flag invocation is a
  distinct minimal usage pattern. A usage renderer that lists both flags on one
  line conflates those patterns; the discrete model here enforces that the
  string descriptions of single-flag usages are pairwise distinct.
-/

/-
  Reviewer fidelity: toy two-flag type; does not model clap usage synthesis,
  wrapping, or blacklist validation order. See `docs/proof_subset_sketches.md`.
-/

namespace EvalLadder.Obligations.ClapRs.Clap2075

inductive Flag | A | B

open Flag

def usageLine : Flag → String
  | A => "-a"
  | B => "-b"

theorem usage_lines_injective (x y : Flag) :
    usageLine x = usageLine y → x = y := by
  cases x <;> cases y <;> simp [usageLine] at * <;> contradiction

end EvalLadder.Obligations.ClapRs.Clap2075
