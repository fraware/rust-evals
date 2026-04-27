# Empirical evidence gates (live status)

See also the [documentation index](README.md) for how this file fits next to
the tranche plan and submission checklist.

This document records **machine-checked** outcomes for the NeurIPS evidence
tranche gates. Two CLI regimes exist:

- **Strict (default):** `check_evidence_quality` without `--gate-profile` uses
  headline NeurIPS thresholds from `docs/evidence_tranche_plan.md`.
- **Release profile:** `--gate-profile release` relaxes thresholds so the
  **currently sealed** repository bundles can exit zero while remediation
  batches are replanned. Strict runs remain the scientific bar; release is a
  Mode 1 repository-closure bar only.

## Phase 0 baseline (strict defaults, 2026-04-26)

Commands below use the repository root. All exited **2** except where noted.

| Tranche | Command | Outcome |
|--------|---------|--------|
| Verified prefclean | `python ci/scripts/check_evidence_quality.py verified --run-dir runs/released/agent_panel_v3_r1/results_verified_prefclean` | Fail: harness rate, distinct agent vectors |
| Verified opt | same with `results_opt` | Same metrics as prefclean (51 candidates) |
| Live export | `python ci/scripts/check_evidence_quality.py live --paper-export-dir paper/exports/live_panel_v1_postbatch` | Fail: tied live rates, tau `0.0` |
| L2 thin slice | `python ci/scripts/check_evidence_quality.py l2 --run-dir runs/released/l2_verified_v2/results` | Fail: counts below defaults |
| Rust seal strict | `python ci/scripts/check_evidence_quality.py rust-proof --run-dir runs/released/rust_proof_subset_v1/results_seal` | Fail: semantic minima (`l3-pass/l4-fail`, all-level pass) |

**Bundle integrity:** `eval-ladder verify run-dir --run-dir runs/released/agent_panel_v3_r1/results_opt` → `51 ok / 0 invalid`.

## Verified flagship (`check_evidence_quality verified`)

**Headline cleanup panel (materialised, awaiting fresh `results_opt`):**

- `runs/released/agent_panel_verified_flagship_v1/` — drops matplotlib,
  scikit-learn, and pytest-dev tasks from v3_r1, reuses its workspaces, and is
  meant to be evaluated with `configs/evaluator/verified_headline.toml` (deny-only
  L3 edit scope). See that directory’s README and `just verified-flagship-batch-optimized-prewarmed`.

**Canonical run directory (optimized batch, identical summary to prefclean):**

- `runs/released/agent_panel_v3_r1/results_opt/`
- Archived summary copy: `runs/released/agent_panel_v3_r1/evidence/batch_summary_results_opt_v1.json`
- Panel input: `runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl`

**Strict gate (headline science thresholds):**

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

- Public-agent L1-pass inventory from all in-repo summaries yields
  `7` tasks with pass evidence for all three public agents
  (`gru`, `honeycomb`, `sweagent`).
- This implies a current upper bound of `21` rows if taking one candidate per
  task-agent pair, below strict Verified `--min-candidates 30`.
- Conclusion: strict flagship is currently blocked by candidate/task inventory;
  further progress requires adding new L1-stable task families and/or new
  candidate rows, not additional reruns of the same panel mix.

## Live comparative (`check_evidence_quality live`)

- **v2 panel (asymmetric live patches):** `runs/released/live_panel_v2/`.
- Canonical export: `paper/exports/live_panel_v2_postbatch/`.
- `runs/released/live_panel_v2/results_opt` integrity: `verify run-dir` reports
  `31 ok / 0 invalid`.
- **Strict:** passes on v2 (non-tied live rates and non-zero Kendall tau).
- **Release:** also passes (strictly negative deltas preserved).
- Historical strict-failing export from v1 remains at:
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

**Canonical strict-pass run directory:**

- `runs/released/l2_verified_flagship_v1/results/` (merged from
  `results_astropy` + `results_regression_fail`).

**Strict gate (default thresholds):**

```bash
python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_flagship_v1/results
```

Current strict metrics (`ok: true`):

- `total_entries=66`
- `l1_passed_from=24`
- `l2_failures=24`
- `l2_reason_counts={L2_AUG_TESTS_FAIL: 12, L2_REGRESSION_FAIL: 12}`

Historical release-profile merge remains at:
`runs/released/l2_verified_merged_v1/results/`.

## Rust proof-subset (`check_evidence_quality rust-proof`)

- **Structural / tier-1:** `results_fast` with explicit `--min-l3-pass-l4-fail 0`
  (see `ci/scripts/run_evidence_tier1_checks.py`).
- **Seal directory:** `runs/released/rust_proof_subset_v1/results_seal/`
- **Paper semantics replay (L3 pass / L4 fail exemplars + all-level pass):**
  `docs/rust_proof_paper_semantics_replay.md` and
  `datasets/derived/proof_subset/manifest_paper_semantics_l4_counterexample.jsonl`
  with `just rust-proof-batch-seal-paper-semantics` writing to a **separate**
  `--out` directory (do not overwrite `results_seal`).

**Strict semantic target (future batch):**

```bash
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal \
  --expected-entries 8 \
  --min-l3-pass-l4-fail 2 \
  --min-all-level-pass 1
```

**Offline real-manifest status (no reruns):**

- `paper/exports/strict_feasibility_report.json` confirms the sealed
  real-manifest run has `l3_pass_l4_fail=0`, `all_level_pass=0`,
  and `L4_OBLIGATION_MET` on all 8 entries.
- Therefore strict Rust semantic minima are not currently met on natural
  evidence. The synthetic paper-semantics replay remains a mechanism test only,
  not a substitute for headline empirical claims.

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
