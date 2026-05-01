# Evaluator Card: Live v2 static vs live

## surface_id

`live_v2_static_vs_live`

## purpose

Diagnostic static-anchor vs live provenance comparison on a small overlapping
task panel for three public agent sources.

## scientific_question

This is a **diagnostic provenance-sensitivity panel**: under fixed candidates,
how much do static-anchor pass rates diverge from observed live outcomes on the
same evaluator stack (not a general live robustness estimator)?

## applicability_domain

Sealed bundles under `runs/released/live_panel_v2/results_opt/`; agents
**gru**, **honeycomb**, **sweagent**; tasks from overlapping verified-anchor and
live manifests (see the Live v2 run README).

## benchmark_sources

SWE-Bench Verified (static anchor) and SWE-bench-Live task manifests checked in
under `benchmarks/verified/manifests` and `benchmarks/live/manifests`.

## candidate_sources

Frozen public candidate patches from the released Live v2 panel input.

## selection_rule

Predeclared Live v2 panel construction documented under
`runs/released/live_panel_v2/`.

## exclusion_rule

Invalid bundles are reported in summaries; the frozen manifest defines the
panel, not post-hoc outcome-based shrinking.

## denominator_rule

Static and live numerators and denominators are reported separately per agent in
paper exports (`static_vs_live.csv`, `live_panel_summary_with_ci.csv`).

## invalid_row_rule

Harness errors and invalid bundles use stable `primary_reason` strings; run
`eval-ladder verify run-dir --run-dir runs/released/live_panel_v2/results_opt`.

## levels_used

L0, L1.

## validator_families

Official adapter scoring (L0); deterministic trusted rerun (L1).

## known_false_positive_risks

OCI drift, registry drift, or harvester issues can fail otherwise valid patches.

## known_false_negative_risks

Passes do not prove full semantic alignment with issue intent beyond benchmark
tests at L1.

## gold_patch_validation

Not part of this surface (gold replay is an L2 harness protocol).

## human_review

Optional; headline statistics are machine-exported.

## reproduction_command

`just reproduce-paper-tables` then:

`python ci/scripts/check_evidence_quality.py live --paper-export-dir paper/exports/live_panel_v2_postbatch`

## source_paths

- `runs/released/live_panel_v2/results_opt/`
- `paper/exports/live_panel_v2_postbatch/`

## claim_status

`central` (diagnostic provenance-sensitivity panel; not a live performance estimator).
