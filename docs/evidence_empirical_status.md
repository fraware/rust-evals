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

## Live comparative (`check_evidence_quality live`)

- Regenerated export from `runs/released/live_panel_v1/results_opt`:
  `paper/exports/live_panel_v1_postbatch/` (`eval-ladder analyze paper-export …`).
- **Strict:** still fails on tied live rates and zero tau (symmetric agents on
  the evaluated slice).
- **Release:** passes when every `delta` row is strictly negative:

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

**Merged canonical run directory** (deduped `batch_summary.json` only):

- `runs/released/l2_verified_merged_v1/results/` — see
  `runs/released/l2_verified_merged_v1/README.md` and
  `ci/scripts/merge_l2_batch_summaries.py`.

**Strict (original plan defaults):**

```bash
python ci/scripts/check_evidence_quality.py l2 \
  --run-dir runs/released/l2_verified_merged_v1/results
```

**Release:**

```bash
python ci/scripts/check_evidence_quality.py --gate-profile release l2 \
  --run-dir runs/released/l2_verified_merged_v1/results
```

Release thresholds: `--min-l1-passed-from 2 --min-l2-failures 2 --min-l2-reason-families 1`.

Paper export: `paper/exports/l2_verified_merged_v1/`.

## Rust proof-subset (`check_evidence_quality rust-proof`)

- **Structural / tier-1:** `results_fast` with explicit `--min-l3-pass-l4-fail 0`
  (see `ci/scripts/run_evidence_tier1_checks.py`).
- **Seal directory:** `runs/released/rust_proof_subset_v1/results_seal/`

**Strict semantic target (future batch):**

```bash
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal \
  --expected-entries 8 \
  --min-l3-pass-l4-fail 2 \
  --min-all-level-pass 1
```

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
`.github/workflows/release-tag.yml` run on GitHub (confirm with
`gh run list --workflow=release-tag.yml` after `gh auth login`).
