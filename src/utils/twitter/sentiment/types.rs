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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── SentimentStats tests ───────────────────────────────────────────────

    #[test]
    fn test_stats_new_empty() {
        let stats = SentimentStats::new();
        assert_eq!(stats.positive, 0);
        assert_eq!(stats.neutral, 0);
        assert_eq!(stats.negative, 0);
        assert_eq!(stats.total(), 0);
    }

    #[test]
    fn test_stats_add_positive() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        assert_eq!(stats.positive, 1);
        assert_eq!(stats.total(), 1);
    }

    #[test]
    fn test_stats_add_neutral() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Neutral);
        assert_eq!(stats.neutral, 1);
        assert_eq!(stats.total(), 1);
    }

    #[test]
    fn test_stats_add_negative() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Negative);
        assert_eq!(stats.negative, 1);
        assert_eq!(stats.total(), 1);
    }

    #[test]
    fn test_stats_add_multiple() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Neutral);
        stats.add(Sentiment::Negative);
        assert_eq!(stats.positive, 2);
        assert_eq!(stats.neutral, 1);
        assert_eq!(stats.negative, 1);
        assert_eq!(stats.total(), 4);
    }

    #[test]
    fn test_stats_dominant_positive() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Neutral);
        assert_eq!(stats.dominant(), Sentiment::Positive);
    }

    #[test]
    fn test_stats_dominant_negative() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Negative);
        stats.add(Sentiment::Negative);
        stats.add(Sentiment::Neutral);
        assert_eq!(stats.dominant(), Sentiment::Negative);
    }

    #[test]
    fn test_stats_dominant_neutral() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Neutral);
        stats.add(Sentiment::Neutral);
        stats.add(Sentiment::Positive);
        // Neutral is dominant here (2 neutral > 1 positive)
        assert_eq!(stats.dominant(), Sentiment::Neutral);
    }

    #[test]
    fn test_stats_dominant_tie_positive_negative() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Negative);
        // Tie between positive and negative — should return Neutral
        assert_eq!(
            stats.dominant(),
            Sentiment::Neutral,
            "Tie between positive and negative should default to Neutral"
        );
    }

    #[test]
    fn test_stats_dominant_tie_with_neutral() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Neutral);
        // Positive == Neutral, Negative == 0 — neither > both others
        assert_eq!(
            stats.dominant(),
            Sentiment::Neutral,
            "When no single sentiment strictly dominates, should return Neutral"
        );
    }

    #[test]
    fn test_stats_dominant_empty() {
        let stats = SentimentStats::new();
        assert_eq!(
            stats.dominant(),
            Sentiment::Neutral,
            "Empty stats should default to Neutral"
        );
    }

    #[test]
    fn test_stats_dominant_positive_vs_negative() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Negative);
        stats.add(Sentiment::Negative);
        // 3 positive > 2 negative > 0 neutral
        assert_eq!(stats.dominant(), Sentiment::Positive);
    }

    #[test]
    fn test_stats_total_large_numbers() {
        let mut stats = SentimentStats::new();
        for _ in 0..1000 {
            stats.add(Sentiment::Positive);
        }
        for _ in 0..500 {
            stats.add(Sentiment::Neutral);
        }
        for _ in 0..250 {
            stats.add(Sentiment::Negative);
        }
        assert_eq!(stats.positive, 1000);
        assert_eq!(stats.neutral, 500);
        assert_eq!(stats.negative, 250);
        assert_eq!(stats.total(), 1750);
    }

    #[test]
    fn test_stats_default_equals_new() {
        let default = SentimentStats::default();
        let new = SentimentStats::new();
        assert_eq!(default.positive, new.positive);
        assert_eq!(default.neutral, new.neutral);
        assert_eq!(default.negative, new.negative);
    }

    #[test]
    fn test_stats_clone() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        let cloned = stats.clone();
        assert_eq!(cloned.positive, 1);
        assert_eq!(cloned.total(), 1);
    }

    #[test]
    fn test_stats_debug_format() {
        let stats = SentimentStats::new();
        let debug = format!("{:?}", stats);
        assert!(debug.contains("SentimentStats"));
    }

    // ─── Sentiment tests ────────────────────────────────────────────────────

    #[test]
    fn test_sentiment_debug() {
        assert_eq!(format!("{:?}", Sentiment::Positive), "Positive");
        assert_eq!(format!("{:?}", Sentiment::Neutral), "Neutral");
        assert_eq!(format!("{:?}", Sentiment::Negative), "Negative");
    }

    #[test]
    fn test_sentiment_clone_copy() {
        let s = Sentiment::Positive;
        let c = s;
        assert_eq!(s, c);
    }

    #[test]
    fn test_sentiment_partial_eq() {
        assert_eq!(Sentiment::Positive, Sentiment::Positive);
        assert_ne!(Sentiment::Positive, Sentiment::Negative);
        assert_ne!(Sentiment::Positive, Sentiment::Neutral);
    }

    #[test]
    fn test_sentiment_stats_derive() {
        // Verify the struct implements required traits
        fn assert_traits<T: std::fmt::Debug + Clone + Default>() {}
        assert_traits::<SentimentStats>();
    }

    // ─── SentimentConfig tests ────────────────────────────────────────────

    #[test]
    fn test_sentiment_config_default() {
        let config = SentimentConfig::default();
        assert!(config.use_basic_keywords);
        assert!(config.use_context);
        assert!(config.use_emoji);
        assert!(config.use_domain);
        assert!(!config.use_llm);
        assert!((config.llm_min_confidence - 0.7).abs() < f32::EPSILON);
        assert!((config.llm_probability - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sentiment_config_custom() {
        let config = SentimentConfig {
            use_basic_keywords: false,
            use_context: false,
            use_emoji: false,
            use_domain: false,
            use_llm: true,
            llm_min_confidence: 0.9,
            llm_probability: 1.0,
        };
        assert!(!config.use_basic_keywords);
        assert!(config.use_llm);
    }

    #[test]
    fn test_sentiment_config_debug() {
        let config = SentimentConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("use_basic_keywords"));
    }

    #[test]
    fn test_sentiment_config_clone() {
        let config = SentimentConfig::default();
        let cloned = config.clone();
        assert_eq!(config.use_basic_keywords, cloned.use_basic_keywords);
        assert_eq!(config.llm_min_confidence, cloned.llm_min_confidence);
    }

    // ─── ThreadContext tests ────────────────────────────────────────────────

    #[test]
    fn test_thread_context_creation() {
        let ctx = ThreadContext {
            reply_count: 5,
            avg_reply_sentiment: 0.5,
            is_reply: true,
            is_quote: false,
            thread_depth: 2,
            conversation_indicators: vec![
                ConversationIndicator::Agreement,
                ConversationIndicator::Question,
            ],
        };
        assert_eq!(ctx.reply_count, 5);
        assert!(ctx.avg_reply_sentiment > 0.0);
        assert!(ctx.is_reply);
        assert_eq!(ctx.conversation_indicators.len(), 2);
    }

    #[test]
    fn test_thread_context_debug() {
        let ctx = ThreadContext {
            reply_count: 0,
            avg_reply_sentiment: 0.0,
            is_reply: false,
            is_quote: false,
            thread_depth: 0,
            conversation_indicators: vec![],
        };
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("ThreadContext"));
    }

    #[test]
    fn test_thread_context_clone() {
        let ctx = ThreadContext {
            reply_count: 3,
            avg_reply_sentiment: -0.2,
            is_reply: false,
            is_quote: true,
            thread_depth: 1,
            conversation_indicators: vec![ConversationIndicator::Disagreement],
        };
        let cloned = ctx.clone();
        assert_eq!(cloned.reply_count, 3);
        assert_eq!(cloned.conversation_indicators.len(), 1);
    }

    // ─── ConversationIndicator tests ────────────────────────────────────────

    #[test]
    fn test_conversation_indicator_all_variants() {
        let variants = [
            ConversationIndicator::Agreement,
            ConversationIndicator::Disagreement,
            ConversationIndicator::Question,
            ConversationIndicator::Clarification,
            ConversationIndicator::Humor,
            ConversationIndicator::Sarcasm,
            ConversationIndicator::Support,
            ConversationIndicator::Criticism,
        ];
        assert_eq!(variants.len(), 8);
    }

    #[test]
    fn test_conversation_indicator_partial_eq() {
        assert_eq!(
            ConversationIndicator::Agreement,
            ConversationIndicator::Agreement
        );
        assert_ne!(
            ConversationIndicator::Agreement,
            ConversationIndicator::Disagreement
        );
    }

    #[test]
    fn test_conversation_indicator_debug() {
        assert_eq!(format!("{:?}", ConversationIndicator::Sarcasm), "Sarcasm");
    }

    #[test]
    fn test_conversation_indicator_clone_copy() {
        let a = ConversationIndicator::Support;
        let b = a;
        assert_eq!(a, b);
    }

    // ─── UserReputation tests ───────────────────────────────────────────────

    #[test]
    fn test_user_reputation_creation() {
        let rep = UserReputation {
            follower_count: 10000,
            is_verified: true,
            account_age_days: 365,
            engagement_rate: 0.05,
            is_influential: true,
            trust_score: 0.8,
        };
        assert!(rep.is_influential);
        assert!(rep.trust_score > 0.5);
        assert_eq!(rep.follower_count, 10000);
    }

    #[test]
    fn test_user_reputation_not_influential() {
        let rep = UserReputation {
            follower_count: 100,
            is_verified: false,
            account_age_days: 30,
            engagement_rate: 0.01,
            is_influential: false,
            trust_score: 0.3,
        };
        assert!(!rep.is_influential);
    }

    #[test]
    fn test_user_reputation_debug() {
        let rep = UserReputation {
            follower_count: 0,
            is_verified: false,
            account_age_days: 0,
            engagement_rate: 0.0,
            is_influential: false,
            trust_score: 0.5,
        };
        let debug = format!("{:?}", rep);
        assert!(debug.contains("UserReputation"));
    }

    #[test]
    fn test_user_reputation_clone() {
        let rep = UserReputation {
            follower_count: 500,
            is_verified: true,
            account_age_days: 100,
            engagement_rate: 0.02,
            is_influential: true,
            trust_score: 0.6,
        };
        let cloned = rep.clone();
        assert_eq!(cloned.follower_count, 500);
        assert_eq!(cloned.trust_score, 0.6);
    }

    // ─── TemporalFactors tests ──────────────────────────────────────────────

    #[test]
    fn test_temporal_factors_creation() {
        let tf = TemporalFactors {
            hour_of_day: 14,
            day_of_week: 3,
            hours_since_post: 6.0,
            is_peak_hour: false,
            trending_bias: 0.5,
            recency: 0.9,
        };
        assert!(!tf.is_peak_hour);
        assert_eq!(tf.hour_of_day, 14);
        assert!(tf.hours_since_post > 0.0);
    }

    #[test]
    fn test_temporal_factors_peak_hour() {
        let tf = TemporalFactors {
            hour_of_day: 8,
            day_of_week: 1,
            hours_since_post: 1.0,
            is_peak_hour: true,
            trending_bias: 0.0,
            recency: 1.0,
        };
        assert!(tf.is_peak_hour);
        assert!((tf.recency - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_temporal_factors_debug() {
        let tf = TemporalFactors {
            hour_of_day: 0,
            day_of_week: 0,
            hours_since_post: 0.0,
            is_peak_hour: false,
            trending_bias: 0.0,
            recency: 0.0,
        };
        let debug = format!("{:?}", tf);
        assert!(debug.contains("TemporalFactors"));
    }

    #[test]
    fn test_temporal_factors_clone() {
        let tf = TemporalFactors {
            hour_of_day: 18,
            day_of_week: 5,
            hours_since_post: 48.0,
            is_peak_hour: true,
            trending_bias: -0.3,
            recency: 0.5,
        };
        let cloned = tf.clone();
        assert!((cloned.recency - 0.5).abs() < f32::EPSILON);
    }

    // ─── ScoreBreakdown tests ───────────────────────────────────────────────

    #[test]
    fn test_score_breakdown_default() {
        let breakdown = ScoreBreakdown::default();
        assert_eq!(breakdown.text_score, 0.0);
        assert_eq!(breakdown.emoji_score, 0.0);
        assert_eq!(breakdown.domain_score, 0.0);
        assert_eq!(breakdown.context_score, 0.0);
        assert_eq!(breakdown.reputation_score, 0.0);
        assert_eq!(breakdown.temporal_score, 0.0);
    }

    #[test]
    fn test_score_breakdown_custom() {
        let breakdown = ScoreBreakdown {
            text_score: 1.0,
            emoji_score: 0.5,
            domain_score: 0.0,
            context_score: -0.3,
            reputation_score: 0.2,
            temporal_score: -0.1,
        };
        assert_eq!(breakdown.text_score, 1.0);
        assert_eq!(breakdown.emoji_score, 0.5);
        assert_eq!(breakdown.context_score, -0.3);
    }

    #[test]
    fn test_score_breakdown_debug() {
        let breakdown = ScoreBreakdown::default();
        let debug = format!("{:?}", breakdown);
        assert!(debug.contains("ScoreBreakdown"));
    }

    #[test]
    fn test_score_breakdown_clone() {
        let breakdown = ScoreBreakdown {
            text_score: 2.0,
            emoji_score: -1.0,
            domain_score: 0.5,
            context_score: 0.0,
            reputation_score: -0.5,
            temporal_score: 0.3,
        };
        let cloned = breakdown.clone();
        assert_eq!(cloned.text_score, 2.0);
    }

    // ─── EnhancedSentimentResult tests ──────────────────────────────────────

    #[test]
    fn test_enhanced_sentiment_result_creation() {
        let result = EnhancedSentimentResult {
            base_sentiment: Sentiment::Neutral,
            final_sentiment: Sentiment::Positive,
            base_score: 0.2,
            final_score: 0.8,
            confidence: 0.95,
            score_breakdown: ScoreBreakdown::default(),
        };
        assert_eq!(result.base_sentiment, Sentiment::Neutral);
        assert_eq!(result.final_sentiment, Sentiment::Positive);
        assert!(result.final_score > result.base_score);
        assert!(result.confidence > 0.9);
    }

    #[test]
    fn test_enhanced_sentiment_result_debug() {
        let result = EnhancedSentimentResult {
            base_sentiment: Sentiment::Negative,
            final_sentiment: Sentiment::Negative,
            base_score: -0.5,
            final_score: -0.5,
            confidence: 0.5,
            score_breakdown: ScoreBreakdown::default(),
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("EnhancedSentimentResult"));
    }

    #[test]
    fn test_enhanced_sentiment_result_clone() {
        let result = EnhancedSentimentResult {
            base_sentiment: Sentiment::Positive,
            final_sentiment: Sentiment::Negative,
            base_score: 0.7,
            final_score: -0.3,
            confidence: 0.6,
            score_breakdown: ScoreBreakdown::default(),
        };
        let cloned = result.clone();
        assert_eq!(cloned.base_score, 0.7);
        assert_eq!(cloned.final_score, -0.3);
    }
}
