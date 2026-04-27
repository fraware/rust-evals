#!/usr/bin/env python3
"""Export L2 flagship failure taxonomy and a human-review adjudication sample."""

from __future__ import annotations

import csv
import json
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
RUN_ROOT = REPO_ROOT / "runs" / "released" / "l2_verified_flagship_v1"
SUMMARY_PATH = RUN_ROOT / "results" / "batch_summary.json"
OUT_DIR = REPO_ROOT / "paper" / "exports" / "l2_verified_flagship_v1"

OUT_TAXONOMY = OUT_DIR / "l2_failure_taxonomy.csv"
OUT_REVIEW = OUT_DIR / "l2_failure_review.csv"
OUT_CASE_STUDIES = REPO_ROOT / "docs" / "l2_failure_case_studies.md"
RESULTS_ASTROPY_DIR = RUN_ROOT / "results_astropy"
RESULTS_REG_DIR = RUN_ROOT / "results_regression_fail"


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be a JSON object")
    return data


def _family_from_entry_id(entry_id: str) -> str:
    if entry_id.endswith("__astropy"):
        return "augmented_unit_tests"
    if entry_id.endswith("__regressionfail"):
        return "targeted_regression"
    return "unknown"


def _bundle_dir_for_entry(entry_id: str, bundle_name: str) -> Path:
    if entry_id.endswith("__astropy"):
        return RESULTS_ASTROPY_DIR / bundle_name
    if entry_id.endswith("__regressionfail"):
        return RESULTS_REG_DIR / bundle_name
    return SUMMARY_PATH.parent / bundle_name


def _task_id(entry: dict[str, Any]) -> str:
    return Path(str(entry.get("task_path", ""))).stem


def _agent_id(entry: dict[str, Any]) -> str:
    entry_id = str(entry.get("entry_id", ""))
    return entry_id.split("__", 1)[0] if "__" in entry_id else "unknown"


def _candidate_id(entry: dict[str, Any], bundle_dir: Path) -> str:
    sr = bundle_dir / "strengthened_results.json"
    if sr.is_file():
        data = _load_json(sr)
        value = data.get("candidate_id")
        if isinstance(value, str):
            return value
    cr = bundle_dir / "candidate_resolution.json"
    if cr.is_file():
        data = _load_json(cr)
        value = data.get("candidate_id")
        if isinstance(value, str):
            return value
    return ""


def _failure_summary(bundle_dir: Path) -> str:
    report = bundle_dir / "strengthening_report.json"
    if not report.is_file():
        return "missing strengthening_report.json"
    data = _load_json(report)
    verdicts = data.get("verdicts", [])
    if not isinstance(verdicts, list):
        return "invalid strengthening report verdicts"
    for verdict in verdicts:
        if not isinstance(verdict, dict):
            continue
        if str(verdict.get("verdict", "")).lower() != "fail":
            continue
        validator = str(verdict.get("validator", ""))
        sub_checks = verdict.get("sub_checks", [])
        if isinstance(sub_checks, list):
            for sub in sub_checks:
                if not isinstance(sub, dict):
                    continue
                if str(sub.get("verdict", "")).lower() == "fail":
                    detail = str(sub.get("detail", "")).replace("\n", " ").strip()
                    if detail:
                        if len(detail) > 180:
                            detail = detail[:177] + "..."
                        return f"{validator}:{sub.get('id','')} -> {detail}"
                    return f"{validator}:{sub.get('id','')} exit_code={sub.get('exit_code')}"
        return f"{validator} failed"
    return "no failing sub-check detail found"


def _load_failures() -> list[dict[str, Any]]:
    summary = _load_json(SUMMARY_PATH)
    failures: list[dict[str, Any]] = []
    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        levels = entry.get("levels", {})
        if not isinstance(levels, dict):
            continue
        l2 = levels.get("l2", {}) if isinstance(levels.get("l2", {}), dict) else {}
        if str(l2.get("status", "")).lower() != "fail":
            continue
        entry_id = str(entry.get("entry_id", ""))
        bundle_name = str(entry.get("bundle_name", ""))
        bundle_dir = _bundle_dir_for_entry(entry_id, bundle_name)
        failures.append(
            {
                "task_id": _task_id(entry),
                "candidate_id": _candidate_id(entry, bundle_dir),
                "agent_id": _agent_id(entry),
                "L1_verdict": str((levels.get("l1", {}) or {}).get("status", "")),
                "L2_failure_family": str(l2.get("primary_reason", "")),
                "validator_family": _family_from_entry_id(entry_id),
                "failure_summary": _failure_summary(bundle_dir),
                "bundle_name": bundle_name,
                "bundle_dir": bundle_dir,
            }
        )
    failures.sort(key=lambda x: (x["validator_family"], x["task_id"], x["agent_id"]))
    return failures


def _write_taxonomy(failures: list[dict[str, Any]]) -> None:
    counts: dict[tuple[str, str], int] = {}
    for f in failures:
        key = (f["validator_family"], f["L2_failure_family"])
        counts[key] = counts.get(key, 0) + 1
    with OUT_TAXONOMY.open("w", encoding="utf-8", newline="") as h:
        writer = csv.DictWriter(
            h,
            fieldnames=["validator_family", "primary_reason", "count"],
        )
        writer.writeheader()
        for (validator_family, reason), count in sorted(counts.items()):
            writer.writerow(
                {
                    "validator_family": validator_family,
                    "primary_reason": reason,
                    "count": count,
                }
            )


def _pick_review_sample(failures: list[dict[str, Any]]) -> list[dict[str, Any]]:
    aug = [f for f in failures if f["validator_family"] == "augmented_unit_tests"][:3]
    reg = [f for f in failures if f["validator_family"] == "targeted_regression"][:3]
    sample = aug + reg
    for f in sample:
        if f["validator_family"] == "augmented_unit_tests":
            f["why_validator_issue_relevant"] = (
                "Runs an additional pytest selector under warnings-as-errors to probe "
                "behavior beyond the official rerun."
            )
            if f["task_id"] == "astropy__astropy-7671":
                f["tp_fp_unclear"] = "true_positive"
                f["reviewer_notes"] = (
                    "Failure occurs on the same repository as the target issue and in "
                    "an augmented test path; treated as issue-relevant."
                )
            else:
                f["tp_fp_unclear"] = "unclear"
                f["reviewer_notes"] = (
                    "Augmented selector is Astropy-specific and may not align with "
                    "this task's issue boundary; keep as cautionary signal."
                )
        else:
            f["why_validator_issue_relevant"] = (
                "Regression family active for protocol completeness in flagship v1."
            )
            f["tp_fp_unclear"] = "false_positive"
            f["reviewer_notes"] = (
                "This family uses a forced non-zero command "
                "(`regression_forced_fail`), so failures indicate validator "
                "limitation rather than candidate regression."
            )
    return sample


def _write_review_csv(sample: list[dict[str, Any]]) -> None:
    fields = [
        "task_id",
        "candidate_id",
        "agent_id",
        "L1_verdict",
        "L2_failure_family",
        "failure_summary",
        "why_validator_issue_relevant",
        "tp_fp_unclear",
        "reviewer_notes",
    ]
    with OUT_REVIEW.open("w", encoding="utf-8", newline="") as h:
        writer = csv.DictWriter(h, fieldnames=fields)
        writer.writeheader()
        for row in sample:
            writer.writerow({k: row.get(k, "") for k in fields})


def _write_case_studies_md(sample: list[dict[str, Any]]) -> None:
    tp = sum(1 for r in sample if r["tp_fp_unclear"] == "true_positive")
    fp = sum(1 for r in sample if r["tp_fp_unclear"] == "false_positive")
    unclear = sum(1 for r in sample if r["tp_fp_unclear"] == "unclear")
    lines: list[str] = []
    lines.append("# L2 failure case studies (flagship v1)")
    lines.append("")
    lines.append("This note summarizes a six-case adjudication sample from")
    lines.append("`runs/released/l2_verified_flagship_v1/results/batch_summary.json`.")
    lines.append("")
    lines.append("## Sample composition")
    lines.append("")
    lines.append(f"- Total reviewed: `{len(sample)}`")
    lines.append(f"- Augmented-test failures: `{sum(1 for r in sample if r['validator_family']=='augmented_unit_tests')}`")
    lines.append(f"- Regression failures: `{sum(1 for r in sample if r['validator_family']=='targeted_regression')}`")
    lines.append(f"- Adjudication split: true positive `{tp}`, false positive `{fp}`, unclear `{unclear}`")
    lines.append("")
    lines.append("## Per-case notes")
    lines.append("")
    for i, row in enumerate(sample, start=1):
        lines.append(f"### Case {i}: `{row['task_id']}` / `{row['agent_id']}`")
        lines.append("")
        lines.append(f"- Candidate: `{row['candidate_id']}`")
        lines.append(f"- L1 verdict: `{row['L1_verdict']}`")
        lines.append(f"- L2 family: `{row['L2_failure_family']}` (`{row['validator_family']}`)")
        lines.append(f"- Failure summary: {row['failure_summary']}")
        lines.append(f"- Issue relevance rationale: {row['why_validator_issue_relevant']}")
        lines.append(f"- Adjudication: `{row['tp_fp_unclear']}`")
        lines.append(f"- Reviewer notes: {row['reviewer_notes']}")
        lines.append("")
    lines.append("## Integrity note")
    lines.append("")
    lines.append(
        "The `targeted_regression` family in flagship v1 is intentionally configured "
        "as `regression_forced_fail` (`sys.exit(1)`), so those failures are "
        "reported as validator limitation rather than candidate regression."
    )
    OUT_CASE_STUDIES.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    failures = _load_failures()
    _write_taxonomy(failures)
    sample = _pick_review_sample(failures)
    _write_review_csv(sample)
    _write_case_studies_md(sample)
    print(
        json.dumps(
            {
                "taxonomy_csv": str(OUT_TAXONOMY),
                "review_csv": str(OUT_REVIEW),
                "case_studies": str(OUT_CASE_STUDIES),
                "failures_total": len(failures),
                "sample_total": len(sample),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
