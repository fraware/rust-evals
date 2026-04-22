"""Tests for the JSON-schema validation helpers."""

from __future__ import annotations

from typing import Any

import pytest
from benchmark_compat.swe_bench import normalize_instance
from benchmark_compat.validate import ValidationError, schema_dir, validate_benchmark_task


def test_schema_dir_points_at_shipped_schemas() -> None:
    d = schema_dir()
    assert d.is_dir()
    assert (d / "benchmark_task.schema.json").is_file()


def test_validate_benchmark_task_accepts_normalized_output(
    swe_bench_instance: dict[str, Any],
) -> None:
    task = normalize_instance(swe_bench_instance)
    payload = task.model_dump(mode="json")
    validate_benchmark_task(payload)


def test_validate_benchmark_task_rejects_missing_required_field(
    swe_bench_instance: dict[str, Any],
) -> None:
    task = normalize_instance(swe_bench_instance)
    payload = task.model_dump(mode="json")
    del payload["task_id"]
    with pytest.raises(ValidationError):
        validate_benchmark_task(payload)


def test_validate_benchmark_task_rejects_unknown_field(
    swe_bench_instance: dict[str, Any],
) -> None:
    task = normalize_instance(swe_bench_instance)
    payload = task.model_dump(mode="json")
    payload["not_in_schema"] = "nope"
    with pytest.raises(ValidationError):
        validate_benchmark_task(payload)


def test_validate_benchmark_task_rejects_bad_base_commit(
    swe_bench_instance: dict[str, Any],
) -> None:
    task = normalize_instance(swe_bench_instance)
    payload = task.model_dump(mode="json")
    payload["base_commit"] = "xyz"
    with pytest.raises(ValidationError):
        validate_benchmark_task(payload)
