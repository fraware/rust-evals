//! Cross-language round-trip for Milestone I.
//!
//! Verifies that a `BenchmarkTask` JSON file emitted by the Python
//! compat layer (`benchmark_compat.cli normalize-swe-bench`) deserializes
//! byte-perfectly into the Rust `BenchmarkTask` type. This pins the
//! contract that the pydantic model on the Python side and the serde
//! type on the Rust side stay in lockstep.
//!
//! The test is robust to Python being absent: if no working Python
//! interpreter with `benchmark_compat` installed can be found, the test
//! prints a skip message and exits zero. CI Tier 2 installs the
//! package explicitly, so the test exercises the full pipeline there.

use std::path::{Path, PathBuf};
use std::process::Command;

use eval_ladder_core::{BenchmarkId, BenchmarkLanguage, BenchmarkTask};
use serde_json::json;
use tempfile::TempDir;

fn repo_root() -> PathBuf {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    here.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(here)
}

fn python_available() -> Option<String> {
    for candidate in ["python3", "python", "py"] {
        let out = Command::new(candidate)
            .arg("-c")
            .arg("import benchmark_compat; print('ok')")
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                return Some(candidate.to_owned());
            }
        }
    }
    None
}

fn write_sample_manifest(path: &Path) {
    let instance = json!({
        "instance_id": "octo-org__widget-7277",
        "repo": "octo-org/widget",
        "base_commit": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        "problem_statement": "Widget panics on empty input\n\nDetails.",
        "patch": "diff --git a/w.py b/w.py\n+pass\n",
        "version": "1.2",
        "FAIL_TO_PASS": ["tests/test_widget.py::test_empty"],
        "PASS_TO_PASS": ["tests/test_widget.py::test_basic"],
        "created_at": "2024-07-01T12:34:56Z",
    });
    let line = serde_json::to_string(&instance).unwrap();
    std::fs::write(path, format!("{line}\n")).unwrap();
}

#[test]
fn milestone_i_python_emitted_benchmark_task_deserializes_in_rust() {
    let Some(python) = python_available() else {
        eprintln!(
            "milestone_i_python_emitted_benchmark_task_deserializes_in_rust: \
             benchmark_compat is not importable; skipping. \
             This is expected in Tier 1 CI where Python is not installed."
        );
        return;
    };

    let tmp = TempDir::new().expect("tempdir");
    let manifest = tmp.path().join("manifest.jsonl");
    let out_dir = tmp.path().join("out");
    write_sample_manifest(&manifest);

    let output = Command::new(&python)
        .args([
            "-m",
            "benchmark_compat.cli",
            "normalize-swe-bench",
            "--source",
        ])
        .arg(&manifest)
        .arg("--out-dir")
        .arg(&out_dir)
        .current_dir(repo_root())
        .output()
        .expect("running benchmark_compat CLI");
    assert!(
        output.status.success(),
        "benchmark_compat normalize-swe-bench failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let emitted = out_dir.join("octo-org__widget-7277.json");
    assert!(
        emitted.is_file(),
        "expected emitted file {}",
        emitted.display()
    );

    let bytes = std::fs::read(&emitted).unwrap();
    let task: BenchmarkTask = serde_json::from_slice(&bytes)
        .expect("Python-emitted BenchmarkTask must deserialize in Rust");

    assert_eq!(task.benchmark_id, BenchmarkId::SweBenchVerified);
    assert_eq!(task.task_id.as_str(), "octo-org__widget-7277");
    assert_eq!(task.language, BenchmarkLanguage::Python);
    assert_eq!(
        task.environment_ref,
        "swebench/sweb.eval.x86_64.octo-org__widget-7277:latest"
    );
    assert!(task
        .gold_patch_ref
        .as_deref()
        .unwrap_or("")
        .starts_with("sha256:"));
    assert!(task
        .labels
        .contains(&"benchmark:swe_bench_verified".to_owned()));
    assert!(task.labels.contains(&"version:1.2".to_owned()));

    // Reserialize through serde and ensure the Python bytes and Rust
    // canonical bytes agree. The Rust `writer.rs` convention for
    // on-disk `BenchmarkTask` manifest files is `canonical_json + "\n"`;
    // the Python CLI mirrors that by appending `\n` at write time. We
    // reproduce the same convention here so this is an exact byte test
    // of the cross-language on-disk format.
    let mut rust_bytes = eval_ladder_core::canonical_json(&task).expect("canonical_json");
    rust_bytes.push(b'\n');
    assert_eq!(
        bytes, rust_bytes,
        "Python-emitted on-disk bytes must equal Rust canonical_json(BenchmarkTask) + '\\n'"
    );
}
