#!/usr/bin/env python3
"""Validate paper claim sources against frozen exports (NeurIPS claim lock).

Reads ``paper/paper_claim_sources.json`` (repo root). Asserts required sources
exist, headline Live/L2 paths use the canonical v2 / flagship directories,
forbidden legacy or synthetic headline paths are not referenced, and optional
YAML mirror plus doc guards hold (case studies, selection protocol,
``scientific_scope`` Tier D phrases, gold-validation paper wording).
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast

yaml_module: Any = None
try:
    import yaml as _yaml
except ImportError:
    pass
else:
    yaml_module = _yaml


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _load_map(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError("paper_claim_sources.json top-level must be an object")
    return cast(dict[str, Any], data)


def _canonical_payload(obj: Any) -> dict[str, Any]:
    """Round-trip through JSON for stable comparison with YAML loader output."""
    return cast(dict[str, Any], json.loads(json.dumps(obj, sort_keys=True)))


def _yaml_sync_failures(root: Path, json_cfg: dict[str, Any]) -> list[str]:
    yaml_path = root / "paper" / "paper_claim_sources.yaml"
    if not yaml_path.is_file():
        return ["missing paper/paper_claim_sources.yaml (mirror of JSON for editors)"]
    if yaml_module is None:  # pragma: no cover - optional dependency
        return [
            "PyYAML not installed; install dev deps "
            "(pip install -e '.[dev]') to validate paper_claim_sources.yaml"
        ]
    loaded = yaml_module.safe_load(yaml_path.read_text(encoding="utf-8"))
    if not isinstance(loaded, dict):
        return ["paper/paper_claim_sources.yaml top-level must be a mapping"]
    if _canonical_payload(loaded) != _canonical_payload(json_cfg):
        return ["paper/paper_claim_sources.yaml does not match paper/paper_claim_sources.json"]
    return []


# Lowercase substrings that must not appear in ``scientific_scope.md`` (Tier D-style overclaim).
_SCIENTIFIC_SCOPE_FORBIDDEN_SUBSTRINGS: tuple[str, ...] = (
    "overstate semantically justified issue resolution",
    "eval-ladder proves patches semantically correct",
    "the true semantic failure rate",
    "all l2 failures are false successes",
    "l3/l4 are central empirical results",
)


def _doc_guards(root: Path) -> list[str]:
    """Reject deprecated adjudication tokens that invited regression-arm overclaim."""
    failures: list[str] = []
    scope_path = root / "docs" / "scientific_scope.md"
    if scope_path.is_file():
        lowered = scope_path.read_text(encoding="utf-8").lower()
        for bad in _SCIENTIFIC_SCOPE_FORBIDDEN_SUBSTRINGS:
            if bad in lowered:
                failures.append(
                    f"docs/scientific_scope.md must not contain forbidden phrase {bad!r} "
                    "(see docs/CLAIM_LOCK_NEURIPS2026.md Tier D)"
                )
    gold_doc = root / "docs" / "l2_gold_patch_validation.md"
    if gold_doc.is_file():
        gtext = gold_doc.read_text(encoding="utf-8")
        if "## Paper wording" not in gtext:
            failures.append(
                "docs/l2_gold_patch_validation.md must retain a '## Paper wording' section "
                "(validator legitimacy framing)"
            )
        if "semantic defect" not in gtext.lower():
            failures.append(
                "docs/l2_gold_patch_validation.md must keep gold-vs-candidate semantics caveat "
                "(e.g. not every candidate L2 failure is a semantic defect)"
            )
    case_path = root / "docs" / "l2_failure_case_studies.md"
    if case_path.is_file():
        text = case_path.read_text(encoding="utf-8")
        if "true_positive" in text:
            failures.append(
                "docs/l2_failure_case_studies.md must not contain token true_positive "
                "(use stress-control / protocol-control labels)"
            )
    proto = root / "docs" / "l2_selection_protocol.md"
    if proto.is_file():
        ptext = proto.read_text(encoding="utf-8")
        if "regression_forced_fail" not in ptext:
            failures.append(
                "docs/l2_selection_protocol.md must mention regression_forced_fail "
                "(protocol-control interpretation)"
            )
        if "validator-focused" not in ptext:
            failures.append(
                "docs/l2_selection_protocol.md must keep validator-focused/diagnostic framing"
            )
    return failures


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
    if (
        name in {"live_static_counts", "live_leave_one_out"}
        and live_prefix
        and not norm.startswith(live_prefix)
    ):
        out.append(f"claim {name}: central Live source must live under {headline_live}/")

    l2_prefix = headline_l2.rstrip("/") + "/" if headline_l2 else ""
    if (
        name
        in {
            "l2_flagship_counts",
            "l2_flagship_arm_breakdown",
            "l2_gold_validation",
        }
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
    failures.extend(_yaml_sync_failures(root, cfg))
    failures.extend(_doc_guards(root))

    frontier_claims = {"verified_inventory_bound", "rust_l4_frontier"}
    for name, spec in claims.items():
        if not isinstance(spec, dict):
            failures.append(f"claim {name}: spec must be object")
            continue
        tier = spec.get("claim_tier", "")
        if name in frontier_claims and tier != "frontier":
            failures.append(
                f"claim {name}: claim_tier must be 'frontier' "
                f"(found {tier!r}); headline tables must not reuse this source"
            )
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
