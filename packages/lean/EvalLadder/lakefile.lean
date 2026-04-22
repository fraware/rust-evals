import Lake
open Lake DSL

package EvalLadder where
  -- Lean 4 project for the L4 semantic layer of eval-ladder.
  -- This project defines proof obligations and proof validation for the
  -- curated proof-carrying subset. See docs/proof_subset_policy.md at the
  -- repository root for the scope and selection rubric.

@[default_target]
lean_lib EvalLadder where
  -- Root library. Sub-libraries live under EvalLadder/.
  srcDir := "."
  roots  := #[`EvalLadder]
