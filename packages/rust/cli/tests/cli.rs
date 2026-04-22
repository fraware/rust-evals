//! Integration tests for the `eval-ladder` binary.
//!
//! These tests exercise the CLI surface we commit to in Milestone A:
//! `version`, `--help`, and `schema validate`. The heavier subcommands are
//! deliberately stubs at this milestone and are not covered here.

use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("eval-ladder").expect("eval-ladder binary must be built")
}

#[test]
fn version_subcommand_prints_crate_version_and_schema_version() {
    bin()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("eval-ladder"))
        .stdout(predicate::str::contains("schema_version="));
}

#[test]
fn help_lists_all_top_level_subcommands() {
    let assert = bin().arg("--help").assert().success();
    let output = assert.get_output().stdout.clone();
    let stdout = String::from_utf8(output).expect("stdout must be UTF-8");
    for expected in [
        "ingest",
        "evaluate",
        "prove-subset",
        "analyze",
        "schema",
        "version",
    ] {
        assert!(
            stdout.contains(expected),
            "help output missing subcommand {expected}: {stdout}"
        );
    }
}

#[test]
fn schema_validate_accepts_shipped_schemas() {
    // Resolve the shipped schemas directory deterministically from this
    // crate's manifest. That makes the test robust to the working directory
    // `cargo test` chooses per platform.
    let manifest = env!("CARGO_MANIFEST_DIR");
    let schemas_dir = std::path::Path::new(manifest)
        .join("..")
        .join("..")
        .join("..")
        .join("schemas");
    assert!(
        schemas_dir.is_dir(),
        "schemas dir not found at {}",
        schemas_dir.display()
    );

    bin()
        .arg("schema")
        .arg("validate")
        .arg("--dir")
        .arg(schemas_dir.as_os_str())
        .assert()
        .success()
        .stdout(
            predicate::str::contains("validated").and(predicate::str::contains("JSON schemas")),
        );
}

#[test]
fn unknown_subcommand_exits_nonzero() {
    bin().arg("frobnicate").assert().failure();
}
