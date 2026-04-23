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
}

impl Default for OpenRouterConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://openrouter.ai/api/v1".into(),
            model: "anthropic/claude-3-haiku".into(),
            timeout_ms: 60000,
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
}
