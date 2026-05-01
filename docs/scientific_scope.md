# Scientific scope

This document records the scientific posture of `eval-ladder`. It is the
reference for reviewers and should be cited by the paper. See also
[`README.md`](../README.md) for the full documentation map, the NeurIPS claim
lock in [`CLAIM_LOCK_NEURIPS2026.md`](CLAIM_LOCK_NEURIPS2026.md), and
[`evidence_empirical_status.md`](evidence_empirical_status.md) for machine-checked
gates.

## Research question

Given a fixed panel of externally generated candidate patches for a benchmark
task suite, how do **official and trusted** benchmark conclusions change when
the evaluator is replaced or strengthened along a predeclared ladder (L0–L4),
and how can those changes be reported as **evaluator-conditioned measurements**
with sealed, auditable evidence?

## Core paper claim

Coding-agent benchmark scores are **evaluator-conditioned measurements**.
Eval-Ladder makes evaluator transformations **executable and auditable** by
scoring fixed candidate patches across official scoring, trusted rerun,
strengthened validation, policy-conformant execution, and semantic-obligation
surfaces. In the current submission, **Live v2** and **L2 flagship** are the
**central diagnostic** surfaces; **Verified feasibility** and **Rust
proof-subset** results are **evidence-frontier** results (see claim lock).

Official or rerun benchmark conclusions **can change** under evaluator
transformations; the framework records **where** and **how** they change rather
than collapsing them into a single population rate claim.

## Scope for evaluator and dataset publication venues

The Evaluation & Datasets track explicitly welcomes:

- Audits of benchmark failure modes.
- Comparisons of evaluation designs.
- Refined evaluation protocols.
- Executable tools that improve how evaluative claims are constructed.

`eval-ladder` produces all four: a structured comparison across evaluator
surfaces, a refined protocol (L0–L4), and an executable tool (the `eval-ladder`
CLI and signed evidence bundles).

### Dataset posture (submission: Mode 1)

For the NeurIPS 2026 E&D submission this repository adopts **Mode 1: code-only
audit submission** (see `docs/submission_checklist.md`):

- Audit **only existing public datasets** at bootstrap (SWE-Bench Verified,
  SWE-bench-Live, Rust-SWE-bench release).
- **No new dataset artifact** for redistribution; **no Croissant** requirement
  on this path.
- The curated proof subset ships as **task identifiers plus obligations** over
  upstream public tasks, not as redistributed benchmark payloads.

Mode 2 (hosting a new proof subset as a Croissant dataset with datasheet and
selection-bias audit) remains a future, heavier path.

## Related work this repository is built to absorb and extend

The numbers below are the reported findings that define what the evaluator
must be able to reproduce or refine. Levels refer to the ladder documented
in `docs/evaluation_ladder.md`.

| Work                  | Scale                                                    | Findings motivating a ladder level |
|-----------------------|----------------------------------------------------------|------------------------------------|
| SWE-bench             | 2,294 tasks, 12 Python repositories.                     | Defines L0 (official scoring).     |
| SWE-Bench+            | Re-evaluates public leaderboards.                        | One system drops 12.47% → 3.97%;<br>motivates L1 (trusted rerun) and L2 (strengthened tests). |
| PatchDiff             | Patch-level behavioural comparison.                      | 7.8% of passing patches diverge from developer tests;<br>~6.2-point inflation; motivates L2's differential module. |
| UTBoost               | Test adequacy audit.                                     | Insufficient/erroneous tests affect 40.9% of Lite and<br>24.4% of Verified entries; motivates L2 augmented tests. |
| SWE-bench-Live        | 1,319 live tasks, 93 repositories, per-task Docker images. | Static-vs-live gap; motivates Live adapter and<br>static-vs-live analysis output. |
| Rust-SWE-bench        | 500 tasks, 34 Rust repositories.                         | Repository-wide Rust semantics;<br>motivates Rust adapter and Lean L4 slice. |

References are maintained in `paper/references.bib` once the paper tree is
seeded.

## What the ladder adds beyond prior audits

Prior audits focus on **better tests**. `eval-ladder` extends this in two
directions.

1. **Process validity (L3).** Many candidates that pass stronger tests still
   took an invalid path to get there: forbidden commands, out-of-scope file
   edits, undeclared network access, non-deterministic behaviour, or
   incomplete traces. L3 surfaces that process-level invalidity explicitly
   and attributes a separable score drop to it.
2. **Semantic obligation (L4).** For a curated subset of tasks we attach
   machine-checkable obligations (Lean). This is an **implemented extension
   surface** for obligation-aware scoring; it is **not**, in the current
   submission, a claim of full semantic verification or of a natural semantic
   failure rate for SWE-bench-scale populations.

## Required outputs for the paper

The analysis crate must deterministically produce:

- Pass rate by level, per benchmark and per agent.
- Conditional false-success rate `P(fail at L_{k+1} | pass at L_k)`.
- Rank correlation between leaderboards derived at each level.
- Benchmark-wise, agent-wise, and task-category drops.
- Static-vs-live gap (Live diagnostic).
- Error taxonomy aggregated by stable code.

See `docs/evaluation_ladder.md` and `packages/rust/analysis` for the full
contract. Proof-subset **natural** semantic minima are **evidence-gated**; see
`docs/evidence_empirical_status.md`.

## Current empirical status in-tree

Frozen panels, exports, and gate outcomes are summarized in
`docs/evidence_empirical_status.md` and bounded offline by
`paper/exports/strict_feasibility_report.json` (from
`ci/scripts/analyze_strict_feasibility.py`).

The repository includes end-to-end pilot and flagship bundles; **headline
empirical claims** in the NeurIPS package are locked to Tier A/B in
`docs/CLAIM_LOCK_NEURIPS2026.md`.

## Non-goals

- Producing our own coding agents. We evaluate externally produced patches.
- Automated Rust-to-Lean translation. Lean obligations are curated.
- A new benchmark dataset at bootstrap. We audit existing public datasets.
- Population-scale leaderboards or natural semantic-failure-rate estimates from
  diagnostic slices alone.
