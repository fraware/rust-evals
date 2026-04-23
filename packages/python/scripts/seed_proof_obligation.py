"""Idempotently seed the first production proof obligation.

This script owns the canonical JSON payload for the
`clap-rs__clap_5873` obligation so reviewers can audit the record in
one place. Running it:

1. Validates the payload against
   `schemas/proof_obligation.schema.json`.
2. Appends the payload as a single line of canonical JSON
   (sorted keys, compact separators) to
   `datasets/derived/proof_subset/manifest.jsonl`.
3. Refuses to append duplicates: reruns are no-ops.

Usage (run from repository root):

    python packages/python/scripts/seed_proof_obligation.py [--dry-run]

The payload is intentionally the only obligation seeded here; future
obligations earn their own seed script or land via a direct
`manifest.jsonl` edit with a corresponding Lean module.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    import jsonschema
except ImportError as exc:
    raise SystemExit("jsonschema is required; `pip install jsonschema`") from exc


REPO_ROOT = Path(__file__).resolve().parents[3]
SCHEMA = REPO_ROOT / "schemas" / "proof_obligation.schema.json"
MANIFEST = REPO_ROOT / "datasets" / "derived" / "proof_subset" / "manifest.jsonl"


OBLIGATION: dict = {
    "schema_version": 1,
    "obligation_id": "obl.rust_swe_bench.clap_rs.clap_5873.ignore_errors_recovery_identity",
    "task_id": "clap-rs__clap_5873",
    "property_name": "ignore_errors_recovery_is_identity",
    "property_type": "state_machine_safety",
    "target_files": ["clap_builder/src/parser/parser.rs"],
    "informal_statement": (
        "When ignore_errors(true) is set, the did-you-mean recovery branch of "
        "Parser::parse_long_arg must leave the ArgMatcher unchanged; in "
        "particular an argument whose ValueSource was DefaultValue must "
        "remain DefaultValue, preventing the spurious CommandLine transition "
        "that caused the issue."
    ),
    # Paths are resolved relative to `lean_root`, which the runner
    # points at `packages/lean/EvalLadder/`. The Lake project uses
    # srcDir = "." with root module `EvalLadder`, so obligation
    # modules live directly under `Obligations/` (not under a nested
    # `EvalLadder/Obligations/`). Using the flat layout keeps
    # `lake env lean` consistent with the on-disk file layout across
    # CI hosts.
    "formal_statement_ref": "Obligations/ClapRs/Clap5873.lean",
    "proof_checker": {
        "command": "python",
        "args": [
            "scripts/check_obligation.py",
            "Obligations/ClapRs/Clap5873.lean",
            "L4_OBLIGATION_MET",
        ],
    },
    "pass_criterion": "L4_OBLIGATION_MET",
    "difficulty": {"reviewer_hours": 1.5},
    "selection_rationale": {
        "one_or_two_sentence_property": True,
        "local_scope": True,
        "matters_to_issue": True,
        "strictly_stronger_than_tests": True,
        "bounded_effort": True,
    },
    "witness_inputs": [],
    "expected_touched_symbols": [
        "clap_builder::parser::parser::Parser::did_you_mean_error",
        "clap_builder::builder::command::Command::is_ignore_errors_set",
    ],
}


def canonical_line(obj: dict) -> str:
    return json.dumps(obj, sort_keys=True, separators=(",", ":"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dry-run", action="store_true", help="Validate only; do not write.")
    args = parser.parse_args()

    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    jsonschema.validate(OBLIGATION, schema)

    line = canonical_line(OBLIGATION)
    if not MANIFEST.parent.exists():
        MANIFEST.parent.mkdir(parents=True, exist_ok=True)

    existing = MANIFEST.read_text(encoding="utf-8") if MANIFEST.exists() else ""
    if any(ln.strip() == line for ln in existing.splitlines()):
        print(f"obligation already present: {OBLIGATION['obligation_id']}")
        return 0

    if args.dry_run:
        print("dry-run: schema OK; would append:")
        print(line)
        return 0

    with MANIFEST.open("a", encoding="utf-8", newline="\n") as fh:
        if existing and not existing.endswith("\n"):
            fh.write("\n")
        fh.write(line + "\n")
    print(f"appended obligation: {OBLIGATION['obligation_id']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
