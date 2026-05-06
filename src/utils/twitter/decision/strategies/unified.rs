//! Unified decision + content generation strategy.
//!
//! Makes a SINGLE LLM call that returns both engagement decision and
//! generated content (reply or quote). Ported from `twitteractivity_decision_unified.rs`.

use crate::utils::twitter::decision::strategies::DecisionStrategyImpl;
use crate::utils::twitter::decision::types::{
    DecisionStrategy, EngagementDecision, EngagementLevel, TweetContext,
};
use async_trait::async_trait;
use log::{error, info, warn};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

/// Unified analysis response - decision + content in one struct
#[derive(Debug, Clone, Deserialize)]
pub struct UnifiedAnalysis {
    /// Engagement score 0-100
    pub score: i32,
    /// Engagement level
    pub level: String,
    /// Brief explanation of decision
    pub reason: String,
    /// Confidence 0.0-1.0
    pub confidence: f64,
    /// Single recommended action: quote/reply/follow/bookmark/like
    pub actions: String,
    /// Whether to engage at all
    pub engage: bool,
    /// Generated content (for quote or reply), null otherwise
    #[allow(dead_code)]
    pub reply: Option<String>,
}

impl UnifiedAnalysis {
    /// Create a skip response for safety triggers
    pub fn skip(reason: &str) -> Self {
        Self {
            score: 0,
            level: "Skip".to_string(),
            reason: reason.to_string(),
            confidence: 0.95,
            actions: "none".to_string(),
            engage: false,
            reply: None,
        }
    }

    /// Convert to EngagementDecision for trait compatibility
    pub fn to_engagement_decision(&self) -> EngagementDecision {
        let level = match self.level.as_str() {
            "Full" => EngagementLevel::Full,
            "Medium" => EngagementLevel::Medium,
            "Minimal" => EngagementLevel::Minimal,
            _ => EngagementLevel::None,
        };

        let multiplier = match level {
            EngagementLevel::Full => 1.5,
            EngagementLevel::Medium => 1.0,
            EngagementLevel::Minimal => 0.5,
            EngagementLevel::None => 0.0,
        };

        EngagementDecision {
            level,
            score: self.score,
            reason: self.reason.clone(),
            multiplier,
            confidence: self.confidence,
        }
    }
}

/// Unified strategy - single LLM call for decision + content
pub(crate) struct UnifiedStrategy {
    api_key: String,
    api_url: String,
    model: String,
    timeout_ms: u64,
    client: Client,
}

impl UnifiedStrategy {
    /// Create new unified strategy with Qwen-Turbo defaults
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            api_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions"
                .to_string(),
            model: "qwen-turbo".to_string(),
            timeout_ms: 5000,
            client: Client::new(),
        }
    }

    /// Build system prompt for unified analysis
    fn build_system_prompt(&self) -> String {
        r#"You are a social media engagement assistant. Analyze tweets and output a COMPLETE engagement strategy.

YOUR OUTPUT must include:
1. SHOULD we engage? (score, level, confidence)
2. For QUOTE or REPLY, generate appropriate content

ACTION RULES:
- "Follow" only if: new account + high quality + aligned interests
- "Bookmark" only if: educational/reference content
- "Like" is the minimal engagement action

SAFETY (override everything - set engage: false if any match):
- Tragedy, grief, death, personal loss
- Crypto scams, "DM for opportunity", excessive promotion
- Divisive politics, hate speech, harassment

CONTENT GENERATION RULES:
- For "reply": Respond directly to the main tweet (not reply threads)
- For "quote": Add commentary that adds value, not just "+1" or emojis
- Match the persona tone provided
- Reference specific details from the tweet
- Maximum 280 characters

Respond ONLY with valid JSON matching this exact schema:
{
  "score": <0-100 integer>,
  "level": "Skip|Minimal|Medium|Full",
  "reason": "<1 sentence explanation>",
  "confidence": <0.0-1.0>,
  "actions": "quote|reply|follow|bookmark|like|none",
  "engage": <true|false>,
  "reply": "<generated content or null>"
}"#.to_string()
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

        // Add persona tone
        let tone = self.infer_persona_tone(&ctx.persona);
        prompt.push_str(&format!("\nTONE: \"{}\"\n", tone));

        // Add context hints
        prompt.push_str(&format!(
            "CONTEXT:\n- Tweet age: Recent\n- Topic alignment: {}\n",
            if self.is_topic_aligned(&ctx.text) {
                "High"
            } else {
                "Medium"
            }
        ));

        prompt.push_str("\nDECIDE ACTION AND GENERATE CONTENT:");
        prompt
    }

    /// Infer persona tone description
    fn infer_persona_tone(
        &self,
        persona: &crate::utils::twitter::twitteractivity_persona::PersonaWeights,
    ) -> String {
        if persona.reply_prob > 0.5 {
            "Casual tech enthusiast, friendly, asks questions, doesn't fake expertise"
        } else if persona.like_prob > 0.7 {
            "Professional, reserved, engages with quality content only"
        } else {
            "Balanced engagement style"
        }
        .to_string()
    }

    /// Check if topic is aligned with tech/dev focus
    fn is_topic_aligned(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        let tech_keywords = ["ai", "code", "dev", "startup", "tech", "product", "app"];
        tech_keywords.iter().any(|kw| text_lower.contains(kw))
    }

    /// Call LLM API
    async fn call_llm(
        &self,
        ctx: &TweetContext,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(ctx);

        let request_body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": user_prompt
                }
            ],
            "temperature": 0.7,
            "max_tokens": 500,
            "response_format": {
                "type": "json_object"
            }
        });

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .timeout(Duration::from_millis(self.timeout_ms))
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("LLM API error: {} - {}", status, error_text).into());
        }

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or("Invalid LLM response format")?;

        Ok(content.to_string())
    }

    /// Check for safety triggers in content
    fn check_safety(&self, ctx: &TweetContext) -> Option<String> {
        let text_lower = ctx.text.to_lowercase();
        let combined = format!("{} {}", text_lower, ctx.replies.join(" ").to_lowercase());

        // Tragedy keywords
        let tragedy = [
            "died",
            "death",
            "passed away",
            "funeral",
            "grief",
            "tragedy",
            "killed",
            "murdered",
            "suicide",
        ];
        if tragedy.iter().any(|kw| combined.contains(kw)) {
            return Some("Safety: tragedy/grief detected".to_string());
        }

        // Crypto scam patterns
        let crypto_scam = [
            "dm me",
            "dm for",
            "check my bio",
            "guaranteed profit",
            "100x gem",
            "airdrop",
        ];
        if crypto_scam.iter().any(|kw| combined.contains(kw)) {
            return Some("Safety: potential crypto scam".to_string());
        }

        // Excessive hashtags
        let hashtag_count = text_lower.matches('#').count();
        if hashtag_count > 5 {
            return Some("Safety: excessive hashtags".to_string());
        }

        None
    }
}

#[async_trait]
impl DecisionStrategyImpl for UnifiedStrategy {
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // 1. Pre-flight safety check (fast path)
        if let Some(safety_reason) = self.check_safety(ctx) {
            info!("UnifiedStrategy: Safety trigger - {}", safety_reason);
            return UnifiedAnalysis::skip(&safety_reason).to_engagement_decision();
        }

        // 2. Call LLM for unified analysis
        let analysis = match self.call_llm(ctx).await {
            Ok(response) => match serde_json::from_str::<UnifiedAnalysis>(&response) {
                Ok(a) => a,
                Err(e) => {
                    error!("UnifiedStrategy: JSON parse error: {}", e);
                    UnifiedAnalysis::skip("Parse error")
                }
            },
            Err(e) => {
                warn!("UnifiedStrategy: LLM call failed: {}", e);
                UnifiedAnalysis::skip("LLM unavailable")
            }
        };

        // 3. Log decision
        info!(
            "UnifiedStrategy: score={}, level={}, action={}, engage={}",
            analysis.score, analysis.level, analysis.actions, analysis.engage
        );

        analysis.to_engagement_decision()
    }

    fn strategy_type(&self) -> DecisionStrategy {
        DecisionStrategy::Unified
    }

    fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn name(&self) -> &'static str {
        "unified"
    }
}
