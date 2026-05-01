#!/usr/bin/env python3
"""Canonical paper export pipeline: regenerate CSV/JSON/TEX under paper/."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

STABLE_EXPORT_SCHEMA_VERSION = 3


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(path)
    return data


def _ensure_manifest_schema(path: Path, label: str) -> None:
    if not path.is_file():
        raise FileNotFoundError(f"missing {label}: {path}")
    m = _load_json(path)
    sv = m.get("schema_version")
    if sv is None:
        raise ValueError(f"{label} missing schema_version: {path}")
    if int(sv) != STABLE_EXPORT_SCHEMA_VERSION:
        raise ValueError(
            f"{label} stale schema_version={sv!r} "
            f"(need {STABLE_EXPORT_SCHEMA_VERSION}): {path}"
        )


def _git_commit(repo_root: Path) -> str:
    try:
        out = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            capture_output=True,
            text=True,
            check=True,
        )
        return out.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def _run_py(repo_root: Path, rel_script: Path, argv: list[str]) -> None:
    cmd = [sys.executable, str(repo_root / rel_script)] + argv
    subprocess.run(cmd, cwd=repo_root, check=True)


def _run_eval_ladder(repo_root: Path, bin_path: Path, run_dir: Path, out_dir: Path) -> None:
    if not bin_path.is_file():
        raise FileNotFoundError(
            f"release CLI missing: {bin_path} (run `cargo build -p eval-ladder-cli --release` "
            "or set --eval-ladder-bin)"
        )
    subprocess.run(
        [
            str(bin_path),
            "analyze",
            "paper-export",
            "--run-dir",
            str(run_dir),
            "--out-dir",
            str(out_dir),
        ],
        cwd=repo_root,
        check=True,
        stdout=subprocess.DEVNULL,
    )


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--eval-ladder-bin",
        type=Path,
        default=None,
        help="Override path to eval-ladder release binary.",
    )
    args = p.parse_args()

    if sys.platform.startswith("win"):
        default_bin = repo_root / "target" / "release" / "eval-ladder.exe"
    else:
        default_bin = repo_root / "target" / "release" / "eval-ladder"
    bin_path = args.eval_ladder_bin or default_bin

    live_run = repo_root / "runs/released/live_panel_v2/results_opt"
    l2_run = repo_root / "runs/released/l2_verified_flagship_v1/results"
    rust_seal_run = repo_root / "runs/released/rust_proof_subset_v1/results_seal"
    live_summary = live_run / "batch_summary.json"
    l2_summary = l2_run / "batch_summary.json"
    rust_seal_summary = rust_seal_run / "batch_summary.json"
    gold_results = (
        repo_root / "runs/released/l2_verified_flagship_v1/gold_patch_results"
    )
    gold_csv = (
        repo_root / "paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv"
    )
    proof_manifest = repo_root / "datasets/derived/proof_subset/manifest.jsonl"

    for path, label in [
        (live_summary, "live batch_summary"),
        (l2_summary, "L2 batch_summary"),
        (rust_seal_summary, "rust proof seal batch_summary"),
        (proof_manifest, "proof manifest"),
    ]:
        if not path.is_file():
            print(f"error: missing {label}: {path}", file=sys.stderr)
            return 1

    if not gold_csv.is_file():
        if not gold_results.is_dir():
            print(
                "error: need paper gold export or sealed gold_patch_results directory.\n"
                f"  missing: {gold_csv}\n"
                f"  missing: {gold_results}",
                file=sys.stderr,
            )
            return 1

    live_out = repo_root / "paper/exports/live_panel_v2_postbatch"
    l2_out = repo_root / "paper/exports/l2_verified_flagship_v1"
    rust_out = repo_root / "paper/exports/rust_proof_subset_v1_seal_release"
    tex_dir = repo_root / "paper/tables"

    _run_eval_ladder(repo_root, bin_path.resolve(), live_run, live_out)
    # Merged L2 `results/` contains `batch_summary.json` but no per-bundle
    # subdirectories, so `analyze paper-export` yields empty analysis inputs.
    # `export_l2_flagship_tables.py` (run below) rebuilds cohort CSV/TeX from the
    # merged summary; keep that step immediately after this export.
    _run_eval_ladder(repo_root, bin_path.resolve(), l2_run, l2_out)
    _run_eval_ladder(repo_root, bin_path.resolve(), rust_seal_run, rust_out)

    _ensure_manifest_schema(live_out / "manifest.json", "live paper-export manifest")
    _ensure_manifest_schema(l2_out / "manifest.json", "L2 paper-export manifest")
    _ensure_manifest_schema(
        rust_out / "manifest.json",
        "rust proof seal paper-export manifest",
    )

    _run_py(
        repo_root,
        Path("packages/python/scripts/export_live_panel_tables.py"),
        [
            "--run-dir",
            str(live_run),
            "--out-dir",
            str(live_out),
        ],
    )

    _run_py(repo_root, Path("ci/scripts/export_l2_flagship_reviews.py"), [])

    _run_py(
        repo_root,
        Path("packages/python/scripts/export_l2_flagship_tables.py"),
        [
            "--run-dir",
            str(l2_run),
            "--out-dir",
            str(l2_out),
            "--tex-dir",
            str(tex_dir),
            "--gold-csv",
            str(gold_csv),
        ],
    )

    feas_out = repo_root / "paper/exports/strict_feasibility_report.json"
    _run_py(
        repo_root,
        Path("ci/scripts/analyze_strict_feasibility.py"),
        ["--out", str(feas_out)],
    )

    _run_py(
        repo_root,
        Path("packages/python/scripts/export_frontier_tables.py"),
        [
            "--feasibility",
            str(feas_out),
            "--proof-manifest",
            str(proof_manifest),
            "--tex-dir",
            str(tex_dir),
        ],
    )

    _run_py(
        repo_root,
        Path("ci/scripts/export_l2_selection_manifest.py"),
        [],
    )

    input_paths = [
        live_summary,
        l2_summary,
        rust_seal_summary,
        gold_csv,
        feas_out,
        proof_manifest,
    ]
    output_paths: list[Path] = []
    for base in [live_out, l2_out, rust_out, tex_dir]:
        if base.is_dir():
            output_paths.extend(sorted(base.rglob("*")))
    output_paths = [x for x in output_paths if x.is_file()]
    extra_exports = [
        repo_root / "paper/exports/reproduction_manifest.json",
        feas_out,
        repo_root / "docs/l2_failure_case_studies.md",
    ]
    output_paths.extend(extra_exports)

    manifest_path = repo_root / "paper/exports/reproduction_manifest.json"
    manifest_path.parent.mkdir(parents=True, exist_ok=True)

    commit = _git_commit(repo_root)
    gen_at = datetime.now(timezone.utc).isoformat()

    inp_hashes = {str(x.relative_to(repo_root)): _sha256_file(x) for x in input_paths}
    out_hashes = {}
    for x in sorted(set(output_paths)):
        if x.is_file():
            try:
                rel = str(x.relative_to(repo_root))
            except ValueError:
                continue
            out_hashes[rel] = _sha256_file(x)

    manifest_path.write_text(
        json.dumps(
            {
                "repo_commit": commit,
                "generated_at": gen_at,
                "export_manifest_schema_expected": STABLE_EXPORT_SCHEMA_VERSION,
                "input_paths": [str(x.relative_to(repo_root)) for x in input_paths],
                "output_paths": sorted(out_hashes.keys()),
                "input_hashes": inp_hashes,
                "output_hashes": out_hashes,
            },
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )

    print(json.dumps({"reproduction_manifest": str(manifest_path.relative_to(repo_root))}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
