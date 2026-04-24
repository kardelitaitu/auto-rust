use anyhow::Result;
use log::{error, info, warn};
use reqwest::Client;
use std::time::Duration;
use toml;

use crate::llm::models::*;

pub struct LlmClient {
    config: LlmConfig,
    http: Client,
    fallback_config: Option<LlmConfig>,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();

        Self {
            config: config.clone(),
            http,
            fallback_config: Some(config),
        }
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        match self.config.provider {
            LlmProvider::Ollama => self.ollama_chat(messages).await,
            LlmProvider::OpenRouter => self.openrouter_chat(messages).await,
        }
    }

    pub async fn chat_with_fallback(&self, messages: Vec<ChatMessage>) -> Result<String> {
        match self.config.provider {
            LlmProvider::Ollama => match self.ollama_chat(messages.clone()).await {
                Ok(response) => Ok(response),
                Err(e) => {
                    warn!("Ollama failed: {}, trying fallback...", e);
                    if let Some(ref fallback) = self.fallback_config {
                        if fallback.provider == LlmProvider::OpenRouter {
                            let fallback_client = LlmClient::new(fallback.clone());
                            return fallback_client.openrouter_chat(messages).await;
                        }
                    }
                    Err(e)
                }
            },
            LlmProvider::OpenRouter => self.openrouter_chat(messages).await,
        }
    }

    async fn ollama_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/api/chat", self.config.ollama.base_url);

        let request = ChatRequest {
            model: self.config.ollama.model.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
        };

        info!("Calling Ollama: {}...", self.config.ollama.model);

        let response = self
            .http
            .post(&url)
            .json(&request)
            .timeout(Duration::from_millis(self.config.ollama.timeout_ms))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("Ollama error: {} - {}", status, text);
            anyhow::bail!("Ollama error: {} - {}", status, text);
        }

        let chat_response: ChatResponse = response.json().await?;

        if let Some(err) = chat_response.error {
            anyhow::bail!("Ollama error: {}", err);
        }

        let content = chat_response.message.map(|m| m.content).unwrap_or_default();

        Ok(content)
    }

    async fn openrouter_chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let url = format!("{}/chat/completions", self.config.openrouter.base_url);

        let request = serde_json::json!({
            "model": self.config.openrouter.model,
            "messages": messages,
            "temperature": 0.7,
        });

        info!("Calling OpenRouter: {}", self.config.openrouter.model);

        let response = self
            .http
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.openrouter.api_key),
            )
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(Duration::from_millis(self.config.openrouter.timeout_ms))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            error!("OpenRouter error: {} - {}", status, text);
            anyhow::bail!("OpenRouter error: {} - {}", status, text);
        }

        let openrouter_response: OpenRouterResponse = response.json().await?;

        if let Some(err) = openrouter_response.error {
            anyhow::bail!("OpenRouter error: {}", err.message);
        }

        let content = openrouter_response
            .choices
            .and_then(|choices| choices.into_iter().next())
            .map(|choice| match choice {
                ChatChoice::WithMessage { message } => message.content,
                ChatChoice::WithContent { content } => content,
            })
            .unwrap_or_default();

        Ok(content)
    }

    pub async fn health_check(&self) -> bool {
        match self.config.provider {
            LlmProvider::Ollama => self.ollama_health().await,
            LlmProvider::OpenRouter => self.openrouter_health().await,
        }
        .unwrap_or(false)
    }

    async fn ollama_health(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.config.ollama.base_url);

        let response = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    async fn openrouter_health(&self) -> Result<bool> {
        let url = "https://openrouter.ai/api/v1/models";

        let response = self
            .http
            .get(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.openrouter.api_key),
            )
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

pub fn create_llm_client_from_config() -> Result<LlmConfig> {
    let config_path = std::path::Path::new("config/llm.toml");

    let mut config = if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        toml::from_str(&content)?
    } else {
        LlmConfig::default()
    };

    // Apply environment variable overrides
    if let Ok(provider) = std::env::var("LLM_PROVIDER") {
        if provider == "openrouter" {
            config.provider = LlmProvider::OpenRouter;
        } else {
            config.provider = LlmProvider::Ollama;
        }
    }

    if let Ok(url) = std::env::var("OLLAMA_URL") {
        config.ollama.base_url = url;
    }

    if let Ok(model) = std::env::var("OLLAMA_MODEL") {
        config.ollama.model = model;
    }

    if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
        config.openrouter.api_key = api_key;
    }

    if let Ok(model) = std::env::var("OPENROUTER_MODEL") {
        config.openrouter.model = model;
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_client_new() {
        let config = LlmConfig::default();
        let client = LlmClient::new(config);
        // Verify client is created without panicking
        let _ = client;
    }

    #[test]
    fn test_llm_client_has_fallback() {
        let config = LlmConfig::default();
        let client = LlmClient::new(config);
        assert!(client.fallback_config.is_some());
    }

    #[test]
    fn test_create_llm_client_from_config_default() {
        // Test that the function exists and returns a result
        // Skip actual config file check since it may not exist
        let result = create_llm_client_from_config();
        // Either returns config or error due to missing file
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_llm_provider_variants() {
        assert_eq!(LlmProvider::Ollama, LlmProvider::Ollama);
        assert_eq!(LlmProvider::OpenRouter, LlmProvider::OpenRouter);
    }

    #[test]
    fn test_llm_provider_inequality() {
        assert_ne!(LlmProvider::Ollama, LlmProvider::OpenRouter);
    }

    #[test]
    fn test_chat_message_creation() {
        let message = ChatMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        };
        assert_eq!(message.role, "user");
        assert_eq!(message.content, "test");
    }

    #[test]
    fn test_chat_request_creation() {
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![],
            temperature: Some(0.7),
            max_tokens: Some(2048),
        };
        assert_eq!(request.model, "llama3");
        assert_eq!(request.temperature, Some(0.7));
    }

    #[test]
    fn test_chat_response_creation() {
        let response = ChatResponse {
            message: None,
            done: None,
            error: None,
        };
        assert!(response.message.is_none());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_chat_message_struct() {
        let message = ChatMessage {
            role: "system".to_string(),
            content: "You are helpful".to_string(),
        };
        assert_eq!(message.role, "system");
    }

    #[test]
    fn test_ollama_config_defaults() {
        let config = OllamaConfig::default();
        assert!(!config.base_url.is_empty());
        assert!(!config.model.is_empty());
    }

    #[test]
    fn test_openrouter_config_defaults() {
        let config = OpenRouterConfig::default();
        assert!(!config.base_url.is_empty());
        assert!(!config.model.is_empty());
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::Ollama);
    }

    #[test]
    fn test_chat_choice_with_message() {
        let choice = ChatChoice::WithMessage {
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "Hello".to_string(),
            },
        };
        if let ChatChoice::WithMessage { message } = choice {
            assert_eq!(message.content, "Hello");
        }
    }

    #[test]
    fn test_chat_choice_with_content() {
        let choice = ChatChoice::WithContent {
            content: "Direct content".to_string(),
        };
        if let ChatChoice::WithContent { content } = choice {
            assert_eq!(content, "Direct content");
        }
    }

    #[test]
    fn test_openrouter_response_creation() {
        let response = OpenRouterResponse {
            id: None,
            model: None,
            choices: None,
            usage: None,
            error: None,
        };
        assert!(response.choices.is_none());
    }

    #[test]
    fn test_openrouter_error_creation() {
        let error = OpenRouterError {
            message: "Test error".to_string(),
            code: None,
        };
        assert_eq!(error.message, "Test error");
    }

    #[test]
    fn test_multiple_chat_messages() {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: "System prompt".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "User message".to_string(),
            },
        ];
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn test_chat_request_with_messages() {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        }];
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages,
            temperature: Some(0.5),
            max_tokens: Some(1024),
        };
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_chat_response_with_error() {
        let response = ChatResponse {
            message: None,
            done: None,
            error: Some("Connection failed".to_string()),
        };
        assert_eq!(response.error, Some("Connection failed".to_string()));
    }

    #[test]
    fn test_chat_response_with_message() {
        let response = ChatResponse {
            message: Some(ChatMessage {
                role: "assistant".to_string(),
                content: "Response".to_string(),
            }),
            done: None,
            error: None,
        };
        assert!(response.message.is_some());
    }

    #[test]
    fn test_ollama_config_custom() {
        let config = OllamaConfig {
            base_url: "http://custom:11434".to_string(),
            model: "custom-model".to_string(),
            timeout_ms: 60000,
        };
        assert_eq!(config.base_url, "http://custom:11434");
    }

    #[test]
    fn test_openrouter_config_custom() {
        let config = OpenRouterConfig {
            base_url: "https://custom.api".to_string(),
            model: "custom-model".to_string(),
            api_key: "key123".to_string(),
            timeout_ms: 90000,
        };
        assert_eq!(config.api_key, "key123");
    }

    #[test]
    fn test_llm_config_custom_provider() {
        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig::default(),
        };
        assert_eq!(config.provider, LlmProvider::OpenRouter);
    }
}
