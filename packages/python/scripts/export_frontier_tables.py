#!/usr/bin/env python3
"""Feasibility and Rust proof frontier TeX for the paper bundle."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def _load_json(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError(f"{path} must be a JSON object")
    return data


def _count_manifest_lines(path: Path) -> int:
    if not path.is_file():
        return 0
    text = path.read_text(encoding="utf-8")
    return sum(1 for line in text.splitlines() if line.strip())


def export_frontier(
    repo_root: Path,
    feasibility_path: Path,
    proof_manifest: Path,
    tex_dir: Path,
) -> dict[str, Any]:
    tex_dir.mkdir(parents=True, exist_ok=True)
    feas = _load_json(feasibility_path)

    verified = feas.get("verified", {})
    inv = verified.get("inventory", {})
    thresh = verified.get("strict_thresholds", {})
    assessment = verified.get("assessment", {})

    vf = tex_dir / "verified_feasibility_bound.tex"
    vf.write_text(
        "\\begin{tabular}{ll}\n"
        "\\hline\n"
        "Quantity & Value \\\\\n"
        "\\hline\n"
        f"Unique task--agent pairs (inventory) & {inv.get('unique_task_agent_pairs', '')} \\\\\n"
        f"Unique tasks with any public-agent L1 pass & "
        f"{inv.get('unique_tasks_with_any_public_agent_pass', '')} \\\\\n"
        f"Min candidates threshold & {thresh.get('min_candidates', '')} \\\\\n"
        f"Supports min candidates without new tasks & "
        f"\\texttt{{{str(assessment.get('supports_min_candidates_without_new_tasks', ''))}}} \\\\\n"
        "\\hline\n"
        "\\end{tabular}\n",
        encoding="utf-8",
    )

    rust = feas.get("rust_real_manifest", {})
    rm = rust.get("metrics", {})
    rpath = tex_dir / "rust_proof_frontier.tex"
    rpath.write_text(
        "\\begin{tabular}{lr}\n"
        "\\hline\n"
        "Metric & Value \\\\\n"
        "\\hline\n"
        f"Sealed real-manifest entries & {rm.get('total_entries', '')} \\\\\n"
        f"L3-pass / L4-fail (real) & {rm.get('l3_pass_l4_fail', '')} \\\\\n"
        f"Strict semantic minima met & "
        f"\\texttt{{{rust.get('assessment', {}).get('strict_semantic_minima_met', '')}}} \\\\\n"
        "\\hline\n"
        "\\end{tabular}\n",
        encoding="utf-8",
    )

    n_lines = _count_manifest_lines(proof_manifest)
    art = tex_dir / "artifact_surfaces.tex"
    art.write_text(
        "\\begin{tabular}{ll}\n"
        "\\hline\n"
        "Surface & Notes \\\\\n"
        "\\hline\n"
        "Live panel v2 & Sealed \\texttt{live\\_panel\\_v2/results\\_opt} batch \\\\\n"
        "L2 verified flagship v1 & Merged astropy + regression arms; 66 rows \\\\\n"
        "Verified feasibility & Offline inventory from \\texttt{strict\\_feasibility\\_report.json} \\\\\n"
        f"Rust proof subset manifest & \\texttt{{manifest.jsonl}} lines = {n_lines} \\\\\n"
        "\\hline\n"
        "\\end{tabular}\n",
        encoding="utf-8",
    )

    return {
        "tex": {
            "verified_feasibility_bound": str(
                vf.relative_to(repo_root)
            ),
            "rust_proof_frontier": str(rpath.relative_to(repo_root)),
            "artifact_surfaces": str(art.relative_to(repo_root)),
        }
    }


def main() -> int:
    repo_root = Path(__file__).resolve().parents[3]
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--feasibility",
        type=Path,
        default=repo_root / "paper/exports/strict_feasibility_report.json",
    )
    p.add_argument(
        "--proof-manifest",
        type=Path,
        default=repo_root / "datasets/derived/proof_subset/manifest.jsonl",
    )
    p.add_argument("--tex-dir", type=Path, default=repo_root / "paper/tables")
    args = p.parse_args()
    meta = export_frontier(
        repo_root,
        args.feasibility.resolve(),
        args.proof_manifest.resolve(),
        args.tex_dir.resolve(),
    )
    print(json.dumps(meta, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
