# Evaluator Card: verified_feasibility

## Purpose

Offline bound on whether **strict headline candidate volume** can be met from
existing sealed summaries **without new tasks or new candidates**.

## Applicability domain

Inventory derived from in-repo agent-panel summaries (see
`ci/scripts/analyze_strict_feasibility.py` sources).

## Native benchmark assumptions

Uses observed L1 pass counts from sealed logs, not fresh reruns.

## Replay environment

None — analytical compilation only.

## Strengthened validators

None.

## Policy assumptions

Uses thresholds embedded in `strict_feasibility_report.json` (`min_candidates`,
harness-error caps).

## Semantic obligations

None.

## Denominators and invalid handling

Counts task–agent pairs with pass evidence; failures are feasibility failures,
not per-patch verdicts.

## Known false-positive risks

Stale summaries could mis-count availability if upstream batches change.

## Known false-negative risks

Inventory may omit unreleased batches that could lift counts.

## Reproduction command

`python ci/scripts/analyze_strict_feasibility.py --out paper/exports/strict_feasibility_report.json`

## Evidence bundle paths

- Output: `paper/exports/strict_feasibility_report.json`
