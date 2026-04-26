# Operational runbook

This runbook covers day-to-day operation of `eval-ladder`: local development,
batch evaluation, CI tiers, and release hygiene. For a map of all technical
docs (scope, ladder, architecture, evidence gates), see [`docs/README.md`](README.md).

## Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml` (1.86 at time of writing).
- Python 3.10+.
- An OCI-compatible container runtime (Docker Engine, Podman, or similar).
- Optional: Lean 4 via `leanprover/lean4:v4.15.0` for L4.
- Optional: `just` for the task recipes in `justfile`.

## Local development

Bootstrap:

```bash
rustup show                                  # confirms pinned toolchain
cargo build --workspace --all-targets
python -m pip install -e ".[dev]"
```

Common flows:

```bash
just ci-tier1            # fmt-check, clippy, test, schema validation
just ci-tier2            # tier1 + Python lint and tests
cargo run --bin eval-ladder -- --help
cargo run --bin eval-ladder -- schema validate
```

## Per-candidate evaluation (Milestone C)

Milestone C ships the full L0 (official) + L1 (trusted rerun) pipeline
as `eval-ladder evaluate candidate`. Each invocation produces:

- a hash-chained `trace.jsonl`,
- a sealed evidence bundle at `--bundle-dir`,
- two `EvaluationResult` JSON documents inside the bundle
  (`official_results.json` for L0, `l1_trusted_rerun_results.json`
  for L1),
- a single-line JSON status report on stdout summarising
  `bundle_hash`, `run_id`, `bundle_id`, and per-level status.

### Inputs

| Flag | Meaning |
| --- | --- |
| `--task` | Normalized benchmark task manifest (output of `ingest`). |
| `--candidate` | `CandidateResolution` JSON. |
| `--patch` | Candidate patch. Empty file is a valid no-op. |
| `--workspace-template` | Unpatched checkout at `base_commit`. Never mutated. |
| `--bundle-dir` | Destination for the evidence bundle. Must be empty or absent. |
| `--config` | Evaluator configuration TOML. |
| `--deterministic-clock` | Use a fixed clock so reruns yield identical bundle hashes. |
| `--seed-tag` | Identity-seed label. Change to separate otherwise-identical reruns. |
| `--levels` | Comma-separated ladder levels to run. Defaults to `L0,L1`; add `L2` to enable strengthening. |
| `--strengthening-spec` | Path to the task-level L2 `StrengtheningSpec` JSON. Required when `--levels` includes `L2`. |
| `--strengthening-mode` | `tests_only`, `tests_plus_diff`, `tests_plus_regression`, or `full_l2`. Defaults to `full_l2`. |
| `--oracle-patch` | Oracle patch bytes for the differential validator. Required for `tests_plus_diff` or `full_l2` runs that exercise `differential_behavior`. |
| `--policy` | Path to an L3 policy TOML (see `configs/policy/default_policy.toml`). Required when `--levels` includes `L3`. |
| `--network-accessed` | Inform L3 that the container engine observed outbound network activity. Defaults to `false`; keep it off for `LocalProcessEngine`. |
| `--obligations` | Path to an L4 obligation manifest (JSONL; one `ProofObligation` per line). Required when `--levels` includes `L4`. |
| `--lean-root` | Path to the Lean project root (typically `packages/lean/EvalLadder`). Passed as the checker's `cwd`. Required when `--levels` includes `L4`. |

### Determinism contract

Running `eval-ladder evaluate candidate` twice with:

1. the same `--task`, `--candidate`, and `--patch` bytes,
2. the same `--workspace-template` contents,
3. `--deterministic-clock` on both runs,
4. the same `--seed-tag`,

produces byte-identical `trace.jsonl` and identical `bundle_hash`. This
is the Milestone C acceptance invariant and is pinned by the
`packages/rust/runner/tests/pipeline_acceptance.rs` integration test
plus the `pipeline::tests::bundle_hash_is_stable_across_reruns` unit
test.

Production runs usually omit `--deterministic-clock`; timestamps then
follow wall-clock time. The evidence-bundle hash still covers the
`created_at` field so diverging timestamps still diverge the hash; the
flag exists specifically for rerun-determinism audits.

## L2 strengthening (Milestone D)

When `--levels` includes `L2`, the pipeline runs the
`eval-ladder-strengthening` extension after L1 and emits two extra
bundle artifacts:

- `strengthened_results.json` - `EvaluationResult` for L2 (aggregate
  pass/fail and primary failure reason).
- `strengthening_report.json` - per-validator breakdown with every
  sub-check verdict, exit code, and truncated stderr. This is the
  source the analysis layer reads when attributing L2 score drops to
  specific sub-checks.

### Strengthening spec

The spec is a JSON document of type
`eval_ladder_strengthening::StrengtheningSpec`. A minimal spec that
enables only augmented tests looks like:

```json
{
  "schema_version": 1,
  "augmented": {
    "commands": [
      { "id": "edge_case_1", "command": ["pytest", "-q", "tests/edge_case_1.py"] }
    ]
  },
  "regression": { "commands": [] },
  "differential": null,
  "property_fuzz": null
}
```

A `full_l2` spec additionally carries a `differential` block with an
`oracle_patch_ref` and a list of observable commands. The actual
oracle-patch bytes are passed at run time via `--oracle-patch`; the
`oracle_patch_ref` is only used for provenance metadata inside the
bundle.

### Determinism contract (L2)

Two `evaluate candidate --levels L0,L1,L2 --deterministic-clock` runs
with the same task, candidate, patch bytes, workspace template,
strengthening spec, strengthening mode, and (if applicable) oracle
patch bytes produce byte-identical `trace.jsonl` and an identical
`bundle_hash`. This extends the Milestone C invariant and is pinned
by `packages/rust/strengthening/tests/milestone_d_acceptance.rs::milestone_d_l2_reruns_are_deterministic`.

### Failure codes

L2 aggregate `primary_reason` values:

- `L2_AUG_TESTS_FAIL` - one or more augmented-unit-tests sub-checks
  failed.
- `L2_REGRESSION_FAIL` - one or more regression sub-checks failed.
- `L2_DIFF_BEHAVIOR` - at least one observable diverged between the
  candidate-patched workspace and the oracle-patched workspace.
- `L2_ORACLE_UNAVAILABLE` - the spec declares a differential block
  but no oracle patch was supplied; differential is reported as
  `NotApplicable`. Does not, on its own, fail L2.

## L3 policy (Milestone E)

When `--levels` includes `L3`, the pipeline runs the
`eval-ladder-policy` extension after L2 (or after L1 when L2 is
absent) and emits one additional bundle artifact:

- `policy_results.json` - full [`PolicyReport`] with ordered
  [`PolicyFinding`] list, plus a `run_context_summary` block pinning
  the inputs the engine judged (commands, modified files, trace
  events seen, and the static observation flags).

The L3 `EvaluationResult` carries:

- `status = pass` and `primary_reason = "PASS"` when the finding list
  is empty.
- `status = fail` and `primary_reason = <first PV_* code>` otherwise,
  with subsequent codes in `secondary_reasons`.

Every finding is also mirrored as a `PolicyViolationDetected` trace
event on the run-level hash chain, so the trace alone is
self-describing even if the JSON artifact is lost.

### Policy document

The policy is a declarative TOML document consumed by
`eval_ladder_policy::Policy::from_path`. A minimal permissive policy
looks like:

```toml
name = "demo_policy"
network_mode = "disabled"
requires_reproducible_seed = true
max_modified_files = 8

allowed_commands = ["cargo", "python", "pytest", "bash", "sh", "git"]
forbidden_commands = ["curl", "wget", "ssh", "sudo"]

allowed_edit_globs = ["src/**", "tests/**"]
forbidden_edit_globs = [".github/**", "secrets/**"]

required_trace_events = [
    "RunStarted",
    "PatchApplied",
    "OfficialEvalStarted",
    "OfficialEvalFinished",
    "RunFinished",
]
```

The shipped default lives at `configs/policy/default_policy.toml`.

### Determinism contract (L3)

Two `evaluate candidate --levels L0,L1,L2,L3 --deterministic-clock`
runs with the same task, candidate, patch bytes, workspace template,
strengthening inputs, policy document, and network-observation flag
produce byte-identical `trace.jsonl` and an identical `bundle_hash`.
This is pinned by
`packages/rust/policy/tests/milestone_e_acceptance.rs::milestone_e_l3_reruns_are_deterministic`.

### Failure codes

L3 aggregate `primary_reason` values match `PolicyViolation::as_str`:

- `PV_NET_ACCESS` - outbound network activity under `network_mode = "disabled"`
  or `"host_allowlist"`.
- `PV_FORBIDDEN_CMD` - the run invoked a command in `forbidden_commands`
  or a command outside a non-empty `allowed_commands`.
- `PV_EDIT_SCOPE` - the patch modified a path matching
  `forbidden_edit_globs` or outside a non-empty `allowed_edit_globs`.
- `PV_FILE_COUNT_EXCEEDED` - the patch modifies more files than
  `max_modified_files`.
- `PV_DEPENDENCY_EDIT` - the patch modifies a known lockfile while
  `allow_dependency_lockfile_edits = false`.
- `PV_GENERATED_TEST_DISALLOWED` - a `generated_tests/` directory is
  present in the bundle while `allow_generated_tests = false`.
- `PV_ENV_NONDETERMINISTIC` - the candidate did not declare a
  reproducible seed while `requires_reproducible_seed = true`, or the
  trusted rerun disagreed with the official run.
- `PV_TRACE_INCOMPLETE` - a required trace event (other than the
  pipeline-guaranteed `RunFinished`) did not appear.

## L4 proof subset (Milestone F)

When `--levels` includes `L4`, the pipeline runs the
`eval-ladder-lean` extension after L3 (or after the lowest available
rung when L3 was not requested) and emits one additional bundle
artifact:

- `proof_results.json` - full [`ProofReport`] with the three-valued
  `LeanStatus` (`valid` / `invalid` / `not_applicable`), the stable
  uppercase `code`, the resolved `ProofObligation` (or `null` when
  the task has no obligation in the manifest), the raw checker
  payload, and `{started_at, finished_at, duration_ms}` timings.

The L4 `EvaluationResult` carries:

- `status = pass` and `primary_reason = <obligation pass_criterion>`
  when the checker returned `LeanStatus::Valid` with the expected
  code.
- `status = fail` and `primary_reason = L4_OBLIGATION_UNMET` when the
  checker returned `LeanStatus::Valid` with an unexpected code or
  `LeanStatus::Invalid` without a more specific code. When the
  harness itself failed, `primary_reason = L4_PROOF_CHECK_FAILED` and
  the error kind (`spawn` / `parse` / `exited` / `io`) is captured
  inside the `proof_results.json` payload.
- `status = not_applicable` and
  `primary_reason = L4_OBLIGATION_NOT_APPLICABLE` when the task has
  no obligation in the manifest.

Every L4 run emits a `ProofCheckStarted` and a `ProofCheckFinished`
event on the trace's hash chain so the verdict is auditable from the
trace alone.

### Obligation manifest

The manifest is a JSONL document; each line is one
`ProofObligation` (schema `schemas/proof_obligation.schema.json`).
Blank lines and comments whose first non-whitespace character is `#`
are skipped so reviewers can annotate entries in PRs. A minimal
manifest entry looks like:

```json
{"schema_version":1,"obligation_id":"obl.example.reflexive","task_id":"example__task-1","property_name":"identity_reflexive","property_type":"no_panic_or_invalid_state","target_files":["src/lib.rs"],"informal_statement":"equality is reflexive on Nat.","formal_statement_ref":"EvalLadder/Obligations/Example/Task1.lean","proof_checker":{"command":"lake","args":["env","lean","EvalLadder/Obligations/Example/Task1.lean"]},"pass_criterion":"L4_OBLIGATION_MET","difficulty":{"reviewer_hours":0.5},"selection_rationale":{"one_or_two_sentence_property":true,"local_scope":true,"matters_to_issue":true,"strictly_stronger_than_tests":true,"bounded_effort":true}}
```

Selection discipline is documented in `docs/proof_subset_policy.md`
and is enforced by review (not the loader). Duplicate `task_id`
entries are rejected at load time.

### Checker contract

Production runs spawn the command declared by each obligation via
`ExternalProcessChecker` with `cwd = --lean-root`. The checker must
print a single JSON object on stdout of the shape:

```json
{ "status": "valid|invalid|not_applicable",
  "code": "L4_OBLIGATION_MET|L4_OBLIGATION_UNMET|...",
  "message": "free-form text",
  "payload": { ... } }
```

Non-zero exit codes are tolerated as long as stdout carries a valid
outcome; that lets checkers communicate `Invalid` through a
canonical exit code without losing structure. Checkers that exit
non-zero without a parseable outcome produce `L4_PROOF_CHECK_FAILED`
and the captured stderr is embedded in `proof_results.json`.

### Determinism contract (L4)

Two `evaluate candidate --levels L0,L1,L2,L3,L4 --deterministic-clock`
runs with the same task, candidate, patch bytes, workspace template,
strengthening inputs, policy document, network-observation flag,
obligation manifest, and a deterministic checker (the in-tree
`ScriptedChecker` is the canonical audit tool) produce byte-identical
`trace.jsonl` and an identical `bundle_hash`. Pinned by
`packages/rust/lean/tests/milestone_f_acceptance.rs::l4_reruns_are_deterministic`.

Production Lean checkers inherit reproducibility from the
`packages/lean/EvalLadder/lean-toolchain` pin; audits that exercise
`lake` directly use the ignored integration test
`l4_external_checker_against_lake_binary_ok`.

### Failure codes (L4)

- `L4_OBLIGATION_UNMET` - the checker returned `Valid` but with a
  code that disagrees with the obligation's `pass_criterion`, or
  returned `Invalid` with its own code.
- `L4_PROOF_CHECK_FAILED` - harness failure: checker process failed
  to spawn, exited non-zero without a parseable outcome, or produced
  unparseable stdout.
- `L4_OBLIGATION_NOT_APPLICABLE` - no obligation registered for the
  task (the empty-manifest case).
- `L4_EXTRACTION_FAILED` - reserved for future use by checkers that
  perform their own extraction step before invoking Lean.

### Batch: `prove-subset`

`eval-ladder prove-subset` runs the L4 checker over an existing
directory of evidence bundles:

```bash
cargo run --bin eval-ladder -- prove-subset \
  --subset      datasets/derived/proof_subset/manifest.jsonl \
  --candidate-dir runs/released/agent_panel_v1/results/ \
  --lean-root   packages/lean/EvalLadder \
  --summary     runs/released/agent_panel_v1/l4_summary.json
```

By default, bundles that already carry `proof_results.json` are
treated as sealed and skipped. Pass `--overwrite` to replace them.
The `--summary` file is a deterministic JSON listing one row per
bundle in sorted-path order.

## Paper pipeline (Milestone G)

Milestone G converts a directory of sealed evidence bundles into the
paper's analysis tables. All subcommands are pure - they never re-run
a candidate - and every `*_results.json` in every bundle is treated as
authoritative.

### Input resolution

Every `analyze` subcommand accepts a single `--run-dir` argument. The
resolver prefers, in order:

1. `<run-dir>/analysis_input.json` (explicit, for curated datasets and
   regression tests).
2. A directory of per-candidate bundles. The loader walks it
   lexicographically via
   `eval_ladder_analysis::load_bundle_dir` and fails with a structured
   error if any bundle is missing `candidate_resolution.json`, has no
   `*_results.json`, or reports a `candidate_id` / `task_id` that
   disagrees with its own `candidate_resolution.json`.

### Subcommands

```bash
# Per-table exports (CSV by default; --out and --json-out are optional).
cargo run --bin eval-ladder -- analyze score-descent             --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze conditional-false-success --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze rank-stability            --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze taxonomy                  --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze static-vs-live            --run-dir runs/released/agent_panel_v1/results/

# One-shot paper export: writes every table + manifest.json into a
# dedicated directory and prints the manifest to stdout.
cargo run --bin eval-ladder -- analyze paper-export \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out-dir paper/exports/agent_panel_v1/
```

### Determinism contract

`analyze paper-export` is byte-deterministic for any fixed
`AnalysisInput`:

- Floats are written with six-digit fixed precision.
- JSON goes through `eval_ladder_core::canonical_json` (sorted keys,
  `\n` line endings, shortest round-trippable floats).
- `manifest.json` records `{path, sha256, bytes}` for every other
  file plus `schema_version`, `evaluator_version`, and
  `input_row_count`.

The invariant is pinned by
`packages/rust/analysis/tests/milestone_g_acceptance.rs`, which runs
the full `load_bundle_dir` -> `write_paper_exports` pipeline twice and
requires byte-identical outputs.

### Static-vs-live comparison (Milestone L)

Milestone L adds a fifth paper-export pair,
`static_vs_live.{csv,json}`, and a matching one-shot subcommand
`analyze static-vs-live`. It is the shipped implementation of the
paper's "overstatement" claim.

```bash
# One-shot static-vs-live table to stdout as CSV.
cargo run --bin eval-ladder -- analyze static-vs-live \
  --run-dir runs/released/agent_panel_v1/results/

# Write the canonical JSON sibling alongside the CSV.
cargo run --bin eval-ladder -- analyze static-vs-live \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out paper/exports/agent_panel_v1/static_vs_live.csv \
  --json-out paper/exports/agent_panel_v1/static_vs_live.json
```

Contract (see `docs/artifact_spec.md` for the field-level schema):

- One row per `(agent_id, level)` with data on either the static
  arm (`SweBenchVerified`) or the live arm (`SweBenchLive`).
- `delta = live_pass_rate - static_pass_rate`; empty when either
  side is undefined.
- `ratio = live_pass_rate / static_pass_rate`; empty when either
  side is undefined or the static rate is zero.
- `RustSweBench` is intentionally excluded from this table; it is a
  separate paper surface.

`analyze paper-export` bumped `PAPER_EXPORT_SCHEMA_VERSION` from `1`
to `2` for static-vs-live, and from `2` to `3` for explicit
`analysis_mode` provenance (`raw` vs `cumulative`) in
`manifest.json`. Readers keyed on the manifest hash re-pin
intentionally.
Filename-based readers remain forward-compatible. Determinism is
pinned by
`packages/rust/analysis/tests/milestone_l_acceptance.rs`.

### Raw vs cumulative headline semantics (P0)

Analysis now supports two semantics:

- `raw`: preserves evaluator contract exactly; each level is independent.
- `cumulative`: for headline reporting only, upper-level pass requires
  lower-level pass prerequisites.

CLI usage:

```bash
# Raw (appendix/debug).
eval-ladder analyze score-descent --run-dir <run_dir> --analysis-mode raw

# Cumulative (headline).
eval-ladder analyze score-descent --run-dir <run_dir> --analysis-mode cumulative

# paper-export defaults to cumulative; override to raw explicitly when needed.
eval-ladder analyze paper-export --run-dir <run_dir> --out-dir <out_dir>
eval-ladder analyze paper-export --run-dir <run_dir> --out-dir <out_dir> --analysis-mode raw
```

Publication gate commands for the NeurIPS evidence tranches (strict vs
`--gate-profile release`) are centralized in
`docs/evidence_empirical_status.md`.

## Batch evaluation (Milestone H)

`eval-ladder evaluate batch` drives the full L0-L4 pipeline over a
panel JSONL, producing one sealed evidence bundle per entry plus a
deterministic `batch_summary.json` at the root of `--out`.

### Panel schema

One JSON object per non-blank, non-`#` line. Per-entry paths are
resolved relative to the directory containing the panel file when
they are not absolute.

```json
{
  "task": "benchmarks/verified/manifests/task_001.json",
  "candidate": "candidates/agent_a/task_001.json",
  "patch": "patches/agent_a/task_001.diff",
  "workspace_template": "/var/eval-ladder/snapshots/task_001/",
  "bundle_name": "agent_a__task_001",
  "entry_id": "agent_a/task_001"
}
```

`bundle_name` and `entry_id` are optional. `bundle_name` defaults to
the candidate's stringified UUID; `entry_id` defaults to
`bundle_name`. Unknown fields are rejected (`serde(deny_unknown_fields)`).

### Full panel drive

```bash
# 1. Ingest the benchmark.
cargo run --bin eval-ladder -- ingest verified \
  --manifest configs/evaluator/verified.toml \
  --source datasets/public_links/verified.jsonl

# 2. Drive the entire panel.
cargo run --bin eval-ladder -- evaluate batch \
  --input runs/released/agent_panel_v1/panel.jsonl \
  --levels L0,L1,L2,L3,L4 \
  --config configs/evaluator/verified.toml \
  --out runs/released/agent_panel_v1/results/ \
  --strengthening-spec configs/strengthening/default.json \
  --policy configs/policy/default_policy.toml \
  --obligations datasets/derived/proof_subset/manifest.jsonl \
  --lean-root packages/lean/EvalLadder

# 3. (Optional) Iterate only the proof subset with the batch-wide
#    Lean checker. prove-subset reuses the bundles written in step 2.
cargo run --bin eval-ladder -- prove-subset \
  --subset datasets/derived/proof_subset/manifest.jsonl \
  --candidate-dir runs/released/agent_panel_v1/results/ \
  --lean-root packages/lean/EvalLadder

# 4. Emit paper outputs (Milestone G).
cargo run --bin eval-ladder -- analyze paper-export \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out-dir paper/exports/agent_panel_v1/
```

### Wall-clock optimizations (long batches)

Use the **release** CLI driver. Long-batch `just` recipes depend on
`just eval-ladder-cli-release` and run `target/release/eval-ladder` (or
`eval-ladder.exe` on Windows) directly so each batch avoids `cargo run` startup.
For one-off invocations you can still use
`cargo run -p eval-ladder-cli --release -- evaluate …`.

| Knob | Effect |
| --- | --- |
| `--rust-target-cache-root <dir>` | Sets `CARGO_TARGET_DIR` for Rust-heavy rows (Verified smart reuse, Rust-SWE-bench). Point at a **stable directory on fast local disk** (for example `runs/released/.eval_ladder_cargo_cache`) so repeated compiles reuse artifacts across batches. |
| `--dedupe-workloads` | Default **on**; skips redundant Docker work when task+patch+candidate bytes match another row (multi-agent safe after the candidate-aware workload key). |
| `--resume` | Skips entries whose bundle dirs already completed; safe for interrupted runs. |
| `--jobs N` | Overlaps **different** panel rows (`N` of 2–4 on a strong host). Reduces wall time when Docker and disk keep up; drop to `1` if the engine thrashes. |
| `--adaptive-timeouts` + `--short-timeout-secs` | After cheap failure patterns in a prior summary, later rows use shorter per-exec timeouts so bad harness rows fail faster than `--timeout-secs`. |
| Fewer `--levels` | For **iteration only**, run the minimum ladder you need (Verified headline gate uses `L0,L1,L3`; skip `L4` until you need proof rows). Rust policy iteration: `--track fast` runs **L3,L4 only**; seal with a full `L0,L1,L3,L4` pass when semantics are stable. |
| Smaller panel | Shrink `panel.jsonl` or tighten `preflight_verified_selectors.py --strict` / `filter_panel_upstream_resolved.py` while debugging harness clusters. |
| Image prewarm | Run `python ci/scripts/prewarm_panel_images.py --panel <panel.jsonl>` (optional `--parallel N`). Uses the **same SWE-bench image name candidates** as the Rust Docker engine (legacy `org__repo` then `org_1776_repo`). Local hits skip `docker pull`. Pull failures are **non-fatal by default** (compact stderr unless `--strict-pulls`). `cargo://…` and other non-OCI schemes are skipped. |

**`just` recipes** (from the repo root; see `just --list`):

- **`just verified-batch-optimized-prewarmed <panel.jsonl> <out_dir> [jobs] [cache] [prewarm_parallel]`** — pull images for that panel, then Verified `L0,L1,L3` batch (recommended default for wall clock).
- `just verified-batch-optimized <panel.jsonl> <out_dir> [jobs] [cache]` — same batch without a preceding pull (use if images are already local).
- **`just live-batch-optimized-prewarmed <out_dir> [jobs] [prewarm_parallel]`** — prewarm `runs/released/live_panel_v1/panel.jsonl`, then Live batch.
- `just live-batch-optimized <out_dir> [jobs]`
- **`just rust-proof-batch-fast-prewarmed <out_dir> [prewarm_parallel]`** / **`just rust-proof-batch-seal-prewarmed <out_dir> [jobs] [cache] [prewarm_parallel]`** — Rust proof subset panel, then fast or seal batch.
- `just rust-proof-batch-fast <out_dir>` — fast L3/L4 iteration
- `just rust-proof-batch-seal <out_dir> [jobs] [cache]` — full ladder for sealing
- `just prewarm-panel <panel.jsonl> [parallel]` — pull only (default parallel **4**; best-effort exit code).
- `just prewarm-panel-strict <panel.jsonl> [parallel]` — same, but `--strict-pulls` (fail if any pull fails).

Also keep Docker Desktop CPU/memory limits reasonable, and place `--out` on
local SSD (not a network filesystem).

#### Full commands (copy-paste)

From the repository root, after a toolchain or dependency change, build the driver once (optional on first batch: the recipes below already depend on `eval-ladder-cli-release`):

```powershell
cd C:\path\to\rust-evals
just eval-ladder-cli-release
```

**Verified panel (prewarm + batch, recommended):**

```powershell
just verified-batch-optimized-prewarmed runs\released\agent_panel_v3_r1\panel_preflight_clean.jsonl runs\released\agent_panel_v3_r1\results_opt
```

That recipe already runs prewarm first; do not also run `python ci/scripts/prewarm_panel_images.py` in the same workflow unless you want a standalone pull step.

**Verified panel (batch only, images already local):**

```powershell
just verified-batch-optimized runs\released\agent_panel_v3_r1\panel_preflight_clean.jsonl runs\released\agent_panel_v3_r1\results_opt
```

**Live panel:**

```powershell
just live-batch-optimized-prewarmed runs\released\live_panel_v1\results_opt
```

**Rust proof subset — fast iteration then seal:**

```powershell
just rust-proof-batch-fast-prewarmed runs\released\rust_proof_subset_v1\results_fast
just rust-proof-batch-seal-prewarmed runs\released\rust_proof_subset_v1\results_seal
```

**Prewarm only (custom `evaluate batch` afterward):**

```powershell
python ci/scripts/prewarm_panel_images.py --panel runs\released\live_panel_v1\panel.jsonl --parallel 4
```

**Bash / WSL / Linux** (same flags; use forward slashes):

```bash
cd /path/to/rust-evals
just eval-ladder-cli-release
just verified-batch-optimized-prewarmed runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl runs/released/agent_panel_v3_r1/results_opt
just live-batch-optimized-prewarmed runs/released/live_panel_v1/results_opt
just rust-proof-batch-fast-prewarmed runs/released/rust_proof_subset_v1/results_fast
just rust-proof-batch-seal-prewarmed runs/released/rust_proof_subset_v1/results_seal
```

**Without `just`** (equivalent to `verified-batch-optimized` after `cargo build -p eval-ladder-cli --release`; Windows: `target\release\eval-ladder.exe`):

```bash
cargo build -q -p eval-ladder-cli --release
./target/release/eval-ladder evaluate batch \
  --input runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl \
  --config configs/evaluator/verified.toml \
  --levels L0,L1,L3 \
  --policy configs/policy/default_policy.toml \
  --out runs/released/agent_panel_v3_r1/results_opt \
  --timeout-secs 3600 --short-timeout-secs 900 \
  --adaptive-timeouts --resume --jobs 2 \
  --l1-strategy smart_rust_reuse \
  --rust-target-cache-root runs/released/.eval_ladder_cargo_cache \
  --seed-tag verified-batch-opt --deterministic-clock
```

### Resilience contract

One bad entry never aborts the batch. Any recoverable error between
panel-line load and pipeline dispatch becomes a row with
`status: "invalid"` and an `error` field whose code starts with
`BATCH_LOAD_FAILED` or `BATCH_PIPELINE_FAILED`. The CLI exits
non-zero only when the panel itself is unreadable or when *every*
entry failed.

### Determinism contract

With `--deterministic-clock`, the batch output is byte-deterministic:

- Each per-entry bundle is sealed by the same deterministic pipeline
  used by `evaluate candidate`, so bundle hashes are stable across
  reruns.
- `batch_summary.json` is produced by
  `eval_ladder_core::canonical_json` (sorted keys, `\n` line endings,
  shortest round-trippable floats). Entries are sorted by
  `bundle_name`.
- Wall-clock `started_at`/`finished_at` fields are omitted in
  deterministic mode so that the summary has no time-dependent bytes.

The invariant is pinned by the `milestone_h_batch_summary_is_deterministic`
test in `packages/rust/cli/src/commands/batch.rs`, which runs the
end-to-end batch twice on a 3-entry panel and asserts matching
bundle hashes and matching summary content.

Single-candidate drive remains available via `evaluate candidate`
when needed for debugging.

### Released Rust-native pilot path

`runs/released/rust_pilot_v1/` is the shipped Docker-free pilot run.
It uses the host Rust toolchain through `LocalProcessEngine`:

```powershell
.\target\debug\eval-ladder.exe evaluate batch `
  --input "runs/released/rust_pilot_v1/panel.jsonl" `
  --config "configs/evaluator/rust.toml" `
  --levels L0,L1,L3,L4 `
  --policy "configs/policy/rust_pilot.toml" `
  --obligations "datasets/derived/proof_subset/manifest.jsonl" `
  --lean-root "packages/lean/EvalLadder" `
  --out "runs/released/rust_pilot_v1/results" `
  --timeout-secs 3600 `
  --deterministic-clock
```

Released summary (`batch_summary.json`):

- L0: `L0_OFFICIAL_TIMEOUT`
- L1: `L1_HARNESS_ERROR`
- L3: `PASS`
- L4: `L4_OBLIGATION_MET`

Released integrity check (`verify_report.json`):

- `1 ok / 0 invalid` (`trace: ok`, bundle hash sealed)

## SWE-bench Verified normalization (Milestone I)

The Python compat layer ships a working ingestor for SWE-bench
Verified release manifests. The CLI is installed as
`eval-ladder-py` (from the repo-root `pyproject.toml`) and is also
reachable via `python -m benchmark_compat.cli`.

```bash
# 1. Install the Python layer in the active environment.
python -m pip install -e ".[dev]"

# 2. Normalize a SWE-bench Verified JSONL manifest into per-task
#    BenchmarkTask files accepted by the Rust evaluator.
eval-ladder-py normalize-swe-bench \
  --source datasets/public_links/verified.jsonl \
  --out-dir benchmarks/verified/manifests/

# 3. (Optional) Abort on the first malformed record instead of
#    continuing past bad entries.
eval-ladder-py normalize-swe-bench \
  --source datasets/public_links/verified.jsonl \
  --out-dir benchmarks/verified/manifests/ \
  --strict
```

### Output contract

- One `<task_id>.json` per input record under `--out-dir`.
- Bytes are canonical JSON: sorted keys, UTF-8, shortest
  round-trippable numbers, no trailing whitespace. This matches
  `eval_ladder_core::canonical_json` byte-for-byte and is pinned by
  the `milestone_i_python_emitted_benchmark_task_deserializes_in_rust`
  integration test.
- Every emitted file is re-validated against
  `schemas/benchmark_task.schema.json` before it is written; emission
  fails loudly if pydantic and the JSON Schema drift.
- Gold patches are referenced as `sha256:<hex>` of the raw patch
  bytes; the patch content is not written to disk by this command
  (benchmark ingest writes it separately).

### Resilience contract

Default mode: per-record errors are logged to stderr and the bad
record is skipped; exit `0` if at least one record was emitted,
exit `3` if every record failed. `--strict` aborts on the first
failure with exit `3`. IO or manifest-parse failures exit `2`.

## Bundle and trace verification (Milestone J)

Any reviewer, CI job, or downstream archivist can recompute every
digest in an evidence bundle with a single shipped command. The
`eval-ladder verify` subcommand is the only endorsed entry point for
this task; it wraps `eval_ladder_evidence::verify_bundle` and
`eval_ladder_traces::TraceReader::read_and_verify` behind a stable
CLI and a canonical JSON report.

```bash
# 1. Verify a single bundle directory (default: also verifies trace.jsonl).
eval-ladder verify bundle --bundle-dir runs/released/bundle-sympy-22005

# 2. Verify only a trace's hash chain.
eval-ladder verify trace --trace runs/released/bundle-sympy-22005/trace.jsonl

# 3. Verify every bundle under a batch run directory and emit a
#    canonical verify_report.json alongside it.
eval-ladder verify run-dir --run-dir runs/released

# 4. Fail-fast variant for short-circuit CI checks.
eval-ladder verify run-dir --run-dir runs/released --fail-fast
```

### Report contract

`verify run-dir` writes `verify_report.json` (canonical JSON, sorted
keys, UTF-8, no trailing whitespace) with the following shape:

- `schema_version`: u32 (currently `1`).
- `evaluator_version`: semver string of the evaluator that produced
  the report.
- `run_dir`: absolute path of the verified directory.
- `total`, `ok`, `invalid`: entry counters.
- `entries`: array of per-bundle rows, **sorted by `bundle_name`**
  for stable diffs.

Each row is:

- `bundle_name`, `bundle_dir`.
- `status`: `ok` iff both bundle and trace checks passed
  (`trace` may be `not_applicable` when requested via
  `--verify-trace false`).
- `bundle_hash`: content-addressed SHA-256 of the bundle index when
  parseable, otherwise omitted.
- `bundle`, `trace`: per-check status.
- `error_code`, `error`: stable error code (`VERIFY_*`, see below)
  and a human message when `status == invalid`.

### Stable error codes

- `VERIFY_FILE_DIGEST_MISMATCH` - a bundle file hashed differently
  from its entry in `artifact_hashes.json`.
- `VERIFY_BUNDLE_DIGEST_MISMATCH` - the recomputed bundle-level
  hash did not match the stored `bundle_hash`.
- `VERIFY_MISSING_FILE` - the index declares a file that is not on
  disk.
- `VERIFY_BUNDLE_PARSE`, `VERIFY_BUNDLE_IO`, `VERIFY_BUNDLE_CORE` -
  structural failures reading `artifact_hashes.json`.
- `VERIFY_TRACE_MISSING`, `VERIFY_TRACE_IO`, `VERIFY_TRACE_PARSE`,
  `VERIFY_TRACE_CORE` - trace file I/O / deserialize failures.
- `VERIFY_TRACE_HASH_MISMATCH` - a trace event's recomputed
  `event_hash` did not match the stored value.
- `VERIFY_TRACE_CHAIN_BROKEN` - a trace event's `prev_event_hash`
  did not match the preceding event's `event_hash`.
- `VERIFY_TRACE_FIRST_NOT_RUN_STARTED`,
  `VERIFY_TRACE_DUPLICATE_RUN_STARTED` - structural trace
  violations.

### Exit codes

- `0`: every bundle verified successfully.
- non-zero: at least one bundle failed. The report is still
  written so reviewers can triage offline. `--fail-fast` aborts
  before enumerating the remaining entries.

### Determinism contract

`verify_report.json` is byte-deterministic across reruns for the
same inputs modulo the `run_dir` / `bundle_dir` strings (which
carry absolute paths for operator ergonomics). Content-bearing
fields (`bundle_hash`, `status`, `bundle`, `trace`, `error_code`)
are strictly deterministic and suitable for CI diff gates.

## Reproducibility demo (Milestone K)

The `eval-ladder demo run` command is the single command a reviewer
runs to confirm the repository builds, executes, and emits
hash-verifiable artifacts without any upstream benchmark data,
network access, or container runtime. It materializes a wholly
synthetic panel, drives the batch pipeline over it with a
deterministic clock, emits the Milestone G paper exports, and
re-verifies every produced bundle in-process.

```bash
# Smallest usable slice (2 tasks; ~1 s on a developer laptop).
eval-ladder demo run --out runs/demo --tasks 2

# Larger slice for timing experiments (stays well under the
# 15-minute reviewer budget).
eval-ladder demo run --out runs/demo --tasks 25

# Batch + verify only (skip the analyze step).
eval-ladder demo run --out runs/demo --tasks 2 --skip-analyze
```

### Output layout

```
<out>/
  inputs/               # Synthetic panel + per-entry fixtures.
    evaluator.toml
    panel.jsonl
    demo-00/
      task.json
      candidate.json
      patch.diff
      workspace/README.md
    ...
  bundles/              # One sealed evidence bundle per task.
    batch_summary.json
    verify_report.json  # Written by the verify step.
    bundle-demo-00/
      artifact_hashes.json
      candidate_resolution.json
      run_manifest.json
      trace.jsonl
      ...
    ...
  paper/                # Milestone G + L paper-export tables.
    score_descent.{csv,json}
    conditional_false_success.{csv,json}
    rank_stability.{csv,json}
    taxonomy.{csv,json}
    static_vs_live.{csv,json}     # Milestone L
    manifest.json                 # schema_version = 3 (includes analysis_mode)
```

### Determinism contract

Every artifact emitted by `demo run` is a pure function of
`(--out layout, --tasks)`:

- Task IDs, candidate IDs, bundle IDs, and timestamps are all
  derived from a pinned namespace UUID and a fixed wall clock
  (`2025-01-01T00:00:00Z`).
- Bundle hashes and `verify_report.json` content are byte-
  deterministic across reruns (pinned by
  `milestone_k_demo_is_byte_deterministic_across_runs`).
- The end-to-end invariants (all bundles ok, all paper tables
  emitted, verify report all-green) are pinned by
  `milestone_k_demo_runs_end_to_end`.

### When to use which flag

| Intent                              | Flags                        |
|-------------------------------------|------------------------------|
| Reviewer smoke test                 | `--tasks 2`                  |
| Exercising the full analysis seam   | default                      |
| Performance budgeting              | `--tasks 25 --skip-analyze` (or more) |
| CI "is it alive?" gate              | `--tasks 2 --skip-analyze`   |

## CI tiers

Full specifications live under `.github/workflows/`.

### Tier 1 (fast)
- Rust unit tests, `cargo fmt --check`, `cargo clippy -D warnings`.
- JSON Schema validation.
- Runs on every PR.

### Tier 2 (medium)
- Tier 1 plus Python adapter tests (`pytest`), `ruff`, `mypy`.
- Trace and evidence integration tests against fixture tasks.
- Mock container runs (no real Docker).
- Runs on every PR.

### Tier 3 (heavy)
- Sampled benchmark replay on the miniature internal fixture suite
  (3 Python tasks, 2 Rust tasks, 2 proof-subset tasks).
- Lean proof-subset smoke tests.
- Runs nightly and on tagged releases; never in the PR critical path.

Full benchmark evaluation is never run in CI. It is run explicitly on
release machines and the outputs are committed under `runs/released/`.

## Release hygiene

Before a release:

- Bump versions in all `Cargo.toml`s and `pyproject.toml`.
- Run `cargo deny check` and `cargo audit` (or `just deny` / `just audit`).
- Refresh `paper/exports/` from a clean Tier 3 run.
- Update `docs/submission_checklist.md`.
- Tag the release `vX.Y.Z` and attach the evidence-bundle index to the
  GitHub release.

## Incident triage

If an evaluator result is disputed:

1. Locate the evidence bundle at
   `runs/released/<panel>/results/<candidate_id>/`.
2. Recompute the bundle hash and verify it matches
   `artifact_hashes.json`.
3. Inspect `trace.jsonl`. If the hash chain is broken, the bundle is
   tampered; escalate.
4. Re-run the single candidate with
   `eval-ladder evaluate candidate --candidate ...`.
5. If the new run diverges, open an issue tagged `evaluator-regression`
   with both bundles attached.
