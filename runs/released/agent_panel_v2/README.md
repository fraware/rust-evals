# agent_panel_v2

Scaled **SWE-Bench Verified** agent panel: the same three public submission
families as `agent_panel_v1`, over **10** pinned tasks (the v1 slice plus five
additional Verified instances with ingested manifests).

## Materialisation

From the repository root (requires network for S3 and GitHub raw JSON):

```powershell
python packages/python/scripts/build_agent_panel_v2.py
```

Optional custom output root:

```powershell
python packages/python/scripts/build_agent_panel_v2.py --out runs/released/agent_panel_v2
```

The script writes `panel.jsonl`, per-agent `candidates/` and `patches/`, and
`provenance.json`. Entries whose patch is missing on S3 are recorded in
provenance as `omitted_no_patch_on_s3` and omitted from `panel.jsonl`.

## Evaluation

Same pattern as v1: Docker-backed `evaluate batch` with
`configs/evaluator/verified.toml`, then `verify run-dir` and analysis CLIs
(`score-descent`, `rank-stability`, `conditional-false-success`, `taxonomy`,
`paper-export`).

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --input runs/released/agent_panel_v2/panel.jsonl `
  --config configs/evaluator/verified.toml `
  --levels L0,L1,L3 `
  --out runs/released/agent_panel_v2/results `
  --timeout-secs 5400 `
  --seed-tag agent-panel-v2 `
  --deterministic-clock
```

Sealed `results/` trees are intentionally large; reproduce locally rather than
expecting every clone to ship a completed batch.
