#!/usr/bin/env python3
"""Structural audit of every ``benchmarks/verified/manifests/*.json`` task.

Classifies ``official_test_entrypoint`` shapes (pytest, Django runtests, other),
counts legacy bare-pytest targets, and validates non-negotiable fields
(``benchmark_id``, ``environment_ref`` pattern, absence of ``..`` tokens).

Use ``--strict`` in CI to fail on empty entrypoints, traversal-like ``..`` in
the entrypoint string, malformed ``environment_ref``, or unknown benchmark id.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
from collections import Counter
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]


def _load_verified_pytest_targets() -> Any:
    path = Path(__file__).resolve().parent / "verified_pytest_targets.py"
    spec = importlib.util.spec_from_file_location("verified_pytest_targets", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load verified_pytest_targets")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_vpt = _load_verified_pytest_targets()
pytest_file_paths = _vpt.pytest_file_paths
is_explicit_pytest_path = _vpt.is_explicit_pytest_path
DEFAULT_MANIFEST_DIR = REPO_ROOT / "benchmarks" / "verified" / "manifests"
_ENV_REF_RE = re.compile(
    r"^swebench/sweb\.eval\.x86_64\.[a-z0-9_.-]+:latest$",
    re.IGNORECASE,
)


def _classify_entrypoint_style(entrypoint: str) -> str:
    s = entrypoint.strip()
    if not s:
        return "empty"
    if ".." in s:
        return "path_traversal_token"
    low = s.lower()
    rules: list[tuple[bool, str]] = [
        (
            low.startswith("python -m pytest")
            or low.startswith("python3 -m pytest"),
            "python_m_pytest",
        ),
        (low.startswith("pytest "), "pytest_direct"),
        (
            low.startswith("python ") and "runtests.py" in low,
            "django_runtests",
        ),
        (low.startswith("python "), "python_other"),
    ]
    for cond, label in rules:
        if cond:
            return label
    return "other_shell"


def _pytest_target_shape(entrypoint: str) -> str:
    paths = pytest_file_paths(entrypoint)
    if not paths:
        return "pytest_no_paths_parsed"
    if all(is_explicit_pytest_path(p) for p in paths):
        return "pytest_explicit_only"
    if any(is_explicit_pytest_path(p) for p in paths):
        return "pytest_mixed_explicit_and_legacy"
    return "pytest_legacy_bare_only"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--manifest-dir",
        type=Path,
        default=DEFAULT_MANIFEST_DIR,
        help="directory containing Verified task JSON manifests",
    )
    ap.add_argument(
        "--strict",
        action="store_true",
        help="non-zero exit on structural violations",
    )
    args = ap.parse_args()
    manifest_dir = args.manifest_dir.resolve()
    if not manifest_dir.is_dir():
        raise SystemExit(f"manifest dir not found: {manifest_dir}")

    style_counts: Counter[str] = Counter()
    pytest_shape_counts: Counter[str] = Counter()
    failures: list[str] = []
    n_manifests = 0
    legacy_bare_tasks: list[str] = []

    for path in sorted(manifest_dir.glob("*.json")):
        n_manifests += 1
        data: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
        task_id = str(data.get("task_id", path.stem))
        bench = str(data.get("benchmark_id", ""))
        env_ref = str(data.get("environment_ref", ""))
        entrypoint = str(data.get("official_test_entrypoint", ""))

        if bench != "swe_bench_verified":
            failures.append(f"{task_id}: benchmark_id {bench!r} != swe_bench_verified")
        if not _ENV_REF_RE.match(env_ref):
            failures.append(
                f"{task_id}: environment_ref {env_ref!r} "
                "does not match harness pattern"
            )

        style = _classify_entrypoint_style(entrypoint)
        style_counts[style] += 1
        if style in ("empty", "path_traversal_token"):
            failures.append(f"{task_id}: entrypoint style {style}")

        if style in ("python_m_pytest", "pytest_direct"):
            shape = _pytest_target_shape(entrypoint)
            pytest_shape_counts[shape] += 1
            if shape == "pytest_legacy_bare_only":
                legacy_bare_tasks.append(task_id)

    report: dict[str, Any] = {
        "manifest_dir": str(manifest_dir.relative_to(REPO_ROOT)).replace("\\", "/"),
        "total_manifests": n_manifests,
        "entrypoint_style_counts": dict(sorted(style_counts.items())),
        "pytest_target_shape_counts": dict(sorted(pytest_shape_counts.items())),
        "legacy_bare_pytest_only_count": len(legacy_bare_tasks),
        "legacy_bare_pytest_sample": sorted(legacy_bare_tasks)[:25],
        "failures": failures,
        "ok": not failures,
    }
    print(json.dumps(report, indent=2, sort_keys=True))

    if args.strict and failures:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
