# Documentation index

Authoritative technical writing for `eval-ladder` lives in this directory. Paths
are relative to the repository root.

| Document | Purpose |
|----------|---------|
| [`scientific_scope.md`](scientific_scope.md) | Paper claim, threats to validity, and literature mapping. |
| [`CLAIM_LOCK_NEURIPS2026.md`](CLAIM_LOCK_NEURIPS2026.md) | NeurIPS 2026 allowed/prohibited claims vs frozen evidence tiers. |
| [`evaluation_ladder.md`](evaluation_ladder.md) | L0–L4 semantics, verdict codes, and level interactions. |
| [`architecture.md`](architecture.md) | Crate graph, responsibilities, and major data flows. |
| [`artifact_spec.md`](artifact_spec.md) | Evidence bundles, manifests, and analysis outputs. |
| [`benchmark_support.md`](benchmark_support.md) | Verified, Live, and Rust benchmark adapters and ingest. |
| [`proof_subset_policy.md`](proof_subset_policy.md) | Proof-subset selection rubric, governance, **Lean sketch fidelity** obligation table, and batching notes. |
| [`operational_runbook.md`](operational_runbook.md) | CLI recipes, batch drives, verification, and CI alignment. |
| [`public_terminology.md`](public_terminology.md) | Public-facing terminology and naming policy for documentation. |
| [`getting_started.md`](getting_started.md) | First-run setup, demo execution, and table regeneration path. |
| [`troubleshooting.md`](troubleshooting.md) | Common setup, runtime, and reproducibility failures with fixes. |
| [`cli_reference.md`](cli_reference.md) | Command reference for core `eval-ladder` workflows. |
| [`evidence_tranche_plan.md`](evidence_tranche_plan.md) | Evidence tranche execution plan and gate commands. |
| [`evidence_empirical_status.md`](evidence_empirical_status.md) | Machine-checked gate outcomes (publication-threshold vs `--gate-profile release`). |
| [`l2_selection_protocol.md`](l2_selection_protocol.md) | Pre-specified L2 primary-cohort row construction, exclusions, and validator-family semantics. |
| [`l2_failure_case_studies.md`](l2_failure_case_studies.md) | Eight-case human adjudication sample for L2 failures (augmented-test vs regression stress-control). |
| [`l2_gold_patch_validation.md`](l2_gold_patch_validation.md) | Gold/developer patch replay outcomes under primary-cohort L2 validators and limitation analysis. |
| [`submission_checklist.md`](submission_checklist.md) | Submission checklist, engineering readiness, and evidence gates. |

## Start here

1. Read [`getting_started.md`](getting_started.md) for local setup and a first successful run.
2. Use [`operational_runbook.md`](operational_runbook.md) for production-like evaluation batches.
3. Use [`artifact_spec.md`](artifact_spec.md) and [`evaluator_card_template.md`](evaluator_card_template.md) when interpreting released run outputs.
