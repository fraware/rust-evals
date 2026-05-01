# Evaluator Card: Verified feasibility frontier

## surface_id

`verified_feasibility_frontier`

## purpose

Offline inventory bound showing whether a strict three-agent Verified comparison
can meet a predeclared candidate threshold using only current in-repo summaries.

## scientific_question

Is strict multi-agent comparison currently blocked by **inventory** rather than
evaluator incapacity?

## applicability_domain

Machine-generated `paper/exports/strict_feasibility_report.json`.

## benchmark_sources

Verified-style tasks represented in summarized panels feeding the analyzer.

## candidate_sources

Frozen public-agent candidate pass observations from existing summaries.

## selection_rule

Implemented in `ci/scripts/analyze_strict_feasibility.py`.

## exclusion_rule

Does not substitute synthetic L4 counterexamples or unpublished candidate stores.

## denominator_rule

See `verified` and related objects in `strict_feasibility_report.json`
(`max_rows_if_single_candidate_per_task`, shared task counts).

## invalid_row_rule

Analyzer consumes published summaries; invalid handling stays in upstream batches.

## levels_used

Inventory references L1-pass stability signals; this JSON is not a ladder replay export.

## validator_families

None beyond the analyzer’s counting logic.

## known_false_positive_risks

Out-of-date summaries could misstate the bound until regenerated.

## known_false_negative_risks

Additional compatible candidates outside summarized inventories would loosen the bound.

## gold_patch_validation

Not applicable.

## human_review

Not used.

## reproduction_command

`python ci/scripts/analyze_strict_feasibility.py`

## source_paths

- `paper/exports/strict_feasibility_report.json`

## claim_status

`frontier` (evidence-gated reporting; not a headline empirical pillar).
