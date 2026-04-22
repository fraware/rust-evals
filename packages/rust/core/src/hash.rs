//! Canonical JSON serialization and SHA-256 digests.
//!
//! The integrity of evidence bundles and trace hash chains rests on the
//! canonical JSON function. The rules are exactly as documented in
//! `docs/artifact_spec.md`:
//!
//! - UTF-8 encoding, no BOM.
//! - Unix newlines (`\n`).
//! - Object keys sorted lexicographically.
//! - No trailing whitespace.
//! - Numbers serialized in their `serde_json` shortest round-trippable form.
//!
//! These rules are implemented by recursively walking a `serde_json::Value`
//! and re-serializing object keys through a `BTreeMap`. Array order is
//! preserved (arrays are ordered in JSON).

use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::CoreError;

/// Canonicalize a JSON value and return the UTF-8 bytes.
///
/// The output is a single line of JSON with no trailing newline, so callers
/// that wish to emit JSONL should append `\n` themselves.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, CoreError> {
    let v = serde_json::to_value(value)?;
    let mut out = Vec::with_capacity(128);
    write_canonical(&v, &mut out)?;
    Ok(out)
}

fn write_canonical(v: &serde_json::Value, out: &mut Vec<u8>) -> Result<(), CoreError> {
    match v {
        serde_json::Value::Null => out.extend_from_slice(b"null"),
        serde_json::Value::Bool(b) => out.extend_from_slice(if *b { b"true" } else { b"false" }),
        serde_json::Value::Number(n) => out.extend_from_slice(n.to_string().as_bytes()),
        serde_json::Value::String(s) => {
            // Delegate string escaping to serde_json so quoting rules match
            // the rest of the ecosystem.
            let encoded = serde_json::to_string(s)?;
            out.extend_from_slice(encoded.as_bytes());
        }
        serde_json::Value::Array(items) => {
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                write_canonical(item, out)?;
            }
            out.push(b']');
        }
        serde_json::Value::Object(map) => {
            // Sort keys lexicographically for canonical form.
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_unstable();
            out.push(b'{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                let encoded_key = serde_json::to_string(k)?;
                out.extend_from_slice(encoded_key.as_bytes());
                out.push(b':');
                // `map` is a Map and contains this key by construction.
                write_canonical(&map[k], out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

/// SHA-256 digest formatted as `sha256:<64-hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// Construct from raw 32-byte digest.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(format!("sha256:{}", hex::encode(bytes)))
    }

    /// Returns the `sha256:<hex>` form.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Compute the SHA-256 digest of arbitrary bytes and return the
/// canonical `sha256:<hex>` form.
#[must_use]
pub fn digest(bytes: &[u8]) -> Sha256Digest {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out: [u8; 32] = hasher.finalize().into();
    Sha256Digest::from_bytes(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_sorts_object_keys() {
        let v = serde_json::json!({
            "b": 2,
            "a": 1,
            "c": { "z": 1, "y": 2 },
        });
        let bytes = canonical_json(&v).unwrap();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(s, r#"{"a":1,"b":2,"c":{"y":2,"z":1}}"#);
    }

    #[test]
    fn canonical_preserves_array_order() {
        let v = serde_json::json!([3, 1, 2]);
        let bytes = canonical_json(&v).unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "[3,1,2]");
    }

    #[test]
    fn digest_of_empty_input_is_known() {
        let d = digest(b"");
        assert_eq!(
            d.as_str(),
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn digest_of_canonical_json_is_deterministic_over_key_order() {
        let a = serde_json::json!({ "a": 1, "b": 2 });
        let b = serde_json::json!({ "b": 2, "a": 1 });
        assert_eq!(
            digest(&canonical_json(&a).unwrap()),
            digest(&canonical_json(&b).unwrap()),
        );
    }
}
