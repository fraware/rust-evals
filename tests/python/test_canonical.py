"""Tests for the canonical-JSON helpers."""

from __future__ import annotations

from datetime import datetime, timezone

from benchmark_compat.canonical import canonical_json, canonical_json_str, sha256_hex


def test_canonical_json_sorts_keys_without_trailing_newline() -> None:
    out = canonical_json({"b": 1, "a": 2})
    assert out == b'{"a":2,"b":1}'


def test_canonical_json_is_byte_stable_across_runs() -> None:
    sample = {"x": [3, 2, 1], "y": "abc", "z": None}
    a = canonical_json(sample)
    b = canonical_json(sample)
    assert a == b


def test_canonical_json_handles_datetime_as_utc_z() -> None:
    ts = datetime(2024, 7, 1, 12, 34, 56, tzinfo=timezone.utc)
    out = canonical_json_str({"ts": ts})
    assert out == '{"ts":"2024-07-01T12:34:56Z"}'


def test_canonical_json_str_round_trips() -> None:
    s = canonical_json_str({"nested": {"k": [1, 2, 3]}})
    assert s == '{"nested":{"k":[1,2,3]}}'
    assert not s.endswith("\n")


def test_sha256_hex_is_lowercase_64_chars() -> None:
    h = sha256_hex(b"hello")
    assert len(h) == 64
    assert h == h.lower()
    assert h == "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
