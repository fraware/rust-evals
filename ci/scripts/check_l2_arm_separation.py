#!/usr/bin/env python3
"""Fail if L2 paper exports lack explicit ``validator_arm`` separation."""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path
from typing import Any, cast


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load_claim_sources(root: Path) -> dict[str, Any]:
    path = root / "docs" / "paper_claim_sources.json"
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError("paper_claim_sources.json must be an object")
    return cast(dict[str, Any], data)


def main() -> int:
    root = _repo_root()
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--export-dir",
        type=Path,
        default=root / "paper" / "exports" / "l2_verified_flagship_v1",
    )
    args = p.parse_args()
    export_dir = args.export_dir.resolve()
    failures: list[str] = []

    csv_path = export_dir / "l2_arm_breakdown.csv"
    if not csv_path.is_file():
        failures.append(f"missing {csv_path}")
        _emit(failures)
        return 1

    with csv_path.open(encoding="utf-8", newline="") as h:
        rows = list(csv.DictReader(h))
    if not rows:
        failures.append("l2_arm_breakdown.csv is empty")
    header = rows[0].keys() if rows else []
    if "validator_arm" not in header:
        failures.append("l2_arm_breakdown.csv missing validator_arm column")

    arms = {str(r.get("validator_arm", "")).strip() for r in rows}
    for need in ("augmented_tests", "regression_stress_control", "total"):
        if need not in arms:
            failures.append(
                "l2_arm_breakdown.csv missing validator_arm="
                f"{need!r} row"
            )

    total_row = next(
        (r for r in rows if str(r.get("validator_arm")) == "total"),
        None,
    )
    if total_row:
        for col in ("allowed_claim", "disallowed_claim"):
            if not str(total_row.get(col, "")).strip():
                failures.append(f"total row missing non-empty {col}")

    cfg = _load_claim_sources(root)
    claims = cfg.get("claims", {})
    arm = claims.get("l2_flagship_arm_breakdown", {})
    src = str(arm.get("source", "")).replace("\\", "/")
    if not src.endswith("l2_arm_breakdown.csv"):
        failures.append(
            "paper_claim_sources.json l2_flagship_arm_breakdown must cite "
            f"l2_arm_breakdown.csv (found {src!r})"
        )

    _emit(failures)
    return 1 if failures else 0


def _emit(failures: list[str]) -> None:
    if failures:
        for line in failures:
            print(f"check_l2_arm_separation: FAIL: {line}", file=sys.stderr)
    else:
        print("check_l2_arm_separation: OK", file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
