# Documentation index

Authoritative technical writing for `eval-ladder` lives in this directory. Paths
are relative to the repository root.

| Document | Purpose |
|----------|---------|
| [`scientific_scope.md`](scientific_scope.md) | Paper claim, threats to validity, and literature mapping. |
| [`paper_claim_sources.json`](paper_claim_sources.json) | Machine-readable headline claim to frozen export paths; YAML mirror [`paper_claim_sources.yaml`](paper_claim_sources.yaml). |
| [`evaluation_ladder.md`](evaluation_ladder.md) | L0–L4 semantics, verdict codes, and level interactions. |
| [`architecture.md`](architecture.md) | Crate graph, responsibilities, data flows, evidence bundles, and analysis outputs. |
| [`benchmark_support.md`](benchmark_support.md) | Verified, Live, and Rust benchmark adapters and ingest. |
| [`proof_subset_policy.md`](proof_subset_policy.md) | Proof-subset governance; **Lean sketch fidelity** obligation table; batching notes. |
| [`evidence_manual.md`](evidence_manual.md) | Selection protocols, L2 gold validation and case studies, Rust paper-semantics replay, and operational runbook. |
| [`public_terminology.md`](public_terminology.md) | Public-facing terminology and naming policy for documentation. |
| [`getting_started.md`](getting_started.md) | First-run setup, demo execution, and table regeneration path. |
| [`troubleshooting.md`](troubleshooting.md) | Common setup, runtime, and reproducibility failures with fixes. |
| [`cli_reference.md`](cli_reference.md) | Command reference for core `eval-ladder` workflows. |
| [`evidence_tranche_plan.md`](evidence_tranche_plan.md) | Evidence tranche execution plan and gate commands. |
| [`evidence_empirical_status.md`](evidence_empirical_status.md) | Machine-checked gate outcomes (publication-threshold vs `--gate-profile release`). |
| [`submission_checklist.md`](submission_checklist.md) | Submission checklist, engineering readiness, and evidence gates. |

Release logs and paper-facing exports live under `paper/` when generated locally; that tree is gitignored in this repository.

## Start here

1. Read [`getting_started.md`](getting_started.md) for local setup and a first successful run.
2. Use [`evidence_manual.md`](evidence_manual.md#operational-runbook) for production-like evaluation batches.
3. Use [`architecture.md`](architecture.md) and [`evaluator_card_template.md`](evaluator_card_template.md) when interpreting released run outputs.
