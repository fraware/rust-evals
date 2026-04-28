# Evaluator Card: live_panel_v2

## Purpose

Compare **static-anchor** SWE-Bench Verified reruns against **live** manifests on
the same agents and overlapping tasks to measure **static-to-live degradation**
on a small diagnostic panel.

## Applicability domain

Sealed bundle directory `runs/released/live_panel_v2/results_opt`. Agents:
**gru**, **honeycomb**, **sweagent**. Tasks combine verified-anchor and live
surfaces listed in `benchmarks/live/manifests` and `benchmarks/verified/manifests`.

## Native benchmark assumptions

L0 reflects the benchmark’s official validation outcome as replayed through the
adapter; L1 reflects deterministic harness rerun outcomes.

## Replay environment

OCI task images per manifest; bundles materialized under `results_opt/` with
canonical JSON summaries (`batch_summary.json`).

## Strengthened validators

None at headline L1 comparison; strengthening is out of scope for this surface.

## Policy assumptions

Standard verified/live adapter policies unless a run README specifies overrides.

## Semantic obligations

None at L1 for this slice.

## Denominators and invalid handling

Pass rates use **evaluated** rows per agent stratum; invalid bundles are counted
per summary rules in the sealed JSON.

## Known false-positive risks

Harvester or Docker drift can fail good patches (false L1 fail).

## Known false-negative risks

Passes can mask silent divergence from intended issue semantics because L1 does
not enforce strengthened obligations.

## Reproduction command

`just reproduce-paper-tables` (requires release `eval-ladder`).

## Evidence bundle paths

- Input summary: `runs/released/live_panel_v2/results_opt/batch_summary.json`
- Paper exports: `paper/exports/live_panel_v2_postbatch/`
