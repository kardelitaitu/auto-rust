//! Decision strategy implementations.

use crate::utils::twitter::decision::types::{DecisionStrategy, EngagementDecision, TweetContext};
use async_trait::async_trait;

pub(crate) mod hybrid;
pub(crate) mod legacy;
pub(crate) mod llm;
pub(crate) mod persona;
pub(crate) mod unified;

/// Internal trait for all decision strategy implementations.
#[async_trait]
pub(crate) trait DecisionStrategyImpl: Send + Sync {
    /// Make engagement decision for a tweet.
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision;

    /// Get the strategy type.
    fn strategy_type(&self) -> DecisionStrategy;

    /// Check if strategy is available.
    fn is_available(&self) -> bool {
        true
    }

    /// Strategy name for logging.
    fn name(&self) -> &'static str;
}
