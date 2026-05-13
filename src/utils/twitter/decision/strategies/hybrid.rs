//! Hybrid decision strategy combining multiple approaches.
//!
//! Uses weighted ensemble of PersonaStrategy and LlmStrategy.
//! Ported from `twitteractivity_decision_hybrid.rs`.

use crate::utils::twitter::decision::strategies::{
    llm::LlmStrategy, persona::PersonaStrategy, DecisionStrategyImpl,
};
use crate::utils::twitter::decision::types::{
    DecisionStrategy, EngagementDecision, EngagementLevel, TweetContext,
};
use async_trait::async_trait;
use log::info;

/// Strategy for combining multiple engine decisions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombinationStrategy {
    /// Weighted average of all engine scores
    WeightedAverage,
    /// Pick the best (highest confidence) decision
    #[allow(dead_code)]
    BestConfidence,
    /// Use LLM if available, fallback to Persona
    #[allow(dead_code)]
    LLMPrimary,
    /// Always require Persona approval (conservative)
    #[allow(dead_code)]
    Consensus,
}

/// Hybrid strategy implementation.
pub(crate) struct HybridStrategy {
    persona: PersonaStrategy,
    llm: Option<LlmStrategy>,
    persona_weight: f64,
    llm_weight: f64,
    combination: CombinationStrategy,
}

impl HybridStrategy {
    /// Create hybrid strategy with LLM
    pub fn with_llm(llm_api_key: String, persona_weight: f64, llm_weight: f64) -> Self {
        let llm = if llm_api_key.is_empty() {
            None
        } else {
            Some(LlmStrategy::new(llm_api_key))
        };

        Self {
            persona: PersonaStrategy::new(),
            llm,
            persona_weight: persona_weight.clamp(0.0, 1.0),
            llm_weight: llm_weight.clamp(0.0, 1.0),
            combination: CombinationStrategy::WeightedAverage,
        }
    }

    /// Create hybrid strategy with only Persona (LLM disabled)
    pub fn persona_only() -> Self {
        Self {
            persona: PersonaStrategy::new(),
            llm: None,
            persona_weight: 1.0,
            llm_weight: 0.0,
            combination: CombinationStrategy::WeightedAverage,
        }
    }

    /// Weighted average combination
    fn combine_weighted(
        &self,
        persona_decision: &EngagementDecision,
        llm_decision: &EngagementDecision,
    ) -> EngagementDecision {
        let total_weight = self.persona_weight + self.llm_weight;

        if total_weight == 0.0 {
            return persona_decision.clone();
        }

        let p_norm = self.persona_weight / total_weight;
        let l_norm = self.llm_weight / total_weight;

        // Weighted score
        let score = ((persona_decision.score as f64) * p_norm
            + (llm_decision.score as f64) * l_norm) as i32;

        // Weighted multiplier
        let multiplier = persona_decision.multiplier * p_norm + llm_decision.multiplier * l_norm;

        // Average confidence
        let confidence = (persona_decision.confidence + llm_decision.confidence) / 2.0;

        // Determine level from combined score
        let level = if score >= 75 {
            EngagementLevel::Full
        } else if score >= 50 {
            EngagementLevel::Medium
        } else if score >= 30 {
            EngagementLevel::Minimal
        } else {
            EngagementLevel::None
        };

        // Combined reason
        let reason = format!(
            "Hybrid (Persona:{} + LLM:{}): {} | {}",
            (p_norm * 100.0) as i32,
            (l_norm * 100.0) as i32,
            persona_decision.reason,
            llm_decision.reason
        );

        EngagementDecision {
            level,
            score,
            reason,
            multiplier,
            confidence,
        }
    }

    /// Best confidence strategy - pick the one with higher confidence
    fn combine_best_confidence(
        &self,
        persona_decision: &EngagementDecision,
        llm_decision: &EngagementDecision,
    ) -> EngagementDecision {
        if llm_decision.confidence > persona_decision.confidence {
            let mut decision = llm_decision.clone();
            decision.reason = format!("LLM selected (higher confidence): {}", decision.reason);
            decision
        } else {
            let mut decision = persona_decision.clone();
            decision.reason = format!("Persona selected (higher confidence): {}", decision.reason);
            decision
        }
    }

    /// LLM primary strategy - use LLM if available and confident
    fn combine_llm_primary(
        &self,
        persona_decision: &EngagementDecision,
        llm_decision: &EngagementDecision,
    ) -> EngagementDecision {
        // Use LLM if it has high confidence (>0.7) and reasonable score (>20)
        if llm_decision.confidence > 0.7 && llm_decision.score > 20 {
            let mut decision = llm_decision.clone();
            decision.reason = format!("LLM primary (confident): {}", decision.reason);
            decision
        } else {
            // Fallback to Persona
            let mut decision = persona_decision.clone();
            decision.reason = format!("Persona fallback (LLM low confidence): {}", decision.reason);
            decision
        }
    }

    /// Consensus strategy - both must agree
    fn combine_consensus(
        &self,
        persona_decision: &EngagementDecision,
        llm_decision: &EngagementDecision,
    ) -> EngagementDecision {
        let persona_skip = matches!(persona_decision.level, EngagementLevel::None);
        let llm_skip = matches!(llm_decision.level, EngagementLevel::None);

        // If either says skip, we skip (conservative)
        if persona_skip || llm_skip {
            return EngagementDecision {
                level: EngagementLevel::None,
                score: (persona_decision.score + llm_decision.score) / 2,
                reason: format!(
                    "Consensus skip: Persona={:?}, LLM={:?}",
                    persona_decision.level, llm_decision.level
                ),
                multiplier: 0.0,
                confidence: (persona_decision.confidence + llm_decision.confidence) / 2.0,
            };
        }

        // Both agree to engage, use weighted
        self.combine_weighted(persona_decision, llm_decision)
    }
}

#[async_trait]
impl DecisionStrategyImpl for HybridStrategy {
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        info!(
            "HybridStrategy: Combining Persona (weight={:.2}) and LLM (weight={:.2}, available={})",
            self.persona_weight,
            self.llm_weight,
            self.llm.is_some()
        );

        // Get Persona decision (always available)
        let persona_decision = self.persona.decide(ctx).await;

        // Check if we have LLM
        let llm_decision = if let Some(ref llm) = self.llm {
            if llm.is_available() {
                Some(llm.decide(ctx).await)
            } else {
                None
            }
        } else {
            None
        };

        // Combine based on strategy
        match (self.combination, llm_decision) {
            // No LLM available - use Persona only
            (_, None) => {
                info!("HybridStrategy: LLM unavailable, using Persona only");
                let mut decision = persona_decision;
                decision.reason = format!("Persona only (LLM unavailable): {}", decision.reason);
                decision
            }

            // Weighted average with LLM
            (CombinationStrategy::WeightedAverage, Some(llm)) => {
                self.combine_weighted(&persona_decision, &llm)
            }

            // Best confidence
            (CombinationStrategy::BestConfidence, Some(llm)) => {
                self.combine_best_confidence(&persona_decision, &llm)
            }

            // LLM primary
            (CombinationStrategy::LLMPrimary, Some(llm)) => {
                self.combine_llm_primary(&persona_decision, &llm)
            }

            // Consensus
            (CombinationStrategy::Consensus, Some(llm)) => {
                self.combine_consensus(&persona_decision, &llm)
            }
        }
    }

    fn strategy_type(&self) -> DecisionStrategy {
        DecisionStrategy::Hybrid
    }

    fn name(&self) -> &'static str {
        "hybrid"
    }
}

impl Default for HybridStrategy {
    fn default() -> Self {
        Self::persona_only()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn persona_decision(score: i32, level: EngagementLevel, confidence: f64) -> EngagementDecision {
        EngagementDecision {
            level,
            score,
            reason: "persona test".to_string(),
            multiplier: 1.0,
            confidence,
        }
    }

    fn llm_decision(score: i32, level: EngagementLevel, confidence: f64) -> EngagementDecision {
        EngagementDecision {
            level,
            score,
            reason: "llm test".to_string(),
            multiplier: 1.2,
            confidence,
        }
    }

    // ========================================================================
    // HybridStrategy Construction Tests
    // ========================================================================

    #[test]
    fn test_hybrid_default_is_persona_only() {
        let strategy = HybridStrategy::default();
        assert!(strategy.llm.is_none());
        assert_eq!(strategy.persona_weight, 1.0);
        assert_eq!(strategy.llm_weight, 0.0);
        assert_eq!(strategy.combination, CombinationStrategy::WeightedAverage);
    }

    #[test]
    fn test_hybrid_persona_only() {
        let strategy = HybridStrategy::persona_only();
        assert!(strategy.llm.is_none());
        assert_eq!(strategy.persona_weight, 1.0);
    }

    #[test]
    fn test_hybrid_with_llm_empty_key() {
        let strategy = HybridStrategy::with_llm(String::new(), 0.6, 0.4);
        assert!(strategy.llm.is_none());
    }

    #[test]
    fn test_hybrid_with_llm() {
        let strategy = HybridStrategy::with_llm("test-key".to_string(), 0.7, 0.3);
        assert!(strategy.llm.is_some());
        assert!((strategy.persona_weight - 0.7).abs() < 0.01);
        assert!((strategy.llm_weight - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_hybrid_weights_clamped() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 5.0, -1.0);
        assert!((strategy.persona_weight - 1.0).abs() < 0.01);
        assert!((strategy.llm_weight - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_hybrid_name() {
        let strategy = HybridStrategy::persona_only();
        assert_eq!(strategy.name(), "hybrid");
    }

    #[test]
    fn test_hybrid_strategy_type() {
        let strategy = HybridStrategy::persona_only();
        assert_eq!(strategy.strategy_type(), DecisionStrategy::Hybrid);
    }

    // ========================================================================
    // combine_weighted Tests
    // ========================================================================

    #[test]
    fn test_combine_weighted_equal_weights() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);
        let p = persona_decision(90, EngagementLevel::Full, 0.8);
        let l = llm_decision(80, EngagementLevel::Full, 0.6);
        let result = strategy.combine_weighted(&p, &l);
        assert_eq!(result.score, 85);
        assert!((result.confidence - 0.7).abs() < 0.01);
        assert_eq!(result.level, EngagementLevel::Full);
    }

    #[test]
    fn test_combine_weighted_persona_heavier() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.8, 0.2);
        let p = persona_decision(100, EngagementLevel::Full, 1.0);
        let l = llm_decision(0, EngagementLevel::None, 0.5);
        let result = strategy.combine_weighted(&p, &l);
        assert_eq!(result.score, 80);
        assert!((result.confidence - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_combine_weighted_zero_total() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.0, 0.0);
        let p = persona_decision(50, EngagementLevel::Medium, 0.7);
        let l = llm_decision(50, EngagementLevel::Medium, 0.7);
        let result = strategy.combine_weighted(&p, &l);
        // Should fall back to persona decision when total_weight == 0
        assert_eq!(result.score, p.score);
        assert_eq!(result.level, p.level);
    }

    #[test]
    fn test_combine_weighted_level_thresholds() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);

        let p = persona_decision(90, EngagementLevel::Full, 1.0);
        let l = llm_decision(90, EngagementLevel::Full, 1.0);
        assert_eq!(
            strategy.combine_weighted(&p, &l).level,
            EngagementLevel::Full
        );

        let p = persona_decision(50, EngagementLevel::Medium, 1.0);
        let l = llm_decision(50, EngagementLevel::Medium, 1.0);
        assert_eq!(
            strategy.combine_weighted(&p, &l).level,
            EngagementLevel::Medium
        );

        let p = persona_decision(30, EngagementLevel::Minimal, 1.0);
        let l = llm_decision(30, EngagementLevel::Minimal, 1.0);
        assert_eq!(
            strategy.combine_weighted(&p, &l).level,
            EngagementLevel::Minimal
        );

        let p = persona_decision(0, EngagementLevel::None, 1.0);
        let l = llm_decision(0, EngagementLevel::None, 1.0);
        assert_eq!(
            strategy.combine_weighted(&p, &l).level,
            EngagementLevel::None
        );
    }

    // ========================================================================
    // combine_best_confidence Tests
    // ========================================================================

    #[test]
    fn test_combine_best_confidence_llm_wins() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(50, EngagementLevel::Medium, 0.6);
        let l = llm_decision(80, EngagementLevel::Full, 0.9);
        let result = strategy.combine_best_confidence(&p, &l);
        assert_eq!(result.score, 80);
        assert!(result.reason.contains("LLM selected"));
    }

    #[test]
    fn test_combine_best_confidence_persona_wins() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(80, EngagementLevel::Full, 0.9);
        let l = llm_decision(50, EngagementLevel::Medium, 0.6);
        let result = strategy.combine_best_confidence(&p, &l);
        assert_eq!(result.score, 80);
        assert!(result.reason.contains("Persona selected"));
    }

    #[test]
    fn test_combine_best_confidence_equal_confidence() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(70, EngagementLevel::Full, 0.8);
        let l = llm_decision(60, EngagementLevel::Medium, 0.8);
        let result = strategy.combine_best_confidence(&p, &l);
        // Persona wins tie (llm.confidence > persona.confidence is false)
        assert!(result.reason.contains("Persona selected"));
    }

    // ========================================================================
    // combine_llm_primary Tests
    // ========================================================================

    #[test]
    fn test_combine_llm_primary_high_confidence_uses_llm() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(30, EngagementLevel::Minimal, 0.5);
        let l = llm_decision(80, EngagementLevel::Full, 0.8);
        let result = strategy.combine_llm_primary(&p, &l);
        assert_eq!(result.score, 80);
        assert!(result.reason.contains("LLM primary"));
    }

    #[test]
    fn test_combine_llm_primary_low_confidence_falls_back() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(70, EngagementLevel::Full, 0.7);
        let l = llm_decision(80, EngagementLevel::Full, 0.5);
        let result = strategy.combine_llm_primary(&p, &l);
        assert_eq!(result.score, 70);
        assert!(result.reason.contains("Persona fallback"));
    }

    #[test]
    fn test_combine_llm_primary_low_score_falls_back() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(70, EngagementLevel::Full, 0.7);
        let l = llm_decision(10, EngagementLevel::None, 0.9);
        let result = strategy.combine_llm_primary(&p, &l);
        assert_eq!(result.score, 70);
        assert!(result.reason.contains("Persona fallback"));
    }

    // ========================================================================
    // combine_consensus Tests
    // ========================================================================

    #[test]
    fn test_combine_consensus_persona_skip() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(0, EngagementLevel::None, 0.9);
        let l = llm_decision(80, EngagementLevel::Full, 0.8);
        let result = strategy.combine_consensus(&p, &l);
        assert_eq!(result.level, EngagementLevel::None);
        assert!(result.reason.contains("Consensus skip"));
    }

    #[test]
    fn test_combine_consensus_llm_skip() {
        let strategy = HybridStrategy::persona_only();
        let p = persona_decision(80, EngagementLevel::Full, 0.8);
        let l = llm_decision(0, EngagementLevel::None, 0.9);
        let result = strategy.combine_consensus(&p, &l);
        assert_eq!(result.level, EngagementLevel::None);
        assert!(result.reason.contains("Consensus skip"));
    }

    #[test]
    fn test_combine_consensus_both_agree() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);
        let p = persona_decision(90, EngagementLevel::Full, 0.8);
        let l = llm_decision(80, EngagementLevel::Full, 0.7);
        let result = strategy.combine_consensus(&p, &l);
        // Both agree to engage, uses weighted
        assert_eq!(result.level, EngagementLevel::Full);
        assert_eq!(result.score, 85);
    }

    // ========================================================================
    // CombinationStrategy Enum Tests
    // ========================================================================

    #[test]
    fn test_combination_strategy_variants() {
        assert_eq!(CombinationStrategy::WeightedAverage as u8, 0);
        assert_eq!(CombinationStrategy::BestConfidence as u8, 1);
        assert_eq!(CombinationStrategy::LLMPrimary as u8, 2);
        assert_eq!(CombinationStrategy::Consensus as u8, 3);
    }

    #[test]
    fn test_combination_strategy_partial_eq() {
        assert_eq!(
            CombinationStrategy::WeightedAverage,
            CombinationStrategy::WeightedAverage
        );
        assert_ne!(
            CombinationStrategy::WeightedAverage,
            CombinationStrategy::Consensus
        );
    }
}
