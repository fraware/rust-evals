# Evaluation ladder

The evaluation ladder defines five evaluator levels, L0 through L4. Each level
has a precise definition, a set of inputs, a set of checks, and a stable code
for each failure reason. Evaluator output is a verdict per level per
candidate. Levels are evaluated independently; a candidate may pass some
levels and fail others in any pattern.

## L0 - Official

**Definition.** The candidate passes official benchmark validation exactly as
the benchmark itself would report it.

**Inputs.** Benchmark-native scorer; candidate patch; official test set.

**Checks.** Wrap the benchmark's native scorer and record its raw output
verbatim, then normalize to `pass | fail | invalid`.

**Failure reasons.** One of `L0_OFFICIAL_FAIL`, `L0_OFFICIAL_INVALID`,
`L0_OFFICIAL_TIMEOUT`, `L0_OFFICIAL_MISSING_ARTIFACT`.

**Scientific role.** Defines the reference point. Nothing above L0 is a
replacement; each higher level is a tightening.

## L1 - Trusted rerun

**Definition.** The candidate passes a deterministic rerun in our evaluator
harness.

**Inputs.** Pinned container image for the task; canonical patch format;
multiple rerun seeds.

**Checks.**
- Container environment fingerprint matches the task manifest.
- The patch applies cleanly to the declared base commit.
- The official test set is re-executed in our harness with deterministic
  seeds.
- Repeated reruns agree on the verdict.
- Harness parsing is unambiguous for the benchmark's native output format.

**Failure reasons.** `L1_ENV_FINGERPRINT_MISMATCH`, `L1_PATCH_APPLY_FAILED`,
`L1_RERUN_DISAGREEMENT`, `L1_PARSER_AMBIGUOUS`, `L1_TIMEOUT`,
`L1_HARNESS_ERROR`.

**Scientific role.** Removes flakiness and harness-induced successes
reported in SWE-Bench+ and UTBoost.

## L2 - Strengthened

**Definition.** The candidate passes a strengthened validator in addition to
the official scorer.

**Inputs.** A composable set of validators configured via a per-task
`StrengtheningSpec` JSON (see `packages/rust/strengthening/src/spec.rs`
and `docs/operational_runbook.md`). The global
`configs/strengthening/*.toml` selects which validator families run;
the per-task spec declares the exact commands.

**Implementation.** L2 plugs into the runner's
`EvaluationPipeline` as a `LevelExtension` implemented by
`eval_ladder_strengthening::L2Extension`. The pipeline runs it after
L1 and before sealing the bundle, so L2 trace events share the L0/L1
hash chain and the bundle hash covers every L2 artifact.

**Validator modules.**
- `AugmentedUnitTests`: curated or generated tests that exercise edge cases.
- `DifferentialBehaviorCheck`: compares observable behaviour of the
  candidate-patched repo to the gold-patched repo on designated observables.
  Only runs where observables are well-defined.
- `TargetedRegressionCheck`: runs a curated regression suite relevant to the
  touched files.
- `PropertyFuzzCheck`: property-based or mutation-based checks for task
  classes where they are meaningful.

Each validator must emit a separate pass/fail verdict and a metrics payload.
The aggregate L2 verdict is a conjunction of the enabled validators.

**Failure reasons.** `L2_AUG_TESTS_FAIL`, `L2_DIFF_BEHAVIOR`,
`L2_REGRESSION_FAIL`, `L2_PROPERTY_VIOLATED`, `L2_ORACLE_UNAVAILABLE`.

**Strengthening modes (selectable).**
- `tests_only`
- `tests_plus_diff`
- `tests_plus_regression`
- `full_l2`

This decomposition lets the paper attribute which sub-check drives a score
drop.

**Generated-test hygiene.** Generated tests must be versioned, the
generation prompt/config must be frozen, flaky tests must be tracked
explicitly, and any generated test that fails on the gold patch is excluded
unless manually whitelisted with a written rationale.

## L3 - Policy-conformant

**Definition.** The candidate's success was achieved through a valid
process.

**Inputs.** A declarative policy loaded from a TOML file under
`configs/policy/` (see `packages/rust/policy` and
`docs/artifact_spec.md`).

**Implementation.** L3 plugs into the runner's `EvaluationPipeline`
as a `LevelExtension` implemented by `eval_ladder_policy::L3Extension`.
The pipeline runs it after L2 (or directly after L1 when L2 was not
requested) and before sealing the bundle. The extension is a
deterministic pure function over the captured `RunContext`, which is
assembled from the candidate patch bytes, the live (hash-verified)
`trace.jsonl`, the L0/L1 verdicts, and a `L3Observation` supplied by
the runner. L3 emits `PolicyCheckStarted` plus one
`PolicyViolationDetected` event per finding so the trace alone is
self-describing.

**Rule families.** Command allow/deny; path allow/deny; file-count threshold;
allowed binary generation; dependency edit policy; generated-test policy;
environment purity; network isolation; trace completeness.

**Failure reasons (stable codes).** `PV_NET_ACCESS`, `PV_FORBIDDEN_CMD`,
`PV_EDIT_SCOPE`, `PV_FILE_COUNT_EXCEEDED`, `PV_DEPENDENCY_EDIT`,
`PV_GENERATED_TEST_DISALLOWED`, `PV_ENV_NONDETERMINISTIC`,
`PV_TRACE_INCOMPLETE`, `PV_BINARY_DISALLOWED`.

**Scientific role.** A candidate may pass L2 and still fail L3. That is
intended. The conditional drop `P(fail L3 | pass L2)` is a first-class
result. The Milestone E acceptance test in
`packages/rust/policy/tests/milestone_e_acceptance.rs` pins both the
"L2 pass / L3 fail" case and L3 rerun determinism.

## L4 - Semantic

**Definition.** The candidate satisfies a machine-checkable semantic
obligation defined for its task in the curated proof subset.

**Inputs.** The obligation manifest entry for the task, the patched
repository snapshot, and the Lean checker invocation declared by the
obligation.

**Checks.**
- Apply the candidate patch to a clean checkout at the task's base commit.
- Extract the obligation context (typically a normalized input/output
  specification or an invariant predicate).
- Run the Lean checker declared by the obligation. The checker must return
  `valid`, `invalid`, or `not_applicable` with a machine-readable payload.

**Applicability.** L4 is only defined on tasks in the curated proof subset.
All other tasks receive `not_applicable` at L4 and are excluded from L4
aggregates.

**Failure reasons.** `L4_OBLIGATION_UNMET`, `L4_PROOF_CHECK_FAILED`,
`L4_OBLIGATION_NOT_APPLICABLE`, `L4_EXTRACTION_FAILED`. A successful
check uses the obligation's `pass_criterion` (by convention
`L4_OBLIGATION_MET`) as the primary reason.

**Implementation.** L4 plugs into the runner's `EvaluationPipeline`
as a `LevelExtension` implemented by `eval_ladder_lean::L4Extension`.
The pipeline runs it after L3 (or after the lowest available rung
when higher rungs were not requested) and before sealing the bundle.
The extension looks up `ctx.task_id` in the supplied
`ObligationManifest` and dispatches to the configured `LeanChecker`
(by default `ExternalProcessChecker`, which spawns the command
declared by the obligation with cwd = `lean_root`). Verdict, code,
checker payload, and timings are written as `proof_results.json`; a
`ProofCheckStarted` / `ProofCheckFinished` pair lands on the trace
hash chain so the run is auditable without the artifact.

The Milestone F acceptance test in
`packages/rust/lean/tests/milestone_f_acceptance.rs` pins the
`Valid` / `Invalid` / `NotApplicable` matrix under the in-tree
`ScriptedChecker` and verifies L4 rerun determinism. An opt-in
integration test invokes `lake` against
`packages/lean/EvalLadder/Obligations/Fixtures/MilestoneF.lean` and is
marked `#[ignore]`, so it only runs when explicitly selected
(`cargo test -p eval-ladder-lean -- --ignored`) or under the Tier 3
heavy workflow that provisions a Lean toolchain.

## Contract across levels

- Each level runs independently and emits its own verdict; the CLI may be
  asked to run any subset `--levels L0,L1,L3` for example.
- Verdicts never overwrite each other; the evidence bundle records each
  level's full payload.
- Every failure reason in this document is a stable string. A renamed code
  requires a schema version bump and a changelog entry.
- No silent fallback logic is permitted. If a level cannot run (for example
  because the benchmark does not expose the needed hook), the verdict must
  be `invalid` or `not_applicable`, not `fail`.

## Paper outputs (Milestone G)

The evaluation ladder is paired with a paper pipeline that reduces a
run directory (one sealed evidence bundle per candidate) to a suite
of audit-stable tables:

- **Score descent** - `passed / evaluated` by level, stratified by
  benchmark and by agent.
- **Conditional false success** -
  `P(fail L_{k+1} | pass L_k)` for every adjacent level pair. This is
  the quantitative expression of "the ladder overstates pass rate".
- **Rank stability** - Kendall tau-b between every pair of agent
  leaderboards (one per level).
- **Taxonomy** - counts of every stable `primary_reason` that appears
  on `fail` rows, grouped by `(benchmark, level, code)`.
- **Static vs live** (Milestone L; see the dedicated section below).

`eval-ladder analyze paper-export --run-dir <dir> --out-dir <dir>`
emits all tables as CSV (RFC 4180, six-digit floats) and canonical
JSON, plus a `manifest.json` that SHA-256s every sibling. Byte
determinism across reruns is pinned by
`packages/rust/analysis/tests/milestone_g_acceptance.rs` (Milestones G
tables) and `packages/rust/analysis/tests/milestone_l_acceptance.rs`
(Milestone L extension plus the `PAPER_EXPORT_SCHEMA_VERSION = 2`
bump).

## Static-vs-live comparison (Milestone L)

Milestone L adds the headline paper table that quantifies the
"overstatement" claim directly. For every agent and every evaluation
level, the analyzer reports the agent's pass rate on the static suite
(`BenchmarkId::SweBenchVerified`) alongside its pass rate on the live
suite (`BenchmarkId::SweBenchLive`), together with `delta` and
`ratio`. Negative `delta` is the paper's quantitative finding.

Design:

- **Arms defined in code, not in docs.** The static and live arms are
  enumerated in
  `packages/rust/analysis/src/static_vs_live.rs::{STATIC_BENCHMARKS,
  LIVE_BENCHMARKS}`; adding a benchmark to either arm is a one-line
  change with an automatic paper-table update.
- **No pooled row.** Pooling across agents would hide the per-agent
  asymmetry that the paper is about. Callers that want a pooled
  number compose it from the rows themselves.
- **Zero-denominator safe.** Missing rates are emitted as `None`
  (empty CSV cell, `null` JSON) so the table never silently omits an
  agent that lacks live coverage.
- **Determinism.** Rows sort by `(agent_id, level)` and all numbers
  are derived from integer counts. Re-running
  `analyze static-vs-live` or `analyze paper-export` on the same
  [`AnalysisInput`] is byte-identical; this is pinned by
  `packages/rust/analysis/tests/milestone_l_acceptance.rs`.

Two shipping surfaces:

- `eval-ladder analyze static-vs-live --run-dir <dir>` - one-shot
  CSV (and optional `--json-out`) for ad-hoc inspection.
- `eval-ladder analyze paper-export --run-dir <dir> --out-dir <dir>` -
  emits `static_vs_live.{csv,json}` alongside the Milestone G
  tables. `PAPER_EXPORT_SCHEMA_VERSION` is bumped from `1` to `2`
  so readers keyed on the manifest hash re-pin intentionally.

## Batch evaluation (Milestone H)

Driving the entire ladder across a panel of candidates is
`eval-ladder evaluate batch`:

```bash
eval-ladder evaluate batch \
  --input runs/released/agent_panel_v1/panel.jsonl \
  --levels L0,L1,L2,L3,L4 \
  --config configs/evaluator/default.toml \
  --out runs/released/agent_panel_v1/results/ \
  --strengthening-spec datasets/derived/strengthening_specs/v1.json \
  --policy configs/evaluator/policy.toml \
  --obligations datasets/derived/proof_subset/manifest.jsonl \
  --lean-root packages/lean/EvalLadder
```

One sealed evidence bundle is written per panel entry plus a
deterministic `batch_summary.json` at the root of `--out`. Any
recoverable load or pipeline error becomes a `status: "invalid"` row
instead of aborting the batch, so a single broken candidate never
sinks the run. With `--deterministic-clock`, reruns produce
byte-identical summary content and bundle hashes
(pinned by `milestone_h_batch_summary_is_deterministic`).

## SWE-bench Verified ingestion (Milestone I)

SWE-bench Verified release manifests are ingested by the Python
compat layer, which writes canonical `BenchmarkTask` JSON files that
the Rust evaluator consumes as-is:

```bash
python -m pip install -e ".[dev]"
eval-ladder-py normalize-swe-bench \
  --source datasets/public_links/verified.jsonl \
  --out-dir benchmarks/verified/manifests/
```

The Python layer mirrors the Rust canonicalization contract exactly
(sorted keys, UTF-8, no trailing whitespace, no BOM), re-validates
every emitted file against `schemas/benchmark_task.schema.json`, and
refuses to guess on malformed input. The cross-language round-trip
is pinned by the
`milestone_i_python_emitted_benchmark_task_deserializes_in_rust`
integration test under `tests/integration/`.

## Bundle and trace verification (Milestone J)

Every artifact the ladder emits is hash-verifiable offline. The
shipped `eval-ladder verify` command wraps
`eval_ladder_evidence::verify_bundle` and
`eval_ladder_traces::TraceReader::read_and_verify` in a single
reviewer-facing binary with three modes - `bundle`, `trace`,
`run-dir` - and emits a canonical `verify_report.json` with stable
`VERIFY_*` error codes. A single tampered byte in any sealed
bundle flips exactly one row to `invalid` without polluting the
others, and two runs against identical inputs produce byte-
identical reports (pinned by `milestone_j_*` acceptance tests). See
`docs/operational_runbook.md#bundle-and-trace-verification-milestone-j`
for CLI recipes and exit semantics.

## Reproducibility demo (Milestone K)

`eval-ladder demo run` is the shipped one-command reproducibility
slice: it materializes a synthetic panel, drives the full batch
pipeline with a deterministic clock, emits every paper-export
table, and re-verifies every bundle end-to-end without any
upstream benchmark data, network access, or container runtime.
The demo is an orchestrator over the same public library surfaces
the full pipeline uses, so any future change to the evaluator's
contract trips `milestone_k_*` on the first offending commit.
Two runs with identical arguments produce byte-identical
`bundle_hash` and `verify_report.json` content. See
`docs/operational_runbook.md#reproducibility-demo-milestone-k` for
invocation, output layout, and the determinism contract.
