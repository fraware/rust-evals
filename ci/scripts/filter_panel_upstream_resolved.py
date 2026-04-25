#!/usr/bin/env python3
"""Filter a panel JSONL to rows whose task is upstream-resolved for that agent.

For each agent family, fetches SWE-bench ``experiments`` ``results.json`` and
keeps only panel lines where ``task_id`` appears in the ``resolved`` list for
that agent. Rows for agents that did not resolve the task upstream are dropped.

This breaks degenerate ``(l0_pass, l1_pass)`` ties across agents when the
underlying panel repeated the same failing patches for every family.

Requires ``httpx`` (declared in the repository root ``pyproject.toml``).
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any, cast

import httpx

AGENTS: list[tuple[str, str]] = [
    ("20240620_sweagent_claude3.5sonnet", "sweagent"),
    ("20240824_gru", "gru"),
    ("20240820_honeycomb", "honeycomb"),
]


def _results_url(agent_slug: str) -> str:
    return (
        "https://raw.githubusercontent.com/SWE-bench/experiments/main/"
        f"evaluation/verified/{agent_slug}/results/results.json"
    )


def _task_id_from_bundle(bundle_name: str) -> str:
    parts = bundle_name.split("__")
    if len(parts) < 2:
        return ""
    return "__".join(parts[1:])


def _load_resolved_sets(client: httpx.Client) -> dict[str, set[str]]:
    out: dict[str, set[str]] = {}
    for slug, aid in AGENTS:
        resp = client.get(_results_url(slug), timeout=60)
        resp.raise_for_status()
        data = cast(dict[str, Any], resp.json())
        out[aid] = set(data.get("resolved", []))
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--in", dest="panel_in", type=Path, required=True)
    ap.add_argument("--out", dest="panel_out", type=Path, required=True)
    ap.add_argument(
        "--summary",
        action="store_true",
        help="print JSON summary to stdout (kept/dropped counts, task coverage)",
    )
    args = ap.parse_args()

    lines_in = [
        ln
        for ln in args.panel_in.read_text(encoding="utf-8").splitlines()
        if ln.strip()
    ]
    if not lines_in:
        print("empty input panel", file=sys.stderr)
        return 2

    with httpx.Client() as client:
        resolved_by_agent = _load_resolved_sets(client)

    kept: list[str] = []
    dropped = 0
    for line in lines_in:
        row = json.loads(line)
        bundle = str(row.get("bundle_name", ""))
        parts = bundle.split("__")
        if len(parts) < 2:
            dropped += 1
            continue
        aid = parts[0]
        tid = "__".join(parts[1:])
        if tid in resolved_by_agent.get(aid, set()):
            kept.append(line)
        else:
            dropped += 1

    args.panel_out.parent.mkdir(parents=True, exist_ok=True)
    args.panel_out.write_text("\n".join(kept) + "\n", encoding="utf-8")

    if args.summary:
        per_task: Counter[str] = Counter()
        for line in kept:
            row = json.loads(line)
            per_task[_task_id_from_bundle(str(row.get("bundle_name", "")))] += 1
        summary = {
            "input_lines": len(lines_in),
            "kept": len(kept),
            "dropped": dropped,
            "unique_tasks": len(per_task),
            "min_rows_per_task": min(per_task.values()) if per_task else 0,
            "max_rows_per_task": max(per_task.values()) if per_task else 0,
        }
        print(json.dumps(summary, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
