//! Persona-based decision strategy.
//!
//! Rule-based engine using persona weights and keyword analysis.
//! Ported from `twitteractivity_decision_persona.rs`.

use async_trait::async_trait;
use log::info;
use crate::utils::twitter::decision::strategies::DecisionStrategyImpl;
use crate::utils::twitter::decision::types::{DecisionStrategy, EngagementDecision, EngagementLevel, TweetContext};
use crate::utils::twitter::decision::strategies::legacy::LegacyStrategy;

/// Rule-based decision engine using persona configuration.
pub(crate) struct PersonaStrategy {
    controversial_keywords: Vec<&'static str>,
    spam_patterns: Vec<&'static str>,
    tragedy_keywords: Vec<&'static str>,
    crypto_keywords: Vec<&'static str>,
    /// Base legacy strategy for shared logic
    _base: LegacyStrategy,
}

impl PersonaStrategy {
    pub fn new() -> Self {
        Self {
            controversial_keywords: vec![
                "election", "vote", "democrat", "republican", "congress", "senate",
                "woke", "fascist", "liberal", "conservative", "biden", "trump",
                "abortion", "gun control", "immigration", "taxes",
                "exposed", "cancelled", "drama", "beef", "feud", "scandal",
                "controversy", "backlash", "callout", "nsfw", "onlyfans",
                "adult content", "xxx",
            ],
            spam_patterns: vec![
                "follow for follow", "f4f", "l4l", "like4like", "follow4follow",
                "check my bio", "link in bio", "dm me", "dm for", "1000x",
                "guaranteed gains", "buy now", "🚀🚀🚀", "💰💰💰",
            ],
            tragedy_keywords: vec![
                "passed away", "died", "death", "funeral", "grief", "mourning",
                "rest in peace", "rip", "lost my", "my grandmother", "my grandfather",
                "my mother died", "my father died", "miss her so much", "miss him so much",
                "devastated", "heartbroken", "tragedy", "accident", "cancer battle",
            ],
            crypto_keywords: vec![
                "nft", "crypto", "bitcoin", "ethereum", "blockchain", "buy my nft",
                "mint now", "limited nft", "airdrop", "token",
            ],
            _base: LegacyStrategy,
        }
    }

    /// Check if text contains any keywords from list.
    fn contains_any(&self, text: &str, keywords: &[&str]) -> bool {
        let text_lower = text.to_lowercase();
        keywords.iter().any(|kw| text_lower.contains(kw))
    }

    /// Calculate base score from persona weights.
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
        let positive_ratio = positive_signals as f64 / total;
        let negative_ratio = negative_signals as f64 / total;

        // Score based on positive ratio minus penalty for negative
        (positive_ratio * 100.0) - (negative_ratio * 50.0) + 30.0
    }
}

#[async_trait]
impl DecisionStrategyImpl for PersonaStrategy {
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        let text = &ctx.text;
        let replies_combined = ctx.replies.join(" ");
        let combined_text = format!("{} {}", text, replies_combined);

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
        let (level, multiplier, reason) = if final_score >= 75.0 {
            (
                EngagementLevel::Full,
                1.5f64.min(ctx.persona.interest_multiplier),
                "High quality content with positive reception".to_string(),
            )
        } else if final_score >= 50.0 {
            (
                EngagementLevel::Medium,
                1.2f64.min(ctx.persona.interest_multiplier),
                "Good content worth engaging".to_string(),
            )
        } else if final_score >= 30.0 {
            (
                EngagementLevel::Minimal,
                0.8,
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
            "PersonaStrategy: score={:.1}, level={:?}, multiplier={:.2}",
            final_score, level, multiplier
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
