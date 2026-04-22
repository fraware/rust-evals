//! Tracing / logging initialization.

use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the tracing subscriber with a level selected from `-v` flags
/// plus the `EVAL_LADDER_LOG` env var.
pub fn init(verbosity: u8) -> Result<()> {
    let level = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter =
        EnvFilter::try_from_env("EVAL_LADDER_LOG").unwrap_or_else(|_| EnvFilter::new(level));
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .compact()
        .try_init()
        .map_err(|e| anyhow::anyhow!("failed to init tracing: {e}"))?;
    Ok(())
}
