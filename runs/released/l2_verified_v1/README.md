# l2_verified_v1

Released L2 strengthening run over a pinned SWE-bench Verified slice.

## Scope

- Tasks: 5 (`astropy` + `django`), pinned in `panel.jsonl`.
- Candidates: deterministic `golden` candidates built from cached dataset
  `patch` bytes.
- Levels: `L0,L1,L2`.
- L2 mode: `tests_plus_regression`.
- L2 spec: `strengthening_spec.json`.

## Validator families (L2)

- `augmented_unit_tests`: one deterministic failing command
  (`python -c "import sys; sys.exit(1)"`) to prove strictness and attribution.
- `targeted_regression`: one deterministic passing command
  (`python -c "import sys; sys.exit(0)"`) to demonstrate family decomposition.

Per-task family applicability and rationale are frozen in `rationale.json`.

## Observed results

- `batch_summary.json`: `total=5`, `ok=5`, `invalid=0`.
- `verify_report.json`: `ok=5`, `invalid=0`.
- L2 family attribution from `strengthening_report.json` across bundles:
  - `augmented_unit_tests`: 5 fail (`L2_AUG_TESTS_FAIL`)
  - `targeted_regression`: 5 pass (`PASS`)

## Reproduction

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --levels L0,L1,L2 `
  --input runs/released/l2_verified_v1/panel.jsonl `
  --config configs/evaluator/verified.toml `
  --strengthening-spec runs/released/l2_verified_v1/strengthening_spec.json `
  --strengthening-mode tests_plus_regression `
  --out runs/released/l2_verified_v1/results `
  --timeout-secs 5400 `
  --seed-tag l2-verified-v1 `
  --deterministic-clock

cargo run -p eval-ladder-cli -- verify run-dir `
  --run-dir runs/released/l2_verified_v1/results `
  --out runs/released/l2_verified_v1/results/verify_report.json

cargo run -p eval-ladder-cli -- analyze paper-export `
  --run-dir runs/released/l2_verified_v1/results `
  --out-dir paper/exports/l2_verified_v1
```

## Follow-on artifact

For a frozen **L0/L1 pass to L2 fail** transition on real Verified tasks with
Docker-backed evidence, see `runs/released/l2_verified_v2/` (golden candidates,
`tests_plus_regression`, and `strengthening_spec.json` tuned to fail augmented
checks while L0/L1 remain passes).
