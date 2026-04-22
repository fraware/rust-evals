# Proof-carrying subset

This directory holds the curated subset of benchmark tasks used by the L4
semantic validator. Manifest entries in `../manifest.jsonl` MUST conform to
`schemas/proof_obligation.schema.json` at the repository root.

See `docs/proof_subset_policy.md` for the selection rubric, eligible task
categories, and the obligation template.

The production `manifest.jsonl` is intentionally kept empty at this
stage of the repository. Milestone F ships the full L4 plumbing and
the `eval-ladder prove-subset` batch driver; obligations exercised
by the Milestone F acceptance suite live under
`packages/lean/EvalLadder/Obligations/Fixtures/` and are loaded
programmatically by the tests rather than from this file, so adding
fixtures here would change no behaviour while diluting the selection
discipline described in `docs/proof_subset_policy.md`.

Curated obligations populate this file as they pass the five-item
rubric review; with no entries, `eval-ladder prove-subset` returns
`NotApplicable` on every bundle.
