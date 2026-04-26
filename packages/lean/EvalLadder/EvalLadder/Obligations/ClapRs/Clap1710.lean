/-
  Obligation sketch: `clap-rs__clap_1710` — ambiguous `InferSubcommands`.

  When two distinct subcommand names share a typed prefix, any error surface
  that lists both candidates must reference at least two different strings.
  This is a minimal finitary fact used as a semantic sanity check: the
  suggestion set for prefix `"te"` in the issue example contains `"test"` and
  `"temp"`, which are not equal as string literals.
-/

/-
  Reviewer fidelity: only proves inequality of two fixed string literals; it does
  not model clap's subcommand trie, scoring, or error templates. See
  `docs/proof_subset_policy.md` (Lean sketch fidelity table).
-/

namespace EvalLadder.Obligations.ClapRs.Clap1710

def candidateA : String := "test"
def candidateB : String := "temp"

theorem ambiguous_candidates_distinct : candidateA ≠ candidateB := by
  native_decide

end EvalLadder.Obligations.ClapRs.Clap1710
