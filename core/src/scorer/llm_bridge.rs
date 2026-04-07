use async_trait::async_trait;
use serde::Deserialize;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String>;
}

pub struct MockProvider;

#[async_trait]
impl LlmProvider for MockProvider {
    async fn complete(&self, _system: &str, _user: &str) -> anyhow::Result<String> {
        Ok(
            r#"{"scores":{"logic_match":0.8,"scope_adherence":0.8,"side_effect_detection":0.8,"structural_proportionality":0.8},"reasoning":{"logic_match":"ok","scope_adherence":"ok","side_effect_detection":"ok","structural_proportionality":"ok"},"flagged_entities":[],"suggested_commit_message":null}"#
                .to_string(),
        )
    }
}

pub struct OllamaProvider {
    client: reqwest::Client,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl OllamaProvider {
    pub fn new(base_url: String, model: String, timeout_secs: u64) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            model,
            timeout_secs,
        }
    }
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn complete(&self, system: &str, user: &str) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role":"system","content":system},
                {"role":"user","content":user}
            ],
            "stream": false,
            "options": {"temperature": 0.0, "seed": 42, "num_predict": 1024}
        });
        let res = self
            .client
            .post(format!("{}/api/chat", self.base_url.trim_end_matches('/')))
            .json(&body)
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .send()
            .await?;
        let parsed: OllamaResponse = res.json().await?;
        Ok(parsed.message.content)
    }
}
