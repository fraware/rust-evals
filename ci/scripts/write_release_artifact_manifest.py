#!/usr/bin/env python3
"""Emit a JSON manifest of release-critical repository fingerprints.

Intended for tag builds (see ``.github/workflows/release-tag.yml``). The
document is **not** a full software bill of materials; it pins the evaluator
core, Lean obligation surface, and curated proof subset so reviewers can
diff releases without hashing the entire tree.

Writes to stdout and optionally to ``--out``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def _git_output(args: list[str], cwd: Path) -> str:
    try:
        return subprocess.check_output(["git", *args], cwd=cwd, text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        return ""


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--repo-root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="repository root (default: inferred from this script)",
    )
    ap.add_argument(
        "--out",
        type=Path,
        default=None,
        help="optional path to write the same JSON (UTF-8, sorted keys)",
    )
    args = ap.parse_args()
    root: Path = args.repo_root.resolve()

    rel_paths = [
        "rust-toolchain.toml",
        "Cargo.toml",
        "Cargo.lock",
        "datasets/derived/proof_subset/manifest.jsonl",
        "packages/lean/EvalLadder/lean-toolchain",
        "packages/lean/EvalLadder/lakefile.lean",
        "packages/lean/EvalLadder/EvalLadder.lean",
    ]

    files: dict[str, str] = {}
    missing: list[str] = []
    for rel in rel_paths:
        p = root / rel
        if not p.is_file():
            missing.append(rel)
            continue
        files[rel.replace("\\", "/")] = _sha256_file(p)

    manifest = {
        "schema_version": 1,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "git_commit": _git_output(["rev-parse", "HEAD"], root),
        "git_describe": _git_output(["describe", "--tags", "--always"], root),
        "git_ref": _git_output(["symbolic-ref", "-q", "HEAD"], root),
        "files_sha256": dict(sorted(files.items())),
        "missing_paths": missing,
    }

    text = json.dumps(manifest, sort_keys=True, indent=2) + "\n"
    sys.stdout.write(text)
    if args.out is not None:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
