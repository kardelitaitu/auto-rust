//! Data types for sentiment analysis.
//!
//! Extracted from `sentiment/analyzer.rs` — spec 0020.

/// Core sentiment classification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

/// Context around a tweet's thread/conversation.
#[derive(Debug, Clone)]
pub struct ThreadContext {
    pub reply_count: u32,
    pub avg_reply_sentiment: f32,
    pub is_reply: bool,
    pub is_quote: bool,
    pub thread_depth: u32,
    pub conversation_indicators: Vec<ConversationIndicator>,
}

/// Types of conversational patterns detected in thread context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConversationIndicator {
    Agreement,
    Disagreement,
    Question,
    Clarification,
    Humor,
    Sarcasm,
    Support,
    Criticism,
}

/// User reputation metrics extracted from tweet author data.
#[derive(Debug, Clone)]
pub struct UserReputation {
    pub follower_count: u32,
    pub is_verified: bool,
    pub account_age_days: u32,
    pub engagement_rate: f32,
    pub is_influential: bool,
    pub trust_score: f32,
}

/// Temporal factors affecting sentiment interpretation.
#[derive(Debug, Clone)]
pub struct TemporalFactors {
    pub hour_of_day: u8,
    pub day_of_week: u8,
    pub hours_since_post: f32,
    pub is_peak_hour: bool,
    pub trending_bias: f32,
    /// Recency/freshness score (0.0–1.0). 1.0 = just posted, 0.0 = >7 days old.
    pub recency: f32,
}

/// Result of enhanced sentiment analysis with factor breakdown.
#[derive(Debug, Clone)]
pub struct EnhancedSentimentResult {
    pub base_sentiment: Sentiment,
    pub final_sentiment: Sentiment,
    pub base_score: f32,
    pub final_score: f32,
    pub confidence: f32,
    pub score_breakdown: ScoreBreakdown,
}

/// Breakdown of sentiment scores by analysis factor.
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    pub text_score: f32,
    pub emoji_score: f32,
    pub domain_score: f32,
    pub context_score: f32,
    pub reputation_score: f32,
    pub temporal_score: f32,
}

/// Configuration for sentiment analysis strategies.
#[derive(Debug, Clone)]
pub struct SentimentConfig {
    pub use_basic_keywords: bool,
    pub use_context: bool,
    pub use_emoji: bool,
    pub use_domain: bool,
    pub use_llm: bool,
    pub llm_min_confidence: f32,
    pub llm_probability: f32,
}

impl Default for SentimentConfig {
    fn default() -> Self {
        Self {
            use_basic_keywords: true,
            use_context: true,
            use_emoji: true,
            use_domain: true,
            use_llm: false,
            llm_min_confidence: 0.7,
            llm_probability: 0.5,
        }
    }
}

/// Aggregate sentiment statistics for feed scoring.
#[derive(Debug, Clone, Default)]
pub struct SentimentStats {
    pub positive: u32,
    pub neutral: u32,
    pub negative: u32,
}

impl SentimentStats {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, s: Sentiment) {
        match s {
            Sentiment::Positive => self.positive += 1,
            Sentiment::Neutral => self.neutral += 1,
            Sentiment::Negative => self.negative += 1,
        }
    }

    #[must_use]
    pub fn dominant(&self) -> Sentiment {
        if self.positive > self.neutral && self.positive > self.negative {
            Sentiment::Positive
        } else if self.negative > self.neutral && self.negative > self.positive {
            Sentiment::Negative
        } else {
            Sentiment::Neutral
        }
    }

    #[must_use]
    pub fn total(&self) -> u32 {
        self.positive + self.neutral + self.negative
    }
}
