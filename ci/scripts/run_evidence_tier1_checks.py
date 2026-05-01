#!/usr/bin/env python3
"""Single entrypoint for the ``evidence-tranche-scripts`` job in ``ci-tier1-fast``.

Runs, in order: ``compileall`` on ``ci/scripts``, structural ``rust-proof`` gate
on tracked ``runs/released/rust_proof_subset_v1/results_fast``, Verified
preflight on ``runs/released/l0l1_pass_hunt_v1/panel.jsonl``, and the full
Verified manifest audit (500 manifests). It does **not** run Rust
fmt/clippy/tests.

Execute from the repository root::

    python ci/scripts/run_evidence_tier1_checks.py

Exit code: first failing subprocess return code, or 0 if all succeed.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _run(argv: list[str], *, cwd: Path) -> int:
    proc = subprocess.run(
        [sys.executable, *argv],
        cwd=cwd,
        check=False,
    )
    return int(proc.returncode)


def main() -> int:
    root = _repo_root()
    steps: list[tuple[str, list[str]]] = [
        ("compileall ci/scripts", ["-m", "compileall", "-q", "ci/scripts"]),
        (
            "paper claim sources",
            [
                str(root / "ci/scripts/check_paper_claim_sources.py"),
            ],
        ),
        (
            "release secret scan",
            [
                str(root / "ci/scripts/secret_scan_release.py"),
                "--repo-root",
                str(root),
            ],
        ),
        (
            "structural rust-proof gate",
            [
                str(root / "ci/scripts/check_evidence_quality.py"),
                "rust-proof",
                "--run-dir",
                str(root / "runs/released/rust_proof_subset_v1/results_fast"),
                "--expected-entries",
                "8",
                "--min-l3-pass-l4-fail",
                "0",
                "--min-all-level-pass",
                "0",
            ],
        ),
        (
            "preflight Verified selectors (l0l1_pass_hunt_v1)",
            [
                str(root / "ci/scripts/preflight_verified_selectors.py"),
                "--panel",
                str(root / "runs/released/l0l1_pass_hunt_v1/panel.jsonl"),
                "--strict",
                "--min-tasks",
                "8",
            ],
        ),
        (
            "audit Verified manifest entrypoints",
            [
                str(root / "ci/scripts/audit_verified_manifest_entrypoints.py"),
                "--strict",
                "--expect-manifest-count",
                "500",
            ],
        ),
    ]

    for label, argv in steps:
        code = _run(argv, cwd=root)
        if code != 0:
            print(f"run_evidence_tier1_checks: FAILED step: {label} (exit {code})", file=sys.stderr)
            return code
    print("run_evidence_tier1_checks: all steps passed", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
