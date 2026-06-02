//! Agent binary entry point. The full extraction → verify → self-correct CLI
//! lands across Phases 2–3; for now it reports the wired-up engine + LLM config.

use engine::ModelVersion;

fn main() {
    println!("HCC Substantiation Auditor — agent v{}", env!("CARGO_PKG_VERSION"));
    println!("engine model: {}", engine::V28::NAME);
    let provider = agent::llm::OllamaProvider::new(
        std::env::var("HCC_MODEL").unwrap_or_else(|_| "qwen2.5:7b-instruct".to_string()),
    );
    use agent::llm::LlmProvider;
    println!("llm provider: {}", provider.id());
}
