#!/usr/bin/env python3
"""Export Live panel v2 transparency tables, Wilson CIs, and leave-one-out rows."""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from typing import Any


def wilson_interval(k: int, n: int, z: float = 1.96) -> tuple[float, float]:
    """Wilson score interval for binomial proportion (95% default)."""
    if n <= 0:
        return (float("nan"), float("nan"))
    p = k / n
    z2 = z * z
    denom = 1.0 + z2 / n
    center = (p + z2 / (2.0 * n)) / denom
    inner = (p * (1.0 - p) + z2 / (4.0 * n)) / n
    half = z * math.sqrt(max(0.0, inner)) / denom
    return (max(0.0, center - half), min(1.0, center + half))


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be a JSON object")
    return data


def _surface_for_task(
    task_id: str,
    live_manifests: Path,
    verified_manifests: Path,
) -> tuple[str, str, str]:
    lp = live_manifests / f"{task_id}.json"
    if lp.is_file():
        m = _load_json(lp)
        return (
            "live",
            str(m.get("repo_name", "")),
            str(m.get("created_at", "")) or str(m.get("source_url", "")),
        )
    vp = verified_manifests / f"{task_id}.json"
    if vp.is_file():
        m = _load_json(vp)
        return (
            "static_anchor",
            str(m.get("repo_name", "")),
            str(m.get("created_at", "")) or str(m.get("source_url", "")),
        )
    return ("unknown", "", "")


def _candidate_id(bundle_dir: Path) -> str:
    cr = bundle_dir / "candidate_resolution.json"
    if cr.is_file():
        try:
            data = _load_json(cr)
            cid = data.get("candidate_id")
            if isinstance(cid, str):
                return cid
        except (OSError, json.JSONDecodeError, TypeError):
            pass
    return ""


def export_live_tables(
    repo_root: Path,
    run_dir: Path,
    out_dir: Path,
) -> dict[str, Any]:
    summary_path = run_dir / "batch_summary.json"
    if not summary_path.is_file():
        raise FileNotFoundError(f"missing batch summary: {summary_path}")

    live_manifests = repo_root / "benchmarks" / "live" / "manifests"
    verified_manifests = repo_root / "benchmarks" / "verified" / "manifests"

    summary = _load_json(summary_path)
    per_task: list[dict[str, Any]] = []
    static_rates: dict[str, float] = {}

    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        levels = entry.get("levels", {})
        if not isinstance(levels, dict):
            levels = {}
        entry_id = str(entry.get("entry_id", ""))
        agent_id = entry_id.split("__", 1)[0] if "__" in entry_id else ""
        task_path = Path(str(entry.get("task_path", "")))
        task_id = task_path.stem
        bundle_name = str(entry.get("bundle_name", ""))
        bundle_dir = run_dir / bundle_name
        rel_bundle = str(bundle_dir.relative_to(repo_root)).replace("\\", "/")

        surf, repo, tsource = _surface_for_task(
            task_id, live_manifests, verified_manifests
        )
        l0 = levels.get("l0", {}) if isinstance(levels.get("l0", {}), dict) else {}
        l1 = levels.get("l1", {}) if isinstance(levels.get("l1", {}), dict) else {}

        per_task.append(
            {
                "task_id": task_id,
                "benchmark_surface": surf,
                "repo": repo,
                "task_date_or_source": tsource,
                "agent_id": agent_id,
                "candidate_id": _candidate_id(bundle_dir),
                "status_L0": str(l0.get("status", "")),
                "status_L1": str(l1.get("status", "")),
                "primary_reason_L0": str(l0.get("primary_reason", "")),
                "primary_reason_L1": str(l1.get("primary_reason", "")),
                "bundle_path": rel_bundle,
            }
        )

    per_task.sort(
        key=lambda r: (r["benchmark_surface"], r["task_id"], r["agent_id"])
    )

    out_dir.mkdir(parents=True, exist_ok=True)
    pt_path = out_dir / "per_task_live_outcomes.csv"
    fields_pt = [
        "task_id",
        "benchmark_surface",
        "repo",
        "task_date_or_source",
        "agent_id",
        "candidate_id",
        "status_L0",
        "status_L1",
        "primary_reason_L0",
        "primary_reason_L1",
        "bundle_path",
    ]
    with pt_path.open("w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields_pt)
        w.writeheader()
        for row in per_task:
            w.writerow({k: row.get(k, "") for k in fields_pt})

    static_rows = [r for r in per_task if r["benchmark_surface"] == "static_anchor"]
    live_rows = [r for r in per_task if r["benchmark_surface"] == "live"]

    by_agent_static: dict[str, list[dict[str, Any]]] = {}
    for r in static_rows:
        by_agent_static.setdefault(r["agent_id"], []).append(r)
    by_agent_live: dict[str, list[dict[str, Any]]] = {}
    for r in live_rows:
        by_agent_live.setdefault(r["agent_id"], []).append(r)

    for aid, sr in by_agent_static.items():
        n = len(sr)
        k = sum(1 for x in sr if x["status_L1"] == "pass")
        static_rates[aid] = k / n if n else 0.0

    loo_rows: list[dict[str, Any]] = []
    for agent_id, agent_live in sorted(by_agent_live.items()):
        static_p = static_rates.get(agent_id, 0.0)
        task_ids = sorted({r["task_id"] for r in agent_live})
        for removed in task_ids:
            subset = [r for r in agent_live if r["task_id"] != removed]
            ev = len(subset)
            passed = sum(1 for r in subset if r["status_L1"] == "pass")
            rate = passed / ev if ev else 0.0
            loo_rows.append(
                {
                    "agent_id": agent_id,
                    "removed_task_id": removed,
                    "live_passed": passed,
                    "live_evaluated": ev,
                    "live_pass_rate": rate,
                    "delta_vs_static": rate - static_p,
                }
            )

    summary_loo: list[dict[str, Any]] = []
    for agent_id, agent_live in sorted(by_agent_live.items()):
        static_p = static_rates.get(agent_id, 0.0)
        rates = []
        deltas = []
        task_ids = sorted({r["task_id"] for r in agent_live})
        for removed in task_ids:
            subset = [r for r in agent_live if r["task_id"] != removed]
            ev = len(subset)
            passed = sum(1 for r in subset if r["status_L1"] == "pass")
            rate = passed / ev if ev else 0.0
            rates.append(rate)
            deltas.append(rate - static_p)
        if rates:
            summary_loo.append(
                {
                    "agent_id": agent_id,
                    "min_live_pass_rate_loo": min(rates),
                    "max_live_pass_rate_loo": max(rates),
                    "min_delta_loo": min(deltas),
                    "max_delta_loo": max(deltas),
                }
            )

    loo_path = out_dir / "live_leave_one_out.csv"
    fields_loo = [
        "row_kind",
        "agent_id",
        "removed_task_id",
        "live_passed",
        "live_evaluated",
        "live_pass_rate",
        "delta_vs_static",
        "min_live_pass_rate_loo",
        "max_live_pass_rate_loo",
        "min_delta_loo",
        "max_delta_loo",
    ]
    with loo_path.open("w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields_loo)
        w.writeheader()
        for row in loo_rows:
            w.writerow(
                {
                    "row_kind": "loo_detail",
                    "agent_id": row["agent_id"],
                    "removed_task_id": row["removed_task_id"],
                    "live_passed": row["live_passed"],
                    "live_evaluated": row["live_evaluated"],
                    "live_pass_rate": row["live_pass_rate"],
                    "delta_vs_static": row["delta_vs_static"],
                    "min_live_pass_rate_loo": "",
                    "max_live_pass_rate_loo": "",
                    "min_delta_loo": "",
                    "max_delta_loo": "",
                }
            )
        for row in summary_loo:
            w.writerow(
                {
                    "row_kind": "loo_summary",
                    "agent_id": row["agent_id"],
                    "removed_task_id": "",
                    "live_passed": "",
                    "live_evaluated": "",
                    "live_pass_rate": "",
                    "delta_vs_static": "",
                    "min_live_pass_rate_loo": row["min_live_pass_rate_loo"],
                    "max_live_pass_rate_loo": row["max_live_pass_rate_loo"],
                    "min_delta_loo": row["min_delta_loo"],
                    "max_delta_loo": row["max_delta_loo"],
                }
            )

    ci_rows: list[dict[str, Any]] = []
    schema_v = summary.get("schema_version")
    eval_v = summary.get("evaluator_version")

    for agent_id in sorted(set(by_agent_static.keys()) | set(by_agent_live.keys())):
        st = by_agent_static.get(agent_id, [])
        lv = by_agent_live.get(agent_id, [])
        sk = sum(1 for x in st if x["status_L1"] == "pass")
        sn = len(st)
        lk = sum(1 for x in lv if x["status_L1"] == "pass")
        ln = len(lv)
        slo, shi = wilson_interval(sk, sn)
        llo, lhi = wilson_interval(lk, ln)
        sp = sk / sn if sn else 0.0
        lp = lk / ln if ln else 0.0
        ci_rows.append(
            {
                "agent_id": agent_id,
                "static_passed": sk,
                "static_evaluated": sn,
                "static_pass_rate": sp,
                "static_ci_low": slo,
                "static_ci_high": shi,
                "live_passed": lk,
                "live_evaluated": ln,
                "live_pass_rate": lp,
                "live_ci_low": llo,
                "live_ci_high": lhi,
                "delta": lp - sp,
                "ratio": (lp / sp) if sp else float("nan"),
            }
        )

    ci_path = out_dir / "live_panel_summary_with_ci.csv"
    ci_fields = [
        "agent_id",
        "static_passed",
        "static_evaluated",
        "static_pass_rate",
        "static_ci_low",
        "static_ci_high",
        "live_passed",
        "live_evaluated",
        "live_pass_rate",
        "live_ci_low",
        "live_ci_high",
        "delta",
        "ratio",
    ]
    meta_path = out_dir / "live_ci_method.txt"
    meta_path.write_text(
        "Uncertainty intervals: Wilson score intervals (95%, z=1.96) "
        "for binomial proportions; nan ratio when static_pass_rate==0.\n"
        f"batch_summary.schema_version={schema_v!r} evaluator_version={eval_v!r}\n",
        encoding="utf-8",
    )

    with ci_path.open("w", encoding="utf-8", newline="") as f:
        cw = csv.DictWriter(f, fieldnames=ci_fields)
        cw.writeheader()
        for row in ci_rows:
            cw.writerow(row)

    tex_dir = repo_root / "paper" / "tables"
    tex_dir.mkdir(parents=True, exist_ok=True)
    live_tex = tex_dir / "live_static_vs_live.tex"
    tex_lines = [
        "\\begin{tabular}{lrrrrrr}",
        "\\hline",
        "Agent & Static pass & Static $n$ & Live pass & Live $n$"
        " & $\\Delta$ rate & Wilson live CI \\\\",
        "\\hline",
    ]
    for row in ci_rows:
        lc = ""
        slo = row["live_ci_low"]
        shi = row["live_ci_high"]
        if slo == slo and shi == shi:
            lc = f"[{slo:.3f},{shi:.3f}]"
        tex_lines.append(
            f"{row['agent_id']} & "
            f"{row['static_pass_rate']:.3f} & {row['static_evaluated']} & "
            f"{row['live_pass_rate']:.3f} & {row['live_evaluated']} & "
            f"{row['delta']:.3f} & {lc} \\\\"
        )
    tex_lines.extend(["\\hline", "\\end{tabular}", ""])
    live_tex.write_text("\n".join(tex_lines), encoding="utf-8")

    return {
        "per_task_csv": str(pt_path.relative_to(repo_root)),
        "live_leave_one_out_csv": str(loo_path.relative_to(repo_root)),
        "live_panel_summary_with_ci_csv": str(ci_path.relative_to(repo_root)),
        "live_ci_method_txt": str(meta_path.relative_to(repo_root)),
        "live_static_vs_live_tex": str(live_tex.relative_to(repo_root)),
        "ci_method": "wilson_95",
    }


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--run-dir",
        type=Path,
        default=repo_root / "runs/released/live_panel_v2/results_opt",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=repo_root / "paper/exports/live_panel_v2_postbatch",
    )
    args = parser.parse_args()
    run_dir = args.run_dir
    out_dir = args.out_dir
    if not run_dir.is_absolute():
        run_dir = (repo_root / run_dir).resolve()
    if not out_dir.is_absolute():
        out_dir = (repo_root / out_dir).resolve()
    meta = export_live_tables(repo_root, run_dir, out_dir)
    print(json.dumps(meta, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())