//! NVIDIA-only LLM model types for the bacon pipeline.
//!
//! Simplified from `auto-rust/src/llm/models.rs` — removes Ollama/OpenRouter.

use serde::{Deserialize, Serialize};

/// Chat message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    System,
    Assistant,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// LLM provider.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    Nvidia,
    Ollama,
}

/// LLM API configuration (supports NVIDIA and Ollama providers).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NvidiaConfig {
    pub provider: LlmProvider,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: u32,
}

impl Default for NvidiaConfig {
    fn default() -> Self {
        Self {
            provider: LlmProvider::Nvidia,
            api_key: String::new(),
            base_url: "https://integrate.api.nvidia.com/v1".to_string(),
            model: "meta/llama-3.3-70b-instruct".to_string(),
            timeout_ms: 600000,
            temperature: 1.0,
            top_p: 0.95,
            max_tokens: 16384,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("Be helpful");
        assert_eq!(msg.role, Role::System);
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("Sure");
        assert_eq!(msg.role, Role::Assistant);
    }

    #[test]
    fn test_nvidia_config_defaults() {
        let cfg = NvidiaConfig::default();
        assert!(cfg.base_url.contains("nvidia.com"));
        assert!(cfg.model.contains("llama"));
        assert_eq!(cfg.max_tokens, 16384);
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage::user("test");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"test\""));
    }
}
