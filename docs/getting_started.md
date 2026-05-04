# Getting started

This guide is the fastest path from clone to a verified local result.

## Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml`
- Python `3.10+`
- Docker (required for SWE-bench Verified and SWE-bench-Live evaluation surfaces)

## 5-minute first run

```bash
cargo build --workspace
cargo run --bin eval-ladder -- schema validate
cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2
cargo run --bin eval-ladder -- verify run-dir --run-dir runs/demo/bundles
```

Expected outcome:

- Demo run completes quickly and writes frozen reproducibility outputs under `runs/demo/`.
- `verify run-dir` reports no invalid bundles.

## Reproduce key paper-facing tables

```bash
just reproduce-paper-tables
```

Primary export locations:

- `paper/exports/live_panel_v2_postbatch/`
- `paper/exports/l2_verified_flagship_v1/`
- `paper/exports/strict_feasibility_report.json`

## Next steps

- Use `docs/cli_reference.md` for command-level workflows.
- Use `docs/evidence_manual.md` for longer batch execution and optimization.
- Use `docs/troubleshooting.md` if setup or runtime checks fail.
