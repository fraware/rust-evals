# Evaluator Card: Rust proof subset (release seal)

## surface_id

`rust_proof_subset_release`

## purpose

Auditable L0–L4 ladder run on a curated Rust proof subset with task-specific Lean
obligations.

## scientific_question

Can semantic obligations be attached and checked reproducibly on real Rust tasks,
and what separation does **current natural sealed evidence** exhibit?

## applicability_domain

`runs/released/rust_proof_subset_v1/results_seal/` and
`paper/exports/rust_proof_subset_v1_seal_release/`, with
`datasets/derived/proof_subset/manifest.jsonl`.

## benchmark_sources

Public Rust-SWE-bench-derived identifiers listed in the proof manifest.

## candidate_sources

Frozen candidates per the sealed batch README.

## selection_rule

Curated manifest and obligation table under `docs/proof_subset_policy.md`.

## exclusion_rule

Paper-semantics counterexample replays are **mechanism tests only** and must not
back headline natural-evidence tables.

## denominator_rule

Current real-manifest sealed cohort is eight rows; semantic minima are evaluated
against publication thresholds in `docs/evidence_empirical_status.md`.

## invalid_row_rule

Use `eval-ladder verify run-dir --run-dir runs/released/rust_proof_subset_v1/results_seal`.

## levels_used

L0–L4.

## validator_families

Rust harness validators; Lean-backed L4 obligations as configured for the subset.

## known_false_positive_risks

Obligations may be narrower than informal issue semantics.

## known_false_negative_risks

Passing L4 does not prove informal completeness beyond the encoded obligation.

## gold_patch_validation

Reference obligations are part of the proof-subset policy; distinct from SWE
Python gold-patch CSV protocols.

## Row-count note

The paper’s Rust proof-subset frontier claim is sourced from
`paper/exports/strict_feasibility_report.json`, which reports the real sealed
proof-subset manifest at the task/obligation-entry level. Some paper-export
manifests may expose level-expanded or cumulative analysis rows. These expanded
analysis rows are **not** the denominator for the manuscript’s frontier claim.
The manuscript denominator is the real sealed proof-subset entry count reported
in `strict_feasibility_report.json`.

## human_review

Optional; natural sealed evidence currently shows no L3-pass/L4-fail separation
under strict publication minima.

## reproduction_command

`python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof --run-dir runs/released/rust_proof_subset_v1/results_seal`

## source_paths

- `runs/released/rust_proof_subset_v1/results_seal/`
- `paper/exports/rust_proof_subset_v1_seal_release/`
- `datasets/derived/proof_subset/manifest.jsonl`

## claim_status

`frontier` (implemented extension surface; not a headline natural semantic-separation result this cycle).
