# Final validation matrix (NeurIPS 2026 E&D)

Fill **Status** during release closure (for example `ok`, `fail`, `skipped`, date).

| Gate | Command | Required | Status |
|------|---------|----------|--------|
| Build | `cargo build --workspace --all-targets` | yes | ok — 2026-05-01 |
| Format | `cargo fmt --all -- --check` | yes | ok — 2026-05-01 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | yes | ok — 2026-05-01 |
| Rust tests | `cargo test --workspace --all-targets` | yes | ok — 2026-05-01 |
| Python lint (`ci/scripts`) | `ruff check ci/scripts` | yes | ok — 2026-05-01 |
| Python lint (`packages/python`) | `ruff check packages/python` | yes | fail — 2026-05-01 (28 issues, mostly `packages/python/scripts/`; CI tier-2 runs this scope) |
| Python typecheck | `mypy` (root `pyproject.toml` `files`: `benchmark_compat/src`, `ci/scripts`) | yes | ok — 2026-05-01 |
| Schema validation | `cargo run --bin eval-ladder -- schema validate` | yes | ok — 2026-05-01 |
| Demo | `cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2` (fresh `runs/demo/bundles`) | yes | ok — 2026-05-01 (`2 ok / 0 invalid`) |
| Live gate | `python ci/scripts/check_evidence_quality.py --gate-profile release live --paper-export-dir paper/exports/live_panel_v2_postbatch` | yes | ok — 2026-05-01 |
| L2 gate | `python ci/scripts/check_evidence_quality.py --gate-profile release l2 --run-dir runs/released/l2_verified_flagship_v1/results` | yes | ok — 2026-05-01 |
| Rust proof release gate | `python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof --run-dir runs/released/rust_proof_subset_v1/results_seal` | yes | ok — 2026-05-01 |
| Verified feasibility report | `python ci/scripts/analyze_strict_feasibility.py` | yes | ok — 2026-05-01 |
| Gold validation | `python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2` | yes | skipped — 2026-05-01 (frozen Table 3 sources under `paper/exports/l2_verified_flagship_v1/gold_patch_validation*`; full regenerate needs release binary + OCI + both sealed arms; see matrix note below) |
| Claim-source check | `python ci/scripts/check_paper_claim_sources.py` (JSON + YAML mirror) | yes | ok — 2026-05-01 |
| Secret scan | `python ci/scripts/secret_scan_release.py` | yes | ok — 2026-05-01 |
| Verify released run-dir (Live v2) | `target/release/eval-ladder verify run-dir --run-dir runs/released/live_panel_v2/results_opt` | yes | ok — 2026-05-01 (`31 ok / 0 invalid`) |
| Verify released run-dir (L2 astropy arm) | `target/release/eval-ladder verify run-dir --run-dir runs/released/l2_verified_flagship_v1/results_astropy` | yes | ok — 2026-05-01 (`33 ok / 0 invalid`) |
| Verify released run-dir (L2 regression arm) | `target/release/eval-ladder verify run-dir --run-dir runs/released/l2_verified_flagship_v1/results_regression_fail` | yes | ok — 2026-05-01 (`33 ok / 0 invalid`) |
| Verify released run-dir (Rust proof) | `target/release/eval-ladder verify run-dir --run-dir runs/released/rust_proof_subset_v1/results_seal` | yes | ok — 2026-05-01 (`8 ok / 0 invalid`) |

**Gold validation note:** `l2_flagship_gold_patch_validation.py --jobs 2` needs the
release `eval-ladder` binary plus an OCI-capable runtime and both sealed arms
under `gold_patch_results/` when regenerating exports; `--skip-evaluate` never
deletes bundle trees but refuses partial row counts (see `artifacts/final_repro_log.md`).

## Verified feasibility summary (inventory frontier)

| Quantity | Value |
|----------|-------|
| Shared L1-pass tasks across three agents | 7 |
| One-candidate task-agent upper bound | 21 |
| Strict threshold | 30 |
| Status | inventory-bound frontier |
