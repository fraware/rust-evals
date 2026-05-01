# Paper directory guide

This directory contains paper-facing exports and table artifacts used by the
`eval-ladder` submission package.

## NeurIPS 2026 E&D alignment

- **Claim lock:** `docs/CLAIM_LOCK_NEURIPS2026.md`
- **Per-claim artifact map:** `paper/exports/CLAIM_SOURCE_MAP.md`
- **Machine-readable claim wiring:** `paper/paper_claim_sources.json` (checked by
  `python ci/scripts/check_paper_claim_sources.py`)
- **Evaluator Cards:** `paper/exports/evaluator_cards/` (see `*.yaml` and `*.md`)

Title for the paper package:
**Eval-Ladder: Evaluator-Conditioned Measurement for Repository-Level Coding-Agent Benchmarks.**

## What is generated

- `paper/exports/**` outputs produced by `eval-ladder analyze paper-export` and
  supporting scripts.
- `paper/tables/**` rendered table fragments used by the manuscript workflow.

## What is curated

- Evaluator card files under `paper/exports/evaluator_cards/`.
- Any manually maintained manuscript-side notes that explain interpretation.

## Regeneration

```bash
just reproduce-paper-tables
```

This command regenerates primary release exports from frozen reproducibility
run directories. The paper must cite **generated** CSV/JSON values only; do not
manually edit table counts. Every manuscript table should list its source path
in internal `paper_claim_sources` wiring (see `paper/paper_claim_sources.json`).

## Safety notes

- Do not edit generated JSON/CSV outputs by hand.
- Regenerate from run directories so provenance remains auditable.
