"""Unit tests for the SWE-bench Verified normalizer."""

from __future__ import annotations

from datetime import timezone
from typing import Any

import pytest
from benchmark_compat.canonical import canonical_json
from benchmark_compat.swe_bench import (
    SweBenchInstance,
    SweBenchNormalizationError,
    normalize_instance,
    normalize_many,
)


def test_normalize_instance_maps_all_required_fields(
    swe_bench_instance: dict[str, Any],
) -> None:
    task = normalize_instance(swe_bench_instance)

    assert task.schema_version == 1
    assert task.benchmark_id == "swe_bench_verified"
    assert task.task_id == "octo-org__widget-7277"
    assert task.repo_name == "octo-org/widget"
    assert task.issue_id == "7277"
    assert task.issue_title == "Widget panics on empty input"
    assert task.issue_text.startswith("Widget panics on empty input\n\n")
    assert task.base_commit == "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
    assert task.gold_patch_ref is not None
    assert task.gold_patch_ref.startswith("sha256:")
    assert len(task.gold_patch_ref) == len("sha256:") + 64
    assert task.environment_ref == (
        "swebench/sweb.eval.x86_64.octo-org__widget-7277:latest"
    )
    # test names sorted lexicographically; pytest -q prefix is canonical.
    assert task.official_test_entrypoint == (
        "pytest -q "
        "tests/test_widget.py::test_basic "
        "tests/test_widget.py::test_empty_returns_empty "
        "tests/test_widget.py::test_nested"
    )
    assert task.language == "python"
    assert task.labels == ["benchmark:swe_bench_verified", "version:1.2"]
    assert task.created_at.tzinfo == timezone.utc
    assert task.source_url == "https://github.com/octo-org/widget/issues/7277"


def test_normalize_instance_is_byte_deterministic(
    swe_bench_instance: dict[str, Any],
) -> None:
    a = canonical_json(normalize_instance(swe_bench_instance).model_dump(mode="json"))
    b = canonical_json(normalize_instance(swe_bench_instance).model_dump(mode="json"))
    assert a == b


def test_normalize_instance_rejects_missing_issue_suffix() -> None:
    bad = {
        "instance_id": "broken-no-suffix",
        "repo": "octo/widget",
        "base_commit": "a1b2c3d4",
        "problem_statement": "hello",
        "created_at": "2024-07-01T12:34:56Z",
    }
    with pytest.raises(SweBenchNormalizationError) as ei:
        normalize_instance(bad)
    assert "instance_id has no trailing" in str(ei.value)


def test_normalize_instance_prefers_issue_numbers_over_suffix() -> None:
    record = {
        "instance_id": "octo__widget-1",
        "repo": "octo/widget",
        "base_commit": "a1b2c3d4",
        "problem_statement": "title\nbody",
        "created_at": "2024-07-01T12:34:56Z",
        "issue_numbers": [999],
    }
    task = normalize_instance(record)
    assert task.issue_id == "999"
    assert task.source_url.endswith("/issues/999")


def test_normalize_instance_rejects_empty_problem_statement() -> None:
    bad = {
        "instance_id": "octo__widget-1",
        "repo": "octo/widget",
        "base_commit": "a1b2c3d4",
        "problem_statement": "   \n\n",
        "created_at": "2024-07-01T12:34:56Z",
    }
    with pytest.raises(SweBenchNormalizationError) as ei:
        normalize_instance(bad)
    assert "problem_statement is empty" in str(ei.value)


def test_normalize_instance_rejects_malformed_repo() -> None:
    bad = {
        "instance_id": "octo__widget-1",
        "repo": "just-one-segment",
        "base_commit": "a1b2c3d4",
        "problem_statement": "hi",
        "created_at": "2024-07-01T12:34:56Z",
    }
    with pytest.raises(SweBenchNormalizationError) as ei:
        normalize_instance(bad)
    assert "repo_name must be 'owner/name'" in str(ei.value)


def test_normalize_instance_handles_missing_gold_patch(
    swe_bench_instance: dict[str, Any],
) -> None:
    swe_bench_instance = dict(swe_bench_instance)
    swe_bench_instance.pop("patch")
    task = normalize_instance(swe_bench_instance)
    assert task.gold_patch_ref is None


def test_normalize_instance_handles_empty_test_lists() -> None:
    record = {
        "instance_id": "octo__widget-1",
        "repo": "octo/widget",
        "base_commit": "a1b2c3d4",
        "problem_statement": "hi",
        "created_at": "2024-07-01T12:34:56Z",
    }
    task = normalize_instance(record)
    assert task.official_test_entrypoint == "pytest -q"


def test_normalize_instance_accepts_preparsed_instance(
    swe_bench_instance: dict[str, Any],
) -> None:
    inst = SweBenchInstance.model_validate(swe_bench_instance)
    task = normalize_instance(inst)
    assert task.task_id == inst.instance_id


def test_normalize_instance_unknown_fields_are_ignored(
    swe_bench_instance: dict[str, Any],
) -> None:
    swe_bench_instance = dict(swe_bench_instance)
    swe_bench_instance["future_metadata_field"] = {"foo": "bar"}
    task = normalize_instance(swe_bench_instance)
    assert task.task_id == "octo-org__widget-7277"


def test_normalize_many_preserves_order(
    swe_bench_instance: dict[str, Any],
) -> None:
    second = dict(swe_bench_instance)
    second["instance_id"] = "octo-org__widget-7278"
    second["problem_statement"] = "another title\nbody"
    tasks = normalize_many([swe_bench_instance, second])
    assert [t.task_id for t in tasks] == [
        "octo-org__widget-7277",
        "octo-org__widget-7278",
    ]
