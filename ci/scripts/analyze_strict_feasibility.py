#!/usr/bin/env python3
"""Offline feasibility analysis for strict Verified and Rust gates.

This script reads existing run artifacts only (no evaluation reruns) and emits
a machine-readable report that helps decide whether strict gates are currently
reachable with the in-repo evidence inventory.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any

AGENTS = {"gru", "honeycomb", "sweagent"}


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be a JSON object")
    return data


def _iter_batch_summaries(root: Path) -> list[Path]:
    return sorted(root.rglob("batch_summary.json"))


def _agent_from_entry_id(entry_id: str) -> str:
    if "__" in entry_id:
        return entry_id.split("__", 1)[0]
    return "unknown"


@dataclass(frozen=True)
class PassObservation:
    task_id: str
    agent_id: str
    summary_path: str


def _task_from_entry(entry: dict[str, Any]) -> str:
    task_path = str(entry.get("task_path", ""))
    if not task_path:
        return "unknown"
    return Path(task_path).stem


def analyze_verified_inventory(
    root: Path, min_candidates: int, max_harness_error_rate: float
) -> dict[str, Any]:
    pass_observations: list[PassObservation] = []

    for summary_path in _iter_batch_summaries(root):
        try:
            summary = _load_json(summary_path)
        except Exception:
            continue
        for entry in summary.get("entries", []):
            if not isinstance(entry, dict):
                continue
            levels = entry.get("levels", {})
            l1 = levels.get("l1", {}) if isinstance(levels, dict) else {}
            if str(l1.get("status", "")).lower() != "pass":
                continue
            entry_id = str(entry.get("entry_id", ""))
            agent_id = _agent_from_entry_id(entry_id)
            if agent_id not in AGENTS:
                continue
            pass_observations.append(
                PassObservation(
                    task_id=_task_from_entry(entry),
                    agent_id=agent_id,
                    summary_path=str(summary_path),
                )
            )

    unique_task_agent_pairs = {
        (row.task_id, row.agent_id) for row in pass_observations
    }
    tasks_by_agent: dict[str, set[str]] = defaultdict(set)
    for task_id, agent_id in unique_task_agent_pairs:
        tasks_by_agent[agent_id].add(task_id)

    all_agents_task_intersection = set.intersection(
        *(tasks_by_agent.get(agent, set()) for agent in sorted(AGENTS))
    )
    max_rows_if_single_candidate_per_task = (
        len(all_agents_task_intersection) * len(AGENTS)
    )
    max_harness_errors_allowed = int(min_candidates * max_harness_error_rate)

    task_counter = Counter(task_id for task_id, _ in unique_task_agent_pairs)
    agent_counter = Counter(agent_id for _, agent_id in unique_task_agent_pairs)

    return {
        "strict_thresholds": {
            "min_candidates": min_candidates,
            "max_l1_harness_error_rate": max_harness_error_rate,
            "max_harness_errors_allowed_at_min_candidates": (
                max_harness_errors_allowed
            ),
        },
        "inventory": {
            "l1_pass_observations": len(pass_observations),
            "unique_task_agent_pairs": len(unique_task_agent_pairs),
            "unique_tasks_with_any_public_agent_pass": len(task_counter),
            "unique_tasks_by_agent": {
                key: len(value)
                for key, value in sorted(tasks_by_agent.items())
            },
            "agent_pair_counts": dict(sorted(agent_counter.items())),
            "task_pair_counts": dict(sorted(task_counter.items())),
            "tasks_passing_for_all_public_agents": sorted(
                all_agents_task_intersection
            ),
            "max_rows_if_single_candidate_per_task": (
                max_rows_if_single_candidate_per_task
            ),
        },
        "assessment": {
            "supports_min_candidates_without_new_tasks": (
                max_rows_if_single_candidate_per_task >= min_candidates
            ),
            "note": (
                "This is an offline inventory bound from existing summaries "
                "only. Failure means strict candidate volume cannot be met "
                "from currently observed stable task-agent pass pairs "
                "without adding new tasks and/or "
                "new candidates."
            ),
        },
    }


def analyze_rust_real_manifest(rust_summary: Path) -> dict[str, Any]:
    summary = _load_json(rust_summary)
    entries = summary.get("entries", [])

    l3_pass_l4_fail = 0
    all_level_pass = 0
    l4_reason_counts: Counter[str] = Counter()
    per_entry: list[dict[str, Any]] = []

    for entry in entries:
        if not isinstance(entry, dict):
            continue
        levels = entry.get("levels", {})
        l0 = levels.get("l0", {}) if isinstance(levels, dict) else {}
        l1 = levels.get("l1", {}) if isinstance(levels, dict) else {}
        l3 = levels.get("l3", {}) if isinstance(levels, dict) else {}
        l4 = levels.get("l4", {}) if isinstance(levels, dict) else {}

        l3_status = str(l3.get("status", "")).lower()
        l4_status = str(l4.get("status", "")).lower()
        l4_reason = str(l4.get("primary_reason", ""))
        l4_reason_counts[l4_reason] += 1

        if l3_status == "pass" and l4_status == "fail":
            l3_pass_l4_fail += 1
        if (
            str(l0.get("status", "")).lower() == "pass"
            and str(l1.get("status", "")).lower() == "pass"
            and l3_status == "pass"
            and l4_status == "pass"
        ):
            all_level_pass += 1

        per_entry.append(
            {
                "entry_id": entry.get("entry_id"),
                "l3_status": l3.get("status"),
                "l4_status": l4.get("status"),
                "l4_reason": l4.get("primary_reason"),
            }
        )

    return {
        "input_summary": str(rust_summary),
        "metrics": {
            "total_entries": len(entries),
            "l3_pass_l4_fail": l3_pass_l4_fail,
            "all_level_pass": all_level_pass,
            "l4_reason_counts": dict(l4_reason_counts),
        },
        "per_entry": per_entry,
        "assessment": {
            "strict_semantic_minima_met": (
                l3_pass_l4_fail >= 2 and all_level_pass >= 1
            ),
            "note": (
                "Computed from the real-manifest sealed summary only; synthetic "
                "paper-semantics replay is intentionally excluded."
            ),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Offline strict-feasibility analysis for Verified and Rust."
        )
    )
    parser.add_argument(
        "--runs-root",
        type=Path,
        default=Path("runs"),
        help="Root directory to scan for batch_summary.json files.",
    )
    parser.add_argument(
        "--rust-summary",
        type=Path,
        default=Path(
            "runs/released/rust_proof_subset_v1/results_seal/batch_summary.json"
        ),
        help="Path to real-manifest rust sealed summary.",
    )
    parser.add_argument(
        "--min-candidates",
        type=int,
        default=30,
        help="Strict verified gate candidate threshold.",
    )
    parser.add_argument(
        "--max-l1-harness-error-rate",
        type=float,
        default=0.10,
        help="Strict verified L1 harness error-rate cap.",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("paper/exports/strict_feasibility_report.json"),
        help="Output report path.",
    )
    args = parser.parse_args()

    report = {
        "verified": analyze_verified_inventory(
            root=args.runs_root,
            min_candidates=args.min_candidates,
            max_harness_error_rate=args.max_l1_harness_error_rate,
        ),
        "rust_real_manifest": analyze_rust_real_manifest(args.rust_summary),
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(report, indent=2, sort_keys=True),
        encoding="utf-8",
    )
    print(json.dumps({"written": str(args.out)}, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
