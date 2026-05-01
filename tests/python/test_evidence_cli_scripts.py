"""Subprocess integration tests for ``ci/scripts`` evidence utilities."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest


def _load_prewarm_panel_images_module(repo_root: Path) -> Any:
    path = repo_root / "ci/scripts/prewarm_panel_images.py"
    spec = importlib.util.spec_from_file_location("_prewarm_panel_images_test", path)
    if spec is None or spec.loader is None:
        raise AssertionError("failed to load prewarm_panel_images module spec")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


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


def test_check_evidence_quality_verified_registers_all_fail_agents(
    repo_root: Path, tmp_path: Path
) -> None:
    """Degenerate panels must still attribute rows to agents for distinct-vector math."""
    run_dir = tmp_path / "verified_all_fail"
    run_dir.mkdir()
    summary = {
        "entries": [
            {
                "entry_id": f"{agent}__task__t{i}",
                "status": "ok",
                "levels": {
                    "l0": {"status": "fail", "primary_reason": "L0_OFFICIAL_FAIL"},
                    "l1": {"status": "fail", "primary_reason": "L1_HARNESS_ERROR"},
                    "l3": {"status": "fail", "primary_reason": "PV_EDIT_SCOPE"},
                },
            }
            for agent in ("gru", "honeycomb", "sweagent")
            for i in range(10)
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
            "30",
            "--max-l1-harness-error-rate",
            "1.0",
            "--min-distinct-agents",
            "2",
            "--min-nonzero-agents",
            "1",
            "--max-l3-single-reason-share",
            "0.80",
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["metrics"]["distinct_agent_pass_vectors"] == 1
    assert "gru" in report["metrics"]["per_agent_l0_l1_pass"]


def test_check_evidence_quality_live_no_crash_when_live_rates_absent(
    repo_root: Path, tmp_path: Path
) -> None:
    """static_vs_live rows with null live_pass_rate must not crash the gate."""
    export = tmp_path / "paper_live_null_live"
    export.mkdir()
    static_vs_live = [
        {
            "agent_id": "gru",
            "level": "L0",
            "delta": None,
            "live_evaluated": 0,
            "live_pass_rate": None,
            "live_passed": 0,
            "ratio": None,
            "static_evaluated": 5,
            "static_pass_rate": 0.4,
            "static_passed": 2,
        },
        {
            "agent_id": "honeycomb",
            "level": "L0",
            "delta": None,
            "live_evaluated": 0,
            "live_pass_rate": None,
            "live_passed": 0,
            "ratio": None,
            "static_evaluated": 5,
            "static_pass_rate": 0.3,
            "static_passed": 1,
        },
        {
            "agent_id": "sweagent",
            "level": "L0",
            "delta": None,
            "live_evaluated": 0,
            "live_pass_rate": None,
            "live_passed": 0,
            "ratio": None,
            "static_evaluated": 5,
            "static_pass_rate": 0.35,
            "static_passed": 2,
        },
    ]
    (export / "static_vs_live.json").write_text(
        json.dumps(static_vs_live) + "\n", encoding="utf-8"
    )
    (export / "rank_stability.json").write_text(
        json.dumps(
            [{"kendall_tau_b": 1.0, "level_a": "L0", "level_b": "L1", "n_agents": 3}]
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


def test_check_evidence_quality_live_symmetric_ok_passes(
    repo_root: Path, tmp_path: Path
) -> None:
    export = tmp_path / "paper_live_sym"
    export.mkdir()
    static_vs_live = [
        _live_row("gru", "L0", 0.50, delta=-0.1),
        _live_row("honeycomb", "L0", 0.50, delta=-0.1),
        _live_row("sweagent", "L0", 0.50, delta=-0.1),
        _live_row("gru", "L1", 0.50, delta=-0.1),
        _live_row("honeycomb", "L1", 0.50, delta=-0.1),
        _live_row("sweagent", "L1", 0.50, delta=-0.1),
    ]
    (export / "static_vs_live.json").write_text(
        json.dumps(static_vs_live) + "\n", encoding="utf-8"
    )
    (export / "rank_stability.json").write_text(
        json.dumps(
            [{"kendall_tau_b": 0.0, "level_a": "L0", "level_b": "L1", "n_agents": 3}]
        )
        + "\n",
        encoding="utf-8",
    )

    proc = _run_script(
        repo_root,
        "ci/scripts/check_evidence_quality.py",
        [
            "live",
            "--paper-export-dir",
            str(export),
            "--min-agents",
            "3",
            "--symmetric-live-ok",
        ],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    report = json.loads(proc.stdout)
    assert report["ok"] is True
    assert report["metrics"]["symmetric_live_ok"] is True


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


def test_filter_panel_upstream_resolved_help(repo_root: Path) -> None:
    proc = _run_script(
        repo_root,
        "ci/scripts/filter_panel_upstream_resolved.py",
        ["--help"],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout


def test_prewarm_panel_images_help(repo_root: Path) -> None:
    proc = _run_script(
        repo_root,
        "ci/scripts/prewarm_panel_images.py",
        ["--help"],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    assert "--strict-pulls" in proc.stdout


def test_prewarm_image_pull_candidates_matches_container_rs(repo_root: Path) -> None:
    m = _load_prewarm_panel_images_module(repo_root)
    c = m.image_pull_candidates(
        "swebench/sweb.eval.x86_64.astropy__astropy-12907:latest"
    )
    assert c == [
        "swebench/sweb.eval.x86_64.astropy__astropy-12907:latest",
        "swebench/sweb.eval.x86_64.astropy_1776_astropy-12907:latest",
    ]
    assert m.image_pull_candidates("ubuntu:22.04") == ["ubuntu:22.04"]
    assert m.map_legacy_swebench_image("starryzhang/foo:1") is None


def test_prewarm_panel_images_dry_run_collects_refs(
    repo_root: Path, tmp_path: Path
) -> None:
    task_dir = tmp_path / "benchmarks" / "verified" / "manifests"
    task_dir.mkdir(parents=True)
    task_path = task_dir / "demo__task-1.json"
    task_path.write_text(
        json.dumps(
            {
                "task_id": "demo__task-1",
                "benchmark_id": "swe_bench_verified",
                "environment_ref": "swebench/sweb.eval.x86_64.demo__task-1:latest",
                "official_test_entrypoint": "python -m pytest -q",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    panel = tmp_path / "panel.jsonl"
    rel_task = "benchmarks/verified/manifests/demo__task-1.json"
    panel.write_text(
        json.dumps(
            {
                "task": rel_task,
                "candidate": "c.json",
                "patch": "p.diff",
                "workspace_template": "w",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    proc = _run_script(
        repo_root,
        "ci/scripts/prewarm_panel_images.py",
        ["--panel", str(panel), "--dry-run"],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    assert "swebench/sweb.eval.x86_64.demo__task-1:latest" in proc.stdout
    assert "docker pull targets: 1" in proc.stdout


def test_prewarm_panel_images_dry_run_skips_cargo_refs(
    repo_root: Path, tmp_path: Path
) -> None:
    task_dir = tmp_path / "benchmarks" / "rust" / "manifests"
    task_dir.mkdir(parents=True)
    task_path = task_dir / "demo__ripgrep.json"
    task_path.write_text(
        json.dumps(
            {
                "task_id": "demo__ripgrep",
                "benchmark_id": "rust_swe_bench",
                "environment_ref": "cargo://BurntSushi/ripgrep@c50b8b4125dc",
                "official_test_entrypoint": "cargo test -q",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    panel = tmp_path / "panel.jsonl"
    rel_task = "benchmarks/rust/manifests/demo__ripgrep.json"
    panel.write_text(
        json.dumps(
            {
                "task": rel_task,
                "candidate": "c.json",
                "patch": "p.diff",
                "workspace_template": "w",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    proc = _run_script(
        repo_root,
        "ci/scripts/prewarm_panel_images.py",
        ["--panel", str(panel), "--dry-run"],
    )
    assert proc.returncode == 0, proc.stderr + proc.stdout
    assert "docker pull targets: 0" in proc.stdout
    assert "skipped (not docker pull targets)" in proc.stderr
    assert "cargo://BurntSushi/ripgrep@c50b8b4125dc" in proc.stderr


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


def test_write_release_artifact_manifest_require_all_succeeds_on_repo(
    repo_root: Path, tmp_path: Path
) -> None:
    out_path = tmp_path / "release_manifest_strict.json"
    proc = _run_script(
        repo_root,
        "ci/scripts/write_release_artifact_manifest.py",
        ["--repo-root", str(repo_root), "--require-all-files", "--out", str(out_path)],
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
    manifest = json.loads(proc.stdout)
    assert manifest["missing_paths"] == []
    assert len(manifest["files_sha256"]) == 7


def test_write_release_artifact_manifest_require_all_fails_without_out_write(
    repo_root: Path, tmp_path: Path
) -> None:
    incomplete = tmp_path / "partial_repo"
    incomplete.mkdir()
    (incomplete / "rust-toolchain.toml").write_text(
        '[toolchain]\nchannel = "1.70.0"\n', encoding="utf-8"
    )
    out_path = tmp_path / "should_not_exist.json"
    proc = _run_script(
        repo_root,
        "ci/scripts/write_release_artifact_manifest.py",
        [
            "--repo-root",
            str(incomplete),
            "--require-all-files",
            "--out",
            str(out_path),
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    assert proc.stdout == ""
    err = json.loads(proc.stderr)
    assert err["error"] == "missing_required_paths"
    assert len(err["missing_paths"]) >= 1
    assert not out_path.exists()


def test_audit_strict_fails_on_expect_manifest_count_mismatch(
    repo_root: Path, tmp_path: Path
) -> None:
    mdir = tmp_path / "manifests_two"
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
            "3",
        ],
    )
    assert proc.returncode == 2, proc.stdout + proc.stderr
    report = json.loads(proc.stdout)
    assert report["ok"] is False
    assert any("manifest count" in f for f in report["failures"])


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


def test_build_verified_flagship_v1_help(repo_root: Path) -> None:
    proc = _run_script(
        repo_root,
        "packages/python/scripts/build_verified_flagship_v1.py",
        ["--help"],
    )
    assert proc.returncode == 0, proc.stderr
    assert "high-precision" in proc.stdout.lower() or "flagship" in proc.stdout.lower()


def test_build_live_panel_v2_py_compile(repo_root: Path) -> None:
    """Syntax check without importing (dataclass loader needs __package__)."""
    path = repo_root / "packages/python/scripts/build_live_panel_v2.py"
    proc = subprocess.run(
        [sys.executable, "-m", "py_compile", str(path)],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, proc.stderr


def test_analyze_strict_feasibility_help(repo_root: Path) -> None:
    proc = _run_script(
        repo_root,
        "ci/scripts/analyze_strict_feasibility.py",
        ["--help"],
    )
    assert proc.returncode == 0, proc.stderr
    assert "offline" in proc.stdout.lower() and "strict" in proc.stdout.lower()


def test_analyze_strict_feasibility_py_compile(repo_root: Path) -> None:
    path = repo_root / "ci/scripts/analyze_strict_feasibility.py"
    proc = subprocess.run(
        [sys.executable, "-m", "py_compile", str(path)],
        cwd=repo_root,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, proc.stderr


def _load_export_l2_flagship_tables(repo_root: Path) -> Any:
    path = repo_root / "packages/python/scripts/export_l2_flagship_tables.py"
    spec = importlib.util.spec_from_file_location("_export_l2_flagship_tables_test", path)
    if spec is None or spec.loader is None:
        raise AssertionError("failed to load export_l2_flagship_tables module spec")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def test_write_l2_paper_export_manifest_row_count(tmp_path: Path, repo_root: Path) -> None:
    mod = _load_export_l2_flagship_tables(repo_root)
    out_dir = tmp_path / "l2_export"
    out_dir.mkdir()
    for rel in mod._L2_PAPER_EXPORT_MANIFEST_PATHS:
        dest = out_dir / rel
        if rel.endswith(".json"):
            dest.write_bytes(b"{}")
        else:
            dest.write_bytes(b"#stub\n")
    mod._write_l2_paper_export_manifest(
        out_dir,
        input_row_count=42,
        evaluator_version="9.9.9-test",
    )
    loaded = json.loads((out_dir / "manifest.json").read_text(encoding="utf-8"))
    assert loaded["schema_version"] == 3
    assert loaded["analysis_mode"] == "cumulative"
    assert loaded["input_row_count"] == 42
    assert loaded["evaluator_version"] == "9.9.9-test"
    assert len(loaded["files"]) == len(mod._L2_PAPER_EXPORT_MANIFEST_PATHS)


def test_secret_scan_release_passes_on_repo(repo_root: Path) -> None:
    proc = _run_script(repo_root, "ci/scripts/secret_scan_release.py", [])
    assert proc.returncode == 0, proc.stderr + proc.stdout


def test_check_paper_claim_sources_passes_on_repo(repo_root: Path) -> None:
    proc = _run_script(repo_root, "ci/scripts/check_paper_claim_sources.py", [])
    assert proc.returncode == 0, proc.stderr + proc.stdout
