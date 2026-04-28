# Evaluator Card: l2_verified_flagship_v1

## Purpose

Stress **L1-passing** candidates with **strengthened L2 validators** along two
pre-declared arms (augmented tests vs regression channel) on a fixed 66-row
slice.

## Applicability domain

Merged results under `runs/released/l2_verified_flagship_v1/results/` (33 base
(task,agent) pairs expanded to two validator arms).

## Native benchmark assumptions

Same as Verified flagship L0/L1 replay before strengthening.

## Replay environment

Harness workspaces from `runs/released/agent_panel_verified_flagship_v1/` with
strengthening specs referenced from the flagship README.

## Strengthened validators

`tests_plus_regression` mode with checked-in specs  
(`strengthening_spec*.json`). Augmented pytest paths may include warnings-as-errors.
Regression arm includes **forced-fail** negative control (`regression_forced_fail`).

## Policy assumptions

Inherited from flagship verified configuration unless README states otherwise.

## Semantic obligations

None beyond strengthened mechanical checks declared in specs.

## Denominators and invalid handling

66 sealed rows total; failures classified as `L2_AUG_TESTS_FAIL` vs
`L2_REGRESSION_FAIL` per strengthening reports.

## Known false-positive risks

Cross-repo augmented selectors may fire on non-issue-aligned sandboxes (see
human review CSV).

## Known false-negative risks

Forced regression channel does not prove absence of semantic regressions by
itself.

## Reproduction command

`just reproduce-paper-tables`

## Evidence bundle paths

- `runs/released/l2_verified_flagship_v1/results/batch_summary.json`
- Gold validation:
  `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv`
- Human review:
  `paper/exports/l2_verified_flagship_v1/l2_failure_review.csv`
