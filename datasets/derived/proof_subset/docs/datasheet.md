# Datasheet: proof subset

## Motivation

This subset provides a curated semantic-obligation surface for `eval-ladder` L4
evaluation over Rust issue-resolution tasks.

## Composition

- Source manifest: `datasets/derived/proof_subset/manifest.jsonl`
- Task count: 8 entries in the current release manifest
- Unit: task-level obligation record tied to upstream repository/task identifiers

## Collection and processing

- Entries are derived from public upstream issue-resolution sources.
- Records are normalized into a manifest format consumed by evaluator pipelines.

## Intended use

- L4 semantic-obligation checks in reproducible evaluator runs.
- Methodology research on evaluator-conditioned measurement.

## Out-of-scope use

- Not a comprehensive benchmark of all Rust issue-resolution behavior.
- Not a substitute for full repository-wide formal verification.

## Distribution and licensing

- This subset references upstream public artifacts and identifiers.
- Follow upstream license and redistribution constraints for source material.
