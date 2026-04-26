/-
  Obligation sketch: `clap-rs__clap_1972` — default `--version` newline.

  Shell-friendly version output should end with a line terminator so the next
  shell prompt does not concatenate onto the version string. Modelled as a
  pure list-of-characters fact: appending a newline strictly increases length.
-/

/-
  Reviewer fidelity: list-of-`Char` algebra only; not wired to clap's `std::fmt`
  or platform newline conventions. See `docs/proof_subset_policy.md` (Lean sketch fidelity table).
-/

namespace EvalLadder.Obligations.ClapRs.Clap1972

theorem version_line_grows (s : List Char) :
    (s ++ ['\n']).length = s.length + 1 := by
  rw [List.length_append, List.length_singleton]

end EvalLadder.Obligations.ClapRs.Clap1972
