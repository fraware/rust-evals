#!/usr/bin/env python3
"""Emit L2 flagship TeX snippets, paper CSV alignments, and conditional-false-success rows."""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from typing import Any


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be a JSON object")
    return data


def _status(levels: dict[str, Any], key: str) -> str:
    d = levels.get(key, {})
    return str(d.get("status", "")).lower() if isinstance(d, dict) else ""


def _rewrite_conditional_false_success(out_dir: Path, entries: list[dict[str, Any]]) -> None:
    pairs: list[tuple[str, str]] = [
        ("L0", "L1"),
        ("L1", "L2"),
        ("L2", "L3"),
        ("L3", "L4"),
    ]
    rows: list[dict[str, Any]] = []
    for a, b in pairs:
        passed_from = [
            e
            for e in entries
            if _status(e.get("levels", {}), a.lower()) == "pass"
        ]
        n_pf = len(passed_from)
        n_failed_to = sum(
            1
            for e in passed_from
            if _status(e.get("levels", {}), b.lower()) == "fail"
        )
        rate = (n_failed_to / n_pf) if n_pf else None
        rows.append(
            {
                "level_from": a,
                "level_to": b,
                "n_passed_from": n_pf,
                "n_failed_to": n_failed_to,
                "rate": rate,
            }
        )

    csv_path = out_dir / "conditional_false_success.csv"
    with csv_path.open("w", encoding="utf-8", newline="") as h:
        w = csv.writer(h, quoting=csv.QUOTE_ALL)
        w.writerow(
            ["level_from", "level_to", "n_passed_from", "n_failed_to", "rate"]
        )
        for r in rows:
            w.writerow(
                [
                    r["level_from"],
                    r["level_to"],
                    str(r["n_passed_from"]),
                    str(r["n_failed_to"]),
                    "" if r["rate"] is None else f"{r['rate']:.6f}",
                ]
            )

    json_path = out_dir / "conditional_false_success.json"
    json_path.write_text(
        json.dumps(rows, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _gold_family_stats(gold_csv: Path) -> dict[str, dict[str, float | int]]:
    """
    Maps user-facing families (L2_*) using primary_reason column in gold CSV when present,
    else infers from validator_family column.
    """
    out: dict[str, dict[str, float | int]] = {
        "L2_AUG_TESTS_FAIL": {"n_tested": 0, "n_pass": 0},
        "L2_REGRESSION_FAIL": {"n_tested": 0, "n_pass": 0},
    }
    if not gold_csv.is_file():
        return out
    with gold_csv.open(encoding="utf-8", newline="") as f:
        for row in csv.DictReader(f):
            reason = (
                row.get("gold_patch_primary_reason_L2", "")
                or row.get("primary_reason", "")
            ).strip()
            vf = row.get("validator_family", "").strip()
            fam = reason
            if not fam.startswith("L2_"):
                if vf == "augmented_unit_tests":
                    fam = "L2_AUG_TESTS_FAIL"
                elif vf == "targeted_regression":
                    fam = "L2_REGRESSION_FAIL"
                else:
                    continue
            if fam not in out:
                continue
            st = str(row.get("gold_patch_status_L2", "")).lower()
            out[fam]["n_tested"] += 1
            if st == "pass":
                out[fam]["n_pass"] += 1
    for _k, v in out.items():
        n = int(v["n_tested"])
        p = int(v["n_pass"])
        v["rate"] = (p / n) if n else float("nan")
    return out


def _fmt_rate(x: float) -> str:
    if math.isnan(x):
        return "nan"
    return f"{x:.3f}"


def _l2_arm(entry_id: str) -> str:
    """Classifier for merged flagship rows (suffix matches bundle arm)."""
    if entry_id.endswith("__regressionfail"):
        return "regression_stress_control"
    if entry_id.endswith("__astropy"):
        return "augmented_tests"
    return "unknown"


def _arm_breakdown_rows(entries: list[dict[str, Any]]) -> list[dict[str, str]]:
    """Rows aligned with the paper's arm-separated headline table."""
    arms = ("augmented_tests", "regression_stress_control")
    rows_out: list[dict[str, str]] = []
    for arm in arms:
        sub = [
            e
            for e in entries
            if isinstance(e, dict) and _l2_arm(str(e.get("entry_id", ""))) == arm
        ]
        n_ent = len(sub)
        l1_pass = [e for e in sub if _status(e.get("levels", {}), "l1") == "pass"]
        n_l1 = len(l1_pass)
        l1_l2f = sum(
            1
            for e in l1_pass
            if _status(e.get("levels", {}), "l2") == "fail"
        )
        if arm == "augmented_tests":
            interp = (
                "Issue-relevant diagnostic; reviewed human-adjudication sample mixed "
                "(see docs/l2_failure_case_studies.md)."
            )
        else:
            interp = (
                "Negative-control / protocol signal (regression_forced_fail); "
                "not natural product-regression evidence."
            )
        rows_out.append(
            {
                "evaluator_arm": arm,
                "entries": str(n_ent),
                "l1_pass_entries": str(n_l1),
                "l1_pass_l2_fail": str(l1_l2f),
                "interpretation": interp,
            }
        )
    total_ent = len(entries)
    total_l1 = sum(
        1
        for e in entries
        if isinstance(e, dict) and _status(e.get("levels", {}), "l1") == "pass"
    )
    total_l1_l2f = sum(
        1
        for e in entries
        if isinstance(e, dict)
        and _status(e.get("levels", {}), "l1") == "pass"
        and _status(e.get("levels", {}), "l2") == "fail"
    )
    rows_out.append(
        {
            "evaluator_arm": "total",
            "entries": str(total_ent),
            "l1_pass_entries": str(total_l1),
            "l1_pass_l2_fail": str(total_l1_l2f),
            "interpretation": (
                "Evaluator sensitivity cohort; not a population bug-prevalence "
                "estimate."
            ),
        }
    )
    return rows_out


def _latex_escape(s: str) -> str:
    return (
        s.replace("\\", "\\textbackslash{}")
        .replace("{", "\\{")
        .replace("}", "\\}")
        .replace("_", "\\_")
        .replace("&", "\\&")
        .replace("%", "\\%")
        .replace("#", "\\#")
    )


def _write_tex(
    tex_dir: Path,
    summary: dict[str, Any],
    gold_stats: dict[str, dict[str, float | int]],
    taxonomy_rows: list[dict[str, str]],
    arm_rows: list[dict[str, str]],
) -> None:
    tex_dir.mkdir(parents=True, exist_ok=True)
    entries = summary.get("entries", [])
    total = int(summary.get("total_entries", len(entries)))
    l1_pass_n = sum(
        1 for e in entries if _status(e.get("levels", {}), "l1") == "pass"
    )
    l1p_l2f = sum(
        1
        for e in entries
        if _status(e.get("levels", {}), "l1") == "pass"
        and _status(e.get("levels", {}), "l2") == "fail"
    )

    flag = tex_dir / "l2_flagship_summary.tex"
    flag.write_text(
        "\\begin{tabular}{lr}\n"
        "\\hline\n"
        "Metric & Value \\\\\n"
        "\\hline\n"
        f"Total sealed entries & {total} \\\\\n"
        f"L1-passing candidates & {l1_pass_n} \\\\\n"
        f"L2 fail $\\mid$ L1 pass (headline gate) & {l1p_l2f} \\\\\n"
        "\\hline\n"
        "\\multicolumn{2}{l}{\\footnotesize "
        "Raw L2 fail counts among L1-fail rows are omitted here; "
        "see conditional\\_false\\_success exports.} \\\\\n"
        "\\hline\n"
        "\\end{tabular}\n",
        encoding="utf-8",
    )

    arm_tex = tex_dir / "l2_flagship_arm_breakdown.tex"
    short_interp = {
        "augmented_tests": "Augmented-test diagnostic (issue relevance varies).",
        "regression_stress_control": "Stress-control protocol (forced-fail hook).",
        "total": "Merged cohort; sensitivity not prevalence.",
    }
    alines = [
        "\\begin{tabular}{lrrrl}",
        "\\hline",
        "Arm & Entries & L1-pass & L1-pass $\\rightarrow$ L2-fail & Interpretation \\\\",
        "\\hline",
    ]
    for row in arm_rows:
        if str(row.get("evaluator_arm")) == "total":
            alines.append("\\hline")
        arm_key = str(row.get("evaluator_arm", ""))
        arm_label = arm_key.replace("_", "\\_")
        interp = _latex_escape(short_interp.get(arm_key, ""))
        alines.append(
            f"{arm_label} & {row.get('entries','')} & {row.get('l1_pass_entries','')} & "
            f"{row.get('l1_pass_l2_fail','')} & {interp} \\\\"
        )
    alines.extend(["\\hline", "\\end{tabular}", ""])
    arm_tex.write_text("\n".join(alines), encoding="utf-8")

    tax_lines = [
        "\\begin{tabular}{lrr}",
        "\\hline",
        "Family & Primary reason & Count \\\\",
        "\\hline",
    ]
    for row in taxonomy_rows:
        tax_lines.append(
            f"{_latex_escape(row.get('validator_family',''))} & "
            f"{_latex_escape(row.get('primary_reason',''))} & "
            f"{row.get('count','')} \\\\"
        )
    tax_lines.extend(["\\hline", "\\end{tabular}", ""])
    (tex_dir / "l2_failure_families.tex").write_text(
        "\n".join(tax_lines), encoding="utf-8"
    )

    aug = gold_stats.get("L2_AUG_TESTS_FAIL", {})
    reg = gold_stats.get("L2_REGRESSION_FAIL", {})
    gold_tex = tex_dir / "l2_gold_patch_family_summary.tex"
    ar = float(aug.get("rate", float("nan")))
    rr = float(reg.get("rate", float("nan")))
    gold_tex.write_text(
        "\\begin{tabular}{lrrr}\n"
        "\\hline\n"
        "Validator family & $n$ tested & $n$ pass L2 & Pass rate \\\\\n"
        "\\hline\n"
        f"L2\\_AUG\\_TESTS\\_FAIL & {aug.get('n_tested', 0)} & "
        f"{aug.get('n_pass', 0)} & "
        f"{_fmt_rate(ar)} \\\\\n"
        f"L2\\_REGRESSION\\_FAIL & {reg.get('n_tested', 0)} & "
        f"{reg.get('n_pass', 0)} & "
        f"{_fmt_rate(rr)} \\\\\n"
        "\\hline\n"
        "\\end{tabular}\n",
        encoding="utf-8",
    )


def export_l2_tables(
    repo_root: Path,
    run_dir: Path,
    out_dir: Path,
    tex_dir: Path,
    gold_csv: Path,
    taxonomy_csv: Path,
) -> dict[str, Any]:
    summary_path = run_dir / "batch_summary.json"
    if not summary_path.is_file():
        raise FileNotFoundError(summary_path)
    summary = _load_json(summary_path)
    entries = summary.get("entries", [])
    if not isinstance(entries, list):
        raise TypeError("batch_summary.entries must be a list")
    sv = summary.get("schema_version")
    if sv is not None and int(sv) != 1:
        raise ValueError(
            f"unexpected batch_summary.schema_version={sv!r} (expected 1)"
        )

    _rewrite_conditional_false_success(out_dir, [e for e in entries if isinstance(e, dict)])

    gold_stats = _gold_family_stats(gold_csv)

    taxonomy_rows: list[dict[str, str]] = []
    if taxonomy_csv.is_file():
        with taxonomy_csv.open(encoding="utf-8", newline="") as h:
            taxonomy_rows = list(csv.DictReader(h))

    arm_rows = _arm_breakdown_rows([e for e in entries if isinstance(e, dict)])
    arm_csv = out_dir / "l2_flagship_arm_breakdown.csv"
    with arm_csv.open("w", encoding="utf-8", newline="") as h:
        w = csv.DictWriter(
            h,
            fieldnames=[
                "evaluator_arm",
                "entries",
                "l1_pass_entries",
                "l1_pass_l2_fail",
                "interpretation",
            ],
        )
        w.writeheader()
        for row in arm_rows:
            w.writerow(row)

    _write_tex(tex_dir, summary, gold_stats, taxonomy_rows, arm_rows)

    return {
        "conditional_false_success_csv": str(
            (out_dir / "conditional_false_success.csv").relative_to(repo_root)
        ),
        "l2_flagship_arm_breakdown_csv": str(arm_csv.relative_to(repo_root)),
        "tex": {
            "l2_flagship_summary": str(
                (tex_dir / "l2_flagship_summary.tex").relative_to(repo_root)
            ),
            "l2_flagship_arm_breakdown": str(
                (tex_dir / "l2_flagship_arm_breakdown.tex").relative_to(repo_root)
            ),
            "l2_failure_families": str(
                (tex_dir / "l2_failure_families.tex").relative_to(repo_root)
            ),
            "l2_gold_patch_family_summary": str(
                (tex_dir / "l2_gold_patch_family_summary.tex").relative_to(repo_root)
            ),
        },
        "gold_family_stats": gold_stats,
    }


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--run-dir",
        type=Path,
        default=repo_root / "runs/released/l2_verified_flagship_v1/results",
    )
    p.add_argument(
        "--gold-run-dir",
        type=Path,
        default=repo_root / "runs/released/l2_verified_flagship_v1/gold_patch_results",
        help="Optional; gold CSV path below is authoritative for paper stats.",
    )
    p.add_argument(
        "--out-dir",
        type=Path,
        default=repo_root / "paper/exports/l2_verified_flagship_v1",
    )
    p.add_argument("--tex-dir", type=Path, default=repo_root / "paper/tables")
    p.add_argument(
        "--gold-csv",
        type=Path,
        default=repo_root
        / "paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv",
    )
    p.add_argument(
        "--taxonomy-csv",
        type=Path,
        default=repo_root
        / "paper/exports/l2_verified_flagship_v1/l2_failure_taxonomy.csv",
    )
    args = p.parse_args()
    run_dir = args.run_dir.resolve()
    out_dir = args.out_dir.resolve()
    tex_dir = args.tex_dir.resolve()
    meta = export_l2_tables(
        repo_root,
        run_dir,
        out_dir,
        tex_dir,
        args.gold_csv.resolve(),
        args.taxonomy_csv.resolve(),
    )
    print(json.dumps(meta, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
