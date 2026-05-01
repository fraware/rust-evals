# NeurIPS 2026 E&D release pointer

This file records the **engineering release tag** used for the evidence-and-discipline
submission freeze, independent of manuscript authoring.

## Tag

- **`v0.1.4-neurips2026-ed`** — annotated tag used for the engineering freeze and
  `release-tag.yml` confirmation (see `docs/github_release_tag_ci_confirmation.md`).

The tag points at a specific commit object on GitHub; **`main` may advance** after later
merges. For bit-for-bit parity with the CI-validated tree, check out the tag:

```bash
git fetch origin tag v0.1.4-neurips2026-ed
git checkout v0.1.4-neurips2026-ed
```

After pushing the tag, confirm **`release-tag.yml`** is green on that ref (same practice
as documented in `docs/submission_checklist.md` and
`docs/github_release_tag_ci_confirmation.md` for earlier tags).

## Closure artifacts

- Gate log: `paper/exports/release/final_validation_matrix.md`
- Gold validation closure: `paper/exports/release/gold_validation_export_only_log.md`
- Manuscript-ready sign-off: `paper/exports/release/MANUSCRIPT_READY_SIGNOFF.md`
- Final repro commands: `artifacts/final_repro_log.md`

## Anonymity / packaging (plan §15)

NeurIPS E&D is **single-blind**. Whether to ship an anonymized code bundle or rely on
the public repo is a **policy decision for authors**, not something engineering can
finalize here.
