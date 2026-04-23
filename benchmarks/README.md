# benchmarks/

Normalized benchmark manifests and adapter descriptors. The Rust adapters in
`packages/rust/benchmarks` are the authoritative ingestion code; this
directory holds only the output of ingestion plus benchmark-specific asset
metadata.

Subdirectories:

- `verified/manifests/`: normalized `BenchmarkTask` JSON files for
  SWE-Bench Verified.
- `verified/adapters/`: descriptor files used by the Verified adapter.
- `live/`: same layout, for SWE-bench-Live. Live metadata keeps task
  timestamps and environment image mappings and is NOT coerced into the
  Verified shape.
- `rust/`: same layout, for Rust-SWE-bench. Cargo workspace metadata and
  lockfile policy live here.

Current checked-in ingest snapshot:

- `verified/manifests/`: 500 normalized tasks
- `live/manifests/`: 500 normalized tasks (live `verified` split)
- `rust/manifests/`: 239 normalized tasks

Do not commit benchmark source archives here. Use `datasets/public_links/`
to reference public releases.
