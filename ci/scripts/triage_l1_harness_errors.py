#!/usr/bin/env python3
"""Cluster L1_HARNESS_ERROR cases by stderr signature for systematic triage.

Reads ``batch_summary.json`` under ``--run-dir``, finds entries whose L1
``primary_reason`` is ``L1_HARNESS_ERROR``, groups ``stderr.log`` content by
SHA-256 and by a small set of heuristic buckets, and prints a deterministic
JSON report to stdout.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path
from typing import Any


def _load_summary(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _l1_reason(entry: dict[str, Any]) -> str:
    levels = entry.get("levels", {})
    if not isinstance(levels, dict):
        return ""
    l1 = levels.get("l1", {})
    if not isinstance(l1, dict):
        return ""
    return str(l1.get("primary_reason", ""))


def _stderr_path(run_dir: Path, bundle_name: str) -> Path:
    return run_dir / bundle_name / "stderr.log"


def _classify_bucket(text: str) -> str:
    lower = text.lower()
    if not text.strip():
        return "empty_stderr"
    if "no module named pytest" in lower:
        return "missing_pytest"
    if "cannot import name '_c_internal_utils'" in lower or (
        "matplotlib" in lower and "partially initialized module" in lower
    ):
        return "matplotlib_native_extension_unbuilt"
    if "sklearn" in lower and "__check_build" in lower:
        return "sklearn_native_build_missing"
    if "astropy" in lower and (
        "extension modules" in lower or "file not found:" in lower
    ):
        return "astropy_extension_or_selector"
    if "numpy.ndarray size changed" in lower or "binary incompatibility" in lower:
        return "numpy_abi_mismatch"
    if "error: not found:" in lower or "no match in any of" in lower:
        return "pytest_selector_not_found"
    if "modulenotfounderror" in lower or "importerror" in lower:
        return "import_error"
    if "cannot connect to the docker" in lower or "docker daemon" in lower:
        return "docker_daemon"
    if "pull access denied" in lower or "manifest unknown" in lower:
        return "image_pull"
    if "timed out" in lower or "deadline exceeded" in lower:
        return "timeout_hint"
    if "permission denied" in lower or "access is denied" in lower:
        return "permission"
    if "unittest.loader._failedtest" in lower and "attributeerror" in lower:
        return "unittest_missing_method"
    return "other"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--run-dir",
        type=Path,
        required=True,
        help="batch run directory containing batch_summary.json and bundles",
    )
    ap.add_argument(
        "--max-sample-chars",
        type=int,
        default=400,
        help="max characters of stderr to keep per cluster sample",
    )
    args = ap.parse_args()

    run_dir = args.run_dir.resolve()
    summary_path = run_dir / "batch_summary.json"
    if not summary_path.is_file():
        raise SystemExit(f"missing {summary_path}")

    summary = _load_summary(summary_path)
    entries = summary.get("entries", [])
    if not isinstance(entries, list):
        raise SystemExit("batch_summary entries must be a list")

    harness_entries: list[dict[str, Any]] = []
    for e in entries:
        if e.get("status") != "ok":
            continue
        if _l1_reason(e) != "L1_HARNESS_ERROR":
            continue
        harness_entries.append(e)

    by_sha: dict[str, dict[str, Any]] = {}
    by_bucket: dict[str, list[str]] = defaultdict(list)

    for e in harness_entries:
        bundle = str(e.get("bundle_name", ""))
        if not bundle:
            continue
        sp = _stderr_path(run_dir, bundle)
        if not sp.is_file():
            raw = b""
        else:
            raw = sp.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        text = raw.decode("utf-8", errors="replace")
        bucket = _classify_bucket(text)
        by_bucket[bucket].append(bundle)

        if digest not in by_sha:
            sample = text[: args.max_sample_chars]
            if len(text) > args.max_sample_chars:
                sample += "\n... [truncated]"
            by_sha[digest] = {
                "sha256": digest,
                "bucket": bucket,
                "byte_len": len(raw),
                "n_bundles": 0,
                "example_bundles": [],
                "stderr_sample": sample,
            }
        rec = by_sha[digest]
        rec["n_bundles"] += 1
        if len(rec["example_bundles"]) < 5:
            rec["example_bundles"].append(bundle)

    # Sort clusters by frequency descending for reviewer-first ordering.
    clusters = sorted(by_sha.values(), key=lambda c: (-c["n_bundles"], c["sha256"]))

    bucket_counts = {k: len(v) for k, v in sorted(by_bucket.items())}

    report = {
        "run_dir": str(run_dir),
        "total_entries": len(entries),
        "l1_harness_error_entries": len(harness_entries),
        "distinct_stderr_sha256": len(by_sha),
        "bucket_bundle_counts": bucket_counts,
        "clusters": clusters,
    }
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
