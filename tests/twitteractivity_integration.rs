//! Integration tests for the twitteractivity task.
//! Tests public API surfaces: configuration, persona selection, sentiment
//! analysis, entry point selection, action chaining, and engagement limits
//! without requiring a live browser.

use rust_orchestrator::config::TwitterActivityConfig;
use rust_orchestrator::task::twitteractivity::TweetActionTracker;
use rust_orchestrator::utils::twitter::{
    twitteractivity_persona::select_persona_weights,
    twitteractivity_sentiment::{analyze_tweet_sentiment, sentiment_score, Sentiment},
};
use serde_json::json;
use std::time::Duration;

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

/// Tests that TweetActionTracker enforces minimum delay between actions on same tweet.
#[test]
fn twitteractivity_action_chaining_prevention_works() {
    let mut tracker = TweetActionTracker::new();
    let tweet_id = "test_tweet_123";

    // First action should be allowed
    assert!(
        tracker.can_perform_action(tweet_id, "like"),
        "first action on tweet should be allowed"
    );

    // Record the action
    tracker.record_action(tweet_id.to_string(), "like");

    // Immediate second action should be blocked due to cooldown
    assert!(
        !tracker.can_perform_action(tweet_id, "retweet"),
        "second action immediately after first should be blocked"
    );

    // Wait for cooldown to expire (MIN_ACTION_CHAIN_DELAY_MS = 3000ms)
    std::thread::sleep(Duration::from_millis(3100));

    // After cooldown, action should be allowed again
    assert!(
        tracker.can_perform_action(tweet_id, "retweet"),
        "action should be allowed after cooldown expires"
    );
}

/// Tests that TweetActionTracker allows actions on different tweets.
#[test]
fn twitteractivity_action_chaining_different_tweets_allowed() {
    let mut tracker = TweetActionTracker::new();
    let tweet_id_1 = "test_tweet_1";
    let tweet_id_2 = "test_tweet_2";

    // Record action on first tweet
    tracker.record_action(tweet_id_1.to_string(), "like");

    // Action on different tweet should be allowed immediately
    assert!(
        tracker.can_perform_action(tweet_id_2, "like"),
        "action on different tweet should be allowed immediately"
    );
}

/// Tests that TweetActionTracker allows same action type on same tweet after cooldown.
#[test]
fn twitteractivity_action_chaining_same_action_after_cooldown() {
    let mut tracker = TweetActionTracker::new();
    let tweet_id = "test_tweet_456";

    // Record first like action
    tracker.record_action(tweet_id.to_string(), "like");

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(3100));

    // Same action type should be allowed after cooldown
    assert!(
        tracker.can_perform_action(tweet_id, "like"),
        "same action type should be allowed after cooldown"
    );
}

/// Tests entry point selection returns valid URLs.
#[test]
fn twitteractivity_entry_point_selection_returns_valid_url() {
    use rust_orchestrator::task::twitteractivity::select_entry_point;

    // Test multiple selections to ensure all return valid URLs
    for _ in 0..10 {
        let entry_url = select_entry_point();
        assert!(
            entry_url.starts_with("https://"),
            "entry point URL should start with https://"
        );
        assert!(
            entry_url.contains("x.com") || entry_url.contains("twitter.com"),
            "entry point URL should be for x.com or twitter.com"
        );
    }
}

/// Tests that entry point selection includes home URL.
#[test]
fn twitteractivity_entry_point_selection_includes_home() {
    use rust_orchestrator::task::twitteractivity::select_entry_point;

    // Sample many times to ensure home URL is in the distribution
    let mut found_home = false;
    for _ in 0..100 {
        let entry_url = select_entry_point();
        if entry_url == "https://x.com/" || entry_url == "https://twitter.com/" {
            found_home = true;
            break;
        }
    }
    assert!(found_home, "home URL should be in entry point distribution");
}

/// Tests that engagement limits prevent actions when limits are reached.
#[test]
fn twitteractivity_engagement_limits_prevent_actions() {
    use rust_orchestrator::utils::twitter::twitteractivity_limits::{
        EngagementCounters, EngagementLimits,
    };

    let limits = EngagementLimits::default();
    let mut counters = EngagementCounters::new();

    // Initially, limits should allow actions
    assert!(
        limits.can_like(&counters),
        "should allow like when counter is zero"
    );
    assert!(
        limits.can_retweet(&counters),
        "should allow retweet when counter is zero"
    );

    // Increment like counter to max limit
    for _ in 0..limits.max_likes {
        counters.increment_like();
    }

    // After reaching limit, should not allow more likes
    assert!(
        !limits.can_like(&counters),
        "should not allow like when limit is reached"
    );

    // But other actions should still be allowed
    assert!(
        limits.can_retweet(&counters),
        "should allow retweet even when like limit is reached"
    );
}

/// Tests that engagement limits track total actions correctly.
#[test]
fn twitteractivity_engagement_limits_total_actions() {
    use rust_orchestrator::utils::twitter::twitteractivity_limits::{
        EngagementCounters, EngagementLimits,
    };

    let limits = EngagementLimits::default();
    let mut counters = EngagementCounters::new();

    // Perform various actions
    counters.increment_like();
    counters.increment_retweet();
    counters.increment_follow();
    counters.increment_reply();

    // Total should be sum of all individual counters
    assert_eq!(
        counters.total_actions(),
        counters.likes + counters.retweets + counters.follows + counters.replies,
        "total actions should equal sum of individual counters"
    );

    // Check against max total limit
    assert!(
        counters.total_actions() < limits.max_total_actions,
        "total actions should be under max limit"
    );
}

/// Tests that engagement limits remaining calculation is correct.
#[test]
fn twitteractivity_engagement_limits_remaining_calculation() {
    use rust_orchestrator::utils::twitter::twitteractivity_limits::{
        EngagementCounters, EngagementLimits,
    };

    let limits = EngagementLimits::default();
    let mut counters = EngagementCounters::new();

    // Increment some actions
    counters.increment_like();
    counters.increment_like();

    let remaining = limits.remaining(&counters);
    assert_eq!(
        remaining.get("likes"),
        Some(&(limits.max_likes - counters.likes)),
        "remaining likes should be max minus current"
    );
}
