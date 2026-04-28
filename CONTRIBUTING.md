# Contributing

Thanks for contributing to `eval-ladder`.

## Development setup

1. Install Rust (toolchain pinned in `rust-toolchain.toml`).
2. Install Python `3.10+`.
3. Clone repository and run:

```bash
cargo build --workspace
just ci-tier1
```

## Pull request expectations

- Keep changes scoped and reproducible.
- Add or update tests for behavior changes.
- Update docs when commands, schema, or outputs change.
- Preserve deterministic/reproducibility guarantees for evaluator outputs.

## Validation checklist before PR

- `cargo fmt --check`
- `cargo clippy --workspace -- -D warnings`
- `cargo test --workspace`
- `just ci-tier2` for cross-language checks

## Commit and review guidance

- Use clear commit messages focused on why.
- Include before/after evidence for release-critical changes.
- For evaluator surface changes, document impact in `docs/` and update evaluator cards.
