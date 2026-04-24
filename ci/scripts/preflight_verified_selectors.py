#!/usr/bin/env python3
"""Preflight: official pytest selectors vs materialized panel workspaces.

For each unique Verified task referenced by a batch ``panel.jsonl``, parse
``official_test_entrypoint`` from the task manifest and verify that every
pytest file path exists under the panel's ``workspace_template`` directory
when that directory is present.

This catches a dominant cause of ``L1_HARNESS_ERROR`` (pytest ``not found`` /
``file not found``) *before* launching Docker batch runs.

Exit code: 0 unless ``--min-tasks`` is set and the panel yields too few Verified
tasks (then 3), or ``--strict`` is set and at least one **explicit** pytest path
(contains ``/`` or ends with ``.py``) is missing under a present workspace
(then 2). Bare legacy arguments such as ``test_issue_11617`` are reported as
warnings when they cannot be resolved to a file.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]


def _is_explicit_pytest_path(rel: str) -> bool:
    return "/" in rel or rel.endswith(".py")


def _resolve_pytest_file(workspace_dir: Path, rel: str) -> tuple[bool, Path]:
    """Return (found, resolved_path) where *resolved_path* is the first match."""
    rel = rel.replace("\\", "/").strip()
    ws = workspace_dir.resolve()
    candidates = [ws / rel, ws / f"{rel}.py"]
    if not _is_explicit_pytest_path(rel):
        candidates.extend([ws / "sympy" / f"{rel}.py", ws / "sympy" / rel])
    for p in candidates:
        try:
            rp = p.resolve()
            rp.relative_to(ws)
        except ValueError:
            continue
        if rp.is_file():
            return True, rp
    return False, (ws / rel).resolve()


def _pytest_file_paths(entrypoint: str) -> list[str]:
    """Return repo-relative pytest file paths (no ``::`` node suffix)."""
    ep = entrypoint.strip()
    lowered = ep.lower()
    if "pytest" not in lowered:
        return []
    for prefix in ("python -m pytest ", "python3 -m pytest ", "pytest "):
        if lowered.startswith(prefix):
            ep = ep[len(prefix) :].lstrip()
            break
    else:
        return []

    paths: list[str] = []
    for tok in ep.split():
        if tok.startswith("-"):
            continue
        if "=" in tok and not tok.endswith(".py"):
            # e.g. -k foo=bar — skip
            continue
        if "::" in tok or tok.endswith(".py") or "/" in tok:
            file_part = tok.split("::", 1)[0].strip()
            if file_part and not file_part.startswith("-"):
                paths.append(file_part.replace("\\", "/"))
        elif re.match(r"^test_[A-Za-z0-9_]+$", tok):
            # Legacy SWE entrypoints: ``python -m pytest test_issue_11617``
            paths.append(tok)
    seen: set[str] = set()
    out: list[str] = []
    for p in paths:
        if p not in seen:
            seen.add(p)
            out.append(p)
    return out


def _load_panel_entries(panel_path: Path) -> list[dict[str, Any]]:
    lines = panel_path.read_text(encoding="utf-8").splitlines()
    entries: list[dict[str, Any]] = []
    for i, raw in enumerate(lines, start=1):
        stripped = raw.strip()
        if not stripped:
            continue
        try:
            entries.append(json.loads(stripped))
        except json.JSONDecodeError as e:
            raise SystemExit(f"{panel_path}: line {i}: invalid json: {e}") from e
    return entries


def _task_rows(panel_path: Path) -> list[tuple[str, Path, Path]]:
    """(task_id, manifest_path, workspace_dir) deduped by (task_id, workspace)."""
    panel_dir = panel_path.parent.resolve()
    seen: set[tuple[str, str]] = set()
    rows: list[tuple[str, Path, Path]] = []
    for row in _load_panel_entries(panel_path):
        task_rel = str(row.get("task", "")).replace("\\", "/")
        ws_rel = str(row.get("workspace_template", "")).replace("\\", "/").rstrip("/")
        if not task_rel or not ws_rel:
            continue
        manifest_path = (panel_dir / task_rel).resolve()
        workspace_dir = (panel_dir / ws_rel).resolve()
        if not manifest_path.is_file():
            raise SystemExit(f"missing manifest: {manifest_path}")
        data = json.loads(manifest_path.read_text(encoding="utf-8"))
        task_id = str(data.get("task_id", manifest_path.stem))
        bench = str(data.get("benchmark_id", ""))
        if bench != "swe_bench_verified":
            continue
        key = (task_id, str(workspace_dir.resolve()))
        if key in seen:
            continue
        seen.add(key)
        rows.append((task_id, manifest_path, workspace_dir))
    rows.sort(key=lambda t: t[0])
    return rows


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--panel",
        type=Path,
        required=True,
        help="path to panel.jsonl (Verified batch input)",
    )
    ap.add_argument(
        "--strict",
        action="store_true",
        help="exit with code 2 if any selector file is missing under a present workspace",
    )
    ap.add_argument(
        "--min-tasks",
        type=int,
        default=0,
        help="exit with code 3 if fewer Verified tasks were found (after panel parse)",
    )
    args = ap.parse_args()
    panel_path = args.panel.resolve()
    if not panel_path.is_file():
        raise SystemExit(f"panel not found: {panel_path}")

    if args.min_tasks < 0:
        raise SystemExit("--min-tasks must be >= 0")

    tasks_out: list[dict[str, Any]] = []
    errors = 0
    warnings = 0
    ok = 0

    for task_id, manifest_path, workspace_dir in _task_rows(panel_path):
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        entrypoint = str(manifest.get("official_test_entrypoint", ""))
        paths = _pytest_file_paths(entrypoint)
        ws_present = workspace_dir.is_dir()
        checks: list[dict[str, Any]] = []
        task_errors = 0
        if not paths:
            checks.append(
                {
                    "relative": None,
                    "resolved": None,
                    "file_exists": None,
                    "note": "no_pytest_file_paths_parsed_from_entrypoint",
                }
            )
            warnings += 1
        elif not ws_present:
            for rel in paths:
                checks.append(
                    {
                        "relative": rel,
                        "resolved": None,
                        "file_exists": None,
                        "note": "workspace_dir_absent_skip_file_check",
                    }
                )
            warnings += 1
        else:
            for rel in paths:
                exists, resolved = _resolve_pytest_file(workspace_dir, rel)
                explicit = _is_explicit_pytest_path(rel)
                if not exists:
                    note = (
                        "missing_explicit_pytest_path"
                        if explicit
                        else "bare_pytest_arg_unresolved_in_workspace"
                    )
                    checks.append(
                        {
                            "relative": rel,
                            "resolved": str(resolved),
                            "file_exists": False,
                            "note": note,
                        }
                    )
                    if explicit:
                        task_errors += 1
                        errors += 1
                    else:
                        warnings += 1
                    continue
                checks.append(
                    {
                        "relative": rel,
                        "resolved": str(resolved),
                        "file_exists": True,
                        "note": None,
                    }
                )
            if task_errors == 0:
                ok += 1

        tasks_out.append(
            {
                "task_id": task_id,
                "manifest": str(manifest_path.relative_to(REPO_ROOT)).replace(
                    "\\", "/"
                ),
                "workspace_dir": str(workspace_dir.relative_to(REPO_ROOT)).replace(
                    "\\", "/"
                ),
                "workspace_present": ws_present,
                "official_test_entrypoint": entrypoint,
                "pytest_file_paths": paths,
                "checks": checks,
                "task_errors": task_errors,
            }
        )

    report = {
        "panel": str(panel_path.relative_to(REPO_ROOT)).replace("\\", "/"),
        "summary": {
            "tasks": len(tasks_out),
            "tasks_all_selector_files_ok": ok,
            "selector_file_errors": errors,
            "warnings": warnings,
            "min_tasks_required": args.min_tasks,
        },
        "tasks": tasks_out,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    if args.min_tasks > 0 and len(tasks_out) < args.min_tasks:
        return 3
    if args.strict and errors > 0:
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
