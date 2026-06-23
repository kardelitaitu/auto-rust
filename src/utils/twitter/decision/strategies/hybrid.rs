//! Hybrid decision strategy combining multiple approaches.
//!
//! Uses weighted ensemble of `PersonaStrategy` and `LlmStrategy`.
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
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CombinationStrategy {
    /// Weighted average of all engine scores
    WeightedAverage,
    /// Pick the best (highest confidence) decision
    BestConfidence,
    /// Use LLM if available, fallback to Persona
    LLMPrimary,
    /// Always require Persona approval (conservative)
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

        // Weighted score (round to avoid floating-point truncation on negative values)
        let score = (f64::from(persona_decision.score) * p_norm
            + f64::from(llm_decision.score) * l_norm)
            .round() as i32;

        // Weighted multiplier
        let multiplier = persona_decision.multiplier * p_norm + llm_decision.multiplier * l_norm;

        // Average confidence
        let confidence = f64::midpoint(persona_decision.confidence, llm_decision.confidence);

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
    #[allow(clippy::unused_self)]
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
                score: i32::midpoint(persona_decision.score, llm_decision.score),
                reason: format!(
                    "Consensus skip: Persona={:?}, LLM={:?}",
                    persona_decision.level, llm_decision.level
                ),
                multiplier: 0.0,
                confidence: f64::midpoint(persona_decision.confidence, llm_decision.confidence),
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
    use crate::utils::twitter::twitteractivity_types::TweetId;

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
    // Boundary Score Tests
    // ========================================================================

    #[test]
    fn test_combine_weighted_boundary_full_75() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);
        let p = persona_decision(75, EngagementLevel::Full, 0.8);
        let l = llm_decision(75, EngagementLevel::Full, 0.8);
        let result = strategy.combine_weighted(&p, &l);
        assert_eq!(result.score, 75);
        assert_eq!(result.level, EngagementLevel::Full);
    }

    #[test]
    fn test_combine_weighted_boundary_below_full_74() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);
        let p = persona_decision(74, EngagementLevel::Medium, 0.8);
        let l = llm_decision(74, EngagementLevel::Medium, 0.8);
        let result = strategy.combine_weighted(&p, &l);
        assert_eq!(result.score, 74);
        assert_eq!(result.level, EngagementLevel::Medium);
    }

    #[test]
    fn test_combine_weighted_boundary_medium_50() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);
        let p = persona_decision(50, EngagementLevel::Medium, 0.8);
        let l = llm_decision(50, EngagementLevel::Medium, 0.8);
        let result = strategy.combine_weighted(&p, &l);
        assert_eq!(result.score, 50);
        assert_eq!(result.level, EngagementLevel::Medium);
    }

    #[test]
    fn test_combine_weighted_boundary_minimal_30() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);
        let p = persona_decision(30, EngagementLevel::Minimal, 0.8);
        let l = llm_decision(30, EngagementLevel::Minimal, 0.8);
        let result = strategy.combine_weighted(&p, &l);
        assert_eq!(result.score, 30);
        assert_eq!(result.level, EngagementLevel::Minimal);
    }

    #[test]
    fn test_combine_weighted_boundary_below_minimal_29() {
        let strategy = HybridStrategy::with_llm("key".to_string(), 0.5, 0.5);
        let p = persona_decision(29, EngagementLevel::None, 0.8);
        let l = llm_decision(29, EngagementLevel::None, 0.8);
        let result = strategy.combine_weighted(&p, &l);
        assert_eq!(result.score, 29);
        assert_eq!(result.level, EngagementLevel::None);
    }

    // ========================================================================
    // decide() Tests (Persona-only fallback path)
    // ========================================================================

    #[tokio::test]
    async fn test_decide_without_llm_uses_persona_only() {
        let strategy = HybridStrategy::persona_only();
        let ctx = TweetContext {
            tweet_id: TweetId::from_unchecked("1"),
            text: "Hello world".to_string(),
            author: "user".to_string(),
            replies: vec![],
            persona: crate::utils::twitter::twitteractivity_persona::PersonaWeights::default(),
            task_config: crate::utils::twitter::twitteractivity_state::TaskConfig::default(),
            tweet_age: "recent".to_string(),
        };
        let decision = strategy.decide(&ctx).await;
        // Persona-only path should return a valid decision
        assert!(decision.reason.contains("Persona only"));
    }

    #[tokio::test]
    async fn test_decide_with_empty_llm_key_falls_to_persona() {
        let strategy = HybridStrategy::with_llm(String::new(), 0.5, 0.5);
        let ctx = TweetContext {
            tweet_id: TweetId::from_unchecked("1"),
            text: "Hello world".to_string(),
            author: "user".to_string(),
            replies: vec![],
            persona: crate::utils::twitter::twitteractivity_persona::PersonaWeights::default(),
            task_config: crate::utils::twitter::twitteractivity_state::TaskConfig::default(),
            tweet_age: "recent".to_string(),
        };
        let decision = strategy.decide(&ctx).await;
        assert!(decision.reason.contains("Persona only"));
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

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Helper: build an EngagementDecision from generated fields
    fn make_dec(score: i32, level: EngagementLevel, conf: f64) -> EngagementDecision {
        EngagementDecision {
            level,
            score,
            reason: "t".into(),
            multiplier: 1.0,
            confidence: conf,
        }
    }

    fn any_level() -> impl Strategy<Value = EngagementLevel> {
        prop_oneof![
            Just(EngagementLevel::Full),
            Just(EngagementLevel::Medium),
            Just(EngagementLevel::Minimal),
            Just(EngagementLevel::None),
        ]
    }

    proptest! {
        // ============================================================
        // combine_weighted
        // ============================================================

        #[test]
        fn pt_weighted_score_in_bounds(
            ps in -200i32..=100, ls in -200i32..=100,
            pc in 0.0f64..=1.0, lc in 0.0f64..=1.0,
            pw in 0.0f64..=1.0, lw in 0.0f64..=1.0,
        ) {
            let s = HybridStrategy::with_llm("k".into(), pw, lw);
            let p = make_dec(ps, EngagementLevel::Full, pc);
            let l = make_dec(ls, EngagementLevel::Full, lc);
            let r = s.combine_weighted(&p, &l);
            // Score is weighted average → bounded by min/max inputs
            let min_score = ps.min(ls);
            let max_score = ps.max(ls);
            if pw + lw > 0.0 {
                prop_assert!(r.score >= min_score, "score {} < min {}", r.score, min_score);
                prop_assert!(r.score <= max_score, "score {} > max {}", r.score, max_score);
            }
        }

        #[test]
        fn pt_weighted_confidence_midpoint(
            pc in 0.0f64..=1.0, lc in 0.0f64..=1.0,
        ) {
            let s = HybridStrategy::with_llm("k".into(), 0.5, 0.5);
            let p = make_dec(50, EngagementLevel::Medium, pc);
            let l = make_dec(50, EngagementLevel::Medium, lc);
            let r = s.combine_weighted(&p, &l);
            let expected = f64::midpoint(pc, lc);
            prop_assert!((r.confidence - expected).abs() < f64::EPSILON,
                "conf {} != mid {} of {} and {}", r.confidence, expected, pc, lc);
        }

        #[test]
        fn pt_weighted_level_matches_score(
            ps in 0i32..=100, ls in 0i32..=100,
        ) {
            let s = HybridStrategy::with_llm("k".into(), 0.5, 0.5);
            let p = make_dec(ps, EngagementLevel::Full, 0.8);
            let l = make_dec(ls, EngagementLevel::Full, 0.8);
            let r = s.combine_weighted(&p, &l);
            let expected = if r.score >= 75 { EngagementLevel::Full }
                else if r.score >= 50 { EngagementLevel::Medium }
                else if r.score >= 30 { EngagementLevel::Minimal }
                else { EngagementLevel::None };
            prop_assert_eq!(r.level, expected, "score {} should be {:?}", r.score, expected);
        }

        #[test]
        fn pt_weighted_zero_total_falls_back(
            ps in -200i32..=100, level in any_level(),
        ) {
            let s = HybridStrategy::with_llm("k".into(), 0.0, 0.0);
            let p = make_dec(ps, level, 0.5);
            let l = make_dec(50, EngagementLevel::Medium, 0.8);
            let r = s.combine_weighted(&p, &l);
            prop_assert_eq!(r.score, p.score);
            prop_assert_eq!(r.level, p.level);
        }

        // ============================================================
        // combine_best_confidence
        // ============================================================

        #[test]
        fn pt_best_conf_llm_wins(
            ps in -200i32..=100, pc in 0.0f64..=1.0,
            ls in -200i32..=100, lc in 0.0f64..=1.0,
        ) {
            // Only test cases where LLM has strictly higher confidence
            prop_assume!(lc > pc);
            let s = HybridStrategy::persona_only();
            let p = make_dec(ps, EngagementLevel::Medium, pc);
            let l = make_dec(ls, EngagementLevel::Medium, lc);
            let r = s.combine_best_confidence(&p, &l);
            prop_assert_eq!(r.score, l.score);
            prop_assert!(r.reason.contains("LLM selected"));
        }

        #[test]
        fn pt_best_conf_persona_wins(
            ps in -200i32..=100, pc in 0.0f64..=1.0,
            ls in -200i32..=100, lc in 0.0f64..=1.0,
        ) {
            // Persona wins when LLM confidence is <= persona confidence
            prop_assume!(lc <= pc);
            let s = HybridStrategy::persona_only();
            let p = make_dec(ps, EngagementLevel::Medium, pc);
            let l = make_dec(ls, EngagementLevel::Medium, lc);
            let r = s.combine_best_confidence(&p, &l);
            prop_assert_eq!(r.score, p.score);
            prop_assert!(r.reason.contains("Persona selected"));
        }

        // ============================================================
        // combine_llm_primary
        // ============================================================

        #[test]
        fn pt_llm_primary_uses_llm_when_confident(
            lc in 0.7000001f64..=1.0, ls in 21i32..=100,
        ) {
            let s = HybridStrategy::persona_only();
            let p = make_dec(10, EngagementLevel::None, 0.3);
            let l = make_dec(ls, EngagementLevel::Full, lc);
            let r = s.combine_llm_primary(&p, &l);
            prop_assert_eq!(r.score, l.score);
            prop_assert!(r.reason.contains("LLM primary"));
        }

        #[test]
        fn pt_llm_primary_falls_back_when_low_confidence(
            lc in 0.0f64..=0.7, ls in -200i32..=100,
        ) {
            let s = HybridStrategy::persona_only();
            let p = make_dec(70, EngagementLevel::Full, 0.7);
            let l = make_dec(ls, EngagementLevel::Full, lc);
            let r = s.combine_llm_primary(&p, &l);
            prop_assert_eq!(r.score, p.score);
            prop_assert!(r.reason.contains("Persona fallback"));
        }

        #[test]
        fn pt_llm_primary_falls_back_when_low_score(
            lc in 0.7000001f64..=1.0, ls in -200i32..=20,
        ) {
            let s = HybridStrategy::persona_only();
            let p = make_dec(70, EngagementLevel::Full, 0.7);
            let l = make_dec(ls, EngagementLevel::Full, lc);
            let r = s.combine_llm_primary(&p, &l);
            prop_assert_eq!(r.score, p.score);
            prop_assert!(r.reason.contains("Persona fallback"));
        }

        // ============================================================
        // combine_consensus
        // ============================================================

        #[test]
        fn pt_consensus_skip_when_either_skips(
            pl in any_level(), ll in any_level(),
            ps in -200i32..=100, ls in -200i32..=100,
        ) {
            prop_assume!(pl == EngagementLevel::None || ll == EngagementLevel::None);
            let s = HybridStrategy::with_llm("k".into(), 0.5, 0.5);
            let p = make_dec(ps, pl, 0.8);
            let l = make_dec(ls, ll, 0.8);
            let r = s.combine_consensus(&p, &l);
            prop_assert_eq!(r.level, EngagementLevel::None);
            prop_assert!(r.reason.contains("Consensus skip"));
            prop_assert_eq!(r.multiplier, 0.0);
            let exp_score = i32::midpoint(ps, ls);
            prop_assert_eq!(r.score, exp_score, "score {} != mid {} of {} and {}", r.score, exp_score, ps, ls);
        }

        #[test]
        fn pt_consensus_both_engage_uses_weighted(
            ps in 0i32..=100, ls in 0i32..=100,
            pw in 0.0f64..=1.0, lw in 0.0f64..=1.0,
        ) {
            // Both levels are not None → both want to engage
            let s = HybridStrategy::with_llm("k".into(), pw, lw);
            let p = make_dec(ps, EngagementLevel::Full, 0.8);
            let l = make_dec(ls, EngagementLevel::Full, 0.8);
            let r = s.combine_consensus(&p, &l);
            // Should behave like combine_weighted
            let expected = s.combine_weighted(&p, &l);
            prop_assert_eq!(r.score, expected.score);
            prop_assert_eq!(r.level, expected.level);
            prop_assert_eq!(r.multiplier, expected.multiplier);
        }
    }
}
