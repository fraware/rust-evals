"""Materialise ``runs/released/agent_panel_verified_flagship_v1/`` from v3_r1.

The flagship panel is a **high-precision cleanup** of
``agent_panel_v3_r1/panel_preflight_clean.jsonl``:

- Drops tasks whose instance ids are dominated by native-build / harness
  fragility in the sealed v3_r1 batch (matplotlib, scikit-learn, pytest-dev).
- Copies patches, candidates, and per-task workspaces from v3_r1 so the
  directory is self-contained (no live network fetch).

After materialising, run the optimized Verified batch with
``configs/evaluator/verified_headline.toml`` so L3 is not inflated by the
narrow ``src/lib/tests`` allow-list (see ``configs/policy/swe_bench_verified_headline.toml``).
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
SRC_PANEL_ROOT = REPO_ROOT / "runs" / "released" / "agent_panel_v3_r1"
DEFAULT_OUT = REPO_ROOT / "runs" / "released" / "agent_panel_verified_flagship_v1"

# Task-id prefixes removed from the flagship slice (see triage on v3_r1).
EXCLUDED_PREFIXES: tuple[str, ...] = (
    "matplotlib__",
    "scikit-learn__",
    "pytest-dev__",
)


def _task_id_from_bundle(bundle_name: str) -> str:
    parts = bundle_name.split("__", 1)
    if len(parts) != 2:
        raise ValueError(f"unexpected bundle_name: {bundle_name}")
    return parts[1]


def _should_drop(task_id: str) -> bool:
    return any(task_id.startswith(p) for p in EXCLUDED_PREFIXES)


def _collect_kept_rows(panel_src: Path) -> tuple[list[str], set[str], int]:
    kept_lines: list[str] = []
    task_ids: set[str] = set()
    dropped = 0
    for raw in panel_src.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line:
            continue
        row = json.loads(line)
        bundle = str(row["bundle_name"])
        tid = _task_id_from_bundle(bundle)
        if _should_drop(tid):
            dropped += 1
            continue
        row["workspace_template"] = f"../agent_panel_v3_r1/workspaces/{tid}/"
        kept_lines.append(json.dumps(row, sort_keys=True))
        task_ids.add(tid)
    kept_lines.sort()
    return kept_lines, task_ids, dropped


def _copy_agent_tree(
    src_root: Path, dst_root: Path, task_ids: set[str], suffix: str
) -> None:
    for agent_dir in (src_root).iterdir():
        if not agent_dir.is_dir():
            continue
        for path in agent_dir.glob(f"*{suffix}"):
            tid = path.stem
            if _should_drop(tid) or tid not in task_ids:
                continue
            dest_dir = dst_root / agent_dir.name
            dest_dir.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, dest_dir / path.name)


def materialise(out_root: Path) -> int:
    panel_src = SRC_PANEL_ROOT / "panel_preflight_clean.jsonl"
    if not panel_src.is_file():
        raise SystemExit(f"missing source panel: {panel_src}")

    out_root.mkdir(parents=True, exist_ok=True)
    candidates_dst = out_root / "candidates"
    patches_dst = out_root / "patches"
    if candidates_dst.exists():
        shutil.rmtree(candidates_dst)
    if patches_dst.exists():
        shutil.rmtree(patches_dst)
    candidates_dst.mkdir(parents=True, exist_ok=True)
    patches_dst.mkdir(parents=True, exist_ok=True)

    kept_lines, task_ids, dropped = _collect_kept_rows(panel_src)
    (out_root / "panel.jsonl").write_text("\n".join(kept_lines) + "\n", encoding="utf-8")

    _copy_agent_tree(SRC_PANEL_ROOT / "candidates", candidates_dst, task_ids, ".json")
    _copy_agent_tree(SRC_PANEL_ROOT / "patches", patches_dst, task_ids, ".diff")

    provenance = {
        "panel_id": "agent_panel_verified_flagship_v1",
        "schema_version": 1,
        "source_panel_root": str(SRC_PANEL_ROOT.relative_to(REPO_ROOT)).replace("\\", "/"),
        "excluded_task_prefixes": list(EXCLUDED_PREFIXES),
        "dropped_panel_lines": dropped,
        "kept_panel_lines": len(kept_lines),
        "unique_tasks": len(task_ids),
        "notes": [
            (
                "Patches and candidates are copied from v3_r1; workspaces are reused "
                "in-place via ../agent_panel_v3_r1/workspaces/<task_id>/ "
                "(no duplicate checkouts)."
            ),
            "Evaluate with configs/evaluator/verified_headline.toml (see README).",
        ],
    }
    (out_root / "provenance.json").write_text(
        json.dumps(provenance, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return len(kept_lines)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT, help="output directory")
    args = ap.parse_args(argv)
    n = materialise(args.out.resolve())
    rel = (args.out / "panel.jsonl").resolve().relative_to(REPO_ROOT)
    print(f"wrote {n} panel lines to {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
