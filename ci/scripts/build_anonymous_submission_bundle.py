#!/usr/bin/env python3
"""Stage an anonymous NeurIPS-style submission tree (no ``.git``) and optional tar.gz.

Usage (from repository root)::

    python ci/scripts/build_anonymous_submission_bundle.py --profile neurips-paper --dry-run-size
    python ci/scripts/build_anonymous_submission_bundle.py --tar-gz --profile neurips-paper

``--tar-gz`` stages under ``build/eval-ladder-anon-stage/`` and writes
``build/eval-ladder-anonymous-bundle.tar.gz`` (override with ``--archive``). Default ``--profile`` is
``neurips-paper`` (sealed claim paths only). Use ``--profile full`` for the
legacy bundle that copies all of ``runs/``.

The script copies an allow-listed set of paths, **sanitizes** identifying metadata
in the staged tree, then **fails closed** if scrub patterns still match. The
final SHA-256 of the tarball must be recorded **only** outside the archive (see
``build/RELEASE_MANIFEST.md`` via ``ci/scripts/write_release_manifest.py``).

**Grep scope policy:** Full-tree greps will match legacy filenames such as
``conditional_false_success.csv`` inside staged ``paper/exports/**`` manifests.
Reviewer-facing terminology greps should use ``--reviewer-facing-only``.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import sys
import tarfile
from pathlib import Path

from anonymization_scrub_lib import (
    collect_violations,
    count_workspace_scrub_hits,
    sanitize_staged_tree,
    scrub_exempt,
)


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


# Full-repository bundle (legacy / disk-rich environments).
_DEFAULT_INCLUDE_DIRS: tuple[str, ...] = (
    "packages",
    "configs",
    "ci",
    "docs",
    "paper",
    "datasets",
    "runs",
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "pyproject.toml",
    "README.md",
    "REVIEWER_REPRODUCTION.md",
    "NOTICE",
    "LICENSE",
    "justfile",
)

_SKIP_DIR_NAMES = {
    ".git",
    ".github",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "target",
    "node_modules",
    ".lake",
}

_SKIP_FILE_NAMES = {".DS_Store", "Thumbs.db"}


# NeurIPS paper bundle: sealed runs + exports only (no historical ``runs/`` root).
_NEURIPS_PAPER_BUNDLE: tuple[tuple[str, str], ...] = (
    ("schemas", "JSON Schema drafts for `eval-ladder schema validate`."),
    (
        "benchmarks/verified/manifests",
        "Verified benchmark task manifests (release manifest audit, 500 tasks).",
    ),
    (
        "datasets/derived/proof_subset/manifest.jsonl",
        "Rust proof subset task list for `reproduce_paper_tables.py`.",
    ),
    ("packages/python", "Python export scripts and benchmark compatibility layer."),
    ("packages/rust", "Rust workspace sources (evaluator core, CLI, runner, evidence)."),
    (
        "packages/lean",
        "Lean obligation sources (``.lake/`` build cache excluded at copy time).",
    ),
    ("configs", "Repository configuration templates referenced by tooling."),
    ("ci", "Gate scripts including `run_evidence_tier1_checks.py`, anonymize, verify."),
    ("paper", "Paper exports and TeX tables (local `paper/`; claim map is `docs/paper_claim_sources.json`)."),
    ("Cargo.toml", "Rust workspace manifest."),
    ("Cargo.lock", "Locked dependency versions."),
    ("rust-toolchain.toml", "Pinned Rust toolchain."),
    ("pyproject.toml", "Python packaging and static-tool configuration."),
    ("README.md", "Top-level reviewer entrypoint."),
    ("LICENSE", "License terms."),
    ("NOTICE", "Third-party notice file."),
    ("justfile", "Hermetic task shortcuts."),
    ("REVIEWER_REPRODUCTION.md", "Tier A/B/C reproduction commands."),
    (
        "docs/evidence_manual.md",
        "Selection protocols, L2 gold validation and case studies, Rust paper-semantics replay, "
        "and operational runbook (claim-source doc guards).",
    ),
    ("docs/evaluation_ladder.md", "Evaluation ladder semantics for reviewers."),
    ("docs/submission_checklist.md", "Venue submission checklist (reference)."),
    ("docs/scientific_scope.md", "Scientific scope and Tier D claim-lock prose (`check_paper_claim_sources`)."),
    (
        "runs/released/live_panel_v2/results_opt",
        "Sealed Live v2 bundles (`analyze paper-export`, Tier B `verify`).",
    ),
    (
        "runs/released/l2_verified_flagship_v1/results",
        "Merged L2 flagship `batch_summary.json` for paper-export regeneration.",
    ),
    (
        "runs/released/l2_verified_flagship_v1/results_astropy",
        "Sealed augmented-test arm per-bundle leaves (integrity verify).",
    ),
    (
        "runs/released/l2_verified_flagship_v1/results_regression_fail",
        "Sealed regression stress-control arm per-bundle leaves (integrity verify).",
    ),
    (
        "runs/released/l2_verified_flagship_v1/gold_patch_results",
        "Sealed gold-patch artifacts for L2 gold validation exports.",
    ),
    (
        "runs/released/rust_proof_subset_v1/results_seal",
        "Sealed Rust proof subset run (`analyze paper-export`, Tier B verify).",
    ),
    (
        "runs/released/rust_proof_subset_v1/results_fast",
        "Sealed structural rust-proof panel (rust-proof structural gate, 8 entries).",
    ),
    (
        "runs/released/l0l1_pass_hunt_v1/panel.jsonl",
        "Verified preflight panel fixture (Verified selector preflight gate).",
    ),
    (
        "tests/integration",
        "Rust workspace integration test crate required by workspace `members`.",
    ),
)


def _neurips_paper_includes() -> tuple[str, ...]:
    return tuple(p for p, _ in _NEURIPS_PAPER_BUNDLE)


def _bundle_inventory_lines(
    profile: str, custom_includes: tuple[str, ...] | None = None
) -> list[tuple[str, str]]:
    if custom_includes is not None:
        return [(p, "Included via explicit `--include` override.") for p in custom_includes]
    if profile == "neurips-paper":
        return list(_NEURIPS_PAPER_BUNDLE)
    return [(p, "Full profile include (see repository layout).") for p in _DEFAULT_INCLUDE_DIRS]


def _write_bundle_inventory_md(
    repo: Path,
    out: Path,
    profile: str,
    *,
    custom_includes: tuple[str, ...] | None = None,
) -> None:
    lines = ["# Bundle inventory", "", f"**Profile:** `{profile}`", ""]
    for path, purpose in _bundle_inventory_lines(profile, custom_includes):
        sp = repo / path
        path_line = f"**Path:** `{path}`" if sp.is_file() else f"**Path:** `{path}/`"
        lines.extend([path_line, f"**Purpose:** {purpose}", ""])
    out.write_text("\n".join(lines), encoding="utf-8")


def _estimate_staged_bytes(repo: Path, includes: tuple[str, ...]) -> tuple[int, int, list[tuple[str, int]]]:
    """Return (file_count, total_bytes, per-include-byte-list sorted descending)."""
    per: list[tuple[str, int]] = []
    total_files = 0
    total_bytes = 0
    for item in includes:
        sp = repo / item
        if not sp.exists():
            per.append((item, 0))
            continue
        n_files = 0
        n_bytes = 0
        if sp.is_file():
            n_files = 1
            n_bytes = sp.stat().st_size
        else:
            for p in sp.rglob("*"):
                if not p.is_file():
                    continue
                if set(p.relative_to(sp).parts) & _SKIP_DIR_NAMES:
                    continue
                if p.name in _SKIP_FILE_NAMES:
                    continue
                if ".lake" in p.parts:
                    continue
                try:
                    n_bytes += p.stat().st_size
                except OSError:
                    continue
                n_files += 1
        total_files += n_files
        total_bytes += n_bytes
        per.append((item, n_bytes))
    per.sort(key=lambda x: x[1], reverse=True)
    return total_files, total_bytes, per


def _copy_tree(src: Path, dst: Path, *, rel: Path) -> None:
    name = rel.name
    if name in _SKIP_DIR_NAMES or name in _SKIP_FILE_NAMES:
        return
    src_path = src / rel
    dst_path = dst / rel
    if src_path.is_dir():
        dst_path.mkdir(parents=True, exist_ok=True)
        for child in sorted(src_path.iterdir()):
            _copy_tree(src, dst, rel=rel / child.name)
    elif src_path.is_file():
        dst_path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src_path, dst_path)


def _stage_repo(repo: Path, out_dir: Path, include: tuple[str, ...]) -> None:
    if out_dir.exists():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    for item in include:
        sp = repo / item
        if not sp.exists():
            continue
        if sp.is_file():
            dest = out_dir / item
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(sp, dest)
        else:
            _copy_tree(repo, out_dir, rel=Path(item))


_TEXT_SUFFIXES = {
    ".md",
    ".toml",
    ".json",
    ".jsonl",
    ".yaml",
    ".yml",
    ".rs",
    ".py",
    ".tex",
    ".txt",
    ".csv",
    ".sh",
    ".just",
    "",
}


def _iter_files(root: Path) -> list[Path]:
    if root.is_file():
        return [root]
    out: list[Path] = []
    for p in root.rglob("*"):
        if p.is_file() and p.suffix in _TEXT_SUFFIXES | {".gitignore"}:
            parts = set(p.parts)
            if parts & _SKIP_DIR_NAMES:
                continue
            if p.name in _SKIP_FILE_NAMES:
                continue
            out.append(p)
    return sorted(out)


def _run_grep(_repo: Path, staged: Path, reviewer_only: bool) -> None:
    """Advisory greps (post-sanitize). Patterns aligned with ``anonymization_scrub_lib``."""
    roots: list[Path]
    if reviewer_only:
        roots = [
            staged / "docs",
            staged / "paper" / "tables",
            staged / "paper" / "exports" / "evaluator_cards",
        ]
        if (staged / "README.md").is_file():
            roots.append(staged / "README.md")
    else:
        roots = [staged]

    patterns: list[tuple[str, re.Pattern[str]]] = [
        (
            "identity_leak",
            re.compile(
                r"fraware|Mateo|Petel|mpetel|stanford|github\.com/fraware|rust-evals",
                re.IGNORECASE,
            ),
        ),
        (
            "paths",
            re.compile(
                r"/Users/|/home/[^/\s]+|C:\\\\Users|/mnt/data|/tmp/.*mateo|D:\\\\a\\\\",
                re.I,
            ),
        ),
        (
            "email",
            re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}"),
        ),
        (
            "social",
            re.compile(r"linkedin\.com|twitter\.com|x\.com/", re.I),
        ),
    ]

    for label, rx in patterns:
        for root in roots:
            if not root.exists():
                continue
            hits: list[str] = []
            for fp in _iter_files(root):
                rel = fp.relative_to(staged)
                if scrub_exempt(rel):
                    continue
                try:
                    text = fp.read_text(encoding="utf-8", errors="replace")
                except OSError:
                    continue
                for i, line in enumerate(text.splitlines(), 1):
                    if rx.search(line):
                        hits.append(f"{rel}:{i}:{line.strip()[:200]}")
                if len(hits) > 500:
                    hits.append("... truncated ...")
                    break
            if hits:
                print(f"[{label}] matches under {root}:", file=sys.stderr)
                for h in hits[:200]:
                    print(h, file=sys.stderr)


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _inject_workspace_audit(report: Path, counts: dict[str, int]) -> None:
    if not report.is_file():
        return
    text = report.read_text(encoding="utf-8")
    for key, val in (
        ("__WORKSPACE_COUNT_URL__", str(counts["url"])),
        ("__WORKSPACE_COUNT_IDENTITY__", str(counts["identity"])),
        ("__WORKSPACE_COUNT_PATHS__", str(counts["paths"])),
        ("__WORKSPACE_COUNT_EMAIL__", str(counts["email"])),
    ):
        text = text.replace(key, val)
    report.write_text(text, encoding="utf-8")


def _write_tar_gz(src_dir: Path, archive: Path) -> None:
    archive.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive, "w:gz") as tf:
        tf.add(src_dir, arcname=src_dir.name, recursive=True)


def main() -> int:
    root = _repo_root()
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--profile",
        choices=("neurips-paper", "full"),
        default="neurips-paper",
        help=(
            "Bundle contents: `neurips-paper` (sealed claim paths only, default) or "
            "`full` (entire `runs/`, `datasets/`, all `docs/`)."
        ),
    )
    p.add_argument(
        "--dry-run-size",
        action="store_true",
        help="Estimate file count and bytes for the selected profile; do not copy.",
    )
    p.add_argument(
        "--tar-gz",
        action="store_true",
        help=(
            "Stage to build/eval-ladder-anon-stage and write "
            "build/eval-ladder-anonymous-bundle.tar.gz."
        ),
    )
    p.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Staging directory (default with --tar-gz: build/eval-ladder-anon-stage).",
    )
    p.add_argument(
        "--archive",
        type=Path,
        default=None,
        help="If set (or implied by --tar-gz), write a .tar.gz of the staged directory.",
    )
    p.add_argument(
        "--include",
        nargs="*",
        default=None,
        help="Override include list (paths relative to repo root). Implies --profile full logic if set.",
    )
    p.add_argument(
        "--reviewer-facing-only",
        action="store_true",
        help="Run advisory greps only on reviewer-facing subtrees (post-sanitize).",
    )
    args = p.parse_args()

    if args.include is not None:
        includes = tuple(args.include)
        profile_name = "custom-include"
    elif args.profile == "full":
        includes = _DEFAULT_INCLUDE_DIRS
        profile_name = "full"
    else:
        includes = _neurips_paper_includes()
        profile_name = "neurips-paper"

    if args.dry_run_size:
        n_files, n_bytes, per = _estimate_staged_bytes(root, includes)
        gb = n_bytes / (1024**3)
        print(
            json.dumps(
                {
                    "profile": profile_name,
                    "estimated_files": n_files,
                    "estimated_uncompressed_bytes": n_bytes,
                    "estimated_uncompressed_gb": round(gb, 3),
                    "largest_include_paths": [{"path": a, "bytes": b} for a, b in per[:12]],
                },
                indent=2,
            ),
            file=sys.stderr,
        )
        return 0

    if args.tar_gz:
        out_dir = (args.out_dir or (root / "build" / "eval-ladder-anon-stage")).resolve()
        archive = args.archive or (root / "build" / "eval-ladder-anonymous-bundle.tar.gz")
        args.archive = archive.resolve()
    else:
        if args.out_dir is None:
            p.error("--out-dir is required unless --tar-gz")
        out_dir = args.out_dir.resolve()

    _stage_repo(root, out_dir, includes)
    inv_path = out_dir / "BUNDLE_INVENTORY.md"
    _write_bundle_inventory_md(
        root,
        inv_path,
        profile_name,
        custom_includes=includes if args.include is not None else None,
    )
    (root / "build").mkdir(parents=True, exist_ok=True)
    shutil.copy2(inv_path, root / "build" / "BUNDLE_INVENTORY.md")

    if (out_dir / ".git").exists():
        print("error: staged tree must not contain .git", file=sys.stderr)
        return 1

    print(f"Staged anonymous tree at: {out_dir}", file=sys.stderr)
    n_san = sanitize_staged_tree(out_dir)
    print(f"Sanitized {n_san} text files in staged tree", file=sys.stderr)

    ws_counts = count_workspace_scrub_hits(out_dir)
    _inject_workspace_audit(out_dir / "ANONYMIZATION_REPORT.md", ws_counts)
    print(
        "Workspace audit (ignored third-party matches): "
        f"url={ws_counts['url']} identity={ws_counts['identity']} "
        f"paths={ws_counts['paths']} email={ws_counts['email']}",
        file=sys.stderr,
    )

    violations = collect_violations(out_dir)
    if violations:
        print(
            "build_anonymous_submission_bundle: staged tree failed scrub "
            "(fix sources or extend sanitization):",
            file=sys.stderr,
        )
        for line in violations[:200]:
            print(line, file=sys.stderr)
        return 1

    # Optional: sync anonymization report if this optional file was staged.
    staged_anon = out_dir / "ANONYMIZATION_REPORT.md"
    if staged_anon.is_file():
        shutil.copy2(staged_anon, root / "ANONYMIZATION_REPORT.md")

    _run_grep(root, out_dir, args.reviewer_facing_only)

    if args.archive:
        args.archive = args.archive.resolve()
        _write_tar_gz(out_dir, args.archive)
        digest = _sha256_file(args.archive)
        print(
            json.dumps(
                {
                    "archive": str(args.archive),
                    "sha256_provisional": digest,
                    "note": "Record final SHA only in build/RELEASE_MANIFEST.md after verify; "
                    "do not embed the tarball hash inside the archive.",
                },
                indent=2,
                sort_keys=True,
            ),
            file=sys.stderr,
        )
        rel_arc = args.archive.relative_to(root)
        print(
            "Next:\n"
            f"  python ci/scripts/verify_anonymous_bundle_scrub.py {rel_arc}\n"
            f"  sha256sum {rel_arc.as_posix()} > build/ANON_BUNDLE_SHA256.txt\n"
            "  python ci/scripts/write_release_manifest.py "
            f"--archive {rel_arc}",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
