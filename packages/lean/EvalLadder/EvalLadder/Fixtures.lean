/-
  Proof fixtures.

  Fixtures are small, self-contained examples used to smoke-test the Lean
  pipeline. They are NOT part of the curated proof subset and MUST NOT be
  cited by any obligation.
-/

namespace EvalLadder.Fixtures

/-- A trivial true proposition used by CI smoke tests. -/
theorem trivial_true : True := ⟨⟩

end EvalLadder.Fixtures
