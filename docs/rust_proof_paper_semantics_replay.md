# Rust proof subset: paper-semantics L4 counterexample replay

The sealed ladder under `runs/released/rust_proof_subset_v1/results_seal/` uses
the production obligation manifest
`datasets/derived/proof_subset/manifest.jsonl`, where every row is wired to a
real `EvalLadder/Obligations/**` module.

For **paper semantics** (strict `check_evidence_quality rust-proof` minima on
L3-pass / L4-fail exemplars and an all-level pass), replay the same eight-task
panel with the companion manifest
`datasets/derived/proof_subset/manifest_paper_semantics_l4_counterexample.jsonl`.
That manifest is identical except for `clap-rs__clap_1624` and
`clap-rs__clap_1710`, which point at
`EvalLadder/Obligations/PaperSemantics/Broken.lean` so `lake env lean` fails
and L4 surfaces as `L4_OBLIGATION_UNMET` while L3 policy can still pass.

Recipe:

```bash
just rust-proof-batch-seal-paper-semantics runs/released/rust_proof_subset_v1/results_seal_paper_semantics
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal_paper_semantics
```

The broken Lean module is not part of the default proof corpus; keep paper
replays on a separate `--out` directory so it does not overwrite the audit
closure bundle.
