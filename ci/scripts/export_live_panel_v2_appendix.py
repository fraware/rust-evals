#!/usr/bin/env python3
"""Export per-task live outcomes and leave-one-out sensitivity tables."""

from __future__ import annotations

import csv
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
RUN_SUMMARY = (
    REPO_ROOT
    / "runs"
    / "released"
    / "live_panel_v2"
    / "results_opt"
    / "batch_summary.json"
)
LIVE_MANIFESTS = REPO_ROOT / "benchmarks" / "live" / "manifests"
VERIFIED_MANIFESTS = REPO_ROOT / "benchmarks" / "verified" / "manifests"

OUT_DIR = REPO_ROOT / "paper" / "exports" / "live_panel_v2_postbatch"
OUT_PER_TASK = OUT_DIR / "per_task_live_outcomes.csv"
OUT_SENSITIVITY = OUT_DIR / "live_sensitivity_analysis.csv"


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be a JSON object")
    return data


def _manifest_meta(task_id: str) -> tuple[str, str, str]:
    live_path = LIVE_MANIFESTS / f"{task_id}.json"
    if live_path.is_file():
        data = _load_json(live_path)
        return (
            "live",
            str(data.get("repo_name", "")),
            str(data.get("created_at", "")) or str(data.get("source_url", "")),
        )
    ver_path = VERIFIED_MANIFESTS / f"{task_id}.json"
    if ver_path.is_file():
        data = _load_json(ver_path)
        return (
            "verified_anchor",
            str(data.get("repo_name", "")),
            str(data.get("created_at", "")) or str(data.get("source_url", "")),
        )
    return ("unknown", "", "")


def _agent_id(entry_id: str) -> str:
    return entry_id.split("__", 1)[0] if "__" in entry_id else "unknown"


def _task_id(entry: dict[str, Any]) -> str:
    task_path = Path(str(entry.get("task_path", "")))
    return task_path.stem


def _status(levels: dict[str, Any], key: str) -> str:
    value = levels.get(key, {}) if isinstance(levels, dict) else {}
    if isinstance(value, dict):
        return str(value.get("status", ""))
    return ""


def _reason(levels: dict[str, Any], key: str) -> str:
    value = levels.get(key, {}) if isinstance(levels, dict) else {}
    if isinstance(value, dict):
        return str(value.get("primary_reason", ""))
    return ""


def _write_per_task(rows: list[dict[str, Any]]) -> None:
    fields = [
        "task_id",
        "benchmark_surface",
        "agent_id",
        "status_L0",
        "status_L1",
        "primary_reason",
        "repo",
        "task_date_or_source",
    ]
    with OUT_PER_TASK.open("w", encoding="utf-8", newline="") as h:
        writer = csv.DictWriter(h, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow({k: row.get(k, "") for k in fields})


def _compute_sensitivity(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    live_rows = [r for r in rows if r["benchmark_surface"] == "live"]
    by_agent: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in live_rows:
        by_agent[row["agent_id"]].append(row)

    out: list[dict[str, Any]] = []
    for agent_id, agent_rows in sorted(by_agent.items()):
        task_ids = sorted({r["task_id"] for r in agent_rows})
        if not task_ids:
            continue
        loo_rates: dict[str, float] = {}
        for leave_out in task_ids:
            subset = [r for r in agent_rows if r["task_id"] != leave_out]
            denom = len(subset)
            if denom == 0:
                rate = 0.0
            else:
                passes = sum(1 for r in subset if r["status_L1"] == "pass")
                rate = passes / denom
            loo_rates[leave_out] = rate
        min_task = min(loo_rates, key=lambda k: loo_rates[k])
        max_task = max(loo_rates, key=lambda k: loo_rates[k])
        out.append(
            {
                "agent_id": agent_id,
                "live_tasks_total": len(task_ids),
                "baseline_live_pass_rate_L1": (
                    sum(
                        1 for r in agent_rows if r["status_L1"] == "pass"
                    ) / len(agent_rows)
                ),
                "leave_one_out_min_rate_L1": loo_rates[min_task],
                "leave_one_out_max_rate_L1": loo_rates[max_task],
                "min_rate_when_excluding_task": min_task,
                "max_rate_when_excluding_task": max_task,
            }
        )
    return out


def _write_sensitivity(rows: list[dict[str, Any]]) -> None:
    fields = [
        "agent_id",
        "live_tasks_total",
        "baseline_live_pass_rate_L1",
        "leave_one_out_min_rate_L1",
        "leave_one_out_max_rate_L1",
        "min_rate_when_excluding_task",
        "max_rate_when_excluding_task",
    ]
    with OUT_SENSITIVITY.open("w", encoding="utf-8", newline="") as h:
        writer = csv.DictWriter(h, fieldnames=fields)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    summary = _load_json(RUN_SUMMARY)
    rows: list[dict[str, Any]] = []
    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        levels = entry.get("levels", {})
        if not isinstance(levels, dict):
            levels = {}
        task_id = _task_id(entry)
        benchmark_surface, repo, source = _manifest_meta(task_id)
        rows.append(
            {
                "task_id": task_id,
                "benchmark_surface": benchmark_surface,
                "agent_id": _agent_id(str(entry.get("entry_id", ""))),
                "status_L0": _status(levels, "l0"),
                "status_L1": _status(levels, "l1"),
                "primary_reason": (
                    _reason(levels, "l1") or _reason(levels, "l0")
                ),
                "repo": repo,
                "task_date_or_source": source,
            }
        )
    rows.sort(
        key=lambda r: (
            r["benchmark_surface"],
            r["task_id"],
            r["agent_id"],
        )
    )
    _write_per_task(rows)
    sensitivity = _compute_sensitivity(rows)
    _write_sensitivity(sensitivity)
    print(
        json.dumps(
            {
                "per_task_rows": len(rows),
                "sensitivity_rows": len(sensitivity),
                "per_task_csv": str(OUT_PER_TASK),
                "sensitivity_csv": str(OUT_SENSITIVITY),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
