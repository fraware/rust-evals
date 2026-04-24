# Evidence Tranche Execution Plan

This plan turns the remaining NeurIPS engineering work into objective,
automated acceptance gates.

## Priority 1 - Verified flagship panel (non-degenerate)

Target outcome:

- at least 30 evaluated candidates,
- at least two agents with distinct L0/L1 pass counts,
- at least two agents with non-zero L0/L1 pass totals,
- L1 harness errors below 10% (target below 5%),
- L3 failures not dominated by one avoidable packaging artifact.

Run quality gate:

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_v3/results_verified_v4 \
  --min-candidates 30 \
  --max-l1-harness-error-rate 0.10 \
  --min-distinct-agents 2 \
  --min-nonzero-agents 2 \
  --max-l3-single-reason-share 0.80
```

Preflight diagnostics before full reruns:

```bash
python ci/scripts/diagnose_batch_summary.py \
  --summary runs/released/agent_panel_v2/results/batch_summary.json
```

When ``diagnose_batch_summary`` flags a high ``L1_HARNESS_ERROR`` rate, cluster
``stderr.log`` signatures for the affected run directory:

```bash
python ci/scripts/triage_l1_harness_errors.py \
  --run-dir runs/released/agent_panel_v3/results_verified_v4
```

Materialize a stronger panel candidate:

```bash
python packages/python/scripts/build_agent_panel_v3.py \
  --out runs/released/agent_panel_v3 \
  --max-tasks 12 \
  --min-agents-with-submission 2
```

The script fetches patches from S3, pins one checkout per task under
``workspaces/<task_id>/`` (from each manifest's ``repo_name`` and
``base_commit``), and writes ``panel.jsonl``. Network access is required for
the git fetches and patch downloads.

Evaluate the panel (``L3`` requires ``--policy``):

```bash
cargo run -p eval-ladder-cli -- evaluate batch \
  --input runs/released/agent_panel_v3/panel.jsonl \
  --config configs/evaluator/verified.toml \
  --levels L0,L1,L3 \
  --policy configs/policy/default_policy.toml \
  --out runs/released/agent_panel_v3/results_verified_v4 \
  --timeout-secs 3600 \
  --short-timeout-secs 900 \
  --adaptive-timeouts \
  --resume \
  --jobs 1 \
  --l1-strategy smart_rust_reuse \
  --rust-target-cache-root runs/released/agent_panel_v3/results_verified_v4/.cargo_target_cache \
  --dedupe-workloads \
  --seed-tag agent-panel-v3 \
  --deterministic-clock
```

If a batch is interrupted mid-entry, bundle directories may be left non-empty
and ``--resume`` can mark later rows invalid. Prefer a fresh ``--out`` path
(or remove incomplete bundle dirs) before resuming.

## Priority 2 - Live panel (comparative, not tie-only)

Target outcome:

- at least one non-tied ranking across agents at L0 or L1,
- negative static-vs-live delta remains,
- rank-stability contains at least one informative non-zero tau row.

Run quality gate:

```bash
python ci/scripts/check_evidence_quality.py live \
  --paper-export-dir paper/exports/live_panel_v1
```

The live gate expects at least one level where agents are not tied on
``live_pass_rate``, plus a non-zero Kendall tau row in ``rank_stability.json``.
If every agent ties on the evaluated slice (common when harness failures
dominate), regenerate the comparative panel or widen the live stratum until
``live_pass_rate_unique_counts_by_level`` in the gate JSON shows spread.

## Priority 3 - L2 expansion slice

Target outcome:

- at least 10 candidates passing from L1 into L2,
- at least 3 L2 failures,
- at least 2 distinct L2 failure families.

Run quality gate:

```bash
python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_v2/results \
  --min-l1-passed-from 10 \
  --min-l2-failures 3 \
  --min-l2-reason-families 2
```

Use a run directory whose batch actually contains enough L1 passes and L2
attempts; small exploratory slices will not meet the defaults above until you
expand the L2 panel.

## Priority 4 - Rust proof-subset empirical usefulness

Target outcome:

- 8/8 ok entries and 0 invalid,
- at least 2 `L3 pass / L4 fail`,
- at least 1 all-level pass.

Run quality gate:

Target gate once a full ladder batch with the right verdict mix exists:

```bash
# Replace RUN_DIR with the directory that contains your sealed L0,L1,L3,L4 batch_summary.json.
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir RUN_DIR \
  --expected-entries 8 \
  --min-l3-pass-l4-fail 2 \
  --min-all-level-pass 1
```

Interim structural check (8 ok entries, 0 invalid; skips semantic L3/L4
contrasts by setting minima to zero) after ``results_fast`` or any complete
8-entry batch:

```bash
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_fast \
  --expected-entries 8 \
  --min-l3-pass-l4-fail 0 \
  --min-all-level-pass 0
```

If ``results_v3`` shows ``invalid`` rows from stale bundle directories, rerun
with a clean ``--out`` (or remove the listed bundle dirs) so the summary can
reach 8/8 ok.

## Priority 5 - Release closure

Required commands on the tagged release commit:

```bash
cargo deny check
cargo audit
```

Tier 3 and release-tag workflow must pass on the same tag.

## Notes

- Every gate prints JSON with pass/fail and detailed metrics.
- Exit code is non-zero on gate failure, making this suitable for CI or
  pre-submission checklists.
- Verified ``results_verified_v4`` has been observed to fail the harness-rate
  and distinct-agent gates until L1 stderr clusters above are driven down; see
  ``runs/released/agent_panel_v3/README.md`` for triage commands.
