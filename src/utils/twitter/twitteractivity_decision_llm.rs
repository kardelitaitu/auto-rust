//! LLM-based decision engine for Twitter engagement.
//!
//! Uses Qwen-Turbo via Alibaba Cloud for smart engagement decisions.
//! Analyzes tweet content and replies to determine engagement quality.

use super::twitteractivity_decision::{
    DecisionEngine, EngagementDecision, EngagementLevel, TweetContext,
};
use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// LLM-powered decision engine
pub struct LLMEngine {
    api_url: String,
    api_key: String,
    model: String,
    timeout_ms: u64,
    client: Client,
}

/// LLM API request structure
#[derive(Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

/// LLM API response structure
#[derive(Deserialize)]
struct LlmResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

/// Parsed decision from LLM JSON output
#[derive(Deserialize, Debug)]
struct LlmDecision {
    score: i32,
    level: String,
    reason: String,
    multiplier: f64,
    confidence: f64,
}

impl LLMEngine {
    /// Create new LLM engine with Qwen-Turbo defaults
    pub fn new(api_key: String) -> Self {
        Self {
            api_url: "https://dashscope-intl.aliyuncs.com/api/v1/services/aigc/text-generation/generation".to_string(),
            api_key,
            model: "qwen-turbo".to_string(),
            timeout_ms: 5000,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create with custom configuration
    pub fn with_config(api_url: String, api_key: String, model: String) -> Self {
        Self {
            api_url,
            api_key,
            model,
            timeout_ms: 5000,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Build system prompt for engagement decisions
    fn build_system_prompt() -> String {
        r#"You are an engagement decision engine for Twitter/X. Analyze tweets and replies to decide engagement intensity.

Respond ONLY with valid JSON in this format:
{
  "score": 0-100,
  "level": "Skip|Low|Medium|High",
  "reason": "one sentence",
  "multiplier": 0.0-3.0,
  "confidence": 0.0-1.0,
  "actions": ["quote|reply|follow|bookmark|like|none"]
}

Rules:
- Skip (0-30, multiplier 0.0): Spam, tragedy, negativity, off-topic
- Low (31-50, multiplier 0.5-0.8): Generic, low-effort, limited value
- Medium (51-75, multiplier 1.0-1.3): Good content, worth engaging
- High (76-100, multiplier 1.5-2.0): Excellent, highly engaging

CRITICAL:
- NEVER engage with death/grief/tragedy posts (score 0-10, multiplier 0.0)
- NEVER engage with crypto/NFT spam (score 0-10, multiplier 0.0)
- Use replies to gauge community reception"#.to_string()
    }

    /// Build user prompt from tweet context
    fn build_user_prompt(&self, ctx: &TweetContext) -> String {
        let mut prompt = format!("TWEET: \"{}\"\nAUTHOR: @{}\n", ctx.text, ctx.author);

        // Add replies if available (max 5)
        if !ctx.replies.is_empty() {
            prompt.push_str("REPLIES:\n");
            for reply in ctx.replies.iter().take(5) {
                prompt.push_str(&format!("- {}\n", reply));
            }
        }

        // Add tone context
        prompt.push_str(
            "\nTONE: \"Casual tech enthusiast, friendly, asks questions, doesn't fake expertise\"",
        );

        // Add tweet metadata
        prompt.push_str(&format!("\nTweet age: {}\n", ctx.tweet_age));
        prompt.push_str(&format!("Topic alignment: {}\n", ctx.topic_alignment));

        // Final prompt instruction
        prompt.push_str("\nDECIDE ACTION AND GENERATE CONTENT:");

        prompt
    }

    /// Call LLM API and parse response
    async fn call_llm(&self, ctx: &TweetContext) -> Result<LlmDecision> {
        let request = LlmRequest {
            model: self.model.clone(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: Self::build_system_prompt(),
                },
                Message {
                    role: "user".to_string(),
                    content: self.build_user_prompt(ctx),
                },
            ],
            temperature: 0.3, // Low temperature for consistent decisions
            max_tokens: 200,
        };

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error: {} - {}", status, text);
        }

        let llm_response: LlmResponse = response.json().await?;
        let content = &llm_response.choices[0].message.content;

        // Parse JSON from response
        let decision: LlmDecision = serde_json::from_str(content).map_err(|e| {
            anyhow::anyhow!("Failed to parse LLM JSON: {} - Content: {}", e, content)
        })?;

        Ok(decision)
    }

    /// Convert LLM level string to EngagementLevel
    fn parse_level(&self, level: &str) -> EngagementLevel {
        match level.to_lowercase().as_str() {
            "skip" | "none" => EngagementLevel::None,
            "low" | "minimal" => EngagementLevel::Minimal,
            "medium" => EngagementLevel::Medium,
            "high" | "full" => EngagementLevel::Full,
            _ => EngagementLevel::None, // Default to skip on unknown
        }
    }
}

#[async_trait]
impl DecisionEngine for LLMEngine {
    fn name(&self) -> &'static str {
        "llm"
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty() && !self.api_url.is_empty()
    }

    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // Check if available
        if !self.is_available() {
            warn!("LLMEngine not available (no API key), returning neutral decision");
            return EngagementDecision {
                level: EngagementLevel::Medium,
                score: 50,
                reason: "LLM unavailable - neutral fallback".to_string(),
                multiplier: 1.0,
                confidence: 0.5,
            };
        }

        info!(
            "LLMEngine: Analyzing tweet from @{} with {} replies",
            ctx.author,
            ctx.replies.len()
        );

        // Call LLM with timeout
        match tokio::time::timeout(Duration::from_millis(self.timeout_ms), self.call_llm(ctx)).await
        {
            Ok(Ok(decision)) => {
                info!(
                    "LLMEngine: score={}, level={}, multiplier={:.2}, confidence={:.2}",
                    decision.score, decision.level, decision.multiplier, decision.confidence
                );

                EngagementDecision {
                    level: self.parse_level(&decision.level),
                    score: decision.score.clamp(0, 100),
                    reason: decision.reason,
                    multiplier: decision.multiplier.clamp(0.0, 3.0),
                    confidence: decision.confidence.clamp(0.0, 1.0),
                }
            }
            Ok(Err(e)) => {
                warn!("LLMEngine error: {}, falling back to neutral", e);
                EngagementDecision {
                    level: EngagementLevel::Medium,
                    score: 50,
                    reason: format!("LLM error fallback: {}", e),
                    multiplier: 1.0,
                    confidence: 0.5,
                }
            }
            Err(_) => {
                warn!(
                    "LLMEngine timeout after {}ms, falling back",
                    self.timeout_ms
                );
                EngagementDecision {
                    level: EngagementLevel::Medium,
                    score: 50,
                    reason: "LLM timeout - using fallback".to_string(),
                    multiplier: 1.0,
                    confidence: 0.5,
                }
            }
        }
    }
}

impl Default for LLMEngine {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use crate::utils::twitter::twitteractivity_state::{SentimentTemplates, TaskConfig};

    fn test_task_config() -> TaskConfig {
        TaskConfig {
            duration_ms: 120_000,
            candidate_count: 5,
            thread_depth: 3,
            max_actions_per_scan: 2,
            scroll_count: 12,
            weights: None,
            llm_enabled: true,
            smart_decision_enabled: true,
            sentiment_templates: SentimentTemplates::default(),
            enhanced_sentiment_enabled: true,
            dry_run_actions: false,
        }
    }

    fn test_context() -> TweetContext {
        TweetContext {
            tweet_id: "tweet-1".to_string(),
            text: "New AI dev tool launch with great docs and code examples".to_string(),
            author: "tech_author".to_string(),
            replies: vec![
                "First reply".to_string(),
                "Second reply".to_string(),
                "Third reply".to_string(),
                "Fourth reply".to_string(),
                "Fifth reply".to_string(),
                "Sixth reply should be truncated".to_string(),
            ],
            persona: PersonaWeights {
                like_prob: 0.3,
                retweet_prob: 0.1,
                quote_prob: 0.8,
                follow_prob: 0.05,
                reply_prob: 0.7,
                bookmark_prob: 0.2,
                thread_dive_prob: 0.1,
                interest_multiplier: 1.0,
            },
            task_config: test_task_config(),
            tweet_age: "Recent".to_string(),
            topic_alignment: "High".to_string(),
        }
    }

    #[test]
    fn test_new_and_with_config_override_defaults() {
        let engine = LLMEngine::new("secret-key".to_string());
        assert!(engine.is_available());
        assert_eq!(
            engine.api_url,
            "https://dashscope-intl.aliyuncs.com/api/v1/services/aigc/text-generation/generation"
        );
        assert_eq!(engine.model, "qwen-turbo");
        assert_eq!(engine.timeout_ms, 5000);

        let configured = LLMEngine::with_config(
            "https://example.invalid/v1/chat".to_string(),
            "api-key-2".to_string(),
            "custom-model".to_string(),
        );
        assert_eq!(configured.api_url, "https://example.invalid/v1/chat");
        assert_eq!(configured.api_key, "api-key-2");
        assert_eq!(configured.model, "custom-model");
        assert_eq!(configured.timeout_ms, 5000);
    }

    #[test]
    fn test_parse_level_maps_known_and_unknown_values() {
        let engine = LLMEngine::new("key".to_string());

        assert_eq!(engine.parse_level("skip"), EngagementLevel::None);
        assert_eq!(engine.parse_level("none"), EngagementLevel::None);
        assert_eq!(engine.parse_level("low"), EngagementLevel::Minimal);
        assert_eq!(engine.parse_level("minimal"), EngagementLevel::Minimal);
        assert_eq!(engine.parse_level("medium"), EngagementLevel::Medium);
        assert_eq!(engine.parse_level("high"), EngagementLevel::Full);
        assert_eq!(engine.parse_level("full"), EngagementLevel::Full);
        assert_eq!(engine.parse_level("mystery"), EngagementLevel::None);
    }

    #[test]
    fn test_build_system_prompt_includes_schema_and_safety_rules() {
        let prompt = LLMEngine::build_system_prompt();
        assert!(prompt.contains("Respond ONLY with valid JSON"));
        assert!(prompt.contains("death/grief/tragedy"));
        assert!(prompt.contains("crypto/NFT spam"));
        assert!(prompt.contains("quote|reply|follow|bookmark|like|none"));
    }

    #[test]
    fn test_build_user_prompt_limits_replies_and_includes_context() {
        let engine = LLMEngine::new("key".to_string());
        let ctx = test_context();
        let prompt = engine.build_user_prompt(&ctx);

        assert!(prompt.contains("TWEET: \"New AI dev tool launch"));
        assert!(prompt.contains("AUTHOR: @tech_author"));
        assert!(prompt.contains("REPLIES:"));
        assert!(prompt.contains("- First reply"));
        assert!(prompt.contains("- Fifth reply"));
        assert!(!prompt.contains("Sixth reply should be truncated"));
        assert!(prompt.contains(
            "TONE: \"Casual tech enthusiast, friendly, asks questions, doesn't fake expertise\""
        ));
        assert!(prompt.contains("Tweet age: Recent"));
        assert!(prompt.contains("Topic alignment: High"));
        assert!(prompt.ends_with("DECIDE ACTION AND GENERATE CONTENT:"));
    }

    #[test]
    fn test_decide_fallback_is_neutral_when_unavailable() {
        let engine = LLMEngine::new(String::new());
        assert!(!engine.is_available());
    }
}
