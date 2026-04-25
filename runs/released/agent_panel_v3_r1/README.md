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

## Sealed batch summary

| File | Description |
|------|-------------|
| `evidence/batch_summary_prefclean_v1.json` | Full `batch_summary.json` from `results_verified_prefclean/` after a complete L0,L1,L3 run (51 ok, 0 invalid). |

## Quality gate (Verified)

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_v3_r1/results_verified_prefclean \
  --min-candidates 30 --max-l1-harness-error-rate 0.10 \
  --min-distinct-agents 2 --min-nonzero-agents 2 \
  --max-l3-single-reason-share 0.80
```

As of the sealed run, the gate still fails on **L1 harness error rate** and
**L3 dominance** triage buckets (`pytest_selector_not_found`, native extension
build gaps in container, etc.). See `docs/evidence_empirical_status.md`.

## Requirements

- Docker Desktop (Linux engine) running for `eval-ladder evaluate batch`.
- Network access for initial panel build (git fetch + S3 patches) and for
  `filter_panel_upstream_resolved.py` (GitHub raw `experiments` JSON).
