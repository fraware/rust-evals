#!/usr/bin/env python3
"""Single entrypoint for the ``evidence-tranche-scripts`` job in ``ci-tier1-fast``.

Runs, in order: ``compileall`` on ``ci/scripts``, structural ``rust-proof`` gate
on tracked ``runs/released/rust_proof_subset_v1/results_fast``, Verified
preflight on ``runs/released/l0l1_pass_hunt_v1/panel.jsonl``, and the full
Verified manifest audit (500 manifests). It does **not** run Rust
fmt/clippy/tests.

Execute from the repository root::

    python ci/scripts/run_evidence_tier1_checks.py

After building the anonymous tree, re-run against the **staged** artifact (same
checks; scripts resolve paths from the staged copy)::

    python ci/scripts/run_evidence_tier1_checks.py --staged-root build/eval-ladder-anon-stage

Exit code: first failing subprocess return code, or 0 if all succeed.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def _default_repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _run(argv: list[str], *, cwd: Path) -> int:
    proc = subprocess.run(
        [sys.executable, *argv],
        cwd=cwd,
        check=False,
    )
    return int(proc.returncode)


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--staged-root",
        type=Path,
        default=None,
        help="Repository root of the staged anonymous tree (e.g. build/eval-ladder-anon-stage).",
    )
    args = p.parse_args()
    root = args.staged_root.resolve() if args.staged_root else _default_repo_root()
    if not (root / "ci" / "scripts").is_dir():
        print(
            f"run_evidence_tier1_checks: missing {root / 'ci' / 'scripts'} "
            "(pass a complete staged tree)",
            file=sys.stderr,
        )
        return 2

    steps: list[tuple[str, list[str]]] = [
        ("compileall ci/scripts", ["-m", "compileall", "-q", "ci/scripts"]),
        (
            "paper claim sources",
            [
                str(root / "ci/scripts/check_paper_claim_sources.py"),
            ],
        ),
        (
            "claim limits",
            [
                str(root / "ci/scripts/check_claim_limits.py"),
            ],
        ),
        (
            "L2 arm separation",
            [
                str(root / "ci/scripts/check_l2_arm_separation.py"),
                "--export-dir",
                str(root / "paper" / "exports" / "l2_verified_flagship_v1"),
            ],
        ),
        (
            "reviewer-facing reversal wording",
            [
                str(root / "ci/scripts/check_reviewer_false_success_language.py"),
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
