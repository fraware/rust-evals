# L2 reference patch validation (primary evaluation cohort v1)

This protocol defines how upstream developer/reference patches are replayed for L2
validator legitimacy checks without conflating known protocol artifacts with
patch quality.

## Scope and objective

- **Input gold source:** `datasets/cache/verified/swe_bench_verified.jsonl` (`patch` field).
- **Task set:** the same 11 task IDs present in
  `runs/released/l2_verified_flagship_v1/results/batch_summary.json`.
- **Evaluator stack held fixed:** `configs/evaluator/default.toml` and
  `--strengthening-mode tests_plus_regression`.
- **Objective:** test whether reference patches can pass an L2 run under a coherent,
  reproducible strengthening profile; this is a validator legitimacy check, not
  a replacement for candidate publication-threshold-arm outcomes.

## Artifacts

- Exports: `paper/exports/l2_verified_flagship_v1/gold_patch_validation.{csv,json}`
- Summary: `paper/exports/l2_verified_flagship_v1/gold_patch_validation_summary.json`
- Frozen reproducibility reruns: `runs/released/l2_verified_flagship_v1/gold_patch_results/`
  - `results_astropy/` — batch mapped to **augmented_unit_tests** in exports
  - `results_regressionfail/` — batch mapped to **targeted_regression** in exports

Regenerate:

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2
```

Diagnostic replay (same strengthening JSON files as agent arms; **not** headline):

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --strict-flagship-specs --jobs 2
```

## Why two strengthening profiles?

### Publication-threshold agent-matched profile (diagnostic)

Frozen agent L2 runs use two distinct strengthening specs:

- **Aug arm:** `runs/released/l2_verified_astropy_v1/strengthening_spec.json`
  (Astropy-specific pytest selector).
- **Reg arm:** `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`
  (contains `regression_forced_fail`, deterministic non-zero exit).

Gold replay under these exact specs is useful for parity diagnostics, but it is
not a fair validator-validity headline because failure can be induced by design
(`regression_forced_fail`) or cross-repo selector non-applicability.

### Headline gold-validity profile (default)

Default gold replay uses the pre-declared
`runs/released/l2_verified_flagship_v1/strengthening_spec_gold_mechanical.json`
for both replay arms. This keeps the evaluator harness and levels unchanged
while removing publication-threshold-arm artifacts that are orthogonal to reference patch quality.

## Headline profile (default): `gold_mechanical`

Default command:

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --jobs 2
```

The script runs both export families (`augmented_unit_tests`,
`targeted_regression`) using the same gold-mechanical strengthening file and
emits:

- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation.json`
- `paper/exports/l2_verified_flagship_v1/gold_patch_validation_summary.json`

### Eligibility and acceptance definition

For release acceptance, define:

- `eligible := { rows with gold_patch_status_L0 == pass and gold_patch_status_L1 == pass }`
- `gold_pass_rate_eligible := eligible rows with gold_patch_status_L2 == pass / |eligible|`

The tranche acceptance condition is:

- `gold_pass_rate_eligible >= 0.90`, or
- explicit documented validator limitation with non-silent handling.

Current frozen export summary reports:

- `eligible_L0_L1_pass.n_eligible = 4` per family,
- `eligible_L0_L1_pass.gold_pass_rate = 1.0` per family,
- therefore the >=90% criterion is satisfied on the pre-declared eligible denominator.

Rows failing L0/L1 remain in exports with notes and are not silently removed.

## Diagnostic profile: `--strict-flagship-specs`

Diagnostic command:

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --strict-flagship-specs --jobs 2
```

This reuses the exact publication-threshold-arm strengthening specs used in candidate runs.
Interpret low gold L2 pass rates under this mode as expected protocol behavior
(especially for `regression_forced_fail`), not as direct evidence that gold
patches are semantically bad.

## Non-overclaim guardrails

- Gold headline validation and strict candidate results answer different
  questions and are both preserved.
- No rows are silently dropped; exclusions are denominator-defined and explicit.
- The diagnostic mode remains available and reproducible.
- Candidate headline findings (`L1-pass -> L2-fail` in primary-cohort arms)
  are unchanged by the gold-mechanical profile.
