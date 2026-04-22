"""Python compatibility layer for the eval-ladder monorepo.

This package provides thin helpers to bridge Python-native benchmark
tooling. No evaluator core logic lives here; see the Rust workspace at
the repository root for the authoritative implementation.
"""

from __future__ import annotations

from benchmark_compat._version import __version__
from benchmark_compat.canonical import canonical_json, canonical_json_str, sha256_hex
from benchmark_compat.schemas import (
    BenchmarkId,
    BenchmarkTask,
    CandidateResolution,
    EvaluationLevel,
    EvaluationStatus,
    GenerationMetadata,
    Language,
)
from benchmark_compat.swe_bench import (
    SweBenchInstance,
    SweBenchNormalizationError,
    normalize_instance,
    normalize_many,
)
from benchmark_compat.validate import (
    SchemaNotFoundError,
    ValidationError,
    schema_dir,
    validate_benchmark_task,
)

__all__ = [
    "BenchmarkId",
    "BenchmarkTask",
    "CandidateResolution",
    "EvaluationLevel",
    "EvaluationStatus",
    "GenerationMetadata",
    "Language",
    "SchemaNotFoundError",
    "SweBenchInstance",
    "SweBenchNormalizationError",
    "ValidationError",
    "__version__",
    "canonical_json",
    "canonical_json_str",
    "normalize_instance",
    "normalize_many",
    "schema_dir",
    "sha256_hex",
    "validate_benchmark_task",
]
