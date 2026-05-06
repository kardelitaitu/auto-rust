//! Decision engine types and shared structures.
//!
//! This module contains all shared types used across decision strategies.
//! Types are extracted from the original `twitteractivity_decision.rs` module
//! to support the unified decision engine architecture.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Context passed to decision engines for analysis.
#[derive(Debug, Clone)]
pub struct TweetContext {
    /// Unique tweet identifier
    pub tweet_id: String,
    /// Tweet text content
    pub text: String,
    /// Tweet author handle
    pub author: String,
    /// Top replies for sentiment analysis
    pub replies: Vec<String>,
    /// Persona weights for decision modification
    pub persona: super::twitteractivity_persona::PersonaWeights,
    /// Task configuration
    pub task_config: super::twitteractivity_state::TaskConfig,
    /// Human-readable tweet age description
    pub tweet_age: String,
    /// Topic alignment score/description
    pub topic_alignment: String,
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
