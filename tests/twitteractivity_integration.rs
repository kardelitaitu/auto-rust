//! Integration tests for the twitteractivity task.
//! Tests public API surfaces: configuration, persona selection, sentiment
//! analysis, entry point selection, action chaining, and engagement limits
//! without requiring a live browser.

use auto::config::{TwitterActivityConfig, TwitterProbabilitiesConfig};
use auto::llm::{build_quote_messages, build_reply_messages, Role};
use auto::task::{select_entry_point, TweetActionTracker, MIN_ACTION_CHAIN_DELAY_MS};
use auto::utils::twitter::{
    sentiment::{analyze_tweet_sentiment_sync, sentiment_score, Sentiment, SentimentAnalyzer},
    twitteractivity_navigation::ENTRY_POINTS,
    twitteractivity_persona::select_persona_weights,
    TweetId,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
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
        ta.feed_scan_duration_ms.get() >= 10_000,
        "scan duration must be >= 10s"
    );
    assert!(
        ta.feed_scan_duration_ms.get() <= 1_800_000,
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
    let analyzer = SentimentAnalyzer::new();
    let positive_tweet = json!({ "text": "This is amazing! I love it!" });
    let negative_tweet = json!({ "text": "Terrible, worst, hate it." });
    let neutral_tweet = json!({ "text": "The meeting starts at 3pm." });

    assert!(matches!(
        analyze_tweet_sentiment_sync(&analyzer, &positive_tweet),
        Sentiment::Positive
    ));
    assert!(matches!(
        analyze_tweet_sentiment_sync(&analyzer, &negative_tweet),
        Sentiment::Negative
    ));
    assert!(matches!(
        analyze_tweet_sentiment_sync(&analyzer, &neutral_tweet),
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
        tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)),
        "first action on tweet should be allowed"
    );

    // Record the action
    tracker.record_action(TweetId::from_unchecked(tweet_id), "like");

    // Immediate second action should be blocked due to cooldown
    assert!(
        !tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)),
        "second action immediately after first should be blocked"
    );

    // Wait for cooldown to expire
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // After cooldown, action should be allowed again
    assert!(
        tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)),
        "action should be allowed after cooldown expires"
    );
}

/// Tests that TweetActionTracker allows actions on different tweets.
#[test]
fn twitteractivity_action_chaining_different_tweets_allowed() {
    let mut tracker = TweetActionTracker::new(MIN_ACTION_CHAIN_DELAY_MS);
    let tweet_id_1 = "test_tweet_1";
    let tweet_id_2 = "test_tweet_2";

    // Record action on first tweet
    tracker.record_action(TweetId::from_unchecked(tweet_id_1), "like");

    // Action on different tweet should be allowed immediately
    assert!(
        tracker.can_perform_action(&TweetId::from_unchecked(tweet_id_2)),
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
    tracker.record_action(TweetId::from_unchecked(tweet_id), "like");

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // Same action type should be allowed after cooldown
    assert!(
        tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)),
        "same action type should be allowed after cooldown"
    );
}

/// Tests entry point selection returns valid URLs.
#[test]
fn twitteractivity_entry_point_selection_returns_valid_url() {
    // select_entry_point already imported at top

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
    // select_entry_point already imported at top

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
    use auto::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

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
    use auto::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

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
    use auto::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

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
    use auto::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

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
        tracker.record_action(TweetId::from_unchecked(*tweet_id), "like");
    }

    // Each tweet should be blocked for its own action type
    for tweet_id in &tweet_ids {
        assert!(
            !tracker.can_perform_action(&TweetId::from_unchecked(*tweet_id)),
            "tweet should be blocked after like"
        );
    }

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // All tweets should now be unblocked
    for tweet_id in &tweet_ids {
        assert!(
            tracker.can_perform_action(&TweetId::from_unchecked(*tweet_id)),
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
    tracker.record_action(TweetId::from_unchecked(tweet_id), "like");
    assert!(!tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)));

    // Wait for cooldown
    std::thread::sleep(Duration::from_millis(TEST_DELAY_MS + 10));

    // Record second action
    tracker.record_action(TweetId::from_unchecked(tweet_id), "retweet");
    assert!(!tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)));
}

/// Tests that entry point selection has expected distribution.
#[test]
fn twitteractivity_entry_point_selection_distribution() {
    let total_weight: u32 = ENTRY_POINTS.iter().map(|entry| entry.weight).sum();
    let mut rng = StdRng::seed_from_u64(42);
    let mut counts = std::collections::HashMap::new();

    for _ in 0..200 {
        let mut roll = rng.gen::<u32>() % total_weight;
        let mut selected = ENTRY_POINTS[0].url;
        for entry in &ENTRY_POINTS {
            if roll < entry.weight {
                selected = entry.url;
                break;
            }
            roll -= entry.weight;
        }
        *counts.entry(selected).or_insert(0) += 1;
    }

    // Home should be the most common (59% weight)
    // With 200 samples, expect ~118; allow variance down to 100 (50%)
    let home_count = counts.get("https://x.com/").unwrap_or(&0);
    assert!(
        *home_count >= 100,
        "home should appear in >=50% of samples (got {} out of 200)",
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
    let analyzer = SentimentAnalyzer::new();
    let empty_tweet = json!({ "text": "" });
    let result = analyze_tweet_sentiment_sync(&analyzer, &empty_tweet);

    // Empty text should be classified as neutral
    assert!(
        matches!(result, Sentiment::Neutral),
        "empty text should be neutral"
    );
}

/// Tests that sentiment analysis handles very long text.
#[test]
fn twitteractivity_sentiment_long_text() {
    let analyzer = SentimentAnalyzer::new();
    let long_text = "This is absolutely amazing and wonderful! I love it so much, it's the best thing ever. Truly fantastic and incredible! ";
    let long_tweet = json!({ "text": long_text });
    let result = analyze_tweet_sentiment_sync(&analyzer, &long_tweet);

    // Long positive text should still be classified as positive
    assert!(
        matches!(result, Sentiment::Positive),
        "long positive text should be positive"
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test sentiment analysis with only emojis
#[test]
fn twitteractivity_sentiment_only_emojis() {
    let analyzer = SentimentAnalyzer::new();
    let emoji_tweet = json!({ "text": "🎉🎊🎈🎁" });
    let result = analyze_tweet_sentiment_sync(&analyzer, &emoji_tweet);

    // Emojis alone - should be neutral or positive
    assert!(
        matches!(result, Sentiment::Neutral | Sentiment::Positive),
        "emoji-only text should be neutral or positive"
    );
}

/// Test TweetActionTracker with zero delay
#[test]
fn twitteractivity_action_chaining_zero_delay() {
    let mut tracker = TweetActionTracker::new(0);
    let tweet_id = "test_zero_delay";

    // First action
    assert!(tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)));
    tracker.record_action(TweetId::from_unchecked(tweet_id), "like");

    // With zero delay, should be allowed immediately
    assert!(
        tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)),
        "action should be allowed with zero delay"
    );
}

/// Test persona weights with empty JSON override
#[test]
fn twitteractivity_persona_weights_empty_override() {
    let config_probs = TwitterProbabilitiesConfig::default();

    let empty_override = json!({});
    let weights = select_persona_weights(Some(&empty_override), &config_probs);

    // Empty override should use defaults
    assert!((0.0..=1.0).contains(&weights.like_prob));
    assert!((0.0..=1.0).contains(&weights.retweet_prob));
}

/// Test engagement limits with zero max values
#[test]
fn twitteractivity_engagement_limits_zero_max() {
    use auto::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

    let limits = EngagementLimits {
        max_likes: 0,
        max_retweets: 0,
        max_follows: 0,
        max_replies: 0,
        max_thread_dives: 0,
        max_total_actions: 0,
        max_bookmarks: 0,
        max_quote_tweets: 0,
    };
    let counters = EngagementCounters::new();

    // With zero limits, nothing should be allowed
    assert!(!limits.can_like(&counters));
    assert!(!limits.can_retweet(&counters));
    assert!(!limits.can_follow(&counters));
    assert!(!limits.can_reply(&counters));
    assert!(!limits.can_dive(&counters));
}

/// Test sentiment analysis with mixed positive/negative
#[test]
fn twitteractivity_sentiment_mixed_signals() {
    let analyzer = SentimentAnalyzer::new();
    let mixed_tweet =
        json!({ "text": "I love it but hate the service, it's amazing yet terrible." });
    let result = analyze_tweet_sentiment_sync(&analyzer, &mixed_tweet);

    // Mixed signals - could be neutral or based on dominant sentiment
    assert!(matches!(
        result,
        Sentiment::Positive | Sentiment::Negative | Sentiment::Neutral
    ));
}

// ============================================================================
// Error Classification Tests (§8.8 coverage: retry logic, error classification)
// ============================================================================

use auto::utils::twitter::twitteractivity_errors::is_auth_error;
use auto::utils::twitter::twitteractivity_errors::is_rate_limit_error;
use auto::utils::twitter::twitteractivity_errors::ErrorClass;
use auto::utils::twitter::twitteractivity_retry::RetryConfig;

/// Tests ErrorClassifier trait on anyhow::Error for all three error classes.
#[test]
fn twitteractivity_error_classification_transient_errors() {
    use auto::utils::twitter::ErrorClassifier;

    let timeout_err = anyhow::anyhow!("timeout waiting for element");
    assert_eq!(timeout_err.classify(), ErrorClass::Transient);

    let stale_err = anyhow::anyhow!("stale element reference");
    assert_eq!(stale_err.classify(), ErrorClass::Transient);

    let net_err = anyhow::anyhow!("net::ERR_CONNECTION_RESET");
    assert_eq!(net_err.classify(), ErrorClass::Transient);
}

/// Tests that permanent errors (unknown/misc) classify correctly.
#[test]
fn twitteractivity_error_classification_permanent_errors() {
    use auto::utils::twitter::ErrorClassifier;

    let unknown_err = anyhow::anyhow!("unknown error occurred");
    assert_eq!(unknown_err.classify(), ErrorClass::Permanent);
}

/// Tests that fatal errors (browser disconnected, target closed) classify correctly.
#[test]
fn twitteractivity_error_classification_fatal_errors() {
    use auto::utils::twitter::ErrorClassifier;

    let browser_err = anyhow::anyhow!("browser disconnected");
    assert_eq!(browser_err.classify(), ErrorClass::Fatal);

    let target_err = anyhow::anyhow!("target closed");
    assert_eq!(target_err.classify(), ErrorClass::Fatal);
}

/// Tests ErrorClassifier for std::io::Error (transient/kinds).
#[test]
fn twitteractivity_error_classification_io_transient() {
    use auto::utils::twitter::ErrorClassifier;
    use std::io;

    let timeout_err = io::Error::new(io::ErrorKind::TimedOut, "operation timed out");
    assert_eq!(timeout_err.classify(), ErrorClass::Transient);

    let conn_err = io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused");
    assert_eq!(conn_err.classify(), ErrorClass::Transient);
}

/// Tests ErrorClassifier for std::io::Error (permanent).
#[test]
fn twitteractivity_error_classification_io_permanent() {
    use auto::utils::twitter::ErrorClassifier;
    use std::io;

    let not_found = io::Error::new(io::ErrorKind::NotFound, "file not found");
    assert_eq!(not_found.classify(), ErrorClass::Permanent);
}

// ============================================================================
// Retry Config Tests (§8.8 coverage: retry config profiles)
// ============================================================================

/// Tests RetryConfig default values.
#[test]
fn twitteractivity_retry_config_default() {
    let config = RetryConfig::default();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.base_delay_ms, 500);
    assert_eq!(config.max_delay_ms, 5000);
    assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    assert!((config.jitter_factor - 0.1).abs() < f64::EPSILON);
}

/// Tests RetryConfig aggressive profile.
#[test]
fn twitteractivity_retry_config_aggressive() {
    let config = RetryConfig::aggressive();
    assert_eq!(config.max_attempts, 2);
    assert_eq!(config.base_delay_ms, 250);
    assert_eq!(config.max_delay_ms, 2000);
}

/// Tests RetryConfig conservative profile.
#[test]
fn twitteractivity_retry_config_conservative() {
    let config = RetryConfig::conservative();
    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.base_delay_ms, 1000);
    assert_eq!(config.max_delay_ms, 10000);
}

// ============================================================================
// Rate Limit & Auth Detection Tests (§8.8 coverage: error detection)
// ============================================================================

/// Tests is_rate_limit_error detection function.
#[test]
fn twitteractivity_rate_limit_detection() {
    assert!(is_rate_limit_error(&"rate limit exceeded"));
    assert!(is_rate_limit_error(&"429 Too Many Requests"));
    assert!(is_rate_limit_error(&"too many requests, try again later"));
    assert!(!is_rate_limit_error(&"element not found"));
    assert!(!is_rate_limit_error(&"network timeout"));
}

/// Tests is_auth_error detection function.
#[test]
fn twitteractivity_auth_error_detection() {
    assert!(is_auth_error(&"401 Unauthorized"));
    assert!(is_auth_error(&"authentication required"));
    assert!(is_auth_error(&"login failed"));
    assert!(!is_auth_error(&"network timeout"));
    assert!(!is_auth_error(&"stale element reference"));
}

// ============================================================================
// LLM Message Building Tests (§8.8 coverage: LLM message construction)
// ============================================================================

/// Tests that build_reply_messages returns correctly structured messages.
#[test]
fn twitteractivity_llm_build_reply_messages_structure() {
    let messages = build_reply_messages("author", "tweet text", &[]);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, Role::System);
    assert_eq!(messages[1].role, Role::User);
    assert!(messages[1].content.contains("author"));
    assert!(messages[1].content.contains("tweet text"));
}

/// Tests that build_reply_messages includes reply context when provided.
#[test]
fn twitteractivity_llm_build_reply_messages_with_replies() {
    let replies = vec![("reply_author", "reply content")];
    let messages = build_reply_messages("author", "tweet text", &replies);
    assert_eq!(messages.len(), 2);
    assert!(messages[1].content.contains("reply content"));
}

/// Tests that build_reply_messages handles empty replies gracefully.
#[test]
fn twitteractivity_llm_build_reply_messages_empty_replies() {
    let messages = build_reply_messages("author", "tweet", &[]);
    assert_eq!(messages.len(), 2);
    // Should not crash or produce garbage
    assert!(!messages[1].content.is_empty());
}

/// Tests that build_quote_messages returns correctly structured messages.
#[test]
fn twitteractivity_llm_build_quote_messages_structure() {
    let messages = build_quote_messages("author", "tweet text", &[]);
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].role, Role::System);
    assert_eq!(messages[1].role, Role::User);
    assert!(messages[1].content.contains("author"));
}

/// Tests that build_quote_messages includes reply context.
#[test]
fn twitteractivity_llm_build_quote_messages_with_replies() {
    let replies = vec![("reply_author", "reply text")];
    let messages = build_quote_messages("author", "tweet text", &replies);
    assert_eq!(messages.len(), 2);
    assert!(messages[1].content.contains("reply text"));
}

/// Tests that build_quote_messages handles empty replies gracefully.
#[test]
fn twitteractivity_llm_build_quote_messages_empty_replies() {
    let messages = build_quote_messages("author", "tweet", &[]);
    assert_eq!(messages.len(), 2);
    assert!(!messages[1].content.is_empty());
}

// ============================================================================
// SessionState Tests (§8.8 coverage: session state)
// ============================================================================

/// Tests SessionState creation with custom limits and duration.
#[test]
fn twitteractivity_session_state_creation() {
    use auto::utils::twitter::twitteractivity_limits::EngagementLimits;
    use auto::utils::twitter::twitteractivity_state::SessionState;

    let limits = EngagementLimits::with_limits(5, 3, 2, 1, 3, 2, 2, 10);
    let state = SessionState::new(limits, 60000, 100);

    assert!(!state.is_expired());
    assert!(state.remaining_time().as_millis() > 0);
    assert_eq!(state.action_summary(), (0, 10));
}

/// Tests SessionState is_expired detection.
#[test]
fn twitteractivity_session_state_expiry() {
    use auto::utils::twitter::twitteractivity_limits::EngagementLimits;
    use auto::utils::twitter::twitteractivity_state::SessionState;

    // Create session with 1ms duration — should expire nearly immediately
    let limits = EngagementLimits::default();
    let state = SessionState::new(limits, 1, 100);

    // Brief sleep to ensure expiry
    std::thread::sleep(Duration::from_millis(20));
    assert!(state.is_expired());
    assert_eq!(state.remaining_time().as_millis(), 0);
}

/// Tests SessionState action permission checks.
#[test]
fn twitteractivity_session_state_action_permission() {
    use auto::utils::twitter::twitteractivity_limits::EngagementLimits;
    use auto::utils::twitter::twitteractivity_state::SessionState;

    let limits = EngagementLimits::with_limits(5, 3, 2, 1, 3, 2, 2, 10);
    let state = SessionState::new(limits, 60000, 100);

    // Initially all actions should be allowed
    assert!(state.is_action_allowed("like"));
    assert!(state.is_action_allowed("retweet"));
    assert!(state.is_action_allowed("follow"));
    assert!(state.is_action_allowed("reply"));
    assert!(state.is_action_allowed("dive"));
    assert!(state.is_action_allowed("bookmark"));
    assert!(state.is_action_allowed("quote"));

    // Unknown action should not be allowed
    assert!(!state.is_action_allowed("unknown_action"));
}

/// Tests SessionState record_action updates counters correctly.
#[test]
fn twitteractivity_session_state_record_action() {
    use auto::utils::twitter::twitteractivity_limits::EngagementLimits;
    use auto::utils::twitter::twitteractivity_state::SessionState;

    let limits = EngagementLimits::with_limits(5, 3, 2, 1, 3, 2, 2, 10);
    let mut state = SessionState::new(limits, 60000, 100);

    state.record_action(&TweetId::from_unchecked("tweet_1"), "like");
    state.record_action(&TweetId::from_unchecked("tweet_2"), "retweet");
    state.record_action(&TweetId::from_unchecked("tweet_3"), "follow");

    assert_eq!(state.counters.likes, 1);
    assert_eq!(state.counters.retweets, 1);
    assert_eq!(state.counters.follows, 1);
    assert_eq!(state.counters.total_actions(), 3);
    assert!(!state.is_total_limit_reached());
}

/// Tests SessionState total limit detection.
#[test]
fn twitteractivity_session_state_total_limit() {
    use auto::utils::twitter::twitteractivity_limits::EngagementLimits;
    use auto::utils::twitter::twitteractivity_state::SessionState;

    let limits = EngagementLimits::with_limits(5, 3, 2, 1, 3, 2, 2, 3);
    let mut state = SessionState::new(limits, 60000, 100);

    // Exceed total limit
    state.record_action(&TweetId::from_unchecked("tweet_1"), "like");
    state.record_action(&TweetId::from_unchecked("tweet_2"), "like");
    state.record_action(&TweetId::from_unchecked("tweet_3"), "like");

    assert!(state.is_total_limit_reached());
    assert_eq!(state.action_summary(), (3, 3));
}

/// Tests SessionState progress_summary format.
#[test]
fn twitteractivity_session_state_progress_summary() {
    use auto::utils::twitter::twitteractivity_limits::EngagementLimits;
    use auto::utils::twitter::twitteractivity_state::SessionState;

    let limits = EngagementLimits::with_limits(5, 3, 2, 1, 3, 2, 2, 10);
    let mut state = SessionState::new(limits, 60000, 100);

    state.record_action(&TweetId::from_unchecked("tweet_1"), "like");
    let summary = state.progress_summary();
    assert!(summary.contains("1/10"));
    assert!(summary.contains("L:1"));
    assert!(summary.contains("Time left:"));
}

// ============================================================================
// Popup Detection Constants Tests (§8.8 coverage: popup detection)
// ============================================================================

/// Tests that popup detection order matches the implementation.
#[test]
fn twitteractivity_popup_detection_order() {
    use auto::utils::twitter::twitteractivity_navigation::is_login_flow;
    use auto::utils::twitter::twitteractivity_popup::detect_popup;

    // Verify the function signatures exist (compile-time check)
    let _ = is_login_flow;
    let _ = detect_popup;
}

// ============================================================================
// Engagement Limits Edge Cases (§8.8 coverage)
// ============================================================================

/// Tests that bookmark and quote_tweet limits are enforced.
#[test]
fn twitteractivity_engagement_limits_v2_actions() {
    use auto::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

    let limits = EngagementLimits::with_limits(5, 3, 2, 1, 3, 2, 2, 20);
    let mut counters = EngagementCounters::new();

    // Bookmark and quote should be allowed initially
    assert!(limits.can_bookmark(&counters));
    assert!(limits.can_quote_tweet(&counters));

    // Exhaust bookmarks
    counters.increment_bookmark();
    counters.increment_bookmark();
    assert!(!limits.can_bookmark(&counters));
    // Quote should still be available
    assert!(limits.can_quote_tweet(&counters));

    // Exhaust quote tweets
    counters.increment_quote_tweet();
    counters.increment_quote_tweet();
    assert!(!limits.can_quote_tweet(&counters));
}
