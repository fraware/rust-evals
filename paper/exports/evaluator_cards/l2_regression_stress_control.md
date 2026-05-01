# Evaluator Card: L2 regression stress-control

## surface_id

`l2_regression_stress_control`

## purpose

Predeclared stress-control / negative-control arm paired with the augmented-test
arm on the same 33 base `(task, agent)` rows.

## scientific_question

How do evaluator transformations that include a deterministic forced-fail hook
map L1 outcomes to L2_REGRESSION_FAIL, separately from augmented-test findings?

## applicability_domain

`runs/released/l2_verified_flagship_v1/results_regression_fail/` merged into the
flagship `results/` summary.

## benchmark_sources

SWE-Bench Verified-style flagship task IDs.

## candidate_sources

Frozen public candidates shared with the augmented arm.

## selection_rule

Every base row is evaluated under `strengthening_spec_regression_fail.json`,
which includes `regression_forced_fail`.

## exclusion_rule

No outcome-dependent removal from the 66-row merged cohort.

## denominator_rule

Failures on this arm appear as `L2_REGRESSION_FAIL` in the merged summary; see
`conditional_false_success.csv` exports.

## invalid_row_rule

Standard batch invalid handling; see sealed `batch_summary.json`.

## levels_used

L0, L1, L2.

## validator_families

`targeted_regression` with forced-fail control.

## known_false_positive_risks

Forced exits are **protocol signals**; they are not standalone proof of product
regressions on the ticket.

## known_false_negative_risks

Passing the arm under the control protocol does not certify absence of real
regressions in general.

## gold_patch_validation

Headline gold replay uses `strengthening_spec_gold_mechanical.json` so
reference-patch checks are not dominated by the forced-fail artifact.

## human_review

See `docs/l2_failure_case_studies.md` for stress-control vs infrastructure
labels.

## reproduction_command

`python ci/scripts/check_evidence_quality.py l2 --run-dir runs/released/l2_verified_flagship_v1/results`

## source_paths

- `runs/released/l2_verified_flagship_v1/results_regression_fail/`
- `paper/exports/l2_verified_flagship_v1/`

## claim_status

`central` (evaluator-sensitivity diagnostic; interpret together with augmented arm).

## interpretation_warning

This arm includes `regression_forced_fail`. Reversals are protocol-control evidence,
not natural product-regression evidence.
