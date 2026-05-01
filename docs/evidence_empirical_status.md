# Empirical evidence gates (live status)

**NeurIPS 2026 claim lock:** Authoritative allowed/prohibited claims for the
submission package are listed in `docs/CLAIM_LOCK_NEURIPS2026.md`, with a
per-surface source map in `paper/exports/CLAIM_SOURCE_MAP.md`.

**Claim alignment:** L3 and L4 are real surfaces in the evaluator
but **not** the two central quantitative pillars of the current paper. The
**primary** frozen evidence is the **Live v2** static-vs-live panel and the
**L2 flagship** batch. **Synthetic** L4 counterexample or
broken-obligation **replays** are for **regression / mechanism** testing only
and are **out of scope** for headline pass/fail statistics. This matches
`README.md` and `docs/proof_subset_policy.md`.

See also the [documentation index](README.md) for how this file fits next to
the tranche plan and submission checklist.

This document records **machine-checked** outcomes for the publication evidence
tranche gates. Two CLI regimes exist:

- **Publication-threshold (default):** `check_evidence_quality` without `--gate-profile` uses
  headline publication thresholds from `docs/evidence_tranche_plan.md`.
- **Release profile:** `--gate-profile release` relaxes thresholds so the
  **currently frozen** repository bundles can exit zero while remediation
  batches are replanned. Publication-threshold runs remain the scientific bar; release is a
  repository-closure bar only.

## Phase 0 baseline (publication-threshold defaults, 2026-04-26)

Commands below use the repository root. All exited **2** except where noted.

| Tranche | Command | Outcome |
|--------|---------|--------|
| Verified preflight-clean | `python ci/scripts/check_evidence_quality.py verified --run-dir runs/released/agent_panel_v3_r1/results_verified_prefclean` | Fail: harness rate, distinct agent vectors |
| Verified optimized | same with `results_opt` | Same metrics as preflight-clean (51 candidates) |
| Live export | `python ci/scripts/check_evidence_quality.py live --paper-export-dir paper/exports/live_panel_v1_postbatch` | Fail: tied live rates, tau `0.0` |
| L2 thin slice | `python ci/scripts/check_evidence_quality.py l2 --run-dir runs/released/l2_verified_v2/results` | Fail: counts below defaults |
| Rust seal strict | `python ci/scripts/check_evidence_quality.py rust-proof --run-dir runs/released/rust_proof_subset_v1/results_seal` | Fail: semantic minima (`l3-pass/l4-fail`, all-level pass) |

**Bundle integrity:** `eval-ladder verify run-dir --run-dir runs/released/agent_panel_v3_r1/results_opt` → `51 ok / 0 invalid`.

## Verified primary evaluation cohort (`check_evidence_quality verified`)

**Headline cleanup panel (materialised, awaiting fresh `results_opt`):**

- `runs/released/agent_panel_verified_flagship_v1/` — drops matplotlib,
  scikit-learn, and pytest-dev tasks from v3_r1, reuses its workspaces, and is
  meant to be evaluated with `configs/evaluator/verified_headline.toml` (deny-only
  L3 edit scope). See that directory’s README and `just verified-flagship-batch-optimized-prewarmed`.

**Canonical run directory (optimized batch, identical summary to preflight-clean):**

- `runs/released/agent_panel_v3_r1/results_opt/`
- Archived summary copy: `runs/released/agent_panel_v3_r1/evidence/batch_summary_results_opt_v1.json`
- Panel input: `runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl`

**Publication-threshold gate (headline science thresholds):**

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_v3_r1/results_opt \
  --min-candidates 30 --max-l1-harness-error-rate 0.10 \
  --min-distinct-agents 2 --min-nonzero-agents 2 \
  --max-l3-single-reason-share 0.80
```

**Release profile (repository closure):**

```bash
python ci/scripts/check_evidence_quality.py --gate-profile release verified \
  --run-dir runs/released/agent_panel_v3_r1/results_opt
```

Release thresholds: `max_l1_harness_error_rate=0.80`, `min_distinct_agents=1`.

**Harness triage (unchanged):**

```bash
python ci/scripts/triage_l1_harness_errors.py \
  --run-dir runs/released/agent_panel_v3_r1/results_opt
```

**Offline feasibility bound (no reruns):**

```bash
python ci/scripts/analyze_strict_feasibility.py
```

Current report: `paper/exports/strict_feasibility_report.json`.

| Quantity | Value |
|----------|-------|
| Shared L1-pass tasks across three agents | 7 |
| One-candidate task-agent upper bound | 21 |
| Strict threshold | 30 |
| Status | inventory-bound frontier |

- Public-agent L1-pass inventory from all in-repo summaries yields
  `7` tasks with pass evidence for all three public agents
  (IDs documented in the run summaries).
- This implies a current upper bound of `21` rows if taking one candidate per
  task-agent pair, below strict Verified `--min-candidates 30`.
- Conclusion: the publication-threshold primary evaluation cohort target is currently blocked by candidate/task inventory;
  further progress requires adding new L1-stable task families and/or new
  candidate rows, not additional reruns of the same panel mix.

**Paper wording (Verified frontier):** The evaluator is capable of running the
comparison, but the **public candidate inventory** in the frozen artifact does
not support a strict three-agent Verified comparison under our predeclared
threshold. This is evidence-gated reporting (inventory-bound frontier), not an
evaluator-runtime failure claim.

## Live comparative (`check_evidence_quality live`)

- **v2 panel (asymmetric live patches):** `runs/released/live_panel_v2/`.
- Canonical export: `paper/exports/live_panel_v2_postbatch/`.
- `runs/released/live_panel_v2/results_opt` integrity: `verify run-dir` reports
  `31 ok / 0 invalid`.
- **Publication-threshold:** passes on v2 (non-tied live rates and non-zero Kendall tau).
- **Release:** also passes (strictly negative deltas preserved).
- Historical publication-threshold-failing export from v1 remains at:
  `paper/exports/live_panel_v1_postbatch/`.

```bash
python ci/scripts/check_evidence_quality.py --gate-profile release live \
  --paper-export-dir paper/exports/live_panel_v1_postbatch
```

Optional explicit flag (same live logic as release for this sub-gate):

```bash
python ci/scripts/check_evidence_quality.py live \
  --paper-export-dir paper/exports/live_panel_v1_postbatch \
  --symmetric-live-ok
```

## L2 expansion (`check_evidence_quality l2`)

**Canonical publication-threshold-pass run directory:**

- `runs/released/l2_verified_flagship_v1/results/` (merged from
  `results_astropy` + `results_regression_fail`).

**Publication-threshold gate (default thresholds):**

```bash
python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_flagship_v1/results
```

Current publication-threshold metrics (`ok: true`):

- `total_entries=66`
- `l1_passed_from=24`
- `l2_failures=24`
- `l2_reason_counts={L2_AUG_TESTS_FAIL: 12, L2_REGRESSION_FAIL: 12}`

Historical release-profile merge remains at:
`runs/released/l2_verified_merged_v1/results/`.

### Gold-patch validator legitimacy check (W1)

Canonical exports:

- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.json`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation_summary.json`

Regeneration command:

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2
```

Protocol highlights:

- Uses same evaluator config/mode (`configs/evaluator/default.toml`,
  `tests_plus_regression`) as primary-cohort L2 runs.
- Uses pre-declared `strengthening_spec_gold_mechanical.json` for headline reference-patch
  validity so publication-threshold-arm artifacts (`regression_forced_fail`, cross-repo
  Astropy selectors) do not dominate gold outcomes.
- Defines headline denominator explicitly as rows where gold passes L0 and L1.

Current summary (`gold_patch_validation_summary.json`):

- `L2_AUG_TESTS_FAIL.eligible_L0_L1_pass.gold_pass_rate = 1.0`
- `L2_REGRESSION_FAIL.eligible_L0_L1_pass.gold_pass_rate = 1.0`

Diagnostic publication-threshold replay remains available:

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --strict-flagship-specs --jobs 2
```

Use this replay for parity debugging only; do not treat its raw pass rate as
the headline validator-legitimacy estimate.

## Rust proof-subset (`check_evidence_quality rust-proof`)

- **Structural / tier-1:** `results_fast` with explicit `--min-l3-pass-l4-fail 0`
  (see `ci/scripts/run_evidence_tier1_checks.py`).
- **Seal directory:** `runs/released/rust_proof_subset_v1/results_seal/`
- **Paper semantics replay (L3 pass / L4 fail exemplars + all-level pass):**
  `docs/rust_proof_paper_semantics_replay.md` and
  `datasets/derived/proof_subset/manifest_paper_semantics_l4_counterexample.jsonl`
  with `just rust-proof-batch-seal-paper-semantics` writing to a **separate**
  `--out` directory (do not overwrite `results_seal`).

**Publication-threshold semantic target (future batch):**

```bash
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal \
  --expected-entries 8 \
  --min-l3-pass-l4-fail 2 \
  --min-all-level-pass 1
```

**Offline real-manifest status (no reruns):**

- `paper/exports/strict_feasibility_report.json` confirms the frozen
  real-manifest run has `l3_pass_l4_fail=0`, `all_level_pass=0`,
  and `L4_OBLIGATION_MET` on all 8 entries.
- Therefore publication-threshold Rust semantic minima are not currently met on natural
  evidence. The synthetic paper-semantics replay remains a mechanism test only,
  not a substitute for headline empirical claims.

**Paper wording (Rust frontier):** The Rust proof subset demonstrates that
task-specific semantic obligations can be attached and checked, but the current
real sealed manifest does not yet provide natural L3-pass/L4-fail separations.
Avoid claiming “L4 reveals semantic overstatement” from natural sealed rows alone.

**Release (structural seal on full ladder output):**

```bash
python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal
```

Paper export: `paper/exports/rust_proof_subset_v1_seal_release/`.

## Engineering gates (local)

On the development machine used for closure work:

- `cargo deny check` exited 0 (warnings only: duplicate lockfile entries,
  unused license allow-list entries).
- `cargo audit` exited 0.

## Release manifest (local prep)

```bash
python ci/scripts/write_release_artifact_manifest.py \
  --out paper/exports/release/v0.1.2/artifact_manifest.json
```

Pushing a new `v*.*.*` tag still requires a green
`.github/workflows/release-tag.yml` run on GitHub; see
`docs/github_release_tag_ci_confirmation.md`.
