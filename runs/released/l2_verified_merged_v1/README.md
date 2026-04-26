# l2_verified_merged_v1

Merged, deduplicated L2 `batch_summary.json` slices from:

- `runs/released/l2_verified_v2/results/batch_summary.json`
- `runs/released/l2_verified_astropy_v1/results/batch_summary.json`
- `runs/released/l2_verified_xarray_v1/results/batch_summary.json`

Produced with:

```bash
python ci/scripts/merge_l2_batch_summaries.py \
  --inputs \
    runs/released/l2_verified_v2/results/batch_summary.json \
    runs/released/l2_verified_astropy_v1/results/batch_summary.json \
    runs/released/l2_verified_xarray_v1/results/batch_summary.json \
  --out-dir runs/released/l2_verified_merged_v1/results
```

Gate (release profile — see `docs/evidence_empirical_status.md`):

```bash
python ci/scripts/check_evidence_quality.py --gate-profile release l2 \
  --run-dir runs/released/l2_verified_merged_v1/results
```

Paper export: `paper/exports/l2_verified_merged_v1/`.
