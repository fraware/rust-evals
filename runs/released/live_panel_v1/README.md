# live_panel_v1

Released **static vs live** comparison panel: the same three agent families
over **SWE-bench-Live** tasks and **SWE-Bench Verified** anchor tasks, with
frozen patches, candidates, and materialized workspace templates.

## Panel design

Built by `packages/python/scripts/build_live_panel_v1.py` (see module docstring
for patch provenance rules).

- **Agents:** `gru`, `honeycomb`, `sweagent` (model IDs in `provenance.json`).
- **Live arm:** 8 SWE-bench-Live tasks (real moving-target surface).
- **Static anchors:** 5 SWE-Bench Verified tasks; two anchors
  (`astropy__astropy-12907`, `pydata__xarray-2905`) are chosen so the static
  arm is not trivially all failures at L0/L1.
- **Entries:** 39 rows in `panel.jsonl` (13 unique tasks times 3 agents).

Patch sourcing differs by arm and agent: gru/honeycomb use dataset gold
`patch` everywhere; sweagent uses `test_patch` on Live tasks and `patch` on
static anchors so live vs static deltas are measurable in end-to-end runs.

## Observed results (frozen bundle)

- `results/batch_summary.json`: `total_entries=39`, `ok_entries=39`,
  `invalid_entries=0`, levels `L0,L1` (no L2/L3/L4 in this release track).
- `results/verify_report.json`: produced with `verify run-dir` (all bundles
  OK in the frozen run).

`analyze static-vs-live` on this run directory yields paired static vs live pass
rates per agent; cumulative-mode paper export tables are under
`paper/exports/live_panel_v1/` (for example `static_vs_live.csv`).

## Reproduction

Prerequisites: Docker, ingested manifests under `benchmarks/live/manifests/` and
`benchmarks/verified/manifests/`, and images reachable for every task in the
panel.

Workload deduplication keys include the **candidate JSON bytes** as well as the
task and patch hash, so three agents that share an identical gold patch on the
same Verified task still get **distinct** sealed bundles (each keeps its own
`candidate_resolution.json`). Re-run batches produced before that fix if
`analyze paper-export` ever shows fewer than three `agent_id` values in
`static_vs_live.json`.

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --levels L0,L1 `
  --input runs/released/live_panel_v1/panel.jsonl `
  --config configs/evaluator/default.toml `
  --out runs/released/live_panel_v1/results `
  --timeout-secs 5400 `
  --seed-tag live-panel-v1 `
  --deterministic-clock

cargo run -p eval-ladder-cli -- verify run-dir `
  --run-dir runs/released/live_panel_v1/results `
  --out runs/released/live_panel_v1/results/verify_report.json

cargo run -p eval-ladder-cli -- analyze static-vs-live `
  --run-dir runs/released/live_panel_v1/results

cargo run -p eval-ladder-cli -- analyze paper-export `
  --run-dir runs/released/live_panel_v1/results `
  --out-dir paper/exports/live_panel_v1
```

To rebuild only the panel artifacts (JSONL, candidates, patches, workspaces,
provenance) without executing Docker:

```powershell
python packages/python/scripts/build_live_panel_v1.py
```
