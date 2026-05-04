#!/usr/bin/env python3
"""Scan release-facing paths for accidental secrets or deanonymization tokens.

Run from the repository root. Intended for NeurIPS E&D / public-release hygiene.
Excludes ``agent-transcripts`` and ``.cursor`` subtrees (local IDE paths).

Exit code: ``0`` when no **error** hits; warnings print to stderr but do not fail.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

# Keep this fast: do not walk all of ``runs/released`` or ``datasets/cache`` (too large).
DEFAULT_ROOTS = (
    "configs",
    "docs",
    "paper",
    ".github/workflows",
    "datasets/public_links",
)

# Paper-cited evidence directories only (exclude vendored task workspaces under
# ``live_panel_v2/workspaces/``, which contain upstream CI YAML mentioning env names).
RELEASE_SLICE_ROOTS = (
    "runs/released/live_panel_v2/results_opt",
    "runs/released/l2_verified_flagship_v1/results",
    "runs/released/rust_proof_subset_v1/results_seal",
)

SKIP_DIR_NAMES = frozenset(
    {".git", "target", "node_modules", "__pycache__", ".venv", "venv", ".mypy_cache"}
)

SECRET_TOKENS = (
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "GITHUB_TOKEN",
)

IDENT_TOKENS = (
    "Mateo",
    "Petel",
    "@stanford",
)

TEXT_SUFFIXES = frozenset(
    {
        ".md",
        ".txt",
        ".json",
        ".jsonl",
        ".yaml",
        ".yml",
        ".toml",
        ".py",
        ".rs",
        ".tex",
        ".csv",
        ".sh",
        ".ps1",
        ".workflow",
        ".graphql",
    }
)

MAX_FILE_BYTES = 2_000_000


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _should_skip_path(path: Path) -> bool:
    parts = frozenset(path.parts)
    if "agent-transcripts" in parts:
        return True
    if ".cursor" in parts:
        return True
    pos = path.as_posix()
    if "/runs/released/" in pos and "/workspaces/" in pos:
        return True
    return any(p in SKIP_DIR_NAMES for p in path.parts)


def _is_probably_text(path: Path) -> bool:
    if path.suffix.lower() in TEXT_SUFFIXES:
        return True
    name = path.name.lower()
    return name in {".env", ".env.example", "dockerfile", "justfile"}


def _iter_candidate_files(repo: Path, roots: tuple[str, ...]) -> list[Path]:
    out: list[Path] = []
    for rel in roots:
        base = (repo / rel).resolve()
        if not base.exists():
            continue
        if base.is_file():
            out.append(base)
            continue
        for p in base.rglob("*"):
            if not p.is_file() or _should_skip_path(p):
                continue
            try:
                if p.stat().st_size > MAX_FILE_BYTES:
                    continue
            except OSError:
                continue
            if _is_probably_text(p):
                out.append(p)
    return out


def _all_scan_roots() -> tuple[str, ...]:
    return DEFAULT_ROOTS + RELEASE_SLICE_ROOTS


def _scan_text(rel_posix: str, text: str) -> tuple[list[str], list[str]]:
    errors: list[str] = []
    warnings: list[str] = []
    for tok in SECRET_TOKENS:
        if tok in text:
            errors.append(f"{rel_posix}: substring {tok}")
    for tok in IDENT_TOKENS:
        if tok in text:
            errors.append(f"{rel_posix}: substring {tok!r}")
    if "fraware" in text.lower():
        warnings.append(
            f"{rel_posix}: contains 'fraware' "
            "(often a GitHub org slug; remove if building an anonymized archive)"
        )
    return errors, warnings


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--repo-root",
        type=Path,
        default=_repo_root(),
        help="Repository root (default: infer from script location).",
    )
    args = p.parse_args()
    repo = args.repo_root.resolve()

    all_errors: list[str] = []
    all_warnings: list[str] = []
    for path in sorted(_iter_candidate_files(repo, _all_scan_roots())):
        try:
            raw = path.read_bytes()
        except OSError:
            continue
        try:
            text = raw.decode("utf-8")
        except UnicodeDecodeError:
            continue
        try:
            rel_posix = path.relative_to(repo).as_posix()
        except ValueError:
            rel_posix = str(path)
        errs, warns = _scan_text(rel_posix, text)
        all_errors.extend(errs)
        all_warnings.extend(warns)

    for w in all_warnings:
        print(f"secret_scan_release: WARN: {w}", file=sys.stderr)
    if all_errors:
        for e in all_errors:
            print(f"secret_scan_release: FAIL: {e}", file=sys.stderr)
        return 1
    print("secret_scan_release: OK (no secret/id hits)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
