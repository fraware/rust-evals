# L2 gold patch validation (flagship v1)

This document reports manual inspection outcomes for:

- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.json`

The validation run replays upstream dataset gold/developer patches for the 11
flagship tasks against the same two L2 validator families used in
`l2_verified_flagship_v1`.

## Scope and artifacts

- Run root:
  `runs/released/l2_verified_flagship_v1/gold_patch_validation/`
- Family outputs:
  - `results_astropy/` (`augmented_unit_tests`)
  - `results_regressionfail/` (`targeted_regression`)
- Export rows: 22 (`11 tasks x 2 families`)

## Manual inspection verdict

Gold patches do **not** pass L2 in this configuration. Inspection shows these
are validator-design limitations, not evidence that gold patches are bad fixes:

1. `targeted_regression` family is intentionally configured as
   `regression_forced_fail` (`sys.exit(1)`) in
   `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`.
   - Outcome: all rows fail with `L2_REGRESSION_FAIL` by construction.
   - Classification: **validator limitation** (controlled negative family),
     explicitly retained for protocol transparency.

2. `augmented_unit_tests` family uses an Astropy-specific command from
   `runs/released/l2_verified_astropy_v1/strengthening_spec.json`.
   - Outcome: rows fail with `L2_AUG_TESTS_FAIL` when the selector is not
     issue-aligned for the task/repo, or when harness-level behavior differs.
   - Classification: **validator non-applicability / limitation** outside the
     Astropy-targeted context.

## Disposition against acceptance criterion

- Every gold-patch L2 failure has been inspected at the family/spec level.
- No rows were silently removed after failure.
- Failures are explicitly reported as validator limitations in this document and
  in `docs/l2_failure_case_studies.md`.

Given the current L2 validator setup, these gold-patch failures should be read
as protocol stress-test outcomes, not as direct counterevidence against the
gold/developer patches.
