//! Legacy rule-based decision strategy.
//!
//! Ported from the original `twitteractivity_decision.rs` module.

use async_trait::async_trait;
use crate::utils::twitter::decision::types::{EngagementDecision, EngagementLevel, TweetContext};

// ============================================================================
// Keyword Blocklists
// ============================================================================

/// Controversial topics to avoid (politics, drama, conflict)
const CONTROVERSIAL_TOPICS: &[&str] = &[
    // Politics
    "election",
    "vote",
    "democrat",
    "republican",
    "congress",
    "senate",
    "woke",
    "fascist",
    "liberal",
    "conservative",
    "biden",
    "trump",
    "abortion",
    "gun control",
    "immigration",
    "taxes",
    // Drama/Conflict
    "exposed",
    "cancelled",
    "drama",
    "beef",
    "feud",
    "scandal",
    "controversy",
    "backlash",
    "callout",
    // NSFW
    "nsfw",
    "onlyfans",
    "adult content",
    "xxx",
];

/// Spam indicators
const SPAM_PATTERNS: &[&str] = &[
    "follow for follow",
    "f4f",
    "l4l",
    "like4like",
    "follow4follow",
    "check my bio",
    "link in bio",
    "dm me",
    "dm for",
    "crypto",
    "giveaway",
    "win bitcoin",
    "free eth",
    "nft drop",
    "make money fast",
    "work from home",
    "passive income",
];

/// Negative sentiment words
const NEGATIVE_WORDS: &[&str] = &[
    "hate",
    "disgusting",
    "terrible",
    "awful",
    "worst",
    "idiot",
    "stupid",
    "dumb",
    "moron",
    "cry",
    "die",
    "kill",
    "suicide",
    "death",
    "sad",
    "angry",
    "upset",
    "disappointed",
    "frustrated",
];

/// Positive sentiment words (quality boosters)
const POSITIVE_WORDS: &[&str] = &[
    "great",
    "amazing",
    "awesome",
    "excellent",
    "wonderful",
    "love",
    "thanks",
    "thank you",
    "appreciate",
    "grateful",
    "happy",
    "excited",
    "proud",
    "congrats",
    "congratulations",
    "beautiful",
    "fantastic",
    "incredible",
    "inspiring",
];

use crate::utils::twitter::decision::strategies::DecisionStrategyImpl;
use crate::utils::twitter::decision::types::DecisionStrategy;

/// Legacy rule-based strategy implementation.
pub(crate) struct LegacyStrategy;

#[async_trait]
impl DecisionStrategyImpl for LegacyStrategy {
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision {
        self.decide_legacy(ctx)
    }

    fn strategy_type(&self) -> DecisionStrategy {
        DecisionStrategy::Legacy
    }

    fn name(&self) -> &'static str {
        "legacy"
    }
}

impl LegacyStrategy {
    /// Evaluates a tweet and returns the appropriate engagement level.
    pub fn decide_legacy(&self, ctx: &TweetContext) -> EngagementDecision {
        let text_lower = ctx.text.to_lowercase();

        // 1. Check hard blocklists (instant skip)
        if self.contains_any(&text_lower, CONTROVERSIAL_TOPICS) {
            return EngagementDecision {
                level: EngagementLevel::None,
                score: 0,
                reason: "controversial topic".to_string(),
                multiplier: 0.0,
                confidence: 0.95,
            };
        }

        if self.contains_any(&text_lower, SPAM_PATTERNS) {
            return EngagementDecision {
                level: EngagementLevel::None,
                score: 0,
                reason: "spam content".to_string(),
                multiplier: 0.0,
                confidence: 0.95,
            };
        }

        // 2. Calculate quality score
        let mut score = 0;
        score += self.calculate_quality_signals(&text_lower, &ctx.text);
        score -= self.calculate_penalty_signals(&text_lower, &ctx.text);

        // 3. Analyze replies for community sentiment
        let reply_analysis = self.analyze_replies(&ctx.replies);

        if reply_analysis.negative_ratio > 0.5 {
            score -= 30; // Penalty for negative community response
        }
        if reply_analysis.spam_ratio > 0.3 {
            score -= 50; // Penalty for spammy replies
        }

        // 4. Determine engagement level based on score
        let (level, reason, multiplier) = if score >= 60 {
            (
                EngagementLevel::Full,
                "high quality content".to_string(),
                1.5,
            )
        } else if score >= 30 {
            (
                EngagementLevel::Medium,
                "medium quality content".to_string(),
                1.0,
            )
        } else if score >= 10 {
            (
                EngagementLevel::Minimal,
                "low quality, like only".to_string(),
                0.5,
            )
        } else {
            (EngagementLevel::None, "skip: low score".to_string(), 0.0)
        };

        EngagementDecision {
            level,
            score,
            reason,
            multiplier,
            confidence: 0.70,
        }
    }

    /// Calculate positive quality signals.
    fn calculate_quality_signals(&self, text_lower: &str, original_text: &str) -> i32 {
        let mut score = 0;

        // Has image/video (+20)
        if original_text.contains("pic.twitter.com") || original_text.contains("t.co/") {
            score += 20;
        }

        // Question asked (+15)
        if original_text.contains('?') {
            score += 15;
        }

        // Thread indicator (+25)
        if original_text.contains("1/") || original_text.contains("🧵") {
            score += 25;
        }

        // Multiple sentences (+10)
        let sentence_count = original_text.matches('.').count();
        if sentence_count >= 2 {
            score += 10;
        }

        // Positive words (+20)
        if self.contains_any(text_lower, POSITIVE_WORDS) {
            score += 20;
        }

        // Long form content (+15)
        if original_text.len() > 200 {
            score += 15;
        }

        score
    }

    /// Calculate penalty signals.
    fn calculate_penalty_signals(&self, text_lower: &str, original_text: &str) -> i32 {
        let mut penalty = 0;

        // All caps (-30)
        let alpha_chars: String = original_text
            .chars()
            .filter(|c| c.is_alphabetic())
            .collect();
        if !alpha_chars.is_empty() && alpha_chars.chars().all(|c| c.is_uppercase()) {
            penalty += 30;
        }

        // Excessive hashtags (-20)
        let hashtag_count = original_text.matches('#').count();
        if hashtag_count >= 3 {
            penalty += 20;
        }

        // Excessive emojis (-15)
        let emoji_count = original_text.chars().filter(|c| self.is_emoji(*c)).count();
        if emoji_count >= 5 {
            penalty += 15;
        }

        // Negative words (-40)
        if self.contains_any(text_lower, NEGATIVE_WORDS) {
            penalty += 40;
        }

        // Very short tweet (-10)
        if original_text.len() < 20 {
            penalty += 10;
        }

        penalty
    }

    /// Analyzes the sentiment and quality of replies.
    fn analyze_replies(&self, replies: &[String]) -> ReplyAnalysis {
        if replies.is_empty() {
            return ReplyAnalysis {
                positive_ratio: 0.0,
                negative_ratio: 0.0,
                spam_ratio: 0.0,
            };
        }

        let mut positive_count = 0;
        let mut negative_count = 0;
        let mut spam_count = 0;

        for text in replies {
            let text_lower = text.to_lowercase();

            if self.contains_any(&text_lower, SPAM_PATTERNS) {
                spam_count += 1;
            } else if self.contains_any(&text_lower, POSITIVE_WORDS) {
                positive_count += 1;
            } else if self.contains_any(&text_lower, NEGATIVE_WORDS) {
                negative_count += 1;
            }
        }

        let total = replies.len() as f64;

        ReplyAnalysis {
            positive_ratio: positive_count as f64 / total,
            negative_ratio: negative_count as f64 / total,
            spam_ratio: spam_count as f64 / total,
        }
    }

    /// Check if text contains any of the given patterns.
    fn contains_any(&self, text: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| text.contains(pattern))
    }

    /// Check if a character is an emoji.
    fn is_emoji(&self, c: char) -> bool {
        let cp = c as u32;
        // Common emoji Unicode ranges
        (0x1F600..=0x1F64F).contains(&cp) ||  // Emoticons
        (0x1F300..=0x1F5FF).contains(&cp) ||  // Misc Symbols and Pictographs
        (0x1F680..=0x1F6FF).contains(&cp) ||  // Transport and Map
        (0x1F1E0..=0x1F1FF).contains(&cp) ||  // Flags
        (0x2600..=0x26FF).contains(&cp) ||    // Misc symbols
        (0x2700..=0x27BF).contains(&cp) // Dingbats
    }
}

/// Analysis results for tweet replies.
struct ReplyAnalysis {
    #[allow(dead_code)]
    positive_ratio: f64,
    negative_ratio: f64,
    spam_ratio: f64,
}
