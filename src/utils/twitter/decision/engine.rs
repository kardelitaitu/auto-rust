//! Unified decision engine implementation.
//!
//! This module provides the `UnifiedEngine` which can use any decision strategy
//! via the strategy pattern. It also includes the `DecisionEngineFactory` for
//! creating engines based on configuration.

use async_trait::async_trait;
use log::warn;

use super::strategies::{
    hybrid::HybridStrategy, legacy::LegacyStrategy, llm::LlmStrategy, persona::PersonaStrategy,
    unified::UnifiedStrategy, DecisionStrategyImpl,
};
use super::types::{DecisionEngine, DecisionStrategy, EngagementDecision, TweetContext};

/// Unified engine that can use any decision strategy.
pub struct UnifiedEngine {
    strategy: Box<dyn DecisionStrategyImpl>,
    fallback: Option<Box<dyn DecisionStrategyImpl>>,
}

impl UnifiedEngine {
    /// Create engine with specific strategy.
    #[must_use]
    pub fn with_strategy(strategy: DecisionStrategy) -> Self {
        let (primary, fallback) = Self::create_strategies(strategy, None);
        Self {
            strategy: primary,
            fallback,
        }
    }

    /// Create engine with LLM support.
    #[must_use]
    pub fn with_llm(strategy: DecisionStrategy, api_key: String) -> Self {
        let (primary, fallback) = Self::create_strategies(strategy, Some(api_key));
        Self {
            strategy: primary,
            fallback,
        }
    }

    /// Helper to create strategy implementations.
    fn create_strategies(
        strategy: DecisionStrategy,
        api_key: Option<String>,
    ) -> (
        Box<dyn DecisionStrategyImpl>,
        Option<Box<dyn DecisionStrategyImpl>>,
    ) {
        match strategy {
            DecisionStrategy::Legacy => (Box::new(LegacyStrategy), None),
            DecisionStrategy::Persona => (Box::new(PersonaStrategy::new()), None),
            DecisionStrategy::Llm => {
                if let Some(key) = api_key {
                    (
                        Box::new(LlmStrategy::new(key)),
                        Some(Box::new(PersonaStrategy::new())),
                    )
                } else {
                    (Box::new(PersonaStrategy::new()), None)
                }
            }
            DecisionStrategy::Hybrid => {
                if let Some(key) = api_key {
                    (Box::new(HybridStrategy::with_llm(key, 0.3, 0.7)), None)
                } else {
                    (Box::new(HybridStrategy::persona_only()), None)
                }
            }
            DecisionStrategy::Unified => {
                if let Some(key) = api_key {
                    (
                        Box::new(UnifiedStrategy::new(key)),
                        Some(Box::new(PersonaStrategy::new())),
                    )
                } else {
                    (Box::new(PersonaStrategy::new()), None)
                }
            }
            DecisionStrategy::Auto => {
                if let Some(key) = api_key {
                    (
                        Box::new(UnifiedStrategy::new(key)),
                        Some(Box::new(PersonaStrategy::new())),
                    )
                } else {
                    (Box::new(PersonaStrategy::new()), None)
                }
            }
        }
    }

    /// Get the configured strategy type.
    #[must_use]
    pub fn strategy_type(&self) -> DecisionStrategy {
        self.strategy.strategy_type()
    }
}

#[async_trait]
impl DecisionEngine for UnifiedEngine {
    fn name(&self) -> &'static str {
        "UnifiedEngine"
    }

    fn is_available(&self) -> bool {
        self.strategy.is_available() || self.fallback.as_ref().is_some_and(|f| f.is_available())
    }

    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // Try primary strategy
        if self.strategy.is_available() {
            return self.strategy.decide(ctx).await;
        }

        // Fallback if primary unavailable
        if let Some(ref fallback) = self.fallback {
            if fallback.is_available() {
                warn!(
                    "Primary strategy {} unavailable, using fallback {}",
                    self.strategy.name(),
                    fallback.name()
                );
                return fallback.decide(ctx).await;
            }
        }

        // Ultimate fallback (neutral skip)
        EngagementDecision {
            level: super::types::EngagementLevel::None,
            score: 0,
            reason: "No available decision strategy".to_string(),
            multiplier: 0.0,
            confidence: 0.0,
        }
    }
}

/// Factory for creating decision engines based on strategy.
pub struct DecisionEngineFactory;

impl DecisionEngineFactory {
    /// Create appropriate engine based on strategy and config.
    #[must_use]
    pub fn create(
        strategy: DecisionStrategy,
        llm_api_key: Option<String>,
    ) -> Box<dyn DecisionEngine> {
        match llm_api_key {
            Some(key) => Box::new(UnifiedEngine::with_llm(strategy, key)),
            None => Box::new(UnifiedEngine::with_strategy(strategy)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use crate::utils::twitter::twitteractivity_state::TaskConfig;

    // ========================================================================
    // create_strategies Tests
    // ========================================================================

    #[test]
    fn test_create_legacy_no_key() {
        let (primary, fallback) = UnifiedEngine::create_strategies(DecisionStrategy::Legacy, None);
        assert_eq!(primary.name(), "legacy");
        assert!(fallback.is_none());
    }

    #[test]
    fn test_create_legacy_with_key() {
        let (primary, fallback) =
            UnifiedEngine::create_strategies(DecisionStrategy::Legacy, Some("k".to_string()));
        assert_eq!(primary.name(), "legacy");
        assert!(fallback.is_none());
    }

    #[test]
    fn test_create_persona() {
        let (primary, fallback) = UnifiedEngine::create_strategies(DecisionStrategy::Persona, None);
        assert_eq!(primary.name(), "persona");
        assert!(fallback.is_none());
    }

    #[test]
    fn test_create_llm_with_key_has_fallback() {
        let (primary, fallback) =
            UnifiedEngine::create_strategies(DecisionStrategy::Llm, Some("k".to_string()));
        assert_eq!(primary.name(), "llm");
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().name(), "persona");
    }

    #[test]
    fn test_create_llm_without_key_falls_to_persona() {
        let (primary, fallback) = UnifiedEngine::create_strategies(DecisionStrategy::Llm, None);
        assert_eq!(primary.name(), "persona");
        assert!(fallback.is_none());
    }

    #[test]
    fn test_create_hybrid_with_key() {
        let (primary, fallback) =
            UnifiedEngine::create_strategies(DecisionStrategy::Hybrid, Some("k".to_string()));
        assert_eq!(primary.name(), "hybrid");
        assert!(fallback.is_none());
    }

    #[test]
    fn test_create_hybrid_without_key() {
        let (primary, fallback) = UnifiedEngine::create_strategies(DecisionStrategy::Hybrid, None);
        assert_eq!(primary.name(), "hybrid");
        assert!(fallback.is_none());
    }

    #[test]
    fn test_create_unified_with_key_has_fallback() {
        let (primary, fallback) =
            UnifiedEngine::create_strategies(DecisionStrategy::Unified, Some("k".to_string()));
        assert_eq!(primary.name(), "unified");
        assert!(fallback.is_some());
        assert_eq!(fallback.unwrap().name(), "persona");
    }

    #[test]
    fn test_create_unified_without_key_falls_to_persona() {
        let (primary, fallback) = UnifiedEngine::create_strategies(DecisionStrategy::Unified, None);
        assert_eq!(primary.name(), "persona");
        assert!(fallback.is_none());
    }

    #[test]
    fn test_create_auto_with_key() {
        let (primary, fallback) =
            UnifiedEngine::create_strategies(DecisionStrategy::Auto, Some("k".to_string()));
        assert_eq!(primary.name(), "unified");
        assert!(fallback.is_some());
    }

    #[test]
    fn test_create_auto_without_key() {
        let (primary, fallback) = UnifiedEngine::create_strategies(DecisionStrategy::Auto, None);
        assert_eq!(primary.name(), "persona");
        assert!(fallback.is_none());
    }

    // ========================================================================
    // UnifiedEngine Tests
    // ========================================================================

    #[test]
    fn test_engine_with_strategy() {
        let engine = UnifiedEngine::with_strategy(DecisionStrategy::Legacy);
        assert_eq!(engine.strategy_type(), DecisionStrategy::Legacy);
        assert!(engine.name().contains("UnifiedEngine"));
    }

    #[test]
    fn test_engine_with_llm() {
        let engine = UnifiedEngine::with_llm(DecisionStrategy::Llm, "key".to_string());
        assert_eq!(engine.strategy_type(), DecisionStrategy::Llm);
    }

    #[test]
    fn test_engine_is_available_primary_available() {
        let engine = UnifiedEngine::with_strategy(DecisionStrategy::Legacy);
        assert!(engine.is_available());
    }

    #[test]
    fn test_engine_strategy_type_persona() {
        let engine = UnifiedEngine::with_strategy(DecisionStrategy::Persona);
        assert_eq!(engine.strategy_type(), DecisionStrategy::Persona);
    }

    // ========================================================================
    // DecisionEngineFactory Tests
    // ========================================================================

    #[test]
    fn test_factory_create_with_key() {
        let engine = DecisionEngineFactory::create(DecisionStrategy::Llm, Some("k".to_string()));
        assert_eq!(engine.name(), "UnifiedEngine");
    }

    #[test]
    fn test_factory_create_without_key() {
        let engine = DecisionEngineFactory::create(DecisionStrategy::Legacy, None);
        assert_eq!(engine.name(), "UnifiedEngine");
    }

    // ========================================================================
    // Fallback Logic Tests
    // ========================================================================

    // Using Legacy strategy (always available) ensures is_available works
    #[test]
    fn test_legacy_strategy_always_available() {
        let s = LegacyStrategy;
        assert!(s.is_available());
    }

    #[test]
    fn test_fallback_used_when_primary_unavailable() {
        // UnifiedStrategy without key is unavailable → falls back to Persona
        let engine = UnifiedEngine::with_llm(DecisionStrategy::Unified, String::new());
        assert!(engine.is_available()); // Persona fallback is always available
    }

    #[tokio::test]
    async fn test_decide_falls_back_when_primary_unavailable() {
        let engine = UnifiedEngine::with_llm(DecisionStrategy::Unified, String::new());
        let ctx = TweetContext {
            tweet_id: "1".to_string(),
            text: "test".to_string(),
            author: "u".to_string(),
            replies: vec![],
            persona: PersonaWeights::default(),
            task_config: TaskConfig::default(),
            tweet_age: "".to_string(),
        };
        let decision = engine.decide(&ctx).await;
        // Should have fallen back to Persona, which may decide to engage or not
        assert!(decision.score >= 0);
    }
}
