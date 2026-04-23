#!/usr/bin/env python3
"""Production L4 obligation driver.

Runs `lake env lean <lean_file>` for a single obligation module and
emits a single `LeanCheckOutcome` JSON document on stdout. The
eval-ladder runner's `ExternalProcessChecker` is happy with any
command that follows this protocol; the script exists so every
obligation entry in `datasets/derived/proof_subset/manifest.jsonl`
can share a single, well-tested wrapper instead of re-inventing the
JSON contract.

Contract (see `packages/rust/lean/src/checker.rs`):

* cwd at invocation is `lean_root`
  (`packages/lean/EvalLadder` in this repository).
* stdout MUST contain exactly one JSON object of shape
  `{ "status": "valid"|"invalid"|"not_applicable",
     "code":   "<UPPER_SNAKE_CODE>",
     "message": "<free form>",
     "payload": { ... } }`.
* Non-zero exit codes are tolerated by the runner as long as stdout
  contains a parseable outcome. This script always exits 0 once it
  has produced a well-formed JSON document so downstream reviewers
  can distinguish "Lean said no" from "driver crashed".

Invocation:

    python scripts/check_obligation.py <LEAN_FILE> <PASS_CODE> [--lake-bin <PATH>]

Arguments:
    LEAN_FILE   Path to the `.lean` module containing the obligation
                proof. Resolved relative to cwd (i.e. `lean_root`).
    PASS_CODE   Stable upper-snake-case code emitted on success;
                must match the obligation's `pass_criterion`.

Flags:
    --lake-bin PATH
                Override the `lake` executable. Defaults to `lake` on
                PATH, which requires the elan-installed Lean toolchain
                pinned by `packages/lean/EvalLadder/lean-toolchain`.

The driver is intentionally side-effect free: it never writes files,
never mutates the workspace, and never consumes stdin. The only
observable output is the JSON document on stdout.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


# Codes mirror `eval_ladder_core::FailureReason` / `eval-ladder-lean`.
# Keep in sync with `docs/proof_subset_policy.md`.
CODE_OBLIGATION_UNMET = "L4_OBLIGATION_UNMET"
CODE_EXTRACTION_FAILED = "L4_EXTRACTION_FAILED"

# Cap the stderr we embed in the JSON payload so a single long
# compilation error cannot blow the runner's log budget. The runner
# itself also enforces a budget on the stderr it captures from the
# checker process; this is a defense in depth.
STDERR_EMBED_LIMIT = 4096


def emit(outcome: dict) -> None:
    """Print `outcome` as a single line of canonical JSON on stdout."""
    json.dump(outcome, sys.stdout, separators=(",", ":"), sort_keys=True)
    sys.stdout.write("\n")
    sys.stdout.flush()


def truncate(text: str, limit: int = STDERR_EMBED_LIMIT) -> str:
    if len(text) <= limit:
        return text
    return text[:limit] + "...<truncated>"


def parse_args(argv: list[str]) -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("lean_file", help="Path to the obligation .lean module.")
    p.add_argument("pass_code", help="Pass criterion code (for example L4_OBLIGATION_MET).")
    p.add_argument(
        "--lake-bin",
        default="lake",
        help="Override the lake executable; defaults to `lake` on PATH.",
    )
    return p.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)

    lean_file = Path(args.lean_file)
    if not lean_file.is_file():
        emit(
            {
                "status": "not_applicable",
                "code": CODE_EXTRACTION_FAILED,
                "message": f"obligation module not found: {lean_file}",
                "payload": {"lean_file": str(lean_file)},
            }
        )
        return 0

    if shutil.which(args.lake_bin) is None:
        emit(
            {
                "status": "not_applicable",
                "code": CODE_EXTRACTION_FAILED,
                "message": (
                    f"lake executable `{args.lake_bin}` not found on PATH; "
                    "install elan and run `elan toolchain install $(cat lean-toolchain)`."
                ),
                "payload": {"lake_bin": args.lake_bin},
            }
        )
        return 0

    try:
        completed = subprocess.run(
            [args.lake_bin, "env", "lean", str(lean_file)],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as exc:
        emit(
            {
                "status": "not_applicable",
                "code": CODE_EXTRACTION_FAILED,
                "message": f"failed to spawn lake: {exc}",
                "payload": {"lake_bin": args.lake_bin},
            }
        )
        return 0

    stdout = completed.stdout or ""
    stderr = completed.stderr or ""

    if completed.returncode == 0 and "error:" not in stderr.lower():
        emit(
            {
                "status": "valid",
                "code": args.pass_code,
                "message": f"lake env lean accepted {lean_file}",
                "payload": {
                    "lean_file": str(lean_file),
                    "stdout_bytes": len(stdout),
                    "stderr_bytes": len(stderr),
                },
            }
        )
        return 0

    emit(
        {
            "status": "invalid",
            "code": CODE_OBLIGATION_UNMET,
            "message": f"lake env lean rejected {lean_file} (exit {completed.returncode})",
            "payload": {
                "lean_file": str(lean_file),
                "exit_code": completed.returncode,
                "stderr": truncate(stderr),
                "stdout": truncate(stdout),
            },
        }
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
