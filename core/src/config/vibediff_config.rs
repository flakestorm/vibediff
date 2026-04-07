#[derive(Debug, Clone)]
pub struct VibeDiffConfig {
    pub local_model: String,
    pub ollama_url: String,
    pub use_mock_llm: bool,
    pub timeout_secs: u64,
}

impl Default for VibeDiffConfig {
    fn default() -> Self {
        Self {
            local_model: "llama3.2:3b".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            use_mock_llm: false,
            timeout_secs: 30,
        }
    }
}
