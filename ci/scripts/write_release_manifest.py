#!/usr/bin/env python3
"""Write ``build/RELEASE_MANIFEST.md`` **outside** the anonymous tarball.

Run **after** the final ``.tar.gz`` is built, ``verify_anonymous_bundle_scrub.py``
passes, and ``sha256sum`` (or ``Get-FileHash``) is written to
``build/ANON_BUNDLE_SHA256.txt``. This file records the public hash without
embedding it inside the archive (which would change the hash).

Example::

    python ci/scripts/verify_anonymous_bundle_scrub.py build/eval-ladder-anonymous-bundle.tar.gz
    sha256sum build/eval-ladder-anonymous-bundle.tar.gz > build/ANON_BUNDLE_SHA256.txt
    python ci/scripts/write_release_manifest.py --archive build/eval-ladder-anonymous-bundle.tar.gz
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def _read_sha256_file(path: Path) -> str | None:
    if not path.is_file():
        return None
    for ln in path.read_text(encoding="utf-8").splitlines():
        ln = ln.strip()
        if not ln:
            continue
        for tok in ln.split():
            if len(tok) == 64 and all(c in "0123456789abcdef" for c in tok.lower()):
                return tok.lower()
    return None


def _run(cmd: list[str], *, cwd: Path) -> int:
    return int(
        subprocess.run(cmd, cwd=cwd, check=False).returncode,
    )


def main() -> int:
    root = _repo_root()
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--archive",
        type=Path,
        required=True,
        help="Path to the final anonymous .tar.gz (under build/ recommended).",
    )
    p.add_argument(
        "--out",
        type=Path,
        default=root / "build" / "RELEASE_MANIFEST.md",
        help="Output manifest path (default: build/RELEASE_MANIFEST.md).",
    )
    p.add_argument(
        "--skip-tier1",
        action="store_true",
        help="Do not run run_evidence_tier1_checks.py (record as SKIPPED).",
    )
    p.add_argument(
        "--skip-verify",
        action="store_true",
        help="Do not run verify_anonymous_bundle_scrub.py (record as SKIPPED).",
    )
    args = p.parse_args()
    archive = args.archive.resolve()
    if not archive.is_file():
        print(f"write_release_manifest: missing archive {archive}", file=sys.stderr)
        return 1

    sha_path = root / "build" / "ANON_BUNDLE_SHA256.txt"
    digest = _read_sha256_file(sha_path) or _sha256_file(archive)
    if not _read_sha256_file(sha_path):
        print(
            "write_release_manifest: warning: build/ANON_BUNDLE_SHA256.txt missing or "
            "unparseable; using freshly computed digest (prefer sha256sum redirect first).",
            file=sys.stderr,
        )

    verify_cmd = [
        sys.executable,
        str(root / "ci/scripts/verify_anonymous_bundle_scrub.py"),
        str(archive),
    ]
    tier1_source_cmd = [sys.executable, str(root / "ci/scripts/run_evidence_tier1_checks.py")]
    staged_root = root / "build" / "eval-ladder-anon-stage"
    tier1_staged_cmd = [
        sys.executable,
        str(root / "ci/scripts/run_evidence_tier1_checks.py"),
        "--staged-root",
        str(staged_root),
    ]

    scrub_rc = 0 if args.skip_verify else _run(verify_cmd, cwd=root)
    tier1_source_rc = 0 if args.skip_tier1 else _run(tier1_source_cmd, cwd=root)
    tier1_staged_rc: int | None = None
    if args.skip_tier1:
        tier1_staged_status = "SKIPPED"
    elif not (staged_root / "ci" / "scripts").is_dir():
        tier1_staged_status = "SKIPPED (no staged anonymous tree)"
    else:
        tier1_staged_rc = _run(tier1_staged_cmd, cwd=root)
        tier1_staged_status = "PASS" if tier1_staged_rc == 0 else "FAIL"

    scrub_status = "SKIPPED" if args.skip_verify else ("PASS" if scrub_rc == 0 else "FAIL")
    tier1_source_status = "SKIPPED" if args.skip_tier1 else ("PASS" if tier1_source_rc == 0 else "FAIL")

    py_ver = (
        subprocess.run(
            [sys.executable, "-c", "import sys; print(sys.version.split()[0])"],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    )
    try:
        rust_ver = subprocess.run(
            ["rustc", "--version"],
            cwd=root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        rust_ver = "(rustc not on PATH)"

    rel_arc = archive.relative_to(root) if archive.is_relative_to(root) else archive

    body = f"""# Release manifest (external to anonymous tarball)

This file is **not** included in the anonymous submission archive. The final
SHA-256 of the tarball must not be embedded inside the archive (it would change
the digest).

- **Archive:** `{rel_arc.as_posix()}`
- **SHA-256:** `{digest}`
- **Scrub command:** `python ci/scripts/verify_anonymous_bundle_scrub.py {rel_arc.as_posix()}`
- **Scrub status:** {scrub_status}
- **Tier-1 source-tree evidence checks:** {tier1_source_status}
- **Command:** `python ci/scripts/run_evidence_tier1_checks.py`
- **Tier-1 staged anonymous-tree evidence checks:** {tier1_staged_status}
- **Command:** `python ci/scripts/run_evidence_tier1_checks.py --staged-root build/eval-ladder-anon-stage`
- **Date/time (UTC):** {datetime.now(timezone.utc).isoformat()}
- **Python version:** {py_ver}
- **Rust version:** {rust_ver}
"""
    out = args.out.resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(body, encoding="utf-8")
    print(f"write_release_manifest: wrote {out.relative_to(root)}", file=sys.stderr)
    tier1_fail = (not args.skip_tier1 and tier1_source_rc != 0) or (
        tier1_staged_rc is not None and tier1_staged_rc != 0
    )
    if scrub_rc != 0 or tier1_fail:
        print(
            "write_release_manifest: one or more checks failed; fix before submission.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
