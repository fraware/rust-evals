"""Subprocess integration tests for ``ci/scripts`` evidence utilities."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest


def _run_script(
    repo_root: Path, script_rel: str, args: list[str]
) -> subprocess.CompletedProcess[str]:
    script = repo_root / script_rel
    return subprocess.run(
        [sys.executable, str(script), *args],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )


def _tiny_manifest(task_id: str, entrypoint: str) -> dict[str, Any]:
    slug = task_id.lower()
    return {
        "task_id": task_id,
        "benchmark_id": "swe_bench_verified",
        "environment_ref": f"swebench/sweb.eval.x86_64.{slug}:latest",
        "official_test_entrypoint": entrypoint,
    }


def test_preflight_verified_selectors_strict_passes(
    repo_root: Path, tmp_path: Path
) -> None:
    panel_dir = tmp_path / "panel_run"
    man_dir = panel_dir / "tasks"
    man_dir.mkdir(parents=True)
    ws = panel_dir / "workspaces" / "tiny__task-1" / "pkg"
    ws.mkdir(parents=True)
    (ws / "test_x.py").write_text("# stub\n", encoding="utf-8")

    man_path = man_dir / "tiny.json"
    man_path.write_text(
        json.dumps(
            _tiny_manifest(
                "tiny__task-1",
                "python -m pytest pkg/test_x.py::trivial",
            )
        )
        + "\n",
        encoding="utf-8",
    )

    panel_path = panel_dir / "panel.jsonl"
    panel_path.write_text(
        json.dumps(
            {
                "task": "tasks/tiny.json",
                "workspace_template": "workspaces/tiny__task-1/",
                "bundle_name": "gru__tiny__task-1",
                "entry_id": "gru__tiny__task-1",
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/preflight_verified_selectors.py",
        ["--panel", str(panel_path), "--strict", "--min-tasks", "1"],
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
    out = json.loads(proc.stdout)
    assert out["summary"]["selector_file_errors"] == 0


def test_preflight_verified_selectors_strict_fails_on_missing_file(
    repo_root: Path, tmp_path: Path
) -> None:
    panel_dir = tmp_path / "panel_bad"
    man_dir = panel_dir / "tasks"
    man_dir.mkdir(parents=True)
    (panel_dir / "workspaces" / "tiny__task-2").mkdir(parents=True)

    man_path = man_dir / "tiny2.json"
    man_path.write_text(
        json.dumps(
            _tiny_manifest(
                "tiny__task-2",
                "python -m pytest pkg/missing.py::nope",
            )
        )
        + "\n",
        encoding="utf-8",
    )

    panel_path = panel_dir / "panel.jsonl"
    panel_path.write_text(
        json.dumps(
            {
                "task": "tasks/tiny2.json",
                "workspace_template": "workspaces/tiny__task-2/",
                "bundle_name": "gru__tiny__task-2",
                "entry_id": "gru__tiny__task-2",
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/preflight_verified_selectors.py",
        ["--panel", str(panel_path), "--strict", "--min-tasks", "1"],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr


def test_preflight_min_tasks_exit_code(repo_root: Path, tmp_path: Path) -> None:
    """Panel with only Live benchmark rows yields zero Verified tasks."""
    panel_dir = tmp_path / "panel_live_only"
    man_dir = panel_dir / "tasks"
    man_dir.mkdir(parents=True)
    man_path = man_dir / "live.json"
    man_path.write_text(
        json.dumps(
            {
                "task_id": "x__y-1",
                "benchmark_id": "swe_bench_live",
                "environment_ref": "swebench/sweb.eval.x86_64.x__y-1:latest",
                "official_test_entrypoint": "python -m pytest t.py",
            }
        )
        + "\n",
        encoding="utf-8",
    )

    panel_path = panel_dir / "panel.jsonl"
    panel_path.write_text(
        json.dumps(
            {
                "task": "tasks/live.json",
                "workspace_template": "workspaces/x/",
                "bundle_name": "gru__x",
                "entry_id": "gru__x",
            },
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/preflight_verified_selectors.py",
        ["--panel", str(panel_path), "--min-tasks", "1"],
    )
    assert proc.returncode == 3, proc.stdout + proc.stderr


def test_audit_verified_manifest_entrypoints_strict_passes(
    repo_root: Path, tmp_path: Path
) -> None:
    mdir = tmp_path / "manifests"
    mdir.mkdir()
    for i in (1, 2):
        tid = f"tinybench__case-{i}"
        (mdir / f"{tid}.json").write_text(
            json.dumps(
                _tiny_manifest(tid, "python -m pytest tests/test_a.py::x")
            )
            + "\n",
            encoding="utf-8",
        )

    proc = _run_script(
        repo_root,
        "ci/scripts/audit_verified_manifest_entrypoints.py",
        [
            "--manifest-dir",
            str(mdir),
            "--strict",
            "--expect-manifest-count",
            "2",
        ],
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr


def test_audit_strict_fails_on_empty_entrypoint(
    repo_root: Path, tmp_path: Path
) -> None:
    mdir = tmp_path / "manifests_bad"
    mdir.mkdir()
    bad = _tiny_manifest("tinybench__bad-1", "")
    bad["official_test_entrypoint"] = ""
    (mdir / "tinybench__bad-1.json").write_text(
        json.dumps(bad) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/audit_verified_manifest_entrypoints.py",
        ["--manifest-dir", str(mdir), "--strict", "--expect-manifest-count", "1"],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr


def test_check_evidence_quality_verified_minimal_pass(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "verified_run"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": "gru__tiny__task-1",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                },
            },
            {
                "entry_id": "honeycomb__tiny__task-2",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {"status": "fail", "primary_reason": "L0_OFFICIAL_FAIL"},
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                },
            },
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "verified",
            "--run-dir",
            str(run_dir),
            "--min-candidates",
            "2",
            "--max-l1-harness-error-rate",
            "0.10",
            "--min-distinct-agents",
            "2",
            "--min-nonzero-agents",
            "2",
            "--max-l3-single-reason-share",
            "0.80",
        ],
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is True
    assert report["metrics"]["distinct_agent_pass_vectors"] == 2


def _live_row(
    agent_id: str,
    level: str,
    live_pass_rate: float,
    *,
    delta: float = -0.05,
) -> dict[str, Any]:
    return {
        "agent_id": agent_id,
        "level": level,
        "delta": delta,
        "live_evaluated": 10,
        "live_pass_rate": live_pass_rate,
        "live_passed": 1,
        "ratio": 0.5,
        "static_evaluated": 10,
        "static_pass_rate": 0.5,
        "static_passed": 5,
    }


def test_diagnose_batch_summary_fail_on_warnings(
    repo_root: Path, tmp_path: Path
) -> None:
    summary_path = tmp_path / "batch_summary.json"
    summary = {
        "entries": [
            {
                "entry_id": "a__t",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {
                        "status": "fail",
                        "primary_reason": "L1_HARNESS_ERROR",
                    },
                },
            }
        ]
    }
    summary_path.write_text(json.dumps(summary) + "\n", encoding="utf-8")

    proc = _run_script(
        repo_root,
        "ci/scripts/diagnose_batch_summary.py",
        [
            "--summary",
            str(summary_path),
            "--l1-harness-threshold",
            "0.10",
            "--fail-on-warnings",
        ],
    )
    assert proc.returncode == 2, proc.stderr + proc.stdout


def test_diagnose_batch_summary_emits_metrics_and_warnings(
    repo_root: Path, tmp_path: Path
) -> None:
    summary_path = tmp_path / "batch_summary.json"
    summary = {
        "entries": [
            {
                "entry_id": f"a{i}__t",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {
                        "status": "fail",
                        "primary_reason": "L1_HARNESS_ERROR",
                    },
                    "l3": {"status": "fail", "primary_reason": "PV_EDIT_SCOPE"},
                },
            }
            for i in range(3)
        ]
    }
    summary_path.write_text(json.dumps(summary) + "\n", encoding="utf-8")

    proc = _run_script(
        repo_root,
        "ci/scripts/diagnose_batch_summary.py",
        ["--summary", str(summary_path), "--l1-harness-threshold", "0.10"],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    out = json.loads(proc.stdout)
    assert out["total_entries"] == 3
    assert out["metrics"]["l1_harness_error_rate"] == pytest.approx(1.0)
    assert len(out["warnings"]) >= 1


def test_triage_l1_harness_errors_clusters_stderr(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "triage_run"
    run_dir.mkdir()
    bundle = "gru__demo__task-1"
    (run_dir / bundle).mkdir(parents=True)
    (run_dir / bundle / "stderr.log").write_text(
        "ERROR: not found: /workspace/t.py::missing\n", encoding="utf-8"
    )
    summary = {
        "entries": [
            {
                "bundle_name": bundle,
                "status": "ok",
                "levels": {
                    "l1": {
                        "status": "fail",
                        "primary_reason": "L1_HARNESS_ERROR",
                    },
                },
            }
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/triage_l1_harness_errors.py",
        ["--run-dir", str(run_dir)],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    out = json.loads(proc.stdout)
    assert out["l1_harness_error_entries"] == 1
    assert out["distinct_stderr_sha256"] >= 1
    assert "pytest_selector_not_found" in out["bucket_bundle_counts"]


def test_check_evidence_quality_live_minimal_pass(
    repo_root: Path, tmp_path: Path
) -> None:
    export = tmp_path / "paper_live"
    export.mkdir()
    static_vs_live = [
        _live_row("gru", "L0", 0.10),
        _live_row("honeycomb", "L0", 0.25),
        _live_row("sweagent", "L0", 0.18),
        _live_row("gru", "L1", 0.40),
        _live_row("honeycomb", "L1", 0.55),
        _live_row("sweagent", "L1", 0.48),
    ]
    (export / "static_vs_live.json").write_text(
        json.dumps(static_vs_live) + "\n", encoding="utf-8"
    )
    rank_stability = [
        {
            "kendall_tau_b": 0.33,
            "level_a": "L0",
            "level_b": "L1",
            "n_agents": 3,
        }
    ]
    (export / "rank_stability.json").write_text(
        json.dumps(rank_stability) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        ["live", "--paper-export-dir", str(export), "--min-agents", "3"],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    report = json.loads(proc.stdout)
    assert report["ok"] is True
    assert report["metrics"]["non_tied_levels"] >= 1


def test_check_evidence_quality_l2_minimal_pass(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "l2_run"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": "gru__t1",
                "status": "ok",
                "levels": {
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l2": {"status": "fail", "primary_reason": "L2_AUG_TESTS_FAIL"},
                },
            },
            {
                "entry_id": "honeycomb__t2",
                "status": "ok",
                "levels": {
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l2": {"status": "fail", "primary_reason": "L2_TIMEOUT"},
                },
            },
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "l2",
            "--run-dir",
            str(run_dir),
            "--min-l1-passed-from",
            "2",
            "--min-l2-failures",
            "2",
            "--min-l2-reason-families",
            "2",
        ],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    report = json.loads(proc.stdout)
    assert report["ok"] is True


def test_check_evidence_quality_rust_proof_structural_pass(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "rust_run"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": f"golden_agent__task{i}",
                "status": "ok",
                "levels": {
                    "l0": {"status": "fail", "primary_reason": "L0_OFFICIAL_FAIL"},
                    "l1": {"status": "fail", "primary_reason": "L1_HARNESS_ERROR"},
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                    "l4": {"status": "pass", "primary_reason": "L4_OBLIGATION_MET"},
                },
            }
            for i in range(2)
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "rust-proof",
            "--run-dir",
            str(run_dir),
            "--expected-entries",
            "2",
            "--min-l3-pass-l4-fail",
            "0",
            "--min-all-level-pass",
            "0",
        ],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    report = json.loads(proc.stdout)
    assert report["ok"] is True
    assert report["metrics"]["ok_entries"] == 2


def test_run_evidence_tier1_checks_passes_on_repo(repo_root: Path) -> None:
    """Integration: same steps as ci-tier1-fast evidence-tranche-scripts."""
    proc = _run_script(repo_root, "ci/scripts/run_evidence_tier1_checks.py", [])
    assert proc.returncode == 0, proc.stdout + proc.stderr
    assert "all steps passed" in proc.stderr


def test_write_release_artifact_manifest_stdout_and_out(
    repo_root: Path, tmp_path: Path
) -> None:
    out_path = tmp_path / "release_manifest.json"
    proc = _run_script(
        repo_root,
        "ci/scripts/write_release_artifact_manifest.py",
        ["--repo-root", str(repo_root), "--out", str(out_path)],
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
    manifest = json.loads(proc.stdout)
    assert manifest["schema_version"] == 1
    assert "generated_at_utc" in manifest
    assert "files_sha256" in manifest
    assert "missing_paths" in manifest
    assert isinstance(manifest["missing_paths"], list)
    toolchain = "rust-toolchain.toml"
    assert toolchain in manifest["files_sha256"]
    assert manifest["files_sha256"][toolchain].startswith("sha256:")
    assert out_path.read_text(encoding="utf-8") == proc.stdout


def test_check_evidence_quality_verified_fails_high_harness_rate(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "verified_bad_harness"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": f"{agent}__t{i}",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {
                        "status": "fail",
                        "primary_reason": "L1_HARNESS_ERROR",
                    },
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                },
            }
            for i, agent in enumerate(("gru", "honeycomb", "sweagent"))
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "verified",
            "--run-dir",
            str(run_dir),
            "--min-candidates",
            "3",
            "--max-l1-harness-error-rate",
            "0.10",
            "--min-distinct-agents",
            "2",
            "--min-nonzero-agents",
            "2",
            "--max-l3-single-reason-share",
            "0.80",
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("L1_HARNESS_ERROR rate" in f for f in report["failures"])


def test_check_evidence_quality_verified_fails_degenerate_pass_vectors(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "verified_degenerate"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": "gru__t1",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                },
            },
            {
                "entry_id": "honeycomb__t2",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                },
            },
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "verified",
            "--run-dir",
            str(run_dir),
            "--min-candidates",
            "2",
            "--max-l1-harness-error-rate",
            "1.0",
            "--min-distinct-agents",
            "2",
            "--min-nonzero-agents",
            "2",
            "--max-l3-single-reason-share",
            "0.80",
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("distinct agent" in f.lower() for f in report["failures"])


def test_check_evidence_quality_live_fails_tied_live_rates(
    repo_root: Path, tmp_path: Path
) -> None:
    export = tmp_path / "paper_live_tied"
    export.mkdir()
    static_vs_live = [
        _live_row("gru", "L0", 0.50),
        _live_row("honeycomb", "L0", 0.50),
        _live_row("sweagent", "L0", 0.50),
        _live_row("gru", "L1", 0.50),
        _live_row("honeycomb", "L1", 0.50),
        _live_row("sweagent", "L1", 0.50),
    ]
    (export / "static_vs_live.json").write_text(
        json.dumps(static_vs_live) + "\n", encoding="utf-8"
    )
    (export / "rank_stability.json").write_text(
        json.dumps(
            [{"kendall_tau_b": 0.5, "level_a": "L0", "level_b": "L1", "n_agents": 3}]
        )
        + "\n",
        encoding="utf-8",
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        ["live", "--paper-export-dir", str(export), "--min-agents", "3"],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("non-tied" in f.lower() for f in report["failures"])


def test_check_evidence_quality_live_fails_only_zero_tau(
    repo_root: Path, tmp_path: Path
) -> None:
    export = tmp_path / "paper_live_tau0"
    export.mkdir()
    static_vs_live = [
        _live_row("gru", "L0", 0.10),
        _live_row("honeycomb", "L0", 0.25),
        _live_row("sweagent", "L0", 0.18),
    ]
    (export / "static_vs_live.json").write_text(
        json.dumps(static_vs_live) + "\n", encoding="utf-8"
    )
    (export / "rank_stability.json").write_text(
        json.dumps(
            [
                {
                    "kendall_tau_b": 0.0,
                    "level_a": "L0",
                    "level_b": "L1",
                    "n_agents": 3,
                }
            ]
        )
        + "\n",
        encoding="utf-8",
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        ["live", "--paper-export-dir", str(export), "--min-agents", "3"],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("tau" in f.lower() for f in report["failures"])


def test_check_evidence_quality_live_fails_non_negative_delta(
    repo_root: Path, tmp_path: Path
) -> None:
    export = tmp_path / "paper_live_bad_delta"
    export.mkdir()
    static_vs_live = [
        _live_row("gru", "L0", 0.10, delta=0.01),
        _live_row("honeycomb", "L0", 0.25),
        _live_row("sweagent", "L0", 0.18),
    ]
    (export / "static_vs_live.json").write_text(
        json.dumps(static_vs_live) + "\n", encoding="utf-8"
    )
    (export / "rank_stability.json").write_text(
        json.dumps(
            [{"kendall_tau_b": 0.33, "level_a": "L0", "level_b": "L1", "n_agents": 3}]
        )
        + "\n",
        encoding="utf-8",
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        ["live", "--paper-export-dir", str(export), "--min-agents", "3"],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("delta" in f.lower() for f in report["failures"])


def test_check_evidence_quality_l2_fails_insufficient_l2_failures(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "l2_thin"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": "gru__t1",
                "status": "ok",
                "levels": {
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l2": {"status": "fail", "primary_reason": "L2_AUG_TESTS_FAIL"},
                },
            },
            {
                "entry_id": "honeycomb__t2",
                "status": "ok",
                "levels": {
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l2": {"status": "pass", "primary_reason": "PASS"},
                },
            },
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "l2",
            "--run-dir",
            str(run_dir),
            "--min-l1-passed-from",
            "2",
            "--min-l2-failures",
            "2",
            "--min-l2-reason-families",
            "1",
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("l2 failures" in f.lower() for f in report["failures"])


def test_check_evidence_quality_rust_proof_fails_invalid_entry(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "rust_invalid"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": "a__t1",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                    "l4": {"status": "fail", "primary_reason": "X"},
                },
            },
            {
                "entry_id": "a__t2",
                "status": "invalid_bundle",
                "levels": {},
            },
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "rust-proof",
            "--run-dir",
            str(run_dir),
            "--expected-entries",
            "2",
            "--min-l3-pass-l4-fail",
            "0",
            "--min-all-level-pass",
            "0",
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("invalid" in f.lower() for f in report["failures"])


def test_check_evidence_quality_rust_proof_fails_semantic_minima(
    repo_root: Path, tmp_path: Path
) -> None:
    run_dir = tmp_path / "rust_semantic"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": f"agent__t{i}",
                "status": "ok",
                "levels": {
                    "l0": {"status": "pass", "primary_reason": "PASS"},
                    "l1": {"status": "pass", "primary_reason": "PASS"},
                    "l3": {"status": "pass", "primary_reason": "PASS"},
                    "l4": {"status": "pass", "primary_reason": "PASS"},
                },
            }
            for i in range(2)
        ]
    }
    (run_dir / "batch_summary.json").write_text(
        json.dumps(summary) + "\n", encoding="utf-8"
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "rust-proof",
            "--run-dir",
            str(run_dir),
            "--expected-entries",
            "2",
            "--min-l3-pass-l4-fail",
            "1",
            "--min-all-level-pass",
            "0",
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("l3-pass/l4-fail" in f.lower() for f in report["failures"])
