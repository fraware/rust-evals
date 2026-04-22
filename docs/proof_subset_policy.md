# Proof subset policy

The proof-carrying subset is the spine of the L4 layer. Its scientific value
depends on selection discipline. This document is the authoritative selection
rubric. Any deviation from it must be recorded per obligation in
`datasets/derived/proof_subset/manifest.jsonl`.

## Selection rubric

A task is eligible for the proof subset only if **all** of the following hold:

1. The issue's intended property can be stated in one or two sentences.
2. The touched code is local enough to reason about without importing large
   external theorem libraries.
3. The property matters to the issue itself; it is not a random adjacent
   invariant.
4. The property is strictly stronger than the official tests for the task.
5. Formalization effort is bounded, measured in reviewer-hours and declared
   at selection time.

All five must hold. Task selection is a source of bias in the audit and must
be justifiable on each of these dimensions.

## Preferred task categories

In priority order:

1. Parser and serializer consistency (roundtrip invariants).
2. State-machine safety (no invalid transitions under declared preconditions).
3. Preservation invariants on transformations (for example: a list reordering
   preserves length and multiset).
4. Numeric bounds and monotonicity.
5. `no-panic` or `no-invalid-state` on selected code paths.

## Categories to avoid

- Whole-repository correctness obligations.
- Obligations that require large imported theorem libraries with high
  setup cost.
- Obligations whose proof burden dwarfs their evaluative value.
- Obligations phrased in terms of the test suite itself; they should be
  phrased in terms of program behaviour.

## Obligation template

Every obligation in `datasets/derived/proof_subset/manifest.jsonl` must
specify:

- `obligation_id`: stable identifier.
- `task_id`: identifier of the benchmark task this obligation attaches to.
- `property_name`: short human-readable name.
- `property_type`: one of the preferred categories above.
- `target_files`: list of files the obligation constrains.
- `informal_statement`: one or two sentences.
- `formal_statement_ref`: path to a Lean declaration in
  `packages/lean/EvalLadder/Obligations/`.
- `proof_checker`: the Lean command to run (for example `lake env lean`).
- `pass_criterion`: stable code returned by the checker that counts as pass
  (for example `L4_OBLIGATION_MET`).
- `difficulty`: declared reviewer-hours at selection time.
- `selection_rationale`: why this task satisfies the five selection rubric
  items.
- `witness_inputs`: optional fixtures or inputs required by the obligation.
- `expected_touched_symbols`: symbols the patch is expected to touch.

The schema is captured in `schemas/proof_obligation.schema.json`.

## Integration discipline

- Candidate patches are applied to a clean checkout of the task's base
  commit; obligations are checked against the post-patch tree.
- Extraction is declarative. The Lean package does **not** parse arbitrary
  diffs. Extraction scripts produce a normalized obligation context, and
  Lean checks a theorem specific to that snapshot and obligation.
- Automatic Rust-to-Lean translation is explicitly out of scope. Here Lean
  is a semantic validator for curated obligations.
- An obligation that cannot be checked (for example because extraction
  fails) yields `L4_EXTRACTION_FAILED`; it does not fall back to L2 tests.

## Governance

- Adding an obligation requires a PR that touches `manifest.jsonl`,
  `packages/lean/EvalLadder/Obligations/`, and this document's changelog
  section.
- Removing or modifying an existing obligation requires the same PR surface
  plus a rationale field in `manifest.jsonl`.
- A selection bias audit is run before any release and archived under
  `paper/exports/`.

## Release posture

The proof subset may ship in either of the two release modes declared in
`docs/submission_checklist.md`. In Mode 1 (code-only) the manifest is the
only artifact; in Mode 2 (code + new dataset) the manifest and its
Croissant metadata live under
`datasets/derived/proof_subset/croissant/`.
