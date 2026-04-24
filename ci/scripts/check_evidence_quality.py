#!/usr/bin/env python3
"""Quality gates for release evidence tranches.

This script enforces objective acceptance thresholds for the empirical
tranche described in the NeurIPS execution plan (Verified, Live, L2,
Rust proof-subset). It returns non-zero when any configured gate fails.
"""

from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_batch_summary(run_dir: Path) -> dict[str, Any]:
    return _load_json(run_dir / "batch_summary.json")


def _level(entry: dict[str, Any], key: str) -> dict[str, Any]:
    levels = entry.get("levels", {})
    value = levels.get(key, {})
    if not isinstance(value, dict):
        return {}
    return value


def _agent_from_entry(entry: dict[str, Any]) -> str:
    entry_id = entry.get("entry_id") or entry.get("bundle_name") or "unknown"
    if "__" in entry_id:
        return entry_id.split("__", 1)[0]
    return "unknown"


def _is_pass(level: dict[str, Any]) -> bool:
    return str(level.get("status", "")).lower() == "pass"


def _reason(level: dict[str, Any]) -> str:
    return str(level.get("primary_reason", ""))


def _print_report(report: dict[str, Any]) -> None:
    print(json.dumps(report, indent=2, sort_keys=True))


def _gate(cond: bool, message: str, failures: list[str]) -> None:
    if not cond:
        failures.append(message)


def check_verified(args: argparse.Namespace) -> int:
    summary = _load_batch_summary(args.run_dir)
    entries = summary.get("entries", [])
    failures: list[str] = []

    _gate(
        len(entries) >= args.min_candidates,
        f"total candidates {len(entries)} < {args.min_candidates}",
        failures,
    )

    l1_harness = sum(
        1
        for e in entries
        if _reason(_level(e, "l1")) == "L1_HARNESS_ERROR"
    )
    harness_rate = (l1_harness / len(entries)) if entries else 1.0
    _gate(
        harness_rate <= args.max_l1_harness_error_rate,
        f"L1_HARNESS_ERROR rate {harness_rate:.3f} > {args.max_l1_harness_error_rate:.3f}",
        failures,
    )

    by_agent: dict[str, dict[str, int]] = defaultdict(
        lambda: {"l0_pass": 0, "l1_pass": 0}
    )
    for e in entries:
        agent = _agent_from_entry(e)
        if _is_pass(_level(e, "l0")):
            by_agent[agent]["l0_pass"] += 1
        if _is_pass(_level(e, "l1")):
            by_agent[agent]["l1_pass"] += 1

    nonzero_agents = sum(
        1
        for s in by_agent.values()
        if (s["l0_pass"] > 0 or s["l1_pass"] > 0)
    )
    _gate(
        nonzero_agents >= args.min_nonzero_agents,
        f"nonzero agents {nonzero_agents} < {args.min_nonzero_agents}",
        failures,
    )

    distinct_score_vectors = {
        (s["l0_pass"], s["l1_pass"]) for s in by_agent.values()
    }
    _gate(
        len(distinct_score_vectors) >= args.min_distinct_agents,
        "distinct agent pass-count vectors "
        f"{len(distinct_score_vectors)} < {args.min_distinct_agents}",
        failures,
    )

    l3_fail_reasons = [
        _reason(_level(e, "l3"))
        for e in entries
        if str(_level(e, "l3").get("status", "")).lower() == "fail"
    ]
    reason_counts = Counter(l3_fail_reasons)
    dominant_share = (
        max(reason_counts.values()) / len(l3_fail_reasons)
        if l3_fail_reasons
        else 0.0
    )
    _gate(
        dominant_share <= args.max_l3_single_reason_share,
        "L3 dominant failure reason share "
        f"{dominant_share:.3f} > "
        f"{args.max_l3_single_reason_share:.3f}",
        failures,
    )

    report = {
        "mode": "verified",
        "ok": not failures,
        "failures": failures,
        "metrics": {
            "total_candidates": len(entries),
            "l1_harness_error_rate": harness_rate,
            "nonzero_agents": nonzero_agents,
            "distinct_agent_pass_vectors": len(distinct_score_vectors),
            "l3_dominant_reason_share": dominant_share,
            "l3_reason_counts": dict(reason_counts),
        },
    }
    _print_report(report)
    return 0 if not failures else 2


def check_live(args: argparse.Namespace) -> int:
    static_vs_live = _load_json(args.paper_export_dir / "static_vs_live.json")
    rank_stability = _load_json(args.paper_export_dir / "rank_stability.json")
    failures: list[str] = []

    levels = {row["level"] for row in static_vs_live}
    agents = {row["agent_id"] for row in static_vs_live}
    _gate(
        len(agents) >= args.min_agents,
        f"agents {len(agents)} < {args.min_agents}",
        failures,
    )

    # Compare live pass rates by level; at least one level should be non-tied.
    non_tied_levels = 0
    negative_delta_rows = 0
    for level in levels:
        rows = [r for r in static_vs_live if r["level"] == level]
        rates = {float(r["live_pass_rate"]) for r in rows}
        if len(rates) > 1:
            non_tied_levels += 1
        negative_delta_rows += sum(1 for r in rows if float(r["delta"]) < 0.0)
    _gate(non_tied_levels >= 1, "no non-tied live ranking at any level", failures)
    _gate(
        negative_delta_rows == len(static_vs_live),
        "static-vs-live delta is not negative for every row",
        failures,
    )

    informative_rank_rows = [
        r
        for r in rank_stability
        if r.get("n_agents", 0) >= args.min_agents
        and r.get("kendall_tau_b") is not None
        and float(r["kendall_tau_b"]) != 0.0
    ]
    _gate(
        len(informative_rank_rows) >= 1,
        "rank_stability has no informative non-zero tau row",
        failures,
    )

    report = {
        "mode": "live",
        "ok": not failures,
        "failures": failures,
        "metrics": {
            "levels": sorted(levels),
            "agents": sorted(agents),
            "non_tied_levels": non_tied_levels,
            "negative_delta_rows": negative_delta_rows,
            "total_rows": len(static_vs_live),
            "informative_rank_rows": len(informative_rank_rows),
        },
    }
    _print_report(report)
    return 0 if not failures else 2


def check_l2(args: argparse.Namespace) -> int:
    summary = _load_batch_summary(args.run_dir)
    entries = summary.get("entries", [])
    failures: list[str] = []

    l1_pass = [e for e in entries if _is_pass(_level(e, "l1"))]
    l2_fail = [
        e
        for e in l1_pass
        if str(_level(e, "l2").get("status", "")).lower() == "fail"
    ]
    l2_reasons = Counter(_reason(_level(e, "l2")) for e in l2_fail)

    _gate(
        len(l1_pass) >= args.min_l1_passed_from,
        f"l1 passed-from {len(l1_pass)} < {args.min_l1_passed_from}",
        failures,
    )
    _gate(
        len(l2_fail) >= args.min_l2_failures,
        f"l2 failures {len(l2_fail)} < {args.min_l2_failures}",
        failures,
    )
    _gate(
        len([k for k in l2_reasons if k]) >= args.min_l2_reason_families,
        f"l2 reason families {len([k for k in l2_reasons if k])} < {args.min_l2_reason_families}",
        failures,
    )

    report = {
        "mode": "l2",
        "ok": not failures,
        "failures": failures,
        "metrics": {
            "total_entries": len(entries),
            "l1_passed_from": len(l1_pass),
            "l2_failures": len(l2_fail),
            "l2_reason_counts": dict(l2_reasons),
        },
    }
    _print_report(report)
    return 0 if not failures else 2


def check_rust_proof(args: argparse.Namespace) -> int:
    summary = _load_batch_summary(args.run_dir)
    entries = summary.get("entries", [])
    failures: list[str] = []

    ok_entries = sum(1 for e in entries if e.get("status") == "ok")
    invalid_entries = sum(1 for e in entries if e.get("status") != "ok")
    _gate(
        ok_entries == args.expected_entries,
        f"ok entries {ok_entries} != {args.expected_entries}",
        failures,
    )
    _gate(
        invalid_entries == 0,
        f"invalid entries {invalid_entries} != 0",
        failures,
    )

    l3_pass_l4_fail = 0
    all_pass = 0
    for e in entries:
        l0 = _level(e, "l0")
        l1 = _level(e, "l1")
        l3 = _level(e, "l3")
        l4 = _level(e, "l4")
        if _is_pass(l3) and str(l4.get("status", "")).lower() == "fail":
            l3_pass_l4_fail += 1
        if _is_pass(l0) and _is_pass(l1) and _is_pass(l3) and _is_pass(l4):
            all_pass += 1

    _gate(
        l3_pass_l4_fail >= args.min_l3_pass_l4_fail,
        f"l3-pass/l4-fail {l3_pass_l4_fail} < {args.min_l3_pass_l4_fail}",
        failures,
    )
    _gate(
        all_pass >= args.min_all_level_pass,
        f"all-level pass {all_pass} < {args.min_all_level_pass}",
        failures,
    )

    report = {
        "mode": "rust_proof",
        "ok": not failures,
        "failures": failures,
        "metrics": {
            "total_entries": len(entries),
            "ok_entries": ok_entries,
            "invalid_entries": invalid_entries,
            "l3_pass_l4_fail": l3_pass_l4_fail,
            "all_level_pass": all_pass,
        },
    }
    _print_report(report)
    return 0 if not failures else 2


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        description="Quality gates for NeurIPS evidence tranches."
    )
    sub = p.add_subparsers(dest="mode", required=True)

    pv = sub.add_parser("verified", help="Gate a headline Verified run.")
    pv.add_argument("--run-dir", type=Path, required=True)
    pv.add_argument("--min-candidates", type=int, default=30)
    pv.add_argument("--max-l1-harness-error-rate", type=float, default=0.10)
    pv.add_argument("--min-distinct-agents", type=int, default=2)
    pv.add_argument("--min-nonzero-agents", type=int, default=2)
    pv.add_argument("--max-l3-single-reason-share", type=float, default=0.80)
    pv.set_defaults(func=check_verified)

    pl = sub.add_parser("live", help="Gate a comparative Live panel export.")
    pl.add_argument("--paper-export-dir", type=Path, required=True)
    pl.add_argument("--min-agents", type=int, default=3)
    pl.set_defaults(func=check_live)

    p2 = sub.add_parser("l2", help="Gate an L2 expansion run.")
    p2.add_argument("--run-dir", type=Path, required=True)
    p2.add_argument("--min-l1-passed-from", type=int, default=10)
    p2.add_argument("--min-l2-failures", type=int, default=3)
    p2.add_argument("--min-l2-reason-families", type=int, default=2)
    p2.set_defaults(func=check_l2)

    pr = sub.add_parser("rust-proof", help="Gate the rust proof-subset run.")
    pr.add_argument("--run-dir", type=Path, required=True)
    pr.add_argument("--expected-entries", type=int, default=8)
    pr.add_argument("--min-l3-pass-l4-fail", type=int, default=2)
    pr.add_argument("--min-all-level-pass", type=int, default=1)
    pr.set_defaults(func=check_rust_proof)

    return p


def main() -> int:
    parser = _build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
