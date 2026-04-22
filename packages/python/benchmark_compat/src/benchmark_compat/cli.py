"""CLI entrypoint for the Python compatibility layer.

This CLI is deliberately minimal. It exposes import / export helpers for
Python-native benchmark tooling and defers every evaluator decision to
the Rust ``eval-ladder`` binary. Each command is a thin wrapper around a
pure function in a sibling module so the heavy lifting can be unit
tested without the CLI layer.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Annotated, Any

import typer

from benchmark_compat._version import __version__
from benchmark_compat.canonical import canonical_json
from benchmark_compat.swe_bench import (
    SweBenchNormalizationError,
    normalize_instance,
)
from benchmark_compat.validate import ValidationError, validate_benchmark_task

app = typer.Typer(
    name="eval-ladder-py",
    help=(
        "Python compatibility layer for eval-ladder. Bridges SWE-bench Python "
        "tooling into the Rust evaluator. No evaluator core logic lives here."
    ),
    add_completion=False,
)


@app.command()
def version() -> None:
    """Print the package version."""

    typer.echo(f"benchmark_compat {__version__}")


def _iter_manifest(source: Path) -> list[dict[str, Any]]:
    """Read a SWE-bench manifest.

    Accepts two shapes:

    - JSONL (``*.jsonl``): one instance per non-blank line; lines
      starting with ``#`` are treated as comments.
    - JSON array (``*.json``): a top-level list of instance objects.

    Any other shape raises :class:`typer.BadParameter`.
    """

    if not source.exists():
        raise typer.BadParameter(f"source does not exist: {source}")
    if not source.is_file():
        raise typer.BadParameter(f"source must be a file: {source}")

    data = source.read_text(encoding="utf-8")

    if source.suffix.lower() == ".jsonl":
        records: list[dict[str, Any]] = []
        for i, raw in enumerate(data.splitlines(), start=1):
            stripped = raw.strip()
            if not stripped or stripped.startswith("#"):
                continue
            try:
                obj = json.loads(stripped)
            except json.JSONDecodeError as e:
                raise typer.BadParameter(
                    f"{source}: JSONL parse error on line {i}: {e}"
                ) from e
            if not isinstance(obj, dict):
                raise typer.BadParameter(
                    f"{source}: line {i} is not a JSON object"
                )
            records.append(obj)
        return records

    try:
        parsed = json.loads(data)
    except json.JSONDecodeError as e:
        raise typer.BadParameter(f"{source}: JSON parse error: {e}") from e
    if not isinstance(parsed, list):
        raise typer.BadParameter(
            f"{source}: top-level JSON must be an array of instance objects"
        )
    for i, obj in enumerate(parsed):
        if not isinstance(obj, dict):
            raise typer.BadParameter(
                f"{source}: element {i} is not a JSON object"
            )
    return parsed


@app.command("normalize-swe-bench")
def normalize_swe_bench(
    source: Annotated[
        Path,
        typer.Option(
            "--source",
            help="Path to a SWE-bench Verified manifest (.jsonl or .json).",
        ),
    ],
    out_dir: Annotated[
        Path,
        typer.Option(
            "--out-dir",
            help="Directory to write normalized BenchmarkTask files into.",
        ),
    ],
    strict: Annotated[
        bool,
        typer.Option(
            "--strict",
            help=(
                "Abort on the first record that fails normalization. "
                "Default is to log errors and continue."
            ),
        ),
    ] = False,
) -> None:
    """Normalize a SWE-bench Verified manifest into ``BenchmarkTask`` files.

    Each record emits ``<out_dir>/<task_id>.json`` in canonical JSON
    form (sorted keys, UTF-8, ``\\n`` line endings). The output is
    deterministic: the same manifest produces byte-identical files on
    every run.

    Exit codes:

    - ``0`` - at least one record normalized successfully.
    - ``2`` - IO / manifest-parse failure (before any record was
      considered).
    - ``3`` - every record failed normalization.
    """

    records = _iter_manifest(source)
    if not records:
        typer.secho(f"{source}: no records found", fg=typer.colors.RED, err=True)
        raise typer.Exit(code=2)

    out_dir.mkdir(parents=True, exist_ok=True)

    ok = 0
    errs: list[SweBenchNormalizationError] = []
    for record in records:
        try:
            task = normalize_instance(record)
        except SweBenchNormalizationError as e:
            errs.append(e)
            if strict:
                typer.secho(str(e), fg=typer.colors.RED, err=True)
                raise typer.Exit(code=3) from e
            typer.secho(f"skip: {e}", fg=typer.colors.YELLOW, err=True)
            continue

        path = out_dir / f"{task.task_id}.json"
        # pydantic's `model_dump(mode='json')` emits the same shape the
        # Rust type round-trips through serde, which is what the JSON
        # schema expects. We double-check against the shipped JSON
        # schema so pydantic / jsonschema divergences fail loudly.
        payload = task.model_dump(mode="json")
        try:
            validate_benchmark_task(payload)
        except ValidationError as e:
            err = SweBenchNormalizationError(
                task.task_id, f"emitted BenchmarkTask fails JSON schema: {e.message}"
            )
            errs.append(err)
            if strict:
                typer.secho(str(err), fg=typer.colors.RED, err=True)
                raise typer.Exit(code=3) from e
            typer.secho(f"skip: {err}", fg=typer.colors.YELLOW, err=True)
            continue
        # On-disk BenchmarkTask files carry a trailing `\n` to match
        # the Rust adapter's writer (see `packages/rust/benchmarks/src/
        # writer.rs`). The canonical_json bytes themselves do not
        # include the newline - adding it here keeps the writer
        # convention identical across the two adapters.
        path.write_bytes(canonical_json(payload) + b"\n")
        ok += 1

    typer.echo(
        f"normalized {ok} / {len(records)} SWE-bench records -> {out_dir}"
    )
    for diag in errs:
        typer.secho(f"skipped: {diag}", fg=typer.colors.YELLOW, err=True)

    if ok == 0:
        typer.secho(
            f"{source}: every record failed normalization", fg=typer.colors.RED, err=True
        )
        raise typer.Exit(code=3)


def main() -> None:
    """Entrypoint for the ``eval-ladder-py`` console script."""

    app()


if __name__ == "__main__":  # pragma: no cover - exercised via the console script
    main()


__all__ = [
    "SweBenchNormalizationError",
    "app",
    "main",
    "normalize_swe_bench",
    "version",
]
