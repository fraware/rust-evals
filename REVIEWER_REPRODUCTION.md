# Reviewer reproduction (Eval-Ladder)

This document is the canonical reviewer path companion to `README.md` and
`paper/exports/CLAIM_SOURCE_MAP.md`. Numerical manuscript claims are pinned to
**sealed exports** under `paper/exports/`; full benchmark replay is optional and
heavy.

On Windows, use `target\\release\\eval-ladder.exe` instead of `eval-ladder`
where shown, or invoke via `cargo run --release --bin eval-ladder --`.

**Optional anonymized bundle:** if you build a venue-specific anonymized archive
with `ci/scripts/build_anonymous_submission_bundle.py`, record the final SHA-256
**outside** the tarball (for example in `build/RELEASE_MANIFEST.md` and
`build/ANON_BUNDLE_SHA256.txt`, which live under `build/` and are normally
gitignored). Do **not** embed the tarball digest inside files that are packed
inside the same tarball.

**Sanity checks on a packaged archive:** run
`python ci/scripts/verify_anonymous_bundle_scrub.py` on the final `.tar.gz`, and
`python ci/scripts/check_reviewer_false_success_language.py --archive <path-to-archive.tar.gz>`
(or `--staged-root <path-to-staged-tree>` before tarring).

---

## Fast sealed-table reproduction

**Purpose:** Regenerate all headline paper exports and TeX tables from sealed
`runs/released/**` inputs without network fetch.

| | |
|---|---|
| **Estimated runtime** | About 2–8 minutes on a typical laptop after Rust release build (dominated by `cargo build --release` once). |
| **Requires network** | No (given `runs/released/**` and `datasets/derived/proof_subset/manifest.jsonl` already present). |
| **Requires Docker** | No. |
| **Expected outputs** | `paper/exports/live_panel_v2_postbatch/*`, `paper/exports/l2_verified_flagship_v1/*`, `paper/exports/strict_feasibility_report.json`, `paper/exports/rust_proof_subset_v1_seal_release/*`, `paper/tables/*.tex`, `paper/exports/reproduction_manifest.json`. |

Commands:

```bash
python -m pip install -e ".[dev]"
cargo build --workspace
cargo run --bin eval-ladder -- schema validate
python packages/python/scripts/reproduce_paper_tables.py
python ci/scripts/check_paper_claim_sources.py
python ci/scripts/check_claim_limits.py
python ci/scripts/check_l2_arm_separation.py --export-dir paper/exports/l2_verified_flagship_v1
python ci/scripts/check_evidence_quality.py live --paper-export-dir paper/exports/live_panel_v2_postbatch
python ci/scripts/check_evidence_quality.py l2 --run-dir runs/released/l2_verified_flagship_v1/results
python ci/scripts/analyze_strict_feasibility.py
python ci/scripts/check_evidence_quality.py --gate-profile release rust-proof --run-dir runs/released/rust_proof_subset_v1/results_seal
```

**Known failure modes:** missing `runs/released/**` directories; missing
`gold_patch_validation.csv` before L2 export; `target/release/eval-ladder` not
built; Python dev extras not installed (YAML mirror check in
`check_paper_claim_sources`).

---

## Artifact integrity verification

**Purpose:** Cryptographically verify sealed per-candidate bundles for the
headline cohorts.

| | |
|---|---|
| **Estimated runtime** | About 1–5 minutes depending on disk (reads bundle contents). |
| **Requires network** | No. |
| **Requires Docker** | No. |
| **Expected outputs** | `verify_report.json` under each verified run directory (CLI writes next to bundles). |

Commands:

```bash
target/release/eval-ladder verify run-dir --run-dir runs/released/live_panel_v2/results_opt
target/release/eval-ladder verify run-dir --run-dir runs/released/l2_verified_flagship_v1/results_astropy
target/release/eval-ladder verify run-dir --run-dir runs/released/l2_verified_flagship_v1/results_regression_fail
target/release/eval-ladder verify run-dir --run-dir runs/released/rust_proof_subset_v1/results_seal
```

**Important (L2):** The merged flagship `results/` tree contains joined
`batch_summary.json` but not every per-bundle leaf. Per-arm verification is the
integrity path cited by `CLAIM_SOURCE_MAP.md`; the merged tree is still used for
paper-export regeneration.

**Known failure modes:** pointing `verify` at the merged L2 `results/` directory
alone may not traverse all bundle leaves expected for arm-level checks.

---

## Full benchmark replay (optional / heavy)

**Purpose:** Re-execute benchmark harnesses and containers for tasks in the
panels.

| | |
|---|---|
| **Estimated runtime** | Hours to days depending on panel, cache, and parallelism; not bounded here. |
| **Requires network** | Yes (dataset fetch, container pulls, upstream mirrors unless fully cached). |
| **Requires Docker** | Yes for standard SWE-bench-style harness paths. |

Full replay requires benchmark assets, containerized environments, and
benchmark-specific dependencies. The manuscript’s numerical claims are reproduced
from **sealed artifacts** and deterministic export scripts, not from
network-dependent replay alone.

**Known failure modes:** rate limits on upstream hosts; Docker credential
issues; host-specific path mounts.

---

## Demo (smoke, not headline claims)

```bash
cargo run --bin eval-ladder -- demo run --out runs/demo --tasks 2
```

This exercises the runner pipeline on a tiny fixture set; it does not reproduce
headline Live v2 / L2 flagship numbers.
