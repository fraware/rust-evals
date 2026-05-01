<h1 align="center">eval-ladder</h1>

<p align="center">
  A deterministic evaluator for repository-level coding-agent benchmarks.
</p>

<p align="center">
  <code>eval-ladder</code> evaluates <strong>existing candidate patches</strong>; it does not generate patches.
  It is built to make benchmark claims auditable, reproducible, and explicitly evaluator-conditioned.
</p>

## Reviewer quick path

This artifact supports the NeurIPS 2026 E&D submission:
**Eval-Ladder: Evaluator-Conditioned Measurement for Repository-Level Coding-Agent Benchmarks.**

The artifact evaluates **fixed candidate patches**. It does **not** generate patches.

**Headline empirical surfaces**

1. **Live v2** static-vs-live diagnostic:
   - `runs/released/live_panel_v2/results_opt/`
   - `paper/exports/live_panel_v2_postbatch/`
2. **L2 flagship** diagnostic:
   - `runs/released/l2_verified_flagship_v1/results/`
   - `paper/exports/l2_verified_flagship_v1/`

**Evidence-frontier surfaces**

1. **Verified** strict comparison (inventory bound):
   - `paper/exports/strict_feasibility_report.json`
2. **Rust proof subset**:
   - `runs/released/rust_proof_subset_v1/results_seal/`

**Minimal reproduction**

```bash
cargo build --workspace
cargo run --bin eval-ladder -- schema validate
cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2
```

**Claim discipline:** `docs/CLAIM_LOCK_NEURIPS2026.md`, `paper/exports/CLAIM_SOURCE_MAP.md`, and `ci/scripts/check_paper_claim_sources.py`.

**Engineering closure:** `paper/exports/release/final_validation_matrix.md` (gate log) and
`paper/exports/release/MANUSCRIPT_READY_SIGNOFF.md` (manuscript-ready sign-off).

### What this artifact does not claim

- It does not generate coding-agent patches.
- It does not replace SWE-bench.
- It does not estimate population-level bug rates from the L2 diagnostic slice.
- It does not prove full semantic correctness of candidate patches.
- It does not use synthetic L4 counterexamples as headline empirical evidence.

## Why this project exists

Benchmark pass rates can change when evaluator assumptions change. `eval-ladder`
makes those assumptions explicit through a levelled evaluation model and
evidence-first outputs.

## Evaluation ladder

| Level | Name | What it asks |
|---|---|---|
| `L0` | Official | Does the benchmark's native scorer mark success? |
| `L1` | Trusted rerun | Does success survive deterministic replay? |
| `L2` | Strengthened | Does success hold under stronger validators? |
| `L3` | Policy-conformant | Was success achieved through an allowed process? |
| `L4` | Semantic | Does the patch satisfy a machine-checkable obligation? |

More detail: [`docs/evaluation_ladder.md`](docs/evaluation_ladder.md)

## Quick start

Prerequisites:

- Rust toolchain pinned by `rust-toolchain.toml`
- Python `3.10+`
- Docker (for SWE-bench Verified / SWE-bench-Live runs)

```bash
# Build
cargo build --workspace

# Inspect CLI
cargo run --bin eval-ladder -- --help

# Validate schemas
cargo run --bin eval-ladder -- schema validate

# Run reproducibility demo (fast, local, no upstream data)
cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2
```

Common task wrappers:

```bash
just ci-tier1
just ci-tier2
```

## Core workflows

### Ingest benchmark manifests

```bash
eval-ladder ingest verified --manifest configs/evaluator/verified.toml
eval-ladder ingest live --manifest configs/evaluator/live.toml
eval-ladder ingest rust --manifest configs/evaluator/rust.toml
```

### Evaluate a batch

Example using the **frozen Live v2** panel (headline NeurIPS comparative surface):

```bash
eval-ladder evaluate batch \
  --input runs/released/live_panel_v2/panel.jsonl \
  --config configs/evaluator/default.toml \
  --levels L0,L1 \
  --resume \
  --jobs 2 \
  --out runs/released/live_panel_v2/results_opt/
```

Verified-style and Rust panels use different `--input` paths (for example
`runs/released/agent_panel_v3_r1/`); see `docs/operational_runbook.md`.

### Analyze and export

```bash
eval-ladder analyze paper-export \
  --run-dir runs/released/live_panel_v2/results_opt \
  --out-dir paper/exports/live_panel_v2_postbatch
```

### Verify artifact integrity

```bash
eval-ladder verify run-dir --run-dir runs/released/live_panel_v2/results_opt
```

### Older tutorial panels

Smaller frozen panels (for example `runs/released/agent_panel_v1/`) remain in-tree for
regression tests and long-form examples in `docs/operational_runbook.md`. For **Verified
flagship**, **batch optimization**, and **Rust proof** recipes, follow Milestone H there rather
than assuming `agent_panel_v1` paths.

## Repository map

```text
packages/rust/      evaluator core, runner, policy, analysis, CLI
packages/python/    benchmark compatibility scripts and pipelines
packages/lean/      L4 semantic obligations
benchmarks/         benchmark adapters + manifests
configs/            evaluator, policy, and strengthening configs
schemas/            JSON schemas for persisted artifacts
datasets/           source links + derived proof subset metadata
runs/               released and local run artifacts
docs/               architecture, runbook, scope, and evidence docs
paper/              paper-facing exports and tables
ci/                 CI scripts and release hygiene tooling
tests/              Rust + Python + integration tests
```

## Released artifacts

Release-track outputs live under `runs/released/` and are accompanied by
paper-facing exports in `paper/exports/`.

Start here:

- `runs/released/agent_panel_v3_r1/`
- `runs/released/l2_verified_flagship_v1/`
- `runs/released/live_panel_v2/`
- `runs/released/rust_proof_subset_v1/`

## Documentation guide

- Docs index: [`docs/README.md`](docs/README.md)
- Claim source map: [`paper/exports/CLAIM_SOURCE_MAP.md`](paper/exports/CLAIM_SOURCE_MAP.md)
- Getting started: [`docs/getting_started.md`](docs/getting_started.md)
- Operational runbook: [`docs/operational_runbook.md`](docs/operational_runbook.md)
- Artifact model: [`docs/artifact_spec.md`](docs/artifact_spec.md)
- Public terminology: [`docs/public_terminology.md`](docs/public_terminology.md)
- Submission/release checklist: [`docs/submission_checklist.md`](docs/submission_checklist.md)

## CI and release

Main workflows in `.github/workflows/`:

- `ci-tier1-fast.yml`
- `ci-tier2-medium.yml`
- `ci-tier3-heavy.yml`
- `release-tag.yml`

Tag releases with semantic version tags (`v*.*.*`) to trigger release checks.

## Contributing

- Contribution guide: [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Security policy: [`SECURITY.md`](SECURITY.md)
- Code of conduct: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)

## License

Dual-licensed under Apache-2.0 or MIT:

- [`LICENSE-APACHE`](LICENSE-APACHE)
- [`LICENSE-MIT`](LICENSE-MIT)

Additional attribution:

- [`THIRD_PARTY_LICENSES.md`](THIRD_PARTY_LICENSES.md)
- [`NOTICE`](NOTICE)
