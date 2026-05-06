//! Unified decision engine implementation.
//!
//! This module provides the `UnifiedEngine` which can use any decision strategy
//! via the strategy pattern. It also includes the `DecisionEngineFactory` for
//! creating engines based on configuration.

use async_trait::async_trait;

use super::types::{DecisionEngine, DecisionStrategy, EngagementDecision, TweetContext};

/// Unified engine that can use any decision strategy.
pub struct UnifiedEngine {
    strategy: DecisionStrategy,
    llm_api_key: Option<String>,
}

impl UnifiedEngine {
    /// Create engine with specific strategy.
    pub fn with_strategy(strategy: DecisionStrategy) -> Self {
        Self {
            strategy,
            llm_api_key: None,
        }
    }

    /// Create engine with LLM support.
    pub fn with_llm(strategy: DecisionStrategy, api_key: String) -> Self {
        Self {
            strategy,
            llm_api_key: Some(api_key),
        }
    }

    /// Get the configured strategy.
    pub fn strategy(&self) -> DecisionStrategy {
        self.strategy
    }

    /// Check if LLM is available.
    fn has_llm(&self) -> bool {
        self.llm_api_key.is_some()
    }
}

#[async_trait]
impl DecisionEngine for UnifiedEngine {
    fn name(&self) -> &'static str {
        "UnifiedEngine"
    }

    fn is_available(&self) -> bool {
        match self.strategy {
            DecisionStrategy::Llm | DecisionStrategy::Unified => self.has_llm(),
            DecisionStrategy::Hybrid => true, // Hybrid can work with or without LLM
            _ => true, // Legacy, Persona, Auto always available
        }
    }

    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        // Delegate to appropriate strategy
        // This will be implemented when strategies are migrated
        // For now, return a default decision
        EngagementDecision {
            level: super::types::EngagementLevel::Medium,
            score: 50,
            reason: "UnifiedEngine placeholder".to_string(),
            multiplier: 1.0,
            confidence: 0.5,
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
