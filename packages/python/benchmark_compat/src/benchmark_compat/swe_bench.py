"""SWE-bench Verified normalization.

A SWE-bench Verified release manifest is a JSONL file where each line is
a single "instance" describing one GitHub issue plus the curated gold
patch. This module converts those instance records into our
:class:`benchmark_compat.schemas.BenchmarkTask` canonical form so the
Rust evaluator can run over them.

The normalizer is a pure function: same input in, same output out,
bit-for-bit. The only file I/O happens at the CLI boundary
(:mod:`benchmark_compat.cli`).

Mapping:

- ``benchmark_id`` is pinned to ``"swe_bench_verified"``.
- ``task_id`` is the ``instance_id`` verbatim (SWE-bench IDs are
  ``[A-Za-z0-9_\\-]`` by construction and match our TaskId regex).
- ``repo_name`` is the ``repo`` field (``owner/name`` in GitHub form).
- ``issue_id`` is derived from the trailing integer suffix of
  ``instance_id`` (e.g. ``pylint-dev__pylint-7277`` -> ``7277``) so it
  round-trips back to the GitHub issue number. SWE-bench records
  occasionally include an ``issue_numbers`` list; when present, the
  first element wins.
- ``issue_title`` is the first non-empty line of ``problem_statement``.
- ``issue_text`` is the full ``problem_statement``.
- ``base_commit`` passes through.
- ``gold_patch_ref`` is the content-addressed SHA-256 of the gold
  ``patch`` bytes, emitted as ``"sha256:<hex>"``. When the record has
  no gold patch we emit ``None``.
- ``environment_ref`` is the deterministic SWE-bench evaluation image
  name: ``"swebench/sweb.eval.x86_64.<instance_id>:latest"``.
- ``official_test_entrypoint`` is ``pytest -q <fail_to_pass...>
  <pass_to_pass...>`` with test names sorted lexicographically for
  deterministic byte output.
- ``language`` is pinned to ``"python"``.
- ``labels`` always contain ``"benchmark:swe_bench_verified"`` plus
  optional ``"version:<version>"`` when the record has a version field.
- ``created_at`` passes through (ISO-8601).
- ``source_url`` is a deterministic issue URL derived from
  ``repo_name`` and ``issue_id``.

The normalizer refuses to make best-effort guesses: any missing or
malformed field raises :class:`SweBenchNormalizationError`.
"""

from __future__ import annotations

import re
from collections.abc import Iterable, Sequence
from datetime import datetime, timezone
from typing import Any

from pydantic import BaseModel, ConfigDict, Field, ValidationError

from benchmark_compat.canonical import sha256_hex
from benchmark_compat.schemas import BenchmarkTask

__all__ = [
    "SweBenchInstance",
    "SweBenchNormalizationError",
    "normalize_instance",
    "normalize_many",
]


_TASK_ID_SUFFIX_RE = re.compile(r"-(\d+)$")


class SweBenchNormalizationError(ValueError):
    """Raised when a SWE-bench record cannot be normalized.

    Carries the offending ``instance_id`` (or a best-effort descriptor
    when the id itself is missing) so batch callers can log stable
    diagnostics.
    """

    def __init__(self, instance_id: str, reason: str) -> None:
        self.instance_id = instance_id
        self.reason = reason
        super().__init__(f"SWE-bench instance {instance_id!r}: {reason}")


class SweBenchInstance(BaseModel):
    """One record in a SWE-bench Verified manifest.

    Only the fields we actually consume are typed; ``extra = "ignore"``
    lets unknown fields flow through (SWE-bench occasionally adds new
    metadata fields in minor releases, and failing on unknown fields
    would gate new manifests behind a normalizer release).
    """

    model_config = ConfigDict(extra="ignore", populate_by_name=True)

    instance_id: str
    repo: str
    base_commit: str
    problem_statement: str
    created_at: str
    patch: str | None = None
    version: str | None = None
    fail_to_pass: list[str] = Field(default_factory=list, alias="FAIL_TO_PASS")
    pass_to_pass: list[str] = Field(default_factory=list, alias="PASS_TO_PASS")
    issue_numbers: list[int] = Field(default_factory=list)


def _derive_issue_id(inst: SweBenchInstance) -> str:
    if inst.issue_numbers:
        return str(inst.issue_numbers[0])
    match = _TASK_ID_SUFFIX_RE.search(inst.instance_id)
    if match is None:
        raise SweBenchNormalizationError(
            inst.instance_id,
            "instance_id has no trailing '-<issue_number>' suffix "
            "and no issue_numbers field was provided",
        )
    return match.group(1)


def _derive_issue_title(problem_statement: str, instance_id: str) -> str:
    for line in problem_statement.splitlines():
        stripped = line.strip()
        if stripped:
            return stripped
    raise SweBenchNormalizationError(
        instance_id, "problem_statement is empty; cannot derive issue_title"
    )


def _coerce_tests(tests: Iterable[str]) -> list[str]:
    # Deterministic order for byte-stable `official_test_entrypoint`.
    out = sorted({t.strip() for t in tests if t and t.strip()})
    return out


def _build_entrypoint(inst: SweBenchInstance) -> str:
    targets = _coerce_tests(list(inst.fail_to_pass) + list(inst.pass_to_pass))
    if not targets:
        # Fall back to a bare invocation. The downstream evaluator can
        # still refuse to run it; we just preserve byte-stable output.
        return "pytest -q"
    return "pytest -q " + " ".join(targets)


def _parse_created_at(raw: str, instance_id: str) -> datetime:
    try:
        # SWE-bench records emit `2020-01-01T00:00:00Z` or
        # `+00:00`-suffixed ISO-8601.
        if raw.endswith("Z"):
            return datetime.fromisoformat(raw[:-1]).replace(tzinfo=timezone.utc)
        dt = datetime.fromisoformat(raw)
        if dt.tzinfo is None:
            return dt.replace(tzinfo=timezone.utc)
        return dt
    except ValueError as e:
        raise SweBenchNormalizationError(
            instance_id, f"created_at not ISO-8601 parseable: {raw!r} ({e})"
        ) from e


def _source_url(repo_name: str, issue_id: str) -> str:
    # SWE-bench repo identifiers are always `owner/name`. We accept
    # anything with one slash and refuse otherwise, preserving the
    # deterministic URL shape.
    if repo_name.count("/") != 1:
        raise SweBenchNormalizationError(
            repo_name, f"repo_name must be 'owner/name'; got {repo_name!r}"
        )
    return f"https://github.com/{repo_name}/issues/{issue_id}"


def normalize_instance(record: dict[str, Any] | SweBenchInstance) -> BenchmarkTask:
    """Normalize one SWE-bench Verified instance.

    Accepts either a raw ``dict`` (the shape of a single manifest line)
    or a pre-validated :class:`SweBenchInstance`. Raises
    :class:`SweBenchNormalizationError` on any missing or malformed
    field.
    """

    if isinstance(record, SweBenchInstance):
        inst = record
    else:
        try:
            inst = SweBenchInstance.model_validate(record)
        except ValidationError as e:
            iid = str(record.get("instance_id", "<missing instance_id>"))
            raise SweBenchNormalizationError(iid, f"schema validation failed: {e}") from e

    issue_id = _derive_issue_id(inst)
    issue_title = _derive_issue_title(inst.problem_statement, inst.instance_id)
    created_at = _parse_created_at(inst.created_at, inst.instance_id)
    source_url = _source_url(inst.repo, issue_id)

    gold_patch_ref: str | None
    if inst.patch is None or inst.patch == "":
        gold_patch_ref = None
    else:
        gold_patch_ref = f"sha256:{sha256_hex(inst.patch.encode('utf-8'))}"

    labels = ["benchmark:swe_bench_verified"]
    if inst.version is not None and inst.version.strip():
        labels.append(f"version:{inst.version.strip()}")

    task = BenchmarkTask(
        benchmark_id="swe_bench_verified",
        task_id=inst.instance_id,
        repo_name=inst.repo,
        issue_id=issue_id,
        issue_title=issue_title,
        issue_text=inst.problem_statement,
        base_commit=inst.base_commit,
        gold_patch_ref=gold_patch_ref,
        environment_ref=f"swebench/sweb.eval.x86_64.{inst.instance_id}:latest",
        official_test_entrypoint=_build_entrypoint(inst),
        language="python",
        labels=labels,
        created_at=created_at,
        source_url=source_url,
    )
    return task


def normalize_many(
    records: Sequence[dict[str, Any] | SweBenchInstance],
) -> list[BenchmarkTask]:
    """Normalize a sequence of SWE-bench instances.

    Stops on the first record that fails normalization and surfaces the
    error. Callers that need per-record resilience should iterate
    :func:`normalize_instance` directly and handle
    :class:`SweBenchNormalizationError` themselves; this function is a
    pure convenience for the common "normalize a whole manifest"
    pathway.
    """

    return [normalize_instance(r) for r in records]
