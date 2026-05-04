#!/usr/bin/env python3
"""Validate claim_limits.json against paper_claim_sources.json."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def main() -> int:
    root = _repo_root()
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--limits",
        type=Path,
        default=root / "paper" / "exports" / "claim_limits.json",
    )
    p.add_argument(
        "--map",
        type=Path,
        default=root / "docs" / "paper_claim_sources.json",
    )
    args = p.parse_args()
    limits_path = args.limits.resolve()
    map_path = args.map.resolve()
    failures: list[str] = []

    if not limits_path.is_file():
        failures.append(f"missing {limits_path}")
        _emit(failures)
        return 1
    if not map_path.is_file():
        failures.append(f"missing {map_path}")
        _emit(failures)
        return 1

    raw_limits = _load_json(limits_path)
    if not isinstance(raw_limits, list):
        failures.append("claim_limits.json top-level must be an array")
        _emit(failures)
        return 1

    limits_by_id: dict[str, dict[str, Any]] = {}
    for entry in raw_limits:
        if not isinstance(entry, dict):
            failures.append("claim_limits.json entries must be objects")
            continue
        cid = entry.get("claim_id")
        if not isinstance(cid, str) or not cid:
            failures.append("claim_limits entry missing claim_id")
            continue
        limits_by_id[cid] = entry

    cfg = cast(dict[str, Any], _load_json(map_path))
    claims = cfg.get("claims", {})
    if not isinstance(claims, dict):
        failures.append("paper_claim_sources.json claims must be an object")
        _emit(failures)
        return 1

    for name, spec in claims.items():
        if not isinstance(spec, dict):
            failures.append(f"claim {name}: spec must be object")
            continue
        if not spec.get("required"):
            continue
        cid = spec.get("claim_limits_id")
        if not isinstance(cid, str) or not cid:
            failures.append(f"claim {name}: missing claim_limits_id")
            continue
        lim = limits_by_id.get(cid)
        if lim is None:
            failures.append(f"claim {name}: unknown claim_limits_id {cid!r}")
            continue

        tier = str(spec.get("claim_tier", ""))
        status = str(lim.get("status", ""))

        if tier == "central":
            allowed = str(lim.get("allowed", "")).strip()
            if not allowed:
                failures.append(
                    f"claim_limits {cid}: central entry needs non-empty allowed"
                )
            na = lim.get("not_allowed")
            if not isinstance(na, list) or not na:
                failures.append(
                    f"claim_limits {cid}: central entry needs not_allowed list"
                )
            elif not all(str(x).strip() for x in na):
                failures.append(
                    f"claim_limits {cid}: central entry needs non-empty not_allowed strings"
                )
            if status == "evidence_frontier":
                failures.append(
                    f"claim {name}: central claim must not use frontier-only "
                    f"limits status ({status!r})"
                )

        if tier == "frontier":
            if status == "central_diagnostic":
                failures.append(
                    f"claim {name}: frontier claim must not use "
                    f"central_diagnostic limits status"
                )

        if status == "central_diagnostic" and tier != "central":
            failures.append(
                f"claim {name}: limits status central_diagnostic requires "
                f"claim_tier central"
            )

    _emit(failures)
    return 1 if failures else 0


def _emit(failures: list[str]) -> None:
    if failures:
        for line in failures:
            print(f"check_claim_limits: FAIL: {line}", file=sys.stderr)
    else:
        print("check_claim_limits: OK", file=sys.stderr)


if __name__ == "__main__":
    raise SystemExit(main())
