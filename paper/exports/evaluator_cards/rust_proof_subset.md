# Evaluator Card: rust_proof_subset

## Purpose

Demonstrate **L3/L4 semantic surfaces** on a curated Rust corpus with Lean-backed
obligations for patches that reach those levels.

## Applicability domain

Sealed batches such as `runs/released/rust_proof_subset_v1/results_seal/` and
manifest `datasets/derived/proof_subset/manifest.jsonl`.

## Native benchmark assumptions

Rust harness builds/tests per manifest; higher levels add policy and obligation
gates.

## Replay environment

Rust toolchain via Docker images or cached roots per run README.

## Strengthened validators

L3 policy traces where configured; L4 obligation checks via Lean linkage.

## Policy assumptions

Configured per run (`configs/policy/rust_proof_subset.toml` patterns).

## Semantic obligations

L4 requires obligation satisfaction on the curated subset.

## Denominators and invalid handling

Per-batch `batch_summary.json`; mechanism-test replays documented separately.

## Known false-positive risks

Toolchain or obligation mismatch can reject valid proofs.

## Known false-negative risks

Passing builds without obligations can still miss semantic gaps.

## Reproduction command

See `runs/released/rust_proof_subset_v1/README.md` and `just reproduce-paper-tables`
for aggregated paper tables touching this surface.

## Evidence bundle paths

- Manifest: `datasets/derived/proof_subset/manifest.jsonl`
- Latest sealed summary referenced from `paper/exports/strict_feasibility_report.json`
