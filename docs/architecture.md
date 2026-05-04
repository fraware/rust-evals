# Architecture

This document describes the architecture of `eval-ladder`. Treat it as the
authoritative map of crates and data flow: every major CLI surface and persisted
artifact should be explainable in terms of these abstractions. When behavior or
on-disk contracts change materially, update this document and any affected JSON
schema versions together.

## Design premise

`eval-ladder` is a **scientific evaluation system**, not a coding-agent
framework. Its job is to ingest externally produced candidate patches for
benchmark tasks, re-execute them deterministically, and score them along an
evaluation ladder whose upper rungs are progressively more semantically
justified than official benchmark scoring.

Three consequences follow from this premise:

1. The evaluator itself must look more trustworthy and disciplined than the
   systems it is evaluating. We therefore own a Rust core with strict types,
   versioned schemas, hash-chained event logs, and signed evidence bundles.
2. We must be able to audit the evaluator's own decisions. Every pass or fail
   verdict carries a stable machine code, a human explanation, and a pointer
   into an evidence bundle.
3. We avoid benchmark-specific branching in evaluator core. All
   benchmark-specific mess lives behind adapter traits in
   `packages/rust/benchmarks`.

## High-level structure

```
+-------------------------------------------------------------------------+
|                                  CLI                                    |
|                      packages/rust/cli/bin/eval-ladder                  |
+-------------------+-------------+--------------------+------------------+
                    |             |                    |
             ingest |    evaluate |             analyze|       prove-subset
                    v             v                    v             v
         +---------------+  +-----------+        +----------+   +---------+
         |   benchmarks  |  |  runner   |        | analysis |   |  lean   |
         |   adapters    |  |  (OCI)    |        +----------+   | (L4)    |
         +-------+-------+  +----+------+                        +----+---+
                 |               |                                    |
                 |               v                                    |
                 |          +---------+     +---------+               |
                 |          | policy  |     |traces   |               |
                 |          | (L3)    |     |(JSONL+  |               |
                 |          +----+----+     | hash    |               |
                 |               |          | chain)  |               |
                 |               v          +----+----+               |
                 |       +-------------+         |                    |
                 |       |strengthening|         |                    |
                 |       | (L2)        |         |                    |
                 |       +------+------+         |                    |
                 |              |                |                    |
                 |              v                v                    |
                 |        +----------+    +-----------+               |
                 +------->|  core    |<---| evidence  |<--------------+
                          | types &  |    | bundles   |
                          | contracts|    +-----------+
                          +----------+
```

- Core types flow upward from `core`. Every other crate depends on `core`
  and on a subset of the domain crates. The CLI is the only crate allowed to
  wire them together.
- Evidence bundles are the only contract that crosses the CLI boundary into
  downstream tooling (analysis, Lean, external reviewers).

## Crate responsibilities

| Crate                               | Responsibility                                                                 | Depends on                                      |
|-------------------------------------|--------------------------------------------------------------------------------|-------------------------------------------------|
| `eval-ladder-core`                  | Versioned types, IDs, `EvaluationLevel`, `EvaluationOutcome`, errors.          | `serde`, `thiserror`, `chrono`, `uuid`, `sha2`. |
| `eval-ladder-traces`                | Append-only JSONL event log with hash chaining. Event enum + writer/reader.    | `core`.                                         |
| `eval-ladder-policy`                | TOML policy spec. Rule evaluation. Stable `PV_*` violation codes.              | `core`, `traces`.                               |
| `eval-ladder-runner`                | OCI/Docker orchestration. `BenchmarkRunner` trait. Patch application.          | `core`, `traces`, `bollard`.                    |
| `eval-ladder-evidence`              | Evidence bundle builder. Canonical JSON. SHA-256 manifest.                     | `core`, `traces`.                               |
| `eval-ladder-benchmarks`            | Adapter traits and per-benchmark modules (`verified`, `live`, `rust_native`).  | `core`, `runner`.                               |
| `eval-ladder-strengthening`         | L2 composable validators (tests, differential, regression, property fuzz).     | `core`, `runner`, `traces`.                     |
| `eval-ladder-lean`                  | L4 obligation manifest + `LeanChecker`. Emits `proof_results.json`.            | `core`, `traces`, `runner`.                     |
| `eval-ladder-analysis`              | Score descent, conditional reversal, rank stability, taxonomy, static-vs-live, paper export. Pure functions over bundles. | `core`, `evidence`.           |
| `eval-ladder-cli`                   | Entrypoint `eval-ladder`. Config loading. Subcommand dispatch. Hosts the `evaluate`, `prove-subset`, `analyze`, `verify`, and `demo` commands. | all of the above.             |

`core` is deliberately dependency-light; it has no runtime concerns and no
knowledge of containers, file systems, or networks.

## Data flow

1. **Ingest.** A benchmark adapter consumes a public dataset (or a cached
   mirror of it) and emits normalized `BenchmarkTask` manifests into
   `benchmarks/<benchmark>/manifests/`.
2. **Candidate submission.** External agents produce `CandidateResolution`
   records, either as single JSON files in `tasks/candidate_resolutions/` or
   as a JSONL panel in `runs/released/<panel>/panel.jsonl`.
3. **Evaluate.** The CLI prepares a containerized run per candidate, applies
   the patch, runs level-specific evaluators, emits a trace, and writes an
   evidence bundle.
4. **Prove.** For candidates whose task is in the curated subset, the Lean
   project checks the corresponding proof obligation and returns an L4
   verdict that is folded into the candidate's evidence bundle.
5. **Analyze.** The analysis crate reads evidence bundles only (never raw
   benchmark files) and emits paper-ready CSV/JSON/figures.

## Determinism requirements

The evaluator must behave the same way on inputs that are byte-identical.
This is enforced by:

- Pinned container images per task (hash-locked).
- Deterministic seed propagation when a candidate's `generation_metadata`
  declares one.
- Canonical JSON serialization (sorted keys, `\n` line endings) when hashing.
- Forbidden network access at L3 unless the task manifest declares an
  allow-list.
- Hash-chained event logs so tampering is detectable.

If any of these is violated the evaluator must emit a policy violation rather
than a success.

## L0/L1 runner

The runner crate (`eval-ladder-runner`) ships the deterministic L0/L1
pipeline. Its internal structure mirrors the data flow and is the
canonical reference for anyone adding L2-L4 stages.

```
            PipelineInputs
                 |
                 v
   +-------------+--------------+
   |  EvaluationPipeline        |
   |     (orchestrator)         |
   |                            |
   |  Clock  ------------------->  trace timestamps + bundle created_at
   |  RunIdentity (UUIDv5) ----->  run_id + bundle_id
   |                            |
   |  prepare_workspace (L0)    |
   |  apply_patch (L0)          |
   |  ContainerEngine.exec (L0) |--> ExecOutcome --+
   |  Scorer.score    (L0)      |                  |
   |                            |                  v
   |  prepare_workspace (L1)    |            TraceWriter (hash-chained JSONL)
   |  apply_patch (L1)          |                  |
   |  ContainerEngine.exec (L1) |                  v
   |  Scorer.score    (L1)      |          BundleBuilder.finalize_at
   |  reconcile_l1              |                  |
   +----------------------------+                  v
                                              EvidenceBundleIndex
                                              + bundle_hash
```

Key abstractions:

- **`Clock`** (`SystemClock`, `FixedClock`). Every timestamp inside the
  pipeline - trace events and `created_at` on the bundle index - is
  sourced here. Tests and reruns use `FixedClock` so bundle hashes are
  reproducible; production uses `SystemClock`.
- **`RunIdentity` / `DeterministicSeed`**. `RunId` and `BundleId` are
  UUIDv5 derivations over a stable namespace plus `(candidate_id,
  task_id, evaluator_version, seed_tag)`. Two runs with identical
  inputs collide on the same IDs on purpose - that is what makes bundle
  hashes stable across machines.
- **`ContainerEngine`** trait with `ExecSpec` (image, workdir, command,
  env, limits). Two in-tree implementations: `DockerEngine` (production)
  and `LocalProcessEngine` (Docker-free, used by the acceptance test).
- **`Scorer`** trait with `SimpleExitCodeScorer` in-tree. Any benchmark
  adapter can plug a richer scorer without editing the pipeline.
- **`prepare_workspace`** + **`apply_patch`**. Every level gets a fresh
  copy of the template workspace; the template is never mutated. The
  patch is applied via `git apply --whitespace=nowarn`, with an empty
  byte slice treated as a no-op.
- **`TraceWriter::append_at`** and **`BundleBuilder::finalize_at`**.
  Additive APIs that take explicit timestamps so the pipeline stays
  pure w.r.t. the outside world.

The pipeline writes one evidence bundle containing: the candidate JSON,
the task manifest, the patch, `run_manifest.json`, both L0 and L1
`EvaluationResult` JSONs, container metadata, stdout/stderr logs, and
the hash-chained `trace.jsonl`. Hashes are computed over canonical JSON
for every JSON artifact and over raw bytes for everything else.

The acceptance invariant - two reruns with identical inputs and
`FixedClock` must produce byte-identical `trace.jsonl` and the same
`bundle_hash` - is pinned by
`packages/rust/runner/tests/pipeline_acceptance.rs`.

## L2 strengthening

Higher ladder rungs plug into the L0/L1 pipeline through a single
trait, `eval_ladder_runner::LevelExtension`. The runner crate stays
unaware of L2-specific concepts; the `eval-ladder-strengthening`
crate implements the trait and is loaded by the CLI when `--levels`
includes `L2`. The extension runs after L1 and before the pipeline
writes `RunFinished` and seals the bundle, so L2 events join the same
trace hash chain and L2 artifacts are folded into the bundle hash.

```
         (after L1 reconcile, before RunFinished)
                          |
                          v
            +----------- L2Extension -----------+
            |                                   |
            |  ValidationContext                |
            |   (template, staging, engine,     |
            |    clock, env, limits, spec,      |
            |    oracle_patch_bytes?)           |
            |                                   |
            |  +-----------+  +---------------+ |
            |  |Augmented  |  |Regression     | |
            |  |UnitTests  |  |Check          | |
            |  +-----+-----+  +-------+-------+ |
            |        |                |         |
            |        v                v         |
            |   ValidatorVerdict   ValidatorVerdict
            |                                   |
            |  +----------------+  +----------+ |
            |  |Differential    |  |PropertyFuzz|
            |  |BehaviorCheck   |  |(stub)    | |
            |  +-------+--------+  +----+-----+ |
            |          |                |       |
            |          v                v       |
            |   ValidatorVerdict   ValidatorVerdict
            |                                   |
            |              aggregate            |
            |                |                  |
            |                v                  |
            +------ EvaluationResult (L2) ------+
                             |
                             v
                  strengthened_results.json
                  strengthening_report.json
                             |
                             v
             RunFinished -> BundleBuilder.finalize_at
```

Key abstractions:

- **`LevelExtension`** (runner crate). Trait: `name`, `level`,
  `result_file`, `run(ctx, trace) -> EvaluationResult`. Every
  post-L1 stage implements this; `L2Extension` is the only in-tree
  implementation today. The runner enforces uniqueness
  of `level()` and `result_file()` so two extensions cannot
  overwrite each other's bundle slot.
- **`StrengtheningSpec`** (strengthening crate). Declarative
  task-level JSON with four sections: `augmented`, `regression`,
  `differential`, `property_fuzz`. Reviewers read the spec to audit
  which commands are being run and what is being compared; authors
  version-control it alongside the task manifest.
- **`Validator`** trait with four implementations:
  - `AugmentedUnitTests` - runs spec commands in a fresh patched
    workspace; any non-zero exit -> `L2_AUG_TESTS_FAIL`.
  - `TargetedRegressionCheck` - same mechanics, different intent,
    failure code `L2_REGRESSION_FAIL`.
  - `DifferentialBehaviorCheck` - prepares a second workspace with
    the oracle patch, runs each observable in both, compares chosen
    streams; any divergence -> `L2_DIFF_BEHAVIOR`.
  - `PropertyFuzzCheck` - scheduled for a future release; currently
    emits `NotApplicable`.
- **Aggregation**. The L2 aggregate `EvaluationResult` passes iff every
  enabled validator's verdict is `Pass` or `NotApplicable`. The first
  failing family's code is the `primary_reason`; all later failing
  codes go to `secondary_reasons`. Every sub-check result (exit code,
  timed-out flag, truncated stderr head, family metrics) is preserved
  in `strengthening_report.json` so analysis can attribute L2 drops to
  specific augmented tests or observables.

The L2 strengthening acceptance invariants - "one fixture candidate passes
L0 but fails L2" and "L2 reruns are deterministic" - are pinned by
`packages/rust/strengthening/tests/milestone_d_acceptance.rs`.

## L3 policy

L3 reuses the same `LevelExtension` seam as L2. The
`eval-ladder-policy` crate implements `L3Extension`, which runs after
L2 (or after L1 when L2 was not requested). Unlike L2, L3 is a pure
function of already-observable state: it does not re-execute the
candidate.

```
    (after L2, before RunFinished; trace writer still open)
                       |
                       v
          +---------- L3Extension ----------+
          |                                  |
          |  ExtensionContext                |
          |   + Policy (TOML)                |
          |   + L3Observation                |
          |        ^                         |
          |        |                         |
          |  build_run_context               |
          |   - parse patch_bytes (diff.rs)  |
          |   - read+verify trace.jsonl      |
          |   - read l1 verdict              |
          |   - inspect bundle layout        |
          |                                  |
          |          v                       |
          |       RunContext                 |
          |          |                       |
          |          v                       |
          |    engine::evaluate              |
          |  (pure, globset-backed)          |
          |          |                       |
          |          v                       |
          |     PolicyReport                 |
          |   (ordered Vec<PolicyFinding>)   |
          |          |                       |
          |  emit PolicyViolationDetected    |
          |   trace event per finding        |
          |          |                       |
          |          v                       |
          +-- EvaluationResult (L3) ---------+
                        |
                        v
                policy_results.json
                        |
                        v
          RunFinished -> BundleBuilder.finalize_at
```

Key abstractions:

- **`Policy`** (policy crate). Declarative TOML document. Nine rule
  families are declared as typed fields (`forbidden_commands`,
  `allowed_edit_globs`, `required_trace_events`, etc.) and the loader
  rejects unknown keys to prevent silent misconfiguration.
- **`RunContext`**. The judged input to `engine::evaluate`. Built by
  `build_run_context` from the runner's `ExtensionContext` plus
  an injected `L3Observation`. The builder re-reads `trace.jsonl`
  through `TraceReader::read_and_verify`, which revalidates the hash
  chain as a defense-in-depth integrity check.
- **`engine::evaluate`**. Pure, deterministic, single function. It
  walks the policy rule families in a fixed order and appends
  `PolicyFinding`s with stable `PV_*` codes (`PolicyViolation::as_str`).
  Finding order is therefore stable across reruns.
- **`L3Extension`**. Wraps `Policy` + `L3Observation` and implements
  `LevelExtension`. Emits one `PolicyViolationDetected` trace event
  per finding, writes `policy_results.json` with the full report plus
  a `run_context_summary` block, and returns an `EvaluationResult`
  whose `primary_reason` is the first finding's code.
- **`diff::modified_paths`**. Lightweight unified-diff path
  extractor. Handles both `diff --git a/X b/Y` headers and
  `--- a/X / +++ b/Y` fallbacks; ignores `/dev/null` sentinels;
  preserves first-seen order for stable reports.

The L3 policy acceptance invariants - "one fixture candidate passes
L0, L1, and L2 but fails L3" and "L3 reruns are deterministic" - are
pinned by `packages/rust/policy/tests/milestone_e_acceptance.rs`.

## L4 proof subset

L4 reuses the same `LevelExtension` seam as L2 and L3. The
`eval-ladder-lean` crate implements `L4Extension`, which runs after
L3 (or after the lowest available rung when higher rungs were not
requested). L4 is narrow on purpose: it does not translate Rust to
Lean, parse arbitrary diffs, or attempt whole-repository
verification. Every obligation is a curated manifest entry pointing
at a pre-landed Lean declaration under
`packages/lean/EvalLadder/Obligations/`.

```
     (after L3, before RunFinished; trace writer still open)
                         |
                         v
            +---------- L4Extension ----------+
            |                                  |
            |  ExtensionContext                |
            |   + ObligationManifest           |
            |   + dyn LeanChecker              |
            |   + lean_root: &Path             |
            |                                  |
            |  manifest.get(task_id)?          |
            |    |                             |
            |    +-- None -> NotApplicable     |
            |    +-- Some(obligation) ---+     |
            |                             |    |
            |                             v    |
            |              LeanChecker.check(..)
            |              (ExternalProcessChecker
            |               or ScriptedChecker)
            |                             |    |
            |                             v    |
            |                    LeanCheckOutcome
            |                    { status, code, .. }
            |                             |    |
            |                  interpret(obligation, outcome)
            |                             |    |
            |                             v    |
            |                        ProofReport
            |                             |    |
            |  emit ProofCheckFinished    v    |
            +------ EvaluationResult (L4) -----+
                            |
                            v
                    proof_results.json
                            |
                            v
             RunFinished -> BundleBuilder.finalize_at
```

Key abstractions:

- **`ProofObligation`** (lean crate). Typed mirror of
  `schemas/proof_obligation.schema.json` with `deny_unknown_fields`
  on every nested struct. `ObligationManifest::from_path` loads a
  JSONL manifest while tolerating blank lines and `#` comments.
- **`LeanChecker`** trait. The single production implementation is
  `ExternalProcessChecker`, which spawns the obligation's
  `proof_checker.command args...` with `cwd = lean_root` and parses
  a single `LeanCheckOutcome` JSON from stdout. The in-tree
  `ScriptedChecker` is a deterministic test double used by the
  L4 acceptance tests (and available to anyone who wants to
  audit L4 bundle hashing without a Lean toolchain).
- **`LeanCheckOutcome`**. Three-valued status (`Valid` / `Invalid`
  / `NotApplicable`) plus a stable uppercase `code`, a free-form
  `message`, and an opaque `payload`. Non-zero exit codes are
  tolerated as long as stdout carries a parseable outcome; that lets
  checkers signal `Invalid` through a canonical exit code without
  losing structure.
- **`L4Extension`**. Holds borrows of the manifest, checker, and
  Lean project root. Emits `ProofCheckStarted` and
  `ProofCheckFinished` on the run's hash chain, writes
  `proof_results.json` (full `ProofReport`), and returns an
  `EvaluationResult` whose `primary_reason` is the obligation's
  `pass_criterion` on success, `L4_OBLIGATION_UNMET` on a
  checker/obligation disagreement, `L4_PROOF_CHECK_FAILED` on
  harness errors, or `L4_OBLIGATION_NOT_APPLICABLE` when the
  manifest has no entry for the task.

The L4 acceptance invariants - "L4 Valid / Invalid /
NotApplicable matrix under a scripted checker" and "L4 reruns are
deterministic" - are pinned by
`packages/rust/lean/tests/milestone_f_acceptance.rs`. An opt-in
integration test (`#[ignore]`) exercises the real `lake` binary
against the seeded fixture obligation.

## Paper export pipeline

This stage is the only one that reads across multiple candidates
at once. It takes a run directory produced by the evaluation pipeline
(one sealed evidence bundle per candidate), projects it into a flat
`AnalysisInput`, and materializes the paper-ready tables
(aggregated exports and static-vs-live).

```
     runs/<panel>/results/
     +-- <candidate-1>/              (sealed evidence bundle)
     |     candidate_resolution.json
     |     official_results.json
     |     l1_trusted_rerun_results.json
     |     strengthened_results.json          (L2 optional)
     |     policy_results.json                (L3 optional)
     |     proof_results.json                 (L4 optional)
     +-- <candidate-2>/ ...
                 |
                 v
        load_bundle_dir(run_dir, opts)
                 |
                 v
            AnalysisInput
        (one row per candidate x level;
         rows sorted by (candidate_id, ladder-index))
                 |
             +---+----+---------------+-----------+-------------+--------------+
             v        v                    v           v             v
      score_descent   conditional_false    rank_      taxonomy_    static_vs_live
             |        _success             stability  counts       (static-vs-live)
             |          |                   |           |             |
             +---+------+------+------------+-----+-----+------+------+
                 |             |                  |            |
                 v             v                  v            v
             CSV + canonical JSON per table
                 |
                 v
          write_paper_exports(input, out_dir)
                 |
                 v
     paper/exports/<panel>/
       score_descent.csv + .json
       conditional_reversal.csv + .json (+ deprecated conditional_false_success.* aliases)
       rank_stability.csv + .json
       taxonomy.csv + .json
       static_vs_live.csv + .json           (static-vs-live)
       manifest.json  (SHA-256 of every sibling; audit-stable)
```

Key abstractions:

- **`bundle_loader::load_bundle_dir`** (analysis crate). Walks a
  run directory lexicographically, reads `candidate_resolution.json`
  and every recognized `*_results.json` per bundle, and refuses to
  silently drop bundles - mismatched `candidate_id`, mismatched
  `task_id`, or an empty bundle all yield structured
  `BundleLoadError`s. The loader does *not* verify bundle hashes; that
  is `eval_ladder_evidence::verify_bundle`'s job.
- **`AnalysisInput`**. Flat, denormalized, one row per
  `(candidate, level)`. Rows are sorted by `(candidate_id,
  ladder-index, primary_reason)` so the downstream tables are
  deterministic for any fixed input directory.
- **Pure analysis functions**. `score_descent`,
  `conditional_reversal`, `rank_stability::kendall_tau_b`, and
  `taxonomy_counts` are free functions over `&AnalysisInput`. They
  are intentionally stateless so the paper pipeline is trivially
  re-runnable in CI without reproducing the runner.
- **`paper_export::write_paper_exports`**. Materializes every table
  (score descent, conditional reversal, rank stability,
  taxonomy, and static-vs-live) into CSV and canonical JSON, then
  emits `manifest.json` containing `{path, sha256, bytes}` for every
  emitted file plus `schema_version`, `evaluator_version`, and
  `input_row_count`. This is the single audit surface that
  downstream tooling (paper builds, CI drift detection) hashes.
  `PAPER_EXPORT_SCHEMA_VERSION` is `3` (`2` after adding
  static-vs-live files, then `3` for explicit `analysis_mode`
  provenance in `manifest.json`).
  `static_vs_live` pair.

The paper-export acceptance invariants - "bundles load into a
deterministic `AnalysisInput`", "rerunning `write_paper_exports` over
the same input produces byte-identical files and manifest", and "the
headline `P(fail L2 | pass L1)` finding is visible end-to-end" - are
pinned by `packages/rust/analysis/tests/milestone_g_acceptance.rs`.

### Static-vs-live comparison

Static-vs-live analysis is a pure extension: it does not touch the
bundle loader, evidence model, or runner. It adds
`static_vs_live::static_vs_live(&AnalysisInput) -> Vec<StaticVsLiveRow>`,
a fifth paper-export pair (`static_vs_live.{csv,json}`), and a
dedicated CLI (`eval-ladder analyze static-vs-live`).

Design points:

- **Arms enumerated in code.** `STATIC_BENCHMARKS` and
  `LIVE_BENCHMARKS` are module-level constants so there is one place
  to update when a benchmark moves between arms. Rows outside either
  arm (for example `RustSweBench`) are silently excluded from this
  table - they are not "not applicable"; they are a different paper
  surface entirely.
- **One row per `(agent_id, level)`.** Pooling across agents loses
  the per-agent asymmetry the paper is about. Keeping rows narrow
  also makes zero-denominator handling trivial.
- **`delta` and `ratio` only when defined.** Both are `Option<f64>`
  in the Rust type, emitted as empty in CSV and `null` in canonical
  JSON. `ratio` is additionally `None` when `static_pass_rate == 0`
  to avoid conveying a relative comparison that has no informational
  content.
- **Schema bump.** `PAPER_EXPORT_SCHEMA_VERSION` is bumped from `1`
  to `2`, then to `3` for the `analysis_mode` field. The manifest
  remains sorted by `files[].path`, so
  filename-based readers are forward-compatible; hash-based readers
  re-pin.

Acceptance invariants - "static-vs-live rows are deterministic",
"`delta < 0` surfaces when live pass rate drops", and
"`static_vs_live.{csv,json}` are present and linked in the manifest" -
are pinned by
`packages/rust/analysis/tests/milestone_l_acceptance.rs`.

## Batch evaluation

Batch evaluation is the CLI-level orchestrator that drives the pipeline
over a panel of candidates in one invocation. It does *not* extend
the evidence model; it only sequences L0–L4 pipeline runs and emits
one extra artifact (`batch_summary.json`).

```
     panel.jsonl
        |
        v
     load_panel(panel_path)
        |
        v
     Vec<PanelEntry>            <-- paths resolved relative to panel_path
        |
        |    batch-wide flags:
        |      --strengthening-spec / --policy / --obligations / --lean-root
        |      --levels, --deterministic-clock, --seed-tag, --timeout-secs
        |
        v
     load_backing_resources(args)       <-- loaded once
     build_extensions(resources, net)   <-- one set of &dyn LevelExtension
        |
        v
     for entry in panel:
         run_entry(entry, out, levels, exts, args)
             |                       |
             | Ok(outcome)           | Err(e)
             v                       v
         BatchEntryRow{Ok}       BatchEntryRow{Invalid,
           bundle_hash            error = "BATCH_*"}
           levels{l0..l4}
        |
        v
     sort_by(bundle_name)
        |
        v
     BatchSummary
        |
        v
     canonical_json(summary) -> <out>/batch_summary.json
```

Key abstractions:

- **`PanelEntry`**. JSONL row with `task`, `candidate`, `patch`,
  `workspace_template`, and optional `bundle_name` / `entry_id`.
  Relative paths are resolved against the panel file's parent
  directory; unknown fields are rejected.
- **`BatchExtensions` / `BatchBackingResources`**. Loaded once so the
  `L2Extension`, `L3Extension`, and `L4Extension` references are
  stable across every panel entry. The lifetime of the `LevelExtension`
  refs is pinned by the enclosing `BatchBackingResources`.
- **`run_entry`**. The per-entry resilience boundary: any recoverable
  load or pipeline error becomes a `BatchEntryStatus::Invalid` row
  with a stable `BATCH_*` code, and the loop continues. Only a
  panel-wide failure (unreadable panel, every entry invalid) yields
  a non-zero exit code.
- **`BatchSummary`**. Deterministic, canonical-JSON artifact at
  `<out>/batch_summary.json`. Entries are sorted by `bundle_name`.
  Wall-clock `started_at`/`finished_at` fields are omitted under
  `--deterministic-clock` so the whole batch is byte-stable.

The batch evaluation acceptance invariants - "bundles are written per
entry", "one bad entry does not abort the batch", and "rerunning the
batch produces byte-identical summary content and bundle hashes" -
are pinned by the batch acceptance tests in
`packages/rust/cli/src/commands/batch.rs`.

## Python compatibility layer

The Python compat layer lives under `packages/python/benchmark_compat/`
and is the only sanctioned place where non-Rust code touches evaluator
artifacts. Its scope is intentionally narrow: **ingest** Python-native
benchmark manifests into canonical `BenchmarkTask` JSON files that the
Rust evaluator consumes unchanged. Evaluator decisions never live in
Python.

```
  SWE-bench Verified manifest (.jsonl / .json)
              |
              v
        _iter_manifest(source)                   [cli.py]
              |
              v
     dict | SweBenchInstance                    [swe_bench.py]
              |
              v
      normalize_instance(record)                 <-- pure, deterministic
              |                                      no I/O, no clocks
              v
         BenchmarkTask                          [schemas.py; pydantic v2]
              |
              v
  model_dump(mode="json") -> dict
              |
              v
    validate_benchmark_task(payload)             [validate.py]
              |   (jsonschema draft 2020-12 against
              |    schemas/benchmark_task.schema.json)
              v
        canonical_json(payload)                  [canonical.py]
              |   (orjson; sorted keys; no trailing newline)
              v
        <out_dir>/<task_id>.json
              |
              v
  serde_json::from_slice::<BenchmarkTask>        [Rust side; integration test]
              |
              v
  eval_ladder_core::canonical_json(task) == bytes-on-disk
```

Key modules:

- **`canonical.py`**. Single canonicalization surface. Wraps `orjson`
  with `OPT_SORT_KEYS | OPT_UTC_Z`, no trailing newline - byte-
  identical to `eval_ladder_core::canonical_json`. A self-check runs
  at import time.
- **`swe_bench.py`**. `SweBenchInstance` (input pydantic model with
  `extra="ignore"` so new upstream fields do not gate ingestion) plus
  a pure `normalize_instance` function. All mappings (task_id derived
  from `instance_id`, `gold_patch_ref` as `sha256:<hex>` of the raw
  patch bytes, deterministic `official_test_entrypoint`, lexicographic
  test ordering) are fixed and refusing-by-design on malformed input.
- **`validate.py`**. `validate_benchmark_task` cross-validates every
  emitted payload against `schemas/benchmark_task.schema.json`, so
  pydantic / JSON Schema drift fails loudly before writing.
- **`cli.py`**. Typer app backing the `eval-ladder-py` console script.
  `normalize-swe-bench` accepts both JSONL and JSON-array manifests,
  defaults to resilient per-record handling, and supports `--strict`
  for fail-fast ingestion.

Cross-language determinism is pinned by
`tests/integration/tests/python_round_trip.rs` (integration test
`milestone_i_python_emitted_benchmark_task_deserializes_in_rust`):
the Rust side deserializes the Python-emitted bytes into the Rust
`BenchmarkTask` struct, re-emits via `canonical_json`, and asserts
the bytes match on both sides.

## Bundle and trace verification

`eval-ladder verify` is the single command reviewers
use to answer "are these artifacts the ones the evaluator actually
produced?". The CLI is a thin dispatcher over two library surfaces
that already existed in the workspace but were never exposed as a
shipped entry point:

- `eval_ladder_evidence::verify_bundle` - recomputes every file's
  SHA-256, then the `bundle_hash` over the canonical JSON of the
  index with `bundle_hash` elided.
- `eval_ladder_traces::TraceReader::read_and_verify` - streams
  `trace.jsonl` and asserts each event's recomputed `event_hash`
  matches the stored value and that `prev_event_hash` links every
  event to the previous one, starting from a unique `RunStarted`.

```
       verify bundle            verify trace           verify run-dir
             |                        |                       |
             v                        v                       v
  verify_single_bundle         TraceReader::        walk run_dir for
   (file hashes + chain)       read_and_verify      bundle subdirs
             \                      /                       |
              \                    /                        v
               \                  /             verify_single_bundle
                v                v              for each; sort rows
             VerifyEntryRow {
               status: ok | invalid | not_applicable,
               bundle_hash: Option<Sha256Digest>,
               bundle, trace: VerifyStatus,
               error_code: Option<VERIFY_*>,
               error: Option<String>,
             }
                              |
                              v
                       VerifyReport
                              |
                              v
                 canonical_json -> verify_report.json
```

Key design points:

- **Stable error codes** (`VERIFY_FILE_DIGEST_MISMATCH`,
  `VERIFY_BUNDLE_DIGEST_MISMATCH`, `VERIFY_TRACE_CHAIN_BROKEN`, ...).
  See `docs/evidence_manual.md#stable-error-codes` for the
  complete list. Downstream scripts match on these codes instead of
  human-readable messages.
- **Canonical report**. `VerifyReport` is serialized via
  `canonical_json`; rows are sorted by `bundle_name`; absolute
  `run_dir` / `bundle_dir` strings are the only non-deterministic
  fields (tempdir / operator paths) and are normalized out by the
  The verify acceptance test.
- **Resilience**. By default a single bad bundle never aborts the
  run; every row is still written to the report. `--fail-fast`
  opts into short-circuit CI mode.
- **Lib re-use, not duplication**. The CLI adds no cryptographic
  logic; it only classifies errors from the two library functions
  into stable codes and aggregates rows into a deterministic
  report.

Verify acceptance invariants live in
`packages/rust/cli/src/commands/verify.rs::tests::milestone_j_*`:
clean bundles pass, tampering a single file flips exactly one row
to `invalid` while leaving its neighbours untouched, and two runs
against byte-identical inputs produce byte-identical reports after
tempdir normalization.

## Reproducibility demo

`eval-ladder demo run` is an orchestrator, not a new layer. It
composes the existing public library surfaces in-process so that a
reviewer with nothing more than a Rust toolchain can exercise the
full end-to-end story:

```
              seed_inputs(out)                    [demo.rs]
               |
               v
     inputs/panel.jsonl, inputs/<tag>/task.json, candidate.json,
     patch.diff, workspace/, evaluator.toml
               |
               v
         run_batch(BatchArgs)                     [batch.rs]
               |
               v
     bundles/bundle-<tag>/artifact_hashes.json, trace.jsonl, ...
     bundles/batch_summary.json
               |
               v
  load_bundle_dir(bundles/)                       [analysis::bundle_loader]
               |
               v
    write_paper_exports(AnalysisInput, paper/)    [analysis::paper_export]
               |
               v
     paper/score_descent.{csv,json}, conditional_reversal.* (+ legacy aliases),
     rank_stability.*, taxonomy.*, manifest.json
               |
               v
  run_run_dir(VerifyRunDirArgs)                    [verify.rs]
               |
               v
     bundles/verify_report.json (all rows `status: ok`)
```

Design constraints:

- **No subprocess spawns.** Every step calls the canonical
  library entry point directly; any future change to those
  surfaces fails demo acceptance tests immediately.
- **No upstream data.** Task manifests, candidates, patches, and
  workspaces are synthesized from a pinned namespace UUID and a
  fixed wall clock (`2025-01-01T00:00:00Z`).
- **Deterministic outputs.** Two demo runs against the same
  `--out` / `--tasks` produce identical bundle hashes and
  identical verify-report content (modulo tempdir paths, which are
  normalized out in the acceptance test).

The demo workflow is pinned by `milestone_k_demo_runs_end_to_end` (all
bundles ok, every paper table present, verify report all-green)
and `milestone_k_demo_is_byte_deterministic_across_runs` (bundle
hashes and report content match byte-for-byte across reruns).

## Extension points

New benchmarks integrate via `eval-ladder-benchmarks::BenchmarkAdapter`.
New strengthening checks integrate via
`eval-ladder-strengthening::Validator`. New post-L1 evaluation levels
integrate via `eval_ladder_runner::LevelExtension` (`L2Extension`,
`L3Extension`, and `L4Extension` are the three in-tree
implementations). New proof-validation backends integrate via
`eval_ladder_lean::LeanChecker` without touching the pipeline. No
extension should require editing `eval-ladder-core` or the CLI
dispatch.

## What this architecture deliberately does not do

- It does not generate patches. Generation support is optional and is not
  part of the evaluator's trust story.
- It does not attempt whole-repository Rust-to-Lean translation. The Lean
  layer only checks curated, task-specific obligations.
- It does not interpret benchmark-specific logic in `core`. Every
  benchmark-specific decision is visible at the adapter boundary and can
  be audited independently.
