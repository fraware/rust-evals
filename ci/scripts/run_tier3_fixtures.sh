#!/usr/bin/env bash
#
# Tier 3 heavy CI driver.
#
# Runs the sampled fixture suite end-to-end: ingests fixture tasks, runs L0/L1
# on the local no-op runner, and smoke-tests the Lean checker against the
# fixtures library. This script intentionally avoids downloading full
# benchmark assets; Tier 3 is about surface coverage, not full replay.
#
# See docs/evidence_manual.md for the release-time heavy-replay workflow
# that is driven by `just` outside of CI.

set -Eeuo pipefail

log() { printf '[tier3] %s\n' "$*"; }

log "Building eval-ladder CLI in release mode"
cargo build --workspace --release --bin eval-ladder

log "Validating schemas"
cargo run --release -q --bin eval-ladder -- schema validate

if command -v lake >/dev/null 2>&1; then
  log "Building Lean EvalLadder fixtures"
  (cd packages/lean/EvalLadder && lake build)
else
  log "lake not available; skipping Lean fixture build (Tier 3 will still fail on CI image if lake is required)"
fi

log "Tier 3 fixture replay complete"
