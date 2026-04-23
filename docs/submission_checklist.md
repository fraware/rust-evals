# Submission checklist

Target venue: NeurIPS 2026 Evaluation & Datasets track.

Key dates:

- Abstract deadline: **May 4, 2026 AoE**.
- Full paper + supplementary material deadline: **May 6, 2026 AoE**.

## Release mode selection

Choose one mode per submission.

### Mode 1 - Code-only audit submission

- Audits only existing public benchmarks (SWE-Bench Verified, SWE-bench-Live,
  Rust-SWE-bench).
- Does **not** redistribute benchmark data.
- The curated proof subset is described as a derived evaluation slice
  referencing upstream task identifiers. No raw benchmark content is
  republished.
- No Croissant metadata is required.

**Required artifacts:**

- [ ] Executable repository at the tagged release commit. Tag pending.
- [x] Documented CLI with worked examples in `docs/operational_runbook.md`.
- [x] Evidence bundles for a released panel under `runs/released/`.
      `runs/released/rust_pilot_v1/results/` is sealed and
      `verify run-dir` clean (`1 ok / 0 invalid`).
- [x] Paper-ready CSV/JSON exports under `paper/exports/`.
      `paper/exports/rust_pilot_v1/` is generated from the sealed pilot
      run-dir via `analyze paper-export`.
- [x] `docs/scientific_scope.md` up to date with the claim and scope.
- [x] Reproducibility fixtures runnable without downloading full benchmarks.
      Milestone K ships `eval-ladder demo run` as the fifteen-minute
      slice; pinned by `milestone_k_demo_runs_end_to_end`.

### Mode 2 - Code + new proof-carrying subset

- Releases the curated proof subset as a new dataset artifact.
- Triggers dataset hosting and Croissant metadata requirements under the
  track rules.

**Additional required artifacts beyond Mode 1:**

- [ ] Proof-subset hosting resolved (identifier, persistent URL, license).
- [ ] Croissant metadata under
      `datasets/derived/proof_subset/croissant/metadata.json`.
- [ ] Dataset datasheet under
      `datasets/derived/proof_subset/docs/datasheet.md`.
- [ ] Selection-bias audit under `paper/exports/proof_subset_bias_audit.*`.

## Scientific checklist

- [x] Paper claim stated explicitly, traceable to `docs/scientific_scope.md`.
- [x] Evaluation ladder levels fully defined and implemented. L0-L4 are
      shipped as `LevelExtension` implementations (Milestones C, D, E,
      and F).
- [x] Stable failure codes used in every pass/fail verdict. Enumerated
      across `FailureReason`, `PolicyViolation::as_str`, the `L4_*`,
      `BATCH_*`, and `VERIFY_*` families.
- [x] Static-vs-live comparison reported. Milestone L ships the
      per-agent, per-level table via `eval_ladder_analysis::static_vs_live`
      and the `analyze static-vs-live` / `analyze paper-export` CLIs
      (`static_vs_live.{csv,json}`). Paper-export manifest schema bumped
      to `2`.
- [x] Rank instability reported with at least one rank correlation.
      Milestone G emits Kendall tau-b between every pair of agent
      leaderboards via `rank_stability.kendall_tau_b`.
- [x] False-success taxonomy reported at stable-code granularity.
      Milestone G emits `taxonomy.{csv,json}` keyed by
      `(benchmark, level, primary_reason)`.
- [x] Conditional false-success rate reported per level transition.
      Milestone G emits `conditional_false_success.{csv,json}` for
      every adjacent level pair.
- [x] Per-benchmark and per-agent breakdowns reported. Milestone G's
      `score_descent` stratifies by both dimensions.

## Engineering checklist

- [x] `cargo build --workspace` succeeds on a clean checkout.
- [x] `cargo clippy -D warnings` clean.
- [x] `cargo fmt --check` clean.
- [x] `cargo test` passes for all crates.
- [x] `mypy` clean on `packages/python`.
- [x] `ruff check` clean on `packages/python`.
- [ ] `cargo deny check` and `cargo audit` show no high-severity issues.
      Configured via `deny.toml` and `just deny` / `just audit`; to be
      re-run immediately before tagging.
- [x] Every JSON schema validates against its own draft 2020-12
      specification. Pinned by `just validate-schemas` /
      `eval-ladder schema validate`.
- [ ] Tier 3 CI has passed on the tagged release commit. Release tag
      still pending.

## Reviewer ergonomics

- [x] README quick-start works on a fresh machine.
- [x] Runbook walks through a complete batch evaluation (Milestone H
      recipe in `docs/operational_runbook.md#batch-evaluation-milestone-h`).
- [x] Evidence bundles are hash-verifiable with a shipped command
      (`eval-ladder verify run-dir --run-dir runs/released`; see
      `docs/operational_runbook.md#bundle-and-trace-verification-milestone-j`).
- [x] Paper exports are regenerable from a single `analyze` invocation
      (`eval-ladder analyze paper-export --run-dir ... --out-dir ...`).
- [x] The smallest reproducible slice runs in under fifteen minutes on a
      developer machine
      (`eval-ladder demo run --out runs/demo --tasks 2`; see
      `docs/operational_runbook.md#reproducibility-demo-milestone-k`).
