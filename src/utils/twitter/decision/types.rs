//! Decision engine types and shared structures.
//!
//! This module contains all shared types used across decision strategies.
//! Types are extracted from the original `twitteractivity_decision.rs` module
//! to support the unified decision engine architecture.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::utils::twitter::twitteractivity_types::TweetId;

/// Context passed to decision engines for analysis.
#[derive(Debug, Clone)]
pub struct TweetContext {
    /// Unique tweet identifier
    pub tweet_id: TweetId,
    /// Tweet text content
    pub text: String,
    /// Tweet author handle
    pub author: String,
    /// Top replies for sentiment analysis
    pub replies: Vec<String>,
    /// Persona weights for decision modification
    pub persona: crate::utils::twitter::twitteractivity_persona::PersonaWeights,
    /// Task configuration
    pub task_config: crate::utils::twitter::twitteractivity_state::TaskConfig,
    /// Human-readable tweet age description
    pub tweet_age: String,
}

/// Engagement level determines which actions are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngagementLevel {
    /// Full engagement: like, retweet, reply, follow, quote tweet
    Full,
    /// Medium engagement: like, retweet only
    Medium,
    /// Minimal engagement: like only
    Minimal,
    /// Skip engagement entirely
    None,
}

/// Extended engagement decision with metadata.
#[derive(Debug, Clone)]
pub struct EngagementDecision {
    /// Engagement level
    pub level: EngagementLevel,
    /// Quality score (-100 to 100)
    pub score: i32,
    /// Human-readable decision reason
    pub reason: String,
    /// Score multiplier applied
    pub multiplier: f64,
    /// Confidence in decision (0.0 - 1.0)
    pub confidence: f64,
}

/// Strategy selection for decision engines.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStrategy {
    /// Rule-based legacy engine
    #[default]
    Legacy,
    /// Persona-weighted engine
    Persona,
    /// LLM-based engine
    Llm,
    /// Combined approach
    Hybrid,
    /// Single LLM call for decision + content
    Unified,
    /// Auto-select based on config
    Auto,
}

impl DecisionStrategy {
    /// Get all available strategies.
    #[must_use]
    pub fn all() -> &'static [DecisionStrategy] {
        &[
            DecisionStrategy::Legacy,
            DecisionStrategy::Persona,
            DecisionStrategy::Llm,
            DecisionStrategy::Hybrid,
            DecisionStrategy::Unified,
            DecisionStrategy::Auto,
        ]
    }

    /// Get human-readable name for strategy.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            DecisionStrategy::Legacy => "legacy",
            DecisionStrategy::Persona => "persona",
            DecisionStrategy::Llm => "llm",
            DecisionStrategy::Hybrid => "hybrid",
            DecisionStrategy::Unified => "unified",
            DecisionStrategy::Auto => "auto",
        }
    }
}

/// Core trait for all decision engines.
#[async_trait]
pub trait DecisionEngine: Send + Sync {
    /// Engine name for logging/metrics.
    fn name(&self) -> &'static str;

    /// Make engagement decision for a tweet.
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision;

    /// Check if engine is available (e.g., LLM API reachable).
    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // EngagementLevel Tests
    // ========================================================================

    #[test]
    fn test_engagement_level_variants() {
        assert_eq!(EngagementLevel::Full as u8, 0);
        assert_eq!(EngagementLevel::Medium as u8, 1);
        assert_eq!(EngagementLevel::Minimal as u8, 2);
        assert_eq!(EngagementLevel::None as u8, 3);
    }

    #[test]
    fn test_engagement_level_partial_eq() {
        assert_eq!(EngagementLevel::Full, EngagementLevel::Full);
        assert_ne!(EngagementLevel::Full, EngagementLevel::None);
    }

    #[test]
    fn test_engagement_level_debug() {
        assert_eq!(format!("{:?}", EngagementLevel::Full), "Full");
        assert_eq!(format!("{:?}", EngagementLevel::None), "None");
    }

    #[test]
    fn test_engagement_level_serialize() {
        let json = serde_json::to_string(&EngagementLevel::Medium).unwrap();
        assert_eq!(json, "\"Medium\"");
    }

    #[test]
    fn test_engagement_level_deserialize() {
        let level: EngagementLevel = serde_json::from_str("\"Full\"").unwrap();
        assert_eq!(level, EngagementLevel::Full);
    }

    #[test]
    fn test_engagement_level_deserialize_snake_case() {
        // Deserialization uses serde rename_all = "snake_case"
        let level: EngagementLevel = serde_json::from_str("\"Full\"").unwrap();
        assert_eq!(level, EngagementLevel::Full);
    }

    // ========================================================================
    // EngagementDecision Tests
    // ========================================================================

    #[test]
    fn test_engagement_decision_creation() {
        let decision = EngagementDecision {
            level: EngagementLevel::Full,
            score: 85,
            reason: "high quality".to_string(),
            multiplier: 1.5,
            confidence: 0.9,
        };
        assert_eq!(decision.level, EngagementLevel::Full);
        assert_eq!(decision.score, 85);
        assert_eq!(decision.reason, "high quality");
        assert_eq!(decision.multiplier, 1.5);
        assert_eq!(decision.confidence, 0.9);
    }

    #[test]
    fn test_engagement_decision_low_score() {
        let decision = EngagementDecision {
            level: EngagementLevel::None,
            score: 0,
            reason: "skip".to_string(),
            multiplier: 0.0,
            confidence: 0.95,
        };
        assert!(decision.score == 0);
        assert!(decision.confidence > 0.9);
        assert!(decision.multiplier == 0.0);
    }

    #[test]
    fn test_engagement_decision_clone() {
        let decision = EngagementDecision {
            level: EngagementLevel::Medium,
            score: 50,
            reason: "test".to_string(),
            multiplier: 1.0,
            confidence: 0.7,
        };
        let cloned = decision.clone();
        assert_eq!(cloned.level, decision.level);
        assert_eq!(cloned.score, decision.score);
        assert_eq!(cloned.reason, decision.reason);
    }

    // ========================================================================
    // DecisionStrategy Tests
    // ========================================================================

    #[test]
    fn test_decision_strategy_default() {
        let strategy = DecisionStrategy::default();
        assert_eq!(strategy, DecisionStrategy::Legacy);
    }

    #[test]
    fn test_decision_strategy_all() {
        let all = DecisionStrategy::all();
        assert_eq!(all.len(), 6);
        assert!(all.contains(&DecisionStrategy::Legacy));
        assert!(all.contains(&DecisionStrategy::Persona));
        assert!(all.contains(&DecisionStrategy::Llm));
        assert!(all.contains(&DecisionStrategy::Hybrid));
        assert!(all.contains(&DecisionStrategy::Unified));
        assert!(all.contains(&DecisionStrategy::Auto));
    }

    #[test]
    fn test_decision_strategy_name() {
        assert_eq!(DecisionStrategy::Legacy.name(), "legacy");
        assert_eq!(DecisionStrategy::Persona.name(), "persona");
        assert_eq!(DecisionStrategy::Llm.name(), "llm");
        assert_eq!(DecisionStrategy::Hybrid.name(), "hybrid");
        assert_eq!(DecisionStrategy::Unified.name(), "unified");
        assert_eq!(DecisionStrategy::Auto.name(), "auto");
    }

    #[test]
    fn test_decision_strategy_deserialize() {
        let strategy: DecisionStrategy = serde_json::from_str("\"legacy\"").unwrap();
        assert_eq!(strategy, DecisionStrategy::Legacy);

        let strategy: DecisionStrategy = serde_json::from_str("\"persona\"").unwrap();
        assert_eq!(strategy, DecisionStrategy::Persona);
    }

    #[test]
    fn test_decision_strategy_partial_eq() {
        assert_eq!(DecisionStrategy::Legacy, DecisionStrategy::Legacy);
        assert_ne!(DecisionStrategy::Legacy, DecisionStrategy::Persona);
    }

    #[test]
    fn test_decision_strategy_debug() {
        assert_eq!(format!("{:?}", DecisionStrategy::Hybrid), "Hybrid");
    }

    // ========================================================================
    // DecisionEngine Trait Tests
    // ========================================================================

    struct TestEngine;

    #[async_trait]
    impl DecisionEngine for TestEngine {
        fn name(&self) -> &'static str {
            "test"
        }

        async fn decide(&self, _ctx: &TweetContext) -> EngagementDecision {
            EngagementDecision {
                level: EngagementLevel::Full,
                score: 100,
                reason: "test".to_string(),
                multiplier: 1.0,
                confidence: 1.0,
            }
        }
    }

    #[tokio::test]
    async fn test_decision_engine_default_available() {
        let engine = TestEngine;
        assert!(engine.is_available());
    }

    #[tokio::test]
    async fn test_decision_engine_name() {
        let engine = TestEngine;
        assert_eq!(engine.name(), "test");
    }

    #[tokio::test]
    async fn test_decision_engine_decide_returns_decision() {
        let engine = TestEngine;
        // Construct a minimal TweetContext for testing
        let ctx = TweetContext {
            tweet_id: TweetId::from_unchecked("1"),
            text: "test".to_string(),
            author: "user".to_string(),
            replies: vec![],
            persona: crate::utils::twitter::twitteractivity_persona::PersonaWeights::default(),
            task_config: crate::utils::twitter::twitteractivity_state::TaskConfig::default(),
            tweet_age: "".to_string(),
        };
        let decision = engine.decide(&ctx).await;
        assert_eq!(decision.level, EngagementLevel::Full);
        assert_eq!(decision.score, 100);
    }

    // ========================================================================
    // TweetContext Creation Tests
    // ========================================================================

    #[test]
    fn test_tweet_context_creation() {
        let ctx = TweetContext {
            tweet_id: TweetId::from_unchecked("123"),
            text: "Hello world".to_string(),
            author: "testuser".to_string(),
            replies: vec!["Great post!".to_string()],
            persona: crate::utils::twitter::twitteractivity_persona::PersonaWeights::default(),
            task_config: crate::utils::twitter::twitteractivity_state::TaskConfig::default(),
            tweet_age: "recent".to_string(),
        };
        assert_eq!(ctx.tweet_id, TweetId::from_unchecked("123"));
        assert_eq!(ctx.author, "testuser");
        assert_eq!(ctx.replies.len(), 1);
    }
}
