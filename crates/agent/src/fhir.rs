//! Minimal Synthea FHIR R4 bundle reader.
//!
//! Pulls just what the auditor needs from a patient bundle: demographics, the
//! SNOMED-coded conditions, and the clinical-note text (US Core
//! `DocumentReference` attachments, base64-decoded).

use base64::Engine as _;
use engine::demographics::Date;
use engine::Sex;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Condition {
    pub snomed: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct PatientRecord {
    pub id: String,
    pub name: String,
    pub date_of_birth: Date,
    pub sex: Sex,
    pub conditions: Vec<Condition>,
    /// Concatenated clinical-note text (most recent encounters first), capped.
    pub note: String,
}

const NOTE_CAP: usize = 12_000;

/// Parse a Synthea patient FHIR bundle. Returns `None` for non-patient bundles
/// (hospital/practitioner info files) or unparseable input.
pub fn parse_bundle(path: impl AsRef<Path>) -> Option<PatientRecord> {
    let text = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entries = v.get("entry")?.as_array()?;

    let mut id = String::new();
    let mut name = String::new();
    let mut dob = None;
    let mut sex = None;
    let mut conditions = Vec::new();
    let mut notes: Vec<(String, String)> = Vec::new(); // (date, text)

    for e in entries {
        let r = match e.get("resource") {
            Some(r) => r,
            None => continue,
        };
        match r.get("resourceType").and_then(|t| t.as_str()) {
            Some("Patient") => {
                id = r.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                dob = r.get("birthDate").and_then(|x| x.as_str()).and_then(parse_date);
                sex = match r.get("gender").and_then(|x| x.as_str()) {
                    Some("male") => Some(Sex::Male),
                    Some("female") => Some(Sex::Female),
                    _ => None,
                };
                if let Some(n) = r.get("name").and_then(|n| n.as_array()).and_then(|a| a.first()) {
                    let given = n
                        .get("given")
                        .and_then(|g| g.as_array())
                        .and_then(|a| a.first())
                        .and_then(|s| s.as_str())
                        .unwrap_or("");
                    let family = n.get("family").and_then(|s| s.as_str()).unwrap_or("");
                    name = format!("{given} {family}").trim().to_string();
                }
            }
            Some("Condition") => {
                if let Some(coding) = r
                    .get("code")
                    .and_then(|c| c.get("coding"))
                    .and_then(|c| c.as_array())
                {
                    for c in coding {
                        if c.get("system").and_then(|s| s.as_str()) == Some("http://snomed.info/sct")
                        {
                            conditions.push(Condition {
                                snomed: c.get("code").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                                description: c
                                    .get("display")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                            });
                        }
                    }
                }
            }
            Some("DocumentReference") => {
                let date = r.get("date").and_then(|x| x.as_str()).unwrap_or("").to_string();
                if let Some(content) = r.get("content").and_then(|c| c.as_array()) {
                    for c in content {
                        if let Some(data) =
                            c.get("attachment").and_then(|a| a.get("data")).and_then(|d| d.as_str())
                        {
                            if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) {
                                if let Ok(txt) = String::from_utf8(bytes) {
                                    notes.push((date.clone(), txt));
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let (id, dob, sex) = (
        (!id.is_empty()).then_some(id)?,
        dob?,
        sex?,
    );

    // Most-recent notes first, concatenated and capped.
    notes.sort_by(|a, b| b.0.cmp(&a.0));
    let mut note = String::new();
    for (_, txt) in &notes {
        if note.len() >= NOTE_CAP {
            break;
        }
        if !note.is_empty() {
            note.push_str("\n\n---\n\n");
        }
        note.push_str(txt);
    }
    note.truncate(NOTE_CAP);

    Some(PatientRecord { id, name, date_of_birth: dob, sex, conditions, note })
}

/// Parse an ISO `YYYY-MM-DD` date.
fn parse_date(s: &str) -> Option<Date> {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 {
        return None;
    }
    Some(Date::new(p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?))
}
