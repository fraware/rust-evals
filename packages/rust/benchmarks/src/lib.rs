//! # eval-ladder-benchmarks
//!
//! Benchmark adapters. Each adapter owns ingestion, environment resolution,
//! and the wrapper around the benchmark's official scoring entrypoint. The
//! adapter boundary is strict: benchmark-specific mess lives here and is not
//! leaked to `eval-ladder-core` or the CLI dispatch.
//!
//! Three adapters ship in MVP scope:
//!
//! - [`verified`] - SWE-Bench Verified (Python).
//! - [`live`] - SWE-bench-Live (Python, per-task Docker image, dated tasks).
//! - [`rust_native`] - Rust-SWE-bench (Rust).
//!
//! Adding a new benchmark means adding a module here, not editing the CLI.
#![deny(missing_docs)]
#![deny(unsafe_code)]

pub mod adapter;
pub mod live;
pub mod normalize;
pub mod raw;
pub mod rust_native;
pub mod schema;
pub mod verified;
pub mod writer;

pub use adapter::{BenchmarkAdapter, BenchmarkAdapterError, IngestOptions, IngestReport};
pub use raw::{read_jsonl, RawReadError, RawRecordWithOrigin, RawSweBenchRecord};
pub use schema::{BenchmarkTaskValidator, SchemaValidatorError, BENCHMARK_TASK_SCHEMA};
pub use writer::{ManifestWriteError, ManifestWriter};

use eval_ladder_core::BenchmarkId;

/// Build the MVP adapter for a benchmark id.
#[must_use]
pub fn adapter_for(id: BenchmarkId) -> Box<dyn BenchmarkAdapter> {
    match id {
        BenchmarkId::SweBenchVerified => Box::new(verified::VerifiedAdapter::new()),
        BenchmarkId::SweBenchLive => Box::new(live::LiveAdapter::new()),
        BenchmarkId::RustSweBench => Box::new(rust_native::RustNativeAdapter::new()),
    }
}
