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
    pub fn with_strategy(strategy: DecisionStrategy) -> Self {
        let (primary, fallback) = Self::create_strategies(strategy, None);
        Self {
            strategy: primary,
            fallback,
        }
    }

    /// Create engine with LLM support.
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
