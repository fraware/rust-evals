# eval-ladder command runner
#
# Install `just` from https://github.com/casey/just and run `just --list`.

set shell := ["bash", "-cu"]
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

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
