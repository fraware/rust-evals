#!/usr/bin/env python3
"""Pull OCI images referenced by a panel JSONL before a long batch run.

Each panel line must resolve to a benchmark task manifest JSON containing
``environment_ref``. Unique refs are collected once; only values that look like
Docker/OCI image names are passed to ``docker pull`` (for example Rust tasks may
use ``cargo://…``, which is skipped).

Requires ``docker`` on ``PATH``. Intended to hide cold-start image pulls from
batch wall-clock measurements.

SWE-bench Verified manifests may use a legacy image name (``org__repo-issue``).
The Docker engine in ``packages/rust/runner/src/container.rs`` tries that name
then a compatibility alias (``org_1776_repo-issue``); this script uses the same
candidate list so ``docker image inspect`` / ``docker pull`` match local images.

By default, failed pulls are logged briefly and the script still exits 0 (use
``--strict-pulls`` for full logs and a non-zero exit).
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import Any

# Must stay aligned with `image_pull_candidates` / `map_legacy_swebench_image` in
# `packages/rust/runner/src/container.rs`.
_SWEBENCH_EVAL_PREFIX = "swebench/sweb.eval.x86_64."


def map_legacy_swebench_image(image: str) -> str | None:
    """Map legacy ``org__repo-issue`` tag to canonical ``org_1776_repo-issue`` form."""
    if not image.startswith(_SWEBENCH_EVAL_PREFIX):
        return None
    rest = image[len(_SWEBENCH_EVAL_PREFIX) :]
    if ":" in rest:
        repo_segment, tag_segment = rest.split(":", 1)
    else:
        repo_segment, tag_segment = rest, ""
    if "__" not in repo_segment:
        return None
    head, tail = repo_segment.split("__", 1)
    mapped_repo = f"{head}_1776_{tail}"
    if not tag_segment:
        return f"{_SWEBENCH_EVAL_PREFIX}{mapped_repo}"
    return f"{_SWEBENCH_EVAL_PREFIX}{mapped_repo}:{tag_segment}"


def image_pull_candidates(image: str) -> list[str]:
    """Return docker pull / inspect names in the same order as the Rust runner."""
    out: list[str] = [image]
    mapped = map_legacy_swebench_image(image)
    if mapped is not None and mapped != image:
        out.append(mapped)
    return out


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path}: top-level JSON must be an object")
    return data


def collect_environment_refs(panel_path: Path) -> list[str]:
    """Return sorted unique ``environment_ref`` values for tasks in the panel."""
    base = panel_path.parent
    seen: set[str] = set()
    for raw_line in panel_path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        row = json.loads(line)
        if not isinstance(row, dict):
            raise TypeError("panel line must be a JSON object")
        task_rel = row.get("task")
        if not isinstance(task_rel, str) or not task_rel.strip():
            raise ValueError(f"missing or invalid 'task' in line: {raw_line[:120]!r}")
        task_path = (base / task_rel).resolve()
        if not task_path.is_file():
            raise FileNotFoundError(f"task manifest not found: {task_path}")
        manifest = _load_json(task_path)
        ref = manifest.get("environment_ref")
        if not isinstance(ref, str) or not ref.strip():
            raise ValueError(
                f"{task_path}: missing non-empty string field 'environment_ref'"
            )
        seen.add(ref.strip())
    return sorted(seen)


def partition_refs_for_docker_pull(refs: list[str]) -> tuple[list[str], list[str]]:
    """Split refs into (docker_pull_targets, skipped_non_docker).

    ``docker pull`` only accepts image references. Skip URL-like and Rust
    cargo environment schemes so prewarm does not fail the whole recipe.
    """
    pull: list[str] = []
    skipped: list[str] = []
    for r in refs:
        s = r.strip()
        low = s.lower()
        if low.startswith("cargo://"):
            skipped.append(r)
            continue
        if low.startswith("file://") or low.startswith("http://") or low.startswith(
            "https://"
        ):
            skipped.append(r)
            continue
        if low.startswith("oci:"):
            skipped.append(r)
            continue
        pull.append(r)
    return sorted(set(pull)), sorted(set(skipped))


def _docker_pull(ref: str) -> tuple[str, int, str]:
    proc = subprocess.run(
        ["docker", "pull", ref],
        capture_output=True,
        text=True,
        check=False,
    )
    tail = (proc.stderr or proc.stdout or "").strip()
    if len(tail) > 2000:
        tail = tail[-2000:]
    return ref, proc.returncode, tail


def _image_exists_locally(ref: str) -> bool:
    proc = subprocess.run(
        ["docker", "image", "inspect", ref],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode == 0


def _format_pull_failure(name: str, code: int, tail: str, verbose: bool) -> str:
    if verbose:
        return f"FAILED {name} exit={code}\n{tail}"
    first = tail.strip().split("\n", 1)[0] if tail.strip() else "(no message)"
    if len(first) > 220:
        first = first[:220] + "…"
    return f"FAILED {name} exit={code}: {first}"


def _ensure_manifest_image(manifest_ref: str, strict_pulls: bool) -> tuple[str, int, str]:
    """Resolve manifest ``environment_ref`` using runner-compatible pull candidates."""
    candidates = image_pull_candidates(manifest_ref)
    for cand in candidates:
        if _image_exists_locally(cand):
            if cand == manifest_ref:
                print(f"present {manifest_ref}", flush=True)
            else:
                print(
                    f"present {manifest_ref} (local image: {cand})",
                    flush=True,
                )
            return manifest_ref, 0, ""

    errors: list[str] = []
    last_code, last_tail = 1, ""
    for cand in candidates:
        print(f"pulling {cand} ...", flush=True)
        name, code, tail = _docker_pull(cand)
        last_code, last_tail = code, tail
        if code == 0:
            print(f"pulled {cand}", flush=True)
            return manifest_ref, 0, ""
        if strict_pulls:
            print(
                _format_pull_failure(name, code, tail, True),
                file=sys.stderr,
            )
        else:
            first = tail.strip().split("\n", 1)[0] if tail.strip() else "(no message)"
            if len(first) > 100:
                first = first[:100] + "…"
            errors.append(f"{cand}: {first}")

    if not strict_pulls and errors:
        joined = "; ".join(errors)
        if len(joined) > 400:
            joined = joined[:400] + "…"
        print(
            f"FAILED {manifest_ref} (tried {len(candidates)} name(s)): {joined}",
            file=sys.stderr,
        )
    return manifest_ref, last_code, last_tail


def _pull_all_images(
    pullable: list[str], parallel: int, strict_pulls: bool
) -> list[tuple[str, int, str]]:
    """Ensure each ref is available locally; return list of (ref, code, tail) failures."""
    failures: list[tuple[str, int, str]] = []
    workers = max(1, parallel)
    if workers == 1:
        for ref in pullable:
            name, code, tail = _ensure_manifest_image(ref, strict_pulls)
            if code != 0:
                failures.append((name, code, tail))
        return failures
    with ThreadPoolExecutor(max_workers=workers) as ex:
        futures = {ex.submit(_ensure_manifest_image, r, strict_pulls): r for r in pullable}
        for fut in as_completed(futures):
            name, code, tail = fut.result()
            if code != 0:
                failures.append((name, code, tail))
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(
        description="docker pull every unique environment_ref from a panel JSONL."
    )
    parser.add_argument(
        "--panel",
        type=Path,
        required=True,
        help="Path to panel.jsonl (task paths resolved relative to its parent).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print refs only; do not invoke docker.",
    )
    parser.add_argument(
        "--parallel",
        type=int,
        default=4,
        metavar="N",
        help="Max concurrent docker pulls (default: 4).",
    )
    parser.add_argument(
        "--strict-pulls",
        action="store_true",
        help="Exit non-zero when any docker pull fails. Default is best-effort: "
        "print failures and still exit 0 so local-only images do not break batches.",
    )
    args = parser.parse_args()

    try:
        refs = collect_environment_refs(args.panel)
    except (OSError, ValueError, TypeError, json.JSONDecodeError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 2

    if not refs:
        print("no environment_ref values found (empty panel?)", file=sys.stderr)
        return 2

    pullable, skipped = partition_refs_for_docker_pull(refs)
    print(f"unique environment_ref: {len(refs)}")
    for r in refs:
        print(f"  {r}")
    print(f"docker pull targets: {len(pullable)}")
    if skipped:
        print(
            f"skipped (not docker pull targets): {len(skipped)}",
            file=sys.stderr,
        )
        for r in skipped:
            print(f"  skip {r}", file=sys.stderr)

    if args.dry_run:
        return 0

    if not pullable:
        print("nothing to pull (no docker image refs); exiting ok", flush=True)
        return 0

    failures = _pull_all_images(pullable, args.parallel, args.strict_pulls)

    if failures:
        if args.strict_pulls:
            return 1
        print(
            f"prewarm: {len(failures)} image(s) missing locally and could not be pulled; "
            "exiting 0 (best-effort). Use --strict-pulls to fail, or build/cache images first.",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
