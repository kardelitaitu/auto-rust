//! LLM-based decision strategy.
//!
//! Uses Qwen-Turbo via Alibaba Cloud for smart engagement decisions.
//! Ported from `twitteractivity_decision_llm.rs`.

use crate::utils::twitter::decision::strategies::DecisionStrategyImpl;
use crate::utils::twitter::decision::types::{
    DecisionStrategy, EngagementDecision, EngagementLevel, TweetContext,
};
use async_trait::async_trait;
use log::{info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// LLM-powered decision strategy.
pub(crate) struct LlmStrategy {
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

impl LlmStrategy {
    /// Create new LLM strategy with Qwen-Turbo defaults
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
    #[allow(clippy::unused_self)]
    fn build_user_prompt(&self, ctx: &TweetContext) -> String {
        let mut prompt = format!("TWEET: \"{}\"\nAUTHOR: @{}\n", ctx.text, ctx.author);

        // Add replies if available (max 5)
        if !ctx.replies.is_empty() {
            prompt.push_str("REPLIES:\n");
            for reply in ctx.replies.iter().take(5) {
                prompt.push_str(&format!("- {reply}\n"));
            }
        }

        // Add tone context
        prompt.push_str(
            "\nTONE: \"Casual tech enthusiast, friendly, asks questions, doesn't fake expertise\"",
        );

        // Add tweet metadata
        prompt.push_str(&format!("\nTweet age: {}\n", ctx.tweet_age));

        // Final prompt instruction
        prompt.push_str("\nDECIDE ACTION AND GENERATE CONTENT:");

        prompt
    }

    /// Call LLM API and parse response
    async fn call_llm(&self, ctx: &TweetContext) -> anyhow::Result<LlmDecision> {
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
            anyhow::bail!("LLM API error: {status} - {text}");
        }

        let llm_response: LlmResponse = response.json().await?;
        let content = &llm_response.choices[0].message.content;

        // Parse JSON from response
        let decision: LlmDecision = serde_json::from_str(content)
            .map_err(|e| anyhow::anyhow!("Failed to parse LLM JSON: {e} - Content: {content}"))?;

        Ok(decision)
    }

    /// Convert LLM level string to `EngagementLevel`
    #[allow(clippy::unused_self)]
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
impl DecisionStrategyImpl for LlmStrategy {
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // Check if available
        if !self.is_available() {
            warn!("LlmStrategy not available (no API key), returning neutral decision");
            return EngagementDecision {
                level: EngagementLevel::Medium,
                score: 50,
                reason: "LLM unavailable - neutral fallback".to_string(),
                multiplier: 1.0,
                confidence: 0.5,
            };
        }

        info!(
            "LlmStrategy: Analyzing tweet from @{} with {} replies",
            ctx.author,
            ctx.replies.len()
        );

        // Call LLM with timeout
        match tokio::time::timeout(Duration::from_millis(self.timeout_ms), self.call_llm(ctx)).await
        {
            Ok(Ok(decision)) => {
                info!(
                    "LlmStrategy: score={}, level={}, multiplier={:.2}, confidence={:.2}",
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
                warn!("LlmStrategy error: {e}, falling back to neutral");
                EngagementDecision {
                    level: EngagementLevel::Medium,
                    score: 50,
                    reason: format!("LLM error fallback: {e}"),
                    multiplier: 1.0,
                    confidence: 0.5,
                }
            }
            Err(_) => {
                warn!(
                    "LlmStrategy timeout after {}ms, falling back",
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

    fn strategy_type(&self) -> DecisionStrategy {
        DecisionStrategy::Llm
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty() && !self.api_url.is_empty()
    }

    fn name(&self) -> &'static str {
        "llm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use crate::utils::twitter::twitteractivity_state::TaskConfig;

    fn make_strategy() -> LlmStrategy {
        LlmStrategy::new("test-key".to_string())
    }

    fn default_ctx() -> TweetContext {
        TweetContext {
            tweet_id: "1".to_string(),
            text: "Hello world".to_string(),
            author: "user".to_string(),
            replies: vec![],
            persona: PersonaWeights::default(),
            task_config: TaskConfig::default(),
            tweet_age: "recent".to_string(),
        }
    }

    // ========================================================================
    // Constructor Tests
    // ========================================================================

    #[test]
    fn test_new_sets_defaults() {
        let s = make_strategy();
        assert!(s.api_url.contains("dashscope-intl"));
        assert_eq!(s.model, "qwen-turbo");
        assert_eq!(s.timeout_ms, 5000);
        assert_eq!(s.api_key, "test-key");
    }

    #[test]
    fn test_new_empty_key() {
        let s = LlmStrategy::new(String::new());
        assert!(s.api_key.is_empty());
        assert_eq!(s.model, "qwen-turbo");
    }

    // ========================================================================
    // build_system_prompt Tests
    // ========================================================================

    #[test]
    fn test_build_system_prompt_contains_keywords() {
        let prompt = LlmStrategy::build_system_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("engagement decision engine"));
        assert!(prompt.contains("score"));
        assert!(prompt.contains("level"));
        assert!(prompt.contains("multiplier"));
        assert!(prompt.contains("confidence"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_build_system_prompt_contains_skip_low_medium_high() {
        let prompt = LlmStrategy::build_system_prompt();
        assert!(prompt.contains("Skip"));
        assert!(prompt.contains("Low"));
        assert!(prompt.contains("Medium"));
        assert!(prompt.contains("High"));
    }

    #[test]
    fn test_build_system_prompt_contains_critical_rules() {
        let prompt = LlmStrategy::build_system_prompt();
        assert!(prompt.contains("death"));
        assert!(prompt.contains("crypto"));
        assert!(prompt.contains("NEVER engage"));
    }

    // ========================================================================
    // build_user_prompt Tests
    // ========================================================================

    #[test]
    fn test_build_user_prompt_includes_text_and_author() {
        let s = make_strategy();
        let ctx = default_ctx();
        let prompt = s.build_user_prompt(&ctx);
        assert!(prompt.contains("Hello world"));
        assert!(prompt.contains("@user"));
    }

    #[test]
    fn test_build_user_prompt_includes_tone_and_tweet_age() {
        let s = make_strategy();
        let ctx = default_ctx();
        let prompt = s.build_user_prompt(&ctx);
        assert!(prompt.contains("Casual tech enthusiast"));
        assert!(prompt.contains("Tweet age: recent"));
        assert!(prompt.contains("DECIDE ACTION"));
    }

    #[test]
    fn test_build_user_prompt_no_replies_no_replies_section() {
        let s = make_strategy();
        let ctx = default_ctx();
        let prompt = s.build_user_prompt(&ctx);
        assert!(!prompt.contains("REPLIES:"));
    }

    #[test]
    fn test_build_user_prompt_with_replies() {
        let s = make_strategy();
        let ctx = TweetContext {
            replies: vec!["Great post!".to_string(), "I agree".to_string()],
            ..default_ctx()
        };
        let prompt = s.build_user_prompt(&ctx);
        assert!(prompt.contains("REPLIES:"));
        assert!(prompt.contains("- Great post!"));
        assert!(prompt.contains("- I agree"));
    }

    #[test]
    fn test_build_user_prompt_limits_replies_to_five() {
        let s = make_strategy();
        let many_replies: Vec<String> = (0..10).map(|i| format!("reply {i}")).collect();
        let ctx = TweetContext {
            replies: many_replies,
            ..default_ctx()
        };
        let prompt = s.build_user_prompt(&ctx);
        assert!(prompt.contains("- reply 0"));
        assert!(prompt.contains("- reply 4"));
        assert!(!prompt.contains("- reply 5"));
    }

    // ========================================================================
    // parse_level Tests
    // ========================================================================

    #[test]
    fn test_parse_level_skip() {
        let s = make_strategy();
        assert_eq!(s.parse_level("skip"), EngagementLevel::None);
        assert_eq!(s.parse_level("Skip"), EngagementLevel::None);
        assert_eq!(s.parse_level("none"), EngagementLevel::None);
        assert_eq!(s.parse_level("None"), EngagementLevel::None);
    }

    #[test]
    fn test_parse_level_low() {
        let s = make_strategy();
        assert_eq!(s.parse_level("low"), EngagementLevel::Minimal);
        assert_eq!(s.parse_level("Low"), EngagementLevel::Minimal);
        assert_eq!(s.parse_level("minimal"), EngagementLevel::Minimal);
    }

    #[test]
    fn test_parse_level_medium() {
        let s = make_strategy();
        assert_eq!(s.parse_level("medium"), EngagementLevel::Medium);
        assert_eq!(s.parse_level("Medium"), EngagementLevel::Medium);
    }

    #[test]
    fn test_parse_level_high() {
        let s = make_strategy();
        assert_eq!(s.parse_level("high"), EngagementLevel::Full);
        assert_eq!(s.parse_level("High"), EngagementLevel::Full);
        assert_eq!(s.parse_level("full"), EngagementLevel::Full);
    }

    #[test]
    fn test_parse_level_unknown_defaults_to_none() {
        let s = make_strategy();
        assert_eq!(s.parse_level(""), EngagementLevel::None);
        assert_eq!(s.parse_level("invalid"), EngagementLevel::None);
        assert_eq!(s.parse_level("garbage"), EngagementLevel::None);
    }

    // ========================================================================
    // is_available Tests
    // ========================================================================

    #[test]
    fn test_is_available_with_key() {
        let s = make_strategy();
        assert!(s.is_available());
    }

    #[test]
    fn test_is_available_without_key() {
        let s = LlmStrategy::new(String::new());
        assert!(!s.is_available());
    }

    #[test]
    fn test_is_available_empty_url() {
        let mut s = make_strategy();
        s.api_url.clear();
        assert!(!s.is_available());
    }

    // ========================================================================
    // Identity Tests
    // ========================================================================

    #[test]
    fn test_name() {
        let s = make_strategy();
        assert_eq!(s.name(), "llm");
    }

    #[test]
    fn test_strategy_type() {
        let s = make_strategy();
        assert_eq!(s.strategy_type(), DecisionStrategy::Llm);
    }

    // ========================================================================
    // decide Tests (sync-only paths)
    // ========================================================================

    #[tokio::test]
    async fn test_decide_unavailable_returns_neutral() {
        let s = LlmStrategy::new(String::new());
        let ctx = default_ctx();
        let decision = s.decide(&ctx).await;
        assert_eq!(decision.level, EngagementLevel::Medium);
        assert_eq!(decision.score, 50);
        assert!((decision.confidence - 0.5).abs() < 0.01);
        assert_eq!(decision.multiplier, 1.0);
        assert!(decision.reason.contains("unavailable"));
    }
}
