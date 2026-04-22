"""End-to-end tests for the ``eval-ladder-py`` CLI."""

from __future__ import annotations

import json
from pathlib import Path

from benchmark_compat.canonical import canonical_json
from benchmark_compat.cli import app
from benchmark_compat.swe_bench import normalize_instance
from typer.testing import CliRunner


def _run(*args: str) -> object:
    runner = CliRunner()
    return runner.invoke(app, list(args))


def test_version_subcommand_prints_package_version() -> None:
    result = _run("version")
    assert result.exit_code == 0
    assert "benchmark_compat" in result.stdout


def test_normalize_swe_bench_emits_canonical_task_files(
    swe_bench_manifest: Path, tmp_path: Path
) -> None:
    out_dir = tmp_path / "out"
    result = _run(
        "normalize-swe-bench",
        "--source",
        str(swe_bench_manifest),
        "--out-dir",
        str(out_dir),
    )
    assert result.exit_code == 0, result.stdout + result.stderr

    first = out_dir / "octo-org__widget-7277.json"
    second = out_dir / "octo-org__widget-7278.json"
    assert first.is_file()
    assert second.is_file()

    # Byte-level determinism: feeding the same record through
    # `normalize_instance` and dumping it manually must match the
    # on-disk file byte-for-byte.
    record = json.loads(
        swe_bench_manifest.read_text(encoding="utf-8").splitlines()[0]
    )
    # The CLI appends a trailing `\n` to match the Rust writer
    # convention; canonical_json itself does not include one.
    expected = canonical_json(normalize_instance(record).model_dump(mode="json")) + b"\n"
    assert first.read_bytes() == expected


def test_normalize_swe_bench_is_deterministic_across_reruns(
    swe_bench_manifest: Path, tmp_path: Path
) -> None:
    out_a = tmp_path / "a"
    out_b = tmp_path / "b"
    first = _run(
        "normalize-swe-bench",
        "--source",
        str(swe_bench_manifest),
        "--out-dir",
        str(out_a),
    )
    second = _run(
        "normalize-swe-bench",
        "--source",
        str(swe_bench_manifest),
        "--out-dir",
        str(out_b),
    )
    assert first.exit_code == 0
    assert second.exit_code == 0
    for name in ("octo-org__widget-7277.json", "octo-org__widget-7278.json"):
        assert (out_a / name).read_bytes() == (out_b / name).read_bytes()


def test_normalize_swe_bench_continues_past_bad_records(
    swe_bench_manifest: Path, tmp_path: Path
) -> None:
    # Append a malformed record to the manifest so one record is bad.
    extra = {"instance_id": "octo-org__widget-9999"}  # missing required fields
    with swe_bench_manifest.open("a", encoding="utf-8") as f:
        f.write(json.dumps(extra) + "\n")

    out_dir = tmp_path / "out"
    result = _run(
        "normalize-swe-bench",
        "--source",
        str(swe_bench_manifest),
        "--out-dir",
        str(out_dir),
    )
    # Bad record is skipped, the two good records are emitted; exit 0.
    assert result.exit_code == 0, result.stdout + result.stderr
    files = sorted(p.name for p in out_dir.iterdir())
    assert files == [
        "octo-org__widget-7277.json",
        "octo-org__widget-7278.json",
    ]


def test_normalize_swe_bench_strict_aborts_on_first_bad_record(
    swe_bench_manifest: Path, tmp_path: Path
) -> None:
    with swe_bench_manifest.open("w", encoding="utf-8") as f:
        f.write(json.dumps({"instance_id": "bad-no-fields"}) + "\n")

    out_dir = tmp_path / "out"
    result = _run(
        "normalize-swe-bench",
        "--source",
        str(swe_bench_manifest),
        "--out-dir",
        str(out_dir),
        "--strict",
    )
    assert result.exit_code == 3


def test_normalize_swe_bench_rejects_missing_source(tmp_path: Path) -> None:
    out_dir = tmp_path / "out"
    result = _run(
        "normalize-swe-bench",
        "--source",
        str(tmp_path / "does-not-exist.jsonl"),
        "--out-dir",
        str(out_dir),
    )
    assert result.exit_code != 0
