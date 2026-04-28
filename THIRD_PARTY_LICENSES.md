# Third-party licenses

This repository depends on third-party Rust and Python packages.

## Source of truth

- Rust dependency policy and license checks: `deny.toml`
- CI enforcement: `.github/workflows/release-hygiene.yml`
- Lockfile inventory: `Cargo.lock`
- Python dependency declarations: `pyproject.toml`

## Reproducible license checks

```bash
cargo deny check
```

For release builds, run the same checks used in CI and record outputs alongside
the tagged artifact manifest.
