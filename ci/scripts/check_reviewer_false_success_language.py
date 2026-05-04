#!/usr/bin/env python3
"""Fail if reviewer-facing docs reintroduce deprecated ``false success`` wording.

Scan either the development tree (default) or a staged anonymous tree / tarball.

Allowed in scanned files:

- Legacy-on-disk explanations that mention ``conditional_false_success`` only on
  lines that also reference deprecated / byte-identical / Legacy / alias wording.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tarfile
import tempfile
from pathlib import Path


_FALSE_SUCCESS = re.compile(
    r"\bfalse\s+success\b|false-success",
    re.IGNORECASE,
)
_FALSE_SUCCESS_UNDERSCORE = re.compile(r"(?<!conditional_)false_success\b")
_CONDITIONAL_FALSE = re.compile(r"conditional_false")


def _legacy_conditional_line(line: str) -> bool:
    low = line.lower()
    return any(
        x in low
        for x in (
            "legacy",
            "deprecated",
            "byte-identical",
            "backward compat",
            "alias",
        )
    )


def _scan_tree(root: Path, *, label: str) -> int:
    scan_rel = [
        Path("README.md"),
        Path("REVIEWER_REPRODUCTION.md"),
        Path("docs") / "evaluation_ladder.md",
        Path("paper") / "MANUSCRIPT_GUIDE_NEURIPS2026.md",
        Path("paper") / "README.md",
        Path("paper") / "exports" / "CLAIM_SOURCE_MAP.md",
    ]
    failures: list[str] = []
    for rel in scan_rel:
        path = root / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="replace")
        for i, line in enumerate(text.splitlines(), 1):
            if _FALSE_SUCCESS.search(line):
                failures.append(f"{label}:{rel.as_posix()}:{i}: {line.strip()[:200]}")
            if _FALSE_SUCCESS_UNDERSCORE.search(line):
                failures.append(
                    f"{label}:{rel.as_posix()}:{i}: false_success token: {line.strip()[:200]}"
                )
            if _CONDITIONAL_FALSE.search(line) and not _legacy_conditional_line(line):
                failures.append(
                    f"{label}:{rel.as_posix()}:{i}: conditional_false without legacy context: "
                    f"{line.strip()[:200]}"
                )

    if failures:
        for f in failures:
            print(f"check_reviewer_false_success_language: FAIL: {f}", file=sys.stderr)
        return 1
    print(f"check_reviewer_false_success_language: OK ({label})", file=sys.stderr)
    return 0


def _scan_repo_default(root: Path) -> int:
    return _scan_tree(root, label="repo")


def _scan_staged_root(staged: Path) -> int:
    return _scan_tree(staged.resolve(), label="staged-bundle")


def _scan_archive(archive: Path) -> int:
    archive = archive.resolve()
    if not archive.is_file():
        print(f"check_reviewer_false_success_language: missing {archive}", file=sys.stderr)
        return 1
    work = Path(tempfile.mkdtemp(prefix="eval_ladder_lang_check_"))
    try:
        with tarfile.open(archive, "r:gz") as tf:
            try:
                tf.extractall(work, filter="data")  # type: ignore[call-arg]
            except TypeError:
                tf.extractall(work)
        children = [c for c in work.iterdir()]
        inner = children[0] if len(children) == 1 and children[0].is_dir() else work
        return _scan_tree(inner, label="tarball")
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main() -> int:
    root = Path(__file__).resolve().parents[2]
    p = argparse.ArgumentParser(description=__doc__)
    g = p.add_mutually_exclusive_group()
    g.add_argument(
        "--staged-root",
        type=Path,
        help="Root of an extracted or staged anonymous tree (e.g. build/eval-ladder-anon-stage).",
    )
    g.add_argument(
        "--archive",
        type=Path,
        help="Anonymous .tar.gz to extract temporarily and scan.",
    )
    args = p.parse_args()
    if args.staged_root is not None:
        return _scan_staged_root(args.staged_root)
    if args.archive is not None:
        return _scan_archive(args.archive)
    return _scan_repo_default(root)


if __name__ == "__main__":
    raise SystemExit(main())
