# runs/released/agent_panel_v1/

First public agent panel driven by the eval-ladder Rust CLI. The panel
consists of 3 agents x 5 SWE-bench Verified tasks = **15 entries**.

## Contents

```
panel.jsonl           15-line JSONL consumed by `eval-ladder evaluate batch`
provenance.json       Deterministic record of how each entry was built
candidates/<agent>/   CandidateResolution JSON per (agent, task)
patches/<agent>/      Raw upstream patch.diff bytes per (agent, task)
workspaces/           Reserved for run-time-provisioned task workspaces
                      (empty in the committed tree; see "Executing" below)
```

## Source provenance

Every patch in `patches/` is a byte-for-byte download from the public
[`swe-bench-submissions` S3 bucket](https://swe-bench-submissions.s3.amazonaws.com/),
`verified/<agent>/logs/<instance>/patch.diff`. No patches were
regenerated, decoded, or otherwise transformed.

`provenance.json` records, for every (agent, task) pair:

- Upstream agent slug (the `YYYYMMDD_*` directory name in
  [SWE-bench/experiments](https://github.com/SWE-bench/experiments/tree/main/evaluation/verified))
- Patch SHA-256 (raw bytes)
- `upstream_resolved`: whether the agent's public `results.json` lists
  the task as resolved (this is the benchmark author's own verdict; we
  do **not** claim it as our own until the full ladder runs)

## Candidate IDs

`candidate_id` is **deterministic**: `UUIDv5` of
`"{agent_id}|{task_id}|{patch_sha256}"` under the namespace
`3f3e0b4e-4f05-5b9d-9e4d-0a1c2a8b1e11`. Rerunning
`packages/python/scripts/build_agent_panel.py` against unchanged
upstream patches produces byte-identical candidate JSON files.

## Agents

| slug                                     | agent_id   | model_id                         | public resolved count |
|------------------------------------------|------------|----------------------------------|------------------------|
| 20240620_sweagent_claude3.5sonnet        | sweagent   | claude-3-5-sonnet-20241022       | 168 / 500              |
| 20240824_gru                             | gru        | gru-2024-08-24                   | 226 / 500              |
| 20240820_honeycomb                       | honeycomb  | honeycomb-2024-08-20             | 203 / 500              |

The three agents were chosen because they (a) are published on the
Verified leaderboard, (b) expose per-instance `patch.diff` files in the
public submissions bucket, and (c) produce differential signal across
the selected task set.

## Task set

5 tasks chosen to exercise the full resolve-profile space across the
three agents:

| task_id                     | resolve profile (sweagent, gru, honeycomb) |
|-----------------------------|--------------------------------------------|
| astropy__astropy-14309      | (1, 1, 1) all resolve                      |
| astropy__astropy-14096      | (0, 0, 1) only honeycomb resolves          |
| django__django-10554        | (0, 0, 0) none resolve                     |
| django__django-11066        | (1, 1, 0) honeycomb fails                  |
| django__django-11211        | (1, 0, 1) gru fails                        |

This spread intentionally stresses the metrics the paper cares about
(per-agent differential signal, contribution to rank-stability, and
zero-variance rows for the taxonomy analysis).

## Executing

**L0/L1 on SWE-bench Verified require Docker Desktop.** The official
test entrypoint for every Verified task (recorded on each manifest as
`python -m swebench.harness.run_evaluation --instance_ids ...`) executes
inside the per-task `swebench/sweb.eval.x86_64.<instance_id>:latest`
image. Without Docker the pipeline flags every entry as invalid and the
bundle-sealing step is skipped.

To execute once Docker Desktop is available:

```bash
# Ensure Docker images are warm (pulls ~1.5-5 GB per task the first time)
for t in astropy__astropy-14309 astropy__astropy-14096 \
         django__django-10554 django__django-11066 django__django-11211; do
  docker pull "swebench/sweb.eval.x86_64.${t}:latest"
done

# Batch run
eval-ladder evaluate batch \
    --input runs/released/agent_panel_v1/panel.jsonl \
    --config configs/evaluator/verified.toml \
    --levels L0,L1,L2,L3 \
    --strengthening-spec configs/strengthening/default.json \
    --policy configs/policy/default_policy.toml \
    --out runs/released/agent_panel_v1/results/
```

L4 is available but excluded by default; the proof subset for this
panel is empty (`datasets/derived/proof_subset/manifest.jsonl`), so
every L4 level would report `NotApplicable`. The L4 seam for the Rust
pilot is demonstrated in the separate `rust_pilot_v1` run directory.

## Why no prebaked results directory?

The repository intentionally commits the **panel descriptor** but not
pre-computed verdicts. The descriptor is reproducible from public
sources; the verdicts are only meaningful if produced by a Docker-backed
reviewer on their own hardware and are therefore expected to be
regenerated, not copied from this commit.
