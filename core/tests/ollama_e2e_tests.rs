#[tokio::test]
async fn ollama_e2e_smoke_if_available() {
    // Optional integration test: skipped unless explicitly enabled and Ollama reachable.
    let enabled = std::env::var("VIBEDIFF_RUN_OLLAMA_E2E").ok().as_deref() == Some("1");
    if !enabled {
        return;
    }
    let client = reqwest::Client::new();
    let ok = client
        .get("http://localhost:11434")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .is_ok();
    assert!(ok, "Ollama is not reachable at http://localhost:11434");
}
