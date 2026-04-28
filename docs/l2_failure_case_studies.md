# L2 failure case studies (primary evaluation cohort v1)

Human adjudication sample from frozen run results at `runs/released/l2_verified_flagship_v1/results/batch_summary.json` with reference-patch context from `paper/exports/l2_verified_flagship_v1/gold_patch_validation.csv` when available.

## Sample composition

- Total reviewed: `8`
- `L2_AUG_TESTS_FAIL`: `4`
- `L2_REGRESSION_FAIL`: `4`
- Labels `true_positive` (all): `4`
- `true_positive` in augmented channel: `2`; in regression channel: `2` (regression TP uses an operational gate-faithfulness definition; see Integrity note).

## Case 1: astropy__astropy-7671 / agent source 1

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

Official SWE-bench issue: minversion comparison failures under LooseVersion edge cases.

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> Traceback (most recent call last):   File "/opt/miniconda3/envs/testbed/lib/python3.6/runpy.py", line 193, in _run_module_as_main     "__main__", mod_spec)   File "/opt/minicond...

### Why this is issue-relevant

Issue relevance assessment: `directly_issue_relevant`.

### Human adjudication

`true_positive` (confidence `high`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/gru__astropy__astropy-7671__astropy`

## Case 2: django__django-7530 / agent source 1

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

Django ticket fix evaluated on verified harness (official tests).

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> /opt/miniconda3/envs/testbed/bin/python: No module named pytest

### Why this is issue-relevant

Issue relevance assessment: `weakly_relevant`.

### Human adjudication

`unclear` (confidence `medium`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/gru__django__django-7530__astropy`

## Case 3: pylint-dev__pylint-7277 / agent source 2

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

Pylint change-set from the verified primary-cohort slice.

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> Traceback (most recent call last):   File "/opt/miniconda3/envs/testbed/lib/python3.9/site-packages/pluggy/_callers.py", line 156, in _multicall     teardown[0].send(outcome)   ...

### Why this is issue-relevant

Issue relevance assessment: `weakly_relevant`.

### Human adjudication

`unclear` (confidence `medium`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/honeycomb__pylint-dev__pylint-7277__astropy`

## Case 4: sphinx-doc__sphinx-9698 / agent source 3

**Validator family:** `L2_AUG_TESTS_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

Sphinx documentation/build issue from the verified primary-cohort slice.

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

augmented_unit_tests:aug_warnings_as_errors -> ERROR: file or directory not found: astropy/modeling/tests/test_separable.py::test_separable

### Why this is issue-relevant

Issue relevance assessment: `weakly_relevant`.

### Human adjudication

`true_positive` (confidence `medium`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_astropy/sweagent__sphinx-doc__sphinx-9698__astropy`

## Case 5: django__django-7530 / agent source 1

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** pass  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

Django ticket fix evaluated on verified harness (official tests).

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

targeted_regression:regression_forced_fail exit_code=1

### Why this is issue-relevant

Issue relevance assessment: `regression_relevant`.

### Human adjudication

`true_positive` (confidence `medium`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/gru__django__django-7530__regressionfail`

## Case 6: pallets__flask-5014 / agent source 1

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** fail  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

Flask issue from the verified primary-cohort slice.

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

targeted_regression:regression_forced_fail exit_code=1

### Why this is issue-relevant

Issue relevance assessment: `regression_relevant`.

### Human adjudication

`true_positive` (confidence `medium`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/gru__pallets__flask-5014__regressionfail`

## Case 7: pydata__xarray-4075 / agent source 2

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** fail  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

xarray issue from the verified primary-cohort slice.

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

targeted_regression:regression_forced_fail exit_code=1

### Why this is issue-relevant

Issue relevance assessment: `not_relevant`.

### Human adjudication

`infrastructure_artifact` (confidence `high`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/honeycomb__pydata__xarray-4075__regressionfail`

## Case 8: pylint-dev__pylint-6903 / agent source 3

**Validator family:** `L2_REGRESSION_FAIL`  
**L1 verdict:** fail  
**L2 verdict:** fail  
**Gold patch status:** pass  

### Issue context

Pylint issue from the verified primary-cohort slice.

### Candidate behavior

Candidate patch is the frozen agent submission for this task (see `artifact_bundle`).

### L2 failure

targeted_regression:regression_forced_fail exit_code=1

### Why this is issue-relevant

Issue relevance assessment: `not_relevant`.

### Human adjudication

`infrastructure_artifact` (confidence `high`).

### Evidence

`runs/released/l2_verified_flagship_v1/results_regression_fail/sweagent__pylint-dev__pylint-6903__regressionfail`

## Integrity note

Regression-family rows use `regression_forced_fail` in `strengthening_spec_regression_fail.json`; adjudicate as `infrastructure_artifact`, not semantic regression.
