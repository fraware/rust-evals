/-
  Obligation sketch: `clap-rs__clap_1624` — `Arg::conflicts_with_everything`.

  The issue asks for a convenience API so a flag can be declared mutually
  exclusive with every other flag without manually listing them. At the
  semantic level, mutual exclusion is a symmetric binary relation on
  argument identities: if `a` cannot co-occur with `b`, then `b` cannot
  co-occur with `a`. The   upstream tests exercise concrete clap wiring; this
  module only pins the discrete relational law that any correct encoding of
  `conflicts_with_everything` must respect.
-/

/-
  Reviewer fidelity: the sketch uses `Fin 4` and inequality as mutual exclusion;
  it does not model clap's `ArgGroup`, override, or positional semantics. See
  `docs/proof_subset_policy.md` (Lean sketch fidelity table) at the repository root.
-/

namespace EvalLadder.Obligations.ClapRs.Clap1624

abbrev ArgSlot := Fin 4

/-- Abstract mutual exclusion: distinct slots conflict. -/
def mutuallyExclusive (a b : ArgSlot) : Prop := a ≠ b

theorem mutual_exclusion_symmetric (a b : ArgSlot) :
    mutuallyExclusive a b ↔ mutuallyExclusive b a := by
  simp only [mutuallyExclusive, ne_comm]

end EvalLadder.Obligations.ClapRs.Clap1624
