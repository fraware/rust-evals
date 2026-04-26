# Submission checklist

Target venue: NeurIPS 2026 Evaluation & Datasets track. Supporting technical
docs are indexed in [`README.md`](README.md).

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
- [x] `cargo deny check` and `cargo audit` show no high-severity issues.
      Configured via `deny.toml` and `just deny` / `just audit`; re-run clean
      on 2026-04-25 (deny: warnings only for unused license allow-list entries
      and duplicate lockfile packages; audit: zero advisories).
- [x] Every JSON schema validates against its own draft 2020-12
      specification. Pinned by `just validate-schemas` /
      `eval-ladder schema validate`.
- [ ] Tier 3 CI has passed on the tagged release commit. SemVer tags ``v0.1.0``
      and ``v0.1.1`` are pushed (each triggers `.github/workflows/release-tag.yml`:
      Lean ``lake build``, ``eval-ladder-lean`` tests, schema validation, and
      ``write_release_artifact_manifest.py --require-all-files`` writing
      ``paper/exports/release/<tag>/artifact_manifest.json``). Confirm green runs
      on GitHub Actions (``gh run list --workflow=release-tag.yml``; tier-3
      dispatch requires ``gh auth login`` or ``GH_TOKEN`` on this machine).
      Local prep (no CI claim): ``paper/exports/release/v0.1.2/artifact_manifest.json``
      from ``write_release_artifact_manifest.py`` without ``--require-all-files``.

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

## Evidence tranche quality gates

- [x] Machine-checkable gate script for empirical tranche quality:
      `ci/scripts/check_evidence_quality.py`.
- [x] L1 harness-error clustering for triage:
      `ci/scripts/triage_l1_harness_errors.py` (see evidence tranche plan).
- [x] Preflight of ``official_test_entrypoint`` selectors vs materialized
      workspaces: `ci/scripts/preflight_verified_selectors.py` (also enforced in
      tier-1 CI on the ``l0l1_pass_hunt_v1`` panel).
- [x] Repo-wide Verified manifest entrypoint audit:
      `ci/scripts/audit_verified_manifest_entrypoints.py` (tier-1 CI, all 500
      manifests, count pinned with ``--expect-manifest-count 500``).
- [x] Contract tests for pytest selector parsing:
      `tests/python/test_verified_pytest_targets.py` (tier-2 ``pytest``).
- [x] Subprocess CLI tests for preflight, audit, diagnose, triage, tier-1 runner,
      upstream-resolved panel filter (``--help`` contract), release manifest
      writer (including ``--require-all-files``), all ``check_evidence_quality``
      modes, and **failure paths** (exit code 2 / ``ok: false``) so gates cannot
      silently weaken: `tests/python/test_evidence_cli_scripts.py`.
- [x] Strict ``mypy`` on ``packages/python/benchmark_compat/src`` and
      ``ci/scripts`` (tier-2 ``mypy``; paths listed in root ``pyproject.toml``).
- [x] Tier-1 evidence checks (local and ``ci-tier1-fast``):
      `ci/scripts/run_evidence_tier1_checks.py`.
- [x] Optional strict exit for batch diagnostics:
      `ci/scripts/diagnose_batch_summary.py --fail-on-warnings`.
- [x] Execution playbook for remaining tranche:
      `docs/evidence_tranche_plan.md`.
- [x] Live empirical gate status, sealed batch notes, and remediation commands:
      `docs/evidence_empirical_status.md` and `runs/released/agent_panel_v3_r1/README.md`.
- [x] Evidence gate script correctness: `check_evidence_quality verified` counts
      every agent on all-fail panels; `live` mode handles null `live_pass_rate` /
      `delta` without crashing and scores ties only on rows with live data.
      Regression coverage in `tests/python/test_evidence_cli_scripts.py`.
- [x] Verified / Live / L2 / Rust **release-profile** gates passing on the
      canonical paths documented in ``docs/evidence_empirical_status.md``
      (``--gate-profile release`` on ``results_opt``, ``paper/exports/live_panel_v1_postbatch``,
      ``runs/released/l2_verified_merged_v1/results``, ``rust_proof_subset_v1/results_seal``).
- [ ] Verified **strict** flagship gate (default CLI: low harness error,
      distinct agent vectors). Still failing on the sealed 51-candidate batch;
      triage commands remain in ``docs/evidence_empirical_status.md``.
- [ ] Live **strict** comparative gate (non-tied live ranking and non-zero tau).
      Use ``--symmetric-live-ok`` or ``--gate-profile release`` only for the
      documented uniform-regression slice until a new batch diverges.
- [ ] L2 **strict** expansion gate on a single large batch
      (``--min-l1-passed-from 10`` defaults). The merged directory is for
      release-profile closure only.
- [ ] Rust proof-subset **strict** semantic gate
      (``--min-l3-pass-l4-fail 2 --min-all-level-pass 1`` on a full ladder out).
      Release profile matches tier-1 structural semantics on ``results_seal``.
