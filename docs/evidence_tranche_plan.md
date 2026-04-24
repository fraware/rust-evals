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
  --run-dir runs/released/agent_panel_v3/results \
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
  --run-dir runs/released/agent_panel_v3/results
```

Materialize a stronger panel candidate:

```bash
python packages/python/scripts/build_agent_panel_v3.py \
  --out runs/released/agent_panel_v3 \
  --max-tasks 12 \
  --min-agents-with-submission 2
```

## Priority 2 - Live panel (comparative, not tie-only)

Target outcome:

- at least one non-tied ranking across agents at L0 or L1,
- negative static-vs-live delta remains,
- rank-stability contains at least one informative non-zero tau row.

Run quality gate:

```bash
python ci/scripts/check_evidence_quality.py live \
  --paper-export-dir paper/exports/live_panel_v2
```

## Priority 3 - L2 expansion slice

Target outcome:

- at least 10 candidates passing from L1 into L2,
- at least 3 L2 failures,
- at least 2 distinct L2 failure families.

Run quality gate:

```bash
python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_v3/results \
  --min-l1-passed-from 10 \
  --min-l2-failures 3 \
  --min-l2-reason-families 2
```

## Priority 4 - Rust proof-subset empirical usefulness

Target outcome:

- 8/8 ok entries and 0 invalid,
- at least 2 `L3 pass / L4 fail`,
- at least 1 all-level pass.

Run quality gate:

```bash
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir runs/released/rust_proof_subset_v2/results \
  --expected-entries 8 \
  --min-l3-pass-l4-fail 2 \
  --min-all-level-pass 1
```

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
