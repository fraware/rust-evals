# L2 failure case studies (flagship v1)

This note summarizes a six-case adjudication sample from
`runs/released/l2_verified_flagship_v1/results/batch_summary.json`.

## Sample composition

- Total reviewed: `6`
- Augmented-test failures: `3`
- Regression failures: `3`
- Adjudication split: true positive `3`, false positive `3`, unclear `0`

## Per-case notes

### Case 1: `astropy__astropy-7671` / `gru`

- Candidate: `203e0cbc-5ece-56a8-9a2f-87c80e23913c`
- L1 verdict: `pass`
- L2 family: `L2_AUG_TESTS_FAIL` (`augmented_unit_tests`)
- Failure summary: augmented_unit_tests:aug_warnings_as_errors -> Traceback (most recent call last):   File "/opt/miniconda3/envs/testbed/lib/python3.6/runpy.py", line 193, in _run_module_as_main     "__main__", mod_spec)   File "/opt/minicond...
- Issue relevance rationale: Runs an additional pytest selector under warnings-as-errors to probe behavior beyond the official rerun.
- Adjudication: `true_positive`
- Reviewer notes: Failure occurs on the same repository as the target issue and in an augmented test path; treated as issue-relevant.

### Case 2: `astropy__astropy-7671` / `honeycomb`

- Candidate: `28bac07d-9ede-5b1b-a50d-060a4f3852dd`
- L1 verdict: `pass`
- L2 family: `L2_AUG_TESTS_FAIL` (`augmented_unit_tests`)
- Failure summary: augmented_unit_tests:aug_warnings_as_errors -> Traceback (most recent call last):   File "/opt/miniconda3/envs/testbed/lib/python3.6/runpy.py", line 193, in _run_module_as_main     "__main__", mod_spec)   File "/opt/minicond...
- Issue relevance rationale: Runs an additional pytest selector under warnings-as-errors to probe behavior beyond the official rerun.
- Adjudication: `true_positive`
- Reviewer notes: Failure occurs on the same repository as the target issue and in an augmented test path; treated as issue-relevant.

### Case 3: `astropy__astropy-7671` / `sweagent`

- Candidate: `111ef252-bbe6-5d6f-9330-7c2037e10b97`
- L1 verdict: `pass`
- L2 family: `L2_AUG_TESTS_FAIL` (`augmented_unit_tests`)
- Failure summary: augmented_unit_tests:aug_warnings_as_errors -> Traceback (most recent call last):   File "/opt/miniconda3/envs/testbed/lib/python3.6/runpy.py", line 193, in _run_module_as_main     "__main__", mod_spec)   File "/opt/minicond...
- Issue relevance rationale: Runs an additional pytest selector under warnings-as-errors to probe behavior beyond the official rerun.
- Adjudication: `true_positive`
- Reviewer notes: Failure occurs on the same repository as the target issue and in an augmented test path; treated as issue-relevant.

### Case 4: `astropy__astropy-7671` / `gru`

- Candidate: `203e0cbc-5ece-56a8-9a2f-87c80e23913c`
- L1 verdict: `pass`
- L2 family: `L2_REGRESSION_FAIL` (`targeted_regression`)
- Failure summary: targeted_regression:regression_forced_fail exit_code=1
- Issue relevance rationale: Regression family active for protocol completeness in flagship v1.
- Adjudication: `false_positive`
- Reviewer notes: This family uses a forced non-zero command (`regression_forced_fail`), so failures indicate validator limitation rather than candidate regression.

### Case 5: `astropy__astropy-7671` / `honeycomb`

- Candidate: `28bac07d-9ede-5b1b-a50d-060a4f3852dd`
- L1 verdict: `pass`
- L2 family: `L2_REGRESSION_FAIL` (`targeted_regression`)
- Failure summary: targeted_regression:regression_forced_fail exit_code=1
- Issue relevance rationale: Regression family active for protocol completeness in flagship v1.
- Adjudication: `false_positive`
- Reviewer notes: This family uses a forced non-zero command (`regression_forced_fail`), so failures indicate validator limitation rather than candidate regression.

### Case 6: `astropy__astropy-7671` / `sweagent`

- Candidate: `111ef252-bbe6-5d6f-9330-7c2037e10b97`
- L1 verdict: `pass`
- L2 family: `L2_REGRESSION_FAIL` (`targeted_regression`)
- Failure summary: targeted_regression:regression_forced_fail exit_code=1
- Issue relevance rationale: Regression family active for protocol completeness in flagship v1.
- Adjudication: `false_positive`
- Reviewer notes: This family uses a forced non-zero command (`regression_forced_fail`), so failures indicate validator limitation rather than candidate regression.

## Integrity note

The `targeted_regression` family in flagship v1 is intentionally configured as `regression_forced_fail` (`sys.exit(1)`), so those failures are reported as validator limitation rather than candidate regression.
