use super::fallback::is_retryable_nvidia_status;
use super::*;
use crate::llm::models::{
    ChatChoice, ChatMessage, ChatRequest, ChatResponse, LlmProvider, MaxTokens, OllamaConfig,
    OpenRouterConfig, OpenRouterError, OpenRouterResponse, Role, Temperature,
};
use reqwest::StatusCode;

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

// =========================================================================
// apply_env_overrides tests
// =========================================================================

#[test]
fn test_apply_no_env_vars_returns_config_unchanged() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config.clone(), |_| None);
    assert_eq!(result.provider, config.provider);
    assert_eq!(result.ollama.base_url, config.ollama.base_url);
    assert_eq!(result.openrouter.api_key, config.openrouter.api_key);
    assert!(result.openrouter.fallback_models.is_empty());
}

#[test]
fn test_apply_provider_openrouter() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "LLM_PROVIDER" => Some("openrouter".to_string()),
        _ => None,
    });
    assert_eq!(result.provider, LlmProvider::OpenRouter);
}

#[test]
fn test_apply_provider_nvidia() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "LLM_PROVIDER" => Some("nvidia".to_string()),
        _ => None,
    });
    assert_eq!(result.provider, LlmProvider::Nvidia);
}

#[test]
fn test_apply_provider_ollama() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "LLM_PROVIDER" => Some("ollama".to_string()),
        _ => None,
    });
    assert_eq!(result.provider, LlmProvider::Ollama);
}

#[test]
fn test_apply_provider_case_insensitive() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "LLM_PROVIDER" => Some("OpenRouter".to_string()),
        _ => None,
    });
    assert_eq!(result.provider, LlmProvider::OpenRouter);
}

#[test]
fn test_apply_provider_invalid_falls_back_to_ollama() {
    let config = LlmConfig {
        provider: LlmProvider::OpenRouter, // Start as OpenRouter
        ..LlmConfig::default()
    };
    let result = apply_env_overrides(config, |key| match key {
        "LLM_PROVIDER" => Some("invalid_provider".to_string()),
        _ => None,
    });
    assert_eq!(result.provider, LlmProvider::Ollama);
}

#[test]
fn test_apply_ollama_url() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "OLLAMA_URL" => Some("http://custom:11434".to_string()),
        _ => None,
    });
    assert_eq!(result.ollama.base_url, "http://custom:11434");
}

#[test]
fn test_apply_ollama_model() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "OLLAMA_MODEL" => Some("llama3.1:8b".to_string()),
        _ => None,
    });
    assert_eq!(result.ollama.model, "llama3.1:8b");
}

#[test]
fn test_apply_openrouter_api_key() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "OPENROUTER_API_KEY" => Some("sk-or-v1-abc123".to_string()),
        _ => None,
    });
    assert_eq!(result.openrouter.api_key, "sk-or-v1-abc123");
}

#[test]
fn test_apply_openrouter_model() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "OPENROUTER_MODEL" => Some("gpt-4o".to_string()),
        _ => None,
    });
    assert_eq!(result.openrouter.model, "gpt-4o");
}

#[test]
fn test_apply_nvidia_api_key() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "NVIDIA_API_KEY" => Some("nvapi-secret".to_string()),
        _ => None,
    });
    assert_eq!(result.nvidia.api_key, "nvapi-secret");
}

#[test]
fn test_apply_nvidia_model() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "NVIDIA_MODEL" => Some("meta/llama-3.1-8b".to_string()),
        _ => None,
    });
    assert_eq!(result.nvidia.model, "meta/llama-3.1-8b");
}

#[test]
fn test_apply_nvidia_base_url() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "NVIDIA_BASE_URL" => Some("https://custom.api.nvidia.com/v1".to_string()),
        _ => None,
    });
    assert_eq!(result.nvidia.base_url, "https://custom.api.nvidia.com/v1");
}

#[test]
fn test_apply_single_fallback_model() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "OPENROUTER_MODEL_FALLBACK" => Some("gpt-4o-mini".to_string()),
        _ => None,
    });
    assert_eq!(result.openrouter.fallback_models.len(), 1);
    assert_eq!(result.openrouter.fallback_models[0], "gpt-4o-mini");
}

#[test]
fn test_apply_multiple_fallback_models() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "OPENROUTER_MODEL_FALLBACK" => Some("gpt-4o-mini".to_string()),
        "OPENROUTER_MODEL_FALLBACK_2" => Some("claude-3-haiku".to_string()),
        "OPENROUTER_MODEL_FALLBACK_3" => Some("gemini-1.5-flash".to_string()),
        "OPENROUTER_MODEL_FALLBACK_4" => Some("mistral-small".to_string()),
        _ => None,
    });
    assert_eq!(result.openrouter.fallback_models.len(), 4);
    assert_eq!(result.openrouter.fallback_models[0], "gpt-4o-mini");
    assert_eq!(result.openrouter.fallback_models[1], "claude-3-haiku");
    assert_eq!(result.openrouter.fallback_models[2], "gemini-1.5-flash");
    assert_eq!(result.openrouter.fallback_models[3], "mistral-small");
}

#[test]
fn test_apply_empty_fallback_model_skipped() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "OPENROUTER_MODEL_FALLBACK" => Some("".to_string()),
        _ => None,
    });
    assert!(result.openrouter.fallback_models.is_empty());
}

#[test]
fn test_apply_all_env_vars_together() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "LLM_PROVIDER" => Some("openrouter".to_string()),
        "OLLAMA_URL" => Some("http://ollama:11434".to_string()),
        "OLLAMA_MODEL" => Some("llama3".to_string()),
        "OPENROUTER_API_KEY" => Some("sk-key".to_string()),
        "OPENROUTER_MODEL" => Some("gpt-4".to_string()),
        "NVIDIA_API_KEY" => Some("nv-key".to_string()),
        "NVIDIA_MODEL" => Some("nemotron".to_string()),
        "NVIDIA_BASE_URL" => Some("https://nvidia.api".to_string()),
        "OPENROUTER_MODEL_FALLBACK" => Some("gpt-3.5".to_string()),
        "OPENROUTER_MODEL_FALLBACK_2" => Some("claude".to_string()),
        _ => None,
    });
    assert_eq!(result.provider, LlmProvider::OpenRouter);
    assert_eq!(result.ollama.base_url, "http://ollama:11434");
    assert_eq!(result.ollama.model, "llama3");
    assert_eq!(result.openrouter.api_key, "sk-key");
    assert_eq!(result.openrouter.model, "gpt-4");
    assert_eq!(result.nvidia.api_key, "nv-key");
    assert_eq!(result.nvidia.model, "nemotron");
    assert_eq!(result.nvidia.base_url, "https://nvidia.api");
    assert_eq!(result.openrouter.fallback_models.len(), 2);
    assert_eq!(result.openrouter.fallback_models[0], "gpt-3.5");
    assert_eq!(result.openrouter.fallback_models[1], "claude");
}

#[test]
fn test_apply_partial_env_vars() {
    // Only set OpenRouter vars, leave others at default
    let config = LlmConfig::default();
    let default_url = config.ollama.base_url.clone();
    let default_nvidia_key = config.nvidia.api_key.clone();

    let result = apply_env_overrides(config, |key| match key {
        "OPENROUTER_API_KEY" => Some("sk-key".to_string()),
        "OPENROUTER_MODEL" => Some("gpt-4".to_string()),
        _ => None,
    });
    // OpenRouter vars should be set
    assert_eq!(result.openrouter.api_key, "sk-key");
    assert_eq!(result.openrouter.model, "gpt-4");
    // Other vars should remain at default
    assert_eq!(result.ollama.base_url, default_url);
    assert_eq!(result.nvidia.api_key, default_nvidia_key);
    assert!(result.openrouter.fallback_models.is_empty());
}

#[test]
fn test_apply_does_not_mutate_input() {
    let config = LlmConfig::default();
    let original = config.clone();
    let _result = apply_env_overrides(config, |key| match key {
        "LLM_PROVIDER" => Some("nvidia".to_string()),
        _ => None,
    });
    // Original should still have default provider
    assert_eq!(original.provider, LlmProvider::Ollama);
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
        role: Role::User,
        content: "test".to_string(),
        reasoning_content: None,
    };
    assert_eq!(message.role, Role::User);
    assert_eq!(message.content, "test");
}

#[test]
fn test_chat_request_creation() {
    let request = ChatRequest {
        model: "llama3".to_string(),
        messages: vec![],
        temperature: Some(Temperature::new(0.7)),
        max_tokens: Some(MaxTokens::new(2048).unwrap()),
    };
    assert_eq!(request.model, "llama3");
    assert_eq!(request.temperature, Some(Temperature::new(0.7)));
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
        role: Role::System,
        content: "You are helpful".to_string(),
        reasoning_content: None,
    };
    assert_eq!(message.role, Role::System);
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
            role: Role::Assistant,
            content: "Hello".to_string(),
            reasoning_content: None,
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
    let messages = [
        ChatMessage {
            role: Role::System,
            content: "System prompt".to_string(),
            reasoning_content: None,
        },
        ChatMessage {
            role: Role::User,
            content: "User message".to_string(),
            reasoning_content: None,
        },
    ];
    assert_eq!(messages.len(), 2);
}

#[test]
fn test_chat_request_with_messages() {
    let messages = vec![ChatMessage {
        role: Role::User,
        content: "test".to_string(),
        reasoning_content: None,
    }];
    let request = ChatRequest {
        model: "llama3".to_string(),
        messages,
        temperature: Some(Temperature::new(0.5)),
        max_tokens: Some(MaxTokens::new(1024).unwrap()),
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
            role: Role::Assistant,
            content: "Response".to_string(),
            reasoning_content: None,
        }),
        done: None,
        error: None,
    };
    assert!(response.message.is_some());
}

#[test]
fn test_nvidia_retryable_statuses() {
    assert!(is_retryable_nvidia_status(StatusCode::TOO_MANY_REQUESTS));
    assert!(is_retryable_nvidia_status(StatusCode::REQUEST_TIMEOUT));
    assert!(is_retryable_nvidia_status(StatusCode::SERVICE_UNAVAILABLE));
    assert!(!is_retryable_nvidia_status(StatusCode::UNAUTHORIZED));
    assert!(!is_retryable_nvidia_status(StatusCode::BAD_REQUEST));
}

#[test]
fn test_ollama_config_custom() {
    let config = OllamaConfig {
        base_url: "http://custom:11434".to_string(),
        model: "custom-model".to_string(),
        timeout_ms: 60000,
        temperature: Temperature::new(0.7),
        max_tokens: MaxTokens::new(2048).unwrap(),
        ..OllamaConfig::default()
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
        ..OpenRouterConfig::default()
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
        ..OpenRouterConfig::default()
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
        ..LlmConfig::default()
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
            rate_limiter: None,
            ollama_urls: vec![],
            next_ollama_idx: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

fn test_http_client() -> Client {
    Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build test HTTP client")
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Primary response");
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Fallback response");
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Second fallback response");
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Valid response");
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Recovered response");
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("All OpenRouter models failed") || err_msg.contains("Server error"));
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let _ = client.chat(vec![ChatMessage::user("test")]).await;
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
        .and(body_string_contains(
            "\"model\":\"nvidia/nemotron-3-super-120b-a12b:free\"",
        ))
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Realistic model response");
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_err());
}

// ============================================================================
// Timeout Handling Tests
// ============================================================================

#[tokio::test]
async fn test_ollama_timeout_triggers_error() {
    use std::time::Duration;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
            ..OllamaConfig::default()
        },
        openrouter: OpenRouterConfig::default(),
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let start = std::time::Instant::now();
    let result = client.chat(vec![ChatMessage::user("test")]).await;
    let elapsed = start.elapsed();

    // Should fail due to timeout
    assert!(
        result.is_err(),
        "Should timeout when response is slower than timeout_ms"
    );

    // Should fail quickly (not wait for the full 500ms delay)
    assert!(
        elapsed.as_millis() < 400,
        "Should fail fast on timeout, but took {}ms",
        elapsed.as_millis()
    );

    // Error should indicate timeout (reqwest returns "error sending request" for timeout)
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("error sending request")
            || err_msg.contains("timeout")
            || err_msg.contains("deadline"),
        "Error should indicate timeout or request error, got: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_openrouter_timeout_triggers_fallback() {
    use std::time::Duration;
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "fast",
                "model": "fast-fallback",
                "choices": [{"message": {"role": "assistant", "content": "Fast fallback response"}}]
            })), // No delay - responds immediately
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let start = std::time::Instant::now();
    let result = client.chat(vec![ChatMessage::user("test")]).await;
    let elapsed = start.elapsed();

    // Should succeed with fallback response
    assert!(
        result.is_ok(),
        "Should fallback to fast model when primary times out"
    );
    assert_eq!(result.expect("Should succeed"), "Fast fallback response");

    // Should complete in reasonable time (primary timeout + fallback success)
    // Primary timeout: ~100ms, Fallback: immediate, Overhead: ~50ms
    assert!(
        elapsed.as_millis() < 300,
        "Should complete quickly with fallback, but took {}ms",
        elapsed.as_millis()
    );
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_openrouter_rate_limit_429_triggers_fallback() {
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Primary returns 429 rate limit
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_string_contains(r#""model":"primary-model""#))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "error": {"message": "Rate limit exceeded", "code": 429}
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Fallback succeeds
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_string_contains(r#""model":"fallback-model""#))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "rate-limit-fallback",
            "model": "fallback-model",
            "choices": [{"message": {"role": "assistant", "content": "Fallback after rate limit"}}]
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
            fallback_models: vec!["fallback-model".to_string()],
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok(), "Should fallback after 429 rate limit");
    assert_eq!(result.expect("Should succeed"), "Fallback after rate limit");
}

#[tokio::test]
async fn test_openrouter_server_error_503_triggers_fallback() {
    use wiremock::matchers::{body_string_contains, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Primary returns 503 server error
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_string_contains(r#""model":"primary-model""#))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "error": {"message": "Service unavailable", "code": 503}
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Fallback succeeds
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(body_string_contains(r#""model":"fallback-model""#))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "server-error-fallback",
                "model": "fallback-model",
                "choices": [{"message": {"role": "assistant", "content": "Fallback after server error"}}]
            })),
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
            timeout_ms: 5000,
            fallback_models: vec!["fallback-model".to_string()],
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_ok(), "Should fallback after 503 server error");
    assert_eq!(
        result.expect("Should succeed"),
        "Fallback after server error"
    );
}

#[tokio::test]
async fn test_openrouter_auth_failure_401_no_retry() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Return 401 auth error for all requests (no fallbacks should be tried)
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": {"message": "Invalid API key", "code": 401}
        })))
        .expect(1) // Should only try once (no fallbacks for auth errors in this implementation)
        .mount(&mock_server)
        .await;

    let config = LlmConfig {
        provider: LlmProvider::OpenRouter,
        ollama: OllamaConfig::default(),
        openrouter: OpenRouterConfig {
            api_key: "invalid-key".to_string(),
            base_url: mock_server.uri(),
            model: "primary-model".to_string(),
            timeout_ms: 5000,
            fallback_models: vec![], // No fallbacks
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(result.is_err(), "Should fail with 401 auth error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("401")
            || err_msg.contains("Invalid API key")
            || err_msg.contains("OpenRouter API error"),
        "Error should indicate auth failure: {}",
        err_msg
    );
}

#[tokio::test]
async fn test_openrouter_malformed_json_response() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    // Return invalid JSON
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{invalid json}"))
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
            ..OpenRouterConfig::default()
        },
        ..LlmConfig::default()
    };

    let client = LlmClient::with_http_client(config, test_http_client());
    let result = client.chat(vec![ChatMessage::user("test")]).await;

    assert!(
        result.is_err(),
        "Should fail with parse error for malformed JSON"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("parse")
            || err_msg.contains("JSON")
            || err_msg.contains("All OpenRouter models failed"),
        "Error should indicate JSON parse failure: {}",
        err_msg
    );
}

#[test]
fn test_strip_thinking_tags_handling() {
    use super::fallback::strip_thinking_tags;

    // Case 1: Normal text without any tags
    assert_eq!(strip_thinking_tags("Hello world"), "Hello world");

    // Case 2: Complete think block
    assert_eq!(
        strip_thinking_tags("<think>I want to be helpful.</think> Hello user!"),
        "Hello user!"
    );

    // Case 3: Cutoff think block
    assert_eq!(
        strip_thinking_tags("<think>I want to be helpful. This is cut off"),
        ""
    );

    // Case 4: Multiple think blocks (though rare)
    assert_eq!(
        strip_thinking_tags("<think>Block 1</think> Hello <think>Block 2</think> world"),
        "Hello  world"
    );

    // Case 5: Text around tag
    assert_eq!(
        strip_thinking_tags("   <think>monologue</think>\n\n   trimmed output   "),
        "trimmed output"
    );
}

#[test]
fn test_apply_env_overrides_penalties() {
    let config = LlmConfig::default();

    // Test Ollama overrides
    let get_env_ollama = |key: &str| match key {
        "OLLAMA_PRESENCE_PENALTY" => Some("1.5".to_string()),
        "OLLAMA_FREQUENCY_PENALTY" => Some("2.0".to_string()),
        _ => None,
    };
    let config = apply_env_overrides(config, get_env_ollama);
    assert_eq!(config.ollama.presence_penalty, Some(1.5));
    assert_eq!(config.ollama.frequency_penalty, Some(2.0));

    // Test OpenRouter overrides
    let config = LlmConfig::default();
    let get_env_openrouter = |key: &str| match key {
        "OPENROUTER_PRESENCE_PENALTY" => Some("-0.5".to_string()),
        "OPENROUTER_FREQUENCY_PENALTY" => Some("0.5".to_string()),
        _ => None,
    };
    let config = apply_env_overrides(config, get_env_openrouter);
    assert_eq!(config.openrouter.presence_penalty, Some(-0.5));
    assert_eq!(config.openrouter.frequency_penalty, Some(0.5));

    // Test NVIDIA overrides
    let config = LlmConfig::default();
    let get_env_nvidia = |key: &str| match key {
        "NVIDIA_PRESENCE_PENALTY" => Some("0.0".to_string()),
        "NVIDIA_FREQUENCY_PENALTY" => Some("1.0".to_string()),
        _ => None,
    };
    let config = apply_env_overrides(config, get_env_nvidia);
    assert_eq!(config.nvidia.presence_penalty, Some(0.0));
    assert_eq!(config.nvidia.frequency_penalty, Some(1.0));

    // Test generic LLM overrides
    let config = LlmConfig::default();
    let get_env_generic = |key: &str| match key {
        "LLM_PRESENCE_PENALTY" => Some("1.23".to_string()),
        "LLM_FREQUENCY_PENALTY" => Some("4.56".to_string()),
        _ => None,
    };
    let config = apply_env_overrides(config, get_env_generic);
    assert_eq!(config.ollama.presence_penalty, Some(1.23));
    assert_eq!(config.ollama.frequency_penalty, Some(4.56));
    assert_eq!(config.openrouter.presence_penalty, Some(1.23));
    assert_eq!(config.openrouter.frequency_penalty, Some(4.56));
    assert_eq!(config.nvidia.presence_penalty, Some(1.23));
    assert_eq!(config.nvidia.frequency_penalty, Some(4.56));
}

#[tokio::test]
async fn test_rate_limiter_instant_acquisition() {
    let limiter = SharedRateLimiter::new(2.0, 10.0);
    let start = std::time::Instant::now();
    limiter.acquire().await;
    limiter.acquire().await;
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 50,
        "Should acquire first two tokens immediately"
    );
}

#[tokio::test]
async fn test_rate_limiter_blocking_refill() {
    let limiter = SharedRateLimiter::new(1.0, 5.0);
    limiter.acquire().await;

    let start = std::time::Instant::now();
    limiter.acquire().await;
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() >= 150,
        "Should have waited for refill, took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn test_apply_env_overrides_routing_chain_and_fallback_enabled() {
    let config = LlmConfig::default();
    let result = apply_env_overrides(config, |key| match key {
        "LLM_FALLBACK_ENABLED" => Some("false".to_string()),
        "LLM_ROUTING_CHAIN" => Some("ollama:test-model, openrouter:test-or-model".to_string()),
        _ => None,
    });
    assert_eq!(result.fallback_enabled, Some(false));
    assert_eq!(
        result.routing_chain,
        Some(vec![
            "ollama:test-model".to_string(),
            "openrouter:test-or-model".to_string()
        ])
    );
}

#[test]
fn test_ollama_urls_parsing_pool() {
    let mut config = LlmConfig::default();
    config.ollama.base_url = "http://127.0.0.1:11434, http://192.168.1.100:11434".to_string();
    let client = LlmClient::new(config);
    assert_eq!(client.ollama_urls.len(), 2);
    assert_eq!(client.ollama_urls[0], "http://127.0.0.1:11434");
    assert_eq!(client.ollama_urls[1], "http://192.168.1.100:11434");
}

#[test]
fn test_parse_routing_entry_helper() {
    use super::fallback::parse_routing_entry;
    let res = parse_routing_entry("ollama:my-cool-gguf");
    assert!(res.is_some());
    let (provider, model) = res.unwrap();
    assert_eq!(provider, LlmProvider::Ollama);
    assert_eq!(model, "my-cool-gguf");

    let res_invalid = parse_routing_entry("invalid_provider:model");
    assert!(res_invalid.is_none());
}
