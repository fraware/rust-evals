# Proof-carrying subset

This directory holds the curated subset of benchmark tasks used by the L4
semantic validator. Manifest entries in `../manifest.jsonl` MUST conform to
`schemas/proof_obligation.schema.json` at the repository root.

See `docs/proof_subset_policy.md` for the selection rubric, eligible task
categories, and the obligation template.

Public dataset metadata for release:

- Croissant metadata: `datasets/derived/proof_subset/croissant/metadata.json`
- Datasheet: `datasets/derived/proof_subset/docs/datasheet.md`

The production `manifest.jsonl` currently holds **eight** curated
obligations over distinct `task_id` values (one row per task; see
`eval_ladder_lean::ObligationManifest`). The flagship entry is:

* `obl.rust_swe_bench.clap_rs.clap_5873.ignore_errors_recovery_identity`
  - task: `clap-rs__clap_5873` (Rust-SWE-bench)
  - property: `state_machine_safety` on clap's did-you-mean recovery
  - Lean proof:
    `packages/lean/EvalLadder/EvalLadder/Obligations/ClapRs/Clap5873.lean`
  - seed script:
    `packages/python/scripts/seed_proof_obligation.py` (idempotent helper for
    the `clap-rs__clap_5873` row only).

Milestone F shipped the full L4 plumbing; the Milestone F acceptance
suite continues to exercise its own fixture under
`packages/lean/EvalLadder/EvalLadder/Obligations/Fixtures/` and loads it
programmatically so production and test obligations never cross
streams.

Additional obligations populate this file as they pass the
five-item rubric review in `docs/proof_subset_policy.md`. Tasks
without an entry return `L4_OBLIGATION_NOT_APPLICABLE` from
`eval-ladder prove-subset`.

The released `rust_pilot_v1` run exercises this first production entry and
records `L4_OBLIGATION_MET` for `clap-rs__clap_5873` in
`runs/released/rust_pilot_v1/results/`.

For **all eight** manifest tasks with golden patches, workspaces, and L4 in one
batch, use `runs/released/rust_proof_subset_v1/` (built by
`packages/python/scripts/build_rust_proof_subset_panel.py`; see that directory’s
README and `docs/proof_subset_policy.md` (Lean sketch fidelity table) for sketch fidelity).
