pub mod client;
pub mod models;
pub mod reply_engine;
pub mod reply_strategies;
pub mod unified_processor;

pub use client::*;
pub use models::*;
pub use reply_engine::*;
pub use reply_strategies::*;
pub use unified_processor::*;

use log::info;

pub struct Llm {
    client: LlmClient,
}

impl Llm {
    pub fn new() -> anyhow::Result<Self> {
        let config = client::create_llm_client_from_config()?;
        let client = LlmClient::new(config);

        info!("LLM client initialized");

        Ok(Self { client })
    }

    pub async fn generate(&self, prompt: &str) -> anyhow::Result<String> {
        let messages = vec![ChatMessage::user(prompt)];
        self.client.chat(messages).await
    }

    pub async fn generate_with_fallback(&self, prompt: &str) -> anyhow::Result<String> {
        let messages = vec![ChatMessage::user(prompt)];
        self.client.chat_with_fallback(messages).await
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String> {
        self.client.chat(messages).await
    }

    pub async fn chat_with_fallback(&self, messages: Vec<ChatMessage>) -> anyhow::Result<String> {
        self.client.chat_with_fallback(messages).await
    }

    pub async fn health_check(&self) -> bool {
        self.client.health_check().await
    }
}

impl Default for Llm {
    fn default() -> Self {
        Self::new().expect("Failed to create LLM client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // ChatMessage Tests
    // =========================================================================

    #[test]
    fn test_chat_message_user_creation() {
        let msg = ChatMessage::user("Hello, world!");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello, world!");
    }

    #[test]
    fn test_chat_message_system_creation() {
        let msg = ChatMessage::system("You are a helpful assistant");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are a helpful assistant");
    }

    #[test]
    fn test_chat_message_assistant_creation() {
        let msg = ChatMessage::assistant("Here's my response");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Here's my response");
    }

    #[test]
    fn test_chat_message_empty_content() {
        let msg = ChatMessage::user("");
        assert_eq!(msg.content, "");
    }

    #[test]
    fn test_chat_message_long_content() {
        let long_content = "a".repeat(10000);
        let msg = ChatMessage::user(&long_content);
        assert_eq!(msg.content.len(), 10000);
    }

    #[test]
    fn test_chat_message_unicode_content() {
        let msg = ChatMessage::user("Hello 世界 🌍");
        assert_eq!(msg.content, "Hello 世界 🌍");
    }

    // =========================================================================
    // Model Struct Tests
    // =========================================================================

    #[test]
    fn test_chat_request_creation() {
        let messages = vec![
            ChatMessage::system("Be helpful"),
            ChatMessage::user("Hello"),
        ];
        let request = ChatRequest {
            model: "gpt-4".to_string(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
        };

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.max_tokens, Some(2048));
    }

    #[test]
    fn test_chat_request_optional_fields() {
        let request = ChatRequest {
            model: "test-model".to_string(),
            messages: vec![ChatMessage::user("test")],
            temperature: None,
            max_tokens: None,
        };

        assert!(request.temperature.is_none());
        assert!(request.max_tokens.is_none());
    }

    #[test]
    fn test_chat_response_creation() {
        let response = ChatResponse {
            message: Some(ChatMessage::assistant("Hello!")),
            done: Some(true),
            error: None,
        };

        assert!(response.message.is_some());
        assert_eq!(response.done, Some(true));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_chat_response_with_error() {
        let response = ChatResponse {
            message: None,
            done: Some(true),
            error: Some("Model not found".to_string()),
        };

        assert!(response.message.is_none());
        assert_eq!(response.error, Some("Model not found".to_string()));
    }

    #[test]
    fn test_llm_provider_default_is_ollama() {
        let provider = LlmProvider::default();
        assert_eq!(provider, LlmProvider::Ollama);
    }

    #[test]
    fn test_llm_provider_equality() {
        assert_eq!(LlmProvider::Ollama, LlmProvider::Ollama);
        assert_eq!(LlmProvider::OpenRouter, LlmProvider::OpenRouter);
        assert_ne!(LlmProvider::Ollama, LlmProvider::OpenRouter);
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert_eq!(config.provider, LlmProvider::Ollama);
    }

    #[test]
    fn test_llm_config_new() {
        let config = LlmConfig::new();
        assert_eq!(config.provider, LlmProvider::Ollama);
    }

    #[test]
    fn test_open_router_error_creation() {
        let error = OpenRouterError {
            message: "Rate limit exceeded".to_string(),
            code: Some(429),
        };
        assert_eq!(error.message, "Rate limit exceeded");
        assert_eq!(error.code, Some(429));
    }

    #[test]
    fn test_open_router_error_without_code() {
        let error = OpenRouterError {
            message: "Unknown error".to_string(),
            code: None,
        };
        assert!(error.code.is_none());
    }

    #[test]
    fn test_usage_creation() {
        let usage = Usage {
            prompt_tokens: Some(100),
            completion_tokens: Some(50),
            total_tokens: Some(150),
        };
        assert_eq!(usage.prompt_tokens, Some(100));
        assert_eq!(usage.completion_tokens, Some(50));
        assert_eq!(usage.total_tokens, Some(150));
    }

    // =========================================================================
    // Serialization Tests
    // =========================================================================

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }

    #[test]
    fn test_chat_message_deserialization() {
        let json = r#"{"role":"assistant","content":"Hi there"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "Hi there");
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            model: "test".to_string(),
            messages: vec![ChatMessage::user("Hello")],
            temperature: Some(0.5),
            max_tokens: Some(100),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"model\":\"test\""));
        assert!(json.contains("\"temperature\":0.5"));
    }

    #[test]
    fn test_llm_provider_serialization() {
        let provider = LlmProvider::OpenRouter;
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, "\"openrouter\"");
    }

    #[test]
    fn test_llm_provider_deserialization() {
        let json = "\"ollama\"";
        let provider: LlmProvider = serde_json::from_str(json).unwrap();
        assert_eq!(provider, LlmProvider::Ollama);
    }

    // =========================================================================
    // ChatChoice Enum Tests
    // =========================================================================

    #[test]
    fn test_chat_choice_with_message() {
        let choice = ChatChoice::WithMessage {
            message: ChatMessage::assistant("Hello"),
        };
        match choice {
            ChatChoice::WithMessage { message } => {
                assert_eq!(message.role, "assistant");
            }
            _ => panic!("Expected WithMessage variant"),
        }
    }

    #[test]
    fn test_chat_choice_with_content() {
        let choice = ChatChoice::WithContent {
            content: "Direct content".to_string(),
        };
        match choice {
            ChatChoice::WithContent { content } => {
                assert_eq!(content, "Direct content");
            }
            _ => panic!("Expected WithContent variant"),
        }
    }
}
