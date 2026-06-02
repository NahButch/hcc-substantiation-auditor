//! Note-text reconstruction + quote matching (ports `hcceval/notes.py`, reusing
//! the agent's FHIR reader so the eval sees exactly the note the auditor saw).

use std::collections::HashMap;
use std::path::Path;

/// Normalize for tolerant matching: lowercase, collapse all whitespace to single
/// spaces, trim.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Whether an auditor quote occurs in the note (whitespace/case-insensitive).
pub fn quote_in_note(quote: &str, note: &str) -> bool {
    let q = norm(quote);
    if q.is_empty() {
        return false;
    }
    norm(note).contains(&q)
}

/// Build patient_id → note text from a directory of Synthea FHIR bundles.
pub fn load_notes(dir: impl AsRef<Path>) -> std::io::Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.ends_with(".json") || name.contains("hospitalInformation") || name.contains("practitionerInformation") {
            continue;
        }
        if let Some(rec) = agent::fhir::parse_bundle(&path) {
            out.insert(rec.id, rec.note);
        }
    }
    Ok(out)
}
