"""Build ``runs/released/agent_panel_v1/`` from public SWE-bench agent submissions.

For each selected ``(agent, task)`` pair we:

    1. Download ``patch.diff`` from the public S3 bucket
       ``s3://swe-bench-submissions/verified/<agent>/logs/<instance>/``.
    2. Write it to ``patches/<agent>/<task>.diff``.
    3. Emit a ``CandidateResolution`` JSON at
       ``candidates/<agent>/<task>.json`` whose ``candidate_id`` is a
       deterministic UUIDv5 derived from ``(agent, task, patch_sha256)``
       so the panel is fully reproducible.
    4. Append one line to ``panel.jsonl`` pointing at the ingested task
       manifest under ``benchmarks/verified/manifests/`` plus the freshly
       written candidate and patch files.

The resulting panel is ready for ``eval-ladder evaluate batch``. L0/L1
require Docker Desktop at run time; without it the pipeline will flag
the entries as invalid. The committed panel is therefore a
reproducible *description* of the evaluation, not a pre-computed
verdict.

Source provenance:
    * SWE-bench experiments repo (leaderboard listing):
      https://github.com/SWE-bench/experiments/tree/main/evaluation/verified
    * Instance-level patch bucket:
      https://swe-bench-submissions.s3.amazonaws.com/
"""

from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import sys
import uuid
from pathlib import Path
from typing import Iterable

import httpx  # type: ignore[import-not-found]

REPO_ROOT = Path(__file__).resolve().parents[3]
PANEL_ROOT = REPO_ROOT / "runs" / "released" / "agent_panel_v1"
CANDIDATES_DIR = PANEL_ROOT / "candidates"
PATCHES_DIR = PANEL_ROOT / "patches"
PANEL_FILE = PANEL_ROOT / "panel.jsonl"
VERIFIED_MANIFESTS = REPO_ROOT / "benchmarks" / "verified" / "manifests"

NAMESPACE_AGENT_PANEL_V1 = uuid.UUID("3f3e0b4e-4f05-5b9d-9e4d-0a1c2a8b1e11")

AGENTS: list[dict] = [
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

TASKS: list[str] = [
    "astropy__astropy-14309",
    "django__django-10554",
    "django__django-11066",
    "django__django-11211",
    "astropy__astropy-14096",
]

S3_BUCKET = "https://swe-bench-submissions.s3.amazonaws.com"


def _patch_url(agent_slug: str, instance_id: str) -> str:
    return f"{S3_BUCKET}/verified/{agent_slug}/logs/{instance_id}/patch.diff"


def _fetch_patch(client: httpx.Client, agent_slug: str, instance_id: str) -> bytes | None:
    url = _patch_url(agent_slug, instance_id)
    resp = client.get(url, timeout=60)
    if resp.status_code == 404:
        return None
    resp.raise_for_status()
    return resp.content


def _deterministic_uuid(*parts: str) -> str:
    name = "|".join(parts)
    return str(uuid.uuid5(NAMESPACE_AGENT_PANEL_V1, name))


def _candidate_json(
    *,
    agent: dict,
    task_id: str,
    patch_ref: str,
    patch_sha256: str,
    submitted_at: str,
) -> dict:
    candidate_id = _deterministic_uuid(agent["agent_id"], task_id, patch_sha256)
    return {
        "schema_version": 1,
        "candidate_id": candidate_id,
        "benchmark_id": "swe_bench_verified",
        "task_id": task_id,
        "agent_id": agent["agent_id"],
        "model_id": agent["model_id"],
        "generation_mode": agent["generation_mode"],
        "patch_format": "unified_diff",
        "patch_ref": patch_ref,
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


def _resolved_lists() -> dict[str, dict[str, set[str]]]:
    """Download each agent's resolved/no_generation/no_logs lists."""
    out: dict[str, dict[str, set[str]]] = {}
    with httpx.Client() as client:
        for agent in AGENTS:
            url = (
                "https://raw.githubusercontent.com/SWE-bench/experiments/main/"
                f"evaluation/verified/{agent['slug']}/results/results.json"
            )
            data = client.get(url, timeout=30).json()
            out[agent["agent_id"]] = {
                "resolved": set(data.get("resolved", [])),
                "no_generation": set(data.get("no_generation", [])),
                "no_logs": set(data.get("no_logs", [])),
            }
    return out


def build_panel() -> int:
    CANDIDATES_DIR.mkdir(parents=True, exist_ok=True)
    PATCHES_DIR.mkdir(parents=True, exist_ok=True)
    submitted_at = _dt.datetime(2024, 9, 1, 0, 0, 0, tzinfo=_dt.timezone.utc).isoformat().replace("+00:00", "Z")

    resolved = _resolved_lists()

    panel_lines: list[str] = []
    panel_rows_summary: list[dict] = []

    with httpx.Client() as client:
        for agent in AGENTS:
            agent_candidate_dir = CANDIDATES_DIR / agent["agent_id"]
            agent_patch_dir = PATCHES_DIR / agent["agent_id"]
            agent_candidate_dir.mkdir(parents=True, exist_ok=True)
            agent_patch_dir.mkdir(parents=True, exist_ok=True)

            for task_id in TASKS:
                task_manifest = VERIFIED_MANIFESTS / f"{task_id}.json"
                if not task_manifest.exists():
                    raise SystemExit(
                        f"missing verified manifest for {task_id} at {task_manifest};"
                        " run fetch_upstream_datasets.py --which verified first"
                    )

                patch_bytes = _fetch_patch(client, agent["slug"], task_id)
                if patch_bytes is None:
                    # Agent did not submit a patch for this task; omit
                    # from the panel. Record for the summary.
                    panel_rows_summary.append(
                        {
                            "agent": agent["agent_id"],
                            "task": task_id,
                            "status": "omitted_no_patch_on_s3",
                        }
                    )
                    continue

                patch_path = agent_patch_dir / f"{task_id}.diff"
                patch_path.write_bytes(patch_bytes)
                patch_sha = hashlib.sha256(patch_bytes).hexdigest()
                candidate = _candidate_json(
                    agent=agent,
                    task_id=task_id,
                    patch_ref=str(patch_path.relative_to(PANEL_ROOT)).replace("\\", "/"),
                    patch_sha256=patch_sha,
                    submitted_at=submitted_at,
                )
                candidate_path = agent_candidate_dir / f"{task_id}.json"
                candidate_path.write_text(
                    json.dumps(candidate, sort_keys=True, ensure_ascii=False) + "\n",
                    encoding="utf-8",
                )

                is_resolved = task_id in resolved[agent["agent_id"]]["resolved"]
                panel_rows_summary.append(
                    {
                        "agent": agent["agent_id"],
                        "task": task_id,
                        "status": "ok",
                        "patch_sha256": patch_sha,
                        "upstream_resolved": is_resolved,
                    }
                )

                # Paths in panel.jsonl are resolved relative to the
                # directory that contains the panel file. The panel
                # lives at runs/released/agent_panel_v1/panel.jsonl, so
                # repo-root siblings need a `../../../` prefix.
                def _rel_from_panel(p: Path) -> str:
                    rel = Path(*([".."] * 3)) / p.relative_to(REPO_ROOT)
                    return str(rel).replace("\\", "/")

                panel_entry = {
                    "task": _rel_from_panel(task_manifest),
                    "candidate": str(candidate_path.relative_to(PANEL_ROOT)).replace("\\", "/"),
                    "patch": str(patch_path.relative_to(PANEL_ROOT)).replace("\\", "/"),
                    # Workspace template is supplied at run time; Docker
                    # engine resolves environment_ref per task.
                    "workspace_template": "workspaces/verified_shared/",
                    "bundle_name": f"{agent['agent_id']}__{task_id}",
                    "entry_id": f"{agent['agent_id']}__{task_id}",
                }
                panel_lines.append(json.dumps(panel_entry, sort_keys=True))

    # Stable sort so the committed panel is reproducible.
    panel_lines.sort()
    PANEL_FILE.write_text("\n".join(panel_lines) + "\n", encoding="utf-8")

    # Emit provenance alongside the panel.
    provenance = {
        "panel_id": "agent_panel_v1",
        "agents": AGENTS,
        "tasks": TASKS,
        "entries": panel_rows_summary,
        "n_entries": len(panel_lines),
        "sources": {
            "leaderboard": "https://github.com/SWE-bench/experiments/tree/main/evaluation/verified",
            "patch_bucket": S3_BUCKET + "/verified/",
        },
        "resolved_counts": {a: len(resolved[a]["resolved"]) for a in resolved},
        "notes": [
            "patch_sha256 is the raw SHA-256 of the per-(agent,task) patch.diff bytes.",
            "candidate_id is UUIDv5(namespace=3f3e0b4e-...;name='agent|task|patch_sha256').",
            "The panel currently assumes workspaces/verified_shared/ is provided at run time; it is NOT materialised here because L0/L1 for SWE-bench Verified requires Docker and per-task environment bootstrap.",
        ],
    }
    (PANEL_ROOT / "provenance.json").write_text(
        json.dumps(provenance, sort_keys=True, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return len(panel_lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.parse_args()
    n = build_panel()
    print(f"wrote {n} panel entries to {PANEL_FILE.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
