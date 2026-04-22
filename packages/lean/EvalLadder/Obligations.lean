/-
  Index of proof obligations.

  Each task in the curated proof-carrying subset that declares a Lean proof
  obligation ships a module under `EvalLadder/Obligations/` matching the
  obligation_id declared in `datasets/derived/proof_subset/manifest.jsonl`.

  At bootstrap no obligations are landed. Milestone F seeds the first batch.
-/

namespace EvalLadder.Obligations

/-- Placeholder: the obligation module is intentionally empty at bootstrap.
    Milestone F adds the first curated obligations. -/
def placeholder : Unit := ()

end EvalLadder.Obligations
