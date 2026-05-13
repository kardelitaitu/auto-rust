use anyhow::{Context, Result};
use log::{debug, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

fn load_env_file() {
    let env_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');
                if std::env::var(key).is_err() {
                    std::env::set_var(key, value);
                }
            }
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    top_p: f32,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

pub struct NvidiaConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
}

impl Default for NvidiaConfig {
    fn default() -> Self {
        load_env_file();
        Self {
            api_key: std::env::var("NVIDIA_API_KEY")
                .unwrap_or_else(|_| "nvapi-placeholder-key".to_string()),
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            model: "minimaxai/minimax-m2.7".to_string(),
            temperature: 1.0,
            top_p: 0.95,
            max_tokens: 8192,
        }
    }
}

pub async fn chat(config: &NvidiaConfig, system_prompt: &str, user_prompt: &str) -> Result<String> {
    let client = Client::new();
    let url = format!("{}/chat/completions", config.base_url);

    info!("NVIDIA API call: model={}, url={}", config.model, url);
    debug!("system_prompt={}", system_prompt);
    debug!(
        "user_prompt={} ({} chars)",
        &user_prompt[..user_prompt.len().min(80)],
        user_prompt.len()
    );

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: Some(system_prompt.to_string()),
            ..Default::default()
        },
        ChatMessage {
            role: "user".to_string(),
            content: Some(user_prompt.to_string()),
            ..Default::default()
        },
    ];

    let request = ChatRequest {
        model: config.model.clone(),
        messages,
        temperature: config.temperature,
        top_p: config.top_p,
        max_tokens: config.max_tokens,
        stream: false,
    };

    let start = Instant::now();
    info!("Sending request to NVIDIA API...");

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(&request)
        .send()
        .await
        .context("Failed to send request to NVIDIA API")?;

    info!("HTTP response: {}", response.status());

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        anyhow::bail!("NVIDIA API error: {} - {}", status, text);
    }

    let chat_response: ChatResponse = response
        .json()
        .await
        .context("Failed to parse NVIDIA API response")?;

    let content = chat_response
        .choices
        .first()
        .and_then(|choice| {
            choice
                .message
                .content
                .as_deref()
                .or(choice.message.reasoning.as_deref())
                .or(choice.message.reasoning_content.as_deref())
        })
        .unwrap_or_default()
        .to_string();

    if content.is_empty() {
        let reason = chat_response
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
            .unwrap_or("unknown");
        anyhow::bail!("Empty response from NVIDIA API (finish_reason: {})", reason);
    }

    let elapsed = start.elapsed();
    info!(
        "NVIDIA API responded in {:.1}s ({} chars)",
        elapsed.as_secs_f64(),
        content.len()
    );

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // NvidiaConfig Tests
    // ========================================================================

    #[test]
    fn test_nvidia_config_default_has_placeholder_key() {
        let config = NvidiaConfig::default();
        // Should have a non-empty api_key (env or placeholder)
        assert!(!config.api_key.is_empty());
    }

    #[test]
    fn test_nvidia_config_default_base_url() {
        let config = NvidiaConfig::default();
        assert_eq!(config.base_url, "https://integrate.api.nvidia.com/v1");
    }

    #[test]
    fn test_nvidia_config_default_model() {
        let config = NvidiaConfig::default();
        assert_eq!(config.model, "minimaxai/minimax-m2.7");
    }

    #[test]
    fn test_nvidia_config_default_temperature() {
        let config = NvidiaConfig::default();
        assert!((config.temperature - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_nvidia_config_default_top_p() {
        let config = NvidiaConfig::default();
        assert!((config.top_p - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_nvidia_config_default_max_tokens() {
        let config = NvidiaConfig::default();
        assert_eq!(config.max_tokens, 8192);
    }

    // ========================================================================
    // ChatRequest Serialization Tests (indirectly validate ChatMessage structs)
    // ========================================================================

    #[test]
    fn test_chat_message_defaults() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: Some("hello".to_string()),
            ..Default::default()
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content.as_deref(), Some("hello"));
        assert!(msg.reasoning.is_none());
        assert!(msg.reasoning_content.is_none());
    }

    #[test]
    fn test_chat_message_no_content() {
        let msg = ChatMessage {
            role: "system".to_string(),
            content: None,
            ..Default::default()
        };
        assert!(msg.content.is_none());
    }

    #[test]
    fn test_chat_message_serialize() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: Some("test".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"role\":\"user\""));
    }

    // ========================================================================
    // ChatRequest Tests
    // ========================================================================

    #[test]
    fn test_chat_request_creation() {
        let request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![ChatMessage {
                role: "system".to_string(),
                content: Some("be helpful".to_string()),
                ..Default::default()
            }],
            temperature: 0.5,
            top_p: 0.9,
            max_tokens: 1000,
            stream: false,
        };
        assert_eq!(request.model, "test-model");
        assert_eq!(request.messages.len(), 1);
        assert!((request.temperature - 0.5).abs() < 0.01);
        assert!((request.top_p - 0.9).abs() < 0.01);
        assert_eq!(request.max_tokens, 1000);
        assert!(!request.stream);
    }

    #[test]
    fn test_chat_request_serialize() {
        let request = ChatRequest {
            model: "m".to_string(),
            messages: vec![],
            temperature: 1.0,
            top_p: 0.9,
            max_tokens: 100,
            stream: false,
        };
        let json = serde_json::to_string(&request).expect("serialize");
        assert!(json.contains("\"model\":\"m\""));
        assert!(json.contains("\"stream\":false"));
    }

    #[test]
    fn test_nvidia_config_default_loads_env() {
        std::env::remove_var("NVIDIA_API_KEY");
        std::env::set_var("NVIDIA_API_KEY", "test-key");
        let config = NvidiaConfig::default();
        assert_eq!(config.api_key, "test-key");
        std::env::remove_var("NVIDIA_API_KEY");
    }
}
