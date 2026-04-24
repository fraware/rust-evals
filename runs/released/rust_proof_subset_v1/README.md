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

## Evaluation (L4 batch)

Use the Rust evaluator profile, the proof-subset L3 policy
(`configs/policy/rust_proof_subset.toml`, which permits `**/examples/**`
alongside `src/tests`), the proof-subset manifest for L4, and a generous timeout.

```powershell
cargo run -p eval-ladder-cli -- evaluate batch `
  --input runs/released/rust_proof_subset_v1/panel.jsonl `
  --config configs/evaluator/rust.toml `
  --levels L0,L1,L3,L4 `
  --policy configs/policy/rust_proof_subset.toml `
  --obligations datasets/derived/proof_subset/manifest.jsonl `
  --lean-root packages/lean/EvalLadder `
  --out runs/released/rust_proof_subset_v1/results `
  --timeout-secs 14400 `
  --deterministic-clock
```

Then `verify run-dir` and `analyze paper-export` as for other released panels.

**Expect variance** across hosts: full `cargo test --workspace --locked` on
historical clap snapshots and on ripgrep is CPU- and disk-heavy; some L0/L1 rows
may time out on smaller machines even when L4 passes for obligations whose
checker is independent of cargo.

## Reviewer notes on Lean sketches

See `docs/proof_subset_sketches.md` for explicit **fidelity gaps** between the
seven lightweight Lean obligations and full crate semantics. The `5873`
obligation is the primary semantic anchor; sketches exist to stress the L4
machinery across eight distinct `task_id` keys without claiming end-to-end
formalisation of every issue body.
