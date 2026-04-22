"""Schema validation helpers.

Python-emitted artifacts (``BenchmarkTask`` in particular) must satisfy
the same JSON Schema that the Rust CLI ships and uses. This module is
the single surface that validates against the shipped schemas so
failures are caught before we write anything to disk.

We intentionally re-validate pydantic output against the JSON Schema
(rather than trusting pydantic alone): pydantic's constraints and the
JSON Schema's constraints are maintained independently, and cross-
validation is the cheapest way to prove they have not drifted.
"""

from __future__ import annotations

import json
from functools import lru_cache
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError

__all__ = [
    "SchemaNotFoundError",
    "ValidationError",
    "schema_dir",
    "validate_benchmark_task",
]


class SchemaNotFoundError(FileNotFoundError):
    """Raised when the shipped ``schemas/`` directory cannot be located."""


def _walk_up_for_schemas(start: Path) -> Path:
    # Traverse parents until we find a directory whose child contains
    # `benchmark_task.schema.json`. We deliberately do not use
    # `__file__`-relative indexing beyond a couple of levels - the
    # package may be installed from a wheel where `schemas/` sits next
    # to the source tree, or run from the repo root where it sits at
    # `schemas/`.
    here = start.resolve()
    for candidate in (here, *here.parents):
        target = candidate / "schemas" / "benchmark_task.schema.json"
        if target.is_file():
            return candidate / "schemas"
    raise SchemaNotFoundError(
        f"could not find shipped schemas/ starting from {start}"
    )


@lru_cache(maxsize=1)
def schema_dir() -> Path:
    """Locate the repo-root ``schemas/`` directory at import time.

    Cached so the upward traversal runs once per process.
    """

    module_path = Path(__file__).resolve()
    return _walk_up_for_schemas(module_path)


@lru_cache(maxsize=16)
def _validator(schema_name: str) -> Draft202012Validator:
    path = schema_dir() / schema_name
    if not path.is_file():
        raise SchemaNotFoundError(f"schema not shipped: {path}")
    schema = json.loads(path.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


def validate_benchmark_task(payload: dict[str, Any]) -> None:
    """Validate ``payload`` against ``benchmark_task.schema.json``.

    Raises the underlying :class:`jsonschema.ValidationError` on the
    first error encountered so callers can surface the exact JSON
    pointer of the offending field. Does not return anything on
    success; failures must be observable via the raised exception.
    """

    validator = _validator("benchmark_task.schema.json")
    errors = sorted(validator.iter_errors(payload), key=lambda e: e.absolute_path)
    if errors:
        # Re-raise the first error so callers get a canonical Python
        # exception. A compound "all errors" message would hide the
        # fact that the first failure is the blocking one.
        raise errors[0]


