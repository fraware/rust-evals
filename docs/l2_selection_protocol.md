# L2 selection protocol (flagship v1)

This protocol documents how `runs/released/l2_verified_flagship_v1/` was
constructed to avoid post-hoc selection bias.

## How were the 66 entries selected?

- Base panel: `runs/released/agent_panel_verified_flagship_v1/panel.jsonl`
  with 33 entries (`11 tasks x 3 agents`).
- Two validator-family variants were generated from the same 33 base entries:
  - `panel_astropy.jsonl` (`__astropy` suffix, augmented-test stress family),
  - `panel_xarray.jsonl` (`__regressionfail` family in merged summaries; see
    `results_regression_fail/` naming).
- Final merged L2 set is `33 + 33 = 66` rows in
  `runs/released/l2_verified_flagship_v1/results/batch_summary.json` using
  `ci/scripts/merge_l2_batch_summaries.py`.

## Was selection fixed before L2 execution?

Yes, at the row level:

- The 33 base rows are copied from the pre-defined flagship verified panel.
- Family expansion is deterministic and applied to every base row.
- `provenance.json` and panel files under `runs/released/l2_verified_flagship_v1/`
  record this construction.

## Were validators generated before seeing candidate outcomes?

Yes:

- Validator specs are checked in as:
  - `runs/released/l2_verified_astropy_v1/strengthening_spec.json`
  - `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`
- These specs are referenced directly in the batch commands in
  `runs/released/l2_verified_flagship_v1/README.md`.

## Were any rows removed after L2 failures?

No.

- The merged output keeps all rows from both family runs.
- No post-hoc row deletion is applied in `results/`.

## What exclusion rules were used?

No additional exclusion rules were applied at L2 layer construction time.

- Exclusions happened upstream when building the *verified flagship* panel
  (`agent_panel_verified_flagship_v1`) from `agent_panel_v3_r1` by dropping
  known harness-fragile task prefixes (`matplotlib__`, `scikit-learn__`,
  `pytest-dev__`).
- L2 inherits that upstream row set without new per-outcome filtering.

## Which validator families were active?

Two families were active in flagship v1:

- `augmented_unit_tests`
- `targeted_regression`

Both run under `--strengthening-mode tests_plus_regression` on every row.

## What counts as augmented-test failure vs regression failure?

- **Augmented-test failure**: at least one sub-check in validator
  `augmented_unit_tests` fails, producing `L2_AUG_TESTS_FAIL`.
- **Regression failure**: at least one sub-check in validator
  `targeted_regression` fails, producing `L2_REGRESSION_FAIL`.

In flagship v1, `targeted_regression` in
`strengthening_spec_regression_fail.json` intentionally contains a forced
non-zero command (`regression_forced_fail`) to provide controlled negative
examples; this is treated as a validator limitation for semantic interpretation,
not as issue-level regression evidence.
# L2 selection protocol (flagship v1)

This protocol documents how `runs/released/l2_verified_flagship_v1/` was
constructed to avoid post-hoc selection bias.

## How were the 66 entries selected?

- Base panel: `runs/released/agent_panel_verified_flagship_v1/panel.jsonl`
  with 33 entries (`11 tasks x 3 agents`).
- Two validator-family variants were generated from the same 33 base entries:
  - `panel_astropy.jsonl` (`__astropy` suffix, augmented-test stress family),
  - `panel_xarray.jsonl` (`__regressionfail` family in merged summaries; see
    `results_regression_fail/` naming).
- Final merged L2 set is `33 + 33 = 66` rows in
  `runs/released/l2_verified_flagship_v1/results/batch_summary.json` using
  `ci/scripts/merge_l2_batch_summaries.py`.

## Was selection fixed before L2 execution?

Yes, at the row level:

- The 33 base rows are copied from the pre-defined flagship verified panel.
- Family expansion is deterministic and applied to every base row.
- `provenance.json` and panel files under `runs/released/l2_verified_flagship_v1/`
  record this construction.

## Were validators generated before seeing candidate outcomes?

Yes:

- Validator specs are checked in as:
  - `runs/released/l2_verified_astropy_v1/strengthening_spec.json`
  - `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`
- These specs are referenced directly in the batch commands in
  `runs/released/l2_verified_flagship_v1/README.md`.

## Were any rows removed after L2 failures?

No.

- The merged output keeps all rows from both family runs.
- No post-hoc row deletion is applied in `results/`.

## What exclusion rules were used?

No additional exclusion rules were applied at L2 layer construction time.

- Exclusions happened upstream when building the *verified flagship* panel
  (`agent_panel_verified_flagship_v1`) from `agent_panel_v3_r1` by dropping
  known harness-fragile task prefixes (`matplotlib__`, `scikit-learn__`,
  `pytest-dev__`).
- L2 inherits that upstream row set without new per-outcome filtering.

## Which validator families were active?

Two families were active in flagship v1:

- `augmented_unit_tests`
- `targeted_regression`

Both run under `--strengthening-mode tests_plus_regression` on every row.

## What counts as augmented-test failure vs regression failure?

- **Augmented-test failure**: at least one sub-check in validator
  `augmented_unit_tests` fails, producing `L2_AUG_TESTS_FAIL`.
- **Regression failure**: at least one sub-check in validator
  `targeted_regression` fails, producing `L2_REGRESSION_FAIL`.

In flagship v1, `targeted_regression` in
`strengthening_spec_regression_fail.json` intentionally contains a forced
non-zero command (`regression_forced_fail`) to provide controlled negative
examples; this is treated as a validator limitation for semantic interpretation,
not as issue-level regression evidence.
