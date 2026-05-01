# Gold validation closure (export verification)

**Date:** 2026-05-03  

Repository HEAD at engineering closure is recorded in `paper/exports/release/MANUSCRIPT_READY_SIGNOFF.md`
(same release bundle).

## Context

Outputs under `runs/released/l2_verified_flagship_v1/gold_patch_results/results_astropy/` and
`.../results_regressionfail/` are **gitignored** (local reruns only). A fresh clone therefore
cannot run `l2_flagship_gold_patch_validation.py --skip-evaluate` unless those sealed bundle
trees have been materialised by a prior full `evaluate batch` on this machine.

## Closure chosen for this freeze

**Frozen paper exports** (committed) are the manuscript Table 3 / gold-summary source of truth:

- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.json`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation_summary.json`

### Verification performed (2026-05-03)

1. **Summary JSON** reports `gold_mechanical` profile with:
   - `L2_AUG_TESTS_FAIL`: `n_gold_tested=11`, `n_gold_pass_L2=11`, eligible L0+L1 subset `n_eligible=4`, all pass L2.
   - `L2_REGRESSION_FAIL`: same headline counts; regression arm note reflects mechanical smoke semantics.
2. **`l2_flagship_gold_patch_validation.py --skip-evaluate`** was executed on the closure checkout; it
   **refused** to overwrite exports because local bundle trees were incomplete (expected row count
   not met). This confirms the script still **rejects partial row counts**—no silent weakening.
3. **Regeneration recipe** when full sealed arms exist locally:

   ```bash
   cargo build -p eval-ladder-cli --release
   python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2
   ```

   Export-only refresh (no Docker batch) when both arms are already complete:

   ```bash
   python ci/scripts/l2_flagship_gold_patch_validation.py --skip-evaluate
   ```

## Sign-off

Gold validation is **closed for manuscript purposes** using the committed frozen exports above,
with tooling enforcement verified and full regeneration path documented for machines with OCI +
complete `gold_patch_results` trees.
