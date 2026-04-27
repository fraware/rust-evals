#!/usr/bin/env python3
"""Run L2 flagship validators against upstream gold/developer patches.

Outputs:
- paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv
- paper/exports/l2_verified_flagship_v1/gold_patch_validation.json
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import shutil
import subprocess
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
RUN_ROOT = REPO_ROOT / "runs" / "released" / "l2_verified_flagship_v1"
RESULTS_MERGED = RUN_ROOT / "results" / "batch_summary.json"

VERIFIED_CACHE = (
    REPO_ROOT / "datasets" / "cache" / "verified" / "swe_bench_verified.jsonl"
)
MANIFEST_DIR = REPO_ROOT / "benchmarks" / "verified" / "manifests"
WORKSPACES_DIR = REPO_ROOT / "runs" / "released" / "agent_panel_v3_r1" / "workspaces"

OUT_DIR = REPO_ROOT / "paper" / "exports" / "l2_verified_flagship_v1"
OUT_JSON = OUT_DIR / "gold_patch_validation.json"
OUT_CSV = OUT_DIR / "gold_patch_validation.csv"

GOLD_RUN_ROOT = RUN_ROOT / "gold_patch_validation"
GOLD_RESULTS_ASTROPY = GOLD_RUN_ROOT / "results_astropy"
GOLD_RESULTS_REGRESSION = GOLD_RUN_ROOT / "results_regressionfail"
GOLD_PANEL_ASTROPY = GOLD_RUN_ROOT / "panel_gold_astropy.jsonl"
GOLD_PANEL_REGRESSION = GOLD_RUN_ROOT / "panel_gold_regressionfail.jsonl"

SPEC_ASTROPY = REPO_ROOT / "runs" / "released" / "l2_verified_astropy_v1" / "strengthening_spec.json"
SPEC_REGRESSION = RUN_ROOT / "strengthening_spec_regression_fail.json"

EVAL_LADDER_BIN = REPO_ROOT / "target" / "release" / "eval-ladder.exe"
SEED_ASTROPY = "l2-flagship-gold-astropy"
SEED_REGRESSION = "l2-flagship-gold-regressionfail"

NAMESPACE = uuid.UUID("3811dfbf-8c6f-4ad0-b8af-9c83ee2a9ca2")
SUBMITTED_AT = "2025-01-01T00:00:00Z"


@dataclass(frozen=True)
class PanelRow:
    task_id: str
    benchmark_id: str
    candidate_path: Path
    patch_path: Path
    manifest_path: Path
    workspace_path: Path
    family: str


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be a JSON object")
    return data


def _load_verified_cache() -> dict[str, dict[str, Any]]:
    rows: dict[str, dict[str, Any]] = {}
    for raw in VERIFIED_CACHE.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        obj = json.loads(line)
        task_id = obj.get("instance_id")
        if isinstance(task_id, str):
            rows[task_id] = obj
    return rows


def _task_ids_from_flagship_results() -> list[str]:
    summary = _load_json(RESULTS_MERGED)
    seen: set[str] = set()
    task_ids: list[str] = []
    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        task_path = Path(str(entry.get("task_path", "")))
        task_id = task_path.stem
        if not task_id or task_id in seen:
            continue
        seen.add(task_id)
        task_ids.append(task_id)
    task_ids.sort()
    return task_ids


def _ensure_dirs() -> None:
    (GOLD_RUN_ROOT / "candidates" / "gold_patch").mkdir(parents=True, exist_ok=True)
    (GOLD_RUN_ROOT / "patches" / "gold_patch").mkdir(parents=True, exist_ok=True)
    OUT_DIR.mkdir(parents=True, exist_ok=True)


def _clean_previous_results() -> None:
    for p in (GOLD_RESULTS_ASTROPY, GOLD_RESULTS_REGRESSION):
        if p.exists():
            shutil.rmtree(p)


def _candidate_id(task_id: str, family: str, patch_sha: str) -> str:
    return str(uuid.uuid5(NAMESPACE, f"gold_patch|{task_id}|{family}|{patch_sha}"))


def _write_candidate_json(task_id: str, family: str, patch_rel: Path, patch_sha: str, out_path: Path) -> None:
    payload = {
        "schema_version": 1,
        "candidate_id": _candidate_id(task_id, family, patch_sha),
        "benchmark_id": "swe_bench_verified",
        "task_id": task_id,
        "agent_id": "gold_patch",
        "model_id": "dataset_patch",
        "generation_mode": "other",
        "patch_format": "unified_diff",
        "patch_ref": str(patch_rel).replace("\\", "/"),
        "generation_metadata": {
            "tool_configuration": {
                "source": "datasets/cache/verified/swe_bench_verified.jsonl",
                "kind": "dataset_patch",
            },
            "context_mode": "retrieval",
            "repo_reproduction_used": True,
            "random_seed": 0,
            "temperature": 0.0,
        },
        "submitted_at": SUBMITTED_AT,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, sort_keys=True) + "\n", encoding="utf-8")


def _build_panel_rows(task_ids: list[str], cache_rows: dict[str, dict[str, Any]], family: str) -> list[PanelRow]:
    rows: list[PanelRow] = []
    suffix = "__astropy" if family == "astropy" else "__regressionfail"
    for task_id in task_ids:
        if task_id not in cache_rows:
            raise SystemExit(f"missing {task_id} in {VERIFIED_CACHE}")
        manifest = MANIFEST_DIR / f"{task_id}.json"
        if not manifest.is_file():
            raise SystemExit(f"missing manifest for task {task_id}: {manifest}")
        workspace = WORKSPACES_DIR / task_id
        if not workspace.is_dir():
            raise SystemExit(f"missing workspace for task {task_id}: {workspace}")
        patch_rel = Path("patches") / "gold_patch" / f"{task_id}.diff"
        patch_abs = GOLD_RUN_ROOT / patch_rel
        candidate_rel = Path("candidates") / "gold_patch" / f"{task_id}.json"
        candidate_abs = GOLD_RUN_ROOT / candidate_rel
        patch_text = str(cache_rows[task_id].get("patch", ""))
        if not patch_text.strip():
            raise SystemExit(f"empty gold patch for task {task_id}")
        patch_abs.parent.mkdir(parents=True, exist_ok=True)
        patch_abs.write_text(patch_text, encoding="utf-8")
        patch_sha = hashlib.sha256(patch_text.encode("utf-8")).hexdigest()
        _write_candidate_json(task_id, family, patch_rel, patch_sha, candidate_abs)
        rows.append(
            PanelRow(
                task_id=task_id,
                benchmark_id="swe_bench_verified",
                candidate_path=candidate_abs,
                patch_path=patch_abs,
                manifest_path=manifest,
                workspace_path=workspace,
                family=family,
            )
        )
    rows.sort(key=lambda r: r.task_id)
    return rows


def _write_panel_file(panel_path: Path, rows: list[PanelRow]) -> None:
    lines: list[str] = []
    for row in rows:
        suffix = "__astropy" if row.family == "astropy" else "__regressionfail"
        obj = {
            "task": str(row.manifest_path),
            "candidate": str(row.candidate_path),
            "patch": str(row.patch_path),
            "workspace_template": str(row.workspace_path),
            "bundle_name": f"gold_patch__{row.task_id}{suffix}",
            "entry_id": f"gold_patch__{row.task_id}{suffix}",
        }
        lines.append(json.dumps(obj, sort_keys=True))
    panel_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def _run_eval(
    panel: Path,
    spec: Path,
    out_dir: Path,
    seed_tag: str,
    timeout_secs: int,
    short_timeout_secs: int,
    jobs: int,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(EVAL_LADDER_BIN),
        "evaluate",
        "batch",
        "--levels",
        "L0,L1,L2",
        "--input",
        str(panel),
        "--config",
        "configs/evaluator/default.toml",
        "--strengthening-spec",
        str(spec),
        "--strengthening-mode",
        "tests_plus_regression",
        "--out",
        str(out_dir),
        "--timeout-secs",
        str(timeout_secs),
        "--short-timeout-secs",
        str(short_timeout_secs),
        "--adaptive-timeouts",
        "--resume",
        "--jobs",
        str(jobs),
        "--seed-tag",
        seed_tag,
        "--deterministic-clock",
    ]
    subprocess.run(cmd, cwd=REPO_ROOT, check=True)


def _rows_from_summary(summary_path: Path, family: str) -> list[dict[str, Any]]:
    summary = _load_json(summary_path)
    rows: list[dict[str, Any]] = []
    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        levels = entry.get("levels", {})
        if not isinstance(levels, dict):
            levels = {}
        task_path = Path(str(entry.get("task_path", "")))
        bundle_name = str(entry.get("bundle_name", ""))
        bundle_dir = summary_path.parent / bundle_name
        l2 = levels.get("l2", {}) if isinstance(levels.get("l2", {}), dict) else {}
        rows.append(
            {
                "task_id": task_path.stem,
                "validator_family": family,
                "gold_patch_status_L0": str((levels.get("l0", {}) or {}).get("status", "")),
                "gold_patch_status_L1": str((levels.get("l1", {}) or {}).get("status", "")),
                "gold_patch_status_L2": str((levels.get("l2", {}) or {}).get("status", "")),
                "primary_reason": str(l2.get("primary_reason", "")),
                "artifact_bundle": str(bundle_dir.relative_to(REPO_ROOT)).replace("\\", "/"),
            }
        )
    rows.sort(key=lambda r: (r["task_id"], r["validator_family"]))
    return rows


def _write_outputs(rows: list[dict[str, Any]]) -> None:
    OUT_JSON.write_text(json.dumps(rows, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    fieldnames = [
        "task_id",
        "validator_family",
        "gold_patch_status_L0",
        "gold_patch_status_L1",
        "gold_patch_status_L2",
        "primary_reason",
        "artifact_bundle",
    ]
    with OUT_CSV.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--skip-evaluate",
        action="store_true",
        help="Skip running evaluate batch and only export from existing gold results.",
    )
    parser.add_argument(
        "--no-clean",
        action="store_true",
        help="Do not clean prior gold evaluation output directories.",
    )
    parser.add_argument(
        "--timeout-secs",
        type=int,
        default=1800,
        help="Batch hard timeout per entry in seconds.",
    )
    parser.add_argument(
        "--short-timeout-secs",
        type=int,
        default=300,
        help="Short timeout hint in seconds.",
    )
    parser.add_argument(
        "--jobs",
        type=int,
        default=1,
        help="Parallel jobs for evaluate batch.",
    )
    args = parser.parse_args()

    _ensure_dirs()
    if not args.no_clean:
        _clean_previous_results()
    task_ids = _task_ids_from_flagship_results()
    cache_rows = _load_verified_cache()
    rows_astropy = _build_panel_rows(task_ids, cache_rows, "astropy")
    rows_reg = _build_panel_rows(task_ids, cache_rows, "regressionfail")
    _write_panel_file(GOLD_PANEL_ASTROPY, rows_astropy)
    _write_panel_file(GOLD_PANEL_REGRESSION, rows_reg)

    if not args.skip_evaluate:
        _run_eval(
            GOLD_PANEL_ASTROPY,
            SPEC_ASTROPY,
            GOLD_RESULTS_ASTROPY,
            SEED_ASTROPY,
            args.timeout_secs,
            args.short_timeout_secs,
            args.jobs,
        )
        _run_eval(
            GOLD_PANEL_REGRESSION,
            SPEC_REGRESSION,
            GOLD_RESULTS_REGRESSION,
            SEED_REGRESSION,
            args.timeout_secs,
            args.short_timeout_secs,
            args.jobs,
        )

    export_rows = _rows_from_summary(
        GOLD_RESULTS_ASTROPY / "batch_summary.json", "augmented_unit_tests"
    )
    export_rows.extend(
        _rows_from_summary(
            GOLD_RESULTS_REGRESSION / "batch_summary.json", "targeted_regression"
        )
    )
    export_rows.sort(key=lambda r: (r["task_id"], r["validator_family"]))
    _write_outputs(export_rows)
    print(
        json.dumps(
            {
                "task_count": len(task_ids),
                "rows": len(export_rows),
                "csv": str(OUT_CSV),
                "json": str(OUT_JSON),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
