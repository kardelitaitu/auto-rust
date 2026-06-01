//! Legacy rule-based decision strategy.
//!
//! Ported from the original `twitteractivity_decision.rs` module.

use crate::utils::twitter::decision::types::{EngagementDecision, EngagementLevel, TweetContext};
use async_trait::async_trait;

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
        if !alpha_chars.is_empty() && alpha_chars.chars().all(char::is_uppercase) {
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
    #[allow(clippy::cast_precision_loss, clippy::unused_self)]
    #[allow(clippy::cast_precision_loss, clippy::unused_self)]
    #[allow(clippy::cast_precision_loss, clippy::unused_self)]
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
            positive_ratio: f64::from(positive_count) / total,
            negative_ratio: f64::from(negative_count) / total,
            spam_ratio: f64::from(spam_count) / total,
        }
    }

    /// Check if text contains any of the given patterns.
    #[allow(clippy::unused_self)]
    fn contains_any(&self, text: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| text.contains(pattern))
    }

    /// Check if a character is an emoji.
    #[allow(clippy::unused_self)]
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
#[derive(Debug)]
struct ReplyAnalysis {
    #[allow(dead_code)]
    positive_ratio: f64,
    negative_ratio: f64,
    spam_ratio: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use crate::utils::twitter::twitteractivity_state::TaskConfig;

    fn default_tweet_context(text: &str) -> TweetContext {
        TweetContext {
            tweet_id: "test-1".to_string(),
            text: text.to_string(),
            author: "testuser".to_string(),
            replies: vec![],
            persona: PersonaWeights::default(),
            task_config: TaskConfig::default(),
            tweet_age: "recent".to_string(),

        }
    }

    fn ctx_with_replies(text: &str, replies: Vec<&str>) -> TweetContext {
        TweetContext {
            replies: replies.into_iter().map(String::from).collect(),
            ..default_tweet_context(text)
        }
    }

    // ========================================================================
    // contains_any Tests
    // ========================================================================

    #[test]
    fn test_contains_any_match() {
        let strategy = LegacyStrategy;
        assert!(strategy.contains_any("follow for follow back", SPAM_PATTERNS));
    }

    #[test]
    fn test_contains_any_no_match() {
        let strategy = LegacyStrategy;
        assert!(!strategy.contains_any("this is perfectly fine content", SPAM_PATTERNS));
    }

    #[test]
    fn test_contains_any_empty_text() {
        let strategy = LegacyStrategy;
        assert!(!strategy.contains_any("", SPAM_PATTERNS));
    }

    #[test]
    fn test_contains_any_empty_patterns() {
        let strategy = LegacyStrategy;
        assert!(!strategy.contains_any("some text", &[]));
    }

    #[test]
    fn test_contains_any_case_sensitive_handling() {
        let strategy = LegacyStrategy;
        // The method receives text_lower already lowercased
        assert!(strategy.contains_any("scandal", CONTROVERSIAL_TOPICS));
    }

    // ========================================================================
    // is_emoji Tests
    // ========================================================================

    #[test]
    fn test_is_emoji_true() {
        let strategy = LegacyStrategy;
        assert!(strategy.is_emoji('😊'));
        assert!(strategy.is_emoji('🔥'));
        assert!(strategy.is_emoji('🚀'));
        assert!(strategy.is_emoji('❤'));
    }

    #[test]
    fn test_is_emoji_false() {
        let strategy = LegacyStrategy;
        assert!(!strategy.is_emoji('a'));
        assert!(!strategy.is_emoji('1'));
        assert!(!strategy.is_emoji('.'));
        assert!(!strategy.is_emoji(' '));
    }

    #[test]
    fn test_is_emoji_ascii_boundary() {
        let strategy = LegacyStrategy;
        assert!(!strategy.is_emoji('\x00'));
        assert!(!strategy.is_emoji('\x7f'));
    }

    // ========================================================================
    // calculate_quality_signals Tests
    // ========================================================================

    #[test]
    fn test_quality_media_link() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_quality_signals("", "check pic.twitter.com/abc123");
        assert!(score >= 20);
    }

    #[test]
    fn test_quality_question() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_quality_signals("", "What do you think about this?");
        assert!(score >= 15);
    }

    #[test]
    fn test_quality_long_content() {
        let strategy = LegacyStrategy;
        let long = "a".repeat(201);
        let score = strategy.calculate_quality_signals("", &long);
        assert!(score >= 15);
    }

    #[test]
    fn test_quality_positive_words() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_quality_signals("this is great and amazing", "");
        assert!(score >= 20);
    }

    #[test]
    fn test_quality_combined() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_quality_signals(
            "this is great and amazing",
            "What do you think? check pic.twitter.com/abc. ",
        );
        // media(20) + question(15) + positive(20) = 55
        assert!(score >= 55);
    }

    #[test]
    fn test_quality_no_signals() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_quality_signals("ok", "hi");
        assert_eq!(score, 0);
    }

    // ========================================================================
    // calculate_penalty_signals Tests
    // ========================================================================

    #[test]
    fn test_penalty_all_caps() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_penalty_signals("", "THIS IS ALL CAPS");
        assert!(score >= 30);
    }

    #[test]
    fn test_penalty_excessive_hashtags() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_penalty_signals("", "#a #b #c #d");
        assert!(score >= 20);
    }

    #[test]
    fn test_penalty_excessive_emojis() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_penalty_signals("", "😊😊😊😊😊");
        assert!(score >= 15);
    }

    #[test]
    fn test_penalty_negative_words() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_penalty_signals("this is terrible and awful", "");
        assert!(score >= 40);
    }

    #[test]
    fn test_penalty_very_short() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_penalty_signals("", "hi");
        assert!(score >= 10);
    }

    #[test]
    fn test_penalty_no_penalties() {
        let strategy = LegacyStrategy;
        let score = strategy.calculate_penalty_signals(
            "this is fine",
            "This is a perfectly normal tweet with no issues",
        );
        assert_eq!(score, 0);
    }

    #[test]
    fn test_penalty_only_short_non_caps() {
        let strategy = LegacyStrategy;
        // Short text (< 20 chars), not all caps → only short penalty
        let score = strategy.calculate_penalty_signals("", "Hi there");
        assert_eq!(score, 10);
    }

    #[test]
    fn test_penalty_mixed_case_long_enough() {
        let strategy = LegacyStrategy;
        // 22 chars, not all caps → no penalties
        let score = strategy.calculate_penalty_signals("", "This is Mixed Case here");
        assert_eq!(score, 0);
    }

    // ========================================================================
    // analyze_replies Tests
    // ========================================================================

    #[test]
    fn test_analyze_replies_empty() {
        let strategy = LegacyStrategy;
        let analysis = strategy.analyze_replies(&[]);
        assert_eq!(analysis.positive_ratio, 0.0);
        assert_eq!(analysis.negative_ratio, 0.0);
        assert_eq!(analysis.spam_ratio, 0.0);
    }

    #[test]
    fn test_analyze_replies_positive() {
        let strategy = LegacyStrategy;
        let analysis =
            strategy.analyze_replies(&["This is great!".to_string(), "Amazing post".to_string()]);
        assert!(analysis.positive_ratio > 0.0);
        assert_eq!(analysis.spam_ratio, 0.0);
    }

    #[test]
    fn test_analyze_replies_spam() {
        let strategy = LegacyStrategy;
        let analysis = strategy.analyze_replies(&["check my bio".to_string(), "dm me".to_string()]);
        assert!(analysis.spam_ratio > 0.0);
    }

    #[test]
    fn test_analyze_replies_negative() {
        let strategy = LegacyStrategy;
        let analysis = strategy
            .analyze_replies(&["this is terrible".to_string(), "awful content".to_string()]);
        assert!(analysis.negative_ratio > 0.0);
    }

    #[test]
    fn test_analyze_replies_mixed() {
        let strategy = LegacyStrategy;
        let analysis = strategy.analyze_replies(&[
            "Great post!".to_string(),
            "check my bio".to_string(),
            "this is terrible".to_string(),
            "thanks for sharing".to_string(),
        ]);
        assert!(analysis.positive_ratio > 0.0);
        assert!(analysis.negative_ratio > 0.0);
        assert!(analysis.spam_ratio > 0.0);
    }

    // ========================================================================
    // decide_legacy Tests (integration of all signals)
    // ========================================================================

    #[test]
    fn test_decide_legacy_controversial_skips() {
        let strategy = LegacyStrategy;
        let ctx = default_tweet_context("This election is important");
        let decision = strategy.decide_legacy(&ctx);
        assert_eq!(decision.level, EngagementLevel::None);
        assert!(decision.reason.contains("controversial"));
    }

    #[test]
    fn test_decide_legacy_spam_skips() {
        let strategy = LegacyStrategy;
        let ctx = default_tweet_context("Follow for follow back");
        let decision = strategy.decide_legacy(&ctx);
        assert_eq!(decision.level, EngagementLevel::None);
        assert!(decision.reason.contains("spam"));
    }

    #[test]
    fn test_decide_legacy_medium_quality() {
        let strategy = LegacyStrategy;
        // Has question (+15) and positive words (+20) → score 35 → Medium
        let ctx = default_tweet_context(
            "Isn't this absolutely great and amazing post? I love the content here!",
        );
        let decision = strategy.decide_legacy(&ctx);
        assert_eq!(decision.level, EngagementLevel::Medium);
    }

    #[test]
    fn test_decide_legacy_high_quality() {
        let strategy = LegacyStrategy;
        // Has media link (+20), question (+15), positive words (+20), long content (+15), multiple sentences (+10)
        let ctx = default_tweet_context(
            "This is a great and amazing post. What do you think about pic.twitter.com/abc123? I love this content it's incredible and fantastic and wonderful and beautiful and everyone should read this.",
        );
        let decision = strategy.decide_legacy(&ctx);
        assert!(decision.score >= 60, "score was {}", decision.score);
        assert_eq!(decision.level, EngagementLevel::Full);
    }

    #[test]
    fn test_decide_legacy_minimal_quality() {
        let strategy = LegacyStrategy;
        // Has question (+15) → score 15 → Minimal
        let ctx = default_tweet_context("What do you think about this?");
        let decision = strategy.decide_legacy(&ctx);
        assert_eq!(decision.level, EngagementLevel::Minimal);
    }

    #[test]
    fn test_decide_legacy_low_quality() {
        let strategy = LegacyStrategy;
        let ctx = default_tweet_context("ok");
        let decision = strategy.decide_legacy(&ctx);
        assert_eq!(decision.level, EngagementLevel::None);
    }

    #[test]
    fn test_decide_legacy_with_reply_penalty() {
        let strategy = LegacyStrategy;
        let ctx = ctx_with_replies("Some content here", vec!["this is terrible", "awful post"]);
        let decision = strategy.decide_legacy(&ctx);
        // Negative replies should penalize the score
        assert_eq!(decision.level, EngagementLevel::None);
    }

    #[test]
    fn test_decide_legacy_name() {
        let strategy = LegacyStrategy;
        assert_eq!(strategy.name(), "legacy");
    }

    #[test]
    fn test_decide_legacy_strategy_type() {
        let strategy = LegacyStrategy;
        assert_eq!(strategy.strategy_type(), DecisionStrategy::Legacy);
    }
}
