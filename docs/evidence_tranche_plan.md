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

Run quality gate (strict defaults — headline NeurIPS bar):

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_v3_r1/results_opt \
  --min-candidates 30 \
  --max-l1-harness-error-rate 0.10 \
  --min-distinct-agents 2 \
  --min-nonzero-agents 2 \
  --max-l3-single-reason-share 0.80
```

Repository closure (same paths, relaxed thresholds) uses
``python ci/scripts/check_evidence_quality.py --gate-profile release …``;
see ``docs/evidence_empirical_status.md``.

Preflight diagnostics before full reruns:

```bash
python ci/scripts/diagnose_batch_summary.py \
  --summary runs/released/agent_panel_v2/results/batch_summary.json
```

Add ``--fail-on-warnings`` so the process exits non-zero when thresholds are
breached (useful in scripted gates).

When ``diagnose_batch_summary`` flags a high ``L1_HARNESS_ERROR`` rate, cluster
``stderr.log`` signatures for the affected run directory:

```bash
python ci/scripts/triage_l1_harness_errors.py \
  --run-dir runs/released/agent_panel_v3_r1/results_opt
```

After materializing per-task ``workspace_template`` trees for a Verified panel,
confirm that manifest ``official_test_entrypoint`` paths exist on disk (catches
pytest ``not found`` / ``file not found`` before Docker):

```bash
python ci/scripts/preflight_verified_selectors.py \
  --panel runs/released/agent_panel_v3/panel.jsonl \
  --strict \
  --min-tasks 8
```

Structural audit of every checked-in Verified manifest (no workspaces required):

```bash
python ci/scripts/audit_verified_manifest_entrypoints.py \
  --strict \
  --expect-manifest-count 500
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

**Remediation track (2026-04-25):** a larger materialized panel and sealed
batch live under ``runs/released/agent_panel_v3_r1/`` (see that directory's
``README.md``). To reduce degenerate per-agent pass vectors, filter the panel
to rows where the task is upstream-resolved for that agent:

```bash
python ci/scripts/filter_panel_upstream_resolved.py \
  --in runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl \
  --out runs/released/agent_panel_v3_r1/panel_resolved_only.jsonl \
  --summary
```

Current pass/fail status for all four empirical gates is summarized in
``docs/evidence_empirical_status.md``.

For long Docker batches, use the wall-clock checklist and ``just`` recipes in
``docs/operational_runbook.md`` (Milestone H, “Wall-clock optimizations”). Prefer
``just …-prewarmed`` recipes (for example ``verified-batch-optimized-prewarmed``) so
image pulls run before ``evaluate batch``, or run
``python ci/scripts/prewarm_panel_images.py --panel …`` manually.

If a batch is interrupted mid-entry, bundle directories may be left non-empty
and ``--resume`` can mark later rows invalid. Prefer a fresh ``--out`` path
(or remove incomplete bundle dirs) before resuming.

## Priority 2 - Live panel (comparative, not tie-only)

Target outcome:

- at least one non-tied ranking across agents at L0 or L1,
- negative static-vs-live delta remains,
- rank-stability contains at least one informative non-zero tau row.

Run quality gate (strict):

```bash
python ci/scripts/check_evidence_quality.py live \
  --paper-export-dir paper/exports/live_panel_v1_postbatch
```

The strict live gate expects at least one level where agents are not tied on
``live_pass_rate``, plus a non-zero Kendall tau row in ``rank_stability.json``.
If every agent ties on the evaluated slice (common when patches are symmetric
across agents), either **regenerate the comparative panel** so agents diverge
on live-evaluated tasks, or use the documented **release** path
(``--gate-profile release`` or ``--symmetric-live-ok`` when every ``delta`` is
strictly negative) per ``docs/evidence_empirical_status.md``.

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
expand the L2 panel. For a **deduplicated merge** of multiple small summaries
(``batch_summary.json`` only), see ``ci/scripts/merge_l2_batch_summaries.py`` and
``runs/released/l2_verified_merged_v1/`` (release-profile gate only).

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

If a proof-subset ``--out`` directory shows ``invalid`` rows from stale bundle
directories, rerun with a clean ``--out`` (or remove the listed bundle dirs) so
the summary can reach 8/8 ok.

## Priority 5 - Release closure

Required commands on the tagged release commit:

```bash
cargo deny check
cargo audit
```

Tier 3 and release-tag workflow must pass on the same tag.

The ``release-tag`` workflow runs
``python ci/scripts/write_release_artifact_manifest.py --require-all-files``
so a sparse or partial tree cannot produce a passing manifest artifact; omit
``--require-all-files`` only when inspecting checkouts that intentionally lack
Lean or proof-subset paths.

## Notes

- ``check_evidence_quality`` supports ``--gate-profile release`` (global flag
  before the subcommand) to apply repository-closure thresholds documented in
  ``docs/evidence_empirical_status.md``. Default remains **strict** for science.
- Every gate prints JSON with pass/fail and detailed metrics.
- Exit code is non-zero on gate failure, making this suitable for CI or
  pre-submission checklists.
- Verified ``runs/released/agent_panel_v3_r1/results_opt`` (and the preflight-clean
  sibling) have been observed to fail the **strict** harness-rate and
  distinct-agent gates until L1 stderr clusters are driven down; see
  ``runs/released/agent_panel_v3_r1/README.md`` and ``runs/released/agent_panel_v3/README.md``
  for triage commands.
- On every push/PR, ``ci-tier1-fast`` runs
  ``python ci/scripts/run_evidence_tier1_checks.py`` (``compileall`` on
  ``ci/scripts``, structural ``rust-proof`` on tracked
  ``runs/released/rust_proof_subset_v1/results_fast``, ``preflight_verified_selectors.py --strict``
  on ``l0l1_pass_hunt_v1``, ``audit_verified_manifest_entrypoints.py --strict``
  over 500 manifests). ``ci-tier2-medium`` runs ``ruff check`` on
  ``packages/python`` and ``ci/scripts``, strict ``mypy`` on
  ``packages/python/benchmark_compat/src`` and ``ci/scripts`` (see root
  ``pyproject.toml``), and ``pytest``, including
  subprocess tests that assert ``check_evidence_quality`` exits **2** with
  ``ok: false`` on representative failure shapes (harness rate, degenerate
  agent vectors, live ties / tau / delta, thin L2, rust invalid rows and
  semantic minima).
- **Local** runs use the same command from the repository root (no Rust build).
  Add or reorder checks in ``ci/scripts/run_evidence_tier1_checks.py`` only;
  the workflow invokes that script so CI and local stay aligned.
- ``diagnose_batch_summary.py`` supports ``--fail-on-warnings`` (exit code 2)
  for strict automation when harness or L3 dominance thresholds are exceeded.
