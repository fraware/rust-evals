"""Build ``runs/released/live_panel_v2/`` with asymmetric live patch assignment.

``live_panel_v1`` gives every agent the same pass pattern on the live arm
(all agents share the same per-task patch strategy except sweagent, and the
sealed batch still ties).  v2 breaks symmetry **without** changing benchmark
manifests:

- **gru** uses the dataset ``patch`` on every live task (gold).
- **honeycomb** uses ``test_patch`` on the **first half** of the live task list
  and ``patch`` on the rest.
- **sweagent** uses ``patch`` on the **first half** and ``test_patch`` on the
  second half (complementary to honeycomb).

Static anchors (SWE-Bench Verified) stay identical to v1: gold ``patch`` for
every agent so the static-vs-live **delta** remains negative when live
underperforms static.

Candidate ids are UUIDv5 under a v2-specific namespace so the panel does not
collide with v1.
"""

from __future__ import annotations

import datetime as dt
import hashlib
import json
import shutil
import stat
import subprocess
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, cast

REPO_ROOT = Path(__file__).resolve().parents[3]
OUT_ROOT = REPO_ROOT / "runs" / "released" / "live_panel_v2"
PATCHES_DIR = OUT_ROOT / "patches"
CANDIDATES_DIR = OUT_ROOT / "candidates"
WORKSPACES_DIR = OUT_ROOT / "workspaces"
PANEL_PATH = OUT_ROOT / "panel.jsonl"
PROVENANCE_PATH = OUT_ROOT / "provenance.json"

LIVE_MANIFEST_DIR = REPO_ROOT / "benchmarks" / "live" / "manifests"
VERIFIED_MANIFEST_DIR = REPO_ROOT / "benchmarks" / "verified" / "manifests"
LIVE_CACHE = REPO_ROOT / "datasets" / "cache" / "live" / "swe_bench_live_verified.jsonl"
VERIFIED_CACHE = REPO_ROOT / "datasets" / "cache" / "verified" / "swe_bench_verified.jsonl"

NAMESPACE = uuid.UUID("c2d4f1a6-8b0e-4f2c-9d11-0a1b2c3d4e5f")
SUBMITTED_AT = dt.datetime(2025, 5, 1, tzinfo=dt.timezone.utc).isoformat().replace("+00:00", "Z")


@dataclass(frozen=True)
class Agent:
    agent_id: str
    model_id: str


AGENTS = [
    Agent("gru", "gru-2024-08-24"),
    Agent("honeycomb", "honeycomb-2024-08-20"),
    Agent("sweagent", "claude-3-5-sonnet-20241022"),
]

LIVE_TASKS = [
    "Pyomo__pyomo-3588",
    "networkx__networkx-8013",
    "networktocode__ntc-templates-2118",
    "jlowin__fastmcp-279",
    "apify__crawlee-python-1181",
    "pyca__cryptography-12812",
    "feature-engine__feature_engine-856",
    "fsspec__filesystem_spec-1829",
]

VERIFIED_TASKS = [
    "astropy__astropy-12907",
    "pydata__xarray-2905",
    "astropy__astropy-14096",
    "astropy__astropy-14309",
    "django__django-11211",
]


def _force_writable(func: Any, path: str, _exc: object) -> None:
    p = Path(path)
    p.chmod(stat.S_IWRITE)
    func(path)


def _robust_rmtree(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path, onerror=_force_writable)


def _load_jsonl_by_task(path: Path) -> dict[str, dict[str, Any]]:
    out: dict[str, dict[str, Any]] = {}
    for raw in path.read_text(encoding="utf-8").splitlines():
        stripped = raw.strip()
        if not stripped:
            continue
        obj = cast(dict[str, Any], json.loads(stripped))
        task_id = obj.get("instance_id") or obj.get("task_id")
        if not task_id:
            continue
        out[task_id] = obj
    return out


def _candidate_id(agent_id: str, benchmark_id: str, task_id: str, patch_sha: str) -> str:
    name = f"{agent_id}|{benchmark_id}|{task_id}|{patch_sha}"
    return str(uuid.uuid5(NAMESPACE, name))


def _write_candidate(
    *,
    agent: Agent,
    benchmark_id: str,
    task_id: str,
    patch_ref: str,
    patch_sha: str,
    source: str,
    out_path: Path,
) -> None:
    candidate = {
        "schema_version": 1,
        "candidate_id": _candidate_id(agent.agent_id, benchmark_id, task_id, patch_sha),
        "benchmark_id": benchmark_id,
        "task_id": task_id,
        "agent_id": agent.agent_id,
        "model_id": agent.model_id,
        "generation_mode": "agent_loop",
        "patch_format": "unified_diff",
        "patch_ref": patch_ref,
        "generation_metadata": {
            "tool_configuration": {"source": source},
            "context_mode": "retrieval",
            "repo_reproduction_used": True,
            "random_seed": 0,
            "temperature": 0.0,
        },
        "submitted_at": SUBMITTED_AT,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(candidate, sort_keys=True) + "\n", encoding="utf-8")


def _materialize_workspace(task_id: str, repo_name: str, base_commit: str) -> str:
    ws = WORKSPACES_DIR / task_id
    if ws.exists():
        return f"workspaces/{task_id}/"
    ws.mkdir(parents=True, exist_ok=True)

    tmp = Path.cwd().joinpath(f".tmp-live-panel-v2-{task_id.replace('/', '_')}")
    _robust_rmtree(tmp)
    subprocess.run(["git", "init", str(tmp)], check=True, cwd=REPO_ROOT)
    subprocess.run(
        ["git", "remote", "add", "origin", f"https://github.com/{repo_name}.git"],
        check=True,
        cwd=tmp,
    )
    subprocess.run(["git", "fetch", "--depth", "1", "origin", base_commit], check=True, cwd=tmp)
    subprocess.run(["git", "checkout", "--detach", "FETCH_HEAD"], check=True, cwd=tmp)

    for item in tmp.iterdir():
        if item.name == ".git":
            continue
        dst = ws / item.name
        if item.is_dir():
            shutil.copytree(item, dst, dirs_exist_ok=True)
        else:
            shutil.copy2(item, dst)
    _robust_rmtree(tmp)
    return f"workspaces/{task_id}/"


def _rel_from_panel(path: Path) -> str:
    rel = Path("..") / ".." / ".." / path.relative_to(REPO_ROOT)
    return str(rel).replace("\\", "/")


def _live_tasks_for_agent(agent_id: str) -> list[str]:
    """Disjoint live slices so per-agent live pass *rates* can diverge.

    ``gru`` evaluates the full eight-task live arm on gold patches. ``honeycomb``
    and ``sweagent`` each evaluate half of the live tasks (non-overlapping) so
    denominators differ even when raw pass counts are similar.
    """
    n = len(LIVE_TASKS)
    half = n // 2
    if agent_id == "gru":
        return list(LIVE_TASKS)
    if agent_id == "honeycomb":
        return list(LIVE_TASKS[:half])
    if agent_id == "sweagent":
        return list(LIVE_TASKS[half:])
    raise AssertionError(agent_id)


def _live_patch_for_agent(agent_id: str, gold: str, test_patch: str) -> tuple[str, str]:
    # Honeycomb and sweagent stress ``test_patch`` on their respective live
    # slices; gru stays on gold for the comparative static-vs-live story.
    if agent_id == "gru":
        return gold, "dataset_patch"
    if agent_id in ("honeycomb", "sweagent"):
        return test_patch, "dataset_test_patch_slice"
    raise AssertionError(agent_id)


def main() -> int:  # noqa: PLR0915
    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    PATCHES_DIR.mkdir(parents=True, exist_ok=True)
    CANDIDATES_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACES_DIR.mkdir(parents=True, exist_ok=True)

    live_cache = _load_jsonl_by_task(LIVE_CACHE)
    verified_cache = _load_jsonl_by_task(VERIFIED_CACHE)

    panel_lines: list[str] = []
    provenance_entries: list[dict[str, Any]] = []

    def add_live_bundle(agent: Agent, task_id: str) -> None:
        manifest_path = LIVE_MANIFEST_DIR / f"{task_id}.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        workspace_template = _materialize_workspace(
            task_id=task_id,
            repo_name=manifest["repo_name"],
            base_commit=manifest["base_commit"],
        )
        cache_row = live_cache[task_id]
        gold_patch = cache_row["patch"]
        test_patch = cache_row["test_patch"]
        patch_text, source = _live_patch_for_agent(agent.agent_id, gold_patch, test_patch)
        patch_bytes = patch_text.encode("utf-8")
        patch_sha = hashlib.sha256(patch_bytes).hexdigest()

        patch_rel = Path("patches") / agent.agent_id / f"{task_id}.diff"
        patch_path = OUT_ROOT / patch_rel
        patch_path.parent.mkdir(parents=True, exist_ok=True)
        patch_path.write_bytes(patch_bytes)

        candidate_rel = Path("candidates") / agent.agent_id / f"{task_id}.json"
        candidate_path = OUT_ROOT / candidate_rel
        _write_candidate(
            agent=agent,
            benchmark_id="swe_bench_live",
            task_id=task_id,
            patch_ref=str(patch_rel).replace("\\", "/"),
            patch_sha=patch_sha,
            source=source,
            out_path=candidate_path,
        )

        entry = {
            "task": _rel_from_panel(manifest_path),
            "candidate": str(candidate_rel).replace("\\", "/"),
            "patch": str(patch_rel).replace("\\", "/"),
            "workspace_template": workspace_template,
            "bundle_name": f"{agent.agent_id}__{task_id}",
            "entry_id": f"{agent.agent_id}__{task_id}",
        }
        panel_lines.append(json.dumps(entry, sort_keys=True))
        provenance_entries.append(
            {
                "agent_id": agent.agent_id,
                "benchmark_id": "swe_bench_live",
                "task_id": task_id,
                "patch_source": source,
                "patch_sha256": patch_sha,
            }
        )

    def add_verified_bundle(agent: Agent, task_id: str) -> None:
        manifest_path = VERIFIED_MANIFEST_DIR / f"{task_id}.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        workspace_template = _materialize_workspace(
            task_id=task_id,
            repo_name=manifest["repo_name"],
            base_commit=manifest["base_commit"],
        )
        cache_row = verified_cache[task_id]
        gold_patch = cache_row["patch"]
        patch_bytes = gold_patch.encode("utf-8")
        patch_sha = hashlib.sha256(patch_bytes).hexdigest()
        source = "dataset_patch"

        patch_rel = Path("patches") / agent.agent_id / f"{task_id}.diff"
        patch_path = OUT_ROOT / patch_rel
        patch_path.parent.mkdir(parents=True, exist_ok=True)
        patch_path.write_bytes(patch_bytes)

        candidate_rel = Path("candidates") / agent.agent_id / f"{task_id}.json"
        candidate_path = OUT_ROOT / candidate_rel
        _write_candidate(
            agent=agent,
            benchmark_id="swe_bench_verified",
            task_id=task_id,
            patch_ref=str(patch_rel).replace("\\", "/"),
            patch_sha=patch_sha,
            source=source,
            out_path=candidate_path,
        )

        entry = {
            "task": _rel_from_panel(manifest_path),
            "candidate": str(candidate_rel).replace("\\", "/"),
            "patch": str(patch_rel).replace("\\", "/"),
            "workspace_template": workspace_template,
            "bundle_name": f"{agent.agent_id}__{task_id}",
            "entry_id": f"{agent.agent_id}__{task_id}",
        }
        panel_lines.append(json.dumps(entry, sort_keys=True))
        provenance_entries.append(
            {
                "agent_id": agent.agent_id,
                "benchmark_id": "swe_bench_verified",
                "task_id": task_id,
                "patch_source": source,
                "patch_sha256": patch_sha,
            }
        )

    for agent in AGENTS:
        for task_id in _live_tasks_for_agent(agent.agent_id):
            add_live_bundle(agent, task_id)
    for task_id in VERIFIED_TASKS:
        for agent in AGENTS:
            add_verified_bundle(agent, task_id)

    panel_lines.sort()
    PANEL_PATH.write_text("\n".join(panel_lines) + "\n", encoding="utf-8")

    provenance = {
        "panel_id": "live_panel_v2",
        "schema_version": 1,
        "agents": [a.__dict__ for a in AGENTS],
        "live_tasks": LIVE_TASKS,
        "live_task_slices": {
            "gru": _live_tasks_for_agent("gru"),
            "honeycomb": _live_tasks_for_agent("honeycomb"),
            "sweagent": _live_tasks_for_agent("sweagent"),
        },
        "verified_anchor_tasks": VERIFIED_TASKS,
        "entry_count": len(panel_lines),
        "entries": provenance_entries,
        "notes": [
            (
                "Disjoint live task subsets per agent (8 / 4 / 4 rows) plus "
                "gold-vs-test_patch choices so pooled live pass rates can diverge."
            ),
            "Verified anchors remain gold ``patch`` for every agent.",
        ],
    }
    PROVENANCE_PATH.write_text(
        json.dumps(provenance, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {len(panel_lines)} panel entries to {PANEL_PATH.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
