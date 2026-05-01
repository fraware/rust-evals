# Confirming tier-3 CI on tagged releases

The submission checklist requires human-visible confirmation that
`.github/workflows/release-tag.yml` completed successfully for each pushed
release tag (historically `v0.1.0`, `v0.1.1`; NeurIPS E&D freeze `v0.1.4-neurips2026-ed`).

## Prerequisites

Install the GitHub CLI (`gh`) and authenticate:

```bash
gh auth login
```

Alternatively set `GH_TOKEN` to a classic PAT or fine-grained token with
`actions:read` on the repository.

## List recent release-tag runs

```bash
gh run list --workflow=release-tag.yml --limit 20
```

Filter to a specific tag by matching the commit subject or the `git_ref`
column shown in the web UI, or open the newest runs after pushing the tag.

## Inspect a single run

Replace `RUN_ID` with the numeric id from `gh run list`:

```bash
gh run view RUN_ID
gh run view RUN_ID --log-failed
```

A green conclusion (`completed success`) closes the checklist item for that
tag.

## Without `gh`

Open `https://github.com/<org>/<repo>/actions/workflows/release-tag.yml` in a
browser, select the workflow run triggered by the tag push, and confirm a green
status for the jobs that execute `lake build`, Rust CLI tests, schema
validation, and `write_release_artifact_manifest.py --require-all-files`.

## Public API confirmation (this repository)

For `fraware/rust-evals`, the GitHub REST API is readable without a token for
workflow metadata. Summary table for shipped tags:

| Tag | Run | Conclusion |
|-----|-----|------------|
| `v0.1.0` | [24924543926](https://github.com/fraware/rust-evals/actions/runs/24924543926) | `success` |
| `v0.1.1` | [24924758322](https://github.com/fraware/rust-evals/actions/runs/24924758322) | `success` |
| `v0.1.4-neurips2026-ed` | [25215248316](https://github.com/fraware/rust-evals/actions/runs/25215248316) | `success` |

List endpoint (same data `gh run list` would show):

`https://api.github.com/repos/fraware/rust-evals/actions/workflows/release-tag.yml/runs?per_page=10`

## v0.1.4-neurips2026-ed

- **Tag:** `v0.1.4-neurips2026-ed`
- **Commit (workflow head SHA):** `513781c0b5ed31deb01d6b0f4e1834dc2d8552e5`
- **Workflow:** `release-tag.yml`
- **Status:** success
- **Run URL:** https://github.com/fraware/rust-evals/actions/runs/25215248316
- **Checked:** 2026-05-03

Note: `main` may advance beyond this commit after later merges; the tag continues to point at the
annotated release object used for the NeurIPS engineering freeze CI run above.
