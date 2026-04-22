//! `eval-ladder schema validate`
//!
//! Validates the shipped JSON schemas under `schemas/` at the workspace root.
//! Each schema must be parseable JSON, must declare
//! `$schema = "https://json-schema.org/draft/2020-12/schema"`, and must
//! declare a unique `$id`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde_json::Value;
use tracing::info;
use walkdir::WalkDir;

/// Schema subcommands.
#[derive(Debug, Subcommand)]
pub enum SchemaCmd {
    /// Validate every schema under `schemas/` at the workspace root.
    Validate {
        /// Override the schemas directory (defaults to `<workspace>/schemas`).
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

/// Entrypoint.
pub fn run(cmd: SchemaCmd) -> Result<()> {
    match cmd {
        SchemaCmd::Validate { dir } => validate(dir.as_deref()),
    }
}

fn workspace_schemas_dir() -> PathBuf {
    // Cargo-run from any workspace member places CARGO_MANIFEST_DIR at the
    // crate root, so walk up to the workspace root by searching for
    // `schemas/` alongside `Cargo.toml`.
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let mut probe = here.clone();
    for _ in 0..6 {
        let candidate = probe.join("schemas");
        if candidate.is_dir() {
            return candidate;
        }
        if !probe.pop() {
            break;
        }
    }
    here.join("..").join("..").join("..").join("schemas")
}

fn validate(dir_override: Option<&Path>) -> Result<()> {
    let dir = dir_override.map_or_else(workspace_schemas_dir, Path::to_path_buf);
    info!(dir = %dir.display(), "validating JSON schemas");

    if !dir.is_dir() {
        bail!("schemas directory not found at {}", dir.display());
    }

    const REQUIRED_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut checked = 0_u32;

    for entry in WalkDir::new(&dir) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let rel = path
            .strip_prefix(&dir)
            .unwrap_or(path)
            .display()
            .to_string();
        let bytes = std::fs::read(path).with_context(|| format!("reading {rel}"))?;
        let value: Value =
            serde_json::from_slice(&bytes).with_context(|| format!("parsing {rel}"))?;
        let Value::Object(map) = &value else {
            bail!("{rel}: top-level schema must be an object");
        };
        let Some(dialect) = map.get("$schema").and_then(Value::as_str) else {
            bail!("{rel}: missing $schema");
        };
        if dialect != REQUIRED_DIALECT {
            bail!("{rel}: $schema = {dialect:?}; expected {REQUIRED_DIALECT:?}");
        }
        let Some(id) = map.get("$id").and_then(Value::as_str) else {
            bail!("{rel}: missing $id");
        };
        if !seen_ids.insert(id.to_owned()) {
            bail!("{rel}: duplicate $id {id:?}");
        }
        if !map.contains_key("title") {
            bail!("{rel}: missing title");
        }
        checked += 1;
        info!(rel = %rel, id = %id, "ok");
    }

    println!("validated {checked} JSON schemas in {}", dir.display());
    Ok(())
}
