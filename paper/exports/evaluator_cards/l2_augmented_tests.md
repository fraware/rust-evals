# Evaluator Card: L2 augmented tests

## surface_id

`l2_augmented_tests`

## purpose

Strengthened augmented-test validator arm for the L2 flagship cohort, measuring
whether L1-passing frozen candidates survive additional pytest-style stress.

## scientific_question

Among L1-passing rows, which fail issue-relevant augmented validators under a
fixed, predeclared strengthening spec?

## applicability_domain

`runs/released/l2_verified_flagship_v1/results_astropy/` merged into the flagship
`results/` directory.

## benchmark_sources

SWE-Bench Verified-style tasks from the eleven-task flagship slice.

## candidate_sources

Frozen candidate JSON files shared with the upstream flagship panel assembly.

## selection_rule

Exactly one augmented-test evaluation per `(task, agent)` base row using the
frozen `strengthening_spec.json` referenced by the L2 flagship README.

## exclusion_rule

The 66-row design is not shrunk post hoc based on pass/fail outcomes.

## denominator_rule

Augmented failures are labeled `L2_AUG_TESTS_FAIL` in merged summaries and exports.

## invalid_row_rule

Invalid bundles remain visible in summaries; analysis exports preserve row-level provenance.

## levels_used

L0, L1, L2.

## validator_families

`augmented_unit_tests` including warnings-as-errors stress paths declared in the spec.

## known_false_positive_risks

Augmented commands or selectors may fail candidates for reasons weakly tied to the ticket.

## known_false_negative_risks

Passing augmented tests does not imply full semantic correctness beyond the declared checks.

## gold_patch_validation

See `gold_patch_validation_summary.json` under the gold-mechanical headline profile.

## human_review

`docs/l2_failure_case_studies.md` (diagnostic sample; not a population estimator).

## reproduction_command

`python ci/scripts/check_evidence_quality.py l2 --run-dir runs/released/l2_verified_flagship_v1/results`

## source_paths

- `runs/released/l2_verified_flagship_v1/results_astropy/`
- `paper/exports/l2_verified_flagship_v1/`

## claim_status

`central` (issue-relevant diagnostic arm; interpret separately from the regression stress-control arm).
