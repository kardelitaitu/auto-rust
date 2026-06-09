//! Persona-based decision strategy.
//!
//! Rule-based engine using persona weights and keyword analysis.
//! Ported from `twitteractivity_decision_persona.rs`.

use crate::utils::twitter::decision::strategies::legacy::LegacyStrategy;
use crate::utils::twitter::decision::strategies::DecisionStrategyImpl;
use crate::utils::twitter::decision::types::{
    DecisionStrategy, EngagementDecision, EngagementLevel, TweetContext,
};
use async_trait::async_trait;
use log::info;

/// Rule-based decision engine using persona configuration.
pub(crate) struct PersonaStrategy {
    controversial_keywords: Vec<String>,
    spam_patterns: Vec<String>,
    tragedy_keywords: Vec<String>,
    crypto_keywords: Vec<String>,
    /// Base legacy strategy for shared logic
    _base: LegacyStrategy,
}

impl PersonaStrategy {
    pub fn new() -> Self {
        Self {
            controversial_keywords: serde_json::from_str(include_str!(
                "../../persona_keywords/controversial_keywords.json"
            ))
            .expect("invalid controversial_keywords.json"),
            spam_patterns: serde_json::from_str(include_str!(
                "../../persona_keywords/spam_patterns.json"
            ))
            .expect("invalid spam_patterns.json"),
            tragedy_keywords: serde_json::from_str(include_str!(
                "../../persona_keywords/tragedy_keywords.json"
            ))
            .expect("invalid tragedy_keywords.json"),
            crypto_keywords: serde_json::from_str(include_str!(
                "../../persona_keywords/crypto_keywords.json"
            ))
            .expect("invalid crypto_keywords.json"),
            _base: LegacyStrategy,
        }
    }

    /// Check if text contains any keywords from list.
    #[allow(clippy::unused_self)]
    fn contains_any(&self, text: &str, keywords: &[impl AsRef<str>]) -> bool {
        let text_lower = text.to_lowercase();
        keywords.iter().any(|kw| text_lower.contains(kw.as_ref()))
    }

    /// Calculate base score from persona weights.
    #[allow(clippy::unused_self)]
    fn calculate_base_score(&self, ctx: &TweetContext) -> f64 {
        let persona = &ctx.persona;

        // Average of engagement probabilities as base quality indicator
        let avg_prob = (persona.like_prob
            + persona.retweet_prob
            + persona.follow_prob
            + persona.reply_prob
            + persona.quote_prob
            + persona.bookmark_prob)
            / 6.0;

        // Scale to 0-100
        (avg_prob * 100.0).min(100.0)
    }

    /// Analyze replies for community reception.
    #[allow(clippy::cast_precision_loss, clippy::unused_self)]
    #[allow(clippy::cast_precision_loss, clippy::unused_self)]
    fn analyze_replies(&self, ctx: &TweetContext) -> f64 {
        if ctx.replies.is_empty() {
            return 50.0; // Neutral if no replies
        }

        let mut positive_signals = 0;
        let mut negative_signals = 0;

        for reply in &ctx.replies {
            let reply_lower = reply.to_lowercase();

            // Positive indicators
            if reply_lower.contains("congrats")
                || reply_lower.contains("great")
                || reply_lower.contains("awesome")
                || reply_lower.contains("love")
                || reply_lower.contains("thanks")
                || reply_lower.contains("agree")
                || reply_lower.contains("well said")
                || reply_lower.contains("looking forward")
                || reply_lower.contains("exciting")
            {
                positive_signals += 1;
            }

            // Negative indicators
            if reply_lower.contains("scam")
                || reply_lower.contains("spam")
                || reply_lower.contains("reported")
                || reply_lower.contains("blocked")
                || reply_lower.contains("fake")
                || reply_lower.contains("bot")
            {
                negative_signals += 1;
            }
        }

        let total = ctx.replies.len() as f64;
        let positive_ratio = f64::from(positive_signals) / total;
        let negative_ratio = f64::from(negative_signals) / total;

        // Score based on positive ratio minus penalty for negative
        (positive_ratio * 100.0) - (negative_ratio * 50.0) + 30.0
    }
}

#[async_trait]
impl DecisionStrategyImpl for PersonaStrategy {
    #[allow(clippy::cast_precision_loss)]
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        let text = &ctx.text;
        let replies_combined = ctx.replies.join(" ");
        let combined_text = format!("{text} {replies_combined}");

        // 1. CRITICAL: Check for tragedy (NEVER engage)
        if self.contains_any(&combined_text, &self.tragedy_keywords) {
            info!("PersonaStrategy: Tragedy detected, skipping");
            return EngagementDecision {
                level: EngagementLevel::None,
                score: 5,
                reason: "Personal tragedy - inappropriate to engage".to_string(),
                multiplier: 0.0,
                confidence: 0.95,
            };
        }

        // 2. CRITICAL: Check for spam/crypto (NEVER engage)
        if self.contains_any(&combined_text, &self.crypto_keywords)
            || self.contains_any(&combined_text, &self.spam_patterns)
        {
            info!("PersonaStrategy: Spam/crypto detected, skipping");
            return EngagementDecision {
                level: EngagementLevel::None,
                score: 5,
                reason: "Spam or promotional content detected".to_string(),
                multiplier: 0.0,
                confidence: 0.90,
            };
        }

        // 3. Check for controversial topics
        if self.contains_any(&combined_text, &self.controversial_keywords) {
            info!("PersonaStrategy: Controversial topic detected, low engagement");
            return EngagementDecision {
                level: EngagementLevel::Minimal,
                score: 25,
                reason: "Controversial topic - minimal engagement".to_string(),
                multiplier: 0.5,
                confidence: 0.75,
            };
        }

        // 4. Calculate scores
        let base_score = self.calculate_base_score(ctx);
        let reply_score = self.analyze_replies(ctx);
        let final_score = (base_score * 0.4 + reply_score * 0.6).min(100.0);

        // 5. Determine level and multiplier
        let im = ctx.persona.interest_multiplier;
        let (level, multiplier, reason) = if final_score >= 75.0 {
            (
                EngagementLevel::Full,
                (1.5 * im).clamp(0.0, 2.0),
                "High quality content with positive reception".to_string(),
            )
        } else if final_score >= 50.0 {
            (
                EngagementLevel::Medium,
                (1.2 * im).clamp(0.0, 2.0),
                "Good content worth engaging".to_string(),
            )
        } else if final_score >= 30.0 {
            (
                EngagementLevel::Minimal,
                (0.8 * im).clamp(0.0, 2.0),
                "Average content, limited engagement value".to_string(),
            )
        } else {
            (
                EngagementLevel::None,
                0.0,
                "Low engagement value".to_string(),
            )
        };

        info!(
            "PersonaStrategy: score={final_score:.1}, level={level:?}, multiplier={multiplier:.2}"
        );

        EngagementDecision {
            level,
            score: final_score as i32,
            reason,
            multiplier,
            confidence: 0.70,
        }
    }

    fn strategy_type(&self) -> DecisionStrategy {
        DecisionStrategy::Persona
    }

    fn name(&self) -> &'static str {
        "persona"
    }
}

impl Default for PersonaStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use crate::utils::twitter::twitteractivity_state::TaskConfig;

    fn persona_with_probs(
        like: f64,
        retweet: f64,
        quote: f64,
        follow: f64,
        reply: f64,
        bookmark: f64,
        multiplier: f64,
    ) -> PersonaWeights {
        PersonaWeights {
            like_prob: like,
            retweet_prob: retweet,
            quote_prob: quote,
            follow_prob: follow,
            reply_prob: reply,
            bookmark_prob: bookmark,
            thread_dive_prob: 0.0,
            interest_multiplier: multiplier,
        }
    }

    fn ctx(text: &str, replies: Vec<&str>, persona: PersonaWeights) -> TweetContext {
        TweetContext {
            tweet_id: "1".to_string(),
            text: text.to_string(),
            author: "user".to_string(),
            replies: replies.into_iter().map(String::from).collect(),
            persona,
            task_config: TaskConfig::default(),
            tweet_age: "recent".to_string(),
        }
    }

    // ========================================================================
    // PersonaStrategy Construction Tests
    // ========================================================================

    #[test]
    fn test_persona_strategy_new() {
        let strategy = PersonaStrategy::new();
        assert!(!strategy.controversial_keywords.is_empty());
        assert!(!strategy.spam_patterns.is_empty());
        assert!(!strategy.tragedy_keywords.is_empty());
        assert!(!strategy.crypto_keywords.is_empty());
    }

    #[test]
    fn test_persona_strategy_default() {
        let strategy = PersonaStrategy::default();
        let new = PersonaStrategy::new();
        assert_eq!(
            strategy.controversial_keywords.len(),
            new.controversial_keywords.len()
        );
    }

    // ========================================================================
    // contains_any Tests
    // ========================================================================

    #[test]
    fn test_contains_any_match() {
        let strategy = PersonaStrategy::new();
        assert!(strategy.contains_any("this is a scam", &["scam", "spam"]));
    }

    #[test]
    fn test_contains_any_no_match() {
        let strategy = PersonaStrategy::new();
        assert!(!strategy.contains_any("hello world", &["scam", "spam"]));
    }

    #[test]
    fn test_contains_any_empty_text() {
        let strategy = PersonaStrategy::new();
        assert!(!strategy.contains_any("", &["test"]));
    }

    #[test]
    fn test_contains_any_empty_keywords() {
        let strategy = PersonaStrategy::new();
        assert!(!strategy.contains_any("test", &[] as &[&str]));
    }

    #[test]
    fn test_contains_any_case_insensitive() {
        let strategy = PersonaStrategy::new();
        assert!(strategy.contains_any("ELECTION", &["election"]));
        assert!(strategy.contains_any("Election", &["election"]));
    }

    // ========================================================================
    // calculate_base_score Tests
    // ========================================================================

    #[test]
    fn test_calculate_base_score_default() {
        let strategy = PersonaStrategy::new();
        let context = ctx("text", vec![], PersonaWeights::default());
        let score = strategy.calculate_base_score(&context);
        // Default weights: like=0.3, retweet=0.1, quote=0.05, follow=0.05, reply=0.02, bookmark=0.0
        // sum = 0.52, avg = 0.0867, *100 = 8.67
        assert!((score - 8.67).abs() < 0.1, "expected ~8.67, got {}", score);
    }

    #[test]
    fn test_calculate_base_score_high_engagement() {
        let strategy = PersonaStrategy::new();
        let weights = persona_with_probs(1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
        let context = ctx("text", vec![], weights);
        let score = strategy.calculate_base_score(&context);
        assert!((score - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_base_score_zero_engagement() {
        let strategy = PersonaStrategy::new();
        let weights = persona_with_probs(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let context = ctx("text", vec![], weights);
        let score = strategy.calculate_base_score(&context);
        assert!((score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_calculate_base_score_capped_at_100() {
        let strategy = PersonaStrategy::new();
        // All probabilities at 2.0 (over 1.0) → avg = 2.0 → *100 = 200 → capped at 100
        let weights = persona_with_probs(2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 1.0);
        let context = ctx("text", vec![], weights);
        let score = strategy.calculate_base_score(&context);
        assert!((score - 100.0).abs() < 0.01);
    }

    // ========================================================================
    // analyze_replies Tests
    // ========================================================================

    #[test]
    fn test_analyze_replies_empty_returns_neutral() {
        let strategy = PersonaStrategy::new();
        let context = ctx("text", vec![], PersonaWeights::default());
        let score = strategy.analyze_replies(&context);
        assert!((score - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_analyze_replies_positive() {
        let strategy = PersonaStrategy::new();
        let context = ctx(
            "text",
            vec!["Great post!", "I agree with this", "exciting stuff"],
            PersonaWeights::default(),
        );
        let score = strategy.analyze_replies(&context);
        // 3 replies, 3 positive → (3/3 * 100) - (0/3 * 50) + 30 = 130
        assert!((score - 130.0).abs() < 0.1, "expected ~130, got {}", score);
    }

    #[test]
    fn test_analyze_replies_negative() {
        let strategy = PersonaStrategy::new();
        let context = ctx(
            "text",
            vec!["this is scam", "fake news", "reported"],
            PersonaWeights::default(),
        );
        let score = strategy.analyze_replies(&context);
        // 3 replies, 3 negative → (0/3 * 100) - (3/3 * 50) + 30 = -20
        assert!((score - (-20.0)).abs() < 0.1, "expected -20, got {}", score);
    }

    #[test]
    fn test_analyze_replies_mixed() {
        let strategy = PersonaStrategy::new();
        let context = ctx(
            "text",
            vec!["Great post!", "this is spam", "I agree"],
            PersonaWeights::default(),
        );
        let score = strategy.analyze_replies(&context);
        // 3 replies, 2 positive → (2/3 * 100) - (1/3 * 50) + 30
        // 66.67 - 16.67 + 30 = 80.0
        assert!((score - 80.0).abs() < 0.5, "expected ~80, got {}", score);
    }

    #[test]
    fn test_analyze_replies_no_signals() {
        let strategy = PersonaStrategy::new();
        let context = ctx(
            "text",
            vec!["just a normal reply", "nothing special"],
            PersonaWeights::default(),
        );
        let score = strategy.analyze_replies(&context);
        // 2 replies, 0 positive, 0 negative → (0/2 * 100) - (0/2 * 50) + 30 = 30
        assert!((score - 30.0).abs() < 0.1);
    }

    // ========================================================================
    // decide Tests (sync helpers, not async)
    // ========================================================================

    #[test]
    fn test_persona_name() {
        let strategy = PersonaStrategy::new();
        assert_eq!(strategy.name(), "persona");
    }

    #[test]
    fn test_persona_strategy_type() {
        let strategy = PersonaStrategy::new();
        assert_eq!(strategy.strategy_type(), DecisionStrategy::Persona);
    }

    #[tokio::test]
    async fn test_decide_tragedy_skips() {
        let strategy = PersonaStrategy::new();
        let context = ctx(
            "He passed away yesterday",
            vec![],
            PersonaWeights::default(),
        );
        let decision = strategy.decide(&context).await;
        assert_eq!(decision.level, EngagementLevel::None);
        assert!(decision.reason.contains("tragedy"));
    }

    #[tokio::test]
    async fn test_decide_spam_skips() {
        let strategy = PersonaStrategy::new();
        let context = ctx(
            "check my bio for more info",
            vec![],
            PersonaWeights::default(),
        );
        let decision = strategy.decide(&context).await;
        assert_eq!(decision.level, EngagementLevel::None);
        assert!(decision.reason.contains("Spam"));
    }

    #[tokio::test]
    async fn test_decide_crypto_skips() {
        let strategy = PersonaStrategy::new();
        let context = ctx("Buy my NFT now!", vec![], PersonaWeights::default());
        let decision = strategy.decide(&context).await;
        assert_eq!(decision.level, EngagementLevel::None);
    }

    #[tokio::test]
    async fn test_decide_controversial_minimal() {
        let strategy = PersonaStrategy::new();
        let context = ctx(
            "The election results are in",
            vec![],
            PersonaWeights::default(),
        );
        let decision = strategy.decide(&context).await;
        assert_eq!(decision.level, EngagementLevel::Minimal);
    }

    #[tokio::test]
    async fn test_decide_neutral_content() {
        let strategy = PersonaStrategy::new();
        let context = ctx("I like pizza", vec![], PersonaWeights::default());
        let decision = strategy.decide(&context).await;
        // Default weights give base_score ~8.67, reply_score 50, final = 8.67*0.4 + 50*0.6 = 33.47
        assert_eq!(decision.level, EngagementLevel::Minimal);
        assert!(decision.score > 0);
    }
    #[tokio::test]
    async fn test_decide_positive_with_good_replies() {
        let strategy = PersonaStrategy::new();
        let weights = persona_with_probs(1.0, 1.0, 0.5, 0.5, 0.5, 0.5, 1.5);
        let context = ctx(
            "A great and amazing post",
            vec!["congrats!", "awesome work!", "totally agree"],
            weights,
        );
        let decision = strategy.decide(&context).await;
        // High persona weights + positive replies should yield Medium or Full
        assert!(
            decision.level == EngagementLevel::Medium || decision.level == EngagementLevel::Full,
            "expected Medium or Full, got {:?}",
            decision.level
        );
        assert!(decision.score > 50);
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[tokio::test]
    async fn test_decide_replies_with_empty_strings() {
        let strategy = PersonaStrategy::new();
        let context = ctx("hello", vec!["", "   "], PersonaWeights::default());
        let decision = strategy.decide(&context).await;
        // Empty strings should not crash — treated as neutral
        assert!(decision.score >= 0);
    }

    #[test]
    fn test_analyze_replies_large_count() {
        let strategy = PersonaStrategy::new();
        let many_positive: Vec<&str> = (0..100).map(|_| "Great post!").collect();
        let context = ctx("text", many_positive, PersonaWeights::default());
        let score = strategy.analyze_replies(&context);
        // 100/100 positive = 100.0 - 0 + 30 = 130
        assert!((score - 130.0).abs() < 0.1, "expected ~130, got {}", score);
    }

    #[test]
    fn test_analyze_replies_with_large_negative_count() {
        let strategy = PersonaStrategy::new();
        let many_negative: Vec<&str> = (0..50).map(|_| "this is a scam").collect();
        let context = ctx("text", many_negative, PersonaWeights::default());
        let score = strategy.analyze_replies(&context);
        // 50/50 negative = 0 - 50 + 30 = -20
        assert!((score - (-20.0)).abs() < 0.1, "expected -20, got {}", score);
    }

    #[test]
    fn test_calculate_base_score_single_high_prob() {
        let strategy = PersonaStrategy::new();
        // One probability high, rest zero
        let weights = persona_with_probs(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let context = ctx("text", vec![], weights);
        let score = strategy.calculate_base_score(&context);
        // (1.0 + 0 + 0 + 0 + 0 + 0) / 6 * 100 = 16.67
        assert!(
            (score - 16.67).abs() < 0.1,
            "expected ~16.67, got {}",
            score
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    use proptest::test_runner::{Config, TestRunner};

    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use crate::utils::twitter::twitteractivity_state::TaskConfig;

    fn make_persona(
        like: f64,
        rt: f64,
        quote: f64,
        follow: f64,
        reply: f64,
        bm: f64,
        im: f64,
    ) -> PersonaWeights {
        PersonaWeights {
            like_prob: like,
            retweet_prob: rt,
            quote_prob: quote,
            follow_prob: follow,
            reply_prob: reply,
            bookmark_prob: bm,
            thread_dive_prob: 0.0,
            interest_multiplier: im,
        }
    }

    fn make_ctx(text: &str, replies: Vec<&str>, persona: PersonaWeights) -> TweetContext {
        TweetContext {
            tweet_id: "pt-1".into(),
            text: text.into(),
            author: "u".into(),
            replies: replies.into_iter().map(String::from).collect(),
            persona,
            task_config: TaskConfig::default(),
            tweet_age: "r".into(),
        }
    }

    // =============================================================
    // calculate_base_score Property Tests
    // =============================================================

    #[test]
    fn pt_base_bounded() {
        let mut runner = TestRunner::new(Config::default());
        let strat = (
            0.0f64..5.0,
            0.0f64..5.0,
            0.0f64..5.0,
            0.0f64..5.0,
            0.0f64..5.0,
            0.0f64..5.0,
        );
        let result = runner.run(&strat, |(like, rt, quote, follow, reply, bm)| {
            let s = PersonaStrategy::new();
            let p = make_persona(like, rt, quote, follow, reply, bm, 1.0);
            let score = s.calculate_base_score(&make_ctx("t", vec![], p));
            prop_assert!((0.0..=100.0).contains(&score));
            Ok(())
        });
        assert!(result.is_ok(), "proptest failed: {:?}", result);
    }

    #[test]
    fn pt_base_zero() {
        let s = PersonaStrategy::new();
        let p = make_persona(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0);
        let score = s.calculate_base_score(&make_ctx("t", vec![], p));
        assert!((score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn pt_base_capped() {
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&(1.0f64..10.0), |v| {
            let s = PersonaStrategy::new();
            let p = make_persona(v, v, v, v, v, v, 1.0);
            let score = s.calculate_base_score(&make_ctx("t", vec![], p));
            prop_assert!((score - 100.0).abs() < 0.01);
            Ok(())
        });
        assert!(result.is_ok(), "proptest failed: {:?}", result);
    }

    // =============================================================
    // analyze_replies Property Tests
    // =============================================================

    #[test]
    fn pt_reply_empty() {
        let s = PersonaStrategy::new();
        let score = s.analyze_replies(&make_ctx("t", vec![], PersonaWeights::default()));
        assert!((score - 50.0).abs() < 0.01);
    }

    #[test]
    fn pt_reply_pos() {
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&(1usize..20), |count| {
            let s = PersonaStrategy::new();
            let replies: Vec<String> = (0..count).map(|_| "Great post".to_string()).collect();
            let score = s.analyze_replies(&make_ctx(
                "t",
                replies.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                PersonaWeights::default(),
            ));
            prop_assert!((score - 130.0).abs() < 0.01);
            Ok(())
        });
        assert!(result.is_ok(), "proptest failed: {:?}", result);
    }

    #[test]
    fn pt_reply_neg() {
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&(1usize..20), |count| {
            let s = PersonaStrategy::new();
            let replies: Vec<String> = (0..count).map(|_| "this is a scam".to_string()).collect();
            let score = s.analyze_replies(&make_ctx(
                "t",
                replies.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                PersonaWeights::default(),
            ));
            prop_assert!((score + 20.0).abs() < 0.01);
            Ok(())
        });
        assert!(result.is_ok(), "proptest failed: {:?}", result);
    }

    // =============================================================
    // contains_any Property Tests
    // =============================================================

    #[test]
    fn pt_contains_find() {
        let mut runner = TestRunner::new(Config::default());
        let strat = (any::<String>(), any::<String>());
        let result = runner.run(&strat, |(prefix, suffix)| {
            let s = PersonaStrategy::new();
            let text = format!("{}x{}", prefix, suffix);
            prop_assert!(s.contains_any(&text, &["x"]));
            Ok(())
        });
        assert!(result.is_ok(), "proptest failed: {:?}", result);
    }

    #[test]
    fn pt_contains_case() {
        let mut runner = TestRunner::new(Config::default());
        let strat = (any::<String>(), any::<String>());
        let result = runner.run(&strat, |(prefix, suffix)| {
            let s = PersonaStrategy::new();
            let text = format!("{}X{}", prefix, suffix);
            prop_assert!(s.contains_any(&text, &["x"]));
            Ok(())
        });
        assert!(result.is_ok(), "proptest failed: {:?}", result);
    }

    #[test]
    fn pt_contains_empty_text() {
        assert!(!PersonaStrategy::new().contains_any("", &["a", "b"]));
    }

    #[test]
    fn pt_contains_empty_kw() {
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&any::<String>(), |text| {
            prop_assert!(!PersonaStrategy::new().contains_any(&text, &[] as &[&str]));
            Ok(())
        });
        assert!(result.is_ok(), "proptest failed: {:?}", result);
    }
}
