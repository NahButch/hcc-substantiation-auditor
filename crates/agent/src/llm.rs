//! Thin, swappable LLM provider interface.
//!
//! The agent depends only on the [`LlmProvider`] trait, so the backing model can
//! be swapped (local Ollama, a hosted API, or a deterministic mock for tests)
//! without touching reasoning logic. The default is a local Ollama model — no API
//! key, fully offline.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug)]
pub enum LlmError {
    Transport(String),
    BadResponse(String),
    Exhausted,
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::Transport(e) => write!(f, "LLM transport error: {e}"),
            LlmError::BadResponse(e) => write!(f, "LLM bad response: {e}"),
            LlmError::Exhausted => write!(f, "mock provider has no more scripted responses"),
        }
    }
}
impl std::error::Error for LlmError {}

/// A single chat request: a system instruction and a user message.
pub struct Prompt<'a> {
    pub system: &'a str,
    pub user: &'a str,
    /// Request strict JSON output when the backend supports it.
    pub json: bool,
}

/// Backend-agnostic LLM interface. One method: prompt in, completion text out.
pub trait LlmProvider {
    fn complete(&self, prompt: &Prompt) -> Result<String, LlmError>;
    /// Identifier for logging / provenance (e.g. `"ollama:qwen2.5:7b-instruct"`).
    fn id(&self) -> String;
}

/// Local Ollama provider — blocking HTTP to the Ollama `/api/chat` endpoint.
pub struct OllamaProvider {
    pub base_url: String,
    pub model: String,
    pub timeout: Duration,
    /// Sampling temperature; 0.0 for maximum determinism.
    pub temperature: f64,
}

impl OllamaProvider {
    pub fn new(model: impl Into<String>) -> Self {
        let base_url = std::env::var("OLLAMA_HOST_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
        OllamaProvider { base_url, model: model.into(), timeout: Duration::from_secs(180), temperature: 0.0 }
    }
}

impl LlmProvider for OllamaProvider {
    fn complete(&self, prompt: &Prompt) -> Result<String, LlmError> {
        let mut body = serde_json::json!({
            "model": self.model,
            "stream": false,
            "options": { "temperature": self.temperature },
            "messages": [
                { "role": "system", "content": prompt.system },
                { "role": "user", "content": prompt.user },
            ],
        });
        if prompt.json {
            body["format"] = serde_json::json!("json");
        }

        let resp = ureq::post(&format!("{}/api/chat", self.base_url))
            .timeout(self.timeout)
            .send_json(body)
            .map_err(|e| LlmError::Transport(e.to_string()))?;
        let v: serde_json::Value =
            resp.into_json().map_err(|e| LlmError::BadResponse(e.to_string()))?;
        v["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LlmError::BadResponse(format!("no message.content in {v}")))
    }

    fn id(&self) -> String {
        format!("ollama:{}", self.model)
    }
}

/// Deterministic mock provider for tests and the eval harness — returns scripted
/// responses in order, so no live model is needed and output is reproducible.
pub struct MockProvider {
    responses: RefCell<VecDeque<String>>,
}

impl MockProvider {
    pub fn new(responses: Vec<String>) -> Self {
        MockProvider { responses: RefCell::new(responses.into()) }
    }
    /// Always return the same single response.
    pub fn always(response: impl Into<String>) -> Self {
        let mut q = VecDeque::new();
        q.push_back(response.into());
        // Re-queue on each call by cloning in `complete`; see below.
        MockProvider { responses: RefCell::new(q) }
    }
}

impl LlmProvider for MockProvider {
    fn complete(&self, _prompt: &Prompt) -> Result<String, LlmError> {
        let mut q = self.responses.borrow_mut();
        if q.len() == 1 {
            // Single-response mock: keep returning it.
            return Ok(q.front().unwrap().clone());
        }
        q.pop_front().ok_or(LlmError::Exhausted)
    }
    fn id(&self) -> String {
        "mock".to_string()
    }
}
