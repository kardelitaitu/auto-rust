pub mod cli;

use anyhow::{Context, Result};
use log::info;
use std::path::PathBuf;

use crate::llm::models::ChatMessage;

pub const DEFAULT_PROMPT: &str = "You are a helpful coding assistant running on a local \
Ollama model. Be concise, practical, and focused on Rust code.";

fn system_prompt(role: Option<&str>) -> String {
    match role {
        Some(r) => {
            let file = match r {
                "observer" => "01_bacon-observer.md",
                "strategist" => "02_bacon-strategy.md",
                "coder" => "03_bacon-coder.md",
                "auditor" => "04_bacon-auditor.md",
                _ => return DEFAULT_PROMPT.to_string(),
            };
            let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), ".bacon/roles", file]
                .iter()
                .collect();
            std::fs::read_to_string(&path).unwrap_or_else(|_| DEFAULT_PROMPT.to_string())
        }
        None => DEFAULT_PROMPT.to_string(),
    }
}

fn ollama_config() -> (String, String) {
    let url =
        std::env::var("BACON_OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
    let model = std::env::var("BACON_OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:3b".to_string());
    (url, model)
}

pub async fn run(prompt: &str, role: Option<&str>) -> Result<String> {
    let system = system_prompt(role);
    let (base_url, model) = ollama_config();

    let messages = vec![ChatMessage::system(&system), ChatMessage::user(prompt)];

    let url = format!("{}/api/chat", base_url);
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false,
        "temperature": 0.2,
    });

    info!("Calling Ollama ({} / {})...", base_url, model);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await
        .context("Ollama request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Ollama error: {} - {}", status, text);
    }

    let chat_resp: serde_json::Value = resp.json().await?;
    let content = chat_resp["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    println!("{}", content);
    Ok(content)
}
