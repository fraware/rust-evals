# Confirming tier-3 CI on tagged releases

The submission checklist requires human-visible confirmation that
`.github/workflows/release-tag.yml` completed successfully for tags such as
`v0.1.0` and `v0.1.1`.

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
workflow metadata. As of the last check, `release-tag.yml` completed
successfully for both shipped tags:

| Tag | Run | Conclusion |
|-----|-----|------------|
| `v0.1.0` | [24924543926](https://github.com/fraware/rust-evals/actions/runs/24924543926) | `success` |
| `v0.1.1` | [24924758322](https://github.com/fraware/rust-evals/actions/runs/24924758322) | `success` |

List endpoint (same data `gh run list` would show):

`https://api.github.com/repos/fraware/rust-evals/actions/workflows/release-tag.yml/runs?per_page=10`
