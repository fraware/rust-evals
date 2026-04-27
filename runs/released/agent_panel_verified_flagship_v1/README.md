# agent_panel_verified_flagship_v1

Canonical **Verified headline** slice derived from `agent_panel_v3_r1` by
dropping the worst harness-native clusters (matplotlib, scikit-learn,
pytest-dev) while keeping three public agents and 11 tasks (33 candidates).

## Panel

| File | Description |
|------|-------------|
| `panel.jsonl` | 33 entries; `workspace_template` points at `../agent_panel_v3_r1/workspaces/<task>/` (no duplicate checkouts). |
| `candidates/`, `patches/` | Copied from v3_r1 for the retained tasks. |
| `provenance.json` | Deterministic record of exclusions and counts. |

Materialise or refresh from v3_r1:

```bash
python packages/python/scripts/build_verified_flagship_v1.py
```

## Evaluator and L3 policy

Use `configs/evaluator/verified_headline.toml`, which references
`configs/policy/swe_bench_verified_headline.toml` (deny-only edit scope so
typical SWE-Bench package layouts are not rejected as `PV_EDIT_SCOPE`).

## Optimized batch (L0, L1, L3)

After `just eval-ladder-cli-release` (or `cargo build -p eval-ladder-cli --release`):

```bash
./target/release/eval-ladder evaluate batch \
  --input runs/released/agent_panel_verified_flagship_v1/panel.jsonl \
  --config configs/evaluator/verified_headline.toml \
  --levels L0,L1,L3 \
  --policy configs/policy/swe_bench_verified_headline.toml \
  --out runs/released/agent_panel_verified_flagship_v1/results_opt \
  --timeout-secs 3600 --short-timeout-secs 900 --adaptive-timeouts \
  --resume --jobs 2 --l1-strategy smart_rust_reuse \
  --rust-target-cache-root runs/released/.eval_ladder_cargo_cache \
  --seed-tag verified-flagship-v1 --deterministic-clock
```

### Sealed `results_opt` (machine run)

- `results_opt/batch_summary.json`: **33 ok / 0 invalid** (`eval-ladder verify run-dir` clean).
- Archived copy: `evidence/batch_summary_results_opt_v1.json`.
- **Strict gate** still fails on harness rate (~0.64), tied L0/L1 pass vectors
  across agents, and L3 failure mass dominated by **`PV_ENV_NONDETERMINISTIC`**
  on this slice (headline deny-only edit scope collapsed **`PV_EDIT_SCOPE`**
  to a single row; remaining work is harness + nondeterminism triage or task
  mix).

Strict headline gate:

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_verified_flagship_v1/results_opt
```

Bundle integrity:

```bash
eval-ladder verify run-dir \
  --run-dir runs/released/agent_panel_verified_flagship_v1/results_opt
```
