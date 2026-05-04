#!/usr/bin/env python3
"""Extract an anonymous ``.tar.gz`` and run NeurIPS-style scrub checks on the tree.

Portable replacement for::

    mkdir /tmp/eval_ladder_review_check
    tar -xzf <bundle>.tar.gz -C /tmp/eval_ladder_review_check
    grep -RInE "fraware|..." /tmp/eval_ladder_review_check
    find ... .git .github

Exit code ``1`` if identity/path/email/social patterns match or forbidden dirs
exist.

Usage::

    python ci/scripts/verify_anonymous_bundle_scrub.py build/eval-ladder-anonymous-bundle.tar.gz
"""

from __future__ import annotations

import argparse
import shutil
import sys
import tarfile
import tempfile
from pathlib import Path

from anonymization_scrub_lib import collect_violations


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("archive", type=Path, help="Path to anonymous .tar.gz")
    p.add_argument(
        "--keep",
        action="store_true",
        help="Do not delete the temporary extract directory (print its path).",
    )
    args = p.parse_args()
    archive = args.archive.resolve()
    if not archive.is_file():
        print(f"verify_anonymous_bundle_scrub: missing {archive}", file=sys.stderr)
        return 1

    work = Path(tempfile.mkdtemp(prefix="eval_ladder_review_check_"))

    try:
        with tarfile.open(archive, "r:gz") as tf:
            try:
                tf.extractall(work, filter="data")  # type: ignore[call-arg]
            except TypeError:
                tf.extractall(work)

        children = [c for c in work.iterdir()]
        root = children[0] if len(children) == 1 and children[0].is_dir() else work

        failures = collect_violations(root)
        if failures:
            print("verify_anonymous_bundle_scrub: FAIL", file=sys.stderr)
            for line in failures[:300]:
                print(line, file=sys.stderr)
            if len(failures) > 300:
                print(f"... and {len(failures) - 300} more", file=sys.stderr)
            return 1

        print("verify_anonymous_bundle_scrub: OK", file=sys.stderr)
        if args.keep:
            print(f"extracted under: {work}", file=sys.stderr)
        return 0
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
