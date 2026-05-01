# Claim source map (NeurIPS 2026)

Each **paper claim** must trace to **exactly one** canonical artifact path.
Manuscript tables must cite generated CSV/JSON only; do not hand-edit counts.
Internal wiring for CI: `paper/paper_claim_sources.json` and
`ci/scripts/check_paper_claim_sources.py`.

## Live v2 (central diagnostic)

| Role | Path |
|------|------|
| Released panel | `runs/released/live_panel_v2/` |
| Sealed bundles | `runs/released/live_panel_v2/results_opt/` |
| Paper export | `paper/exports/live_panel_v2_postbatch/` |

Transparency exports (regenerated with `export_live_panel_tables.py` / `just reproduce-paper-tables`):

- `live_panel_summary_with_ci.csv` — static/live numerators, denominators, Wilson intervals.
- `live_leave_one_out.csv` — leave-one-out sensitivity (also via `ci/scripts/live_panel_leave_one_out.py`).
- `live_integrity_summary.json` — `batch_summary.json` integrity fields + verify command.
- `live_rows_L0_or_L1_invalid.csv` — live tasks whose official verdict is `invalid` at L0/L1.
- `per_task_live_outcomes.csv` — full per-task provenance table.

Gate:

```bash
python ci/scripts/check_evidence_quality.py live \
  --paper-export-dir paper/exports/live_panel_v2_postbatch
```

Integrity:

```bash
target/release/eval-ladder verify run-dir \
  --run-dir runs/released/live_panel_v2/results_opt
```

**Allowed (main text):** In a small diagnostic Live panel, static-anchor pass
rates can overstate observed live outcomes.

**Not allowed:** “We estimate live coding-agent robustness.”

**Appendix / sensitivity:** leave-one-out table
`paper/exports/live_panel_v2_postbatch/live_leave_one_out.csv` (claim map key
`live_leave_one_out` in `paper/paper_claim_sources.json`).

## L2 flagship (central diagnostic)

| Role | Path |
|------|------|
| Sealed bundles | `runs/released/l2_verified_flagship_v1/results/` |
| Paper export | `paper/exports/l2_verified_flagship_v1/` |

Arm-separated headline table (CSV + TeX):

- `paper/exports/l2_verified_flagship_v1/l2_flagship_arm_breakdown.csv`
- `paper/tables/l2_flagship_arm_breakdown.tex`

Gate:

```bash
python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_flagship_v1/results
```

**Paper-export manifest (`manifest.json`).** The merged flagship `results/` tree carries
`batch_summary.json` but no per-bundle leaves, so Rust `eval-ladder analyze paper-export`
sets `input_row_count` to `0`. After analysis, always run
`packages/python/scripts/export_l2_flagship_tables.py` (invoked from
`packages/python/scripts/reproduce_paper_tables.py`) so `manifest.json` lists the ten
standard export siblings with correct SHA-256 hashes and an `input_row_count` matching
`len(batch_summary.entries)` (66 for the frozen cohort).

Per-bundle integrity for the flagship cohort is checked on each **arm**
directory (the merged `results/` tree holds the joined `batch_summary.json`):

```bash
target/release/eval-ladder verify run-dir \
  --run-dir runs/released/l2_verified_flagship_v1/results_astropy
target/release/eval-ladder verify run-dir \
  --run-dir runs/released/l2_verified_flagship_v1/results_regression_fail
```

**Allowed:** L1-passing entries can reverse under strengthened validators;
augmented-test and regression stress-control arms must be interpreted
separately.

**Preferred summary table (arms):**

| Arm | Entries | L1-pass entries | L1-pass / L2-fail | Interpretation |
|-----|---------|-----------------|-------------------|----------------|
| Augmented tests | 33 | 12 | 12 | Issue-relevant diagnostic; reviewed subset mixed |
| Regression stress-control | 33 | 12 | 12 | Negative-control / protocol signal |
| Total | 66 | 24 | 24 | Evaluator sensitivity, not bug prevalence |

## Verified feasibility (evidence frontier)

| Role | Path |
|------|------|
| Offline bound | `paper/exports/strict_feasibility_report.json` |

Regenerate: `python ci/scripts/analyze_strict_feasibility.py`

**Allowed:** The current public candidate inventory bounds strict three-agent
Verified comparison below our predeclared threshold.

**Not allowed:** “Verified comparison fails because the evaluator cannot run.”

## Rust proof subset (evidence frontier)

| Role | Path |
|------|------|
| Sealed bundles | `runs/released/rust_proof_subset_v1/results_seal/` |
| Paper export | `paper/exports/rust_proof_subset_v1_seal_release/` |
| Manifest | `datasets/derived/proof_subset/manifest.jsonl` |

Gate:

```bash
python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal
```

Do **not** use paper-semantics counterexample manifests as headline empirical
evidence (`docs/rust_proof_paper_semantics_replay.md`).

**Allowed:** L4 is implemented and auditable on a curated Rust proof subset, but
current natural evidence does not yet provide semantic separation.

## Gold patch validation (L2 harness legitimacy)

| Role | Path |
|------|------|
| Summary | `paper/exports/l2_verified_flagship_v1/gold_patch_validation_summary.json` |

Regenerate: `python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2`
(headline profile: gold-mechanical; not `--strict-flagship-specs` for main paper).
