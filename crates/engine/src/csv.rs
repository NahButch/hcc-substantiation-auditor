//! Minimal, dependency-free CSV reader.
//!
//! The CMS V28 internal tables are well-formed UTF-8 CSVs with optional
//! double-quoted fields and no embedded newlines. This reader handles quoting
//! and a leading UTF-8 BOM, which is all those files require. It is intentionally
//! small rather than a general RFC-4180 implementation.

use std::path::Path;

/// A parsed CSV: the header row plus data rows. All cells are trimmed of a BOM
/// only on the first cell; field whitespace is preserved (callers trim as needed,
/// mirroring the reference software which `rstrip`s specific fields).
#[derive(Debug, Clone)]
pub struct Csv {
    pub header: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl Csv {
    /// Column index for a header name (exact match), or `None`.
    pub fn col(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }
}

/// Parse a single CSV line into fields, honoring `"..."` quoting and `""` escapes.
fn parse_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    fields.push(cur);
    fields
}

/// Read and parse a CSV file. Empty lines are skipped.
pub fn read(path: impl AsRef<Path>) -> std::io::Result<Csv> {
    let text = std::fs::read_to_string(path)?;
    // Strip a leading UTF-8 BOM if present.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().map(parse_line).unwrap_or_default();
    let rows = lines.map(parse_line).collect();
    Ok(Csv { header, rows })
}
