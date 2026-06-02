//! NVIDIA-only LLM module for the bacon pipeline.
//!
//! Simplified from `auto-rust/src/llm/mod.rs` — removes Ollama/OpenRouter
//! and multi-provider dispatch. The [`Llm`] struct holds [`NvidiaConfig`]
//! directly and delegates to [`LlmClient`] for API calls.

pub mod client;
pub mod models;

use anyhow::Result;
use log::info;

pub use client::LlmClient;
pub use models::{ChatMessage, NvidiaConfig};

/// NVIDIA-only LLM wrapper.
///
/// Usage:
/// ```ignore
/// let llm = Llm::from_env()?;
/// let response = llm.chat(vec![
///     ChatMessage::system("Be helpful"),
///     ChatMessage::user("Hello"),
/// ]).await?;
/// ```
pub struct Llm {
    client: LlmClient,
}

impl Llm {
    /// Create an [`Llm`] from environment variables + bacon.toml overrides.
    ///
    /// Reads `NVIDIA_API_KEY`, `NVIDIA_BASE_URL`, `NVIDIA_MODEL` from env,
    /// falling back to `.bacon/bacon.toml [agents.nvidia]` defaults.
    pub fn from_env() -> Result<Self> {
        let config = load_nvidia_config_from_env();
        info!("LLM client initialized (model: {})", config.model);
        Ok(Self {
            client: LlmClient::new(config),
        })
    }

    /// Create an [`Llm`] from a given [`NvidiaConfig`].
    pub fn from_config(config: NvidiaConfig) -> Self {
        Self {
            client: LlmClient::new(config),
        }
    }

    /// Send a chat request and return the response text.
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        self.client.chat(messages).await
    }

    /// Quick connectivity check.
    pub async fn health_check(&self) -> bool {
        self.client.health_check().await
    }
}

/// Load NVIDIA config from environment, with .env file support.
fn load_nvidia_config_from_env() -> NvidiaConfig {
    // Try loading .env file
    let env_path = crate::config::project_config().env_file.clone();
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

    let mut config = NvidiaConfig::default();

    if let Ok(api_key) = std::env::var("NVIDIA_API_KEY") {
        if !api_key.trim().is_empty() {
            config.api_key = api_key;
        }
    }
    if let Ok(base_url) = std::env::var("NVIDIA_BASE_URL") {
        if !base_url.trim().is_empty() {
            config.base_url = base_url;
        }
    }
    if let Ok(model) = std::env::var("NVIDIA_MODEL") {
        if !model.trim().is_empty() {
            config.model = model;
        }
    }
    if let Ok(temp) = std::env::var("NVIDIA_TEMPERATURE") {
        if let Ok(v) = temp.parse::<f64>() {
            config.temperature = v;
        }
    }
    if let Ok(max_tokens) = std::env::var("NVIDIA_MAX_TOKENS") {
        if let Ok(v) = max_tokens.parse::<u32>() {
            config.max_tokens = v;
        }
    }

    config
}

/// Build a [`NvidiaConfig`] for a specific agent, applying per-agent overrides
/// from `bacon.toml [agents.<name>]` on top of env defaults.
pub fn llm_config_for_agent(agent_name: &str) -> NvidiaConfig {
    let mut config = load_nvidia_config_from_env();
    let agent_cfg = crate::core::PipelineConfig::agent_llm_config(agent_name);

    if let Some(ref model) = agent_cfg.model {
        config.model = model.clone();
    }
    if let Some(ref base_url) = agent_cfg.base_url {
        config.base_url = base_url.clone();
    }
    if let Some(ref api_key) = agent_cfg.api_key {
        let resolved = if api_key.starts_with("{env:") && api_key.ends_with('}') {
            let var_name = &api_key[5..api_key.len() - 1];
            std::env::var(var_name).unwrap_or_else(|_| api_key.clone())
        } else {
            api_key.clone()
        };
        config.api_key = resolved;
    }
    if let Some(temp) = agent_cfg.temperature {
        config.temperature = temp;
    }
    if let Some(top_p) = agent_cfg.top_p {
        config.top_p = top_p;
    }
    if let Some(max_tokens) = agent_cfg.max_tokens {
        config.max_tokens = max_tokens as u32;
    }
    if let Some(timeout_ms) = agent_cfg.timeout_ms {
        config.timeout_ms = timeout_ms;
    }

    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_constructors() {
        let user = ChatMessage::user("Hello");
        assert_eq!(user.role, models::Role::User);

        let sys = ChatMessage::system("Be helpful");
        assert_eq!(sys.role, models::Role::System);

        let asst = ChatMessage::assistant("Response");
        assert_eq!(asst.role, models::Role::Assistant);
    }

    #[test]
    fn test_nvidia_config_default_has_placeholder() {
        let config = NvidiaConfig::default();
        assert!(config.base_url.contains("nvidia.com"));
    }
}
