# Scientific scope

This document records the scientific posture of `eval-ladder`. It is the
reference for reviewers and should be cited by the paper.

## Research question

Given a fixed panel of externally generated candidate patches for a benchmark
task suite, how much of the officially reported resolution rate survives when
the evaluator is replaced by a trusted, strengthened, policy-aware, and
(for a curated subset) proof-carrying evaluator?

## Core paper claim

> Official coding-agent benchmark scores overstate semantically justified
> issue resolution; a trusted evaluator and a curated proof-carrying subset
> reveal the size and structure of that overstatement.

## Scope in the NeurIPS 2026 Evaluation & Datasets track

The Evaluation & Datasets track explicitly welcomes:

- Audits of benchmark failure modes.
- Comparisons of evaluation designs.
- Refined evaluation protocols.
- Executable tools that improve how evaluative claims are constructed.

`eval-ladder` produces all four: an audit (the ladder descent), a comparison
(official vs trusted vs strengthened vs policy-aware vs semantic), a refined
protocol (L0-L4), and an executable tool (the `eval-ladder` CLI and the
signed evidence bundles).

### Dataset posture

The repository audits **only existing public datasets** at bootstrap:
SWE-Bench Verified, SWE-bench-Live, and the Rust-SWE-bench release. Under
the track's rules this removes the hosting and Croissant requirements that
apply to new dataset releases.

The curated proof-carrying subset is a derived evaluation slice over those
public datasets. Two release modes are planned, tracked in
`docs/submission_checklist.md`:

1. **Code-only audit submission.** The proof subset ships as a manifest of
   task identifiers plus obligations; no raw benchmark data is redistributed.
2. **Code + new proof subset as a dataset.** The same manifest is published
   as a Croissant-compliant dataset. This path is strictly stronger for the
   paper and strictly heavier operationally.

## Related work this repository is built to absorb and extend

The numbers below are the reported findings that define what the evaluator
must be able to reproduce or refine. Levels refer to the ladder documented
in `docs/evaluation_ladder.md`.

| Work                  | Scale                                                    | Findings motivating a ladder level |
|-----------------------|----------------------------------------------------------|------------------------------------|
| SWE-bench             | 2,294 tasks, 12 Python repositories.                     | Defines L0 (official scoring).     |
| SWE-Bench+            | Re-evaluates public leaderboards.                        | One system drops 12.47% -> 3.97%; motivates L1 (trusted rerun) and L2 (strengthened tests). |
| PatchDiff             | Patch-level behavioural comparison.                      | 7.8% of passing patches diverge from developer tests; ~6.2-point inflation; motivates L2's differential module. |
| UTBoost               | Test adequacy audit.                                     | Insufficient/erroneous tests affect 40.9% of Lite and 24.4% of Verified entries; motivates L2 augmented tests. |
| SWE-bench-Live        | 1,319 live tasks, 93 repositories, per-task Docker images. | Static-vs-live gap; motivates Live adapter and static-vs-live analysis output. |
| Rust-SWE-bench        | 500 tasks, 34 Rust repositories.                         | Repository-wide Rust semantics; motivates Rust adapter and Lean L4 slice. |

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
2. **Semantic obligation (L4).** For a curated subset of tasks we encode
   the intended semantic property as a machine-checkable Lean obligation.
   A patch that passes all tests but does not establish the intended
   property fails L4. The residual gap between L2/L3 and L4 is the clearest
   available estimate of how much "semantically justified" success the
   benchmarks would lose under a stricter criterion.

## Required outputs for the paper

The analysis crate must deterministically produce:

- Pass rate by level, per benchmark and per agent.
- Conditional false-success rate `P(fail at L_{k+1} | pass at L_k)`.
- Rank correlation between leaderboards derived at each level.
- Benchmark-wise, agent-wise, and task-category drops.
- Static-vs-live gap.
- Proof-subset residual gap.
- Error taxonomy aggregated by stable code.

See `docs/evaluation_ladder.md` and `packages/rust/analysis` for the full
contract.

## Non-goals

- Producing our own coding agents. We evaluate externally produced patches.
- Automated Rust-to-Lean translation. Lean obligations are curated.
- A new benchmark dataset at bootstrap. We audit existing public datasets.
