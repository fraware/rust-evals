# Final reproduction log (NeurIPS 2026 closure)

Commands were executed from the repository root on a Windows 10 developer host
(2026-05-01). Adjust paths for `eval-ladder.exe` vs `eval-ladder` on Unix.

## 5.1 Bootstrap

```text
$ rustup show
(active toolchain overridden by rust-toolchain.toml — e.g. 1.86.x stable MSVC)
```

```text
$ cargo build --workspace --all-targets
   Finished `dev` profile [unoptimized + debuginfo] target(s) in ~6s
```

Python environment (per `pyproject.toml` / operational runbook):

```bash
python -m pip install -e ".[dev]"
```

## 5.2 Tier CI

```bash
just ci-tier1
just ci-tier2
```

## 5.3 Direct checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo run --bin eval-ladder -- schema validate
python -m mypy packages/python ci/scripts
python -m ruff check packages/python ci/scripts
```

## 5.4 Reproducibility demo

If `runs/demo/bundles` already exists from a prior run, remove it first; the
demo requires empty bundle directories.

```powershell
Remove-Item -Recurse -Force runs/demo/bundles -ErrorAction SilentlyContinue
```

```bash
cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2
```

Observed success line:

```text
demo: ok (2 tasks …)
verify: 2 ok / 0 invalid (2 total)
```

## 5.5 Verify released run directories

Release binary (after `cargo build --release` for the CLI crate name used in
this workspace):

```bash
target/release/eval-ladder verify run-dir \
  --run-dir runs/released/live_panel_v2/results_opt
```

Observed: `31 ok / 0 invalid`.

```bash
target/release/eval-ladder verify run-dir \
  --run-dir runs/released/l2_verified_flagship_v1/results_astropy
target/release/eval-ladder verify run-dir \
  --run-dir runs/released/l2_verified_flagship_v1/results_regression_fail
```

Observed: `33 ok / 0 invalid` for each arm. The merged `results/` directory
contains the joined `batch_summary.json` used for analysis gates but does not
store per-candidate bundle leaf directories; use the arm paths above for
`verify run-dir`.

```bash
target/release/eval-ladder verify run-dir \
  --run-dir runs/released/rust_proof_subset_v1/results_seal
```

Observed: `8 ok / 0 invalid`.

## Evidence gates (publication / release)

```bash
python ci/scripts/check_evidence_quality.py live \
  --paper-export-dir paper/exports/live_panel_v2_postbatch
python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_flagship_v1/results
python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal
```

Observed: all three returned `"ok": true` for this workspace snapshot.

## Claim-source checker

Requires dev extras (`pip install -e ".[dev]"`) so `PyYAML` is available for the
YAML mirror check.

```bash
python ci/scripts/check_paper_claim_sources.py
```

Observed: `check_paper_claim_sources: OK`.

Canonical wiring:

- `paper/paper_claim_sources.json` (machine-readable)
- `paper/paper_claim_sources.yaml` (editor mirror; must match JSON byte-for-byte after canonicalisation)

## Live leave-one-out appendix CSV

```bash
python ci/scripts/live_panel_leave_one_out.py \
  --paper-export-dir paper/exports/live_panel_v2_postbatch \
  --out paper/exports/live_panel_v2_postbatch/live_leave_one_out.csv
```

Requires `per_task_live_outcomes.csv` in the export directory (from
`export_live_panel_tables.py` / `just reproduce-paper-tables`).

## Paper export regeneration

Requires a **release** `eval-ladder` binary (see `just reproduce-paper-tables`):

```bash
just reproduce-paper-tables
```

This runs `eval-ladder analyze paper-export` for Live v2, L2 flagship, and the
Rust proof seal release, plus downstream Python exporters and strict
feasibility analysis.

## Gold patch validation (headline profile)

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2
```

## Lean (optional, L4)

From the operational runbook / proof-subset policy:

```bash
cd packages/lean/EvalLadder
lake build
```

## Secret scan (maintainer)

Example patterns (extend as needed for anonymized archives):

```bash
grep -R "OPENAI_API_KEY" . || true
grep -R "ANTHROPIC_API_KEY" . || true
grep -R "GITHUB_TOKEN" . || true
```

On Windows, prefer `rg` with equivalent patterns and explicit `--glob` excludes
for large vendored trees.
