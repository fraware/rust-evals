# Paper directory guide

This directory contains paper-facing exports and table artifacts used by the
`eval-ladder` submission package.

## What is generated

- `paper/exports/**` outputs produced by `eval-ladder analyze paper-export` and
  supporting scripts.
- `paper/tables/**` rendered table fragments used by the manuscript workflow.

## What is curated

- Evaluator card markdown files under `paper/exports/evaluator_cards/`.
- Any manually maintained manuscript-side notes that explain interpretation.

## Regeneration

```bash
just reproduce-paper-tables
```

This command regenerates primary release exports from frozen reproducibility
run directories.

## Safety notes

- Do not edit generated JSON/CSV outputs by hand.
- Regenerate from run directories so provenance remains auditable.
