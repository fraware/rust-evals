"""Bootstrap ``runs/released/rust_proof_subset_v1/`` for L4 batch design.

Materialises **every** ``task_id`` listed in ``datasets/derived/proof_subset/manifest.jsonl``
as a golden-agent row: shallow clone at ``base_commit``, strip ``.git``, fetch the
merged PR unified diff from GitHub, and emit ``panel.jsonl`` + ``CandidateResolution``
JSON mirroring ``build_rust_pilot.py``.

Use this panel for end-to-end ``evaluate batch`` runs with::

    --levels L0,L1,L3,L4 \\
    --obligations datasets/derived/proof_subset/manifest.jsonl \\
    --policy configs/policy/rust_pilot.toml \\
    --lean-root packages/lean/EvalLadder

Expect long wall times: ``cargo test --workspace --locked`` over ripgrep and
multiple clap snapshots is heavy; raise ``--timeout-secs`` accordingly.
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
DEFAULT_OUT = REPO_ROOT / "runs" / "released" / "rust_proof_subset_v1"
RUST_MANIFESTS = REPO_ROOT / "benchmarks" / "rust" / "manifests"
RUST_CACHE = REPO_ROOT / "datasets" / "cache" / "rust" / "multi_swe_bench_rust.jsonl"
PROOF_MANIFEST = REPO_ROOT / "datasets" / "derived" / "proof_subset" / "manifest.jsonl"

NAMESPACE_RUST_PROOF_SUBSET_V1 = uuid.UUID("c4d9e1a2-7f3b-5c6d-8e9f-0a1b2c3d4e5f")

AGENT_ID = "golden_agent"
MODEL_ID = "upstream_pr_diff"


def _task_ids_from_proof_manifest(path: Path) -> list[str]:
    out: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        t = line.strip()
        if not t or t.startswith("#"):
            continue
        row = json.loads(t)
        out.append(row["task_id"])
    out.sort()
    return out


def _deterministic_uuid(*parts: str) -> str:
    return str(uuid.uuid5(NAMESPACE_RUST_PROOF_SUBSET_V1, "|".join(parts)))


def _fetch_pr_diff(repo: str, pull_number: int) -> bytes:
    url = f"https://github.com/{repo}/pull/{pull_number}.diff"
    with httpx.Client(follow_redirects=True) as client:
        resp = client.get(url, timeout=120)
    resp.raise_for_status()
    return resp.content


def _raw_record(task_id: str) -> dict:
    with RUST_CACHE.open("r", encoding="utf-8") as fh:
        for line in fh:
            r = json.loads(line)
            if r.get("instance_id") == task_id:
                return r
    raise SystemExit(f"task {task_id} not found in {RUST_CACHE}")


def _materialize_symlinks(root: Path) -> int:
    """Replace symlinks under `root` with concrete copies of their targets."""
    links: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        base = Path(dirpath)
        for name in dirnames + filenames:
            p = base / name
            if p.is_symlink():
                links.append(p)

    for link in links:
        try:
            raw_target = os.readlink(link)
        except OSError as exc:
            raise SystemExit(f"failed to read symlink target for {link}: {exc}") from exc
        if os.path.isabs(raw_target):
            target = Path(raw_target)
        else:
            target = (link.parent / raw_target).resolve()
        if not target.exists():
            raise SystemExit(f"symlink target does not exist for {link}: {target}")

        link.unlink()
        if target.is_dir():
            shutil.copytree(target, link)
        else:
            shutil.copy2(target, link)

    return len(links)


def _prepare_workspace(workspaces_dir: Path, task_id: str, repo: str, base_commit: str) -> Path:
    dest = workspaces_dir / task_id
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
    materialized_links = _materialize_symlinks(dest)
    if materialized_links:
        print(f"[{task_id}] materialized {materialized_links} symlink(s) into plain files/dirs")
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
            "random_seed": 0,
            "temperature": 0.0,
        },
        "submitted_at": submitted_at,
    }


def _panel_entry(
    *,
    panel_root: Path,
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
        "candidate": str(candidate_path.relative_to(panel_root)).replace("\\", "/"),
        "patch": str(patch_path.relative_to(panel_root)).replace("\\", "/"),
        "workspace_template": str(workspace.relative_to(panel_root)).replace("\\", "/"),
        "bundle_name": f"{AGENT_ID}__{task_id}",
        "entry_id": f"{AGENT_ID}__{task_id}",
    }


def build_panel(*, panel_root: Path, proof_manifest: Path) -> int:
    candidates_dir = panel_root / "candidates"
    patches_dir = panel_root / "patches"
    workspaces_dir = panel_root / "workspaces"
    panel_file = panel_root / "panel.jsonl"

    candidates_dir.mkdir(parents=True, exist_ok=True)
    patches_dir.mkdir(parents=True, exist_ok=True)
    workspaces_dir.mkdir(parents=True, exist_ok=True)

    submitted_at = _dt.datetime(2026, 4, 23, 0, 0, 0, tzinfo=_dt.timezone.utc).isoformat().replace(
        "+00:00", "Z"
    )

    task_ids = _task_ids_from_proof_manifest(proof_manifest)
    if not task_ids:
        raise SystemExit(f"no task ids in {proof_manifest}")

    provenance_entries: list[dict] = []
    panel_lines: list[str] = []

    for task_id in task_ids:
        task_manifest = RUST_MANIFESTS / f"{task_id}.json"
        if not task_manifest.exists():
            raise SystemExit(f"missing manifest {task_manifest}")

        raw = _raw_record(task_id)
        repo = raw["repo"]
        base_commit = raw["base_commit"]
        pull_number = int(raw["pull_number"])

        patch_bytes = _fetch_pr_diff(repo, pull_number)
        patch_sha = hashlib.sha256(patch_bytes).hexdigest()
        patch_path = patches_dir / AGENT_ID / f"{task_id}.diff"
        patch_path.parent.mkdir(parents=True, exist_ok=True)
        patch_path.write_bytes(patch_bytes)

        print(f"[{task_id}] cloning {repo}@{base_commit[:10]} ...")
        workspace = _prepare_workspace(workspaces_dir, task_id, repo, base_commit)
        print(f"[{task_id}] workspace ready: {workspace.relative_to(REPO_ROOT)}")

        candidate = _candidate_json(
            task_id=task_id,
            patch_ref=str(patch_path.relative_to(panel_root)).replace("\\", "/"),
            patch_sha256=patch_sha,
            submitted_at=submitted_at,
        )
        candidate_path = candidates_dir / AGENT_ID / f"{task_id}.json"
        candidate_path.parent.mkdir(parents=True, exist_ok=True)
        candidate_path.write_text(
            json.dumps(candidate, sort_keys=True, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

        entry = _panel_entry(
            panel_root=panel_root,
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
    panel_file.write_text("\n".join(panel_lines) + "\n", encoding="utf-8")

    provenance = {
        "panel_id": "rust_proof_subset_v1",
        "proof_subset_manifest": str(proof_manifest.relative_to(REPO_ROOT)).replace("\\", "/"),
        "agent_id": AGENT_ID,
        "model_id": MODEL_ID,
        "entries": provenance_entries,
        "notes": [
            "UUIDv5 namespace c4d9e1a2-7f3b-5c6d-8e9f-0a1b2c3d4e5f (distinct from rust_pilot_v1).",
            "Task list is derived from datasets/derived/proof_subset/manifest.jsonl.",
            (
                "Use configs/policy/rust_proof_subset.toml for L3 edit-scope parity "
                "on src/tests/examples edits."
            ),
            (
                "Reviewer-facing Lean sketch fidelity notes: "
                "docs/proof_subset_policy.md (Lean sketch fidelity table)."
            ),
        ],
    }
    (panel_root / "provenance.json").write_text(
        json.dumps(provenance, sort_keys=True, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return len(panel_lines)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_OUT,
        help=f"output directory (default: {DEFAULT_OUT})",
    )
    ap.add_argument(
        "--proof-manifest",
        type=Path,
        default=PROOF_MANIFEST,
        help="JSONL obligation manifest whose task_id rows drive the panel",
    )
    args = ap.parse_args()
    root = args.out.resolve()
    proof = args.proof_manifest.resolve()
    n = build_panel(panel_root=root, proof_manifest=proof)
    print(f"wrote {n} entries to {(root / 'panel.jsonl').relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
