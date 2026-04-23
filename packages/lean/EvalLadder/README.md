# EvalLadder (Lean 4)

The L4 semantic validator for the eval-ladder evaluation system.

Scope and selection policy live at the repository root in
`docs/proof_subset_policy.md`. This Lean project defines:

- `EvalLadder/Obligations/`: versioned problem statements, one module per
  obligation declared in `datasets/derived/proof_subset/manifest.jsonl`.
- `EvalLadder/Theorems.lean` (and future `EvalLadder/Theorems/` subtree):
  accepted proofs.
- `EvalLadder/Tactics.lean`: reusable helper tactics.
- `EvalLadder/Fixtures.lean`: smoke-test fixtures. These are NOT part of the
  curated subset and MUST NOT be cited by any obligation.

Build:

```bash
cd packages/lean/EvalLadder
lake build
```

The curated proof subset currently lists **eight** Rust tasks in
`datasets/derived/proof_subset/manifest.jsonl`; the flagship obligation remains
`obl.rust_swe_bench.clap_rs.clap_5873.ignore_errors_recovery_identity` at
`EvalLadder/Obligations/ClapRs/Clap5873.lean`.

Lean toolchain is pinned by `lean-toolchain`. The Rust CLI invokes `lake`
through `eval-ladder prove-subset --lean-root packages/lean/EvalLadder` once
Milestone F is landed.
