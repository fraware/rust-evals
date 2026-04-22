# Artifact specification

This document describes every persisted artifact that `eval-ladder` produces
or consumes. Each artifact has a JSON schema under `schemas/`. Schemas are
versioned; an incompatible change requires a `schema_version` bump and a
changelog entry.

## Schema conventions

- Schema dialect: JSON Schema draft 2020-12.
- All schemas declare `$id`, `$schema`, and `schema_version`.
- All objects declare `additionalProperties: false` unless explicitly
  extending a family.
- Identifiers are UUIDv4 or UUIDv7 strings.
- Timestamps are RFC 3339 strings in UTC (suffix `Z`).
- Hashes are lowercase hex `sha256:` prefixed strings.

## Artifacts

### `BenchmarkTask` (`schemas/benchmark_task.schema.json`)

Normalized benchmark task. Produced by the ingest step for each adapter.

Required fields: `schema_version`, `benchmark_id`, `task_id`, `repo_name`,
`issue_id`, `issue_title`, `issue_text`, `base_commit`, `environment_ref`,
`official_test_entrypoint`, `language`, `labels`, `created_at`,
`source_url`. Optional: `gold_patch_ref`.

### `CandidateResolution` (`schemas/candidate_resolution.schema.json`)

The canonical evaluation unit. Produced by external agents, consumed by the
evaluator.

Required fields: `schema_version`, `candidate_id`, `benchmark_id`,
`task_id`, `agent_id`, `model_id`, `generation_mode`, `patch_format`,
`patch_ref`, `generation_metadata`, `submitted_at`. Optional:
`trajectory_ref`. The `generation_metadata` object includes `temperature`,
`tool_configuration`, `context_mode`, `repo_reproduction_used`, and
`random_seed` when available.

### `EvaluationResult` (`schemas/evaluation_result.schema.json`)

Per-level verdict. Produced by the evaluator per candidate per level.

Required fields: `schema_version`, `candidate_id`, `task_id`, `level`,
`status`, `primary_reason`, `secondary_reasons`, `artifacts`, `metrics`,
`started_at`, `finished_at`, `evaluator_version`.

`status` is one of `pass`, `fail`, `invalid`, `not_applicable`.

### `TraceEvent` (`schemas/trace_event.schema.json`)

Single append-only event. Emitted as one line of JSONL. Events carry a
`prev_event_hash` and `event_hash` so the stream is a hash chain.

Event types (stable strings): `RunStarted`, `ContainerPrepared`,
`PatchApplied`, `OfficialEvalStarted`, `OfficialEvalFinished`,
`StrengthenedEvalStarted`, `StrengthenedEvalFinished`,
`PolicyCheckStarted`, `PolicyViolationDetected`, `ProofCheckStarted`,
`ProofCheckFinished`, `RunFinished`.

### `RunManifest` (`schemas/run_manifest.schema.json`)

Single-run metadata. One per candidate. Produced at run start, finalized at
run end.

Required fields: `schema_version`, `run_id`, `candidate_id`, `task_id`,
`benchmark_id`, `levels_requested`, `evaluator_version`,
`container_metadata`, `started_at`, `finished_at`, `status`.

### `EvidenceBundle` (`schemas/evidence_bundle.schema.json`)

Reproducible bundle for a single candidate.

Mandatory contents: `candidate_resolution.json`, `run_manifest.json`,
`trace.jsonl`, `official_results.json`, plus any level-specific
results:

- L1: `l1_trusted_rerun_results.json`.
- L2: `strengthened_results.json` (aggregate L2 `EvaluationResult`)
  plus `strengthening_report.json` (per-validator sub-check
  breakdown).
- L3: `policy_results.json`. Contains the full `PolicyReport`
  (ordered findings with `PV_*` codes) plus a `run_context_summary`
  block pinning the inputs the engine judged. Every finding is also
  mirrored as a `PolicyViolationDetected` trace event on the
  run-level hash chain.
- L4: `proof_results.json`. Contains the full `ProofReport` emitted by
  `eval_ladder_lean::L4Extension`: the three-valued `LeanStatus`
  (`valid` / `invalid` / `not_applicable`), the stable uppercase
  `code` (obligation pass criterion, `L4_OBLIGATION_UNMET`,
  `L4_PROOF_CHECK_FAILED`, or `L4_OBLIGATION_NOT_APPLICABLE`), the
  resolved `ProofObligation` (or `null` when the task has no
  obligation in the manifest), the raw checker payload, and the
  extension's `{started_at, finished_at, duration_ms}` timings. Every
  L4 run also emits a `ProofCheckStarted` / `ProofCheckFinished` pair
  on the trace's hash chain so violations are auditable from the
  trace alone.

Every bundle also carries `patch.diff`, `container_metadata.json`,
`stdout.log`, `stderr.log`, and `artifact_hashes.json`.

Each bundle declares a `bundle_hash` (SHA-256 over canonical JSON plus file
digests) and a `files` array with `{path, sha256, bytes}` entries.

### Verification report (Milestone J)

`eval-ladder verify run-dir` emits `verify_report.json` (canonical
JSON) inside the verified run directory. Shape:

- `schema_version` (`u32`), `evaluator_version` (semver), `run_dir`
  (absolute path), `total`, `ok`, `invalid` counters.
- `entries`: array sorted by `bundle_name`, each carrying
  `bundle_name`, `bundle_dir`, `status` (`ok` | `invalid`),
  `bundle` / `trace` (`ok` | `invalid` | `not_applicable`),
  `bundle_hash` (when the bundle parsed successfully), and
  optional `error_code` (stable `VERIFY_*` string) plus `error`
  (human-readable). Stable codes are enumerated in
  `docs/operational_runbook.md#stable-error-codes`.

The report is byte-deterministic for identical inputs modulo
absolute-path fields (`run_dir`, `bundle_dir`), and is safe to
commit as part of a release artifact tarball.

### `ProofObligation` (`schemas/proof_obligation.schema.json`)

A curated obligation. Consumed by the Lean layer.

Required fields: per
[`docs/proof_subset_policy.md`](proof_subset_policy.md).

### Paper-export set (Milestone G)

Produced by `eval-ladder analyze paper-export`. A single output
directory contains every paper-ready analysis table, each emitted in
two formats so downstream consumers can pick whichever is most
convenient:

| File                                 | Shape            | Source function                                           |
|--------------------------------------|------------------|-----------------------------------------------------------|
| `score_descent.csv`                  | CSV (RFC 4180)   | `eval_ladder_analysis::score_descent::score_descent`      |
| `score_descent.json`                 | Canonical JSON   | same                                                      |
| `conditional_false_success.csv`      | CSV (RFC 4180)   | `eval_ladder_analysis::score_descent::conditional_false_success` |
| `conditional_false_success.json`     | Canonical JSON   | same                                                      |
| `rank_stability.csv`                 | CSV (RFC 4180)   | `eval_ladder_analysis::rank_stability::rank_stability`    |
| `rank_stability.json`                | Canonical JSON   | same                                                      |
| `taxonomy.csv`                       | CSV (RFC 4180)   | `eval_ladder_analysis::taxonomy::taxonomy_counts`         |
| `taxonomy.json`                      | Canonical JSON   | same                                                      |
| `static_vs_live.csv`                 | CSV (RFC 4180)   | `eval_ladder_analysis::static_vs_live::static_vs_live` (Milestone L) |
| `static_vs_live.json`                | Canonical JSON   | same                                                      |
| `manifest.json`                      | Canonical JSON   | `PaperExportManifest` (audit index, `schema_version = 2`) |

Every `*.csv` field is RFC-4180 quoted and floats use six-digit fixed
precision so diffs are stable across platforms. Every `*.json` file
goes through `eval_ladder_core::canonical_json` (sorted keys,
`\n` line endings, shortest round-trippable floats). The
`manifest.json` records `{path, sha256, bytes}` for every other file
along with `schema_version`, `evaluator_version`, and
`input_row_count`. Rerunning the command against the same
[`AnalysisInput`] must produce byte-identical files; the Milestone G
acceptance test in
`packages/rust/analysis/tests/milestone_g_acceptance.rs` pins this
invariant.

The manifest's `schema_version` is the single breaking-change knob for
this artifact. It is currently `2`:

- `1` (Milestone G): `score_descent`, `conditional_false_success`,
  `rank_stability`, `taxonomy`.
- `2` (Milestone L): all of the above plus `static_vs_live`. Readers
  keyed on the manifest hash must re-pin; readers keyed on filenames
  or `files[].path` remain forward-compatible because the manifest
  stays sorted.

#### `static_vs_live` (Milestone L)

One row per `(agent_id, level)` present in the loaded
[`AnalysisInput`]; rows are sorted by `(agent_id, level)`. Fields:

| Field                | Type               | Notes                                                                                                           |
|----------------------|--------------------|-----------------------------------------------------------------------------------------------------------------|
| `agent_id`           | string             | Agent identifier as it appears on `CandidateResolution`.                                                        |
| `level`              | `EvaluationLevel`  | Level short-code on CSV, enum on JSON.                                                                          |
| `static_passed`      | unsigned           | Pass count on the static arm (`BenchmarkId::SweBenchVerified`).                                                 |
| `static_evaluated`   | unsigned           | Evaluated count on the static arm (excluding `NotApplicable`).                                                  |
| `static_pass_rate`   | f64 or empty       | `static_passed / static_evaluated`; empty / `null` when the denominator is zero.                                |
| `live_passed`        | unsigned           | Pass count on the live arm (`BenchmarkId::SweBenchLive`).                                                       |
| `live_evaluated`     | unsigned           | Evaluated count on the live arm.                                                                                |
| `live_pass_rate`     | f64 or empty       | `live_passed / live_evaluated`.                                                                                 |
| `delta`              | f64 or empty       | `live_pass_rate - static_pass_rate`. Empty when either rate is missing. Negative values are the paper's claim.  |
| `ratio`              | f64 or empty       | `live_pass_rate / static_pass_rate`. Empty when either rate is missing **or** the static rate is zero.          |

The static arm is `{SweBenchVerified}` and the live arm is
`{SweBenchLive}`; `RustSweBench` and any future benchmarks are
excluded from this table so the paper claim stays unambiguous. See
`packages/rust/analysis/src/static_vs_live.rs` for the pinned contract.

### `BatchSummary` (Milestone H)

Produced by `eval-ladder evaluate batch` at
`<out>/batch_summary.json`. Schema (current `schema_version`: `1`):

| Field              | Type                                         | Notes                                                                                       |
|--------------------|----------------------------------------------|---------------------------------------------------------------------------------------------|
| `schema_version`   | integer                                      | `1` for this revision.                                                                      |
| `evaluator_version`| `EvaluatorVersion`                           | Transparent wrapper around the running evaluator semver.                                    |
| `levels`           | `EvaluationLevel[]`                          | Batch-wide `--levels` (canonical order).                                                    |
| `total_entries`    | unsigned                                     | Count of panel rows actually attempted.                                                     |
| `ok_entries`       | unsigned                                     | Count with sealed bundle.                                                                   |
| `invalid_entries`  | unsigned                                     | Count of `status: "invalid"` rows.                                                          |
| `entries`          | `BatchEntryRow[]`                            | Sorted by `bundle_name`.                                                                    |
| `started_at`       | RFC 3339 timestamp (optional)                | Omitted when `--deterministic-clock` is set.                                                |
| `finished_at`      | RFC 3339 timestamp (optional)                | Omitted when `--deterministic-clock` is set.                                                |

Per-entry row (`BatchEntryRow`):

| Field            | Type                                           | Notes                                                                                  |
|------------------|------------------------------------------------|----------------------------------------------------------------------------------------|
| `entry_id`       | string                                         | Panel-declared identifier (defaults to `bundle_name`).                                 |
| `bundle_name`    | string                                         | Output subdirectory name under `<out>/`.                                               |
| `task_path`      | string (display form)                          | Panel-declared task manifest path, for audit.                                          |
| `candidate_path` | string (display form)                          | Panel-declared candidate path, for audit.                                              |
| `status`         | `"ok" \| "invalid"`                            | See resilience rules below.                                                            |
| `levels`         | object                                         | Keys `l0..l4`; values `{status, primary_reason}`.                                      |
| `bundle_hash`    | `Sha256Digest` (optional)                      | Present only when the pipeline sealed a bundle.                                        |
| `error`          | string (optional)                              | Starts with `BATCH_LOAD_FAILED` or `BATCH_PIPELINE_FAILED` when `status` is `invalid`. |

The file is emitted through `eval_ladder_core::canonical_json`, and
the Milestone H determinism acceptance test
(`milestone_h_batch_summary_is_deterministic`) pins byte-identity of
the summary's content-bearing fields and of every per-entry
`bundle_hash` across reruns with a fixed clock and identical inputs.

## Hashing

All SHA-256 hashes are computed with the canonical serialization rules
described below.

### Canonical JSON

- UTF-8 encoding, no BOM.
- Unix newlines (`\n`).
- Object keys sorted lexicographically.
- No trailing whitespace.
- Floats serialized in shortest round-trippable form.

### File digests

- Digest: `sha256:<64-hex>`.
- Computed over the file's raw bytes on disk.
- Stored alongside `bytes` (file size) and `path` (POSIX relative to
  bundle root).

## Version discipline

- `schema_version` is an integer. Breaking changes bump it.
- The evaluator refuses to load an artifact whose `schema_version` it does
  not recognize.
- Migration scripts for old versions live under `packages/python/scripts/`.
