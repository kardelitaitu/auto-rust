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
