# Manuscript guide — NeurIPS 2026 E&D (Eval-Ladder)

This note translates engineering claim discipline into paper structure. Pair with
`docs/CLAIM_LOCK_NEURIPS2026.md`, `paper/exports/CLAIM_SOURCE_MAP.md`, and
`paper/paper_claim_sources.json`.

## Title (preferred)

Eval-Ladder: Evaluator-Conditioned Measurement for Repository-Level Coding-Agent Benchmarks

Avoid framing the contribution primarily as “auditing evaluator sensitivity” in
the title; keep “evaluator-conditioned measurement” precise for E&D.

## Abstract (six-sentence skeleton)

1. Repository-level coding-agent benchmark scores are evaluator-conditioned measurements.
2. Eval-Ladder scores fixed candidate patches across L0–L4 evaluator levels.
3. Live v2 shows static-to-live provenance sensitivity in a small diagnostic panel.
4. L2 flagship shows L1-passing rows can reverse under strengthened validation, with augmented-test and stress-control arms interpreted separately.
5. Verified and Rust proof-subset results are reported as evidence frontiers rather than inflated headline claims.
6. The contribution is an executable, evidence-gated reporting protocol for coding-agent evaluation.

Keep numeric density low in the abstract. Optional only if clearly qualified:
static-vs-live deltas for the small panel, the 66-row L2 cohort with arm separation,
Verified `21 < 30` as an inventory-bound frontier.

## Results section order (recommended)

1. Live v2 diagnostic (denominators, Wilson intervals, leave-one-out file under `paper/exports/live_panel_v2_postbatch/`).
2. L2 augmented-test vs regression stress-control diagnostics (use `l2_flagship_arm_breakdown.csv`; never merge arms into a single “false success” headline).
3. Gold-patch legitimacy (gold-mechanical profile) and human review sample caveat.
4. Verified feasibility frontier (`strict_feasibility_report.json`).
5. Rust proof-subset frontier (natural sealed cohort; no synthetic L4 headline stats).
6. Evaluator Cards (`paper/exports/evaluator_cards/`).

## Appendix candidates

Move here unless strictly needed in main text:

- Raw vs cumulative semantics for derived tables.
- Full failure-code taxonomy exports.
- Rank stability (only if panel supports a clean story).
- L3 implementation acceptance tests.
- L4 synthetic mechanism replays (explicitly non-headline).
- Old release-profile gate history and superseded v1 surfaces.

## Single-blind / artifact posture

NeurIPS E&D is single-blind per FAQ; artifact professionalism still matters. Code
and data must be final at the full-paper deadline. Decide explicitly whether any
anonymized archive is needed beyond the public repository.

## Source-of-truth rule

All quantitative statements must trace to generated exports listed in
`paper/paper_claim_sources.json` / `.yaml`. Do not hand-edit CSV/JSON counts in
`paper/exports/**`.
