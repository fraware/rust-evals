#!/usr/bin/env python3
"""Merge several L2 `batch_summary.json` files into one deduped summary.

Used to form a single canonical run directory for
`check_evidence_quality l2` when multiple small slices were evaluated
separately but share disjoint `entry_id` space.

Writes `batch_summary.json` under ``--out-dir``. Bundle directories are not
copied; use this merge only for summary-level gates and documentation.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, cast


def _load_summary(path: Path) -> dict[str, Any]:
    return cast(dict[str, Any], json.loads(path.read_text(encoding="utf-8")))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--inputs",
        type=Path,
        nargs="+",
        required=True,
        help="paths to batch_summary.json files to merge",
    )
    ap.add_argument(
        "--out-dir",
        type=Path,
        required=True,
        help="directory to write merged batch_summary.json",
    )
    args = ap.parse_args()

    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    seen: set[str] = set()
    merged: list[dict[str, Any]] = []
    evaluator_version = "0.1.0"
    schema_version = 1
    levels: list[str] | None = None

    for path in args.inputs:
        p = path.resolve()
        data = _load_summary(p)
        entries = data.get("entries", [])
        if not isinstance(entries, list):
            raise SystemExit(f"{p}: entries must be a list")
        ev = data.get("evaluator_version")
        if isinstance(ev, str):
            evaluator_version = ev
        sch = data.get("schema_version")
        if isinstance(sch, int):
            schema_version = sch
        lev = data.get("levels")
        if isinstance(lev, list) and lev and levels is None:
            levels = [str(x) for x in lev]
        for e in entries:
            if not isinstance(e, dict):
                continue
            eid = str(e.get("entry_id", ""))
            if not eid or eid in seen:
                continue
            seen.add(eid)
            merged.append(e)

    summary = {
        "entries": merged,
        "evaluator_version": evaluator_version,
        "invalid_entries": 0,
        "levels": levels or ["L0", "L1", "L2"],
        "ok_entries": len(merged),
        "schema_version": schema_version,
        "total_entries": len(merged),
    }
    (out_dir / "batch_summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps({"written": str(out_dir / "batch_summary.json"), "entries": len(merged)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
