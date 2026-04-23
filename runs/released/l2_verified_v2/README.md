# l2_verified_v2

Released Docker-backed run demonstrating a **real L0/L1 pass to L2 fail**
transition on SWE-Bench Verified using deterministic golden candidates.

## Scope

- Tasks: 2 (`astropy__astropy-12907`, `pydata__xarray-2905`), pinned in
  `panel.jsonl`. Candidates and workspace templates are reused from
  `runs/released/l0l1_pass_hunt_v1/` via relative paths.
- Levels: `L0`, `L1`, `L2`.
- L2 mode: `tests_plus_regression` (augmented unit tests plus targeted
  regression).
- L2 spec: `strengthening_spec.json` (warnings-as-errors style augmented
  check on `import pkg_resources`, plus a baseline import sanity command in
  regression).

Per-task rationale and family intent are frozen in `rationale.json`.

## Observed results (frozen bundle)

- `results/batch_summary.json`: `total_entries=2`, `ok_entries=2`,
  `invalid_entries=0`.
- Both entries: L0 `pass`, L1 `pass`, L2 `fail` with primary reason
  `L2_AUG_TESTS_FAIL` (augmented check trips; regression family passes in
  `strengthening_report.json`).
- `results/verify_report.json`: all bundles OK when produced with
  `verify run-dir`.

Paper exports (schema v3 cumulative default) live under
`paper/exports/l2_verified_v2/`.

## Reproduction

Prerequisites: Docker, pulled SWE-bench evaluation images for the two tasks,
and ingested manifests under `benchmarks/verified/manifests/`.

From the repository root (PowerShell-friendly separators):

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --levels L0,L1,L2 `
  --input runs/released/l2_verified_v2/panel.jsonl `
  --config configs/evaluator/default.toml `
  --strengthening-spec runs/released/l2_verified_v2/strengthening_spec.json `
  --strengthening-mode tests_plus_regression `
  --out runs/released/l2_verified_v2/results `
  --timeout-secs 5400 `
  --seed-tag l2-verified-v2 `
  --deterministic-clock

cargo run -p eval-ladder-cli -- verify run-dir `
  --run-dir runs/released/l2_verified_v2/results `
  --out runs/released/l2_verified_v2/results/verify_report.json

cargo run -p eval-ladder-cli -- analyze paper-export `
  --run-dir runs/released/l2_verified_v2/results `
  --out-dir paper/exports/l2_verified_v2
```

## Relation to l2_verified_v1

`l2_verified_v1` freezes an earlier L2 family-decomposition slice on five
tasks. This directory is the successor artifact for a **scientifically
interpretable** lower-rung pass with L2 strictness failure on real benchmark
tasks.
