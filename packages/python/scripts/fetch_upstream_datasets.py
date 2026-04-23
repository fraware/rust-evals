"""Download upstream benchmark releases into datasets/cache/*/*.jsonl.

This script materializes the public benchmark releases that feed
``eval-ladder ingest``. All outputs are written as line-delimited JSON
that preserves upstream column names; the permissive
``RawSweBenchRecord`` in the Rust benchmarks crate is tolerant of new
columns, so the mirrors stay forward-compatible with upstream releases.

Sources (canonical, public):
    * Verified: https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified
    * Live:     https://huggingface.co/datasets/SWE-bench-Live/SWE-bench-Live
    * Rust:     https://huggingface.co/datasets/r1v3r/multi_SWE_Bench_Rust

The Rust slice on Hugging Face is Multi-SWE-Bench's Rust split (239 tasks
at the time of writing). We use it as the Rust-language benchmark for
the pilot; provenance is recorded in ``datasets/public_links/rust_sources.json``.

Usage:
    python packages/python/scripts/fetch_upstream_datasets.py --which verified
    python packages/python/scripts/fetch_upstream_datasets.py --which live
    python packages/python/scripts/fetch_upstream_datasets.py --which rust
    python packages/python/scripts/fetch_upstream_datasets.py --which all
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import sys
from pathlib import Path
from typing import Any, Iterable

import pyarrow.parquet as pq  # type: ignore[import-not-found]
from huggingface_hub import hf_hub_download  # type: ignore[import-not-found]

REPO_ROOT = Path(__file__).resolve().parents[3]
CACHE_DIR = REPO_ROOT / "datasets" / "cache"


def _json_default(value: Any) -> Any:
    """JSON fallback for HF-loaded datetimes and other non-JSON scalars."""
    if isinstance(value, (_dt.datetime, _dt.date, _dt.time)):
        # RFC 3339; upstream Live datetimes are timezone-aware UTC.
        return value.isoformat()
    if isinstance(value, (bytes, bytearray)):
        return value.decode("utf-8", errors="replace")
    raise TypeError(f"Object of type {type(value).__name__} is not JSON serializable")


def _write_jsonl(records: Iterable[dict], out_path: Path) -> int:
    """Write ``records`` as UTF-8 JSONL; return count."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with out_path.open("w", encoding="utf-8", newline="\n") as fh:
        for rec in records:
            fh.write(json.dumps(rec, ensure_ascii=False, sort_keys=True, default=_json_default))
            fh.write("\n")
            count += 1
    return count


def _parquet_to_records(path: Path) -> Iterable[dict]:
    table = pq.read_table(str(path))
    for batch in table.to_batches():
        for row in batch.to_pylist():
            yield row


def fetch_verified() -> Path:
    out = CACHE_DIR / "verified" / "swe_bench_verified.jsonl"
    pq_path = Path(
        hf_hub_download(
            repo_id="princeton-nlp/SWE-bench_Verified",
            filename="data/test-00000-of-00001.parquet",
            repo_type="dataset",
        )
    )
    n = _write_jsonl(_parquet_to_records(pq_path), out)
    print(f"[verified] wrote {n} records -> {out.relative_to(REPO_ROOT)}")
    return out


def _live_image_name(instance_id: str) -> str:
    """SWE-bench-Live instance-level Docker image name (Linux x86_64).

    Formula copied verbatim from microsoft/SWE-bench-Live README so the
    derived name is auditable against upstream. The Live HF dataset does
    not embed this field; it is deterministically derived from the
    instance id at ingest time.
    """
    name = instance_id.replace("__", "_1776_").lower()
    return f"starryzhang/sweb.eval.x86_64.{name}"


def fetch_live() -> Path:
    """Download the ``verified`` split of SWE-bench-Live.

    The HF dataset does not embed a ``docker_image`` column, but upstream
    publishes a deterministic DockerHub image per instance. We synthesize
    the field at mirror time so Rust-side ingest can honor its "Live
    records must carry an image reference" invariant without runner-time
    guesswork.
    """
    from datasets import load_dataset  # type: ignore[import-not-found]

    ds = load_dataset("SWE-bench-Live/SWE-bench-Live", split="verified")

    def _enriched() -> Iterable[dict]:
        for row in ds:
            rec = dict(row)
            rec.setdefault("docker_image", _live_image_name(rec["instance_id"]))
            created = rec.get("created_at")
            if isinstance(created, _dt.datetime):
                if created.tzinfo is None:
                    created = created.replace(tzinfo=_dt.timezone.utc)
                rec["created_at"] = created.isoformat().replace("+00:00", "Z")
            elif isinstance(created, str) and created and not created.endswith("Z") and "+" not in created:
                rec["created_at"] = f"{created}Z"
            yield rec

    out = CACHE_DIR / "live" / "swe_bench_live_verified.jsonl"
    n = _write_jsonl(_enriched(), out)
    print(f"[live] wrote {n} records -> {out.relative_to(REPO_ROOT)}")
    return out


def fetch_rust() -> Path:
    out = CACHE_DIR / "rust" / "multi_swe_bench_rust.jsonl"
    src = Path(
        hf_hub_download(
            repo_id="r1v3r/multi_SWE_Bench_Rust",
            filename="all_instances.jsonl",
            repo_type="dataset",
        )
    )
    records: list[dict] = []
    with src.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            records.append(json.loads(line))
    n = _write_jsonl(records, out)
    print(f"[rust] wrote {n} records -> {out.relative_to(REPO_ROOT)}")
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--which",
        choices=("verified", "live", "rust", "all"),
        required=True,
        help="Which benchmark mirror to refresh.",
    )
    args = ap.parse_args()

    targets = {
        "verified": fetch_verified,
        "live": fetch_live,
        "rust": fetch_rust,
    }
    if args.which == "all":
        for fn in targets.values():
            fn()
    else:
        targets[args.which]()
    return 0


if __name__ == "__main__":
    sys.exit(main())
