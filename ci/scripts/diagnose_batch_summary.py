#!/usr/bin/env python3
"""Diagnose dominant failure modes in a batch_summary.json.

Use this before expensive reruns to identify harness or policy pathologies
(for example L1_HARNESS_ERROR dominance or single-code L3 collapse). With
``--fail-on-warnings``, exit code is 2 when any warning is emitted (default is
always 0 so the report can be inspected interactively).
"""

from __future__ import annotations

import argparse
import json
from collections import Counter
from pathlib import Path
from typing import Any, cast


def _load(path: Path) -> dict[str, Any]:
    return cast(dict[str, Any], json.loads(path.read_text(encoding="utf-8")))


def _code(entry: dict[str, Any], level: str) -> str:
    levels = entry.get("levels", {})
    if not isinstance(levels, dict):
        return "MISSING"
    lvl = levels.get(level, {})
    if not isinstance(lvl, dict):
        return "MISSING"
    return str(lvl.get("primary_reason", "MISSING"))


def _status(entry: dict[str, Any], level: str) -> str:
    levels = entry.get("levels", {})
    if not isinstance(levels, dict):
        return "missing"
    lvl = levels.get(level, {})
    if not isinstance(lvl, dict):
        return "missing"
    return str(lvl.get("status", "missing")).lower()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--summary", type=Path, required=True, help="path to batch_summary.json")
    ap.add_argument("--l1-harness-threshold", type=float, default=0.10)
    ap.add_argument("--l3-dominant-threshold", type=float, default=0.80)
    ap.add_argument(
        "--fail-on-warnings",
        action="store_true",
        help="exit with code 2 when any diagnostic warning is emitted",
    )
    args = ap.parse_args()

    summary = _load(args.summary)
    entries = summary.get("entries", [])
    if not isinstance(entries, list):
        raise SystemExit("invalid summary format: entries is not a list")
    total = len(entries)
    if total == 0:
        raise SystemExit("summary has zero entries")

    l0_codes = Counter(_code(e, "l0") for e in entries)
    l1_codes = Counter(_code(e, "l1") for e in entries)
    l3_codes = Counter(_code(e, "l3") for e in entries if _status(e, "l3") in {"fail", "invalid"})

    l1_harness = l1_codes.get("L1_HARNESS_ERROR", 0)
    l1_harness_rate = l1_harness / total
    l3_dom = (max(l3_codes.values()) / sum(l3_codes.values())) if l3_codes else 0.0

    warnings: list[str] = []
    report: dict[str, Any] = {
        "total_entries": total,
        "l0_codes": dict(l0_codes),
        "l1_codes": dict(l1_codes),
        "l3_fail_codes": dict(l3_codes),
        "metrics": {
            "l1_harness_error_rate": l1_harness_rate,
            "l3_dominant_fail_code_share": l3_dom,
        },
        "warnings": warnings,
    }
    if l1_harness_rate > args.l1_harness_threshold:
        warnings.append(
            "L1_HARNESS_ERROR rate "
            f"{l1_harness_rate:.3f} exceeds threshold "
            f"{args.l1_harness_threshold:.3f}"
        )
    if l3_dom > args.l3_dominant_threshold:
        warnings.append(
            "L3 dominant fail-code share "
            f"{l3_dom:.3f} exceeds threshold "
            f"{args.l3_dominant_threshold:.3f}"
        )

    print(json.dumps(report, indent=2, sort_keys=True))
    if args.fail_on_warnings and warnings:
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
