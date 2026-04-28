<h1 align="center">eval-ladder</h1>

<p align="center">
  A deterministic evaluator for repository-level coding-agent benchmarks.
</p>

<p align="center">
  <code>eval-ladder</code> evaluates <strong>existing candidate patches</strong>; it does not generate patches.
  It is built to make benchmark claims auditable, reproducible, and explicitly evaluator-conditioned.
</p>

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

```bash
eval-ladder evaluate batch \
  --input runs/released/agent_panel_v1/panel.jsonl \
  --levels L0,L1,L2,L3 \
  --resume \
  --jobs 2 \
  --out runs/released/agent_panel_v1/results/
```

### Analyze and export

```bash
eval-ladder analyze paper-export \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out-dir paper/exports/agent_panel_v1/
```

### Verify artifact integrity

```bash
eval-ladder verify run-dir --run-dir runs/released/agent_panel_v1/results/
```

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
