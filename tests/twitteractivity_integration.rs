//! Integration tests for the twitteractivity task.
//! Tests public API surfaces: configuration, persona selection, sentiment
//! analysis, entry point selection, action chaining, and engagement limits
//! without requiring a live browser.

use auto::config::{TwitterActivityConfig, TwitterProbabilitiesConfig};
use auto::task::{twitteractivity::{TweetActionTracker, MIN_ACTION_CHAIN_DELAY_MS, select_entry_point}};
use auto::utils::twitter::{
    twitteractivity_persona::select_persona_weights,
    twitteractivity_sentiment::{analyze_tweet_sentiment, sentiment_score, Sentiment},
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
};
use serde_json::json;
use std::time::Duration;

/// Ensures the twitteractivity module is linked and its entry point is accessible.
#[test]
fn twitteractivity_module_loads() {
    use auto::task::twitteractivity;
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
    let config_probs = TwitterProbabilitiesConfig::default();
    let weights = select_persona_weights(None, &config_probs);
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
    // Use smaller delay for tests to speed up execution
    const TEST_DELAY_MS: u64 = 100;
    let mut tracker = TweetActionTracker::new(TEST_DELAY_MS);
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

    // Wait for cooldown to expire
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // After cooldown, action should be allowed again
    assert!(
        tracker.can_perform_action(tweet_id, "retweet"),
        "action should be allowed after cooldown expires"
    );
}

/// Tests that TweetActionTracker allows actions on different tweets.
#[test]
fn twitteractivity_action_chaining_different_tweets_allowed() {
    let mut tracker = TweetActionTracker::new(
        MIN_ACTION_CHAIN_DELAY_MS,
    );
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
    // Use smaller delay for tests to speed up execution
    const TEST_DELAY_MS: u64 = 100;
    let mut tracker = TweetActionTracker::new(TEST_DELAY_MS);
    let tweet_id = "test_tweet_456";

    // Record first like action
    tracker.record_action(tweet_id.to_string(), "like");

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // Same action type should be allowed after cooldown
    assert!(
        tracker.can_perform_action(tweet_id, "like"),
        "same action type should be allowed after cooldown"
    );
}

/// Tests entry point selection returns valid URLs.
#[test]
fn twitteractivity_entry_point_selection_returns_valid_url() {
    use auto::task::twitteractivity::select_entry_point;

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
    use auto::task::twitteractivity::select_entry_point;

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
    use auto::utils::twitter::twitteractivity_limits::{
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
    use auto::utils::twitter::twitteractivity_limits::{
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
    use auto::utils::twitter::twitteractivity_limits::{
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

/// Tests that engagement limits work for all action types.
#[test]
fn twitteractivity_engagement_limits_all_action_types() {
    use auto::utils::twitter::twitteractivity_limits::{
        EngagementCounters, EngagementLimits,
    };

    let limits = EngagementLimits::default();
    let mut counters = EngagementCounters::new();

    // Test each action type limit
    assert!(limits.can_like(&counters), "should allow like initially");
    assert!(
        limits.can_retweet(&counters),
        "should allow retweet initially"
    );
    assert!(
        limits.can_follow(&counters),
        "should allow follow initially"
    );
    assert!(limits.can_reply(&counters), "should allow reply initially");
    assert!(limits.can_dive(&counters), "should allow dive initially");

    // Increment all counters to their limits
    for _ in 0..limits.max_likes {
        counters.increment_like();
    }
    for _ in 0..limits.max_retweets {
        counters.increment_retweet();
    }
    for _ in 0..limits.max_follows {
        counters.increment_follow();
    }
    for _ in 0..limits.max_replies {
        counters.increment_reply();
    }
    for _ in 0..limits.max_thread_dives {
        counters.increment_thread_dive();
    }

    // All should now be blocked
    assert!(
        !limits.can_like(&counters),
        "should not allow like when limit reached"
    );
    assert!(
        !limits.can_retweet(&counters),
        "should not allow retweet when limit reached"
    );
    assert!(
        !limits.can_follow(&counters),
        "should not allow follow when limit reached"
    );
    assert!(
        !limits.can_reply(&counters),
        "should not allow reply when limit reached"
    );
    assert!(
        !limits.can_dive(&counters),
        "should not allow dive when limit reached"
    );
}

/// Tests that persona weights can be overridden via payload.
#[test]
fn twitteractivity_persona_weights_override() {
    let config_probs = TwitterProbabilitiesConfig::default();

    // Default weights
    let default_weights = select_persona_weights(None, &config_probs);

    // Override weights
    let custom_weights = json!({
        "like_prob": 0.9,
        "retweet_prob": 0.1,
        "follow_prob": 0.05,
        "reply_prob": 0.02,
        "thread_dive_prob": 0.3
    });

    let override_weights = select_persona_weights(Some(&custom_weights), &config_probs);

    // Override should use custom values
    assert_eq!(
        override_weights.like_prob, 0.9,
        "like_prob should be overridden"
    );
    assert_eq!(
        override_weights.retweet_prob, 0.1,
        "retweet_prob should be overridden"
    );
    assert_eq!(
        override_weights.follow_prob, 0.05,
        "follow_prob should be overridden"
    );
    assert_eq!(
        override_weights.reply_prob, 0.02,
        "reply_prob should be overridden"
    );
    assert_eq!(
        override_weights.thread_dive_prob, 0.3,
        "thread_dive_prob should be overridden"
    );

    // Default should be different
    assert_ne!(
        default_weights.like_prob, 0.9,
        "default should differ from override"
    );
}

/// Tests that TweetActionTracker handles multiple tweets correctly.
#[test]
fn twitteractivity_action_chaining_multiple_tweets() {
    // Use smaller delay for tests to speed up execution
    const TEST_DELAY_MS: u64 = 100;
    let mut tracker = TweetActionTracker::new(TEST_DELAY_MS);
    let tweet_ids = vec!["tweet_1", "tweet_2", "tweet_3"];

    // Record actions on different tweets
    for tweet_id in &tweet_ids {
        tracker.record_action(tweet_id.to_string(), "like");
    }

    // Each tweet should be blocked for its own action type
    for tweet_id in &tweet_ids {
        assert!(
            !tracker.can_perform_action(tweet_id, "retweet"),
            "tweet should be blocked after like"
        );
    }

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // All tweets should now be unblocked
    for tweet_id in &tweet_ids {
        assert!(
            tracker.can_perform_action(tweet_id, "retweet"),
            "tweet should be unblocked after cooldown"
        );
    }
}

/// Tests that TweetActionTracker overwrites previous actions correctly.
#[test]
fn twitteractivity_action_chaining_overwrites_previous() {
    // Use smaller delay for tests to speed up execution
    const TEST_DELAY_MS: u64 = 100;
    let mut tracker = TweetActionTracker::new(TEST_DELAY_MS);
    let tweet_id = "test_tweet_overwrite";

    // Record first action
    tracker.record_action(tweet_id.to_string(), "like");
    assert!(!tracker.can_perform_action(tweet_id, "retweet"));

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // Record second action
    tracker.record_action(tweet_id.to_string(), "retweet");
    assert!(!tracker.can_perform_action(tweet_id, "follow"));
}

/// Tests that entry point selection has expected distribution.
#[test]
fn twitteractivity_entry_point_selection_distribution() {
    use auto::task::twitteractivity::select_entry_point;

    // Sample many times to check distribution (reduced from 1000 for speed)
    let mut counts = std::collections::HashMap::new();
    for _ in 0..100 {
        let entry_url = select_entry_point();
        *counts.entry(entry_url).or_insert(0) += 1;
    }

    // Home should be the most common (59% weight)
    let home_count = counts.get("https://x.com/").unwrap_or(&0);
    assert!(
        *home_count > 50,
        "home should appear in >50% of samples (got {})",
        home_count
    );

    // At least some other entry points should appear
    assert!(
        counts.len() > 1,
        "should have multiple different entry points"
    );
}

/// Tests that sentiment analysis handles empty text.
#[test]
fn twitteractivity_sentiment_empty_text() {
    let empty_tweet = json!({ "text": "" });
    let result = analyze_tweet_sentiment(&empty_tweet);

    // Empty text should be classified as neutral
    assert!(
        matches!(result, Sentiment::Neutral),
        "empty text should be neutral"
    );
}

/// Tests that sentiment analysis handles very long text.
#[test]
fn twitteractivity_sentiment_long_text() {
    let long_text = "This is absolutely amazing and wonderful! I love it so much, it's the best thing ever. Truly fantastic and incredible! ";
    let long_tweet = json!({ "text": long_text });
    let result = analyze_tweet_sentiment(&long_tweet);

    // Long positive text should still be classified as positive
    assert!(
        matches!(result, Sentiment::Positive),
        "long positive text should be positive"
    );
}
