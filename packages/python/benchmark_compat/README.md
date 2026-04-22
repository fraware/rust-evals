# benchmark_compat

Python compatibility layer for the eval-ladder Rust evaluation monorepo.

This package exists **only** to bridge Python-heavy benchmark ecosystems
(principally SWE-bench tooling). No evaluator core logic lives here.
The authoritative evaluation implementation is the Rust workspace at
the repo root.

## Scope

- Import and normalize task metadata from SWE-bench-family release
  manifests into JSON files that match
  `schemas/benchmark_task.schema.json`.
- Wrap benchmark-preparation scripts that are only distributed as
  Python.
- Provide notebooks for sanity checks against the Rust outputs.

## Non-scope

- Running the evaluator ladder.
- Writing evidence bundles.
- Computing analysis metrics.

## Install

From the repository root, in editable mode with dev extras:

```bash
python -m pip install -e ".[dev]"
```

This installs the `eval-ladder-py` console script as well as the
`benchmark_compat` import path.

## Usage

Normalize a SWE-bench Verified release manifest into per-task
`BenchmarkTask` files that the Rust evaluator accepts verbatim:

```bash
eval-ladder-py normalize-swe-bench \
  --source datasets/public_links/verified.jsonl \
  --out-dir benchmarks/verified/manifests/
```

Add `--strict` to abort on the first malformed record.

## Canonicalization

All JSON the Python layer writes is canonical (sorted keys, UTF-8, no
BOM, no trailing whitespace) and matches
`eval_ladder_core::canonical_json` byte-for-byte. On-disk
`BenchmarkTask` manifest files carry a trailing `\n` to match the
Rust writer's convention in
`packages/rust/benchmarks/src/writer.rs`.

Cross-language parity is pinned by
`tests/integration/tests/python_round_trip.rs`.

## Tests, lint, types

```bash
python -m ruff check packages/python tests/python
python -m mypy
python -m pytest tests/python
```

All three are required green for CI Tier 2.
