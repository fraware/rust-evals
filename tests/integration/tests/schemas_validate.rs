//! Validate every shipped JSON schema against the JSON Schema draft 2020-12
//! dialect, and validate fixture JSON files against their schemas.

use std::path::{Path, PathBuf};

use jsonschema::Validator;
use serde_json::Value;

fn repo_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` points to `tests/integration`. The repo root is
    // two levels up.
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    here.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(here)
}

fn load_json(path: &Path) -> Value {
    let bytes =
        std::fs::read(path).unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    serde_json::from_slice(&bytes).unwrap_or_else(|err| panic!("parsing {}: {err}", path.display()))
}

fn compile(schema: &Value) -> Validator {
    jsonschema::draft202012::new(schema).expect("schema must compile under draft 2020-12")
}

#[test]
fn every_schema_compiles_as_draft_2020_12() {
    let root = repo_root();
    let schemas_dir = root.join("schemas");
    assert!(
        schemas_dir.is_dir(),
        "schemas dir not found at {}",
        schemas_dir.display()
    );
    let mut compiled = 0_u32;
    for entry in std::fs::read_dir(&schemas_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let schema = load_json(&path);
        let _validator = compile(&schema);
        compiled += 1;
    }
    assert!(compiled >= 7, "expected at least 7 schemas, got {compiled}");
}

#[test]
fn benchmark_task_fixture_validates() {
    let root = repo_root();
    let schema = load_json(&root.join("schemas").join("benchmark_task.schema.json"));
    let validator = compile(&schema);
    let fixture = load_json(
        &root
            .join("tests")
            .join("fixtures")
            .join("sample_benchmark_task.json"),
    );
    let result = validator.validate(&fixture);
    assert!(result.is_ok(), "fixture invalid: {:?}", result.err());
}
