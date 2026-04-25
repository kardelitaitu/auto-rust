use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: Option<ChatMessage>,
    pub done: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ChatChoice {
    WithMessage { message: ChatMessage },
    WithContent { content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub choices: Option<Vec<ChatChoice>>,
    pub usage: Option<Usage>,
    pub error: Option<OpenRouterError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterError {
    pub message: String,
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    #[default]
    Ollama,
    OpenRouter,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmConfig {
    pub provider: LlmProvider,
    pub ollama: OllamaConfig,
    pub openrouter: OpenRouterConfig,
}

impl LlmConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".into(),
            model: "llama3.2:3b".into(),
            timeout_ms: 120000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
    /// Fallback models to try if primary fails (timeout or error)
    #[serde(default)]
    pub fallback_models: Vec<String>,
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://openrouter.ai/api/v1".into(),
            model: "anthropic/claude-3-haiku".into(),
            timeout_ms: 60000,
            fallback_models: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("You are helpful");
        assert_eq!(msg.role, "system");
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("Sure, I can help");
        assert_eq!(msg.role, "assistant");
    }

    #[test]
    fn test_chat_request_defaults() {
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
        };
        assert_eq!(request.model, "llama3");
        assert!(request.messages.is_empty());
    }

    #[test]
    fn test_llm_config_defaults() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::Ollama);
    }

    #[test]
    fn test_ollama_config_defaults() {
        let config = OllamaConfig::default();
        assert_eq!(config.base_url, "http://localhost:11434");
        assert_eq!(config.model, "llama3.2:3b");
        assert_eq!(config.timeout_ms, 120000);
    }

    #[test]
    fn test_openrouter_config_defaults() {
        let config = OpenRouterConfig::default();
        assert_eq!(config.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(config.model, "anthropic/claude-3-haiku");
    }

    #[test]
    fn test_llm_provider_default() {
        let provider = LlmProvider::default();
        assert_eq!(provider, LlmProvider::Ollama);
    }

    #[test]
    fn test_chat_response_parsing() {
        let response = ChatResponse {
            message: Some(ChatMessage::assistant("Response text")),
            done: Some(true),
            error: None,
        };
        assert!(response.done.unwrap());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_chat_message_content_conversion() {
        let msg = ChatMessage::user("123");
        assert_eq!(msg.content, "123");
    }

    #[test]
    fn test_chat_message_clone() {
        let msg = ChatMessage::user("test");
        let cloned = msg.clone();
        assert_eq!(msg.role, cloned.role);
        assert_eq!(msg.content, cloned.content);
    }

    #[test]
    fn test_chat_request_with_temperature() {
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![],
            temperature: Some(0.5),
            max_tokens: None,
        };
        assert_eq!(request.temperature, Some(0.5));
    }

    #[test]
    fn test_chat_request_with_max_tokens() {
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: Some(1024),
        };
        assert_eq!(request.max_tokens, Some(1024));
    }

    #[test]
    fn test_chat_response_with_done_false() {
        let response = ChatResponse {
            message: None,
            done: Some(false),
            error: None,
        };
        assert!(!response.done.unwrap());
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
    fn test_openrouter_response_with_id() {
        let response = OpenRouterResponse {
            id: Some("req-123".to_string()),
            model: None,
            choices: None,
            usage: None,
            error: None,
        };
        assert_eq!(response.id, Some("req-123".to_string()));
    }

    #[test]
    fn test_openrouter_response_with_model() {
        let response = OpenRouterResponse {
            id: None,
            model: Some("claude-3".to_string()),
            choices: None,
            usage: None,
            error: None,
        };
        assert_eq!(response.model, Some("claude-3".to_string()));
    }

    #[test]
    fn test_openrouter_error_with_code() {
        let error = OpenRouterError {
            message: "Rate limit".to_string(),
            code: Some(429),
        };
        assert_eq!(error.code, Some(429));
    }

    #[test]
    fn test_usage_with_tokens() {
        let usage = Usage {
            prompt_tokens: Some(10),
            completion_tokens: Some(20),
            total_tokens: Some(30),
        };
        assert_eq!(usage.total_tokens, Some(30));
    }

    #[test]
    fn test_usage_partial() {
        let usage = Usage {
            prompt_tokens: Some(10),
            completion_tokens: None,
            total_tokens: None,
        };
        assert_eq!(usage.prompt_tokens, Some(10));
    }

    #[test]
    fn test_chat_choice_with_message_variant() {
        let choice = ChatChoice::WithMessage {
            message: ChatMessage::user("test"),
        };
        if let ChatChoice::WithMessage { message } = choice {
            assert_eq!(message.role, "user");
        }
    }

    #[test]
    fn test_chat_choice_with_content_variant() {
        let choice = ChatChoice::WithContent {
            content: "direct".to_string(),
        };
        if let ChatChoice::WithContent { content } = choice {
            assert_eq!(content, "direct");
        }
    }

    #[test]
    fn test_llm_config_new() {
        let config = LlmConfig::new();
        assert_eq!(config.provider, LlmProvider::Ollama);
    }

    #[test]
    fn test_llm_config_custom() {
        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig::default(),
        };
        assert_eq!(config.provider, LlmProvider::OpenRouter);
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
            api_key: "key".to_string(),
            base_url: "https://api.example.com".to_string(),
            model: "gpt-4".to_string(),
            timeout_ms: 90000,
            fallback_models: vec!["gpt-3.5-turbo".to_string()],
        };
        assert_eq!(config.api_key, "key");
        assert_eq!(config.fallback_models.len(), 1);
    }

    #[test]
    fn test_openrouter_fallback_models_empty_by_default() {
        let config = OpenRouterConfig::default();
        assert!(config.fallback_models.is_empty());
    }

    #[test]
    fn test_openrouter_with_multiple_fallbacks() {
        let fallbacks = vec![
            "nvidia/nemotron-3-super-120b-a12b:free".to_string(),
            "minimax/minimax-m2.5:free".to_string(),
            "nvidia/nemotron-3-nano-30b-a3b:free".to_string(),
            "openrouter/free".to_string(),
        ];
        let config = OpenRouterConfig {
            api_key: "test-key".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model: "tencent/hy3-preview:free".to_string(),
            timeout_ms: 60000,
            fallback_models: fallbacks.clone(),
        };
        assert_eq!(config.fallback_models.len(), 4);
        assert_eq!(config.fallback_models, fallbacks);
    }

    #[test]
    fn test_llm_config_with_openrouter_fallbacks() {
        let config = LlmConfig {
            provider: LlmProvider::OpenRouter,
            ollama: OllamaConfig::default(),
            openrouter: OpenRouterConfig {
                api_key: "key".to_string(),
                base_url: "https://openrouter.ai/api/v1".to_string(),
                model: "primary-model".to_string(),
                timeout_ms: 60000,
                fallback_models: vec!["fb1".to_string(), "fb2".to_string()],
            },
        };
        assert_eq!(config.openrouter.fallback_models.len(), 2);
    }

    #[test]
    fn test_chat_message_empty_content() {
        let msg = ChatMessage::user("");
        assert_eq!(msg.content, "");
    }

    #[test]
    fn test_chat_request_empty_messages() {
        let request = ChatRequest {
            model: "test".to_string(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
        };
        assert!(request.messages.is_empty());
    }

    #[test]
    fn test_openrouter_response_empty() {
        let response = OpenRouterResponse {
            id: None,
            model: None,
            choices: None,
            usage: None,
            error: None,
        };
        assert!(response.id.is_none());
    }
}
