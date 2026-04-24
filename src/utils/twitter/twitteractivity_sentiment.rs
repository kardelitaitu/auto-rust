//! Sentiment analysis utilities for tweet content.
//! Enhanced with contextual analysis (negation, sarcasm, intensifiers), emoji sentiment,
//! and domain-specific keyword detection (Tech, Crypto, Gaming, Sports, Entertainment).
//!
//! ## Features
//! - Keyword-based sentiment with contextual modifiers
//! - Negation detection ("not good" → negative)
//! - Sarcasm markers ("oh great" → negative)
//! - Intensifier handling ("very bad" → stronger negative)
//! - Comprehensive emoji sentiment (300+ emojis)
//! - Domain-specific keywords (Tech, Crypto, Gaming, Sports, Entertainment)

use crate::internal::text::truncate_chars;
use crate::utils::twitter::{
    twitteractivity_sentiment_context, twitteractivity_sentiment_domains,
    twitteractivity_sentiment_emoji,
};
use serde_json::Value;
use tracing::instrument;

/// Simple sentiment polarity with enhanced scoring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

/// Analyzes the sentiment of a tweet's text content using enhanced keyword matching
/// with contextual analysis (negation, intensifiers, sarcasm), emoji sentiment, and
/// domain-specific keywords.
///
/// # Arguments
/// * `text` - The tweet text to analyze
///
/// # Returns
/// A `Sentiment` enum with the determined polarity
#[instrument]
pub fn analyze_sentiment(text: &str) -> Sentiment {
    let lower = text.to_ascii_lowercase();

    // Check for sarcasm first (overrides other signals)
    if twitteractivity_sentiment_context::has_sarcasm_markers(&lower) {
        return Sentiment::Negative;
    }

    // Get emoji sentiment contribution
    let emoji_score = twitteractivity_sentiment_emoji::analyze_emoji_sentiment(text);

    // Detect domain and get domain-specific contribution
    let domain = twitteractivity_sentiment_domains::detect_domain(text);
    let domain_score = twitteractivity_sentiment_domains::analyze_domain_sentiment(text, domain);

    // Initialize score with emoji and domain contributions
    let mut score: f32 = emoji_score + (domain_score / 10.0); // Normalize domain contribution

    // Count positive vs negative with context
    for &word in POSITIVE_WORDS {
        if lower.contains(word) {
            // Calculate contextual score (handles negation and intensifiers)
            let contextual_score =
                twitteractivity_sentiment_context::calculate_contextual_score(&lower, 1.0, word);
            score += contextual_score;
        }
    }

    for &word in NEGATIVE_WORDS {
        if lower.contains(word) {
            // Calculate contextual score (handles negation and intensifiers)
            let contextual_score =
                twitteractivity_sentiment_context::calculate_contextual_score(&lower, -1.0, word);
            score += contextual_score;
        }
    }

    // Apply contextual modifiers (sarcasm, excessive punctuation)
    let context_modifier = twitteractivity_sentiment_context::analyze_contextual_modifiers(&lower);
    score += context_modifier;

    // Classify based on score with hysteresis to avoid borderline flips
    if score > 1.0 {
        Sentiment::Positive
    } else if score < -1.0 {
        Sentiment::Negative
    } else {
        Sentiment::Neutral
    }
}

/// Weighted sentiment score: positive = +1, neutral = 0, negative = -1.
/// Can be aggregated across multiple tweets.
pub fn sentiment_score(sentiment: Sentiment) -> i32 {
    match sentiment {
        Sentiment::Positive => 1,
        Sentiment::Neutral => 0,
        Sentiment::Negative => -1,
    }
}

/// Extracts text content from a tweet object (from `selector_all_tweets()` result) and analyzes it.
/// The tweet object should contain a `text` field or nested text content.
pub fn analyze_tweet_sentiment(tweet_obj: &Value) -> Sentiment {
    let text = extract_tweet_text(tweet_obj);
    analyze_sentiment(&text)
}

/// Extracts raw text from a tweet JSON object.
/// Handles multiple possible text field locations.
fn extract_tweet_text(tweet_obj: &Value) -> String {
    if let Some(text) = tweet_obj.get("text").and_then(|v: &Value| v.as_str()) {
        return text.to_string();
    }
    if let Some(full_text) = tweet_obj.get("full_text").and_then(|v: &Value| v.as_str()) {
        return full_text.to_string();
    }
    if let Some(obj) = tweet_obj.as_object() {
        // Check nested legacy structure sometimes returned by Twitter
        if let Some(retweeted) = obj.get("retweeted_status") {
            return extract_tweet_text(retweeted);
        }
    }
    // Fallback: stringify the object and take first 280 chars
    truncate_chars(&tweet_obj.to_string(), 280)
}

/// Collects sentiment statistics for a batch of tweets.
#[derive(Debug, Clone, Default)]
pub struct SentimentStats {
    pub positive: u32,
    pub neutral: u32,
    pub negative: u32,
}

impl SentimentStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, sentiment: Sentiment) {
        match sentiment {
            Sentiment::Positive => self.positive += 1,
            Sentiment::Neutral => self.neutral += 1,
            Sentiment::Negative => self.negative += 1,
        }
    }

    pub fn dominant(&self) -> Sentiment {
        if self.positive >= self.neutral && self.positive >= self.negative {
            Sentiment::Positive
        } else if self.negative >= self.neutral && self.negative >= self.positive {
            Sentiment::Negative
        } else {
            Sentiment::Neutral
        }
    }

    pub fn total(&self) -> u32 {
        self.positive + self.neutral + self.negative
    }
}

/// Returns a composite sentiment score for the feed window.
/// Range: -1.0 to +1.0, normalized by count.
pub fn feed_sentiment_score(stats: &SentimentStats) -> f64 {
    let total = stats.total() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let pos = stats.positive as f64 / total;
    let neg = stats.negative as f64 / total;
    pos - neg
}

// ============================================================================
// Keyword Lists
// ============================================================================

const POSITIVE_WORDS: &[&str] = &[
    "good",
    "great",
    "awesome",
    "amazing",
    "excellent",
    "love",
    "like",
    "nice",
    "wonderful",
    "fantastic",
    "best",
    "happy",
    "glad",
    "joy",
    "cool",
    "brilliant",
    "thank",
    "thanks",
    "appreciate",
    "beautiful",
    "perfect",
    "ideal",
    "superb",
    "outstanding",
    "impressive",
    "enjoy",
    "fun",
    "yes",
    "win",
    "won",
    "celebrate",
    "congrats",
    "congratulations",
    "well done",
    "welldone",
    "spot on",
    "correct",
    "right",
    "smart",
    "wise",
    "kind",
    "friendly",
    "helpful",
    "support",
    "bless",
    "awesome",
    "marvelous",
    "pleasure",
    "delighted",
    "thrilled",
    "excited",
    "yay",
    "😊",
    "❤️",
    "🔥",
    "💯",
    "👏",
];

const NEGATIVE_WORDS: &[&str] = &[
    "bad",
    "terrible",
    "awful",
    "worst",
    "hate",
    "dislike",
    "horrible",
    "disgusting",
    "poor",
    "sad",
    "angry",
    "mad",
    "upset",
    "annoyed",
    "disappointed",
    "fail",
    "failed",
    "failure",
    "wrong",
    "error",
    "mistake",
    "bug",
    "broken",
    "useless",
    "waste",
    "sucks",
    "sucked",
    "suck",
    "hell",
    "shit",
    "damn",
    "fuck",
    "fucking",
    "idiot",
    "stupid",
    "dumb",
    "ridiculous",
    "absurd",
    "fake",
    "scam",
    "liar",
    "lies",
    "lying",
    "toxic",
    "abuse",
    "abusive",
    "toxic",
    " harassment",
    "harassing",
    "block",
    "report",
    "spam",
    "spammer",
    "clown",
    "joke",
    "pathetic",
    "disaster",
    "mess",
    "nightmare",
    "regret",
    "depressing",
    "depressed",
    "anxious",
    "anxiety",
    "cry",
    "crying",
    "😢",
    "😡",
    "💩",
];

/// Detects controversial content (mixed or strongly polarized sentiment).
/// Returns true if the text has both positive and negative indicators.
pub fn is_controversial(text: &str, stats: Option<&SentimentStats>) -> bool {
    let lower = text.to_ascii_lowercase();
    let has_positive = POSITIVE_WORDS.iter().any(|&w| lower.contains(w));
    let has_negative = NEGATIVE_WORDS.iter().any(|&w| lower.contains(w));

    // If both are present, likely controversial or sarcastic
    if has_positive && has_negative {
        return true;
    }

    // Or if stats provided, mixed sentiment across tweets
    if let Some(stats) = stats {
        if stats.positive > 0 && stats.negative > 0 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentiment_enum_variants() {
        let pos = Sentiment::Positive;
        let neu = Sentiment::Neutral;
        let neg = Sentiment::Negative;
        assert_eq!(pos, Sentiment::Positive);
        assert_eq!(neu, Sentiment::Neutral);
        assert_eq!(neg, Sentiment::Negative);
    }

    #[test]
    fn test_sentiment_score_positive() {
        assert_eq!(sentiment_score(Sentiment::Positive), 1);
    }

    #[test]
    fn test_sentiment_score_neutral() {
        assert_eq!(sentiment_score(Sentiment::Neutral), 0);
    }

    #[test]
    fn test_sentiment_score_negative() {
        assert_eq!(sentiment_score(Sentiment::Negative), -1);
    }

    #[test]
    fn test_sentiment_stats_default() {
        let stats = SentimentStats::default();
        assert_eq!(stats.positive, 0);
        assert_eq!(stats.neutral, 0);
        assert_eq!(stats.negative, 0);
    }

    #[test]
    fn test_sentiment_stats_new() {
        let stats = SentimentStats::new();
        assert_eq!(stats.positive, 0);
        assert_eq!(stats.neutral, 0);
        assert_eq!(stats.negative, 0);
    }

    #[test]
    fn test_sentiment_stats_add_positive() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        assert_eq!(stats.positive, 1);
        assert_eq!(stats.neutral, 0);
        assert_eq!(stats.negative, 0);
    }

    #[test]
    fn test_sentiment_stats_add_neutral() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Neutral);
        assert_eq!(stats.positive, 0);
        assert_eq!(stats.neutral, 1);
        assert_eq!(stats.negative, 0);
    }

    #[test]
    fn test_sentiment_stats_add_negative() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Negative);
        assert_eq!(stats.positive, 0);
        assert_eq!(stats.neutral, 0);
        assert_eq!(stats.negative, 1);
    }

    #[test]
    fn test_sentiment_stats_dominant_positive() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Negative);
        assert_eq!(stats.dominant(), Sentiment::Positive);
    }

    #[test]
    fn test_sentiment_stats_dominant_negative() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Negative);
        stats.add(Sentiment::Negative);
        stats.add(Sentiment::Positive);
        assert_eq!(stats.dominant(), Sentiment::Negative);
    }

    #[test]
    fn test_sentiment_stats_dominant_neutral() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Neutral);
        stats.add(Sentiment::Neutral);
        assert_eq!(stats.dominant(), Sentiment::Neutral);
    }

    #[test]
    fn test_sentiment_stats_total() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Neutral);
        stats.add(Sentiment::Negative);
        assert_eq!(stats.total(), 3);
    }

    #[test]
    fn test_feed_sentiment_score_empty() {
        let stats = SentimentStats::new();
        assert_eq!(feed_sentiment_score(&stats), 0.0);
    }

    #[test]
    fn test_feed_sentiment_score_all_positive() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        assert_eq!(feed_sentiment_score(&stats), 1.0);
    }

    #[test]
    fn test_feed_sentiment_score_all_negative() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Negative);
        stats.add(Sentiment::Negative);
        assert_eq!(feed_sentiment_score(&stats), -1.0);
    }

    #[test]
    fn test_feed_sentiment_score_mixed() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Negative);
        assert_eq!(feed_sentiment_score(&stats), 0.0);
    }

    #[test]
    fn test_positive_words_not_empty() {
        assert!(!POSITIVE_WORDS.is_empty());
    }

    #[test]
    fn test_negative_words_not_empty() {
        assert!(!NEGATIVE_WORDS.is_empty());
    }

    #[test]
    fn test_positive_words_contains_common() {
        assert!(POSITIVE_WORDS.contains(&"good"));
        assert!(POSITIVE_WORDS.contains(&"great"));
        assert!(POSITIVE_WORDS.contains(&"love"));
    }

    #[test]
    fn test_negative_words_contains_common() {
        assert!(NEGATIVE_WORDS.contains(&"bad"));
        assert!(NEGATIVE_WORDS.contains(&"hate"));
        assert!(NEGATIVE_WORDS.contains(&"terrible"));
    }

    #[test]
    fn test_is_controversial_mixed_words() {
        let text = "This is good but also bad";
        assert!(is_controversial(text, None));
    }

    #[test]
    fn test_is_controversial_only_positive() {
        let text = "This is great and wonderful";
        assert!(!is_controversial(text, None));
    }

    #[test]
    fn test_is_controversial_only_negative() {
        let text = "This is terrible and awful";
        assert!(!is_controversial(text, None));
    }

    #[test]
    fn test_is_controversial_with_stats_mixed() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Negative);
        assert!(is_controversial("", Some(&stats)));
    }

    #[test]
    fn test_is_controversial_with_stats_only_positive() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        assert!(!is_controversial("", Some(&stats)));
    }
}
