//! Integration tests for the twitteractivity task.
//! Tests public API surfaces: configuration, persona selection, and sentiment
//! analysis without requiring a live browser.

use rust_orchestrator::config::TwitterActivityConfig;
use rust_orchestrator::utils::twitter::{
    twitteractivity_persona::select_persona_weights,
    twitteractivity_sentiment::{analyze_tweet_sentiment, sentiment_score, Sentiment},
};
use serde_json::json;

/// Ensures the twitteractivity module is linked and its entry point is accessible.
#[test]
fn twitteractivity_module_loads() {
    use rust_orchestrator::task::twitteractivity;
    let _ = &twitteractivity::run;
}

/// Validates that default Twitter Activity configuration has sensible values.
#[test]
fn twitteractivity_config_has_valid_defaults() {
    let ta = TwitterActivityConfig::default();

    assert!(
        ta.feed_scan_duration_ms >= 10_000,
        "scan duration must be >= 10s"
    );
    assert!(
        ta.feed_scan_duration_ms <= 1_800_000,
        "scan duration must be <= 30min"
    );
    assert!(ta.feed_scroll_count >= 1, "scroll count must be at least 1");
    assert!(
        ta.engagement_candidate_count >= 1,
        "candidate count must be at least 1"
    );
}

/// Checks that persona selection returns weights within allowed ranges.
#[test]
fn twitteractivity_persona_weights_in_range() {
    let weights = select_persona_weights(None);
    assert!((0.0..=1.0).contains(&weights.like_prob));
    assert!((0.0..=1.0).contains(&weights.retweet_prob));
    assert!((0.0..=1.0).contains(&weights.follow_prob));
    assert!((0.0..=1.0).contains(&weights.reply_prob));
    assert!((0.0..=1.0).contains(&weights.thread_dive_prob));
}

/// Confirms that sentiment classification returns expected categories for tweet objects.
#[test]
fn twitteractivity_sentiment_classification_works() {
    let positive_tweet = json!({ "text": "This is amazing! I love it!" });
    let negative_tweet = json!({ "text": "Terrible, worst, hate it." });
    let neutral_tweet = json!({ "text": "The meeting starts at 3pm." });

    assert!(matches!(
        analyze_tweet_sentiment(&positive_tweet),
        Sentiment::Positive
    ));
    assert!(matches!(
        analyze_tweet_sentiment(&negative_tweet),
        Sentiment::Negative
    ));
    assert!(matches!(
        analyze_tweet_sentiment(&neutral_tweet),
        Sentiment::Neutral
    ));
}

/// Verifies sentiment score ordering: Positive > Neutral > Negative.
#[test]
fn twitteractivity_sentiment_score_ordering() {
    let pos = sentiment_score(Sentiment::Positive);
    let neu = sentiment_score(Sentiment::Neutral);
    let neg = sentiment_score(Sentiment::Negative);

    assert!(pos > neu, "positive score should exceed neutral");
    assert!(neu > neg, "neutral score should exceed negative");
}
