#!/usr/bin/env python3
"""Emit L2 flagship selection manifest (CSV + JSON) from sealed summaries."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_SUMMARY = (
    REPO_ROOT
    / "runs/released/l2_verified_flagship_v1/results/batch_summary.json"
)
FLAGSHIP_RUN_ROOT = REPO_ROOT / "runs/released/l2_verified_flagship_v1"
DEFAULT_GOLD_CSV = (
    REPO_ROOT
    / "paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv"
)
OUT_DIR = REPO_ROOT / "paper/exports/l2_verified_flagship_v1"
OUT_CSV = OUT_DIR / "l2_selection_manifest.csv"
OUT_JSON = OUT_DIR / "l2_selection_manifest.json"


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be JSON object")
    return data


def _load_gold_lookup(path: Path) -> dict[tuple[str, str], str]:
    """Map (task_id, semantic_family) -> gold L2 status."""
    lookup: dict[tuple[str, str], str] = {}
    if not path.is_file():
        return lookup
    with path.open(encoding="utf-8", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            tid = row.get("task_id", "")
            vf = row.get("validator_family", "")
            st = row.get("gold_patch_status_L2", "")
            if tid and vf:
                lookup[(tid, vf)] = st
    return lookup


def _semantic_family(entry_id: str) -> str:
    if entry_id.endswith("__astropy"):
        return "augmented_unit_tests"
    if entry_id.endswith("__regressionfail"):
        return "targeted_regression"
    return "unknown"


def _bundle_rel_path(entry_id: str, bundle_name: str) -> Path:
    if entry_id.endswith("__astropy"):
        return FLAGSHIP_RUN_ROOT / "results_astropy" / bundle_name
    if entry_id.endswith("__regressionfail"):
        return FLAGSHIP_RUN_ROOT / "results_regression_fail" / bundle_name
    return FLAGSHIP_RUN_ROOT / "results" / bundle_name


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--summary", type=Path, default=DEFAULT_SUMMARY)
    parser.add_argument("--gold-csv", type=Path, default=DEFAULT_GOLD_CSV)
    parser.add_argument("--out-dir", type=Path, default=OUT_DIR)
    args = parser.parse_args()
    summary_path = args.summary.resolve()
    gold_csv = args.gold_csv.resolve()
    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    summary = _load_json(summary_path)
    gold_l2 = _load_gold_lookup(gold_csv)

    rows: list[dict[str, Any]] = []
    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        entry_id = str(entry.get("entry_id", ""))
        bundle_name = str(entry.get("bundle_name", ""))
        task_path = Path(str(entry.get("task_path", "")))
        task_id = task_path.stem
        aid = entry_id.split("__", 1)[0] if "__" in entry_id else ""
        levels = entry.get("levels", {})
        if not isinstance(levels, dict):
            levels = {}
        l1 = levels.get("l1", {}) if isinstance(levels.get("l1"), dict) else {}
        l2 = levels.get("l2", {}) if isinstance(levels.get("l2"), dict) else {}
        vf = _semantic_family(entry_id)
        bundle_path = _bundle_rel_path(entry_id, bundle_name).relative_to(REPO_ROOT)
        cid = ""
        cand = entry.get("candidate_path")
        if isinstance(cand, str) and cand:
            p = Path(cand)
            try:
                cj = json.loads(p.read_text(encoding="utf-8"))
                if isinstance(cj, dict):
                    cid = str(cj.get("candidate_id", ""))
            except (OSError, json.JSONDecodeError):
                cid = ""

        gp = gold_l2.get((task_id, vf), "")

        rows.append(
            {
                "task_id": task_id,
                "candidate_id": cid,
                "agent_id": aid,
                "validator_family": vf,
                "selection_status": "selected",
                "selection_reason": (
                    "Deterministic twin-family expansion from "
                    "agent_panel_verified_flagship_v1 base rows (fixed before L2)."
                ),
                "exclusion_reason": "",
                "selected_before_l2": "true",
                "gold_patch_available": "true",
                "gold_patch_l2_status": gp,
                "candidate_l1_status": str(l1.get("status", "")),
                "candidate_l2_status": str(l2.get("status", "")),
                "candidate_l2_reason": str(l2.get("primary_reason", "")),
                "bundle_path": str(bundle_path).replace("\\", "/"),
            }
        )

    rows.sort(key=lambda r: (r["task_id"], r["validator_family"], r["agent_id"]))

    fields = [
        "task_id",
        "candidate_id",
        "agent_id",
        "validator_family",
        "selection_status",
        "selection_reason",
        "exclusion_reason",
        "selected_before_l2",
        "gold_patch_available",
        "gold_patch_l2_status",
        "candidate_l1_status",
        "candidate_l2_status",
        "candidate_l2_reason",
        "bundle_path",
    ]
    csv_path = out_dir / "l2_selection_manifest.csv"
    with csv_path.open("w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for row in rows:
            w.writerow({k: row.get(k, "") for k in fields})

    payload = {
        "schema_note": (
            "One row per merged batch_summary entry; gold_patch_l2_status "
            "joined from gold_patch_validation.csv when present."
        ),
        "source_summary": str(summary_path.relative_to(REPO_ROOT)).replace("\\", "/"),
        "gold_patch_validation_csv": str(gold_csv.relative_to(REPO_ROOT)).replace(
            "\\", "/"
        ),
        "rows": rows,
    }
    json_path = out_dir / "l2_selection_manifest.json"
    json_path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({"csv": str(csv_path), "json": str(json_path)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
