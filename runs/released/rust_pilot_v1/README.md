# rust_pilot_v1

Rust-native pilot run for the eval-ladder. One curated
Rust-SWE-bench task (`clap-rs__clap_5873`) evaluated end-to-end on a
native cargo toolchain, with L4 semantic validation wired to a real
Lean proof under `packages/lean/EvalLadder/Obligations/ClapRs/`.

## Scope and honest labels

* **Tasks:** 1 (`clap-rs__clap_5873`), selected because it clones
  cleanly at `base_commit=1d5c6798d`, compiles on MSVC/Windows, and
  admits a narrow semantic obligation on clap's parser.
* **Agent:** `golden_agent` — the candidate patch is the verbatim
  upstream merged PR #5873 diff (fetched from GitHub). This is a
  **golden baseline** for the ladder; it is not a synthesised agent
  attempt. The candidate's `generation_mode` is `human_assisted`
  and its `random_seed` is `0` by convention for deterministic
  baselines.
* **Backend:** `LocalProcessEngine` (no Docker). The batch runs the
  task's `cargo test --workspace --locked` entrypoint directly on
  the host, so results reflect the host's cargo/rustc/MSVC
  toolchain. The `LocalProcessEngine` env allow-list in
  `packages/rust/runner/src/container.rs` carries PATH, the MSVC
  build vars (`INCLUDE`, `LIB`, `LIBPATH`, `VCToolsInstallDir`,
  `WindowsSdkDir`, ...), cargo/rustup home overrides, and a
  writable temp dir through to the subprocess; without this
  allow-list rustc cannot locate a scratch directory or link
  against the Windows SDK.
* **Levels:** `L0,L1,L3,L4`. `L2` is not run here because this pilot
  does not ship a strengthening spec (see `docs/evaluation_ladder.md`
  for the L2 contract).
* **L3 policy:** `configs/policy/rust_pilot.toml`, a Rust-workspace-
  aware variant of `default_policy.toml`. The default profile is
  Python-flat and flags legitimate `clap_builder/src/...` edits with
  `PV_EDIT_SCOPE`; the pilot profile widens the edit globs to
  `**/src/**` and `**/tests/**` while keeping `requires_reproducible_seed`,
  the network_mode, and the required trace events unchanged.
* **L4 obligation:** The `clap-rs__clap_5873` entry in
  `datasets/derived/proof_subset/manifest.jsonl`. The proof_checker
  invokes `python scripts/check_obligation.py <lean_file>
  L4_OBLIGATION_MET` with cwd = `packages/lean/EvalLadder/`; the
  driver runs `lake env lean <lean_file>` and emits a
  `LeanCheckOutcome` JSON on stdout.

## Layout

* `panel.jsonl`           — batch driver input (one PanelEntry per
  line).
* `candidates/<agent>/<task>.json` — `CandidateResolution`
  per (agent, task). `candidate_id` is
  `uuidv5(EVAL_LADDER_NAMESPACE, agent|task|sha256(patch))`.
* `patches/<agent>/<task>.diff`    — unified diff bytes.
* `workspaces/<task>/`             — clean source tree exported at
  `base_commit` (no `.git`, no build artifacts). Used as the
  `workspace_template` by the runner.
* `provenance.json`        — agent/model identifiers plus the public
  GitHub PR URL and sha256 of each fetched diff.
* `results/`               — batch output. Produced by
  `eval-ladder evaluate batch`; see `batch_summary.json` for the
  per-entry summary row and `<bundle_name>/` for the full evidence
  bundle.

## Rerunning the batch

```powershell
# From repo root. Requires: cargo toolchain, Lake, Python, and
# the host's MSVC env activated (vcvars64.bat) for `cargo test`
# to link.

cargo run --quiet --bin eval-ladder -- evaluate batch `
    --input runs/released/rust_pilot_v1/panel.jsonl `
    --config configs/evaluator/rust.toml `
    --levels L0,L1,L3,L4 `
    --policy configs/policy/rust_pilot.toml `
    --obligations datasets/derived/proof_subset/manifest.jsonl `
    --lean-root packages/lean/EvalLadder `
    --out runs/released/rust_pilot_v1/results `
    --timeout-secs 3600 `
    --deterministic-clock
```

A full run compiles clap's workspace twice (once for L0, once for
the fresh L1 staging copy) plus all tests and takes 30-60 minutes
on a warm CARGO_HOME.

## Paper export and verify

```powershell
cargo run --quiet --bin eval-ladder -- analyze paper-export `
    --run-dir runs/released/rust_pilot_v1/results `
    --out-dir paper/exports/rust_pilot_v1

cargo run --quiet --bin eval-ladder -- verify run-dir `
    --run-dir runs/released/rust_pilot_v1/results
```

## Released observed results

The committed run output in `results/` is from a completed local run and
was re-verified after sealing.

- `batch_summary.json`:
  - `total_entries = 1`, `ok_entries = 1`, `invalid_entries = 0`
  - L0: `invalid` (`L0_OFFICIAL_TIMEOUT`)
  - L1: `invalid` (`L1_HARNESS_ERROR`)
  - L3: `pass` (`PASS`)
  - L4: `pass` (`L4_OBLIGATION_MET`)
- `verify_report.json`:
  - `total = 1`, `ok = 1`, `invalid = 0`
  - bundle row status `ok`, trace status `ok`

Interpretation:

- This pilot demonstrates the pipeline and L4 seam end-to-end with
  hash-verifiable artifacts.
- It is not a benchmark score claim because the single task timed out at
  L0/L1 under the configured wall-clock budget and host toolchain.

## Regenerating the panel

The panel, candidate, patch, and workspace artifacts are produced by
`packages/python/scripts/build_rust_pilot.py` (idempotent; reruns
reconcile with the existing files).
