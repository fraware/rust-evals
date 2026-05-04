#!/usr/bin/env python3
"""Regenerate a sealed-export numerics summary (default: ``build/final_numbers.md``)."""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path
from typing import Any


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _regen_all() -> str:
    return (
        "`python packages/python/scripts/reproduce_paper_tables.py` "
        "(after `cargo build --release` and `python -m pip install -e \".[dev]\"`)"
    )


def _emit(path: Path) -> None:
    root = _repo_root()
    live_dir = root / "paper" / "exports" / "live_panel_v2_postbatch"
    l2_dir = root / "paper" / "exports" / "l2_verified_flagship_v1"
    feas = root / "paper" / "exports" / "strict_feasibility_report.json"

    lines: list[str] = [
        "# FINAL_NUMBERS (sealed exports summary)",
        "",
        "Each value below is traced to a generated export. Regenerate the tree with:",
        "",
        f"- {_regen_all()}",
        "",
        "## Live v2",
        "",
    ]

    svl = live_dir / "live_panel_summary_with_ci.csv"
    if svl.is_file():
        with svl.open(encoding="utf-8", newline="") as h:
            rows = list(csv.DictReader(h))
        for agent in ("gru", "honeycomb", "sweagent"):
            row = next((r for r in rows if r.get("agent_id") == agent), None)
            if not row:
                continue
            lines.append(f"### {agent}")
            for field, label in (
                ("static_pass_rate", "static pass rate"),
                ("live_pass_rate", "live pass rate"),
                ("static_ci_low", "Wilson low (static)"),
                ("static_ci_high", "Wilson high (static)"),
                ("live_ci_low", "Wilson low (live)"),
                ("live_ci_high", "Wilson high (live)"),
            ):
                val = row.get(field, "")
                rel = svl.relative_to(root).as_posix()
                lines.append(
                    f"- **{label}:** `{val}` — source_file `{rel}`, "
                    f"source_row_or_key `agent_id={agent}`, command_to_regenerate {_regen_all()}"
                )
            lines.append("")

    loo = live_dir / "live_leave_one_out.csv"
    if loo.is_file():
        lines.append("### Leave-one-out")
        lines.append(
            f"- **Table:** `{loo.relative_to(root).as_posix()}` — see each row for excluded task and "
            f"range columns; command_to_regenerate {_regen_all()}"
        )
        lines.append("")

    lines.extend(["## L2 flagship", ""])

    arm_csv = l2_dir / "l2_arm_breakdown.csv"
    if arm_csv.is_file():
        with arm_csv.open(encoding="utf-8", newline="") as h:
            arms = list(csv.DictReader(h))
        lines.append("### Arm breakdown (`validator_arm`)")
        for r in arms:
            vid = r.get("validator_arm", "")
            lines.append(
                f"- **{vid}:** entries={r.get('n_entries')}, "
                f"l1_pass={r.get('n_l1_pass_entries')}, l1_pass_l2_fail={r.get('n_l1_pass_l2_fail')} "
                f"— source_file `{arm_csv.relative_to(root).as_posix()}`, source_row_or_key `{vid}`, "
                f"command_to_regenerate {_regen_all()}"
            )
        lines.append("")

    gsum = l2_dir / "gold_patch_validation_summary.json"
    if gsum.is_file():
        data = _load_json(gsum)
        lines.append("### Gold patch validation summary")
        lines.append(
            f"- **JSON:** `{gsum.relative_to(root).as_posix()}` — keys: {', '.join(sorted(data.keys()))} "
            f"— command_to_regenerate `python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2` "
            f"then {_regen_all()}"
        )
        lines.append("")

    hsum = l2_dir / "l2_human_review_summary.csv"
    if hsum.is_file():
        lines.append("### Human review summary")
        lines.append(
            f"- **CSV:** `{hsum.relative_to(root).as_posix()}` — command_to_regenerate "
            f"`python ci/scripts/export_l2_flagship_reviews.py` then {_regen_all()}"
        )
        lines.append("")

    lines.extend(["## Verified feasibility", ""])
    if feas.is_file():
        data = _load_json(feas)
        verified = data.get("verified", {}) if isinstance(data, dict) else {}
        lines.append(
            f"- **strict_feasibility_report `/verified`:** see `{feas.relative_to(root).as_posix()}` "
            f"— command_to_regenerate `python ci/scripts/analyze_strict_feasibility.py` "
            f"then {_regen_all()}"
        )
        if isinstance(verified, dict):
            lines.append(f"  - snapshot keys: {', '.join(sorted(verified.keys()))}")
        lines.append("")

    lines.extend(["## Rust proof subset", ""])
    rust_m = root / "paper" / "exports" / "rust_proof_subset_v1_seal_release" / "manifest.json"
    if rust_m.is_file():
        lines.append(
            f"- **Paper-export manifest:** `{rust_m.relative_to(root).as_posix()}` — "
            f"command_to_regenerate {_regen_all()}"
        )
        if feas.is_file():
            data_feas = _load_json(feas)
            rust = (
                data_feas.get("rust_real_manifest", {})
                if isinstance(data_feas, dict)
                else {}
            )
            if isinstance(rust, dict) and rust:
                lines.append(
                    "- **strict_feasibility_report `/rust_real_manifest`:** keys "
                    f"{', '.join(sorted(rust.keys()))}"
                )
        lines.append("")

    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    root = _repo_root()
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--out",
        type=Path,
        default=root / "build" / "final_numbers.md",
    )
    args = p.parse_args()
    out = args.out.resolve()
    try:
        _emit(out)
    except OSError as e:
        print(f"write_final_numbers_md: {e}", file=sys.stderr)
        return 1
    print(f"wrote {out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
