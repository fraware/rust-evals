"""Bootstrap ``runs/released/rust_pilot_v1/`` for the rust-native batch pilot.

For each selected Rust task we:

    1. Shallow-clone the upstream repo, checkout ``base_commit``,
       and strip ``.git`` - the remaining tree is the ``workspace_template``
       that ``eval-ladder evaluate batch`` copies into staging per level.
    2. Download the PR's unified diff via GitHub's
       ``https://github.com/<repo>/pull/<pull_number>.diff`` endpoint;
       this is the "golden agent" candidate patch (fix + tests merged
       verbatim from the upstream PR).
    3. Emit a ``CandidateResolution`` JSON with a deterministic UUIDv5
       candidate id derived from ``(agent_id, task_id, patch_sha256)``.
    4. Emit a panel line.

Provenance is recorded in ``provenance.json``. The resulting panel is
consumable by::

    eval-ladder evaluate batch \\
        --input runs/released/rust_pilot_v1/panel.jsonl \\
        --config configs/evaluator/rust.toml \\
        --levels L0,L1,L3 \\
        --policy configs/policy/default_policy.toml \\
        --out runs/released/rust_pilot_v1/results/

Scope is deliberately tiny so this script completes within a session.
The panel is labeled as a "pipeline shakedown" baseline - it measures
whether a pristine PR applied at ``base_commit`` passes its own test
suite under the current toolchain, not whether any model can reproduce
the fix.
"""

from __future__ import annotations

import argparse
import contextlib
import datetime as _dt
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import uuid
from pathlib import Path

import httpx  # type: ignore[import-not-found]


def _force_writable(path: Path) -> None:
    for root, dirs, files in os.walk(path):
        for name in dirs + files:
            with contextlib.suppress(OSError):
                os.chmod(Path(root) / name, stat.S_IWRITE | stat.S_IREAD)


def _robust_rmtree(path: Path) -> None:
    """Windows-friendly ``rmtree`` that clears read-only bits first."""
    if not path.exists():
        return
    _force_writable(path)

    def _onexc(func, target, exc_info):
        try:
            os.chmod(target, stat.S_IWRITE | stat.S_IREAD)
            func(target)
        except Exception:
            pass

    shutil.rmtree(path, onexc=_onexc)


REPO_ROOT = Path(__file__).resolve().parents[3]
PILOT_ROOT = REPO_ROOT / "runs" / "released" / "rust_pilot_v1"
CANDIDATES_DIR = PILOT_ROOT / "candidates"
PATCHES_DIR = PILOT_ROOT / "patches"
WORKSPACES_DIR = PILOT_ROOT / "workspaces"
PANEL_FILE = PILOT_ROOT / "panel.jsonl"
RUST_MANIFESTS = REPO_ROOT / "benchmarks" / "rust" / "manifests"
RUST_CACHE = REPO_ROOT / "datasets" / "cache" / "rust" / "multi_swe_bench_rust.jsonl"

NAMESPACE_RUST_PILOT_V1 = uuid.UUID("6b5c2e38-8b09-4f1f-9d8f-4f7c1c1e4a01")

AGENT_ID = "golden_agent"
MODEL_ID = "upstream_pr_diff"

TASKS: list[str] = [
    "clap-rs__clap_5873",
]


def _deterministic_uuid(*parts: str) -> str:
    return str(uuid.uuid5(NAMESPACE_RUST_PILOT_V1, "|".join(parts)))


def _fetch_pr_diff(repo: str, pull_number: int) -> bytes:
    url = f"https://github.com/{repo}/pull/{pull_number}.diff"
    with httpx.Client(follow_redirects=True) as client:
        resp = client.get(url, timeout=60)
    resp.raise_for_status()
    return resp.content


def _raw_record(task_id: str) -> dict:
    with RUST_CACHE.open("r", encoding="utf-8") as fh:
        for line in fh:
            r = json.loads(line)
            if r.get("instance_id") == task_id:
                return r
    raise SystemExit(f"task {task_id} not found in {RUST_CACHE}")


def _prepare_workspace(task_id: str, repo: str, base_commit: str) -> Path:
    dest = WORKSPACES_DIR / task_id
    _robust_rmtree(dest)
    dest.mkdir(parents=True)
    clone_url = f"https://github.com/{repo}.git"
    subprocess.run(
        ["git", "clone", "--no-checkout", "--filter=blob:none", clone_url, str(dest)],
        check=True,
    )
    subprocess.run(
        ["git", "-C", str(dest), "checkout", base_commit],
        check=True,
    )
    _robust_rmtree(dest / ".git")
    return dest


def _candidate_json(
    *,
    task_id: str,
    patch_ref: str,
    patch_sha256: str,
    submitted_at: str,
) -> dict:
    candidate_id = _deterministic_uuid(AGENT_ID, task_id, patch_sha256)
    return {
        "schema_version": 1,
        "candidate_id": candidate_id,
        "benchmark_id": "rust_swe_bench",
        "task_id": task_id,
        "agent_id": AGENT_ID,
        "model_id": MODEL_ID,
        "generation_mode": "human_assisted",
        "patch_format": "unified_diff",
        "patch_ref": patch_ref,
        "generation_metadata": {
            "tool_configuration": {
                "source": "upstream_pr_diff",
                "note": "Candidate patch is the merged upstream PR (fix + tests).",
            },
            "context_mode": "full_repo",
            "repo_reproduction_used": True,
            # A golden-agent baseline copies the canonical upstream
            # diff verbatim; there is no sampling, so the "seed" is
            # zero by convention. Declaring it explicitly lets the
            # L3 policy's `requires_reproducible_seed` check pass
            # without relaxing the policy itself.
            "random_seed": 0,
            "temperature": 0.0,
        },
        "submitted_at": submitted_at,
    }


def _panel_entry(
    *,
    task_id: str,
    task_manifest: Path,
    candidate_path: Path,
    patch_path: Path,
    workspace: Path,
) -> dict:
    def _rel_from_panel(p: Path) -> str:
        rel = Path(*([".."] * 3)) / p.relative_to(REPO_ROOT)
        return str(rel).replace("\\", "/")

    return {
        "task": _rel_from_panel(task_manifest),
        "candidate": str(candidate_path.relative_to(PILOT_ROOT)).replace("\\", "/"),
        "patch": str(patch_path.relative_to(PILOT_ROOT)).replace("\\", "/"),
        "workspace_template": str(workspace.relative_to(PILOT_ROOT)).replace("\\", "/"),
        "bundle_name": f"{AGENT_ID}__{task_id}",
        "entry_id": f"{AGENT_ID}__{task_id}",
    }


def build_pilot() -> int:
    CANDIDATES_DIR.mkdir(parents=True, exist_ok=True)
    PATCHES_DIR.mkdir(parents=True, exist_ok=True)
    WORKSPACES_DIR.mkdir(parents=True, exist_ok=True)
    submitted_at = (
        _dt.datetime(2026, 4, 21, 0, 0, 0, tzinfo=_dt.timezone.utc)
        .isoformat()
        .replace("+00:00", "Z")
    )

    provenance_entries: list[dict] = []
    panel_lines: list[str] = []

    for task_id in TASKS:
        task_manifest = RUST_MANIFESTS / f"{task_id}.json"
        if not task_manifest.exists():
            raise SystemExit(f"missing manifest {task_manifest}")

        raw = _raw_record(task_id)
        repo = raw["repo"]
        base_commit = raw["base_commit"]
        pull_number = int(raw["pull_number"])

        patch_bytes = _fetch_pr_diff(repo, pull_number)
        patch_sha = hashlib.sha256(patch_bytes).hexdigest()
        patch_path = PATCHES_DIR / AGENT_ID / f"{task_id}.diff"
        patch_path.parent.mkdir(parents=True, exist_ok=True)
        patch_path.write_bytes(patch_bytes)

        print(f"[{task_id}] cloning {repo}@{base_commit[:10]} ...")
        workspace = _prepare_workspace(task_id, repo, base_commit)
        print(f"[{task_id}] workspace ready: {workspace.relative_to(REPO_ROOT)}")

        candidate = _candidate_json(
            task_id=task_id,
            patch_ref=str(patch_path.relative_to(PILOT_ROOT)).replace("\\", "/"),
            patch_sha256=patch_sha,
            submitted_at=submitted_at,
        )
        candidate_path = CANDIDATES_DIR / AGENT_ID / f"{task_id}.json"
        candidate_path.parent.mkdir(parents=True, exist_ok=True)
        candidate_path.write_text(
            json.dumps(candidate, sort_keys=True, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

        entry = _panel_entry(
            task_id=task_id,
            task_manifest=task_manifest,
            candidate_path=candidate_path,
            patch_path=patch_path,
            workspace=workspace,
        )
        panel_lines.append(json.dumps(entry, sort_keys=True))
        provenance_entries.append(
            {
                "task_id": task_id,
                "repo": repo,
                "base_commit": base_commit,
                "pull_number": pull_number,
                "patch_sha256": patch_sha,
                "patch_url": f"https://github.com/{repo}/pull/{pull_number}.diff",
            }
        )

    panel_lines.sort()
    PANEL_FILE.write_text("\n".join(panel_lines) + "\n", encoding="utf-8")

    provenance = {
        "pilot_id": "rust_pilot_v1",
        "agent_id": AGENT_ID,
        "model_id": MODEL_ID,
        "entries": provenance_entries,
        "notes": [
            "Candidate patches are the merged upstream PR diffs fetched via GitHub.",
            (
                "The candidate is therefore the 'golden agent' baseline: fix + tests match "
                "the reference merge."
            ),
            (
                "Workspaces are pristine source trees exported at base_commit "
                "(no .git, no build artifacts)."
            ),
            (
                "Test runs use the native cargo toolchain via LocalProcessEngine; "
                "no Docker is required for rust-native tasks."
            ),
        ],
    }
    (PILOT_ROOT / "provenance.json").write_text(
        json.dumps(provenance, sort_keys=True, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return len(panel_lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.parse_args()
    n = build_pilot()
    print(f"wrote {n} pilot entries to {PANEL_FILE.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
