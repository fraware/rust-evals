//! End-to-end ingest tests: raw JSONL fixtures -> schema-valid
//! `BenchmarkTask` manifests. Asserts deterministic and idempotent output.

use std::path::{Path, PathBuf};

use eval_ladder_benchmarks::{adapter_for, BenchmarkTaskValidator, IngestOptions};
use eval_ladder_core::{BenchmarkId, BenchmarkTask};
use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at packages/rust/benchmarks.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace root must exist above the crate")
        .to_path_buf()
}

fn fixture(name: &str) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("benchmarks")
        .join(name)
}

fn read_manifest(path: &Path) -> BenchmarkTask {
    let bytes = std::fs::read(path).expect("read manifest");
    serde_json::from_slice(&bytes).expect("parse manifest")
}

fn run_ingest(
    id: BenchmarkId,
    fixture_name: &str,
    out: &Path,
) -> eval_ladder_benchmarks::IngestReport {
    let adapter = adapter_for(id);
    let opts = IngestOptions {
        only_task_ids: Vec::new(),
        output_dir: Some(out.to_path_buf()),
        limit: None,
    };
    adapter
        .ingest(&fixture(fixture_name), &opts)
        .expect("ingest must succeed on fixtures")
}

#[test]
fn verified_fixtures_produce_schema_valid_manifests() {
    let tmp = TempDir::new().unwrap();
    let report = run_ingest(BenchmarkId::SweBenchVerified, "verified.jsonl", tmp.path());
    assert_eq!(report.tasks_ingested, 2);
    assert_eq!(report.tasks_skipped, 0);

    let v = BenchmarkTaskValidator::new().unwrap();
    let mut files: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    files.sort();
    assert_eq!(files.len(), 2);

    for path in &files {
        let task = read_manifest(path);
        assert_eq!(task.benchmark_id, BenchmarkId::SweBenchVerified);
        v.validate(&task).expect("schema-valid");
    }

    // Exact file names come from task ids.
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"astropy__astropy-12907.json".to_string()));
    assert!(names.contains(&"django__django-11179.json".to_string()));
}

#[test]
fn live_fixtures_produce_schema_valid_manifests() {
    let tmp = TempDir::new().unwrap();
    let report = run_ingest(BenchmarkId::SweBenchLive, "live.jsonl", tmp.path());
    assert_eq!(report.tasks_ingested, 2);
    assert_eq!(report.tasks_skipped, 0);

    let v = BenchmarkTaskValidator::new().unwrap();
    for entry in std::fs::read_dir(tmp.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let task = read_manifest(&path);
        assert_eq!(task.benchmark_id, BenchmarkId::SweBenchLive);
        assert!(
            task.environment_ref.starts_with("swebenchlive/"),
            "Live tasks must use the docker_image from the raw record; got {}",
            task.environment_ref
        );
        assert!(task.labels.contains(&"live".to_string()));
        v.validate(&task).expect("schema-valid");
    }
}

#[test]
fn rust_fixtures_produce_schema_valid_manifests() {
    let tmp = TempDir::new().unwrap();
    let report = run_ingest(BenchmarkId::RustSweBench, "rust.jsonl", tmp.path());
    assert_eq!(report.tasks_ingested, 2);

    let v = BenchmarkTaskValidator::new().unwrap();
    let mut saw_docker = false;
    let mut saw_cargo = false;
    for entry in std::fs::read_dir(tmp.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let task = read_manifest(&path);
        assert_eq!(task.benchmark_id, BenchmarkId::RustSweBench);
        assert_eq!(task.language, eval_ladder_core::BenchmarkLanguage::Rust);
        v.validate(&task).expect("schema-valid");
        if task.environment_ref.starts_with("ghcr.io/") {
            saw_docker = true;
        } else if task.environment_ref.starts_with("cargo://") {
            saw_cargo = true;
        }
    }
    assert!(saw_docker, "expected an explicit docker_image manifest");
    assert!(saw_cargo, "expected a synthesized cargo:// manifest");
}

#[test]
fn ingest_is_deterministic_and_idempotent() {
    // Two consecutive ingests into separate directories produce identical
    // byte sequences for every matching filename.
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    run_ingest(BenchmarkId::SweBenchVerified, "verified.jsonl", a.path());
    run_ingest(BenchmarkId::SweBenchVerified, "verified.jsonl", b.path());

    let mut a_files: Vec<_> = std::fs::read_dir(a.path())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    a_files.sort();
    assert!(!a_files.is_empty());
    for ap in a_files {
        let name = ap.file_name().unwrap().to_owned();
        let bp = b.path().join(&name);
        let ab = std::fs::read(&ap).unwrap();
        let bb = std::fs::read(&bp).unwrap();
        assert_eq!(
            ab,
            bb,
            "manifest {} differs between two ingest runs",
            name.to_string_lossy()
        );
    }

    // Re-ingesting into the same directory also produces identical bytes.
    let before: Vec<Vec<u8>> = sorted_file_bytes(a.path());
    run_ingest(BenchmarkId::SweBenchVerified, "verified.jsonl", a.path());
    let after: Vec<Vec<u8>> = sorted_file_bytes(a.path());
    assert_eq!(before, after, "second ingest must be byte-identical");
}

fn sorted_file_bytes(dir: &Path) -> Vec<Vec<u8>> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths.iter().map(|p| std::fs::read(p).unwrap()).collect()
}

#[test]
fn filter_and_limit_apply() {
    let tmp = TempDir::new().unwrap();
    let adapter = adapter_for(BenchmarkId::SweBenchVerified);
    let opts = IngestOptions {
        only_task_ids: vec!["astropy__astropy-12907".into()],
        output_dir: Some(tmp.path().to_path_buf()),
        limit: None,
    };
    let r = adapter.ingest(&fixture("verified.jsonl"), &opts).unwrap();
    assert_eq!(r.tasks_ingested, 1);

    let tmp2 = TempDir::new().unwrap();
    let opts2 = IngestOptions {
        only_task_ids: Vec::new(),
        output_dir: Some(tmp2.path().to_path_buf()),
        limit: Some(1),
    };
    let r2 = adapter.ingest(&fixture("verified.jsonl"), &opts2).unwrap();
    assert_eq!(r2.tasks_ingested, 1);
}

#[test]
fn live_missing_docker_image_reports_skip() {
    // Compose a temporary Live fixture whose single record lacks docker_image.
    let tmp_src = TempDir::new().unwrap();
    let src = tmp_src.path().join("bad.jsonl");
    std::fs::write(
        &src,
        br#"{"instance_id":"a__b-1","repo":"a/b","base_commit":"deadbeefcafe","problem_statement":"x"}
"#,
    )
    .unwrap();
    let tmp_out = TempDir::new().unwrap();
    let adapter = adapter_for(BenchmarkId::SweBenchLive);
    let opts = IngestOptions {
        only_task_ids: Vec::new(),
        output_dir: Some(tmp_out.path().to_path_buf()),
        limit: None,
    };
    let r = adapter.ingest(&src, &opts).unwrap();
    assert_eq!(r.tasks_ingested, 0);
    assert_eq!(r.tasks_skipped, 1);
    assert!(r.diagnostics.iter().any(|d| d.contains("docker_image")));
}
