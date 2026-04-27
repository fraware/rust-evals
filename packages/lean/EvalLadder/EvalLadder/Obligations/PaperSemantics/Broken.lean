/-
Intentionally invalid Lean surface syntax.

Used only with `datasets/derived/proof_subset/manifest_paper_semantics_l4_counterexample.jsonl`
so `lake env lean` fails and the obligation driver emits `L4_OBLIGATION_UNMET`
while L3 policy evaluation on the Rust bundle can still pass.
-/
def paper_semantics_broken_surface_syntax : Nat := foo_undefined_identifier_xyz
