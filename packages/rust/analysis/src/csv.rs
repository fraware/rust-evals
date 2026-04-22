//! Minimal, dependency-free CSV writer for analysis tables.
//!
//! The analysis crate deliberately avoids a `csv` dependency so the workspace
//! stays dependency-light at the leaves. This writer implements RFC 4180
//! conservatively: every field is quoted and internal quotes are doubled.

use std::io::{self, Write};

/// Write a single CSV row.
pub fn write_row(w: &mut impl Write, fields: &[&str]) -> io::Result<()> {
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            w.write_all(b",")?;
        }
        write_quoted_field(w, f)?;
    }
    w.write_all(b"\n")
}

/// Write a header row and then one row per item. `to_fields` converts each
/// item into a `Vec<String>`; strings are borrowed as `&str` for writing.
pub fn write_table<T>(
    w: &mut impl Write,
    header: &[&str],
    items: &[T],
    to_fields: impl Fn(&T) -> Vec<String>,
) -> io::Result<()> {
    write_row(w, header)?;
    for item in items {
        let fields = to_fields(item);
        let refs: Vec<&str> = fields.iter().map(String::as_str).collect();
        write_row(w, &refs)?;
    }
    Ok(())
}

fn write_quoted_field(w: &mut impl Write, f: &str) -> io::Result<()> {
    w.write_all(b"\"")?;
    for byte in f.as_bytes() {
        if *byte == b'"' {
            w.write_all(b"\"\"")?;
        } else {
            w.write_all(std::slice::from_ref(byte))?;
        }
    }
    w.write_all(b"\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_embedded_quotes() {
        let mut buf = Vec::new();
        write_row(&mut buf, &["a", "b\"c", "d,e"]).unwrap();
        assert_eq!(
            std::str::from_utf8(&buf).unwrap(),
            "\"a\",\"b\"\"c\",\"d,e\"\n"
        );
    }
}
