# gold_patch_results

Gold/developer patch replay for ``l2_verified_flagship_v1`` (11 tasks).

**Profile:** ``gold_mechanical``

## Evaluator stack

- Config: ``configs/evaluator/default.toml``
- Mode: ``tests_plus_regression``
- **Both** arm batches use: ``runs/released/l2_verified_flagship_v1/strengthening_spec_gold_mechanical.json`` (pre-spec headline gold validation; trivial L2 smoke checks).

## Layout

- ``results_astropy/`` — replay labeled ``augmented_unit_tests`` in exports.
- ``results_regressionfail/`` — replay labeled ``targeted_regression`` in exports.

Exports: ``paper/exports/l2_verified_flagship_v1/gold_patch_validation*.csv/json``.

Regenerate:

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 1
python ci/scripts/l2_flagship_gold_patch_validation.py --strict-flagship-specs --jobs 1  # diagnostic
```
