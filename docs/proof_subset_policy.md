# Proof subset policy

The proof-carrying subset is the spine of the L4 layer. Its scientific value
depends on selection discipline. This document is the authoritative selection
rubric. Any deviation from it must be recorded per obligation in
`datasets/derived/proof_subset/manifest.jsonl`.

**NeurIPS 2026 alignment:** L4 on this subset is an **extension surface** for
semantic evaluation. For the **current sealed** Rust proof batch, natural
**L3-pass / L4-fail** separations are **not** demonstrated at headline scale.
**Synthetic** obligation-breaking replays are **mechanism tests** only and must
not be cited as representative empirical prevalence without an explicit sampling
story.

## Selection rubric

A task is eligible for the proof subset only if **all** of the following hold:

1. The issue's intended property can be stated in one or two sentences.
2. The touched code is local enough to reason about without importing large
   external theorem libraries.
3. The property matters to the issue itself; it is not a random adjacent
   invariant.
4. The property is strictly stronger than the official tests for the task.
5. Formalization effort is bounded, measured in reviewer-hours and declared
   at selection time.

All five must hold. Task selection is a source of bias in the audit and must
be justifiable on each of these dimensions.

## Preferred task categories

In priority order:

1. Parser and serializer consistency (roundtrip invariants).
2. State-machine safety (no invalid transitions under declared preconditions).
3. Preservation invariants on transformations (for example: a list reordering
   preserves length and multiset).
4. Numeric bounds and monotonicity.
5. `no-panic` or `no-invalid-state` on selected code paths.

## Categories to avoid

- Whole-repository correctness obligations.
- Obligations that require large imported theorem libraries with high
  setup cost.
- Obligations whose proof burden dwarfs their evaluative value.
- Obligations phrased in terms of the test suite itself; they should be
  phrased in terms of program behaviour.

## Obligation template

Every obligation in `datasets/derived/proof_subset/manifest.jsonl` must
specify:

- `obligation_id`: stable identifier.
- `task_id`: identifier of the benchmark task this obligation attaches to.
- `property_name`: short human-readable name.
- `property_type`: one of the preferred categories above.
- `target_files`: list of files the obligation constrains.
- `informal_statement`: one or two sentences.
- `formal_statement_ref`: path to a Lean declaration in
  `packages/lean/EvalLadder/EvalLadder/Obligations/` (relative to the Lean
  project root passed as `--lean-root`, typically `packages/lean/EvalLadder`).
- `proof_checker`: the Lean command to run (for example `lake env lean`).
- `pass_criterion`: stable code returned by the checker that counts as pass
  (for example `L4_OBLIGATION_MET`).
- `difficulty`: declared reviewer-hours at selection time.
- `selection_rationale`: why this task satisfies the five selection rubric
  items.
- `witness_inputs`: optional fixtures or inputs required by the obligation.
- `expected_touched_symbols`: symbols the patch is expected to touch.

The schema is captured in `schemas/proof_obligation.schema.json`.

## Integration discipline

- Candidate patches are applied to a clean checkout of the task's base
  commit; obligations are checked against the post-patch tree.
- Extraction is declarative. The Lean package does **not** parse arbitrary
  diffs. Extraction scripts produce a normalized obligation context, and
  Lean checks a theorem specific to that snapshot and obligation.
- Automatic Rust-to-Lean translation is explicitly out of scope. Here Lean
  is a semantic validator for curated obligations.
- An obligation that cannot be checked (for example because extraction
  fails) yields `L4_EXTRACTION_FAILED`; it does not fall back to L2 tests.

## Lean sketch fidelity (obligation table)

The flagship obligation **`clap-rs__clap_5873`**
(`EvalLadder/Obligations/ClapRs/Clap5873.lean`) carries a small-step semantic model
of the cited `did_you_mean` recovery path and is intended to be read alongside
the merged PR hunk on `Parser::parse_long_arg`.

The seven additional obligations listed for other `task_id` values in
`datasets/derived/proof_subset/manifest.jsonl` are **sketch obligations** for the
evaluation ladder: each states a discrete lemma that reflects *one structural
aspect* of the GitHub issue (symmetry, distinct strings, length growth, disjoint
surfaces, injectivity, cardinality). They are **not** claimed to be bit-exact
extractions of the full Rust program semantics, and they do not replace official
`cargo test` verdicts at L0/L1. Reviewers should read this table before
interpreting L4 `PASS` on those rows as semantic parity with upstream tests.

| task_id | Lean module | What is modeled | Deliberate fidelity gap |
|---------|-------------|-----------------|-------------------------|
| `BurntSushi__ripgrep_454` | `Ripgrep454.lean` | List length / `map id` preservation | Does not model the search automaton, `--only-matching` buffering, or PCRE edge cases from the issue. |
| `clap-rs__clap_1624` | `Clap1624.lean` | Symmetry of a binary exclusion predicate on finite slots | Real `conflicts_with_everything` interacts with groups, overrides, and positional args; not captured. |
| `clap-rs__clap_1710` | `Clap1710.lean` | Distinctness of two concrete candidate strings | Does not model subcommand trie structure, scoring, or error formatting. |
| `clap-rs__clap_1972` | `Clap1972.lean` | `List Char` length grows by one after `'\n'` | Does not tie to clap's `std::fmt` pipeline or platform-specific newline policy. |
| `clap-rs__clap_2008` | `Clap2008.lean` | Disjointness of an enum of help surfaces | Does not encode the `App` builder API or routing between `--help` / `--help -v`. |
| `clap-rs__clap_2075` | `Clap2075.lean` | Injectivity of a toy `usageLine` map on two flags | Does not model clap's usage string synthesis, wrapping, or conflict error layout. |
| `clap-rs__clap_2093` | `Clap2093.lean` | Inequality of default vs example override strings | Does not connect to the actual default help subcommand copy in-tree at the base commit. |
| `clap-rs__clap_5873` | `Clap5873.lean` | State-machine step for `applyDidYouMeanRecovery` | Fidelity is **higher** but still declarative: the Lean `ArgMatcher` is a minimal projection of upstream `ArgMatcher`; reviewers judge Rust–Lean alignment. |

### End-to-end batching

Golden patches and workspaces for **all eight** tasks are materialised under
`runs/released/rust_proof_subset_v1/` by
`packages/python/scripts/build_rust_proof_subset_panel.py` so `evaluate batch`
can exercise L0–L4 on the same rows as the proof manifest without ad-hoc path
wiring per task.

## Governance

- Adding an obligation requires a PR that touches `manifest.jsonl`,
  `packages/lean/EvalLadder/EvalLadder/Obligations/`, and this document's changelog
  section (plus the **Lean sketch fidelity** table above when the entry is a sketch).
- Removing or modifying an existing obligation requires the same PR surface
  plus a rationale field in `manifest.jsonl`.
- A selection bias audit is run before any release and archived under
  `paper/exports/`.

## Release posture

The proof subset may ship in either of the two release modes declared in
`docs/submission_checklist.md`. In Mode 1 (code-only) the manifest is the
only artifact; in Mode 2 (code + new dataset) the manifest and its
Croissant metadata live under
`datasets/derived/proof_subset/croissant/`.

## Changelog

- 2026-04-26: merged former `docs/proof_subset_sketches.md` into this document as
  **Lean sketch fidelity (obligation table)**; all references now point here.
- 2026-04-23 (panel): added `runs/released/rust_proof_subset_v1/` (golden workspaces
  + PR diffs for all eight manifest tasks) via
  `packages/python/scripts/build_rust_proof_subset_panel.py`, plus the reviewer
  obligation table (then `proof_subset_sketches.md`) and in-Lean fidelity notes on sketch modules.
- 2026-04-23 (later): expanded the manifest to **eight** distinct Rust-SWE-bench
  `task_id` rows (seven additional sketch obligations plus the original clap
  `5873` entry), relocated the Lake-visible library under
  `packages/lean/EvalLadder/EvalLadder/` so `import EvalLadder.*` resolves on
  Windows CI, and pointed every `proof_checker` path at
  `EvalLadder/Obligations/...` relative to the Lean root.
- 2026-04-23: seeded first production obligation
  `obl.rust_swe_bench.clap_rs.clap_5873.ignore_errors_recovery_identity`
  for task `clap-rs__clap_5873` (`state_machine_safety`), with Lean module
  `packages/lean/EvalLadder/EvalLadder/Obligations/ClapRs/Clap5873.lean` and
  pass criterion `L4_OBLIGATION_MET`.
