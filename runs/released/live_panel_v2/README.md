# live_panel_v2

Comparative Live + static anchor panel with an **asymmetric live patch
matrix** so agents are not forced into identical live pass rates (addressing
the strict `check_evidence_quality live` tie and zero Kendall tau on v1).

See `packages/python/scripts/build_live_panel_v2.py` for the exact assignment.

## Build

Requires network for first-time workspace materialisation (git shallow fetch).

```bash
python packages/python/scripts/build_live_panel_v2.py
```

## Evaluate and export

```bash
just eval-ladder-cli-release
./target/release/eval-ladder evaluate batch \
  --levels L0,L1 \
  --input runs/released/live_panel_v2/panel.jsonl \
  --config configs/evaluator/default.toml \
  --out runs/released/live_panel_v2/results_opt \
  --timeout-secs 5400 --short-timeout-secs 900 --adaptive-timeouts \
  --resume --jobs 2 --seed-tag live-panel-v2-opt --deterministic-clock

./target/release/eval-ladder analyze paper-export \
  --run-dir runs/released/live_panel_v2/results_opt \
  --out-dir paper/exports/live_panel_v2_postbatch
```

Strict comparative gate:

```bash
python ci/scripts/check_evidence_quality.py live \
  --paper-export-dir paper/exports/live_panel_v2_postbatch
```
