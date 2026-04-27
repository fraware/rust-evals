# l2_verified_flagship_v1

Canonical strict-pass L2 expansion batch built from the Verified flagship panel
with two strengthening families and deduped merge semantics.

## Inputs and families

- Base panel: `runs/released/agent_panel_verified_flagship_v1/panel.jsonl`
  (33 entries, 11 tasks x 3 agents).
- Family A (augmented-fail): `runs/released/l2_verified_astropy_v1/strengthening_spec.json`.
- Family B (regression-fail): `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`.

Family-specific panel variants append a suffix to `entry_id` and `bundle_name`
so merge dedupe keeps both families:

- `runs/released/agent_panel_verified_flagship_v1/panel_l2_astropy.jsonl`
- `runs/released/agent_panel_verified_flagship_v1/panel_l2_regression_fail.jsonl`

## Execute

```bash
./target/release/eval-ladder evaluate batch \
  --levels L0,L1,L2 \
  --input runs/released/agent_panel_verified_flagship_v1/panel_l2_astropy.jsonl \
  --config configs/evaluator/default.toml \
  --strengthening-spec runs/released/l2_verified_astropy_v1/strengthening_spec.json \
  --strengthening-mode tests_plus_regression \
  --out runs/released/l2_verified_flagship_v1/results_astropy \
  --timeout-secs 5400 --short-timeout-secs 900 --adaptive-timeouts \
  --resume --jobs 2 --seed-tag l2-flagship-astropy --deterministic-clock

./target/release/eval-ladder evaluate batch \
  --levels L0,L1,L2 \
  --input runs/released/agent_panel_verified_flagship_v1/panel_l2_regression_fail.jsonl \
  --config configs/evaluator/default.toml \
  --strengthening-spec runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json \
  --strengthening-mode tests_plus_regression \
  --out runs/released/l2_verified_flagship_v1/results_regression_fail \
  --timeout-secs 5400 --short-timeout-secs 900 --adaptive-timeouts \
  --resume --jobs 2 --seed-tag l2-flagship-regressionfail --deterministic-clock
```

Both family runs are sealed as `33 ok / 0 invalid`.

## Merge and strict gate

```bash
python ci/scripts/merge_l2_batch_summaries.py \
  --inputs \
    runs/released/l2_verified_flagship_v1/results_astropy/batch_summary.json \
    runs/released/l2_verified_flagship_v1/results_regression_fail/batch_summary.json \
  --out-dir runs/released/l2_verified_flagship_v1/results

python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_flagship_v1/results
```

Strict result (`ok: true`):

- `total_entries`: `66`
- `l1_passed_from`: `24`
- `l2_failures`: `24`
- `l2_reason_counts`:
  - `L2_AUG_TESTS_FAIL`: `12`
  - `L2_REGRESSION_FAIL`: `12`
