# Agent panel v3 (Verified flagship)

Panel v3 is built by ``packages/python/scripts/build_agent_panel_v3.py`` and
materializes multi-agent submissions on Verified tasks. The headline batch
artifacts for gate checks live under ``results_verified_v4/``.

## Commands

Preflight on the written summary:

```bash
python ci/scripts/diagnose_batch_summary.py \
  --summary runs/released/agent_panel_v3/results_verified_v4/batch_summary.json
```

Official pytest paths vs materialized workspaces (run after ``build_agent_panel_v3``):

```bash
python ci/scripts/preflight_verified_selectors.py \
  --panel runs/released/agent_panel_v3/panel.jsonl \
  --strict \
  --min-tasks 8
```

Harness-error clustering (stderr signatures):

```bash
python ci/scripts/triage_l1_harness_errors.py \
  --run-dir runs/released/agent_panel_v3/results_verified_v4
```

Verified quality gate (NeurIPS tranche):

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_v3/results_verified_v4 \
  --min-candidates 30 \
  --max-l1-harness-error-rate 0.10 \
  --min-distinct-agents 2 \
  --min-nonzero-agents 2 \
  --max-l3-single-reason-share 0.80
```

## Interpreting a high ``L1_HARNESS_ERROR`` rate

The v4 batch mixes Python ecosystems; dominant stderr families typically map to:

- ``pytest_selector_not_found`` / ``unittest_missing_method``: test IDs in the
  manifest no longer match the checked-out base commit, or unittest discovery
  skew versus the SWE-bench harness.
- ``missing_pytest``: image or conda env lacks pytest for that task family.
- ``matplotlib_native_extension_unbuilt`` / ``sklearn_native_build_missing``:
  native extensions not built inside the evaluation workspace.
- ``astropy_extension_or_selector``: Astropy built from source without compiled
  extensions, or selector drift.

Remediation is benchmark and image alignment (manifest ``FAIL_TO_PASS`` / install
steps, Docker image parity with SWE-bench), not evaluator randomness. After
fixes, rerun ``evaluate batch`` with a fresh ``--out`` or clean incomplete
bundle directories before relying on ``--resume``.
