#!/usr/bin/env python3
"""Regenerate ``live_leave_one_out.csv`` from the Live v2 paper export slice.

Requires ``per_task_live_outcomes.csv`` in the paper export directory (produced
by ``packages/python/scripts/export_live_panel_tables.py``). Recomputes
leave-one-out rates and summary rows to match that pipeline.
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _write_loo_from_per_task(per_task_path: Path, out_path: Path) -> dict[str, Any]:
    with per_task_path.open(encoding="utf-8", newline="") as h:
        reader = csv.DictReader(h)
        per_task = list(reader)

    static_rows = [r for r in per_task if r.get("benchmark_surface") == "static_anchor"]
    live_rows = [r for r in per_task if r.get("benchmark_surface") == "live"]

    static_rates: dict[str, float] = {}
    by_agent_static: dict[str, list[dict[str, str]]] = defaultdict(list)
    for r in static_rows:
        by_agent_static[r["agent_id"]].append(r)
    by_agent_live: dict[str, list[dict[str, str]]] = defaultdict(list)
    for r in live_rows:
        by_agent_live[r["agent_id"]].append(r)

    for aid, sr in by_agent_static.items():
        n = len(sr)
        k = sum(1 for x in sr if x.get("status_L1") == "pass")
        static_rates[aid] = k / n if n else 0.0

    loo_rows: list[dict[str, Any]] = []
    for agent_id, agent_live in sorted(by_agent_live.items()):
        static_p = static_rates.get(agent_id, 0.0)
        task_ids = sorted({r["task_id"] for r in agent_live})
        for removed in task_ids:
            subset = [r for r in agent_live if r["task_id"] != removed]
            ev = len(subset)
            passed = sum(1 for r in subset if r.get("status_L1") == "pass")
            rate = passed / ev if ev else 0.0
            loo_rows.append(
                {
                    "agent_id": agent_id,
                    "removed_task_id": removed,
                    "live_passed": passed,
                    "live_evaluated": ev,
                    "live_pass_rate": rate,
                    "delta_vs_static": rate - static_p,
                }
            )

    summary_loo: list[dict[str, Any]] = []
    for agent_id, agent_live in sorted(by_agent_live.items()):
        static_p = static_rates.get(agent_id, 0.0)
        rates: list[float] = []
        deltas: list[float] = []
        task_ids = sorted({r["task_id"] for r in agent_live})
        for removed in task_ids:
            subset = [r for r in agent_live if r["task_id"] != removed]
            ev = len(subset)
            passed = sum(1 for r in subset if r.get("status_L1") == "pass")
            rate = passed / ev if ev else 0.0
            rates.append(rate)
            deltas.append(rate - static_p)
        if rates:
            summary_loo.append(
                {
                    "agent_id": agent_id,
                    "min_live_pass_rate_loo": min(rates),
                    "max_live_pass_rate_loo": max(rates),
                    "min_delta_loo": min(deltas),
                    "max_delta_loo": max(deltas),
                }
            )

    out_path.parent.mkdir(parents=True, exist_ok=True)
    fields_loo = [
        "row_kind",
        "agent_id",
        "removed_task_id",
        "live_passed",
        "live_evaluated",
        "live_pass_rate",
        "delta_vs_static",
        "min_live_pass_rate_loo",
        "max_live_pass_rate_loo",
        "min_delta_loo",
        "max_delta_loo",
    ]
    with out_path.open("w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields_loo)
        w.writeheader()
        for row in loo_rows:
            w.writerow(
                {
                    "row_kind": "loo_detail",
                    "agent_id": row["agent_id"],
                    "removed_task_id": row["removed_task_id"],
                    "live_passed": row["live_passed"],
                    "live_evaluated": row["live_evaluated"],
                    "live_pass_rate": row["live_pass_rate"],
                    "delta_vs_static": row["delta_vs_static"],
                    "min_live_pass_rate_loo": "",
                    "max_live_pass_rate_loo": "",
                    "min_delta_loo": "",
                    "max_delta_loo": "",
                }
            )
        for row in summary_loo:
            w.writerow(
                {
                    "row_kind": "loo_summary",
                    "agent_id": row["agent_id"],
                    "removed_task_id": "",
                    "live_passed": "",
                    "live_evaluated": "",
                    "live_pass_rate": "",
                    "delta_vs_static": "",
                    "min_live_pass_rate_loo": row["min_live_pass_rate_loo"],
                    "max_live_pass_rate_loo": row["max_live_pass_rate_loo"],
                    "min_delta_loo": row["min_delta_loo"],
                    "max_delta_loo": row["max_delta_loo"],
                }
            )

    return {
        "per_task_rows": len(per_task),
        "loo_detail_rows": len(loo_rows),
        "loo_summary_rows": len(summary_loo),
        "out": str(out_path),
    }


def main() -> int:
    root = _repo_root()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--paper-export-dir",
        type=Path,
        default=root / "paper" / "exports" / "live_panel_v2_postbatch",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Output CSV (default: <paper-export-dir>/live_leave_one_out.csv)",
    )
    args = parser.parse_args()
    export_dir = args.paper_export_dir
    if not export_dir.is_absolute():
        export_dir = (root / export_dir).resolve()
    per_task = export_dir / "per_task_live_outcomes.csv"
    if not per_task.is_file():
        print(
            f"live_panel_leave_one_out: missing {per_task}\n"
            "Run: python packages/python/scripts/export_live_panel_tables.py "
            f"--run-dir runs/released/live_panel_v2/results_opt --out-dir {export_dir}",
            file=sys.stderr,
        )
        return 1
    out_path = args.out or (export_dir / "live_leave_one_out.csv")
    if not out_path.is_absolute():
        out_path = (root / out_path).resolve()
    meta = _write_loo_from_per_task(per_task, out_path)
    print(json.dumps(meta, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
