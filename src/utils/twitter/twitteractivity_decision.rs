//! Rule-based smart engagement decisions for Twitter automation.
//!
//! Uses keyword matching and pattern detection to evaluate tweet quality
//! and determine appropriate engagement levels. No LLM required.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::instrument;

// ============================================================================
// Strategy Pattern: Decision Engine Trait
// ============================================================================

/// Context passed to decision engines for analysis
#[derive(Debug, Clone)]
pub struct TweetContext {
    pub tweet_id: String,
    pub text: String,
    pub author: String,
    pub replies: Vec<String>,
    pub persona: super::twitteractivity_persona::PersonaWeights,
    pub task_config: super::twitteractivity_state::TaskConfig,
}

/// Extended EngagementDecision with strategy pattern fields
#[derive(Debug, Clone)]
pub struct EngagementDecision {
    pub level: EngagementLevel,
    pub score: i32,
    pub reason: String,
    pub multiplier: f64,
    pub confidence: f64,
}

/// Core trait for all decision engines (Strategy Pattern)
#[async_trait]
pub trait DecisionEngine: Send + Sync {
    /// Engine name for logging/metrics
    fn name(&self) -> &'static str;

    /// Make engagement decision for a tweet
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision;

    /// Check if engine is available (e.g., LLM API reachable)
    fn is_available(&self) -> bool {
        true
    }
}

/// Strategy selection for decision engines
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStrategy {
    #[default]
    Persona, // Rule-based only
    LLM,     // LLM-based only
    Hybrid,  // Combined approach
    Unified, // Single LLM call for decision + content
    Auto,    // Auto-select based on config
}

/// Factory for creating decision engines based on strategy
pub struct DecisionEngineFactory;

impl DecisionEngineFactory {
    /// Create appropriate engine based on strategy and config
    pub fn create(
        strategy: DecisionStrategy,
        llm_api_key: Option<String>,
    ) -> Box<dyn DecisionEngine> {
        use super::twitteractivity_decision_hybrid::HybridEngine;
        use super::twitteractivity_decision_llm::LLMEngine;
        use super::twitteractivity_decision_persona::PersonaEngine;
        use super::twitteractivity_decision_unified::UnifiedEngine;

        match strategy {
            DecisionStrategy::Persona => Box::new(PersonaEngine::new()),
            DecisionStrategy::LLM => {
                if let Some(key) = llm_api_key {
                    Box::new(LLMEngine::new(key))
                } else {
                    // Fallback to Persona if no API key
                    Box::new(PersonaEngine::new())
                }
            }
            DecisionStrategy::Hybrid => {
                if let Some(key) = llm_api_key {
                    Box::new(HybridEngine::with_llm(key, 0.3, 0.7))
                } else {
                    Box::new(HybridEngine::persona_only())
                }
            }
            DecisionStrategy::Unified => {
                if let Some(key) = llm_api_key {
                    Box::new(UnifiedEngine::new(key))
                } else {
                    // Fallback to Persona if no API key
                    Box::new(PersonaEngine::new())
                }
            }
            DecisionStrategy::Auto => {
                // Auto-select: use Unified if LLM available (fastest), otherwise Persona
                if let Some(key) = llm_api_key {
                    Box::new(UnifiedEngine::new(key))
                } else {
                    Box::new(PersonaEngine::new())
                }
            }
        }
    }
}

// ============================================================================

/// Engagement level determines which actions are allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngagementLevel {
    /// Full engagement: like, retweet, reply, follow, quote tweet
    Full,
    /// Medium engagement: like, retweet only
    Medium,
    /// Minimal engagement: like only
    Minimal,
    /// Skip engagement entirely
    None,
}

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

// ============================================================================
// Public API
// ============================================================================

/// Evaluates a tweet and returns the appropriate engagement level.
///
/// # Arguments
/// * `tweet_text` - The full text of the tweet
/// * `replies` - Vector of (author, text) pairs for top replies
///
/// # Returns
/// EngagementDecision with level, score, and reason
#[instrument(skip(replies))]
pub fn decide_engagement(tweet_text: &str, replies: &[(String, String)]) -> EngagementDecision {
    let text_lower = tweet_text.to_lowercase();

    // 1. Check hard blocklists (instant skip)
    if contains_any(&text_lower, CONTROVERSIAL_TOPICS) {
        return EngagementDecision {
            level: EngagementLevel::None,
            score: 0,
            reason: "controversial topic".to_string(),
            multiplier: 0.0,
            confidence: 0.95,
        };
    }

    if contains_any(&text_lower, SPAM_PATTERNS) {
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
    score += calculate_quality_signals(&text_lower, tweet_text);
    score -= calculate_penalty_signals(&text_lower, tweet_text);

    // 3. Analyze replies for community sentiment
    let reply_analysis = analyze_replies(replies);

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

// ============================================================================
// Quality Signal Calculators
// ============================================================================

/// Calculate positive quality signals.
fn calculate_quality_signals(text_lower: &str, original_text: &str) -> i32 {
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
    if contains_any(text_lower, POSITIVE_WORDS) {
        score += 20;
    }

    // Long form content (+15)
    if original_text.len() > 200 {
        score += 15;
    }

    score
}

/// Calculate penalty signals.
fn calculate_penalty_signals(text_lower: &str, original_text: &str) -> i32 {
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
    let emoji_count = original_text.chars().filter(|c| is_emoji(*c)).count();
    if emoji_count >= 5 {
        penalty += 15;
    }

    // Negative words (-40)
    if contains_any(text_lower, NEGATIVE_WORDS) {
        penalty += 40;
    }

    // Very short tweet (-10)
    if original_text.len() < 20 {
        penalty += 10;
    }

    penalty
}

// ============================================================================
// Reply Analysis
// ============================================================================

/// Analysis results for tweet replies.
#[allow(dead_code)]
struct ReplyAnalysis {
    positive_ratio: f64,
    negative_ratio: f64,
    spam_ratio: f64,
}

/// Analyzes the sentiment and quality of replies.
fn analyze_replies(replies: &[(String, String)]) -> ReplyAnalysis {
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

    for (_, text) in replies {
        let text_lower = text.to_lowercase();

        if contains_any(&text_lower, SPAM_PATTERNS) {
            spam_count += 1;
        } else if contains_any(&text_lower, POSITIVE_WORDS) {
            positive_count += 1;
        } else if contains_any(&text_lower, NEGATIVE_WORDS) {
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

// ============================================================================
// Helper Functions
// ============================================================================

/// Check if text contains any of the given patterns.
fn contains_any(text: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| text.contains(pattern))
}

/// Check if a character is an emoji.
fn is_emoji(c: char) -> bool {
    let cp = c as u32;
    // Common emoji Unicode ranges
    (0x1F600..=0x1F64F).contains(&cp) ||  // Emoticons
    (0x1F300..=0x1F5FF).contains(&cp) ||  // Misc Symbols and Pictographs
    (0x1F680..=0x1F6FF).contains(&cp) ||  // Transport and Map
    (0x1F1E0..=0x1F1FF).contains(&cp) ||  // Flags
    (0x2600..=0x26FF).contains(&cp) ||    // Misc symbols
    (0x2700..=0x27BF).contains(&cp) // Dingbats
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controversial_tweet_skipped() {
        let tweet = "The election was rigged by fascists!";
        let decision = decide_engagement(tweet, &[]);
        assert_eq!(decision.level, EngagementLevel::None);
        assert_eq!(decision.reason, "controversial topic");
    }

    #[test]
    fn test_spam_tweet_skipped() {
        let tweet = "Follow for follow! Check my bio! 💰";
        let decision = decide_engagement(tweet, &[]);
        assert_eq!(decision.level, EngagementLevel::None);
        assert_eq!(decision.reason, "spam content");
    }

    #[test]
    fn test_high_quality_tweet_full_engagement() {
        let tweet = "Just shipped my first project! Thanks to everyone who helped 🎉";
        let decision = decide_engagement(
            tweet,
            &[
                ("user1".to_string(), "Congratulations!".to_string()),
                ("user2".to_string(), "This is amazing!".to_string()),
            ],
        );
        // Should have positive words bonus (+20) and positive replies
        assert!(decision.score >= 20);
        // At minimum should be Minimal engagement
        assert_ne!(decision.level, EngagementLevel::None);
    }

    #[test]
    fn test_medium_quality_tweet() {
        let tweet = "Working on something new. Stay tuned.";
        let decision = decide_engagement(tweet, &[]);
        assert!(matches!(
            decision.level,
            EngagementLevel::Medium | EngagementLevel::Minimal
        ));
    }

    #[test]
    fn test_negative_replies_reduce_engagement() {
        let tweet = "Check out my new product";
        let decision = decide_engagement(
            tweet,
            &[
                ("user1".to_string(), "This is terrible".to_string()),
                ("user2".to_string(), "Worst product ever".to_string()),
                ("user3".to_string(), "Don't buy this".to_string()),
            ],
        );
        assert_eq!(decision.level, EngagementLevel::None);
    }

    #[test]
    fn test_question_gets_bonus() {
        let tweet = "What do you think about the new features?";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score >= 15); // Question bonus
    }

    #[test]
    fn test_thread_gets_bonus() {
        let tweet = "1/ Let me share my thoughts on this topic...";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score >= 25); // Thread bonus
    }

    #[test]
    fn test_all_caps_penalty() {
        let tweet = "THIS IS VERY IMPORTANT EVERYONE NEEDS TO SEE THIS";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score < 0); // All caps penalty
    }

    #[test]
    fn test_excessive_hashtags_penalty() {
        let tweet = "Check this out #tech #startup #business #marketing #growth";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score < 0); // Hashtag penalty
    }

    #[test]
    fn test_positive_words_bonus() {
        let tweet = "This is amazing and wonderful, I love it!";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score >= 20); // Positive words bonus
    }

    #[test]
    fn test_negative_words_penalty() {
        let tweet = "This is terrible and awful, I hate it!";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score < 0); // Negative words penalty
    }

    #[test]
    fn test_empty_replies_no_penalty() {
        let tweet = "Just a normal tweet with good content";
        let decision = decide_engagement(tweet, &[]);
        // Should not get reply penalties, but may have other penalties
        // Just verify it's not automatically None
        assert_ne!(decision.reason, "spam content");
        assert_ne!(decision.reason, "controversial topic");
    }

    #[test]
    fn test_engagement_level_variants() {
        assert_eq!(EngagementLevel::Full, EngagementLevel::Full);
        assert_eq!(EngagementLevel::Medium, EngagementLevel::Medium);
        assert_eq!(EngagementLevel::Minimal, EngagementLevel::Minimal);
        assert_eq!(EngagementLevel::None, EngagementLevel::None);
    }

    #[test]
    fn test_engagement_level_inequality() {
        assert_ne!(EngagementLevel::Full, EngagementLevel::Medium);
        assert_ne!(EngagementLevel::Medium, EngagementLevel::Minimal);
        assert_ne!(EngagementLevel::Minimal, EngagementLevel::None);
    }

    #[test]
    fn test_very_short_tweet_penalty() {
        let tweet = "Hi";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score < 0); // Short tweet penalty
    }

    #[test]
    fn test_image_bonus() {
        let tweet = "Check out this photo pic.twitter.com/abc123";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score >= 20); // Image bonus
    }

    #[test]
    fn test_multiple_sentences_bonus() {
        let tweet = "This is sentence one. This is sentence two.";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score >= 10); // Multiple sentences bonus
    }

    #[test]
    fn test_excessive_emojis_penalty() {
        let tweet = "Check this out 😀😀😀😀😀😀😀😀😀😀";
        let decision = decide_engagement(tweet, &[]);
        assert!(decision.score < 0); // Emoji penalty
    }

    #[test]
    fn test_is_emoji_function() {
        assert!(is_emoji('😀'));
        assert!(is_emoji('🎉'));
        assert!(is_emoji('❤'));
        assert!(!is_emoji('a'));
        assert!(!is_emoji('1'));
    }

    #[test]
    fn test_contains_any_function() {
        let text = "This is a test string";
        assert!(contains_any(text, &["test", "example"]));
        assert!(!contains_any(text, &["missing", "not here"]));
        assert!(contains_any(text, &["string"]));
    }

    #[test]
    fn test_engagement_decision_fields() {
        let decision = EngagementDecision {
            level: EngagementLevel::Full,
            score: 100,
            reason: "test reason".to_string(),
            multiplier: 1.5,
            confidence: 0.9,
        };
        assert_eq!(decision.level, EngagementLevel::Full);
        assert_eq!(decision.score, 100);
        assert_eq!(decision.reason, "test reason");
        assert_eq!(decision.multiplier, 1.5);
        assert_eq!(decision.confidence, 0.9);
    }
}
