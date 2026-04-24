"""Build ``runs/released/agent_panel_v3/`` with non-degenerate task selection.

This script extends v2 by selecting Verified tasks that have submissions from
multiple agent families and at least one known upstream resolution. The goal is
to avoid "all-zero" panels and improve comparative signal for NeurIPS evidence.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import sys
import uuid
from collections.abc import Iterable
from pathlib import Path

import httpx  # type: ignore[import-not-found]

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_OUT = REPO_ROOT / "runs" / "released" / "agent_panel_v3"
VERIFIED_MANIFESTS = REPO_ROOT / "benchmarks" / "verified" / "manifests"
S3_BUCKET = "https://swe-bench-submissions.s3.amazonaws.com"
NAMESPACE = uuid.UUID("22d79d39-624e-5e9e-83a7-891f47fa2a8a")

AGENTS: list[dict[str, str]] = [
    {
        "slug": "20240620_sweagent_claude3.5sonnet",
        "agent_id": "sweagent",
        "model_id": "claude-3-5-sonnet-20241022",
        "generation_mode": "agent_loop",
    },
    {
        "slug": "20240824_gru",
        "agent_id": "gru",
        "model_id": "gru-2024-08-24",
        "generation_mode": "agent_loop",
    },
    {
        "slug": "20240820_honeycomb",
        "agent_id": "honeycomb",
        "model_id": "honeycomb-2024-08-20",
        "generation_mode": "agent_loop",
    },
]


def _results_url(agent_slug: str) -> str:
    return (
        "https://raw.githubusercontent.com/SWE-bench/experiments/main/"
        f"evaluation/verified/{agent_slug}/results/results.json"
    )


def _patch_url(agent_slug: str, task_id: str) -> str:
    return f"{S3_BUCKET}/verified/{agent_slug}/logs/{task_id}/patch.diff"


def _det_uuid(*parts: str) -> str:
    return str(uuid.uuid5(NAMESPACE, "|".join(parts)))


def _load_results(client: httpx.Client) -> dict[str, dict[str, set[str]]]:
    out: dict[str, dict[str, set[str]]] = {}
    for agent in AGENTS:
        data = client.get(_results_url(agent["slug"]), timeout=30).json()
        out[agent["agent_id"]] = {
            "resolved": set(data.get("resolved", [])),
            "no_generation": set(data.get("no_generation", [])),
            "no_logs": set(data.get("no_logs", [])),
        }
    return out


def _task_score(
    task_id: str,
    results: dict[str, dict[str, set[str]]],
) -> tuple[int, int]:
    # Prioritize tasks with known submissions across agents and at least one
    # upstream resolution signal.
    with_patch = 0
    resolved = 0
    for agent in AGENTS:
        aid = agent["agent_id"]
        if (
            task_id not in results[aid]["no_generation"]
            and task_id not in results[aid]["no_logs"]
        ):
            with_patch += 1
        if task_id in results[aid]["resolved"]:
            resolved += 1
    return (with_patch, resolved)


def _select_tasks(
    results: dict[str, dict[str, set[str]]],
    *,
    max_tasks: int,
    min_agents_with_submission: int,
) -> list[str]:
    candidates = sorted(p.stem for p in VERIFIED_MANIFESTS.glob("*.json"))
    scored: list[tuple[str, int, int]] = []
    for task_id in candidates:
        with_patch, resolved = _task_score(task_id, results)
        if with_patch < min_agents_with_submission:
            continue
        if resolved == 0:
            continue
        scored.append((task_id, with_patch, resolved))
    scored.sort(key=lambda t: (t[2], t[1], t[0]), reverse=True)
    return [task for task, _, _ in scored[:max_tasks]]


def _fetch_patch(
    client: httpx.Client,
    agent_slug: str,
    task_id: str,
) -> bytes | None:
    resp = client.get(_patch_url(agent_slug, task_id), timeout=60)
    if resp.status_code == 404:
        return None
    resp.raise_for_status()
    return resp.content


def build_panel(
    out_root: Path,
    *,
    max_tasks: int,
    min_agents_with_submission: int,
) -> int:
    panel_root = out_root
    candidates_dir = panel_root / "candidates"
    patches_dir = panel_root / "patches"
    panel_file = panel_root / "panel.jsonl"
    candidates_dir.mkdir(parents=True, exist_ok=True)
    patches_dir.mkdir(parents=True, exist_ok=True)

    submitted_at = (
        _dt.datetime(2024, 9, 1, 0, 0, 0, tzinfo=_dt.timezone.utc)
        .isoformat()
        .replace("+00:00", "Z")
    )

    with httpx.Client() as client:
        results = _load_results(client)
        tasks = _select_tasks(
            results,
            max_tasks=max_tasks,
            min_agents_with_submission=min_agents_with_submission,
        )
        if not tasks:
            raise SystemExit("no tasks selected for agent_panel_v3")

        panel_lines: list[str] = []
        entries: list[dict[str, object]] = []
        for agent in AGENTS:
            aid = agent["agent_id"]
            agent_candidate_dir = candidates_dir / aid
            agent_patch_dir = patches_dir / aid
            agent_candidate_dir.mkdir(parents=True, exist_ok=True)
            agent_patch_dir.mkdir(parents=True, exist_ok=True)
            for task_id in tasks:
                task_manifest = VERIFIED_MANIFESTS / f"{task_id}.json"
                patch_bytes = _fetch_patch(client, agent["slug"], task_id)
                if patch_bytes is None:
                    entries.append(
                        {
                            "agent": aid,
                            "task": task_id,
                            "status": "omitted_no_patch_on_s3",
                            "upstream_resolved": (
                                task_id in results[aid]["resolved"]
                            ),
                        }
                    )
                    continue

                patch_path = agent_patch_dir / f"{task_id}.diff"
                patch_path.write_bytes(patch_bytes)
                patch_sha = hashlib.sha256(patch_bytes).hexdigest()

                candidate = {
                    "schema_version": 1,
                    "candidate_id": _det_uuid(aid, task_id, patch_sha),
                    "benchmark_id": "swe_bench_verified",
                    "task_id": task_id,
                    "agent_id": aid,
                    "model_id": agent["model_id"],
                    "generation_mode": agent["generation_mode"],
                    "patch_format": "unified_diff",
                    "patch_ref": str(
                        patch_path.relative_to(panel_root)
                    ).replace("\\", "/"),
                    "generation_metadata": {
                        "tool_configuration": {
                            "submission_slug": agent["slug"],
                            "source": "SWE-bench/experiments",
                        },
                        "context_mode": "retrieval",
                        "repo_reproduction_used": True,
                    },
                    "submitted_at": submitted_at,
                }
                candidate_path = agent_candidate_dir / f"{task_id}.json"
                candidate_path.write_text(
                    json.dumps(candidate, sort_keys=True, ensure_ascii=False) + "\n",
                    encoding="utf-8",
                )

                rel_task = (
                    Path("..")
                    / ".."
                    / ".."
                    / task_manifest.relative_to(REPO_ROOT)
                )
                panel_entry = {
                    "task": str(rel_task).replace("\\", "/"),
                    "candidate": str(
                        candidate_path.relative_to(panel_root)
                    ).replace("\\", "/"),
                    "patch": str(
                        patch_path.relative_to(panel_root)
                    ).replace("\\", "/"),
                    "workspace_template": "workspaces/verified_shared/",
                    "bundle_name": f"{aid}__{task_id}",
                    "entry_id": f"{aid}__{task_id}",
                }
                panel_lines.append(json.dumps(panel_entry, sort_keys=True))
                entries.append(
                    {
                        "agent": aid,
                        "task": task_id,
                        "status": "ok",
                        "patch_sha256": patch_sha,
                        "upstream_resolved": task_id in results[aid]["resolved"],
                    }
                )

    panel_lines.sort()
    panel_file.write_text("\n".join(panel_lines) + "\n", encoding="utf-8")

    provenance = {
        "panel_id": "agent_panel_v3",
        "agents": AGENTS,
        "n_entries": len(panel_lines),
        "selection": {
            "max_tasks": max_tasks,
            "min_agents_with_submission": min_agents_with_submission,
            "policy": (
                "resolved>=1 and multi-agent patch availability"
            ),
        },
        "entries": entries,
        "sources": {
            "leaderboard": (
                "https://github.com/SWE-bench/experiments/tree/main/"
                "evaluation/verified"
            ),
            "patch_bucket": S3_BUCKET + "/verified/",
        },
    }
    (panel_root / "provenance.json").write_text(
        json.dumps(
            provenance, sort_keys=True, ensure_ascii=False, indent=2
        )
        + "\n",
        encoding="utf-8",
    )
    return len(panel_lines)


def main(argv: Iterable[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--max-tasks", type=int, default=12)
    ap.add_argument("--min-agents-with-submission", type=int, default=2)
    args = ap.parse_args(list(argv) if argv is not None else None)
    n = build_panel(
        args.out.resolve(),
        max_tasks=args.max_tasks,
        min_agents_with_submission=args.min_agents_with_submission,
    )
    rel = (args.out / "panel.jsonl").resolve().relative_to(REPO_ROOT)
    print(f"wrote {n} panel entries to {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
