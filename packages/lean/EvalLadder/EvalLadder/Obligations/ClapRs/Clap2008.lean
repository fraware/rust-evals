/-
  Obligation sketch: `clap-rs__clap_2008` — short vs long help text.

  Distinct help surfaces (`short` vs `long`) must remain distinguishable at
  the type level so clap can route `--help` vs `--help --verbose` style
  behaviour without conflating the two channels.
-/

/-
  Reviewer fidelity: enum disjointness only; no connection to clap's settings
  bitset or help pipeline. See `docs/proof_subset_sketches.md`.
-/

namespace EvalLadder.Obligations.ClapRs.Clap2008

inductive HelpSurface
  | Short
  | Long
  deriving DecidableEq, Repr

theorem short_ne_long : HelpSurface.Short ≠ HelpSurface.Long := by
  simp

end EvalLadder.Obligations.ClapRs.Clap2008
