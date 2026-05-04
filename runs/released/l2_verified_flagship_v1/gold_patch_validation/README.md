# gold_patch_validation

Gold/developer patch replay for `l2_verified_flagship_v1`.

## Purpose

Run upstream dataset `patch` candidates for each flagship task through the same
L2 validator families used by the flagship run:

- `augmented_unit_tests` (`results_astropy/`)
- `targeted_regression` (`results_regressionfail/`)

## Inputs

- Tasks: inferred from `runs/released/l2_verified_flagship_v1/results/batch_summary.json`
- Gold patches: `datasets/cache/verified/swe_bench_verified.jsonl` (`patch` field)
- Specs:
  - `runs/released/l2_verified_astropy_v1/strengthening_spec.json`
  - `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`

## Commands

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py \
  --timeout-secs 900 \
  --short-timeout-secs 180 \
  --jobs 1
```

Exports are written to:

- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.json`

Manual inspection notes:

- `docs/evidence_manual.md` (L2 reference patch validation section)
