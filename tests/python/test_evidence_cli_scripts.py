"""Subprocess integration tests for ``ci/scripts`` evidence utilities."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any


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
