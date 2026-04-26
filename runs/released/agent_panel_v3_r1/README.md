# agent_panel_v3_r1 (remediation panel)

Deterministic multi-agent Verified panel built with
`packages/python/scripts/build_agent_panel_v3.py` into this directory
(`--max-tasks 20`, `--min-agents-with-submission 2`).

## Panels

| File | Description |
|------|-------------|
| `panel.jsonl` | Full 60-entry panel (20 tasks x 3 agents). |
| `panel_preflight_clean.jsonl` | Drops tasks with strict preflight file errors (51 entries). |
| `panel_resolved_only.jsonl` | Subset where each row's `task_id` is in that agent's SWE-bench `resolved` list (46 entries). Produced with `ci/scripts/filter_panel_upstream_resolved.py`. |

## Sealed batch summaries

| File | Description |
|------|-------------|
| `evidence/batch_summary_prefclean_v1.json` | Copy from `results_verified_prefclean/` (51 ok, 0 invalid). |
| `evidence/batch_summary_results_opt_v1.json` | Copy from `results_opt/` after the optimized batch (`verified-batch-optimized` recipe). |

## Quality gates (Verified)

**Strict (headline NeurIPS thresholds):**

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_v3_r1/results_opt \
  --min-candidates 30 --max-l1-harness-error-rate 0.10 \
  --min-distinct-agents 2 --min-nonzero-agents 2 \
  --max-l3-single-reason-share 0.80
```

**Release profile (Mode 1 repository closure — see empirical status):**

```bash
python ci/scripts/check_evidence_quality.py --gate-profile release verified \
  --run-dir runs/released/agent_panel_v3_r1/results_opt
```

**Canonical run-dir for exports:** `results_opt/` (bundle integrity:
`eval-ladder verify run-dir --run-dir runs/released/agent_panel_v3_r1/results_opt`).

**Harness triage:**

```bash
python ci/scripts/triage_l1_harness_errors.py \
  --run-dir runs/released/agent_panel_v3_r1/results_opt
```

## Requirements

- Docker Desktop (Linux engine) running for `eval-ladder evaluate batch`.
- Network access for initial panel build (git fetch + S3 patches) and for
  `filter_panel_upstream_resolved.py` (GitHub raw `experiments` JSON).
