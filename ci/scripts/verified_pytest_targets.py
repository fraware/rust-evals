"""Shared helpers to parse pytest file targets from SWE-style entrypoints."""

from __future__ import annotations

import re


def is_explicit_pytest_path(rel: str) -> bool:
    """True when the selector names a concrete file path (not bare ``test_*``)."""
    return "/" in rel or rel.endswith(".py")


def pytest_file_paths(entrypoint: str) -> list[str]:
    """Return repo-relative pytest file path stems (no ``::`` node suffix)."""
    ep = entrypoint.strip()
    lowered = ep.lower()
    if "pytest" not in lowered:
        return []
    for prefix in ("python -m pytest ", "python3 -m pytest ", "pytest "):
        if lowered.startswith(prefix):
            ep = ep[len(prefix) :].lstrip()
            break
    else:
        return []

    paths: list[str] = []
    for tok in ep.split():
        if tok.startswith("-"):
            continue
        if "=" in tok and not tok.endswith(".py"):
            continue
        if "::" in tok or tok.endswith(".py") or "/" in tok:
            file_part = tok.split("::", 1)[0].strip()
            if file_part and not file_part.startswith("-"):
                paths.append(file_part.replace("\\", "/"))
        elif re.match(r"^test_[A-Za-z0-9_]+$", tok):
            paths.append(tok)
    seen: set[str] = set()
    out: list[str] = []
    for p in paths:
        if p not in seen:
            seen.add(p)
            out.append(p)
    return out
