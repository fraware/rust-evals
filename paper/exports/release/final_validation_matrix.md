# Final validation matrix (NeurIPS 2026 E&D)

Fill **Status** during release closure (for example `ok`, `fail`, `skipped`, date).

| Gate | Command | Required | Status |
|------|---------|----------|--------|
| Build | `cargo build --workspace --all-targets` | yes |  |
| Format | `cargo fmt --all -- --check` | yes |  |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | yes |  |
| Rust tests | `cargo test --workspace --all-targets` | yes |  |
| Python lint | `ruff check packages/python ci/scripts` | yes |  |
| Python typecheck | `mypy packages/python ci/scripts` | yes |  |
| Schema validation | `cargo run --bin eval-ladder -- schema validate` | yes |  |
| Demo | `cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2` (use a fresh `runs/demo/bundles` or delete it first) | yes |  |
| Live gate | `python ci/scripts/check_evidence_quality.py live --paper-export-dir paper/exports/live_panel_v2_postbatch` | yes |  |
| L2 gate | `python ci/scripts/check_evidence_quality.py l2 --run-dir runs/released/l2_verified_flagship_v1/results` | yes |  |
| Rust proof release gate | `python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof --run-dir runs/released/rust_proof_subset_v1/results_seal` | yes |  |
| Verified feasibility report | `python ci/scripts/analyze_strict_feasibility.py` | yes |  |
| Gold validation | `python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2` | yes |  |
| Claim-source check | `python ci/scripts/check_paper_claim_sources.py` (JSON + YAML mirror) | yes |  |
| Secret scan | Maintainer grep / custom script (see `artifacts/final_repro_log.md`) | yes |  |

## Verified feasibility summary (inventory frontier)

| Quantity | Value |
|----------|-------|
| Shared L1-pass tasks across three agents | 7 |
| One-candidate task-agent upper bound | 21 |
| Strict threshold | 30 |
| Status | inventory-bound frontier |
