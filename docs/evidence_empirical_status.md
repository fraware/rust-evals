# Empirical evidence gates (live status)

This document records **machine-checked** outcomes for the NeurIPS evidence
tranche gates. Checklist items in `docs/submission_checklist.md` that refer to
headline empirical passes should only be flipped when the commands here exit
zero on the stated paths.

## Verified flagship (`check_evidence_quality verified`)

**Canonical sealed batch (preflight-clean panel, 51 candidates):**

- Run directory: `runs/released/agent_panel_v3_r1/results_verified_prefclean/`
- Summary copy: `runs/released/agent_panel_v3_r1/evidence/batch_summary_prefclean_v1.json`
- Panel input: `runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl`

**Gate command (defaults as in `docs/evidence_tranche_plan.md`):**

```bash
python ci/scripts/check_evidence_quality.py verified \
  --run-dir runs/released/agent_panel_v3_r1/results_verified_prefclean \
  --min-candidates 30 --max-l1-harness-error-rate 0.10 \
  --min-distinct-agents 2 --min-nonzero-agents 2 \
  --max-l3-single-reason-share 0.80
```

**Gate script note:** `check_evidence_quality verified` registers every agent
even when no row has an L0/L1 pass so degenerate panels report
`distinct_agent_pass_vectors` correctly (regression tests in
`tests/python/test_evidence_cli_scripts.py`).

**Observed failure modes (latest sealed run):**

- `L1_HARNESS_ERROR` rate above 10 percent (dominant buckets: pytest selector
  drift after patch, matplotlib/scikit-learn native extensions not importable
  in the harness image, Django unittest target drift).
- Degenerate per-agent pass vectors when all three agents share the same
  failing patch outcome on the same task rows. Mitigation: filter the panel
  with `ci/scripts/filter_panel_upstream_resolved.py` so each row is
  upstream-resolved for that agent; this restores **distinct** pass-count
  vectors on the sealed summary subset but does **not** by itself fix harness
  rate.

## Live comparative (`check_evidence_quality live`)

- Export path: `paper/exports/live_panel_v1/`
- Fails: all agents tie on `live_pass_rate` at L0 and L1 for the shipped slice,
  and `rank_stability.json` has Kendall tau-b `0.0` for the only observed level
  pair (`L0` vs `L1`), which the gate treats as non-informative.
- The gate script skips rows with null `live_pass_rate` for tie detection and
  requires at least one row with a computable `delta` (no `TypeError` on
  verified-only paper exports).

**Remediation:** regenerate `static_vs_live` / `rank_stability` from a new
comparative batch where agents diverge on live-evaluated tasks (panel design +
rerun), then re-run the gate on the new export directory. Prefer a batch that
also reaches `L3` on the same candidates so rank-stability has additional level
pairs beyond the symmetric `L0`/`L1` leaderboard.

## L2 expansion (`check_evidence_quality l2`)

- Canonical path in the plan: `runs/released/l2_verified_v2/results`
- Current batch is too small (L1 pass-through and L2 failure counts below
  defaults).

**Remediation:** expand the L2 panel and rerun until
`--min-l1-passed-from 10 --min-l2-failures 3 --min-l2-reason-families 2` pass.

## Rust proof-subset strict (`check_evidence_quality rust-proof`)

- Structural CI path remains `runs/released/rust_proof_subset_v1/results_fast`
  with semantic minima set to zero in tier-1.
- Strict semantic gate (`--min-l3-pass-l4-fail 2 --min-all-level-pass 1`) is
  not met on `results_fast` (no L3-pass/L4-fail contrast in that sealed slice).

**Remediation:** complete an L0-L4 ladder batch on the eight proof-subset
tasks with a clean `--out`, then rerun the strict gate on that directory.

## Engineering gates (local)

On the development machine used for closure work:

- `cargo deny check` exited 0 (warnings only: duplicate lockfile entries,
  unused license allow-list entries).
- `cargo audit` exited 0.

## CI after SemVer tag

Pushing a `v*.*.*` tag triggers `.github/workflows/release-tag.yml`. Tier-3
heavy work is scheduled / dispatch-only via `.github/workflows/ci-tier3-heavy.yml`;
confirm runs on GitHub Actions after tagging.
