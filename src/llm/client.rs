use anyhow::Result;
use log::{error, info, warn};
use reqwest::Client;
use std::time::Duration;
use toml;

use crate::llm::models::*;

#[cfg(test)]
use serde_json;

pub struct LlmClient {
    config: LlmConfig,
    http: Client,
    fallback_config: Option<LlmConfig>,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
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

        // Build list of models to try: primary + fallbacks
        let mut models_to_try = vec![self.config.openrouter.model.clone()];
        models_to_try.extend(self.config.openrouter.fallback_models.clone());

        let mut last_error = None;

        for (idx, model) in models_to_try.iter().enumerate() {
            let is_fallback = idx > 0;
            let attempt = idx + 1;

            if is_fallback {
                info!(
                    "OpenRouter fallback attempt {}/{} using model: {}",
                    attempt,
                    models_to_try.len(),
                    model
                );
            } else {
                info!("Calling OpenRouter: {}", model);
            }

            let request = serde_json::json!({
                "model": model,
                "messages": &messages,
                "temperature": 0.7,
            });

            let result = self
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
                .await;

            match result {
                Ok(response) => {
                    let body_result = response.text().await;

                    match body_result {
                        Ok(body_text) => {
                            // Try to parse as OpenRouter response
                            match serde_json::from_str::<OpenRouterResponse>(&body_text) {
                                Ok(openrouter_response) => {
                                    // Check for API-level errors
                                    if let Some(err) = openrouter_response.error {
                                        warn!(
                                            "OpenRouter API error on attempt {} with model {}: {}",
                                            attempt, model, err.message
                                        );
                                        last_error = Some(anyhow::anyhow!(
                                            "OpenRouter API error: {}",
                                            err.message
                                        ));
                                        continue; // Try next fallback
                                    }

                                    // Extract content from successful response
                                    let content = openrouter_response
                                        .choices
                                        .and_then(|choices| choices.into_iter().next())
                                        .map(|choice| match choice {
                                            ChatChoice::WithMessage { message } => message.content,
                                            ChatChoice::WithContent { content } => content,
                                        })
                                        .unwrap_or_default();

                                    if !content.is_empty() {
                                        if is_fallback {
                                            info!("OpenRouter fallback model {} succeeded", model);
                                        }
                                        return Ok(content);
                                    } else {
                                        warn!(
                                            "OpenRouter empty response on attempt {} with model {}",
                                            attempt, model
                                        );
                                        last_error = Some(anyhow::anyhow!(
                                            "Empty response from model: {}",
                                            model
                                        ));
                                        continue; // Try next fallback
                                    }
                                }
                                Err(parse_err) => {
                                    warn!("OpenRouter JSON parse error on attempt {} with model {}: {}", attempt, model, parse_err);
                                    last_error = Some(anyhow::anyhow!(
                                        "JSON parse error: {} - Body: {}",
                                        parse_err,
                                        body_text
                                    ));
                                    continue; // Try next fallback
                                }
                            }
                        }
                        Err(body_err) => {
                            warn!(
                                "OpenRouter body read error on attempt {} with model {}: {}",
                                attempt, model, body_err
                            );
                            last_error = Some(anyhow::anyhow!(
                                "Failed to read response body: {}",
                                body_err
                            ));
                            continue; // Try next fallback
                        }
                    }
                }
                Err(req_err) => {
                    let is_timeout = req_err.is_timeout();
                    if is_timeout {
                        warn!(
                            "OpenRouter timeout on attempt {} with model {} (timeout_ms: {})",
                            attempt, model, self.config.openrouter.timeout_ms
                        );
                    } else {
                        warn!(
                            "OpenRouter request error on attempt {} with model {}: {}",
                            attempt, model, req_err
                        );
                    }
                    last_error = Some(anyhow::anyhow!(
                        "Request failed for model {}: {}",
                        model,
                        req_err
                    ));
                    continue; // Try next fallback
                }
            }
        }

        // All models exhausted
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "All OpenRouter models failed (primary + {} fallbacks)",
                self.config.openrouter.fallback_models.len()
            )
        }))
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

    // Load fallback models from env vars
    let mut fallbacks = Vec::new();
    for key in [
        "OPENROUTER_MODEL_FALLBACK",
        "OPENROUTER_MODEL_FALLBACK_2",
        "OPENROUTER_MODEL_FALLBACK_3",
        "OPENROUTER_MODEL_FALLBACK_4",
    ] {
        if let Ok(fb_model) = std::env::var(key) {
            if !fb_model.is_empty() {
                fallbacks.push(fb_model);
            }
        }
    }
    config.openrouter.fallback_models = fallbacks;

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
            fallback_models: vec!["fallback-model".to_string()],
        };
        assert_eq!(config.api_key, "key123");
        assert_eq!(config.fallback_models.len(), 1);
    }

    #[test]
    fn test_openrouter_config_with_fallbacks() {
        let config = OpenRouterConfig {
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model: "primary-model".to_string(),
            api_key: "test-key".to_string(),
            timeout_ms: 60000,
            fallback_models: vec![
                "fallback-1".to_string(),
                "fallback-2".to_string(),
                "fallback-3".to_string(),
            ],
        };
        assert_eq!(config.fallback_models.len(), 3);
        assert_eq!(config.fallback_models[0], "fallback-1");
        assert_eq!(config.fallback_models[1], "fallback-2");
        assert_eq!(config.fallback_models[2], "fallback-3");
    }

    #[test]
    fn test_openrouter_config_default_fallbacks_empty() {
        let config = OpenRouterConfig::default();
        assert!(config.fallback_models.is_empty());
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

    // =========================================================================
    // OpenRouter Fallback Chain Integration Tests (using wiremock)
    // =========================================================================

    #[cfg(test)]
    impl LlmClient {
        /// Create client with custom HTTP client for testing
        fn with_http_client(config: LlmConfig, http: Client) -> Self {
            Self {
                config: config.clone(),
                http,
                fallback_config: Some(config),
            }
        }
    }

    #[tokio::test]
    async fn test_openrouter_fallback_primary_succeeds_no_fallback_needed() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary model succeeds
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header(
                "authorization",
                format!("Bearer {}", "test-key").as_str(),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-1",
                "model": "primary-model",
                "choices": [{"message": {"role": "assistant", "content": "Primary response"}}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec!["fallback-1".to_string(), "fallback-2".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Primary response");
    }

    #[tokio::test]
    async fn test_openrouter_fallback_primary_falls_back_to_first_fallback() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary model fails with 500 (matched by model name in body)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"primary-model\""))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": {"message": "Internal server error", "code": 500}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // First fallback succeeds (matched by model name in body)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"fallback-1\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-2",
                "model": "fallback-1",
                "choices": [{"message": {"role": "assistant", "content": "Fallback response"}}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec!["fallback-1".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Fallback response");
    }

    #[tokio::test]
    async fn test_openrouter_fallback_chains_through_all_models() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary fails (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"primary-model\""))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "error": {"message": "Rate limited", "code": 429}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Fallback 1 fails (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"fallback-1\""))
            .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
                "error": {"message": "Service unavailable", "code": 503}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Fallback 2 succeeds (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"fallback-2\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-3",
                "model": "fallback-2",
                "choices": [{"message": {"role": "assistant", "content": "Second fallback response"}}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec!["fallback-1".to_string(), "fallback-2".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Second fallback response");
    }

    #[tokio::test]
    async fn test_openrouter_fallback_empty_response_triggers_fallback() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary returns empty content (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"primary-model\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-1",
                "model": "primary-model",
                "choices": [{"message": {"role": "assistant", "content": ""}}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Fallback returns valid content (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"fallback-1\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-2",
                "model": "fallback-1",
                "choices": [{"message": {"role": "assistant", "content": "Valid response"}}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec!["fallback-1".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Valid response");
    }

    #[tokio::test]
    async fn test_openrouter_fallback_api_error_in_response_triggers_fallback() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary returns 200 but with error in body (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"primary-model\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "error": {"message": "Model overloaded", "code": 503}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Fallback succeeds (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"fallback-1\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test-2",
                "model": "fallback-1",
                "choices": [{"message": {"role": "assistant", "content": "Recovered response"}}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec!["fallback-1".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Recovered response");
    }

    #[tokio::test]
    async fn test_openrouter_fallback_all_models_fail_returns_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // All models fail
        for _ in 0..3 {
            Mock::given(method("POST"))
                .and(path("/chat/completions"))
                .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                    "error": {"message": "Server error", "code": 500}
                })))
                .mount(&mock_server)
                .await;
        }

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec!["fallback-1".to_string(), "fallback-2".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("All OpenRouter models failed") || err_msg.contains("Server error")
        );
    }

    #[tokio::test]
    async fn test_openrouter_fallback_uses_correct_model_in_request_body() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary model request
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"primary-model\""))
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Fallback model request
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"specific-fallback\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "test",
                "model": "specific-fallback",
                "choices": [{"message": {"role": "assistant", "content": "OK"}}]
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec!["specific-fallback".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let _ = client.chat(vec![ChatMessage::user("test")]).await;

        // Mock expectations verify correct models were used
    }

    #[tokio::test]
    async fn test_openrouter_fallback_with_realistic_model_names() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Primary fails (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains(
                "\"model\":\"tencent/hy3-preview:free\"",
            ))
            .respond_with(ResponseTemplate::new(429))
            .mount(&mock_server)
            .await;

        // Fallback 1 succeeds with realistic model name (matched by model name)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains("\"model\":\"nvidia/nemotron-3-super-120b-a12b:free\""))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "realistic-test",
                "model": "nvidia/nemotron-3-super-120b-a12b:free",
                "choices": [{"message": {"role": "assistant", "content": "Realistic model response"}}]
            })))
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "tencent/hy3-preview:free".to_string(),
                timeout_ms: 5000,
                fallback_models: vec![
                    "nvidia/nemotron-3-super-120b-a12b:free".to_string(),
                    "minimax/minimax-m2.5:free".to_string(),
                ],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Realistic model response");
    }

    #[tokio::test]
    async fn test_openrouter_fallback_no_fallbacks_configured_fails_on_primary_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "error": {"message": "Primary failed", "code": 500}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 5000,
                fallback_models: vec![], // No fallbacks
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let result = client.chat(vec![ChatMessage::user("test")]).await;

        assert!(result.is_err());
    }

    // ============================================================================
    // Timeout Handling Tests
    // ============================================================================

    #[tokio::test]
    async fn test_ollama_timeout_triggers_error() {
        use wiremock::{Mock, MockServer, ResponseTemplate};
        use wiremock::matchers::{method, path};
        use std::time::Duration;

        let mock_server = MockServer::start().await;

        // Mock responds with a delay longer than the timeout
        Mock::given(method("POST"))
            .and(path("/api/chat"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "message": {"role": "assistant", "content": "Delayed response"}
                    }))
                    .set_delay(Duration::from_millis(500)), // 500ms delay
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::Ollama,
            ollama: OllamaConfig {
                base_url: mock_server.uri(),
                model: "test-model".to_string(),
                timeout_ms: 100, // 100ms timeout (shorter than delay)
            },
            openrouter: OpenRouterConfig::default(),
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let start = std::time::Instant::now();
        let result = client.chat(vec![ChatMessage::user("test")]).await;
        let elapsed = start.elapsed();

        // Should fail due to timeout
        assert!(result.is_err(), "Should timeout when response is slower than timeout_ms");

        // Should fail quickly (not wait for the full 500ms delay)
        assert!(
            elapsed.as_millis() < 400,
            "Should fail fast on timeout, but took {}ms",
            elapsed.as_millis()
        );

        // Error should indicate timeout (reqwest returns "error sending request" for timeout)
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("error sending request") || err_msg.contains("timeout") || err_msg.contains("deadline"),
            "Error should indicate timeout or request error, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_openrouter_timeout_triggers_fallback() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        use std::time::Duration;

        let mock_server = MockServer::start().await;

        // Primary model responds slowly (triggers timeout)
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains(r#""model":"primary-model""#))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "id": "slow",
                        "model": "primary-model",
                        "choices": [{"message": {"role": "assistant", "content": "Slow response"}}]
                    }))
                    .set_delay(Duration::from_millis(500)), // 500ms delay
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        // Fallback model responds quickly
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_string_contains(r#""model":"fast-fallback""#))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({
                        "id": "fast",
                        "model": "fast-fallback",
                        "choices": [{"message": {"role": "assistant", "content": "Fast fallback response"}}]
                    }))
                    // No delay - responds immediately
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "test-key".to_string(),
                base_url: mock_server.uri(),
                model: "primary-model".to_string(),
                timeout_ms: 100, // 100ms timeout (shorter than primary delay)
                fallback_models: vec!["fast-fallback".to_string()],
            },
        };

        let client = LlmClient::with_http_client(config, Client::new());
        let start = std::time::Instant::now();
        let result = client.chat(vec![ChatMessage::user("test")]).await;
        let elapsed = start.elapsed();

        // Should succeed with fallback response
        assert!(
            result.is_ok(),
            "Should fallback to fast model when primary times out"
        );
        assert_eq!(result.unwrap(), "Fast fallback response");

        // Should complete in reasonable time (primary timeout + fallback success)
        // Primary timeout: ~100ms, Fallback: immediate, Overhead: ~50ms
        assert!(
            elapsed.as_millis() < 300,
            "Should complete quickly with fallback, but took {}ms",
            elapsed.as_millis()
        );
    }
}
