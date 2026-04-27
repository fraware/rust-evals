# rust_proof_subset_v1

Golden-agent **Rust-SWE-bench** panel covering **every** `task_id` in
`datasets/derived/proof_subset/manifest.jsonl` (eight tasks). Workspaces are
pristine checkouts at each task’s `base_commit` (no `.git`); patches are the
merged upstream PR diffs from GitHub (`pull/<n>.diff`).

## Materialisation

From the repository root (network required):

```powershell
python packages/python/scripts/build_rust_proof_subset_panel.py
```

Optional custom paths:

```powershell
python packages/python/scripts/build_rust_proof_subset_panel.py `
  --out runs/released/rust_proof_subset_v1 `
  --proof-manifest datasets/derived/proof_subset/manifest.jsonl
```

## Evaluation (optimized batch workflow)

Use the Rust evaluator profile, the proof-subset L3 policy
(`configs/policy/rust_proof_subset.toml`, which permits `**/examples/**`
alongside `src/tests`), the proof-subset manifest for L4, and a generous timeout.

### 1) Fast semantic iteration (policy + proof loop)

Runs only L3/L4, enables resume, and limits contention with `--jobs 2`.

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --input runs/released/rust_proof_subset_v1/panel.jsonl `
  --config configs/evaluator/rust.toml `
  --track fast `
  --policy configs/policy/rust_proof_subset.toml `
  --obligations datasets/derived/proof_subset/manifest.jsonl `
  --lean-root packages/lean/EvalLadder `
  --out runs/released/rust_proof_subset_v1/results `
  --resume `
  --jobs 2 `
  --deterministic-clock
```

### 2) Heavy execution gate (official + trusted rerun)

Runs L0/L1 with shared Rust build cache, adaptive timeouts, and smart rerun
strategy for Rust (`smart_rust_reuse` default).

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --input runs/released/rust_proof_subset_v1/panel.jsonl `
  --config configs/evaluator/rust.toml `
  --track heavy `
  --policy configs/policy/rust_proof_subset.toml `
  --out runs/released/rust_proof_subset_v1/results `
  --resume `
  --jobs 2 `
  --adaptive-timeouts `
  --short-timeout-secs 180 `
  --timeout-secs 14400 `
  --rust-target-cache-root runs/released/rust_proof_subset_v1/.cargo_target_cache `
  --deterministic-clock
```

### 3) Full L0/L1/L3/L4 release replay

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --input runs/released/rust_proof_subset_v1/panel.jsonl `
  --config configs/evaluator/rust.toml `
  --levels L0,L1,L3,L4 `
  --policy configs/policy/rust_proof_subset.toml `
  --obligations datasets/derived/proof_subset/manifest.jsonl `
  --lean-root packages/lean/EvalLadder `
  --out runs/released/rust_proof_subset_v1/results `
  --resume `
  --jobs 2 `
  --adaptive-timeouts `
  --short-timeout-secs 180 `
  --rust-target-cache-root runs/released/rust_proof_subset_v1/.cargo_target_cache `
  --timeout-secs 14400 `
  --deterministic-clock
```

Then `verify run-dir` and `analyze paper-export` as for other released panels.

### Paper-semantics ladder (strict rust-proof gate)

For two controlled **L3 pass / L4 fail** exemplars plus unchanged rows for an
**all-level pass** story, replay with
`datasets/derived/proof_subset/manifest_paper_semantics_l4_counterexample.jsonl`
(see `docs/rust_proof_paper_semantics_replay.md` and
`just rust-proof-batch-seal-paper-semantics`). Use a fresh `--out` directory so
the audit `results_seal` tree is not overwritten.

### 4) Seal and export (required for release evidence)

```powershell
cargo run -p eval-ladder-cli -- verify run-dir `
  --run-dir runs/released/rust_proof_subset_v1/results

cargo run -p eval-ladder-cli -- analyze paper-export `
  --run-dir runs/released/rust_proof_subset_v1/results `
  --out-dir paper/exports/rust_proof_subset_v1
```

### Recommended defaults by machine class

- **Laptop (8-12 logical cores, limited thermal headroom):** `--jobs 1`,
  `--adaptive-timeouts`, `--short-timeout-secs 120`.
- **Workstation (16-24 logical cores):** `--jobs 2` (default recommendation),
  `--adaptive-timeouts`, `--short-timeout-secs 180`.
- **High-core server (32+ logical cores):** start with `--jobs 2`, then
  benchmark `--jobs 3`; avoid `--jobs 4+` unless cache hit rates remain high
  and wall time improves.

**Expect variance** across hosts: full `cargo test --workspace --locked` on
historical clap snapshots and on ripgrep is CPU- and disk-heavy; some L0/L1 rows
may time out on smaller machines even when L4 passes for obligations whose
checker is independent of cargo.

## Reviewer notes on Lean sketches

See `docs/proof_subset_policy.md` (**Lean sketch fidelity** table) for explicit **fidelity gaps** between the
seven lightweight Lean obligations and full crate semantics. The `5873`
obligation is the primary semantic anchor; sketches exist to stress the L4
machinery across eight distinct `task_id` keys without claiming end-to-end
formalisation of every issue body.
