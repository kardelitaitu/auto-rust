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
            model: "llama3.2:latest".into(),
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
