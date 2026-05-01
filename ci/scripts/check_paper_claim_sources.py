#!/usr/bin/env python3
"""Validate paper claim sources against frozen exports (NeurIPS claim lock).

Reads ``paper/paper_claim_sources.json`` (repo root). Asserts required sources
exist, headline Live/L2 paths use the canonical v2 / flagship directories, and
forbidden legacy or synthetic headline paths are not referenced.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load_map(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError("paper_claim_sources.json top-level must be an object")
    return cast(dict[str, Any], data)


def _claim_failures(
    name: str,
    spec: dict[str, Any],
    root: Path,
    forbidden: list[Any],
    headline_live: str,
    headline_l2: str,
) -> list[str]:
    out: list[str] = []
    src = spec.get("source", "")
    if not isinstance(src, str) or not src:
        out.append(f"claim {name}: missing source string")
        return out

    norm = src.replace("\\", "/")
    for bad in forbidden:
        if isinstance(bad, str) and bad and bad in norm:
            out.append(
                f"claim {name}: source {norm!r} contains forbidden substring {bad!r}"
            )

    full = root / src
    if spec.get("required") and not full.is_file():
        out.append(f"claim {name}: missing required file {full}")

    tier = spec.get("claim_tier", "")
    if tier != "central":
        return out

    live_prefix = headline_live.rstrip("/") + "/" if headline_live else ""
    if name == "live_static_counts" and live_prefix and not norm.startswith(live_prefix):
        out.append(f"claim {name}: central Live source must live under {headline_live}/")

    l2_prefix = headline_l2.rstrip("/") + "/" if headline_l2 else ""
    if (
        name in {"l2_flagship_counts", "l2_gold_validation"}
        and l2_prefix
        and not norm.startswith(l2_prefix)
    ):
        out.append(f"claim {name}: central L2 source must live under {headline_l2}/")

    return out


def main() -> int:
    root = _repo_root()
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--map",
        type=Path,
        default=root / "paper" / "paper_claim_sources.json",
        help="Path to paper_claim_sources.json",
    )
    args = p.parse_args()
    cfg_path = args.map
    if not cfg_path.is_file():
        print(f"check_paper_claim_sources: missing {cfg_path}", file=sys.stderr)
        return 1

    cfg = _load_map(cfg_path)
    claims = cfg.get("claims", {})
    if not isinstance(claims, dict):
        print("check_paper_claim_sources: claims must be an object", file=sys.stderr)
        return 1

    raw_forbidden = cfg.get("forbidden_headline_path_substrings", [])
    forbidden = raw_forbidden if isinstance(raw_forbidden, list) else []

    headline_live = str(cfg.get("headline_live_export_dir", "")).replace("\\", "/")
    headline_l2 = str(cfg.get("headline_l2_export_dir", "")).replace("\\", "/")

    failures: list[str] = []
    for name, spec in claims.items():
        if not isinstance(spec, dict):
            failures.append(f"claim {name}: spec must be object")
            continue
        failures.extend(
            _claim_failures(name, spec, root, forbidden, headline_live, headline_l2)
        )

    if failures:
        for line in failures:
            print(f"check_paper_claim_sources: FAIL: {line}", file=sys.stderr)
        return 1

    print("check_paper_claim_sources: OK", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
