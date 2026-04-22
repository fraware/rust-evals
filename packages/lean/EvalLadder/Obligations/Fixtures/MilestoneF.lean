/-
  Milestone F smoke-test obligation.

  This obligation is intentionally trivial. It exists only so the L4
  integration path (manifest loader -> ExternalProcessChecker ->
  ProofReport) can be exercised end-to-end without depending on a
  non-trivial Lean proof library.

  Real curated obligations land under
  `EvalLadder/Obligations/<benchmark>/<task_slug>.lean` and are cited
  by `datasets/derived/proof_subset/manifest.jsonl`. This file lives
  under `Fixtures/` by convention so it is never accidentally cited by
  a real obligation entry; see `docs/proof_subset_policy.md` and the
  Fixtures note in `EvalLadder/Fixtures.lean`.
-/

namespace EvalLadder.Obligations.Fixtures.MilestoneF

/-- The identity relation on `Nat` is reflexive. This is the tiny
    formal statement the fixture obligation's `formal_statement_ref`
    points at. -/
theorem identity_is_reflexive (n : Nat) : n = n := rfl

end EvalLadder.Obligations.Fixtures.MilestoneF
