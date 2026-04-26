/-
  Obligation sketch: `clap-rs__clap_2093` — custom `help` subcommand text.

  The feature request is to override the canned help string while keeping the
  generated `help` subcommand. At minimum, the default copy and any non-equal
  override must be distinguishable so clap can decide which string to render.
-/

/-
  Reviewer fidelity: uses illustrative strings, not the verbatim default help
  text from the pinned base commit. See `docs/proof_subset_policy.md` (Lean sketch fidelity table).
-/

namespace EvalLadder.Obligations.ClapRs.Clap2093

def defaultHelpBlurb : String :=
  "Prints this message or the help of the given subcommand(s)"

def exampleOverride : String := "Show usage information"

theorem override_differs_from_default : exampleOverride ≠ defaultHelpBlurb := by
  native_decide

end EvalLadder.Obligations.ClapRs.Clap2093
