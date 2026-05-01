#!/usr/bin/env python3
"""Wrapper for L2 flagship gold/developer patch validation.

Delegates to ``ci/scripts/l2_flagship_gold_patch_validation.py`` so callers can
run from repo root:

  python packages/python/scripts/run_l2_gold_patch_validation.py --help

Outputs (see CI script):

- ``paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv``
- ``paper/exports/l2_verified_flagship_v1/gold_patch_validation.json``
- ``runs/released/l2_verified_flagship_v1/gold_patch_results/``

Default strengthening profile is ``gold_mechanical`` (see ``docs/l2_gold_patch_validation.md``).
Pass ``--strict-flagship-specs`` for agent-matched diagnostics only.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    ci_script = repo_root / "ci" / "scripts" / "l2_flagship_gold_patch_validation.py"
    if not ci_script.is_file():
        raise SystemExit(f"missing {ci_script}")
    forwarded = sys.argv[1:]
    cmd = [sys.executable, str(ci_script), *forwarded]
    return subprocess.call(cmd, cwd=repo_root)


if __name__ == "__main__":
    raise SystemExit(main())
