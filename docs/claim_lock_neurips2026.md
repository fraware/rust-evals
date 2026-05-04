# Claim lock — NeurIPS 2026 E&D (Eval-Ladder)

This document constrains what the paper and public documentation may assert
relative to **frozen, machine-verifiable evidence** in this repository. It is
the normative claim lock for the submission cycle; pair it with
[`paper/exports/CLAIM_SOURCE_MAP.md`](../paper/exports/CLAIM_SOURCE_MAP.md) and
[`paper/paper_claim_sources.json`](../paper/paper_claim_sources.json) (enforced
by `ci/scripts/check_paper_claim_sources.py`).

## Tier A — Headline claims

Allowed in abstract, introduction, results, and conclusion:

- Eval-Ladder evaluates **fixed externally produced** candidate patches; it does
  not generate patches.
- The **candidate patch** is separated from the **evaluator** that scores it.
- The framework implements **L0–L4 evaluator levels**: L0 official benchmark
  scoring; L1 trusted deterministic rerun; L2 strengthened validation; L3
  policy-conformant execution; L4 semantic obligations on a curated proof
  subset.
- It emits **sealed evidence bundles** and **deterministic paper exports**.
- **Live v2** documents static-vs-live sensitivity in a **small diagnostic**
  panel.
- **L2 flagship** shows that trusted-rerun conclusions can **reverse** under
  predeclared evaluator transformations; augmented-test and regression
  stress-control arms must be interpreted **separately**.
- **Evaluator Cards** document assumptions, applicability domain, risks,
  denominators, and reproduction paths for each evaluator surface.
- **Verified** strict comparison and **Rust proof-subset** natural evidence are
  **frontier / boundary** results, not headline empirical pillars.

Levels are **independent** surfaces; verdicts do not overwrite one another;
failure reasons are stable strings (see `docs/evaluation_ladder.md`).

## Tier B — Central empirical surfaces

Only these two surfaces are **central** in the paper:

1. **Live v2** static-vs-live diagnostic (`paper/exports/live_panel_v2_postbatch/`,
   `runs/released/live_panel_v2/results_opt/`).
2. **L2 flagship** diagnostic (`paper/exports/l2_verified_flagship_v1/`,
   `runs/released/l2_verified_flagship_v1/results/`).

L3 and L4 remain **real evaluator surfaces** in the artifact but are **not**
central quantitative pillars of the current paper (see
`docs/evidence_empirical_status.md`).

## Tier C — Evidence-frontier surfaces

Allowed **only** in a clearly labeled **Evidence frontiers** section (or
equivalent):

- **Verified strict comparison:** inventory-bound; currently **below** the
  predeclared strict threshold (`paper/exports/strict_feasibility_report.json`).
- **Rust proof subset:** implemented and auditable; current natural sealed
  evidence does **not** yet meet publication-threshold semantic minima for
  L3-pass / L4-fail separation.

## Tier D — Prohibited claims

Do **not** claim:

- “Eval-Ladder proves patches semantically correct.”
- “Eval-Ladder estimates the true semantic failure rate of SWE-bench.”
- “All L2 failures are false successes.”
- “The regression stress-control arm measures natural product regressions.”
- “The Rust proof subset currently demonstrates natural L3/L4 separation.”
- “Verified comparison is fully powered.”
- “L3/L4 are central empirical results.”

Prefer:

**diagnostic**, **evaluator-conditioned**, **evidence-gated**, **frontier result**,
**negative-control / stress-control**, **implemented extension surface**.

## Release posture (Mode 1)

Submission follows **Mode 1: code-only audit submission** (see
`docs/submission_checklist.md`): auditing existing public benchmarks without
redistributing benchmark data; proof subset as task IDs plus obligations; no
Croissant requirement for this deadline.
