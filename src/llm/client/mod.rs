use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

use crate::llm::models::{LlmConfig, LlmProvider};

pub mod fallback;

#[cfg(test)]
mod tests;

pub struct LlmClient {
    pub(crate) config: LlmConfig,
    pub(crate) http: Client,
    pub(crate) fallback_config: Option<LlmConfig>,
}

impl LlmClient {
    #[must_use]
    pub fn new(config: LlmConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(600))
            // Enable HTTP/2 for better throughput with LLM APIs (negotiated)
            .http2_adaptive_window(true)
            // Connection pool settings for concurrent requests
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();

        Self {
            config: config.clone(),
            http,
            fallback_config: Some(config),
        }
    }
}

fn load_env_file() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let env_path = root.join(".env");
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

/// Apply environment variable overrides to an `LlmConfig`.
///
/// This is a pure function that takes a config and an environment lookup function,
/// making it testable without touching real environment variables.
/// The `get_env` callback receives an env var name and returns `Some(value)` if set.
#[must_use]
pub(crate) fn apply_env_overrides(
    mut config: LlmConfig,
    get_env: impl Fn(&str) -> Option<String>,
) -> LlmConfig {
    // Provider override
    if let Some(provider) = get_env("LLM_PROVIDER") {
        match provider.to_lowercase().as_str() {
            "openrouter" => config.provider = LlmProvider::OpenRouter,
            "nvidia" => config.provider = LlmProvider::Nvidia,
            _ => config.provider = LlmProvider::Ollama,
        }
    }

    // Ollama overrides
    if let Some(url) = get_env("OLLAMA_URL") {
        config.ollama.base_url = url;
    }
    if let Some(model) = get_env("OLLAMA_MODEL") {
        config.ollama.model = model;
    }

    // OpenRouter overrides
    if let Some(api_key) = get_env("OPENROUTER_API_KEY") {
        config.openrouter.api_key = api_key;
    }
    if let Some(model) = get_env("OPENROUTER_MODEL") {
        config.openrouter.model = model;
    }

    // NVIDIA overrides
    if let Some(api_key) = get_env("NVIDIA_API_KEY") {
        config.nvidia.api_key = api_key;
    }
    if let Some(model) = get_env("NVIDIA_MODEL") {
        config.nvidia.model = model;
    }
    if let Some(base_url) = get_env("NVIDIA_BASE_URL") {
        config.nvidia.base_url = base_url;
    }

    // Fallback models from env vars
    let mut fallbacks = Vec::new();
    for key in [
        "OPENROUTER_MODEL_FALLBACK",
        "OPENROUTER_MODEL_FALLBACK_2",
        "OPENROUTER_MODEL_FALLBACK_3",
        "OPENROUTER_MODEL_FALLBACK_4",
    ] {
        if let Some(fb_model) = get_env(key) {
            if !fb_model.is_empty() {
                fallbacks.push(fb_model);
            }
        }
    }
    config.openrouter.fallback_models = fallbacks;

    config
}

pub fn create_llm_client_from_config() -> Result<LlmConfig> {
    load_env_file();
    let config_path = std::path::Path::new("config/llm.toml");

    let config = if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        LlmConfig::default()
    };

    // Apply environment variable overrides
    let config = apply_env_overrides(config, |key| std::env::var(key).ok());

    Ok(config)
}
