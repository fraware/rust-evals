"""Build ``runs/released/agent_panel_v2/`` — scaled Verified agent panel.

Same three SWE-bench submission families as ``build_agent_panel.py`` (v1), but
over **10** Verified tasks instead of five. Patch bytes are still fetched from
the public ``swe-bench-submissions`` S3 layout documented in v1.

The committed directory is a reproducible *description* of the evaluation;
``eval-ladder evaluate batch`` still requires Docker for L0/L1 on these tasks.
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
DEFAULT_OUT = REPO_ROOT / "runs" / "released" / "agent_panel_v2"
CANDIDATES_DIR_NAME = "candidates"
PATCHES_DIR_NAME = "patches"
PANEL_FILE_NAME = "panel.jsonl"
VERIFIED_MANIFESTS = REPO_ROOT / "benchmarks" / "verified" / "manifests"

NAMESPACE_AGENT_PANEL_V2 = uuid.UUID("8b1c0f62-9a31-5ab4-9f7e-2c4d3e5f6a01")

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

TASKS_V1: tuple[str, ...] = (
    "astropy__astropy-14309",
    "django__django-10554",
    "django__django-11066",
    "django__django-11211",
    "astropy__astropy-14096",
)

TASKS_V2_EXTRA: tuple[str, ...] = (
    "matplotlib__matplotlib-25311",
    "psf__requests-2317",
    "pydata__xarray-4094",
    "pytest-dev__pytest-10051",
    "sphinx-doc__sphinx-8721",
)

TASKS: tuple[str, ...] = tuple(sorted(set(TASKS_V1 + TASKS_V2_EXTRA)))

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
    return str(uuid.uuid5(NAMESPACE_AGENT_PANEL_V2, "|".join(parts)))


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


def build_panel(out_root: Path) -> int:
    panel_root = out_root
    candidates_dir = panel_root / CANDIDATES_DIR_NAME
    patches_dir = panel_root / PATCHES_DIR_NAME
    panel_file = panel_root / PANEL_FILE_NAME

    candidates_dir.mkdir(parents=True, exist_ok=True)
    patches_dir.mkdir(parents=True, exist_ok=True)
    submitted_at = _dt.datetime(2024, 9, 1, 0, 0, 0, tzinfo=_dt.timezone.utc).isoformat().replace(
        "+00:00", "Z"
    )

    resolved = _resolved_lists()

    panel_lines: list[str] = []
    panel_rows_summary: list[dict] = []

    with httpx.Client() as client:
        for agent in AGENTS:
            agent_candidate_dir = candidates_dir / agent["agent_id"]
            agent_patch_dir = patches_dir / agent["agent_id"]
            agent_candidate_dir.mkdir(parents=True, exist_ok=True)
            agent_patch_dir.mkdir(parents=True, exist_ok=True)

            for task_id in TASKS:
                task_manifest = VERIFIED_MANIFESTS / f"{task_id}.json"
                if not task_manifest.exists():
                    raise SystemExit(
                        f"missing verified manifest for {task_id} at {task_manifest};"
                        " run ingest verified first"
                    )

                patch_bytes = _fetch_patch(client, agent["slug"], task_id)
                if patch_bytes is None:
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
                    patch_ref=str(patch_path.relative_to(panel_root)).replace("\\", "/"),
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

                def _rel_from_panel(p: Path) -> str:
                    rel = Path(*([".."] * 3)) / p.relative_to(REPO_ROOT)
                    return str(rel).replace("\\", "/")

                panel_entry = {
                    "task": _rel_from_panel(task_manifest),
                    "candidate": str(candidate_path.relative_to(panel_root)).replace("\\", "/"),
                    "patch": str(patch_path.relative_to(panel_root)).replace("\\", "/"),
                    "workspace_template": "workspaces/verified_shared/",
                    "bundle_name": f"{agent['agent_id']}__{task_id}",
                    "entry_id": f"{agent['agent_id']}__{task_id}",
                }
                panel_lines.append(json.dumps(panel_entry, sort_keys=True))

    panel_lines.sort()
    panel_file.write_text("\n".join(panel_lines) + "\n", encoding="utf-8")

    provenance = {
        "panel_id": "agent_panel_v2",
        "agents": AGENTS,
        "tasks": list(TASKS),
        "tasks_v1": list(TASKS_V1),
        "tasks_v2_extra": list(TASKS_V2_EXTRA),
        "entries": panel_rows_summary,
        "n_entries": len(panel_lines),
        "sources": {
            "leaderboard": "https://github.com/SWE-bench/experiments/tree/main/evaluation/verified",
            "patch_bucket": S3_BUCKET + "/verified/",
        },
        "resolved_counts": {a: len(resolved[a]["resolved"]) for a in resolved},
        "notes": [
            "UUIDv5 namespace is 8b1c0f62-9a31-5ab4-9f7e-2c4d3e5f6a01 (distinct from v1).",
            "See runs/released/agent_panel_v1/ for the smaller frozen reference panel.",
        ],
    }
    (panel_root / "provenance.json").write_text(
        json.dumps(provenance, sort_keys=True, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return len(panel_lines)


def main(argv: Iterable[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_OUT,
        help=f"output directory (default: {DEFAULT_OUT})",
    )
    args = ap.parse_args(list(argv) if argv is not None else None)
    n = build_panel(args.out.resolve())
    rel_panel = (args.out / PANEL_FILE_NAME).resolve().relative_to(REPO_ROOT)
    print(f"wrote {n} panel entries to {rel_panel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
