# L2 selection protocol (flagship v1)

This document is the **auditable protocol** for the 66-row L2 flagship slice
`runs/released/l2_verified_flagship_v1/`. The slice is **validator-focused and
diagnostic**: it is not a random draw from a well-defined superpopulation, and
it should not support population-level generalizations beyond the stated
construction. A per-row machine-generated index is
`paper/exports/l2_verified_flagship_v1/l2_selection_manifest.{csv,json}`.

## 1. Source and scope

- **Source benchmark (candidates).** The L2 layer reuses the same **sealed
  agent candidate files** as the **Verified flagship** panel
  `runs/released/agent_panel_verified_flagship_v1/`, which in turn was derived
  from `agent_panel_v3_r1` under fixed prefix exclusions (Fragile tooling
  prefixes such as `matplotlib__`, `scikit-learn__`, `pytest-dev__` were dropped
  when assembling the flagship panel).

- **Task IDs included.** Exactly the **eleven SWE-Bench Verified–style tasks**
  present in `agent_panel_verified_flagship_v1/panel.jsonl` (each task appears
  once per agent in the base 33 rows).

- **Agents / candidate sources.** Three public agent IDs used in that flagship
  panel **gru**, **honeycomb**, **sweagent** — one winner-style candidate JSON
  per (task, agent) drawn from the flagship candidate store.

- **Candidate patches per (task, agent).** **One** sealed candidate per pair in
  the base panel; L2 does not sweep multiple candidates per pair in flagship v1.

## 2. Selection rule (what produced the 66 entries)

The L2 flagship slice was built by taking the **33** base `(task, agent)`
rows and applying a **deterministic, pre-declared pair of validator arms**:

1. Each base row is evaluated under the **augmented-test** arm (bundle suffix
   `__astropy`, results under `results_astropy/`).
2. The **same** base row is evaluated under the **regression** arm (bundle
   suffix `__regressionfail`, results under `results_regression_fail/`).

Thus **33 + 33 = 66** sealed rows appear in
`runs/released/l2_verified_flagship_v1/results/batch_summary.json`. The two
single-family batch summaries were merged with
`ci/scripts/merge_l2_batch_summaries.py` **without dropping rows**.

Selection does **not** depend on whether a candidate passed or failed L2.

## 3. Exclusion rule (upstream and operational)

No additional candidate rows were excluded **after** fixing the flagship base
panel for L2 purposes.

**Upstream exclusions (verified flagship assembly only):**

| Category | Applied in flagship v1? |
|----------|-------------------------|
| Known fragile task prefixes / tooling (`matplotlib__`, `scikit-learn__`, `pytest-dev__`) | Yes — tasks removed when trimming `agent_panel_v3_r1` into `agent_panel_verified_flagship_v1`. |
| Missing workspace / bundle materialization failure | Handled at batch execution time (row may be `invalid` in summaries); not an L2-specific exclusion rule. |
| Patch does not apply | Surfaced as harness or strengthener failure per row; not used to shrink the 66-row design. |
| Official scorer unavailable | Same — execution-level, not used to redefine the 66-row set post hoc. |
| Gold patch unavailable | Does not exclude a candidate row from L2; it affects **gold-patch validation** evidence only. |
| Validator not applicable | Not used to delete rows in flagship v1; both arms are run for every base row. |
| Known flaky task | Only via the upstream prefix policy above. |

## 4. Validator construction

### L2_AUG_TESTS_FAIL (`augmented_unit_tests`)

- **How validators were chosen.** Checked-in strengthening specs under
  `runs/released/l2_verified_astropy_v1/strengthening_spec.json` (referenced by
  the flagship batch README) define augmented pytest-style commands including
  warnings-as-errors stress paths.
- **Written before seeing candidate failures?** The specs are **versioned
  artifacts** committed independently of any particular candidate outcome.
- **Checked against gold patches?** Yes — see
  `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv` and
  `docs/l2_gold_patch_validation.md`.
- **Task-specific vs generic.** Commands can target repo-specific selectors; the
  **pattern** (official rerun + augmented selectors) is shared.
- **Generated tests failing on gold?** Gold outcomes are reported explicitly;
  failures trigger manual review per the gold validation protocol (validator
  bug, exclusion, or documented limitation).

### L2_REGRESSION_FAIL (`targeted_regression`)

- **How validators were chosen.**
  `runs/released/l2_verified_flagship_v1/strengthening_spec_regression_fail.json`
  defines the regression validator, including **`regression_forced_fail`**.
- **Before candidate outcomes?** Same as above — checked-in spec.
- **Gold-patch check?** Same CSV as above — gold rows must be interpreted with
  the forced-fail caveat (see Integrity note below).
- **Task-specific vs generic.** The forced-fail hook is **generic protocol** on
  flagship v1.
- **Tests failing on gold removed post hoc?** No dynamic removal — outcomes are
  reported.

## 5. Post-hoc handling

- **Rows removed after L2 execution?** **No.** The merged `results/` directory
  retains both arms.

- **Validators modified after observing candidate failures?** **Not as part of
  flagship v1 sealed reruns.** Any future change would require a new protocol
  version and new sealed directories.

- **Task families merged?** The **66-row** summary is a **merge of two
  validator arms** (`results_astropy` + `results_regression_fail`) for the same
  33 base rows — not a merge of disjoint task universes.

- **Summaries deduplicated?** Entry IDs are unique per `(agent, task, arm)`; no
  deduplication beyond the merge script’s deterministic join.

## Integrity note (regression arm)

In flagship v1, `targeted_regression` includes **`regression_forced_fail`**
(non-zero exit by design). Interpret **L2_REGRESSION_FAIL** on this arm as a
**controlled protocol signal**, not standalone proof of product regression on
the ticket.

## Relationship to gold-patch validation

Gold-patch validation is intentionally documented as a separate protocol in
`docs/l2_gold_patch_validation.md` and does **not** change candidate row
selection for this 66-row slice.

- Candidate strict-arm evidence remains tied to
  `results_astropy/` + `results_regression_fail/`.
- Gold headline legitimacy checks use the pre-declared
  `strengthening_spec_gold_mechanical.json` profile to avoid conflating strict
  negative-control artifacts with validator validity.
- Strict gold replay with agent specs is still available via
  `--strict-flagship-specs` for parity diagnostics.
