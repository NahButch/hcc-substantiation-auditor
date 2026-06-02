//! Tolerant JSON extraction from LLM output.
//!
//! With Ollama's `format: "json"` the response is already a bare JSON object, but
//! this guards against models that wrap JSON in prose or code fences.

/// Return the substring spanning the first balanced top-level `{...}` object,
/// or `None` if there isn't one. Brace-counting ignores braces inside strings.
pub fn extract_object(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let start = s.find('{')?;
    let mut depth = 0usize;
    let mut in_str = false;
    let mut escaped = false;
    for i in start..bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_str = false;
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_object() {
        assert_eq!(extract_object(r#"{"a":1}"#), Some(r#"{"a":1}"#));
    }
    #[test]
    fn wrapped_in_prose_and_fences() {
        let s = "Here you go:\n```json\n{\"a\": {\"b\": 2}}\n```\nDone.";
        assert_eq!(extract_object(s), Some("{\"a\": {\"b\": 2}}"));
    }
    #[test]
    fn brace_inside_string() {
        assert_eq!(extract_object(r#"{"x":"a}b"}"#), Some(r#"{"x":"a}b"}"#));
    }
    #[test]
    fn none_when_absent() {
        assert_eq!(extract_object("no json here"), None);
    }
}
