# Proof subset sketch obligations — reviewer fidelity notes

The flagship obligation **`clap-rs__clap_5873`** (`EvalLadder/Obligations/ClapRs/Clap5873.lean`)
carries a small-step semantic model of the cited `did_you_mean` recovery path and is
intended to be read alongside the merged PR hunk on `Parser::parse_long_arg`.

The seven additional obligations listed for other `task_id` values in
`datasets/derived/proof_subset/manifest.jsonl` are **sketch obligations** for the
evaluation ladder: each states a discrete lemma that reflects *one structural
aspect* of the GitHub issue (symmetry, distinct strings, length growth, disjoint
surfaces, injectivity, cardinality). They are **not** claimed to be
bit-exact extractions of the full Rust program semantics, and they do not replace
official `cargo test` verdicts at L0/L1.

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

## End-to-end batching

Golden patches and workspaces for **all eight** tasks are materialised under
`runs/released/rust_proof_subset_v1/` by `packages/python/scripts/build_rust_proof_subset_panel.py`
so `evaluate batch` can exercise L0–L4 on the same rows as the proof manifest without
ad-hoc path wiring per task.
