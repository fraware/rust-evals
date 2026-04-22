"""Pydantic models that mirror the JSON schemas shipped at the repo root.

These models exist so Python-side importers can validate and produce
well-formed artifacts without round-tripping through the Rust binary. They
MUST stay in lockstep with `schemas/` and with `packages/rust/core`.

If you change a schema, update all three: `schemas/<name>.schema.json`, the
Rust type, and the pydantic model here.
"""

from __future__ import annotations

from datetime import datetime
from typing import Any, Literal

from pydantic import BaseModel, ConfigDict, Field

BenchmarkId = Literal["swe_bench_verified", "swe_bench_live", "rust_swe_bench"]
Language = Literal["python", "rust", "mixed"]
EvaluationLevel = Literal["L0", "L1", "L2", "L3", "L4"]
EvaluationStatus = Literal["pass", "fail", "invalid", "not_applicable"]


class _Strict(BaseModel):
    """Base model forbidding unknown fields, matching `#[serde(deny_unknown_fields)]`."""

    model_config = ConfigDict(extra="forbid")


class BenchmarkTask(_Strict):
    """Normalized benchmark task."""

    schema_version: int = Field(default=1)
    benchmark_id: BenchmarkId
    task_id: str
    repo_name: str
    issue_id: str
    issue_title: str
    issue_text: str
    base_commit: str
    gold_patch_ref: str | None = None
    environment_ref: str
    official_test_entrypoint: str
    language: Language
    labels: list[str] = Field(default_factory=list)
    created_at: datetime
    source_url: str


class GenerationMetadata(_Strict):
    """Metadata the agent must declare about how the candidate was generated."""

    temperature: float | None = None
    tool_configuration: Any
    context_mode: Literal["full_repo", "retrieval", "file_level", "window", "other"]
    repo_reproduction_used: bool
    random_seed: int | None = None


class CandidateResolution(_Strict):
    """The canonical evaluation unit."""

    schema_version: int = Field(default=1)
    candidate_id: str
    benchmark_id: BenchmarkId
    task_id: str
    agent_id: str
    model_id: str
    generation_mode: Literal["agent_loop", "single_shot", "rerank", "human_assisted", "other"]
    patch_format: Literal["unified_diff", "git_patch", "json_edits"]
    patch_ref: str
    trajectory_ref: str | None = None
    generation_metadata: GenerationMetadata
    submitted_at: datetime


__all__ = [
    "BenchmarkId",
    "BenchmarkTask",
    "CandidateResolution",
    "EvaluationLevel",
    "EvaluationStatus",
    "GenerationMetadata",
    "Language",
]
