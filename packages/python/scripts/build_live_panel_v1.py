"""Build ``runs/released/live_panel_v1/`` with real Live + static anchors.

This builder creates a reproducible panel with the same 3-agent family used in
``agent_panel_v1``:

- gru
- honeycomb
- sweagent

Panel composition:
- 8 SWE-bench-Live tasks (real benchmark arm).
- 5 SWE-bench-Verified tasks (static anchors for static_vs_live export).

Patch strategy:
- gru/honeycomb: use dataset gold patch (`patch`) on both benchmark arms.
- sweagent: use `patch` on static anchors, but `test_patch` on live tasks to
  create a measurable live-vs-static delta in end-to-end runs.

All candidate IDs are deterministic UUIDv5 over:
``agent_id | benchmark_id | task_id | patch_sha256``.
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

REPO_ROOT = Path(__file__).resolve().parents[3]
OUT_ROOT = REPO_ROOT / "runs" / "released" / "live_panel_v1"
PATCHES_DIR = OUT_ROOT / "patches"
CANDIDATES_DIR = OUT_ROOT / "candidates"
WORKSPACES_DIR = OUT_ROOT / "workspaces"
PANEL_PATH = OUT_ROOT / "panel.jsonl"
PROVENANCE_PATH = OUT_ROOT / "provenance.json"

LIVE_MANIFEST_DIR = REPO_ROOT / "benchmarks" / "live" / "manifests"
VERIFIED_MANIFEST_DIR = REPO_ROOT / "benchmarks" / "verified" / "manifests"
LIVE_CACHE = REPO_ROOT / "datasets" / "cache" / "live" / "swe_bench_live_verified.jsonl"
VERIFIED_CACHE = REPO_ROOT / "datasets" / "cache" / "verified" / "swe_bench_verified.jsonl"

NAMESPACE = uuid.UUID("b6a4ce30-8715-5e83-9aca-3dd0b0a71e71")
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

# Keep task sets pinned and reviewable.
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


def _force_writable(func, path, _exc):
    p = Path(path)
    p.chmod(stat.S_IWRITE)
    func(path)


def _robust_rmtree(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path, onerror=_force_writable)


def _load_jsonl_by_task(path: Path) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line:
            continue
        obj = json.loads(line)
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

    tmp = Path(Path.cwd().joinpath(f".tmp-live-panel-{task_id.replace('/', '_')}"))
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
    # panel is at runs/released/live_panel_v1/panel.jsonl
    rel = Path("..") / ".." / ".." / path.relative_to(REPO_ROOT)
    return str(rel).replace("\\", "/")


def main() -> int:
    OUT_ROOT.mkdir(parents=True, exist_ok=True)
    PATCHES_DIR.mkdir(parents=True, exist_ok=True)
    CANDIDATES_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACES_DIR.mkdir(parents=True, exist_ok=True)

    live_cache = _load_jsonl_by_task(LIVE_CACHE)
    verified_cache = _load_jsonl_by_task(VERIFIED_CACHE)

    panel_lines: list[str] = []
    provenance_entries: list[dict] = []

    def add_entry(
        task_id: str,
        benchmark_id: str,
        manifest_dir: Path,
        cache_row: dict,
        is_live: bool,
    ) -> None:
        manifest_path = manifest_dir / f"{task_id}.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        workspace_template = _materialize_workspace(
            task_id=task_id,
            repo_name=manifest["repo_name"],
            base_commit=manifest["base_commit"],
        )

        gold_patch = cache_row["patch"]
        test_patch = cache_row["test_patch"]
        for agent in AGENTS:
            patch_text = gold_patch
            source = "dataset_patch"
            if is_live and agent.agent_id == "sweagent":
                patch_text = test_patch
                source = "dataset_test_patch"
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
                benchmark_id=benchmark_id,
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
                    "benchmark_id": benchmark_id,
                    "task_id": task_id,
                    "patch_source": source,
                    "patch_sha256": patch_sha,
                }
            )

    for task_id in LIVE_TASKS:
        add_entry(
            task_id=task_id,
            benchmark_id="swe_bench_live",
            manifest_dir=LIVE_MANIFEST_DIR,
            cache_row=live_cache[task_id],
            is_live=True,
        )
    for task_id in VERIFIED_TASKS:
        add_entry(
            task_id=task_id,
            benchmark_id="swe_bench_verified",
            manifest_dir=VERIFIED_MANIFEST_DIR,
            cache_row=verified_cache[task_id],
            is_live=False,
        )

    panel_lines.sort()
    PANEL_PATH.write_text("\n".join(panel_lines) + "\n", encoding="utf-8")

    provenance = {
        "panel_id": "live_panel_v1",
        "schema_version": 1,
        "agents": [a.__dict__ for a in AGENTS],
        "live_tasks": LIVE_TASKS,
        "verified_anchor_tasks": VERIFIED_TASKS,
        "entry_count": len(panel_lines),
        "entries": provenance_entries,
        "notes": [
            "sweagent uses dataset test_patch on live tasks to stress live-vs-static drift.",
            "gru and honeycomb use dataset patch on both benchmark arms.",
            "workspaces are pinned to manifest repo_name/base_commit checkouts with .git removed.",
        ],
    }
    PROVENANCE_PATH.write_text(
        json.dumps(provenance, sort_keys=True, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {len(panel_lines)} panel entries to {PANEL_PATH.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

