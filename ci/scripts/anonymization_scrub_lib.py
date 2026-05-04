"""Shared anonymization patterns, staged-tree sanitization, and scrub verification."""

from __future__ import annotations

import re
from pathlib import Path

_TEXT_SUFFIXES = frozenset(
    {
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
)

_SKIP_DIR_NAMES = frozenset({".git", ".github", "__pycache__", ".lake"})
_SKIP_FILE_NAMES = frozenset({".DS_Store", "Thumbs.db"})


def iter_text_files_under(root: Path) -> list[Path]:
    out: list[Path] = []
    for p in root.rglob("*"):
        if not p.is_file():
            continue
        if p.suffix not in _TEXT_SUFFIXES and p.name != ".gitignore":
            continue
        if set(p.parts) & _SKIP_DIR_NAMES:
            continue
        if p.name in _SKIP_FILE_NAMES:
            continue
        out.append(p)
    return sorted(out)


def _under_third_party_workspace(rel: Path) -> bool:
    return "workspaces" in rel.parts


def scrub_exempt(rel: Path) -> bool:
    """Paths where third-party or checker-source text must not fail the scrub."""
    parts = rel.parts
    if "workspaces" in parts:
        return True
    if len(parts) >= 3 and parts[0:3] == ("benchmarks", "verified", "manifests"):
        return True
    if rel.as_posix() in {
        "ci/scripts/anonymization_scrub_lib.py",
        "ci/scripts/build_anonymous_submission_bundle.py",
        "ci/scripts/verify_anonymous_bundle_scrub.py",
    }:
        return True
    return False


def identity_pattern() -> re.Pattern[str]:
    return re.compile(
        r"fraware|Mateo|Petel|mpetel|stanford|github\.com/fraware|rust-evals",
        re.I,
    )


def path_pattern() -> re.Pattern[str]:
    return re.compile(
        r"/Users/|/home/[^/\s]+|C:\\\\Users|/mnt/data|/tmp/.*mateo|D:\\\\a\\\\",
        re.I,
    )


def email_pattern() -> re.Pattern[str]:
    return re.compile(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")


def url_like_pattern() -> re.Pattern[str]:
    """Broad ``http(s)://`` occurrences (workspace audit only)."""
    return re.compile(r"https?://", re.I)


def social_pattern() -> re.Pattern[str]:
    """Narrow list to avoid matching scrub docs and benchmark issue text."""
    return re.compile(r"linkedin\.com|twitter\.com|x\.com/", re.I)


def count_workspace_scrub_hits(root: Path) -> dict[str, int]:
    """Count regex matches under ``**/workspaces/**`` only (audit; not violations)."""
    url_re = url_like_pattern()
    id_re = identity_pattern()
    path_re = path_pattern()
    email_re = email_pattern()
    totals = {"url": 0, "identity": 0, "paths": 0, "email": 0}
    root = root.resolve()
    for fp in iter_text_files_under(root):
        rel = fp.relative_to(root)
        if not _under_third_party_workspace(rel):
            continue
        try:
            text = fp.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        totals["url"] += len(url_re.findall(text))
        totals["identity"] += len(id_re.findall(text))
        totals["paths"] += len(path_re.findall(text))
        totals["email"] += len(email_re.findall(text))
    return totals


def collect_violations(root: Path, *, max_reports: int = 400) -> list[str]:
    """Return human-readable violation lines (empty list means scrub OK)."""
    failures: list[str] = []
    failures.extend(f"forbidden path present: {h}" for h in check_forbidden_dirs(root))

    patterns: list[tuple[str, re.Pattern[str]]] = [
        ("identity_leak", identity_pattern()),
        ("paths", path_pattern()),
        ("email", email_pattern()),
        ("social", social_pattern()),
    ]

    for label, rx in patterns:
        for fp in iter_text_files_under(root):
            rel = fp.relative_to(root)
            if scrub_exempt(rel):
                continue
            try:
                text = fp.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            for i, line in enumerate(text.splitlines(), 1):
                if not rx.search(line):
                    continue
                failures.append(f"{label}: {rel.as_posix()}:{i}:{line.strip()[:240]}")
                if len(failures) >= max_reports:
                    failures.append("... truncated ...")
                    return failures
    return failures


def check_forbidden_dirs(root: Path) -> list[str]:
    bad = (".git", ".github", "__pycache__")
    hits: list[str] = []
    for name in bad:
        for p in root.rglob(name):
            if p.is_dir() and p.name == name:
                hits.append(str(p.relative_to(root)))
    for p in root.rglob(".DS_Store"):
        if p.is_file():
            hits.append(str(p.relative_to(root)))
    return hits


def sanitize_text(text: str) -> str:
    """Best-effort redaction for one file body."""
    out = text
    out = re.sub(
        r"https?://github\.com/fraware[^\s\"'`<>)\]]+",
        "https://anonymous.invalid/",
        out,
        flags=re.I,
    )
    out = re.sub(
        r"https?://api\.github\.com/repos/fraware[^\s\"'`<>)\]]+",
        "https://anonymous.invalid/api-placeholder",
        out,
        flags=re.I,
    )
    out = re.sub(
        r"github\.com/fraware[^\s\"'`<>)\]]+",
        "anonymous.invalid/",
        out,
        flags=re.I,
    )
    out = re.sub(r"(?i)\bfraware\b", "anon-org", out)
    out = re.sub(r"(?i)\brust-evals\b", "eval-ladder-artifact", out)
    out = re.sub(r"(?i)\bmateo\b", "redacted-user", out)
    out = re.sub(r"(?i)\bpetel\b", "redacted-name", out)
    out = re.sub(r"(?i)\bmpetel\b", "redacted-handle", out)
    out = re.sub(r"(?i)\bstanford\b", "redacted-institution", out)
    out = re.sub(
        r"/home/runner/work/[^\s\"'`\])>]+",
        "<artifact-root>",
        out,
        flags=re.I,
    )
    out = re.sub(r"/mnt/data[^\s\"'`\])>]*", "<artifact-root>", out, flags=re.I)
    out = re.sub(
        r"/tmp/[^\s\"'`\])>]*mateo[^\s\"'`\])>]*",
        "<artifact-root>/tmp/",
        out,
        flags=re.I,
    )
    out = email_pattern().sub("<redacted-email>", out)
    # Absolute paths (common in sealed JSON and logs). JSON often doubles ``\``.
    prev = None
    while prev != out:
        prev = out
        out = re.sub(r"C:\\\\Users\\\\[^\"]+", "<artifact-root>", out, flags=re.I)
        out = re.sub(r"C:\\Users\\[^\"]+", "<artifact-root>", out, flags=re.I)
    out = re.sub(r"/Users/[^\"]+", "<artifact-root>", out)
    out = re.sub(r"D:\\a\\[^\"]+", "<artifact-root>", out, flags=re.I)
    return out


def _anonymize_root_cargo_toml(text: str) -> str:
    out = text
    out = re.sub(
        r"(?m)^\s*repository\s*=\s*\"[^\"]*\"\s*$",
        'repository = "https://anonymous.invalid/eval-ladder"',
        out,
    )
    out = re.sub(
        r"(?m)^\s*homepage\s*=\s*\"[^\"]*\"\s*$",
        'homepage = "https://anonymous.invalid/eval-ladder"',
        out,
    )
    out = re.sub(r"(?m)^\s*documentation\s*=\s*\"[^\"]*\"\s*\n", "", out)
    out = re.sub(
        r"(?m)^\s*authors\s*=\s*\[[^\]]*\]\s*$",
        'authors = ["Anonymous Authors"]',
        out,
    )
    return out


def _anonymize_root_pyproject_toml(text: str) -> str:
    out = text
    out = re.sub(
        r"(?m)^authors\s*=\s*\[[^\]]*\]\s*$",
        'authors = [{ name = "Anonymous Authors" }]',
        out,
    )
    return out


def sanitize_staged_tree(root: Path) -> int:
    """Rewrite text files under ``root`` in place. Returns number of files changed."""
    root = root.resolve()
    changed = 0
    for fp in iter_text_files_under(root):
        rel = fp.relative_to(root)
        if scrub_exempt(rel):
            continue
        try:
            raw = fp.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        new = sanitize_text(raw)
        if fp.parent.resolve() == root:
            if fp.name == "Cargo.toml":
                new = _anonymize_root_cargo_toml(new)
            elif fp.name == "pyproject.toml":
                new = _anonymize_root_pyproject_toml(new)
        if new != raw:
            fp.write_text(new, encoding="utf-8")
            changed += 1
    return changed
