"""Contract tests for ``ci/scripts/verified_pytest_targets.py``."""

from __future__ import annotations

import importlib.util
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[2]
_SPEC = importlib.util.spec_from_file_location(
    "verified_pytest_targets_under_test",
    _ROOT / "ci/scripts/verified_pytest_targets.py",
)
assert _SPEC is not None and _SPEC.loader is not None
_MOD = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(_MOD)

pytest_file_paths = _MOD.pytest_file_paths
is_explicit_pytest_path = _MOD.is_explicit_pytest_path


def test_is_explicit_pytest_path() -> None:
    assert is_explicit_pytest_path("astropy/foo.py") is True
    assert is_explicit_pytest_path("tests/test_x.py") is True
    assert is_explicit_pytest_path("test_issue_11617") is False


def test_pytest_file_paths_explicit_with_node_id() -> None:
    ep = (
        "python -m pytest astropy/units/tests/test_quantity_annotations.py"
        "::test_return_annotation_none"
    )
    assert pytest_file_paths(ep) == ["astropy/units/tests/test_quantity_annotations.py"]


def test_pytest_file_paths_multiple_targets() -> None:
    ep = (
        "python -m pytest tests/a.py::t1 tests/b.py::t2 -q"
    )
    assert pytest_file_paths(ep) == ["tests/a.py", "tests/b.py"]


def test_pytest_file_paths_python3_prefix() -> None:
    ep = "python3 -m pytest pkg/mod_tests.py::Foo::bar"
    assert pytest_file_paths(ep) == ["pkg/mod_tests.py"]


def test_pytest_file_paths_direct_pytest_invocation() -> None:
    ep = "pytest testing/test_foo.py -k slow"
    assert pytest_file_paths(ep) == ["testing/test_foo.py"]


def test_pytest_file_paths_legacy_bare_test_token() -> None:
    ep = "python -m pytest test_issue_11617"
    assert pytest_file_paths(ep) == ["test_issue_11617"]


def test_pytest_file_paths_dedupes() -> None:
    ep = "python -m pytest foo.py::a foo.py::b"
    assert pytest_file_paths(ep) == ["foo.py"]


def test_pytest_file_paths_skips_dash_k_value_with_equals() -> None:
    ep = "python -m pytest -k foo=bar tests/x.py"
    assert pytest_file_paths(ep) == ["tests/x.py"]


def test_non_pytest_entrypoint_returns_empty() -> None:
    ep = "python tests/runtests.py pagination.tests.PaginationTests.t_iter"
    assert pytest_file_paths(ep) == []


def test_whitespace_only_inside_pytest_prefix() -> None:
    ep = "python -m pytest   tests/t.py::x"
    assert pytest_file_paths(ep) == ["tests/t.py"]
