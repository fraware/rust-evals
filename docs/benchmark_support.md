# Benchmark support

The MVP supports three benchmarks. Each has a dedicated adapter under
`packages/rust/benchmarks` and a dedicated configuration file under
`configs/evaluator/`.

## Ingestion contract (shared across adapters)

Every adapter consumes a JSONL source (a single `.jsonl` file or a
directory of `*.jsonl` files sharing the upstream SWE-bench-family
schema) and emits one normalized `BenchmarkTask` manifest per task.

The ingest pipeline provides the following guarantees:

- Every manifest is validated against
  `schemas/benchmark_task.schema.json` before it is written.
- Manifests are serialized as canonical JSON (sorted keys, Unix
  newlines, one trailing newline) so two ingests of the same input
  produce byte-identical files.
- Writes are atomic (tempfile + rename). A crashed ingest cannot leave
  a truncated manifest on disk.
- Missing or malformed records are reported in the
  `IngestReport.diagnostics` field without aborting the whole batch.
  Adapters use stable, machine-readable error strings.

Run ingest from the CLI, for example:

```bash
eval-ladder ingest verified \
  --manifest configs/evaluator/verified.toml \
  --source   datasets/cache/verified/swe_bench_verified.jsonl \
  --out-dir  benchmarks/verified/manifests
```

Current repository ingest state:

- `benchmarks/verified/manifests/`: 500 tasks (SWE-bench Verified)
- `benchmarks/live/manifests/`: 500 tasks (SWE-bench-Live `verified` split)
- `benchmarks/rust/manifests/`: 239 tasks (Multi-SWE-Bench-Rust mirror)

## SWE-Bench Verified

- Source: curated static subset of SWE-bench (`princeton-nlp/SWE-bench_Verified`).
- Language: Python.
- Adapters (interchangeable):
  - `eval_ladder_benchmarks::verified` (Rust; first-class).
  - `benchmark_compat` Python CLI (`eval-ladder-py normalize-swe-bench`;
    Milestone I). Emits byte-identical manifests to the Rust adapter
    and is the recommended path when the upstream tooling is Python-
    native. Cross-language parity is pinned by
    `tests/integration/tests/python_round_trip.rs::`
    `milestone_i_python_emitted_benchmark_task_deserializes_in_rust`.
- Evaluator config: `configs/evaluator/verified.toml`.
- Task manifests: `benchmarks/verified/manifests/`.

The Verified adapter:
- Ingests task metadata from the upstream JSONL release.
- Normalizes issue text; the first non-empty line of the problem
  statement becomes `issue_title` (truncated on a character boundary).
- Pins environment assets per task using the standard harness image
  name scheme: `swebench/sweb.eval.x86_64.<instance_id>:latest`.
- Wraps the benchmark's official validation entrypoint (the
  `swebench.harness.run_evaluation` module).
- Carries upstream metadata in `labels` as stable `key:value` strings
  (`upstream_version:4.3`, `fail_to_pass_count:3`,
  `env_setup_commit:<sha>`, ...).
- Emits immutable task manifests that conform to
  `schemas/benchmark_task.schema.json`.

## SWE-bench-Live

- Source: SWE-bench-Live release (1,319 tasks, 93 repositories).
- Language: Python.
- Adapter: `eval_ladder_benchmarks::live`.
- Evaluator config: `configs/evaluator/live.toml`.
- Task manifests: `benchmarks/live/manifests/`.

The current ingest snapshot intentionally tracks the public
`SWE-bench-Live/SWE-bench-Live` `verified` split mirror used by the
bootstrap scripts, not the full 1,319-task aggregate.

The Live adapter preserves:
- Task timestamp (the upstream `created_at` becomes the manifest
  `created_at`, giving analysis code a stable freshness signal).
- Repository freshness metadata (emitted as a `repo:<owner>/<name>`
  label).
- The per-task Docker image mapping declared by SWE-bench-Live
  (the `docker_image` field becomes `environment_ref` verbatim).
- Any benchmark-specific execution assumptions.

`docker_image` is **required** for Live ingest. Records missing it are
skipped with a stable `EnvironmentUnresolved` diagnostic; Live cannot
synthesize a per-task image reference and pretending otherwise would
silently corrupt downstream runner state.

Live tasks retain a distinct provenance model. They are not coerced into
the Verified metadata shape when doing so would hide freshness. The
`static_vs_live` analysis output depends on this separation.

## Rust-native (Rust-SWE-bench)

- Source: Rust-SWE-bench release (500 tasks, 34 repositories).
- Language: Rust.
- Adapter: `eval_ladder_benchmarks::rust_native`.
- Evaluator config: `configs/evaluator/rust.toml`.
- Task manifests: `benchmarks/rust/manifests/`.

The repository currently ingests the public HF mirror with 239 exported
records (`r1v3r/multi_SWE_Bench_Rust`), which is sufficient for the
Rust-native pilot release and leaves room to grow to the full upstream
release set as mirrors converge.

The Rust adapter handles:
- Cargo workspace resolution. `official_test_entrypoint` is
  `cargo test --workspace --locked` so that evaluator runs refuse to
  mutate `Cargo.lock` (policy layer L3 enforces this).
- `cargo test`, `cargo check`, and reproduction hooks.
- Rust-specific file filtering (`Cargo.lock`, `target/`, generated files).
- Lockfile and dependency-edit policy.
- Rust Edition handling and toolchain pinning.
- Environment reference resolution: if the upstream record carries a
  `docker_image`, it is honored verbatim; otherwise the adapter emits
  a deterministic content-addressed descriptor of the form
  `cargo://<owner>/<repo>@<short_sha>` that the runner resolves
  against a checked-out workspace at L0/L1 time.

Rust is a first-class benchmark here because Rust-SWE-bench highlights
repository-wide understanding and strict type/trait semantics as central
difficulty sources; those are precisely the dimensions where weak-test
inflation is most visible.

## Adding a new benchmark

1. Implement `BenchmarkAdapter` in a new module under
   `packages/rust/benchmarks/src/`. Reuse the shared helpers in
   `benchmarks::raw`, `benchmarks::normalize`, `benchmarks::schema`, and
   `benchmarks::writer`; a new adapter typically only has to provide a
   `record_to_task` function and thread the usual `ingest` loop.
2. Add ingestion tests under `packages/rust/benchmarks/tests/` with
   hand-written JSONL fixtures under `tests/fixtures/benchmarks/`.
3. Ship an evaluator config under `configs/evaluator/<name>.toml`.
4. Ship a manifest directory under `benchmarks/<name>/manifests/`.
5. Register the new benchmark id in `eval_ladder_core::BenchmarkId`
   and in the benchmark_task JSON Schema enum.
6. Update this document with the adapter's scope and invariants.

The adapter boundary is strict: benchmark-specific logic never leaks into
`eval-ladder-core` or the CLI dispatch.
