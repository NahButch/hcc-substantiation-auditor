//! Live sanity check for the Ollama provider.
//! Usage: ask "your question"   (requires a running Ollama server + the model)

use agent::llm::{LlmProvider, OllamaProvider, Prompt};

fn main() {
    let model = std::env::var("HCC_MODEL").unwrap_or_else(|_| "qwen2.5:7b-instruct".to_string());
    let q = std::env::args().nth(1).unwrap_or_else(|| "Say 'ok' and nothing else.".to_string());
    let provider = OllamaProvider::new(model);
    let prompt = Prompt { system: "You are a concise assistant.", user: &q, json: false };
    match provider.complete(&prompt) {
        Ok(answer) => println!("[{}] {}", provider.id(), answer.trim()),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
