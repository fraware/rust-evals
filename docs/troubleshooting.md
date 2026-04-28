# Troubleshooting

## Build failures

- **Rust build fails on first checkout**
  - Run: `rustup show` and confirm the pinned toolchain is installed.
  - Re-run: `cargo build --workspace`.

- **Python tooling fails**
  - Confirm Python `3.10+`.
  - Run lint/type/test with the same commands used in CI (`just ci-tier2`).

## Runtime failures

- **Docker-backed evaluations fail to start**
  - Confirm Docker engine is running.
  - Prewarm images for large panels before `evaluate batch`.

- **Batch run resumes with unexpected invalid rows**
  - Use a fresh `--out` directory when prior interrupted bundles exist.
  - Re-run with `--resume` only after checking partial bundle state.

- **Timeout-heavy runs on smaller machines**
  - Start with lower `--jobs` and enable `--adaptive-timeouts`.
  - Use the runbook machine-class recommendations for Rust panels.

## Reproducibility mismatches

- **Unexpected summary differences**
  - Validate schema: `cargo run --bin eval-ladder -- schema validate`
  - Verify bundle integrity: `eval-ladder verify run-dir --run-dir <path>`
  - Confirm identical evaluator config and strengthening spec paths.

- **Paper exports missing expected tables**
  - Re-run: `eval-ladder analyze paper-export --run-dir <run> --out-dir <export>`
  - Ensure the run includes the levels needed for the requested analysis.

## Terminology confusion

Use `docs/public_terminology.md` for canonical public labels and mappings from
legacy internal shorthand.
