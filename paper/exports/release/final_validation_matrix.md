# Final validation matrix (NeurIPS 2026 E&D)

Fill **Status** during release closure (for example `ok`, `fail`, `skipped`, date).

Last closure refresh: **2026-05-03** (engineering manuscript-ready pass).

| Gate | Command | Required | Status |
|------|---------|----------|--------|
| Build | `cargo build --workspace --all-targets` | yes | ok — 2026-05-03 |
| Format | `cargo fmt --all -- --check` | yes | ok — 2026-05-03 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | yes | ok — 2026-05-03 |
| Rust tests | `cargo test --workspace --all-targets` | yes | ok — 2026-05-03 |
| Python lint (`ci/scripts`) | `ruff check ci/scripts` | yes | ok — 2026-05-03 |
| Python lint (`packages/python`) | `ruff check packages/python` | yes | ok — 2026-05-03 |
| Python typecheck | `mypy` (root `pyproject.toml` `files`: `benchmark_compat/src`, `ci/scripts`) | yes | ok — 2026-05-03 |
| Schema validation | `cargo run --bin eval-ladder -- schema validate` | yes | ok — 2026-05-03 |
| Tier-1 evidence script | `python ci/scripts/run_evidence_tier1_checks.py` | yes | ok — 2026-05-03 |
| Claim-source check | `python ci/scripts/check_paper_claim_sources.py` | yes | ok — 2026-05-03 |
| Secret scan | `python ci/scripts/secret_scan_release.py` | yes | ok — 2026-05-03 |
| Demo | `cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2` (fresh `runs/demo/bundles`) | yes | ok — 2026-05-01 |
| Live gate | `python ci/scripts/check_evidence_quality.py --gate-profile release live --paper-export-dir paper/exports/live_panel_v2_postbatch` | yes | ok — 2026-05-03 |
| L2 gate | `python ci/scripts/check_evidence_quality.py --gate-profile release l2 --run-dir runs/released/l2_verified_flagship_v1/results` | yes | ok — 2026-05-03 |
| Rust proof release gate | `python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof --run-dir runs/released/rust_proof_subset_v1/results_seal` | yes | ok — 2026-05-03 |
| Verified feasibility report | `python ci/scripts/analyze_strict_feasibility.py` | yes | ok — 2026-05-03 |
| Gold validation | See **Gold validation commands** (below) | yes | ok — 2026-05-03 (frozen exports verified; partial row counts refused by script) |
| Verify released run-dir (Live v2) | `target/release/eval-ladder verify run-dir --run-dir runs/released/live_panel_v2/results_opt` | yes | ok — 2026-05-03 (`31 ok / 0 invalid`) |
| Verify released run-dir (L2 astropy arm) | `target/release/eval-ladder verify run-dir --run-dir runs/released/l2_verified_flagship_v1/results_astropy` | yes | ok — 2026-05-03 (`33 ok / 0 invalid`) |
| Verify released run-dir (L2 regression arm) | `target/release/eval-ladder verify run-dir --run-dir runs/released/l2_verified_flagship_v1/results_regression_fail` | yes | ok — 2026-05-03 (`33 ok / 0 invalid`) |
| Verify released run-dir (Rust proof) | `target/release/eval-ladder verify run-dir --run-dir runs/released/rust_proof_subset_v1/results_seal` | yes | ok — 2026-05-03 (`8 ok / 0 invalid`) |

**Gold validation note:** Bundle outputs under `gold_patch_results/results_*` are gitignored.
Committed `paper/exports/l2_verified_flagship_v1/gold_patch_validation*` files are the Table 3
source; `--skip-evaluate` regenerates them only when both sealed arms produce complete row counts.

### Gold validation commands

Full documentation: `paper/exports/release/gold_validation_export_only_log.md`.

- **Export-only / skip evaluate** (requires complete local bundle trees under both arms):

  ```bash
  python ci/scripts/l2_flagship_gold_patch_validation.py --skip-evaluate
  ```

- **Full batch regeneration** (release `eval-ladder` binary, OCI runtime, both arms):

  ```bash
  python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2
  ```

## Verified feasibility summary (inventory frontier)

| Quantity | Value |
|----------|-------|
| Shared L1-pass tasks across three agents | 7 |
| One-candidate task-agent upper bound | 21 |
| Strict threshold | 30 |
| Status | inventory-bound frontier |
