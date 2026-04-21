//! Sentiment analysis utilities for tweet content.
//! Lightweight word-based sentiment scoring (no ML model required).

use crate::internal::text::truncate_chars;
use serde_json::Value;

/// Simple sentiment polarity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

/// Analyzes the sentiment of a tweet's text content using keyword matching.
/// This is a lightweight approach suitable for task routing decisions.
///
/// # Arguments
/// * `text` - The tweet text to analyze
///
/// # Returns
/// A `Sentiment` enum with the determined polarity
pub fn analyze_sentiment(text: &str) -> Sentiment {
    let lower = text.to_ascii_lowercase();

    // Count positive vs negative keywords
    let positive_count = POSITIVE_WORDS.iter().filter(|&w| lower.contains(w)).count();
    let negative_count = NEGATIVE_WORDS.iter().filter(|&w| lower.contains(w)).count();

    if positive_count > negative_count {
        Sentiment::Positive
    } else if negative_count > positive_count {
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
