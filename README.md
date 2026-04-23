# eval-ladder

A Rust-first scientific evaluation monorepo for auditing coding-agent benchmarks.

eval-ladder is **not** a coding-agent framework. It is an evaluator. It takes
*externally generated* candidate patches on SWE-Bench Verified, SWE-bench-Live,
and a Rust-native benchmark slice, re-runs them deterministically, and scores
them along an evaluation ladder whose higher rungs are progressively more
semantically justified than official benchmark scoring.

## Paper claim this repository is built to support

> Official coding-agent benchmark scores overstate semantically justified issue
> resolution; a trusted evaluator and a curated proof-carrying subset reveal
> the size and structure of that overstatement.

## The evaluation ladder

| Level | Name                | Definition                                                             |
|-------|---------------------|------------------------------------------------------------------------|
| L0    | Official            | Passes official benchmark validation exactly as the benchmark reports. |
| L1    | Trusted rerun       | Passes deterministic rerun in our evaluator harness.                   |
| L2    | Strengthened        | Passes augmented tests, differential checks, regression checks.        |
| L3    | Policy-conformant   | The success was achieved through a valid process (commands, edits, network, determinism). |
| L4    | Semantic            | Satisfies a machine-checkable semantic obligation on the curated proof subset. |

See [`docs/evaluation_ladder.md`](docs/evaluation_ladder.md) for complete
semantics. A candidate may pass L2 and still fail L3. That is intended.

## Repository layout

```
packages/rust/       # Rust workspace: evaluator core, runner, policy, traces,
                     # evidence, benchmark adapters, strengthening, analysis, CLI.
packages/python/     # Thin Python compatibility layer for SWE-bench tooling.
packages/lean/       # Lean 4 project defining the L4 proof obligations.
schemas/             # Versioned JSON schemas for every persisted artifact.
configs/             # Default evaluator, policy, strengthening, and proof-subset configs.
benchmarks/          # Per-benchmark adapters and manifests (verified, live, rust).
datasets/            # Public-source links and the curated proof subset manifest.
tasks/               # Candidate resolutions and derived task artifacts.
runs/                # Released, local, and CI run outputs.
tests/               # Rust, Python, and integration fixtures.
docs/                # Architecture, scope, runbooks, submission checklist.
ci/                  # GitHub Actions workflows and helper scripts.
paper/               # Tables, figures, and exports for the submission.
```

## Quick start

Prerequisites: the Rust toolchain pinned by `rust-toolchain.toml`
(1.86 at the time of writing) and Python 3.10+.

Docker (or another OCI runtime) is required for the Python benchmark
surfaces (`SWE-bench Verified`, `SWE-bench-Live`) because their official
entrypoints run in benchmark-provided task images. Rust-native runs can
execute L0/L1 with `LocalProcessEngine` directly on the host toolchain.
The Milestone K demo needs only the Rust toolchain.

```bash
# Build the Rust workspace.
cargo build --workspace

# Inspect the CLI.
cargo run --bin eval-ladder -- --help

# Validate the shipped JSON schemas.
cargo run --bin eval-ladder -- schema validate

# Run the reproducibility demo: generates a synthetic panel, drives
# the full batch pipeline, emits paper-export tables, and re-verifies
# every sealed bundle. No upstream data, no network, no containers.
# Completes in about a second on a developer laptop.
cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2
```

The `just` recipes in [`justfile`](justfile) wrap the common flows:

```bash
just ci-tier1       # fmt-check, clippy, test, schema validation
just ci-tier2       # tier1 + python lint and tests
```

## Primary CLI surface

```bash
eval-ladder ingest verified --manifest configs/evaluator/verified.toml
eval-ladder ingest live     --manifest configs/evaluator/live.toml
eval-ladder ingest rust     --manifest configs/evaluator/rust.toml

eval-ladder evaluate candidate \
  --candidate tasks/candidate_resolutions/<id>.json \
  --levels L0,L1,L2,L3 \
  --config configs/evaluator/verified.toml

eval-ladder evaluate batch \
  --input runs/released/agent_panel_v1/panel.jsonl \
  --levels L0,L1,L2,L3 \
  --out runs/released/agent_panel_v1/results/

eval-ladder prove-subset \
  --subset datasets/derived/proof_subset/manifest.jsonl \
  --candidate-dir runs/released/agent_panel_v1/results/ \
  --lean-root packages/lean/EvalLadder

eval-ladder analyze score-descent             --run-dir runs/released/agent_panel_v1/results/
eval-ladder analyze conditional-false-success --run-dir runs/released/agent_panel_v1/results/
eval-ladder analyze rank-stability            --run-dir runs/released/agent_panel_v1/results/
eval-ladder analyze taxonomy                  --run-dir runs/released/agent_panel_v1/results/
eval-ladder analyze static-vs-live            --run-dir runs/released/agent_panel_v1/results/
eval-ladder analyze paper-export \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out-dir paper/exports/agent_panel_v1/

eval-ladder verify run-dir --run-dir runs/released/agent_panel_v1/results/
eval-ladder demo run --out runs/demo --tasks 2
```

## Released run status

The repository currently ships two release-track run directories:

- `runs/released/agent_panel_v1/`: panel artifacts (candidates, patches,
  panel metadata) for 3 agents x 5 Verified tasks. This release is
  intentionally metadata-only until a Docker-backed reviewer run is
  executed.
- `runs/released/rust_pilot_v1/`: completed Rust-native pilot run for
  `clap-rs__clap_5873` with:
  - `evaluate batch` levels `L0,L1,L3,L4`,
  - `analyze paper-export` output in `paper/exports/rust_pilot_v1/`,
  - `verify run-dir` passing with `1 ok / 0 invalid`.

## Scientific scope and related work

eval-ladder is designed to absorb, reproduce, and extend the findings in:

- Jimenez et al., **SWE-bench** (2023): 2,294 tasks, 12 Python repositories.
- SWE-Bench+ (2024): solution leakage and weak tests that collapse a system from 12.47% to 3.97%.
- **PatchDiff** (2024): 7.8% of accepted patches differ from developer tests; ~6.2-point inflation.
- **UTBoost** (2024): insufficient tests affect 40.9% of SWE-Bench Lite and 24.4% of SWE-Bench Verified entries.
- **SWE-bench-Live** (2025): 1,319 live tasks from 93 repositories, reducing static-benchmark contamination.
- **Rust-SWE-bench** (2025): 500 tasks from 34 Rust repositories.

Full references and the mapping from each finding to an evaluator level live in
[`docs/scientific_scope.md`](docs/scientific_scope.md).

## Submission posture

This repository targets the NeurIPS 2026 Evaluation & Datasets track. The
submission checklist is maintained at
[`docs/submission_checklist.md`](docs/submission_checklist.md). Two release
modes are planned:

1. **Code-only audit submission.** Safest; audits only existing public datasets.
2. **Code + new proof-carrying subset.** Triggers dataset hosting and
   Croissant metadata requirements (see
   [`datasets/derived/proof_subset`](datasets/derived/proof_subset)).

## License

Licensed under either Apache-2.0 or MIT at your option. See
[`LICENSE-APACHE`](LICENSE-APACHE) and [`LICENSE-MIT`](LICENSE-MIT).
