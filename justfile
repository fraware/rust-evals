# eval-ladder command runner
#
# Install `just` from https://github.com/casey/just and run `just --list`.

set shell := ["bash", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# Release CLI path: batch recipes invoke this directly so long runs skip `cargo run`
# startup on every `just` invocation. Run `just eval-ladder-cli-release` once after
# toolchain changes (recipes that run `evaluate batch` depend on it).
eval-ladder-bin := if env_var_or_default('OS', '') == 'Windows_NT' {
  'target/release/eval-ladder.exe'
} else {
  './target/release/eval-ladder'
}

default:
    @just --list

# ---------------------------------------------------------------------------
# Rust
# ---------------------------------------------------------------------------

build:
    cargo build --workspace --all-targets

release:
    cargo build --workspace --release

check:
    cargo check --workspace --all-targets

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-targets

# ---------------------------------------------------------------------------
# Schemas
# ---------------------------------------------------------------------------

validate-schemas:
    cargo run -q --bin eval-ladder -- schema validate

# ---------------------------------------------------------------------------
# Python
# ---------------------------------------------------------------------------

py-install:
    python -m pip install -e ".[dev]"

py-lint:
    ruff check packages/python
    mypy

py-test:
    pytest

# ---------------------------------------------------------------------------
# Lean
# ---------------------------------------------------------------------------

lean-build:
    cd packages/lean/EvalLadder && lake build

# ---------------------------------------------------------------------------
# CI tiers (see docs/operational_runbook.md)
# ---------------------------------------------------------------------------

ci-tier1: fmt-check clippy test validate-schemas

ci-tier2: ci-tier1 py-lint py-test

# ci-tier3 is invoked only from .github/workflows/ci-tier3-heavy.yml
# because it runs container-backed fixtures.

# ---------------------------------------------------------------------------
# Release hygiene
# ---------------------------------------------------------------------------

deny:
    cargo deny check

audit:
    cargo audit

# Build the batch driver once (incremental rebuilds are fast when already up to date).
eval-ladder-cli-release:
    cargo build -q -p eval-ladder-cli --release

# ---------------------------------------------------------------------------
# Long-batch wall-clock helpers (see docs/operational_runbook.md § Milestone H)
# ---------------------------------------------------------------------------
#
# Batch recipes depend on `eval-ladder-cli-release` and invoke `{{eval-ladder-bin}}`
# (not `cargo run`). Pass panel/out/cache as paths relative to the repo root.
#
# Example (PowerShell):
#   just verified-batch-optimized-prewarmed runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl runs/released/agent_panel_v3_r1/results_opt
#   just live-batch-optimized-prewarmed runs/released/live_panel_v1/results_opt
#
# Prefer *-prewarmed recipes for long runs: image pulls happen up front so batch
# wall-clock is dominated by harness work, not first-touch docker pull.
# `verified-batch-optimized-prewarmed` is the recommended default for Verified panels.

verified-batch-optimized panel out jobs='2' cache='runs/released/.eval_ladder_cargo_cache': eval-ladder-cli-release
    {{eval-ladder-bin}} evaluate batch --input {{panel}} --config configs/evaluator/verified.toml --levels L0,L1,L3 --policy configs/policy/default_policy.toml --out {{out}} --timeout-secs 3600 --short-timeout-secs 900 --adaptive-timeouts --resume --jobs {{jobs}} --l1-strategy smart_rust_reuse --rust-target-cache-root {{cache}} --seed-tag verified-batch-opt --deterministic-clock

verified-batch-optimized-prewarmed panel out jobs='2' cache='runs/released/.eval_ladder_cargo_cache' prewarm_parallel='4':
    python ci/scripts/prewarm_panel_images.py --panel {{panel}} --parallel {{prewarm_parallel}}
    just verified-batch-optimized {{panel}} {{out}} {{jobs}} {{cache}}

live-batch-optimized out jobs='2': eval-ladder-cli-release
    {{eval-ladder-bin}} evaluate batch --levels L0,L1 --input runs/released/live_panel_v1/panel.jsonl --config configs/evaluator/default.toml --out {{out}} --timeout-secs 5400 --short-timeout-secs 900 --adaptive-timeouts --resume --jobs {{jobs}} --seed-tag live-panel-opt --deterministic-clock

live-batch-optimized-prewarmed out jobs='2' prewarm_parallel='4':
    python ci/scripts/prewarm_panel_images.py --panel runs/released/live_panel_v1/panel.jsonl --parallel {{prewarm_parallel}}
    just live-batch-optimized {{out}} {{jobs}}

rust-proof-batch-fast out: eval-ladder-cli-release
    {{eval-ladder-bin}} evaluate batch --input runs/released/rust_proof_subset_v1/panel.jsonl --config configs/evaluator/rust.toml --track fast --policy configs/policy/rust_proof_subset.toml --obligations datasets/derived/proof_subset/manifest.jsonl --lean-root packages/lean/EvalLadder --out {{out}} --resume --jobs 2 --adaptive-timeouts --short-timeout-secs 180 --rust-target-cache-root runs/released/rust_proof_subset_v1/.cargo_target_cache --timeout-secs 14400 --deterministic-clock

rust-proof-batch-fast-prewarmed out prewarm_parallel='4':
    python ci/scripts/prewarm_panel_images.py --panel runs/released/rust_proof_subset_v1/panel.jsonl --parallel {{prewarm_parallel}}
    just rust-proof-batch-fast {{out}}

rust-proof-batch-seal out jobs='2' cache='runs/released/rust_proof_subset_v1/.cargo_target_cache': eval-ladder-cli-release
    {{eval-ladder-bin}} evaluate batch --input runs/released/rust_proof_subset_v1/panel.jsonl --config configs/evaluator/rust.toml --track heavy --levels L0,L1,L3,L4 --policy configs/policy/rust_proof_subset.toml --obligations datasets/derived/proof_subset/manifest.jsonl --lean-root packages/lean/EvalLadder --out {{out}} --resume --jobs {{jobs}} --adaptive-timeouts --short-timeout-secs 180 --rust-target-cache-root {{cache}} --timeout-secs 14400 --deterministic-clock

rust-proof-batch-seal-prewarmed out jobs='2' cache='runs/released/rust_proof_subset_v1/.cargo_target_cache' prewarm_parallel='4':
    python ci/scripts/prewarm_panel_images.py --panel runs/released/rust_proof_subset_v1/panel.jsonl --parallel {{prewarm_parallel}}
    just rust-proof-batch-seal {{out}} {{jobs}} {{cache}}

# `prewarm-panel` is best-effort (matches script default). Use `prewarm-panel-strict` for CI gates.
prewarm-panel panel parallel='4':
    python ci/scripts/prewarm_panel_images.py --panel {{panel}} --parallel {{parallel}}

prewarm-panel-strict panel parallel='4':
    python ci/scripts/prewarm_panel_images.py --panel {{panel}} --parallel {{parallel}} --strict-pulls
