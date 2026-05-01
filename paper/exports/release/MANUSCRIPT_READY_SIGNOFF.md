# Manuscript-ready signoff

Date: 2026-05-03  
Tag: `v0.1.4-neurips2026-ed` (engineering freeze; CI recorded in `docs/github_release_tag_ci_confirmation.md`)

Identifying commit: run `git log -1 --pretty=format:%H -- paper/exports/release/MANUSCRIPT_READY_SIGNOFF.md` from a checkout that contains this file (the hash is the commit object that adds or updates this sign-off).

## Central empirical surfaces

- [x] Live v2 static-vs-live diagnostic verified.
- [x] L2 flagship diagnostic verified.
- [x] L2 arms interpreted separately.
- [x] Regression stress-control arm labeled protocol-control, not natural regression.

## Evidence-frontier surfaces

- [x] Verified feasibility reported as inventory-bound frontier.
- [x] Rust proof subset reported as implemented extension/frontier, not semantic-failure-rate evidence.

## Validation gates

- [x] Build, format, clippy, tests.
- [x] Python lint/typecheck.
- [x] Schema validation.
- [x] Demo run.
- [x] Live gate.
- [x] L2 gate.
- [x] Rust proof release gate.
- [x] Verified feasibility report.
- [x] Gold validation closed.
- [x] Claim-source check.
- [x] Secret scan.
- [x] Release tag green.

## Documentation consistency

- [x] `docs/scientific_scope.md` matches `CLAIM_LOCK_NEURIPS2026.md`.
- [x] `docs/l2_failure_case_studies.md` contains no natural-regression claim for forced-fail rows.
- [x] `docs/submission_checklist.md` no longer references stale `rust_pilot_v1` as paper evidence.
- [x] Evaluator Cards exist for all paper surfaces.
- [x] README reviewer path is up to date.

## Go/no-go

Status: GO for manuscript writing.
