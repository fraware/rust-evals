/-
  Obligation sketch: `BurntSushi__ripgrep_454` — `--only-matching` multiplicity.

  The reporter observes `N` intended non-empty match segments on a line but
  receives the first segment printed `N` times. A correct implementation must
  preserve the count of emitted segments when each emission is drawn from the
  list of matched spans. Modelled as: replicating a single span `N` times
  yields length `N`, while emitting an `N`-element list of distinct spans also
  yields length `N` — the cardinality invariant the regression tests should
  preserve even when the chosen span varies.
-/

namespace EvalLadder.Obligations.BurntSushi.Ripgrep454

variable {α : Type}

theorem replicate_count (x : α) (n : Nat) :
    (List.replicate n x).length = n :=
  List.length_replicate n x

theorem map_id_count (xs : List α) :
    (xs.map id).length = xs.length := by
  simp

end EvalLadder.Obligations.BurntSushi.Ripgrep454
