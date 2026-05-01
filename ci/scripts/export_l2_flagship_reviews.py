#!/usr/bin/env python3
"""Export L2 flagship failure taxonomy and human adjudication sample (publication-ready)."""

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
OUT_REVIEW_JSON = OUT_DIR / "l2_failure_review.json"
OUT_CASE_STUDIES = REPO_ROOT / "docs" / "l2_failure_case_studies.md"
RESULTS_ASTROPY_DIR = RUN_ROOT / "results_astropy"
RESULTS_REG_DIR = RUN_ROOT / "results_regression_fail"
GOLD_VALIDATION_CSV = OUT_DIR / "gold_patch_validation.csv"


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


def _candidate_id(bundle_dir: Path) -> str:
    sr = bundle_dir / "strengthened_results.json"
    if sr.is_file():
        try:
            data = _load_json(sr)
            if isinstance(data.get("candidate_id"), str):
                return str(data["candidate_id"])
        except (OSError, json.JSONDecodeError):
            pass
    cr = bundle_dir / "candidate_resolution.json"
    if cr.is_file():
        try:
            data = _load_json(cr)
            if isinstance(data.get("candidate_id"), str):
                return str(data["candidate_id"])
        except (OSError, json.JSONDecodeError):
            pass
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
                        return (
                            f"{validator}:{sub.get('id','')} -> {detail}"
                        )
                    return (
                        f"{validator}:{sub.get('id','')} exit_code="
                        f"{sub.get('exit_code')}"
                    )
        return f"{validator} failed"
    return "no failing sub-check detail found"


def _load_failures_index() -> dict[str, dict[str, Any]]:
    summary = _load_json(SUMMARY_PATH)
    idx: dict[str, dict[str, Any]] = {}
    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        eid = str(entry.get("entry_id", ""))
        idx[eid] = entry
    return idx


def _gold_lookup() -> dict[tuple[str, str], str]:
    """Keys: (task_id, gold_csv_family).

    ``gold_csv_family`` is ``augmented_unit_tests`` or ``targeted_regression``.
    """
    lookup: dict[tuple[str, str], str] = {}
    if not GOLD_VALIDATION_CSV.is_file():
        return lookup
    with GOLD_VALIDATION_CSV.open(encoding="utf-8", newline="") as f:
        for row in csv.DictReader(f):
            tid = row.get("task_id", "")
            vf = row.get("validator_family", "")
            st = row.get("gold_patch_status_L2", "")
            if tid and vf:
                lookup[(tid, vf)] = st
    return lookup


def _gold_pass_display(
    task_id: str,
    entry_id: str,
    gold: dict[tuple[str, str], str],
) -> str:
    if entry_id.endswith("__astropy"):
        key = (task_id, "augmented_unit_tests")
    elif entry_id.endswith("__regressionfail"):
        key = (task_id, "targeted_regression")
    else:
        return ""
    st = str(gold.get(key, "")).strip().lower()
    if st == "pass":
        return "pass"
    if st == "fail":
        return "fail"
    if st:
        return st
    return "not_available"


# Eight sealed rows: four augmented-test, four regression (diverse tasks/agents).
CURATED_ENTRY_IDS: tuple[str, ...] = (
    "gru__astropy__astropy-7671__astropy",
    "gru__django__django-7530__astropy",
    "honeycomb__pylint-dev__pylint-7277__astropy",
    "sweagent__sphinx-doc__sphinx-9698__astropy",
    "gru__django__django-7530__regressionfail",
    "gru__pallets__flask-5014__regressionfail",
    "honeycomb__pydata__xarray-4075__regressionfail",
    "sweagent__pylint-dev__pylint-6903__regressionfail",
)

# Human adjudication keyed by entry_id (reviewer-facing).
# Labels must stay aligned with ``docs/CLAIM_LOCK_NEURIPS2026.md`` / claim-lock
# prose: never emit deprecated ``true_positive`` tokens into ``docs/*.md``.
LABEL_ISSUE_WEAKNESS = "issue_relevant_candidate_weakness"
LABEL_STRESS_REVERSAL = "valid_stress_control_reversal"
LABEL_UNCLEAR_INFRA = "unclear_or_infrastructure_artifact"

ADJ: dict[str, dict[str, str]] = {
    "gru__astropy__astropy-7671__astropy": {
        "issue_relevance": "directly_issue_relevant",
        "human_label": LABEL_ISSUE_WEAKNESS,
        "confidence": "high",
        "reviewer_notes": (
            "Augmented pytest under -W error trips on the same repo as the "
            "reported minversion issue; failure reflects harness strictness, "
            "not an unrelated requirement."
        ),
    },
    "gru__django__django-7530__astropy": {
        "issue_relevance": "weakly_relevant",
        "human_label": LABEL_UNCLEAR_INFRA,
        "confidence": "medium",
        "reviewer_notes": (
            "Augmented selector targets Astropy modeling tests; on Django "
            "this is non-issue-aligned (validator non-applicability)."
        ),
    },
    "honeycomb__pylint-dev__pylint-7277__astropy": {
        "issue_relevance": "weakly_relevant",
        "human_label": LABEL_UNCLEAR_INFRA,
        "confidence": "medium",
        "reviewer_notes": (
            "Same cross-repo augmented command limitation; treat as "
            "cautionary, not definitive bad patch."
        ),
    },
    "sweagent__sphinx-doc__sphinx-9698__astropy": {
        "issue_relevance": "weakly_relevant",
        "human_label": LABEL_ISSUE_WEAKNESS,
        "confidence": "medium",
        "reviewer_notes": (
            "Warnings-as-errors stress surfaces fragility beyond official "
            "rerun; relevant as strengthened-check signal on Sphinx."
        ),
    },
    "gru__django__django-7530__regressionfail": {
        "issue_relevance": "regression_relevant",
        "human_label": LABEL_STRESS_REVERSAL,
        "confidence": "medium",
        "reviewer_notes": (
            "Valid stress-control reversal: sealed bundle records "
            "L2_REGRESSION_FAIL on the regression arm including "
            "`regression_forced_fail` as predeclared in the Evaluator Card. "
            "This documents evaluator behavior, not a natural product "
            "regression on the ticket."
        ),
    },
    "gru__pallets__flask-5014__regressionfail": {
        "issue_relevance": "regression_relevant",
        "human_label": LABEL_STRESS_REVERSAL,
        "confidence": "medium",
        "reviewer_notes": (
            "Same stress-control reading as regression django-7530: harness "
            "protocol signal, not semantic regression adjudication."
        ),
    },
    "honeycomb__pydata__xarray-4075__regressionfail": {
        "issue_relevance": "not_relevant",
        "human_label": LABEL_UNCLEAR_INFRA,
        "confidence": "high",
        "reviewer_notes": (
            "Forced-fail regression subcheck; comparator protocol, not patch "
            "quality signal."
        ),
    },
    "sweagent__pylint-dev__pylint-6903__regressionfail": {
        "issue_relevance": "not_relevant",
        "human_label": LABEL_UNCLEAR_INFRA,
        "confidence": "high",
        "reviewer_notes": (
            "Forced-fail regression subcheck; protocol artifact only."
        ),
    },
}

ISSUE_CONTEXT: dict[str, str] = {
    "astropy__astropy-7671": (
        "Official SWE-bench issue: minversion comparison failures under "
        "LooseVersion edge cases."
    ),
    "django__django-7530": (
        "Django ticket fix evaluated on verified harness (official tests)."
    ),
    "pylint-dev__pylint-7277": (
        "Pylint change-set from verified flagship slice."
    ),
    "sphinx-doc__sphinx-9698": (
        "Sphinx documentation/build issue from verified flagship slice."
    ),
    "pallets__flask-5014": "Flask issue from verified flagship slice.",
    "pydata__xarray-4075": "xarray issue from verified flagship slice.",
    "pylint-dev__pylint-6903": "Pylint issue from verified flagship slice.",
}


def _build_review_rows() -> list[dict[str, Any]]:
    idx = _load_failures_index()
    gold = _gold_lookup()
    rows: list[dict[str, Any]] = []
    for eid in CURATED_ENTRY_IDS:
        entry = idx.get(eid)
        if entry is None:
            raise SystemExit(f"missing batch entry {eid} in {SUMMARY_PATH}")
        levels = entry.get("levels", {})
        if not isinstance(levels, dict):
            levels = {}
        l1 = levels.get("l1", {}) if isinstance(levels.get("l1"), dict) else {}
        l2 = levels.get("l2", {}) if isinstance(levels.get("l2"), dict) else {}
        bundle_name = str(entry.get("bundle_name", ""))
        bundle_dir = _bundle_dir_for_entry(eid, bundle_name)
        tid = _task_id(entry)
        pr = str(l2.get("primary_reason", ""))
        adj = ADJ.get(eid, {})
        gps = _gold_pass_display(tid, eid, gold)

        artifact = str(bundle_dir.relative_to(REPO_ROOT)).replace("\\", "/")

        rows.append(
            {
                "task_id": tid,
                "candidate_id": _candidate_id(bundle_dir),
                "agent_id": _agent_id(entry),
                "validator_family": pr,
                "l1_status": str(l1.get("status", "")),
                "l2_status": str(l2.get("status", "")),
                "l2_primary_reason": pr,
                "failure_summary": _failure_summary(bundle_dir),
                "issue_relevance": adj.get("issue_relevance", "unclear"),
                "gold_patch_passes_validator": gps,
                "human_label": adj.get("human_label", "unclear"),
                "confidence": adj.get("confidence", "low"),
                "reviewer_notes": adj.get("reviewer_notes", ""),
                "artifact_bundle": artifact,
            }
        )
    return rows


def _write_taxonomy_from_summary() -> None:
    summary = _load_json(SUMMARY_PATH)
    counts: dict[tuple[str, str], int] = {}
    for entry in summary.get("entries", []):
        if not isinstance(entry, dict):
            continue
        levels = entry.get("levels", {})
        if not isinstance(levels, dict):
            continue
        l2 = levels.get("l2", {}) if isinstance(levels.get("l2"), dict) else {}
        if str(l2.get("status", "")).lower() != "fail":
            continue
        eid = str(entry.get("entry_id", ""))
        vf = _family_from_entry_id(eid)
        pr = str(l2.get("primary_reason", ""))
        counts[(vf, pr)] = counts.get((vf, pr), 0) + 1
    with OUT_TAXONOMY.open("w", encoding="utf-8", newline="") as h:
        w = csv.DictWriter(
            h,
            fieldnames=["validator_family", "primary_reason", "count"],
        )
        w.writeheader()
        for (vf, reason), c in sorted(counts.items()):
            w.writerow(
                {
                    "validator_family": vf,
                    "primary_reason": reason,
                    "count": c,
                }
            )


def _write_review_outputs(rows: list[dict[str, Any]]) -> None:
    fields = [
        "task_id",
        "candidate_id",
        "agent_id",
        "validator_family",
        "l1_status",
        "l2_status",
        "l2_primary_reason",
        "failure_summary",
        "issue_relevance",
        "gold_patch_passes_validator",
        "human_label",
        "confidence",
        "reviewer_notes",
        "artifact_bundle",
    ]
    with OUT_REVIEW.open("w", encoding="utf-8", newline="") as h:
        w = csv.DictWriter(h, fieldnames=fields)
        w.writeheader()
        for row in rows:
            w.writerow({k: row.get(k, "") for k in fields})
    OUT_REVIEW_JSON.write_text(
        json.dumps(rows, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _write_case_studies(rows: list[dict[str, Any]]) -> None:  # noqa: PLR0915
    aug = sum(1 for r in rows if r["validator_family"] == "L2_AUG_TESTS_FAIL")
    reg = sum(1 for r in rows if r["validator_family"] == "L2_REGRESSION_FAIL")

    def _count_arm(label: str, vf: str) -> int:
        return sum(
            1
            for r in rows
            if r["human_label"] == label and r["validator_family"] == vf
        )

    n_issue_aug = _count_arm(LABEL_ISSUE_WEAKNESS, "L2_AUG_TESTS_FAIL")
    n_issue_reg = _count_arm(LABEL_ISSUE_WEAKNESS, "L2_REGRESSION_FAIL")
    n_stress_aug = _count_arm(LABEL_STRESS_REVERSAL, "L2_AUG_TESTS_FAIL")
    n_stress_reg = _count_arm(LABEL_STRESS_REVERSAL, "L2_REGRESSION_FAIL")
    n_unclear_aug = _count_arm(LABEL_UNCLEAR_INFRA, "L2_AUG_TESTS_FAIL")
    n_unclear_reg = _count_arm(LABEL_UNCLEAR_INFRA, "L2_REGRESSION_FAIL")

    lines: list[str] = []
    lines.append("# L2 failure case studies (L2 flagship primary cohort v1)")
    lines.append("")
    lines.append(
        "Human adjudication sample from frozen run results at "
        "`runs/released/l2_verified_flagship_v1/results/batch_summary.json` "
        "with reference-patch context from "
        "`paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv` "
        "when available."
    )
    lines.append("")
    lines.append(
        "The **regression stress-control arm** is a **negative-control / protocol** "
        "arm. Its reversals demonstrate **evaluator-induced score changes**, not "
        "natural product regressions. Rows that fail via `regression_forced_fail` "
        "are **protocol-control evidence**, not evidence that the upstream issue "
        "regressed in production."
    )
    lines.append("")
    lines.append("## Human review summary (diagnostic sample)")
    lines.append("")
    lines.append(
        "The review sample is **diagnostic** and **single-reviewer**; it is **not** "
        "used to estimate population-level semantic-defect rates."
    )
    lines.append("")
    lines.append("| Review label | Augmented tests | Regression stress-control | Total |")
    lines.append("|--------------|-----------------|---------------------------|-------|")
    lines.append(
        f"| Issue-relevant candidate weakness | {n_issue_aug} | {n_issue_reg} | "
        f"{n_issue_aug + n_issue_reg} |"
    )
    lines.append(
        f"| Valid stress-control reversal | {n_stress_aug} | {n_stress_reg} | "
        f"{n_stress_aug + n_stress_reg} |"
    )
    lines.append(
        f"| Unclear or infrastructure artifact | {n_unclear_aug} | {n_unclear_reg} | "
        f"{n_unclear_aug + n_unclear_reg} |"
    )
    lines.append("")
    lines.append("## Sample composition")
    lines.append("")
    lines.append(f"- Total reviewed: `{len(rows)}`")
    lines.append(f"- Augmented-test failures (`L2_AUG_TESTS_FAIL`): `{aug}`")
    lines.append(f"- Regression stress-control failures (`L2_REGRESSION_FAIL`): `{reg}`")
    lines.append(
        f"- Issue-relevant candidate weakness: `{n_issue_aug}` augmented cases"
    )
    lines.append(
        f"- Valid stress-control reversal: `{n_stress_reg}` regression-control cases "
        "(validator behaved according to its declared Evaluator Card; "
        "`regression_forced_fail` as designed)"
    )
    lines.append(
        f"- Unclear or infrastructure artifact: `{n_unclear_aug + n_unclear_reg}` cases"
    )
    lines.append("")
    lines.append(
        "Do **not** describe forced-fail regression rows as confirmations of natural "
        "product regression on the ticket. Use **protocol_control_reversal** / "
        "**stress_control_reversal** when referring to score reversals on that arm, "
        "or **valid stress-control reversal** when the outcome matches the "
        "predeclared control specification."
    )
    lines.append("")
    for i, row in enumerate(rows, start=1):
        vf = row["validator_family"]
        lines.append(f"## Case {i}: {row['task_id']} / {row['agent_id']}")
        lines.append("")
        lines.append(f"**Validator family:** `{vf}`  ")
        lines.append(f"**L1 verdict:** {row['l1_status']}  ")
        lines.append(f"**L2 verdict:** {row['l2_status']}  ")
        gp = row.get("gold_patch_passes_validator", "")
        lines.append(
            f"**Gold patch status:** {gp if gp else 'see gold_patch_validation.csv'}  "
        )
        lines.append("")
        lines.append("### Issue context")
        lines.append("")
        lines.append(ISSUE_CONTEXT.get(row["task_id"], "(see benchmark manifest)."))
        lines.append("")
        lines.append("### Candidate behavior")
        lines.append("")
        lines.append(
            "Candidate patch is the sealed agent submission for this task "
            "(see `artifact_bundle`)."
        )
        lines.append("")
        lines.append("### L2 failure")
        lines.append("")
        lines.append(row["failure_summary"])
        lines.append("")
        lines.append("### Why this is issue-relevant")
        lines.append("")
        lines.append(
            f"Issue relevance assessment: `{row['issue_relevance']}`."
        )
        lines.append("")
        lines.append("### Human adjudication")
        lines.append("")
        lines.append(
            f"`{row['human_label']}` (confidence `{row['confidence']}`)."
        )
        lines.append("")
        lines.append("### Evidence")
        lines.append("")
        lines.append(f"`{row['artifact_bundle']}`")
        lines.append("")
    lines.append("## Protocol note (regression arm)")
    lines.append("")
    lines.append(
        "Regression-family rows use `regression_forced_fail` in "
        "`strengthening_spec_regression_fail.json`. Interpret them through "
        "`docs/CLAIM_LOCK_NEURIPS2026.md` and the regression Evaluator Card "
        "(protocol-control / stress-control evidence)."
    )
    OUT_CASE_STUDIES.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    _write_taxonomy_from_summary()
    rows = _build_review_rows()
    _write_review_outputs(rows)
    _write_case_studies(rows)
    print(
        json.dumps(
            {
                "taxonomy_csv": str(OUT_TAXONOMY),
                "review_csv": str(OUT_REVIEW),
                "review_json": str(OUT_REVIEW_JSON),
                "case_studies": str(OUT_CASE_STUDIES),
                "sample_total": len(rows),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
