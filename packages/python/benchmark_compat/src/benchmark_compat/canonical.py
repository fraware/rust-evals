"""Canonical JSON serialization helpers.

The Rust workspace pins a strict canonicalization contract (sorted keys,
shortest round-trippable floats, UTF-8, ``\n`` line endings) so that
bundle hashes and analysis outputs are byte-stable across platforms.
Python-side artifacts that end up inside those bundles must follow the
same rules; otherwise a Python-emitted `BenchmarkTask` would be
bit-different from an equivalent Rust-emitted one.

This module is the single canonicalization surface for the Python
compat layer.
"""

from __future__ import annotations

import hashlib
import json
from typing import Any, Final

import orjson

__all__ = [
    "CANONICAL_JSON_ENCODING",
    "canonical_json",
    "canonical_json_str",
    "sha256_hex",
]

#: Encoding of canonical JSON output. Always UTF-8 with no BOM.
CANONICAL_JSON_ENCODING: Final[str] = "utf-8"


def canonical_json(value: Any) -> bytes:
    """Serialize ``value`` to canonical JSON bytes.

    Contract (identical to ``eval_ladder_core::canonical_json``):

    - UTF-8, no BOM.
    - Object keys sorted lexicographically.
    - No trailing newline (callers who want JSONL must append ``b"\\n"``
      themselves, exactly as Rust callers do).
    - No other trailing whitespace.
    - Integers and strings serialized in the shortest exact form.
    - Floats use Python's round-trippable :py:func:`repr` (via orjson).

    ``value`` may be any ``orjson``-compatible object (``dict``, ``list``,
    ``str``, ``int``, ``float``, ``bool``, ``None``,
    :py:class:`datetime.datetime`). For :py:class:`datetime.datetime` the
    serializer preserves ISO-8601 output with a trailing ``Z`` for UTC,
    which matches ``chrono``'s default.
    """

    body = orjson.dumps(
        value,
        option=(orjson.OPT_SORT_KEYS | orjson.OPT_UTC_Z),
    )
    return body


def canonical_json_str(value: Any) -> str:
    """Same as :func:`canonical_json` but returns a ``str``.

    Use when the consumer is a text-mode caller (pytest assertions,
    logging). File writes should always use :func:`canonical_json`
    directly so that byte-level determinism survives OS newline
    translation.
    """

    return canonical_json(value).decode(CANONICAL_JSON_ENCODING)


def sha256_hex(data: bytes) -> str:
    """Return the SHA-256 of ``data`` as a lowercase hex string.

    The matching Rust digest format is ``sha256:<64-hex>`` (see
    :class:`eval_ladder_core::Sha256Digest`). This helper returns only the
    hex, so callers that need the prefixed digest should format
    ``f"sha256:{sha256_hex(data)}"`` explicitly.
    """

    return hashlib.sha256(data).hexdigest()


# Sanity guard: orjson's behavior must match the invariants above. We
# verify at import time because a regression here is catastrophic - a
# bundle emitted against the wrong canonicalization silently diverges
# from Rust without any loud failure.
def _self_check() -> None:
    sample = {"b": 1, "a": [3, 2, 1]}
    out = canonical_json(sample)
    expected = b'{"a":[3,2,1],"b":1}'
    if out != expected:  # pragma: no cover - defensive
        raise RuntimeError(
            f"canonical_json invariant broken: got {out!r}, expected {expected!r}"
        )
    # Negative control: the stdlib `json` default order would differ
    # without sort_keys; we guard against the stdlib being accidentally
    # substituted.
    if json.dumps(sample).encode() == out:  # pragma: no cover - defensive
        raise RuntimeError("canonical_json must not equal default json.dumps output")


_self_check()
