"""Shared pytest fixtures for the Python compat layer."""

from __future__ import annotations

import json
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest

_REPO_ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture(scope="session")
def repo_root() -> Path:
    """Absolute path to the repository root."""

    return _REPO_ROOT


@pytest.fixture(scope="session")
def schemas_dir(repo_root: Path) -> Path:
    """Absolute path to the shipped ``schemas/`` directory."""

    return repo_root / "schemas"


@pytest.fixture()
def swe_bench_instance() -> dict[str, Any]:
    """A synthetic SWE-bench Verified instance record.

    Deliberately synthetic so tests do not require network or a real
    dataset. The shape matches what we observe in real manifests: an
    ``instance_id`` of the form ``<owner>__<name>-<issue>``, a
    ``problem_statement`` multi-line string, and FAIL_TO_PASS/
    PASS_TO_PASS test name lists.
    """

    return {
        "instance_id": "octo-org__widget-7277",
        "repo": "octo-org/widget",
        "base_commit": "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        "problem_statement": (
            "Widget panics on empty input\n\n"
            "When the user supplies an empty string, Widget.render crashes "
            "instead of returning an empty element."
        ),
        "patch": "diff --git a/widget.py b/widget.py\n+pass\n",
        "version": "1.2",
        "FAIL_TO_PASS": ["tests/test_widget.py::test_empty_returns_empty"],
        "PASS_TO_PASS": [
            "tests/test_widget.py::test_basic",
            "tests/test_widget.py::test_nested",
        ],
        "created_at": "2024-07-01T12:34:56Z",
    }


@pytest.fixture()
def swe_bench_manifest(
    tmp_path: Path, swe_bench_instance: dict[str, Any]
) -> Iterator[Path]:
    """A tiny SWE-bench manifest on disk (JSONL)."""

    path = tmp_path / "manifest.jsonl"
    # Emit two records (the second is a renamed clone) so we exercise
    # multi-record behavior.
    other = dict(swe_bench_instance)
    other["instance_id"] = "octo-org__widget-7278"
    other["problem_statement"] = "Second issue\n\ntext."
    lines = [json.dumps(swe_bench_instance), json.dumps(other)]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    yield path
