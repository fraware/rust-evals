# NeurIPS 2026 E&D release pointer

This file records the **engineering release tag** used for the evidence-and-discipline
submission freeze, independent of manuscript authoring.

## Tag

- **`v0.1.4-neurips2026-ed`** — annotated lightweight tag at the commit that carries
  closed validation matrix rows and checklist updates for NeurIPS 2026 E&D.

After pushing the tag, confirm **`release-tag.yml`** is green on that ref (same practice
as documented in `docs/submission_checklist.md` and
`docs/github_release_tag_ci_confirmation.md` for earlier tags).

## Closure artifacts

- Gate log: `paper/exports/release/final_validation_matrix.md`
- Final repro commands: `artifacts/final_repro_log.md`

## Anonymity / packaging (plan §15)

NeurIPS E&D is **single-blind**. Whether to ship an anonymized code bundle or rely on
the public repo is a **policy decision for authors**, not something engineering can
finalize here.
