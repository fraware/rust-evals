# Evidence manual

Single entry point for frozen **selection protocols**, **L2 evidence** (gold validation and adjudication case studies), the **Rust proof paper-semantics** replay recipe, and day-to-day **operations** (`eval-ladder` CLI, batches, verification, CI). For other technical writing, see [`readme.md`](readme.md).

## Table of contents

1. [Selection protocols](#selection-protocols)
2. [L2 reference patch validation](#l2-reference-patch-validation-primary-evaluation-cohort-v1)
3. [L2 failure case studies](#l2-failure-case-studies-l2-flagship-primary-cohort-v1)
4. [Rust proof subset: paper-semantics L4 replay](#rust-proof-subset-paper-semantics-l4-counterexample-replay)
5. [Operational runbook](#operational-runbook)

---

## Selection protocols

### Verified feasibility (offline inventory bound)

Output: `paper/exports/strict_feasibility_report.json` (regenerate: `python ci/scripts/analyze_strict_feasibility.py`).

### Population and inventory

- **Population:** All `batch_summary.json` files discovered under `runs/released/` (see `analyze_strict_feasibility.py::_iter_batch_summaries`) that enumerate L1-pass observations for agents in `{gru, honeycomb, sweagent}`.

- **Eligible rows:** L1-pass entries with identifiable `(task_id, agent_id)` pairs from frozen batch summaries.

- **Excluded / unavailable:** Summaries that fail JSON load are skipped silently (inventory script continues). Harness-error-heavy tasks are not removed post hoc for the bound; the report surfaces counts and thresholds explicitly.

- **Selection timing:** The offline bound is computed from **already-sealed** run artifacts; threshold comparisons are declared in code (`min_candidates`, `max_harness_error_rate` parameters) and are not tuned per headline outcome in this repository version.

- **Freeze:** Report inputs are whatever sealed directories exist under `runs/released/` at analysis time; reproducibility is anchored by re-running the script on the same tree and comparing `strict_feasibility_report.json`.

### Missing, invalid, N/A

- Missing batch summaries reduce coverage counts but do not silently upgrade claims.
- Invalid L1 rows are excluded from pass observations by the analyzer’s L1 status filter.

### Allowed vs disallowed claims

- **Allowed:** The current public candidate inventory bounds strict three-agent Verified comparison below the predeclared threshold (see report `/verified` metrics).

- **Not allowed:** Claiming Verified comparison fails because the evaluator cannot run tasks, or treating the bound as a central prevalence estimate for all agents.

### Downstream tables

- `paper/tables/verified_feasibility_bound.tex` and evaluator card `paper/exports/evaluator_cards/verified_feasibility_frontier.md` consume this report.


### Rust proof subset (L3/L4 seal)


Sealed run: `runs/released/rust_proof_subset_v1/results_seal/`. Obligation inventory: `datasets/derived/proof_subset/manifest.jsonl`. Paper export: `paper/exports/rust_proof_subset_v1_seal_release/`.

### Population

- **Population:** Curated Rust SWE-bench tasks listed in `manifest.jsonl` used for proof obligations and L3/L4 gating experiments.

- **Eligible tasks:** Tasks successfully materialized by `packages/python/scripts/build_rust_proof_subset_panel.py` into `runs/released/rust_proof_subset_v1/panel.jsonl` with golden-agent candidate rows.

- **Excluded:** Tasks failing materialization (network, patch fetch, clone) are not silently replaced; the builder records failures and the panel reflects what was frozen.

- **Selection timing:** Task list and obligations are fixed by the checked-in manifest; headline paper exports use the **seal** profile (`results_seal/`) rather than fast/paper-semantics replay variants.

### Missing / invalid handling

- Missing obligation entries make bundles incomplete and fail verification rather than defaulting to pass.
- `not_applicable` levels remain explicit per ladder semantics.

### Allowed vs disallowed claims

- **Allowed:** The proof subset demonstrates sealed L3/L4 obligations, verification, and gate profiles on a curated Rust slice.

- **Not allowed:** Claiming L4 proves universal semantic correctness for all Rust agents, or generalizing proof-subset counts beyond the curated obligations.

### Exports

- `eval-ladder analyze paper-export` siblings under `paper/exports/rust_proof_subset_v1_seal_release/` including canonical `conditional_reversal.{csv,json}` plus deprecated byte-identical `conditional_false_success.{csv,json}` aliases.


### Live v2 diagnostic panel


Paper exports: `paper/exports/live_panel_v2_postbatch/`. Sealed bundles: `runs/released/live_panel_v2/results_opt/`.

### Protocol fields

- **static_anchor_selection_rule:** Static-anchor (SWE-Bench Verified) tasks and gold `patch` assignment mirror `live_panel_v1` so every agent shares the same official harness inputs on the Verified arm (`packages/python/scripts/build_live_panel_v2.py`).

- **live_row_selection_rule:** Live tasks are the fixed eight-task list in `build_live_panel_v2.LIVE_TASKS`; each `(task, agent)` row receives a deterministic live patch strategy (gru: gold `patch` everywhere; honeycomb / sweagent: split halves between `patch` and `test_patch` for asymmetry without changing benchmark manifests).

- **candidate_availability_rule:** Candidates are UUIDv5-derived under the v2 namespace with frozen JSON in `runs/released/live_panel_v2/candidates/`. Rows require resolvable workspace materialization for batch execution.

- **missing_candidate_policy:** **Missing candidate material or unavailable live image is not treated as an invalid evaluation verdict by this protocol document**; execution surfaces `invalid` / harness errors in summaries and transparency exports (`live_rows_L0_or_L1_invalid.csv`). **Missing candidate is distinct from benchmark-invalid:** missing/unavailable rows are operational gaps, not redefinitions of benchmark semantics.

- **invalid_row_policy:** Official L0/L1 `invalid` verdicts are enumerated in `live_rows_L0_or_L1_invalid.csv` and excluded from headline pass-rate denominators per `export_live_panel_tables.py` rules.

- **denominator_policy:** Wilson intervals and static/live numerators in `live_panel_summary_with_ci.csv` use row counts after invalid filtering; leave-one-out uses the same frozen panel.

- **freeze_commit_or_hash:** Frozen panel root `runs/released/live_panel_v2/` with provenance `provenance.json` and `SUBMITTED_AT` stamp in builder (`2025-05-01Z` in `build_live_panel_v2.py`).

### Allowed vs disallowed claims

- **Allowed:** In this small denominator-aware diagnostic panel, static-anchor pass rates can overstate observed live outcomes.

- **Not allowed:** Estimating population live robustness, ranking agents for production deployment from this panel alone, or claiming coverage of all SWE-bench-style tasks.

### Generated exports

- `live_panel_summary_with_ci.csv`, `live_leave_one_out.csv`, `per_task_live_outcomes.csv`, `live_integrity_summary.json`, standard `analyze paper-export` siblings under `paper/exports/live_panel_v2_postbatch/`.


### L2 verified flagship v1


Machine-readable selection index: `paper/exports/l2_verified_flagship_v1/l2_selection_manifest.{csv,json}`.

#### Protocol fields

- **base_row_selection_rule:** Start from the 33 frozen `(task, agent)` rows in `runs/released/agent_panel_verified_flagship_v1/panel.jsonl` (Verified primary evaluation cohort), derived from `agent_panel_v3_r1` with fixed fragile-tooling prefix exclusions (`matplotlib__`, `scikit-learn__`, `pytest-dev__`). Exactly eleven Verified-style tasks, three agents, one winner-style candidate JSON each. **The 33 base rows are not used to estimate population-level bug prevalence; they are a validator-focused diagnostic slice.**

- **validator_arm_definitions:**
  - `augmented_tests`: bundle suffix `__astropy`, results under `results_astropy/`, strengthened augmented pytest-style validation (issue-relevance varies by task).
  - `regression_stress_control`: bundle suffix `__regressionfail`, results under `results_regression_fail/`, includes `regression_forced_fail` as a **negative-control protocol surface** (not natural product-regression evidence).

- **arm_expansion_rule:** Each base row is duplicated deterministically across both arms, producing **33 + 33 = 66** sealed entries merged into `runs/released/l2_verified_flagship_v1/results/batch_summary.json` via `ci/scripts/merge_l2_batch_summaries.py` without dropping rows.

- **gold_patch_validation_rule:** Oracle gold-patch checks use `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv` and `gold_patch_validation_summary.json` under the frozen gold-mechanical profile for headline paper tables; interpret forced-fail regression rows per [L2 reference patch validation](#l2-reference-patch-validation-primary-evaluation-cohort-v1).

- **human_review_sampling_rule:** Eight sealed L1-pass/L2-fail rows are curated in `ci/scripts/export_l2_flagship_reviews.py` (`CURATED_ENTRY_IDS`) for single-reviewer adjudication labels exported to `l2_failure_review.csv` and summarized in `l2_human_review_summary.csv`. Diagnostic only; not a prevalence sample.

- **pooled_total_interpretation:** The merged **66-row** cohort totals (e.g. 24 L1-pass/L2-fail pairs across both arms) quantify **evaluator sensitivity** on this frozen slice. They **must not** be read as SWE-bench population bug prevalence.

- **freeze_commit_or_hash:** Sealed cohort directory `runs/released/l2_verified_flagship_v1/` is the frozen evidence root; regenerate paper exports with `python packages/python/scripts/reproduce_paper_tables.py` after `cargo build --release`.

- **Selection timing:** Arm expansion and merge rules were fixed **before** interpreting headline L2 counts as diagnostic evidence (no post-hoc row drops based on pass/fail).



#### L2 selection protocol (primary evaluation cohort v1)

> **Important interpretation note:** The 66-row L2 primary cohort is
> **validator-focused** and **diagnostic**. It is **not** a random sample and
> must **not** be used to estimate the population rate of semantic failure in
> SWE-bench-style tasks. The regression arm includes `regression_forced_fail`;
> those rows are **protocol-control evidence**, not natural product-regression
> evidence.

This document is the **auditable protocol** for the 66-row L2 primary-cohort slice
`runs/released/l2_verified_flagship_v1/`. The slice is **validator-focused and
diagnostic**: it is not a random draw from a well-defined superpopulation, and it
should not support population-level generalizations beyond the stated construction.
A per-row machine-generated index is
`paper/exports/l2_verified_flagship_v1/l2_selection_manifest.{csv,json}`.

##### 1. Source and scope

- **Source benchmark (candidates).** The L2 layer reuses the same **frozen
  agent candidate files** as the **Verified primary evaluation cohort** panel
  `runs/released/agent_panel_verified_flagship_v1/`, which in turn was derived
  from `agent_panel_v3_r1` under fixed prefix exclusions (Fragile tooling
  prefixes such as `matplotlib__`, `scikit-learn__`, `pytest-dev__` were dropped
  when assembling the primary-cohort panel).

- **Task IDs included.** Exactly the **eleven SWE-Bench Verified–style tasks**
  present in `agent_panel_verified_flagship_v1/panel.jsonl` (each task appears
  once per agent in the base 33 rows).

- **Agents / candidate sources.** Three public agent sources are included in this
  panel (IDs remain in the manifest and summaries) — one winner-style candidate
  JSON per (task, agent) drawn from the candidate store.

- **Candidate patches per (task, agent).** **One** frozen candidate per pair in
  the base panel; L2 does not sweep multiple candidates per pair in primary evaluation cohort v1.

##### 2. Selection rule (what produced the 66 entries)

The L2 primary-cohort slice was built by taking the **33** base `(task, agent)`
rows and applying a **deterministic, pre-declared pair of validator arms**:

1. Each base row is evaluated under the **augmented-test** arm (bundle suffix
   `__astropy`, results under `results_astropy/`).
2. The **same** base row is evaluated under the **regression** arm (bundle
   suffix `__regressionfail`, results under `results_regression_fail/`).

Thus **33 + 33 = 66** frozen rows appear in
`runs/released/l2_verified_flagship_v1/results/batch_summary.json`. The two
single-family batch summaries were merged with
`ci/scripts/merge_l2_batch_summaries.py` **without dropping rows**.

Selection does **not** depend on whether a candidate passed or failed L2.

##### 3. Exclusion rule (upstream and operational)

No additional candidate rows were excluded **after** fixing the primary-cohort base
panel for L2 purposes.

**Upstream exclusions (primary-cohort assembly only):**

| Category | Applied in primary evaluation cohort v1? |
|----------|-------------------------|
| Known fragile task prefixes / tooling (`matplotlib__`, `scikit-learn__`, `pytest-dev__`) | Yes — tasks removed when trimming `agent_panel_v3_r1` into `agent_panel_verified_flagship_v1`. |
| Missing workspace / bundle materialization failure | Handled at batch execution time (row may be `invalid` in summaries); not an L2-specific exclusion rule. |
| Patch does not apply | Surfaced as harness or strengthener failure per row; not used to shrink the 66-row design. |
| Official scorer unavailable | Same — execution-level, not used to redefine the 66-row set post hoc. |
| Gold patch unavailable | Does not exclude a candidate row from L2; it affects **reference-patch validation** evidence only. |
| Validator not applicable | Not used to delete rows in primary evaluation cohort v1; both arms are run for every base row. |
| Known flaky task | Only via the upstream prefix policy above. |

##### 4. Validator construction

###### L2_AUG_TESTS_FAIL (`augmented_unit_tests`)

- **How validators were chosen.** Checked-in strengthening specs under
  `runs/released/l2_verified_astropy_v1/strengthening_spec.json` (referenced by
  the primary-cohort batch README) define augmented pytest-style commands including
  warnings-as-errors stress paths.
- **Written before seeing candidate failures?** The specs are **versioned
  artifacts** committed independently of any particular candidate outcome.
- **Checked against reference patches?** Yes — see
  `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv` and
  [L2 reference patch validation](#l2-reference-patch-validation-primary-evaluation-cohort-v1).
- **Task-specific vs generic.** Commands can target repo-specific selectors; the
  **pattern** (official rerun + augmented selectors) is shared.
- **Generated tests failing on gold?** Gold outcomes are reported explicitly;
  failures trigger manual review per the gold validation protocol (validator
  bug, exclusion, or documented limitation).

###### L2_REGRESSION_FAIL (`targeted_regression`)

- **How validators were chosen.**
  `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`
  defines the regression validator, including **`regression_forced_fail`**.
- **Before candidate outcomes?** Same as above — checked-in spec.
- **Gold-patch check?** Same CSV as above — gold rows must be interpreted with
  the forced-fail caveat (see Integrity note below).
- **Task-specific vs generic.** The forced-fail hook is **generic protocol** on
  primary evaluation cohort v1.
- **Tests failing on reference patches removed post hoc?** No dynamic removal — outcomes are
  reported.

##### 5. Post-hoc handling

- **Rows removed after L2 execution?** **No.** The merged `results/` directory
  retains both arms.

- **Validators modified after observing candidate failures?** **Not as part of
  primary evaluation cohort v1 frozen reruns.** Any future change would require a new protocol
  version and new frozen directories.

- **Task families merged?** The **66-row** summary is a **merge of two
  validator arms** (`results_astropy` + `results_regression_fail`) for the same
  33 base rows — not a merge of disjoint task universes.

- **Summaries deduplicated?** Entry IDs are unique per `(agent, task, arm)`; no
  deduplication beyond the merge script’s deterministic join.

##### Integrity note (regression arm)

In primary evaluation cohort v1, `targeted_regression` includes **`regression_forced_fail`**
(non-zero exit by design). Interpret **L2_REGRESSION_FAIL** on this arm as a
**controlled protocol signal**, not standalone proof of product regression on
the ticket.

##### Relationship to reference-patch validation

Reference-patch validation is intentionally documented as a separate protocol in
[L2 reference patch validation](#l2-reference-patch-validation-primary-evaluation-cohort-v1) and does **not** change candidate row
selection for this 66-row slice.

- Candidate publication-threshold-arm evidence remains tied to
  `results_astropy/` + `results_regression_fail/`.
- Gold headline legitimacy checks use the pre-declared
  `strengthening_spec_gold_mechanical.json` profile to avoid conflating strict
  negative-control artifacts with validator validity.
- Publication-threshold replay with agent specs is still available via
  `--strict-flagship-specs` for parity diagnostics.


---


## L2 reference patch validation (primary evaluation cohort v1)

This protocol defines how upstream developer/reference patches are replayed for
L2 validator legitimacy checks without conflating known protocol artifacts with
patch quality.

### Scope and objective

- **Input gold source:** `datasets/cache/verified/swe_bench_verified.jsonl` (`patch` field).
- **Task set:** the same 11 task IDs present in
  `runs/released/l2_verified_flagship_v1/results/batch_summary.json`.
- **Evaluator stack held fixed:** `configs/evaluator/default.toml` and
  `--strengthening-mode tests_plus_regression`.
- **Objective:** test whether reference patches can pass an L2 run under a coherent,
  reproducible strengthening profile; this is a validator legitimacy check, not
  a replacement for candidate publication-threshold-arm outcomes.

### Artifacts

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

### Why two strengthening profiles?

#### Publication-threshold agent-matched profile (diagnostic)

Frozen agent L2 runs use two distinct strengthening specs:

- **Aug arm:** `runs/released/l2_verified_astropy_v1/strengthening_spec.json`
  (Astropy-specific pytest selector).
- **Reg arm:** `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`
  (contains `regression_forced_fail`, deterministic non-zero exit).

Gold replay under these exact specs is useful for parity diagnostics, but it is
not a fair validator-validity headline because failure can be induced by design
(`regression_forced_fail`) or cross-repo selector non-applicability.

#### Headline gold-validity profile (default)

Default gold replay uses the pre-declared
`runs/released/l2_verified_flagship_v1/strengthening_spec_gold_mechanical.json`
for both replay arms. This keeps the evaluator harness and levels unchanged
while removing publication-threshold-arm artifacts that are orthogonal to reference patch quality.

### Headline profile (default): `gold_mechanical`

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

#### Eligibility and acceptance definition

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

### Diagnostic profile: `--strict-flagship-specs`

Diagnostic command:

```bash
python ci/scripts/l2_flagship_gold_patch_validation.py --strict-flagship-specs --jobs 2
```

This reuses the exact publication-threshold-arm strengthening specs used in candidate runs.
Interpret low gold L2 pass rates under this mode as expected protocol behavior
(especially for `regression_forced_fail`), not as direct evidence that gold
patches are semantically bad.

### Paper wording (validator legitimacy only)

Gold-patch validation shows that the headline L2 harness can accept reference
patches under the predeclared **gold-mechanical** profile. This validates the
harness at the **eligible denominator** (gold passes L0 and L1), but does **not**
imply that every candidate L2 failure is a semantic defect.

### Non-overclaim guardrails

- Gold headline validation and strict candidate results answer different
  questions and are both preserved.
- No rows are silently dropped; exclusions are denominator-defined and explicit.
- The diagnostic mode remains available and reproducible.
- Candidate headline findings (`L1-pass -> L2-fail` in primary-cohort arms)
  are unchanged by the gold-mechanical profile.


---


## L2 failure case studies (L2 flagship primary cohort v1)


<!-- BEGIN_EXPORT_L2_CASE_STUDIES -->

Human adjudication sample from frozen run results at `runs/released/l2_verified_flagship_v1/results/batch_summary.json` with reference-patch context from `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv` when available.

The **regression stress-control arm** is a **negative-control / protocol** arm. Its reversals demonstrate **evaluator-induced score changes**, not natural product regressions. Rows that fail via `regression_forced_fail` are **protocol-control evidence**, not evidence that the upstream issue regressed in production.

### Human review summary (diagnostic sample)

The review sample is **diagnostic** and **single-reviewer**; it is **not** used to estimate population-level semantic-defect rates.

| Review label | Augmented tests | Regression stress-control | Total |
|--------------|-----------------|---------------------------|-------|
| Issue-relevant candidate weakness | 2 | 0 | 2 |
| Valid stress-control reversal | 0 | 2 | 2 |
| Unclear or infrastructure artifact | 2 | 2 | 4 |

### Sample composition

- Total reviewed: `8`
- Augmented-test failures (`L2_AUG_TESTS_FAIL`): `4`
- Regression stress-control failures (`L2_REGRESSION_FAIL`): `4`
- Issue-relevant candidate weakness: `2` augmented cases
- Valid stress-control reversal: `2` regression-control cases (validator behaved according to its declared Evaluator Card; `regression_forced_fail` as designed)
- Unclear or infrastructure artifact: `4` cases

Do **not** describe forced-fail regression rows as confirmations of natural product regression on the ticket. Use **protocol_control_reversal** / **stress_control_reversal** when referring to score reversals on that arm, or **valid stress-control reversal** when the outcome matches the predeclared control specification.

### Case 1: astropy__astropy-7671 / gru

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

Official SWE-bench issue: minversion comparison failures under LooseVersion edge cases.

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> Traceback (most recent call last):   File "/opt/miniconda3/envs/testbed/lib/python3.6/runpy.py", line 193, in _run_module_as_main     "__main__", mod_spec)   File "/opt/minicond...

#### Why this is issue-relevant

Issue relevance assessment: `directly_issue_relevant`.

#### Human adjudication

`issue_relevant_candidate_weakness` (confidence `high`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/gru__astropy__astropy-7671__astropy`

### Case 2: django__django-7530 / gru

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

Django ticket fix evaluated on verified harness (official tests).

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> /opt/miniconda3/envs/testbed/bin/python: No module named pytest

#### Why this is issue-relevant

Issue relevance assessment: `weakly_relevant`.

#### Human adjudication

`unclear_or_infrastructure_artifact` (confidence `medium`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/gru__django__django-7530__astropy`

### Case 3: pylint-dev__pylint-7277 / honeycomb

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

Pylint change-set from verified flagship slice.

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> Traceback (most recent call last):   File "/opt/miniconda3/envs/testbed/lib/python3.9/site-packages/pluggy/_callers.py", line 156, in _multicall     teardown[0].send(outcome)   ...

#### Why this is issue-relevant

Issue relevance assessment: `weakly_relevant`.

#### Human adjudication

`unclear_or_infrastructure_artifact` (confidence `medium`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/honeycomb__pylint-dev__pylint-7277__astropy`

### Case 4: sphinx-doc__sphinx-9698 / sweagent

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

Sphinx documentation/build issue from verified flagship slice.

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> ERROR: file or directory not found: astropy/modeling/tests/test_separable.py::test_separable

#### Why this is issue-relevant

Issue relevance assessment: `weakly_relevant`.

#### Human adjudication

`issue_relevant_candidate_weakness` (confidence `medium`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/sweagent__sphinx-doc__sphinx-9698__astropy`

### Case 5: django__django-7530 / gru

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

Django ticket fix evaluated on verified harness (official tests).

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

targeted_regression:regression_forced_fail exit_code=1

#### Why this is issue-relevant

Issue relevance assessment: `regression_relevant`.

#### Human adjudication

`valid_stress_control_reversal` (confidence `medium`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/gru__django__django-7530__regressionfail`

### Case 6: pallets__flask-5014 / gru

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** fail  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

Flask issue from verified flagship slice.

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

targeted_regression:regression_forced_fail exit_code=1

#### Why this is issue-relevant

Issue relevance assessment: `regression_relevant`.

#### Human adjudication

`valid_stress_control_reversal` (confidence `medium`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/gru__pallets__flask-5014__regressionfail`

### Case 7: pydata__xarray-4075 / honeycomb

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** fail  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

xarray issue from verified flagship slice.

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

targeted_regression:regression_forced_fail exit_code=1

#### Why this is issue-relevant

Issue relevance assessment: `not_relevant`.

#### Human adjudication

`unclear_or_infrastructure_artifact` (confidence `high`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/honeycomb__pydata__xarray-4075__regressionfail`

### Case 8: pylint-dev__pylint-6903 / sweagent

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** fail  
**L2 verdict:** fail  
**Gold patch status:** pass  

#### Issue context

Pylint issue from verified flagship slice.

#### Candidate behavior

Candidate patch is the sealed agent submission for this task (see `artifact_bundle`).

#### L2 failure

targeted_regression:regression_forced_fail exit_code=1

#### Why this is issue-relevant

Issue relevance assessment: `not_relevant`.

#### Human adjudication

`unclear_or_infrastructure_artifact` (confidence `high`).

#### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/sweagent__pylint-dev__pylint-6903__regressionfail`

### Protocol note (regression arm)

Regression-family rows use `regression_forced_fail` in `strengthening_spec_regression_fail.json`. Interpret them through `docs/scientific_scope.md` and the regression Evaluator Card (protocol-control / stress-control evidence).

<!-- END_EXPORT_L2_CASE_STUDIES -->



---


## Rust proof subset: paper-semantics L4 counterexample replay

The frozen ladder under `runs/released/rust_proof_subset_v1/results_seal/` uses
the production obligation manifest
`datasets/derived/proof_subset/manifest.jsonl`, where every row is wired to a
real `EvalLadder/Obligations/**` module.

For **paper semantics** (strict `check_evidence_quality rust-proof` minima on
L3-pass / L4-fail exemplars and an all-level pass), replay the same eight-task
panel with the companion manifest
`datasets/derived/proof_subset/manifest_paper_semantics_l4_counterexample.jsonl`.
That manifest is identical except for `clap-rs__clap_1624` and
`clap-rs__clap_1710`, which point at
`EvalLadder/Obligations/PaperSemantics/Broken.lean` so `lake env lean` fails
and L4 surfaces as `L4_OBLIGATION_UNMET` while L3 policy can still pass.

Recipe:

```bash
just rust-proof-batch-seal-paper-semantics runs/released/rust_proof_subset_v1/results_seal_paper_semantics
python ci/scripts/check_evidence_quality.py rust-proof \
  --run-dir runs/released/rust_proof_subset_v1/results_seal_paper_semantics
```

The broken Lean module is not part of the default proof corpus; keep paper
replays on a separate `--out` directory so it does not overwrite the audit
closure bundle.


---


## Operational runbook

This runbook covers day-to-day operation of `eval-ladder`: local development,
batch evaluation, CI tiers, and release hygiene. For a map of all technical
docs (scope, ladder, architecture, evidence gates), see [`readme.md`](readme.md).

### Prerequisites

- Rust toolchain pinned by `rust-toolchain.toml` (1.86 at time of writing).
- Python 3.10+.
- An OCI-compatible container runtime (Docker Engine, Podman, or similar).
- Optional: Lean 4 via `leanprover/lean4:v4.15.0` for L4.
- Optional: `just` for the task recipes in `justfile`.

### Local development

Bootstrap:

```bash
rustup show                                  # confirms pinned toolchain
cargo build --workspace --all-targets
python -m pip install -e ".[dev]"
```

Common flows:

```bash
just ci-tier1            # fmt-check, clippy, test, schema validation
just ci-tier2            # tier1 + Python lint and tests
cargo run --bin eval-ladder -- --help
cargo run --bin eval-ladder -- schema validate
```

### Per-candidate evaluation (Milestone C)

Milestone C ships the full L0 (official) + L1 (trusted rerun) pipeline
as `eval-ladder evaluate candidate`. Each invocation produces:

- a hash-chained `trace.jsonl`,
- a sealed evidence bundle at `--bundle-dir`,
- two `EvaluationResult` JSON documents inside the bundle
  (`official_results.json` for L0, `l1_trusted_rerun_results.json`
  for L1),
- a single-line JSON status report on stdout summarising
  `bundle_hash`, `run_id`, `bundle_id`, and per-level status.

#### Inputs

| Flag | Meaning |
| --- | --- |
| `--task` | Normalized benchmark task manifest (output of `ingest`). |
| `--candidate` | `CandidateResolution` JSON. |
| `--patch` | Candidate patch. Empty file is a valid no-op. |
| `--workspace-template` | Unpatched checkout at `base_commit`. Never mutated. |
| `--bundle-dir` | Destination for the evidence bundle. Must be empty or absent. |
| `--config` | Evaluator configuration TOML. |
| `--deterministic-clock` | Use a fixed clock so reruns yield identical bundle hashes. |
| `--seed-tag` | Identity-seed label. Change to separate otherwise-identical reruns. |
| `--levels` | Comma-separated ladder levels to run. Defaults to `L0,L1`; add `L2` to enable strengthening. |
| `--strengthening-spec` | Path to the task-level L2 `StrengtheningSpec` JSON. Required when `--levels` includes `L2`. |
| `--strengthening-mode` | `tests_only`, `tests_plus_diff`, `tests_plus_regression`, or `full_l2`. Defaults to `full_l2`. |
| `--oracle-patch` | Oracle patch bytes for the differential validator. Required for `tests_plus_diff` or `full_l2` runs that exercise `differential_behavior`. |
| `--policy` | Path to an L3 policy TOML (see `configs/policy/default_policy.toml`). Required when `--levels` includes `L3`. |
| `--network-accessed` | Inform L3 that the container engine observed outbound network activity. Defaults to `false`; keep it off for `LocalProcessEngine`. |
| `--obligations` | Path to an L4 obligation manifest (JSONL; one `ProofObligation` per line). Required when `--levels` includes `L4`. |
| `--lean-root` | Path to the Lean project root (typically `packages/lean/EvalLadder`). Passed as the checker's `cwd`. Required when `--levels` includes `L4`. |

#### Determinism contract

Running `eval-ladder evaluate candidate` twice with:

1. the same `--task`, `--candidate`, and `--patch` bytes,
2. the same `--workspace-template` contents,
3. `--deterministic-clock` on both runs,
4. the same `--seed-tag`,

produces byte-identical `trace.jsonl` and identical `bundle_hash`. This
is the Milestone C acceptance invariant and is pinned by the
`packages/rust/runner/tests/pipeline_acceptance.rs` integration test
plus the `pipeline::tests::bundle_hash_is_stable_across_reruns` unit
test.

Production runs usually omit `--deterministic-clock`; timestamps then
follow wall-clock time. The evidence-bundle hash still covers the
`created_at` field so diverging timestamps still diverge the hash; the
flag exists specifically for rerun-determinism audits.

### L2 strengthening (Milestone D)

When `--levels` includes `L2`, the pipeline runs the
`eval-ladder-strengthening` extension after L1 and emits two extra
bundle artifacts:

- `strengthened_results.json` - `EvaluationResult` for L2 (aggregate
  pass/fail and primary failure reason).
- `strengthening_report.json` - per-validator breakdown with every
  sub-check verdict, exit code, and truncated stderr. This is the
  source the analysis layer reads when attributing L2 score drops to
  specific sub-checks.

#### Strengthening spec

The spec is a JSON document of type
`eval_ladder_strengthening::StrengtheningSpec`. A minimal spec that
enables only augmented tests looks like:

```json
{
  "schema_version": 1,
  "augmented": {
    "commands": [
      { "id": "edge_case_1", "command": ["pytest", "-q", "tests/edge_case_1.py"] }
    ]
  },
  "regression": { "commands": [] },
  "differential": null,
  "property_fuzz": null
}
```

A `full_l2` spec additionally carries a `differential` block with an
`oracle_patch_ref` and a list of observable commands. The actual
oracle-patch bytes are passed at run time via `--oracle-patch`; the
`oracle_patch_ref` is only used for provenance metadata inside the
bundle.

#### Determinism contract (L2)

Two `evaluate candidate --levels L0,L1,L2 --deterministic-clock` runs
with the same task, candidate, patch bytes, workspace template,
strengthening spec, strengthening mode, and (if applicable) oracle
patch bytes produce byte-identical `trace.jsonl` and an identical
`bundle_hash`. This extends the Milestone C invariant and is pinned
by `packages/rust/strengthening/tests/milestone_d_acceptance.rs::milestone_d_l2_reruns_are_deterministic`.

#### Failure codes

L2 aggregate `primary_reason` values:

- `L2_AUG_TESTS_FAIL` - one or more augmented-unit-tests sub-checks
  failed.
- `L2_REGRESSION_FAIL` - one or more regression sub-checks failed.
- `L2_DIFF_BEHAVIOR` - at least one observable diverged between the
  candidate-patched workspace and the oracle-patched workspace.
- `L2_ORACLE_UNAVAILABLE` - the spec declares a differential block
  but no oracle patch was supplied; differential is reported as
  `NotApplicable`. Does not, on its own, fail L2.

### L3 policy (Milestone E)

When `--levels` includes `L3`, the pipeline runs the
`eval-ladder-policy` extension after L2 (or after L1 when L2 is
absent) and emits one additional bundle artifact:

- `policy_results.json` - full [`PolicyReport`] with ordered
  [`PolicyFinding`] list, plus a `run_context_summary` block pinning
  the inputs the engine judged (commands, modified files, trace
  events seen, and the static observation flags).

The L3 `EvaluationResult` carries:

- `status = pass` and `primary_reason = "PASS"` when the finding list
  is empty.
- `status = fail` and `primary_reason = <first PV_* code>` otherwise,
  with subsequent codes in `secondary_reasons`.

Every finding is also mirrored as a `PolicyViolationDetected` trace
event on the run-level hash chain, so the trace alone is
self-describing even if the JSON artifact is lost.

#### Policy document

The policy is a declarative TOML document consumed by
`eval_ladder_policy::Policy::from_path`. A minimal permissive policy
looks like:

```toml
name = "demo_policy"
network_mode = "disabled"
requires_reproducible_seed = true
max_modified_files = 8

allowed_commands = ["cargo", "python", "pytest", "bash", "sh", "git"]
forbidden_commands = ["curl", "wget", "ssh", "sudo"]

allowed_edit_globs = ["src/**", "tests/**"]
forbidden_edit_globs = [".github/**", "secrets/**"]

required_trace_events = [
    "RunStarted",
    "PatchApplied",
    "OfficialEvalStarted",
    "OfficialEvalFinished",
    "RunFinished",
]
```

The shipped default lives at `configs/policy/default_policy.toml`.

#### Determinism contract (L3)

Two `evaluate candidate --levels L0,L1,L2,L3 --deterministic-clock`
runs with the same task, candidate, patch bytes, workspace template,
strengthening inputs, policy document, and network-observation flag
produce byte-identical `trace.jsonl` and an identical `bundle_hash`.
This is pinned by
`packages/rust/policy/tests/milestone_e_acceptance.rs::milestone_e_l3_reruns_are_deterministic`.

#### Failure codes

L3 aggregate `primary_reason` values match `PolicyViolation::as_str`:

- `PV_NET_ACCESS` - outbound network activity under `network_mode = "disabled"`
  or `"host_allowlist"`.
- `PV_FORBIDDEN_CMD` - the run invoked a command in `forbidden_commands`
  or a command outside a non-empty `allowed_commands`.
- `PV_EDIT_SCOPE` - the patch modified a path matching
  `forbidden_edit_globs` or outside a non-empty `allowed_edit_globs`.
- `PV_FILE_COUNT_EXCEEDED` - the patch modifies more files than
  `max_modified_files`.
- `PV_DEPENDENCY_EDIT` - the patch modifies a known lockfile while
  `allow_dependency_lockfile_edits = false`.
- `PV_GENERATED_TEST_DISALLOWED` - a `generated_tests/` directory is
  present in the bundle while `allow_generated_tests = false`.
- `PV_ENV_NONDETERMINISTIC` - the candidate did not declare a
  reproducible seed while `requires_reproducible_seed = true`, or the
  trusted rerun disagreed with the official run.
- `PV_TRACE_INCOMPLETE` - a required trace event (other than the
  pipeline-guaranteed `RunFinished`) did not appear.

### L4 proof subset (Milestone F)

When `--levels` includes `L4`, the pipeline runs the
`eval-ladder-lean` extension after L3 (or after the lowest available
rung when L3 was not requested) and emits one additional bundle
artifact:

- `proof_results.json` - full [`ProofReport`] with the three-valued
  `LeanStatus` (`valid` / `invalid` / `not_applicable`), the stable
  uppercase `code`, the resolved `ProofObligation` (or `null` when
  the task has no obligation in the manifest), the raw checker
  payload, and `{started_at, finished_at, duration_ms}` timings.

The L4 `EvaluationResult` carries:

- `status = pass` and `primary_reason = <obligation pass_criterion>`
  when the checker returned `LeanStatus::Valid` with the expected
  code.
- `status = fail` and `primary_reason = L4_OBLIGATION_UNMET` when the
  checker returned `LeanStatus::Valid` with an unexpected code or
  `LeanStatus::Invalid` without a more specific code. When the
  harness itself failed, `primary_reason = L4_PROOF_CHECK_FAILED` and
  the error kind (`spawn` / `parse` / `exited` / `io`) is captured
  inside the `proof_results.json` payload.
- `status = not_applicable` and
  `primary_reason = L4_OBLIGATION_NOT_APPLICABLE` when the task has
  no obligation in the manifest.

Every L4 run emits a `ProofCheckStarted` and a `ProofCheckFinished`
event on the trace's hash chain so the verdict is auditable from the
trace alone.

#### Obligation manifest

The manifest is a JSONL document; each line is one
`ProofObligation` (schema `schemas/proof_obligation.schema.json`).
Blank lines and comments whose first non-whitespace character is `#`
are skipped so reviewers can annotate entries in PRs. A minimal
manifest entry looks like:

```json
{"schema_version":1,"obligation_id":"obl.example.reflexive","task_id":"example__task-1","property_name":"identity_reflexive","property_type":"no_panic_or_invalid_state","target_files":["src/lib.rs"],"informal_statement":"equality is reflexive on Nat.","formal_statement_ref":"EvalLadder/Obligations/Example/Task1.lean","proof_checker":{"command":"lake","args":["env","lean","EvalLadder/Obligations/Example/Task1.lean"]},"pass_criterion":"L4_OBLIGATION_MET","difficulty":{"reviewer_hours":0.5},"selection_rationale":{"one_or_two_sentence_property":true,"local_scope":true,"matters_to_issue":true,"strictly_stronger_than_tests":true,"bounded_effort":true}}
```

Selection discipline is documented in `docs/proof_subset_policy.md`
and is enforced by review (not the loader). Duplicate `task_id`
entries are rejected at load time.

#### Checker contract

Production runs spawn the command declared by each obligation via
`ExternalProcessChecker` with `cwd = --lean-root`. The checker must
print a single JSON object on stdout of the shape:

```json
{ "status": "valid|invalid|not_applicable",
  "code": "L4_OBLIGATION_MET|L4_OBLIGATION_UNMET|...",
  "message": "free-form text",
  "payload": { ... } }
```

Non-zero exit codes are tolerated as long as stdout carries a valid
outcome; that lets checkers communicate `Invalid` through a
canonical exit code without losing structure. Checkers that exit
non-zero without a parseable outcome produce `L4_PROOF_CHECK_FAILED`
and the captured stderr is embedded in `proof_results.json`.

#### Determinism contract (L4)

Two `evaluate candidate --levels L0,L1,L2,L3,L4 --deterministic-clock`
runs with the same task, candidate, patch bytes, workspace template,
strengthening inputs, policy document, network-observation flag,
obligation manifest, and a deterministic checker (the in-tree
`ScriptedChecker` is the canonical audit tool) produce byte-identical
`trace.jsonl` and an identical `bundle_hash`. Pinned by
`packages/rust/lean/tests/milestone_f_acceptance.rs::l4_reruns_are_deterministic`.

Production Lean checkers inherit reproducibility from the
`packages/lean/EvalLadder/lean-toolchain` pin; audits that exercise
`lake` directly use the ignored integration test
`l4_external_checker_against_lake_binary_ok`.

#### Failure codes (L4)

- `L4_OBLIGATION_UNMET` - the checker returned `Valid` but with a
  code that disagrees with the obligation's `pass_criterion`, or
  returned `Invalid` with its own code.
- `L4_PROOF_CHECK_FAILED` - harness failure: checker process failed
  to spawn, exited non-zero without a parseable outcome, or produced
  unparseable stdout.
- `L4_OBLIGATION_NOT_APPLICABLE` - no obligation registered for the
  task (the empty-manifest case).
- `L4_EXTRACTION_FAILED` - reserved for future use by checkers that
  perform their own extraction step before invoking Lean.

#### Batch: `prove-subset`

`eval-ladder prove-subset` runs the L4 checker over an existing
directory of evidence bundles:

```bash
cargo run --bin eval-ladder -- prove-subset \
  --subset      datasets/derived/proof_subset/manifest.jsonl \
  --candidate-dir runs/released/agent_panel_v1/results/ \
  --lean-root   packages/lean/EvalLadder \
  --summary     runs/released/agent_panel_v1/l4_summary.json
```

By default, bundles that already carry `proof_results.json` are
treated as sealed and skipped. Pass `--overwrite` to replace them.
The `--summary` file is a deterministic JSON listing one row per
bundle in sorted-path order.

### Paper pipeline (Milestone G)

Milestone G converts a directory of sealed evidence bundles into the
paper's analysis tables. All subcommands are pure - they never re-run
a candidate - and every `*_results.json` in every bundle is treated as
authoritative.

Examples below use `runs/released/agent_panel_v1/results/` as a compact **frozen**
directory so commands fit on one line. For **paper-facing** reruns, set `--run-dir`
(and matching `--out-dir` parents) to the released surfaces named in `README.md`
and `docs/submission_checklist.md` (for example Live v2, L2 flagship, Rust proof).

#### Input resolution

Every `analyze` subcommand accepts a single `--run-dir` argument. The
resolver prefers, in order:

1. `<run-dir>/analysis_input.json` (explicit, for curated datasets and
   regression tests).
2. A directory of per-candidate bundles. The loader walks it
   lexicographically via
   `eval_ladder_analysis::load_bundle_dir` and fails with a structured
   error if any bundle is missing `candidate_resolution.json`, has no
   `*_results.json`, or reports a `candidate_id` / `task_id` that
   disagrees with its own `candidate_resolution.json`.

#### Subcommands

```bash
# Per-table exports (CSV by default; --out and --json-out are optional).
cargo run --bin eval-ladder -- analyze score-descent             --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze conditional-reversal --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze rank-stability            --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze taxonomy                  --run-dir runs/released/agent_panel_v1/results/
cargo run --bin eval-ladder -- analyze static-vs-live            --run-dir runs/released/agent_panel_v1/results/

# One-shot paper export: writes every table + manifest.json into a
# dedicated directory and prints the manifest to stdout.
cargo run --bin eval-ladder -- analyze paper-export \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out-dir paper/exports/agent_panel_v1/
```

#### Determinism contract

`analyze paper-export` is byte-deterministic for any fixed
`AnalysisInput`:

- Floats are written with six-digit fixed precision.
- JSON goes through `eval_ladder_core::canonical_json` (sorted keys,
  `\n` line endings, shortest round-trippable floats).
- `manifest.json` records `{path, sha256, bytes}` for every other
  file plus `schema_version`, `evaluator_version`, and
  `input_row_count`.

The invariant is pinned by
`packages/rust/analysis/tests/milestone_g_acceptance.rs`, which runs
the full `load_bundle_dir` -> `write_paper_exports` pipeline twice and
requires byte-identical outputs.

#### Static-vs-live comparison (Milestone L)

Milestone L adds a fifth paper-export pair,
`static_vs_live.{csv,json}`, and a matching one-shot subcommand
`analyze static-vs-live`. It is the shipped implementation of the
paper's "overstatement" claim.

```bash
# One-shot static-vs-live table to stdout as CSV.
cargo run --bin eval-ladder -- analyze static-vs-live \
  --run-dir runs/released/agent_panel_v1/results/

# Write the canonical JSON sibling alongside the CSV.
cargo run --bin eval-ladder -- analyze static-vs-live \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out paper/exports/agent_panel_v1/static_vs_live.csv \
  --json-out paper/exports/agent_panel_v1/static_vs_live.json
```

Contract (see `docs/architecture.md` for bundle layout and field-level schema):

- One row per `(agent_id, level)` with data on either the static
  arm (`SweBenchVerified`) or the live arm (`SweBenchLive`).
- `delta = live_pass_rate - static_pass_rate`; empty when either
  side is undefined.
- `ratio = live_pass_rate / static_pass_rate`; empty when either
  side is undefined or the static rate is zero.
- `RustSweBench` is intentionally excluded from this table; it is a
  separate paper surface.

`analyze paper-export` bumped `PAPER_EXPORT_SCHEMA_VERSION` from `1`
to `2` for static-vs-live, and from `2` to `3` for explicit
`analysis_mode` provenance (`raw` vs `cumulative`) in
`manifest.json`. Readers keyed on the manifest hash re-pin
intentionally.
Filename-based readers remain forward-compatible. Determinism is
pinned by
`packages/rust/analysis/tests/milestone_l_acceptance.rs`.

#### Raw vs cumulative headline semantics (P0)

Analysis now supports two semantics:

- `raw`: preserves evaluator contract exactly; each level is independent.
- `cumulative`: for headline reporting only, upper-level pass requires
  lower-level pass prerequisites.

CLI usage:

```bash
# Raw (appendix/debug).
eval-ladder analyze score-descent --run-dir <run_dir> --analysis-mode raw

# Cumulative (headline).
eval-ladder analyze score-descent --run-dir <run_dir> --analysis-mode cumulative

# paper-export defaults to cumulative; override to raw explicitly when needed.
eval-ladder analyze paper-export --run-dir <run_dir> --out-dir <out_dir>
eval-ladder analyze paper-export --run-dir <run_dir> --out-dir <out_dir> --analysis-mode raw
```

Publication gate commands for the evidence tranches (strict vs
`--gate-profile release`) are centralized in
`docs/evidence_empirical_status.md`.

### Batch evaluation (Milestone H)

`eval-ladder evaluate batch` drives the full L0-L4 pipeline over a
panel JSONL, producing one sealed evidence bundle per entry plus a
deterministic `batch_summary.json` at the root of `--out`.

The **full panel drive** recipe below uses `runs/released/agent_panel_v1/` as a
small frozen **CLI illustration**. NeurIPS headline panels and optimization
outputs live under paths documented in `README.md` (Core workflows) and
`docs/submission_checklist.md`. Swap `--input` / `--out` / `--run-dir` to match
the panel you are reproducing.

#### Panel schema

One JSON object per non-blank, non-`#` line. Per-entry paths are
resolved relative to the directory containing the panel file when
they are not absolute.

```json
{
  "task": "benchmarks/verified/manifests/task_001.json",
  "candidate": "candidates/agent_a/task_001.json",
  "patch": "patches/agent_a/task_001.diff",
  "workspace_template": "/var/eval-ladder/snapshots/task_001/",
  "bundle_name": "agent_a__task_001",
  "entry_id": "agent_a/task_001"
}
```

`bundle_name` and `entry_id` are optional. `bundle_name` defaults to
the candidate's stringified UUID; `entry_id` defaults to
`bundle_name`. Unknown fields are rejected (`serde(deny_unknown_fields)`).

#### Full panel drive

```bash
# 1. Ingest the benchmark.
cargo run --bin eval-ladder -- ingest verified \
  --manifest configs/evaluator/verified.toml \
  --source datasets/public_links/verified.jsonl

# 2. Drive the entire panel.
cargo run --bin eval-ladder -- evaluate batch \
  --input runs/released/agent_panel_v1/panel.jsonl \
  --levels L0,L1,L2,L3,L4 \
  --config configs/evaluator/verified.toml \
  --out runs/released/agent_panel_v1/results/ \
  --strengthening-spec configs/strengthening/default.json \
  --policy configs/policy/default_policy.toml \
  --obligations datasets/derived/proof_subset/manifest.jsonl \
  --lean-root packages/lean/EvalLadder

# 3. (Optional) Iterate only the proof subset with the batch-wide
#    Lean checker. prove-subset reuses the bundles written in step 2.
cargo run --bin eval-ladder -- prove-subset \
  --subset datasets/derived/proof_subset/manifest.jsonl \
  --candidate-dir runs/released/agent_panel_v1/results/ \
  --lean-root packages/lean/EvalLadder

# 4. Emit paper outputs (Milestone G).
cargo run --bin eval-ladder -- analyze paper-export \
  --run-dir runs/released/agent_panel_v1/results/ \
  --out-dir paper/exports/agent_panel_v1/
```

#### Wall-clock optimizations (long batches)

Use the **release** CLI driver. Long-batch `just` recipes depend on
`just eval-ladder-cli-release` and run `target/release/eval-ladder` (or
`eval-ladder.exe` on Windows) directly so each batch avoids `cargo run` startup.
For one-off invocations you can still use
`cargo run -p eval-ladder-cli --release -- evaluate …`.

| Knob | Effect |
| --- | --- |
| `--rust-target-cache-root <dir>` | Sets `CARGO_TARGET_DIR` for Rust-heavy rows (Verified smart reuse, Rust-SWE-bench). Point at a **stable directory on fast local disk** (for example `runs/released/.eval_ladder_cargo_cache`) so repeated compiles reuse artifacts across batches. |
| `--dedupe-workloads` | Default **on**; skips redundant Docker work when task+patch+candidate bytes match another row (multi-agent safe after the candidate-aware workload key). |
| `--resume` | Skips entries whose bundle dirs already completed; safe for interrupted runs. |
| `--jobs N` | Overlaps **different** panel rows (`N` of 2–4 on a strong host). Reduces wall time when Docker and disk keep up; drop to `1` if the engine thrashes. |
| `--adaptive-timeouts` + `--short-timeout-secs` | After cheap failure patterns in a prior summary, later rows use shorter per-exec timeouts so bad harness rows fail faster than `--timeout-secs`. |
| Fewer `--levels` | For **iteration only**, run the minimum ladder you need (Verified headline gate uses `L0,L1,L3`; skip `L4` until you need proof rows). Rust policy iteration: `--track fast` runs **L3,L4 only**; seal with a full `L0,L1,L3,L4` pass when semantics are stable. |
| Smaller panel | Shrink `panel.jsonl` or tighten `preflight_verified_selectors.py --strict` / `filter_panel_upstream_resolved.py` while debugging harness clusters. |
| Image prewarm | Run `python ci/scripts/prewarm_panel_images.py --panel <panel.jsonl>` (optional `--parallel N`). Uses the **same SWE-bench image name candidates** as the Rust Docker engine (legacy `org__repo` then `org_1776_repo`). Local hits skip `docker pull`. Pull failures are **non-fatal by default** (compact stderr unless `--strict-pulls`). `cargo://…` and other non-OCI schemes are skipped. |

**`just` recipes** (from the repo root; see `just --list`):

- **`just verified-batch-optimized-prewarmed <panel.jsonl> <out_dir> [jobs] [cache] [prewarm_parallel]`** — pull images for that panel, then Verified `L0,L1,L3` batch (recommended default for wall clock).
- `just verified-batch-optimized <panel.jsonl> <out_dir> [jobs] [cache]` — same batch without a preceding pull (use if images are already local).
- **`just live-batch-v2-optimized-prewarmed <out_dir> [jobs] [prewarm_parallel]`** — prewarm `runs/released/live_panel_v2/panel.jsonl`, then Live batch (NeurIPS comparative panel).
- `just live-batch-v2-optimized <out_dir> [jobs]`
- Legacy **v1** recipes (`just live-batch-optimized-prewarmed`, paths under `runs/released/live_panel_v1/`) remain for older comparisons only.
- **`just rust-proof-batch-fast-prewarmed <out_dir> [prewarm_parallel]`** / **`just rust-proof-batch-seal-prewarmed <out_dir> [jobs] [cache] [prewarm_parallel]`** — Rust proof subset panel, then fast or seal batch.
- `just rust-proof-batch-fast <out_dir>` — fast L3/L4 iteration
- `just rust-proof-batch-seal <out_dir> [jobs] [cache]` — full ladder for sealing
- `just prewarm-panel <panel.jsonl> [parallel]` — pull only (default parallel **4**; best-effort exit code).
- `just prewarm-panel-strict <panel.jsonl> [parallel]` — same, but `--strict-pulls` (fail if any pull fails).

Also keep Docker Desktop CPU/memory limits reasonable, and place `--out` on
local SSD (not a network filesystem).

##### Full commands (copy-paste)

From the repository root, after a toolchain or dependency change, build the driver once (optional on first batch: the recipes below already depend on `eval-ladder-cli-release`):

```powershell
cd <path-to-repository>
just eval-ladder-cli-release
```

**Verified panel (prewarm + batch, recommended):**

```powershell
just verified-batch-optimized-prewarmed runs\released\agent_panel_v3_r1\panel_preflight_clean.jsonl runs\released\agent_panel_v3_r1\results_opt
```

That recipe already runs prewarm first; do not also run `python ci/scripts/prewarm_panel_images.py` in the same workflow unless you want a standalone pull step.

**Verified panel (batch only, images already local):**

```powershell
just verified-batch-optimized runs\released\agent_panel_v3_r1\panel_preflight_clean.jsonl runs\released\agent_panel_v3_r1\results_opt
```

**Live panel (v2):**

```powershell
just live-batch-v2-optimized-prewarmed runs\released\live_panel_v2\results_opt
```

**Rust proof subset — fast iteration then seal:**

```powershell
just rust-proof-batch-fast-prewarmed runs\released\rust_proof_subset_v1\results_fast
just rust-proof-batch-seal-prewarmed runs\released\rust_proof_subset_v1\results_seal
```

**Prewarm only (custom `evaluate batch` afterward):**

```powershell
python ci/scripts/prewarm_panel_images.py --panel runs\released\live_panel_v2\panel.jsonl --parallel 4
```

**Bash / WSL / Linux** (same flags; use forward slashes):

```bash
cd <path-to-repository>
just eval-ladder-cli-release
just verified-batch-optimized-prewarmed runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl runs/released/agent_panel_v3_r1/results_opt
just live-batch-v2-optimized-prewarmed runs/released/live_panel_v2/results_opt
just rust-proof-batch-fast-prewarmed runs/released/rust_proof_subset_v1/results_fast
just rust-proof-batch-seal-prewarmed runs/released/rust_proof_subset_v1/results_seal
```

**Without `just`** (equivalent to `verified-batch-optimized` after `cargo build -p eval-ladder-cli --release`; Windows: `target\release\eval-ladder.exe`):

```bash
cargo build -q -p eval-ladder-cli --release
./target/release/eval-ladder evaluate batch \
  --input runs/released/agent_panel_v3_r1/panel_preflight_clean.jsonl \
  --config configs/evaluator/verified.toml \
  --levels L0,L1,L3 \
  --policy configs/policy/default_policy.toml \
  --out runs/released/agent_panel_v3_r1/results_opt \
  --timeout-secs 3600 --short-timeout-secs 900 \
  --adaptive-timeouts --resume --jobs 2 \
  --l1-strategy smart_rust_reuse \
  --rust-target-cache-root runs/released/.eval_ladder_cargo_cache \
  --seed-tag verified-batch-opt --deterministic-clock
```

#### Resilience contract

One bad entry never aborts the batch. Any recoverable error between
panel-line load and pipeline dispatch becomes a row with
`status: "invalid"` and an `error` field whose code starts with
`BATCH_LOAD_FAILED` or `BATCH_PIPELINE_FAILED`. The CLI exits
non-zero only when the panel itself is unreadable or when *every*
entry failed.

#### Determinism contract

With `--deterministic-clock`, the batch output is byte-deterministic:

- Each per-entry bundle is sealed by the same deterministic pipeline
  used by `evaluate candidate`, so bundle hashes are stable across
  reruns.
- `batch_summary.json` is produced by
  `eval_ladder_core::canonical_json` (sorted keys, `\n` line endings,
  shortest round-trippable floats). Entries are sorted by
  `bundle_name`.
- Wall-clock `started_at`/`finished_at` fields are omitted in
  deterministic mode so that the summary has no time-dependent bytes.

The invariant is pinned by the `milestone_h_batch_summary_is_deterministic`
test in `packages/rust/cli/src/commands/batch.rs`, which runs the
end-to-end batch twice on a 3-entry panel and asserts matching
bundle hashes and matching summary content.

Single-candidate drive remains available via `evaluate candidate`
when needed for debugging.

#### Released Rust-native pilot path

`runs/released/rust_pilot_v1/` is the shipped Docker-free pilot run.
It uses the host Rust toolchain through `LocalProcessEngine`:

```powershell
.\target\debug\eval-ladder.exe evaluate batch `
  --input "runs/released/rust_pilot_v1/panel.jsonl" `
  --config "configs/evaluator/rust.toml" `
  --levels L0,L1,L3,L4 `
  --policy "configs/policy/rust_pilot.toml" `
  --obligations "datasets/derived/proof_subset/manifest.jsonl" `
  --lean-root "packages/lean/EvalLadder" `
  --out "runs/released/rust_pilot_v1/results" `
  --timeout-secs 3600 `
  --deterministic-clock
```

Released summary (`batch_summary.json`):

- L0: `L0_OFFICIAL_TIMEOUT`
- L1: `L1_HARNESS_ERROR`
- L3: `PASS`
- L4: `L4_OBLIGATION_MET`

Released integrity check (`verify_report.json`):

- `1 ok / 0 invalid` (`trace: ok`, bundle hash sealed)

### SWE-bench Verified normalization (Milestone I)

The Python compat layer ships a working ingestor for SWE-bench
Verified release manifests. The CLI is installed as
`eval-ladder-py` (from the repo-root `pyproject.toml`) and is also
reachable via `python -m benchmark_compat.cli`.

```bash
# 1. Install the Python layer in the active environment.
python -m pip install -e ".[dev]"

# 2. Normalize a SWE-bench Verified JSONL manifest into per-task
#    BenchmarkTask files accepted by the Rust evaluator.
eval-ladder-py normalize-swe-bench \
  --source datasets/public_links/verified.jsonl \
  --out-dir benchmarks/verified/manifests/

# 3. (Optional) Abort on the first malformed record instead of
#    continuing past bad entries.
eval-ladder-py normalize-swe-bench \
  --source datasets/public_links/verified.jsonl \
  --out-dir benchmarks/verified/manifests/ \
  --strict
```

#### Output contract

- One `<task_id>.json` per input record under `--out-dir`.
- Bytes are canonical JSON: sorted keys, UTF-8, shortest
  round-trippable numbers, no trailing whitespace. This matches
  `eval_ladder_core::canonical_json` byte-for-byte and is pinned by
  the `milestone_i_python_emitted_benchmark_task_deserializes_in_rust`
  integration test.
- Every emitted file is re-validated against
  `schemas/benchmark_task.schema.json` before it is written; emission
  fails loudly if pydantic and the JSON Schema drift.
- Gold patches are referenced as `sha256:<hex>` of the raw patch
  bytes; the patch content is not written to disk by this command
  (benchmark ingest writes it separately).

#### Resilience contract

Default mode: per-record errors are logged to stderr and the bad
record is skipped; exit `0` if at least one record was emitted,
exit `3` if every record failed. `--strict` aborts on the first
failure with exit `3`. IO or manifest-parse failures exit `2`.

### Bundle and trace verification (Milestone J)

Any reviewer, CI job, or downstream archivist can recompute every
digest in an evidence bundle with a single shipped command. The
`eval-ladder verify` subcommand is the only endorsed entry point for
this task; it wraps `eval_ladder_evidence::verify_bundle` and
`eval_ladder_traces::TraceReader::read_and_verify` behind a stable
CLI and a canonical JSON report.

```bash
# 1. Verify a single bundle directory (default: also verifies trace.jsonl).
eval-ladder verify bundle --bundle-dir runs/released/bundle-sympy-22005

# 2. Verify only a trace's hash chain.
eval-ladder verify trace --trace runs/released/bundle-sympy-22005/trace.jsonl

# 3. Verify every bundle under a batch run directory and emit a
#    canonical verify_report.json alongside it.
eval-ladder verify run-dir --run-dir runs/released

# 4. Fail-fast variant for short-circuit CI checks.
eval-ladder verify run-dir --run-dir runs/released --fail-fast
```

#### Report contract

`verify run-dir` writes `verify_report.json` (canonical JSON, sorted
keys, UTF-8, no trailing whitespace) with the following shape:

- `schema_version`: u32 (currently `1`).
- `evaluator_version`: semver string of the evaluator that produced
  the report.
- `run_dir`: absolute path of the verified directory.
- `total`, `ok`, `invalid`: entry counters.
- `entries`: array of per-bundle rows, **sorted by `bundle_name`**
  for stable diffs.

Each row is:

- `bundle_name`, `bundle_dir`.
- `status`: `ok` iff both bundle and trace checks passed
  (`trace` may be `not_applicable` when requested via
  `--verify-trace false`).
- `bundle_hash`: content-addressed SHA-256 of the bundle index when
  parseable, otherwise omitted.
- `bundle`, `trace`: per-check status.
- `error_code`, `error`: stable error code (`VERIFY_*`, see below)
  and a human message when `status == invalid`.

#### Stable error codes

- `VERIFY_FILE_DIGEST_MISMATCH` - a bundle file hashed differently
  from its entry in `artifact_hashes.json`.
- `VERIFY_BUNDLE_DIGEST_MISMATCH` - the recomputed bundle-level
  hash did not match the stored `bundle_hash`.
- `VERIFY_MISSING_FILE` - the index declares a file that is not on
  disk.
- `VERIFY_BUNDLE_PARSE`, `VERIFY_BUNDLE_IO`, `VERIFY_BUNDLE_CORE` -
  structural failures reading `artifact_hashes.json`.
- `VERIFY_TRACE_MISSING`, `VERIFY_TRACE_IO`, `VERIFY_TRACE_PARSE`,
  `VERIFY_TRACE_CORE` - trace file I/O / deserialize failures.
- `VERIFY_TRACE_HASH_MISMATCH` - a trace event's recomputed
  `event_hash` did not match the stored value.
- `VERIFY_TRACE_CHAIN_BROKEN` - a trace event's `prev_event_hash`
  did not match the preceding event's `event_hash`.
- `VERIFY_TRACE_FIRST_NOT_RUN_STARTED`,
  `VERIFY_TRACE_DUPLICATE_RUN_STARTED` - structural trace
  violations.

#### Exit codes

- `0`: every bundle verified successfully.
- non-zero: at least one bundle failed. The report is still
  written so reviewers can triage offline. `--fail-fast` aborts
  before enumerating the remaining entries.

#### Determinism contract

`verify_report.json` is byte-deterministic across reruns for the
same inputs modulo the `run_dir` / `bundle_dir` strings (which
carry absolute paths for operator ergonomics). Content-bearing
fields (`bundle_hash`, `status`, `bundle`, `trace`, `error_code`)
are strictly deterministic and suitable for CI diff gates.

### Reproducibility demo (Milestone K)

The `eval-ladder demo run` command is the single command a reviewer
runs to confirm the repository builds, executes, and emits
hash-verifiable artifacts without any upstream benchmark data,
network access, or container runtime. It materializes a wholly
synthetic panel, drives the batch pipeline over it with a
deterministic clock, emits the Milestone G paper exports, and
re-verifies every produced bundle in-process.

```bash
# Smallest usable slice (2 tasks; ~1 s on a developer laptop).
eval-ladder demo run --out runs/demo --tasks 2

# Larger slice for timing experiments (stays well under the
# 15-minute reviewer budget).
eval-ladder demo run --out runs/demo --tasks 25

# Batch + verify only (skip the analyze step).
eval-ladder demo run --out runs/demo --tasks 2 --skip-analyze
```

#### Output layout

```
<out>/
  inputs/               # Synthetic panel + per-entry fixtures.
    evaluator.toml
    panel.jsonl
    demo-00/
      task.json
      candidate.json
      patch.diff
      workspace/README.md
    ...
  bundles/              # One sealed evidence bundle per task.
    batch_summary.json
    verify_report.json  # Written by the verify step.
    bundle-demo-00/
      artifact_hashes.json
      candidate_resolution.json
      run_manifest.json
      trace.jsonl
      ...
    ...
  paper/                # Milestone G + L paper-export tables.
    score_descent.{csv,json}
    conditional_reversal.{csv,json} (plus deprecated byte-identical
    conditional_false_success.{csv,json} aliases)
    rank_stability.{csv,json}
    taxonomy.{csv,json}
    static_vs_live.{csv,json}     # Milestone L
    manifest.json                 # schema_version = 3 (includes analysis_mode)
```

#### Determinism contract

Every artifact emitted by `demo run` is a pure function of
`(--out layout, --tasks)`:

- Task IDs, candidate IDs, bundle IDs, and timestamps are all
  derived from a pinned namespace UUID and a fixed wall clock
  (`2025-01-01T00:00:00Z`).
- Bundle hashes and `verify_report.json` content are byte-
  deterministic across reruns (pinned by
  `milestone_k_demo_is_byte_deterministic_across_runs`).
- The end-to-end invariants (all bundles ok, all paper tables
  emitted, verify report all-green) are pinned by
  `milestone_k_demo_runs_end_to_end`.

#### When to use which flag

| Intent                              | Flags                        |
|-------------------------------------|------------------------------|
| Reviewer smoke test                 | `--tasks 2`                  |
| Exercising the full analysis seam   | default                      |
| Performance budgeting              | `--tasks 25 --skip-analyze` (or more) |
| CI "is it alive?" gate              | `--tasks 2 --skip-analyze`   |

### CI tiers

Full specifications live under `.github/workflows/`.

#### Tier 1 (fast)
- Rust unit tests, `cargo fmt --check`, `cargo clippy -D warnings`.
- JSON Schema validation.
- Runs on every PR.

#### Tier 2 (medium)
- Tier 1 plus Python adapter tests (`pytest`), `ruff`, `mypy`.
- Trace and evidence integration tests against fixture tasks.
- Mock container runs (no real Docker).
- Runs on every PR.

#### Tier 3 (heavy)
- Sampled benchmark replay on the miniature internal fixture suite
  (3 Python tasks, 2 Rust tasks, 2 proof-subset tasks).
- Lean proof-subset smoke tests.
- Runs nightly and on tagged releases; never in the PR critical path.

Full benchmark evaluation is never run in CI. It is run explicitly on
release machines and the outputs are committed under `runs/released/`.

### Release hygiene

Before a release:

- Bump versions in all `Cargo.toml`s and `pyproject.toml`.
- Run `cargo deny check` and `cargo audit` (or `just deny` / `just audit`).
- Refresh `paper/exports/` from a clean Tier 3 run.
- Update `docs/submission_checklist.md`.
- Tag the release `vX.Y.Z` and attach the evidence-bundle index to the
  GitHub release.

### Incident triage

If an evaluator result is disputed:

1. Locate the evidence bundle at
   `runs/released/<panel>/results/<candidate_id>/`.
2. Recompute the bundle hash and verify it matches
   `artifact_hashes.json`.
3. Inspect `trace.jsonl`. If the hash chain is broken, the bundle is
   tampered; escalate.
4. Re-run the single candidate with
   `eval-ladder evaluate candidate --candidate ...`.
5. If the new run diverges, open an issue tagged `evaluator-regression`
   with both bundles attached.
