# EvalLadder (Lean 4)

The L4 semantic validator for the eval-ladder evaluation system.

Scope and selection policy live at the repository root in
`docs/proof_subset_policy.md`. This Lean project defines:

- `EvalLadder/Obligations/`: versioned problem statements, one module per
  obligation declared in `datasets/derived/proof_subset/manifest.jsonl`.
- `EvalLadder/Theorems/`: accepted proofs.
- `EvalLadder/Tactics/`: reusable helper tactics.
- `EvalLadder/Fixtures/`: smoke-test fixtures. These are NOT part of the
  curated subset and MUST NOT be cited by any obligation.

Build:

```bash
cd packages/lean/EvalLadder
lake build
```

Current production obligation set includes
`Obligations/ClapRs/Clap5873.lean`, referenced by
`datasets/derived/proof_subset/manifest.jsonl` as
`obl.rust_swe_bench.clap_rs.clap_5873.ignore_errors_recovery_identity`.

Lean toolchain is pinned by `lean-toolchain`. The Rust CLI invokes `lake`
through `eval-ladder prove-subset --lean-root packages/lean/EvalLadder` once
Milestone F is landed.
