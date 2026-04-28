# CLI reference

Core `eval-ladder` commands used in public release workflows.

## Ingest

```bash
eval-ladder ingest verified --manifest configs/evaluator/verified.toml
eval-ladder ingest live --manifest configs/evaluator/live.toml
eval-ladder ingest rust --manifest configs/evaluator/rust.toml
```

## Evaluate

```bash
eval-ladder evaluate candidate --candidate <candidate.json> --levels L0,L1 --config <config.toml>
eval-ladder evaluate batch --input <panel.jsonl> --levels L0,L1,L2 --config <config.toml> --out <run_dir>
```

Common flags:

- `--resume`
- `--jobs <n>`
- `--adaptive-timeouts`
- `--deterministic-clock`

## Verify

```bash
eval-ladder verify run-dir --run-dir <run_dir>
```

## Analyze

```bash
eval-ladder analyze score-descent --run-dir <run_dir>
eval-ladder analyze static-vs-live --run-dir <run_dir>
eval-ladder analyze taxonomy --run-dir <run_dir>
eval-ladder analyze paper-export --run-dir <run_dir> --out-dir <export_dir>
```

## Demo

```bash
eval-ladder demo run --out runs/demo --tasks 2
```

## Release checks

```bash
python ci/scripts/check_evidence_quality.py verified --run-dir <run_dir>
python ci/scripts/check_evidence_quality.py live --paper-export-dir <export_dir>
python ci/scripts/check_evidence_quality.py l2 --run-dir <run_dir>
python ci/scripts/check_evidence_quality.py rust-proof --run-dir <run_dir>
```
