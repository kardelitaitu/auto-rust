//! Twitter activity TDD helpers.
//!
//! This module provides reusable helpers for writing TDD-style tests
//! for the twitteractivity module. It follows the Red-Green-Refactor
//! pattern and provides:
//!
//! - Tweet builders for creating test tweets
//! - Config factories for test configurations
//! - Session state builders
//! - Assertion helpers
//! - TDD lifecycle markers
//!
//! # TDD Cycle
//!
//! 1. **Red**: Write a test that fails (use `#[tdd_red]` or prefix tests with `tdd_red_`)
//! 2. **Green**: Write minimal code to make the test pass
//! 3. **Refactor**: Clean up the code while keeping tests green
//!
//! # Usage
//!
//! In unit tests (inside `src/`):
//! ```rust,ignore
//! use crate::tests::twitter_helpers::*;
//! ```
//!
//! In integration tests (inside `tests/`):
//! ```rust,ignore
//! use auto::tests::twitter_helpers::*;
//! // or via the common module:
//! use crate::common::twitter_helpers::*;
//! ```

use serde_json::{json, Value};

use crate::config::TwitterActivityConfig;
use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};
use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
use crate::utils::twitter::twitteractivity_state::{SessionState, TaskConfig, TweetActionTracker};
use crate::utils::twitter::twitteractivity_types::TweetId;

// ============================================================================
// TDD Lifecycle Markers
// ============================================================================

/// Marker constant for Red (failing) tests.
/// Used to label tests that demonstrate desired behavior before it's implemented.
pub const TDD_RED: &str = "RED: Test demonstrates desired behavior (expected to fail)";

/// Marker constant for Green (passing) tests.
/// Used to label tests that validate implemented behavior.
pub const TDD_GREEN: &str = "GREEN: Test validates working behavior";

/// Marker constant for Refactor (cleanup) tests.
/// Used to label tests that verify behavior after refactoring.
pub const TDD_REFACTOR: &str = "REFACTOR: Test validates behavior after cleanup";

/// Marker for edge case tests.
pub const TDD_EDGE: &str = "EDGE: Test validates edge case behavior";

/// Marker for regression tests.
pub const TDD_REGRESSION: &str = "REGRESSION: Test prevents regression of fixed bug";

// ============================================================================
// Tweet Builders
// ============================================================================

/// Build a standard tweet with the given text.
///
/// # Example
/// ```rust,ignore
/// let tweet = build_tweet("Hello world", "user123");
/// assert_eq!(tweet["author"], "user123");
/// ```
#[must_use]
pub fn build_tweet(text: &str, author: &str) -> Value {
    json!({
        "tweet_id": format!("tweet_{}", fast_random_id()),
        "author": author,
        "text": text,
        "display_name": author,
        "author_handle": format!("@{}", author),
        "timestamp": "2026-05-21T12:00:00Z",
        "url": format!("https://x.com/{}/status/{}", author, fast_random_id()),
        "metrics": {
            "likes": 0,
            "retweets": 0,
            "replies": 0,
            "views": 100
        },
        "is_verified": false,
        "has_thread": false
    })
}

/// Build a positive-sentiment tweet.
#[must_use]
pub fn build_positive_tweet() -> Value {
    build_tweet(
        "This is amazing! I absolutely love this product! #great",
        "happy_user",
    )
}

/// Build a negative-sentiment tweet.
#[must_use]
pub fn build_negative_tweet() -> Value {
    build_tweet(
        "This is terrible. Worst experience ever. Avoid at all costs.",
        "angry_user",
    )
}

/// Build a neutral-sentiment tweet.
#[must_use]
pub fn build_neutral_tweet() -> Value {
    build_tweet(
        "The meeting has been rescheduled to 3pm tomorrow.",
        "calendar_bot",
    )
}

/// Build a tweet with media (image/video).
#[must_use]
pub fn build_tweet_with_media() -> Value {
    let mut tweet = build_tweet("Check out this photo!", "photographer");
    tweet["has_media"] = json!(true);
    tweet["media_type"] = json!("image");
    tweet
}

/// Build a tweet with a thread (multiple parts).
#[must_use]
pub fn build_thread_tweet() -> Value {
    let mut tweet = build_tweet("This is the start of a thread (1/5)", "thread_author");
    tweet["has_thread"] = json!(true);
    tweet["thread_id"] = json!("thread_abc123");
    tweet["thread_position"] = json!(1);
    tweet
}

/// Build a tweet from a verified account.
#[must_use]
pub fn build_verified_tweet() -> Value {
    let mut tweet = build_tweet("Official announcement from our team.", "official_account");
    tweet["is_verified"] = json!(true);
    tweet
}

/// Build a tweet with replies.
#[must_use]
pub fn build_tweet_with_replies(reply_count: usize) -> Value {
    let mut tweet = build_tweet("What do you think about this?", "asker");
    let replies: Vec<Value> = (0..reply_count)
        .map(|i| {
            json!({
                "author": format!("replier_{}", i),
                "text": format!("Reply number {} to this tweet", i + 1)
            })
        })
        .collect();
    tweet["replies"] = json!(replies);
    tweet["metrics"]["replies"] = json!(reply_count as u64);
    tweet
}

/// Build a tweet with custom metrics.
#[must_use]
pub fn build_tweet_with_metrics(likes: u64, retweets: u64, replies: u64, views: u64) -> Value {
    let mut tweet = build_tweet("Popular tweet content here", "influencer");
    tweet["metrics"]["likes"] = json!(likes);
    tweet["metrics"]["retweets"] = json!(retweets);
    tweet["metrics"]["replies"] = json!(replies);
    tweet["metrics"]["views"] = json!(views);
    tweet
}

/// Build a tweet that looks like an ad/promotion.
#[must_use]
pub fn build_promotional_tweet() -> Value {
    let mut tweet = build_tweet("Buy now! Limited time offer! 50% off!", "sponsor");
    tweet["is_promoted"] = json!(true);
    tweet["promoted_by"] = json!("sponsor_brand");
    tweet
}

// ============================================================================
// Config Factories
// ============================================================================

/// Create a default `TwitterActivityConfig` for tests.
#[must_use]
pub fn test_twitter_config() -> TwitterActivityConfig {
    TwitterActivityConfig::default()
}

/// Create a `TaskConfig` with custom values for testing.
#[must_use]
pub fn test_task_config() -> TaskConfig {
    TaskConfig {
        duration_ms: 60_000,
        candidate_count: 10,
        thread_depth: 3,
        max_actions_per_scan: 5,
        scroll_count: 5,
        weights: None,
        llm_enabled: false,
        llm_api_key: None,
        smart_decision_enabled: false,
        sentiment_templates: Default::default(),
        enhanced_sentiment_enabled: false,
        dry_run_actions: true,
        simulate_only: true,
        seed: 42,
    }
}

/// Create a `TaskConfig` for engagement testing with specific limits.
#[allow(dead_code, clippy::too_many_arguments)]
#[must_use]
pub fn test_task_config_with_limits(
    _max_likes: u32,
    _max_retweets: u32,
    _max_follows: u32,
    _max_replies: u32,
    _max_total: u32,
) -> TaskConfig {
    let mut config = test_task_config();
    config.dry_run_actions = true;
    config.simulate_only = true;
    config
}

/// Create a standard test payload JSON.
#[must_use]
pub fn test_payload() -> Value {
    json!({
        "duration_ms": 60000,
        "candidate_count": 10,
        "thread_depth": 3,
        "max_actions_per_scan": 5,
        "simulate_only": true,
        "dry_run_actions": true,
        "weights": {
            "like_prob": 0.5,
            "retweet_prob": 0.3,
            "follow_prob": 0.2,
            "reply_prob": 0.1,
            "thread_dive_prob": 0.3
        }
    })
}

// ============================================================================
// Session State Builders
// ============================================================================

/// Create a `SessionState` with default limits for testing.
#[must_use]
pub fn test_session_state() -> SessionState {
    let limits = EngagementLimits::default();
    SessionState::new(limits, 60_000, 100)
}

/// Create a `SessionState` with custom limits and duration.
#[allow(dead_code, clippy::too_many_arguments)]
#[must_use]
pub fn test_session_state_with_limits(
    max_likes: u32,
    max_retweets: u32,
    max_follows: u32,
    max_replies: u32,
    max_thread_dives: u32,
    max_bookmarks: u32,
    max_quote_tweets: u32,
    max_total: u32,
    duration_ms: u64,
) -> SessionState {
    let limits = EngagementLimits::with_limits(
        max_likes,
        max_retweets,
        max_follows,
        max_replies,
        max_thread_dives,
        max_bookmarks,
        max_quote_tweets,
        max_total,
    );
    SessionState::new(limits, duration_ms, 100)
}

/// Create a `TweetActionTracker` for testing action chain prevention.
#[must_use]
pub fn test_action_tracker(min_delay_ms: u64) -> TweetActionTracker {
    TweetActionTracker::new(min_delay_ms)
}

/// Create `EngagementCounters` with some actions already recorded.
#[must_use]
pub fn test_counters_with_actions(
    likes: u32,
    retweets: u32,
    follows: u32,
    replies: u32,
) -> EngagementCounters {
    let mut counters = EngagementCounters::new();
    for _ in 0..likes {
        counters.increment_like();
    }
    for _ in 0..retweets {
        counters.increment_retweet();
    }
    for _ in 0..follows {
        counters.increment_follow();
    }
    for _ in 0..replies {
        counters.increment_reply();
    }
    counters
}

// ============================================================================
// Persona Helpers
// ============================================================================

/// Create standard test persona weights.
#[must_use]
pub fn test_persona_weights() -> PersonaWeights {
    PersonaWeights {
        like_prob: 0.5,
        retweet_prob: 0.3,
        follow_prob: 0.2,
        reply_prob: 0.1,
        quote_prob: 0.05,
        bookmark_prob: 0.05,
        thread_dive_prob: 0.3,
        interest_multiplier: 1.0,
    }
}

/// Create persona weights that heavily favor a single action type.
#[allow(dead_code)]
#[must_use]
pub fn test_persona_favoring_likes() -> PersonaWeights {
    PersonaWeights {
        like_prob: 0.95,
        retweet_prob: 0.01,
        follow_prob: 0.01,
        reply_prob: 0.01,
        quote_prob: 0.01,
        bookmark_prob: 0.01,
        thread_dive_prob: 0.01,
        interest_multiplier: 1.0,
    }
}

/// Create persona weights that heavily favor replies.
#[allow(dead_code)]
#[must_use]
pub fn test_persona_favoring_replies() -> PersonaWeights {
    PersonaWeights {
        like_prob: 0.01,
        retweet_prob: 0.01,
        follow_prob: 0.01,
        reply_prob: 0.95,
        quote_prob: 0.01,
        bookmark_prob: 0.01,
        thread_dive_prob: 0.01,
        interest_multiplier: 1.0,
    }
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that a `SessionState` is in a valid initial state.
///
/// Checks that:
/// - The session is not expired
/// - Remaining time is positive
/// - All actions are initially allowed
/// - Action summary is (0, `max_total`)
pub fn assert_session_valid(session: &SessionState, expected_max_total: u32) {
    assert!(!session.is_expired(), "New session should not be expired");
    assert!(
        session.remaining_time().as_millis() > 0,
        "New session should have remaining time"
    );
    assert!(
        session.is_action_allowed("like"),
        "New session should allow likes"
    );
    assert_eq!(
        session.action_summary(),
        (0, expected_max_total),
        "New session should have 0 actions out of {expected_max_total}"
    );
}

/// Assert that an action is allowed and record it.
pub fn assert_action_allowed(session: &mut SessionState, tweet_id: &str, action: &'static str) {
    assert!(
        session.is_action_allowed(action),
        "Action '{action}' should be allowed"
    );
    session.record_action(&TweetId::from_unchecked(tweet_id), action);
}

/// Assert that an action is blocked (either by limit or cooldown).
pub fn assert_action_blocked(session: &SessionState, action: &str) {
    assert!(
        !session.is_action_allowed(action),
        "Action '{action}' should be blocked"
    );
}

/// Assert that total actions count matches expected value.
#[allow(dead_code)]
pub fn assert_total_actions(session: &SessionState, expected: u32) {
    assert_eq!(
        session.counters.total_actions(),
        expected,
        "Total actions should be {expected}"
    );
}

/// Assert that remaining time is approximately expected.
#[allow(dead_code)]
pub fn assert_remaining_time_approx(session: &SessionState, expected_ms: u64, tolerance_ms: u64) {
    let remaining = session.remaining_time().as_millis() as u64;
    let diff = remaining.abs_diff(expected_ms);
    assert!(
        diff <= tolerance_ms,
        "Remaining time {remaining}ms should be close to {expected_ms}ms (within {tolerance_ms}ms)"
    );
}

// ============================================================================
// TweetActionTracker Assertions
// ============================================================================

/// Assert that a tweet action is allowed by the tracker.
#[allow(dead_code)]
pub fn assert_tracker_allows(tracker: &TweetActionTracker, tweet_id: &str) {
    assert!(
        tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)),
        "Tracker should allow action on tweet '{tweet_id}'"
    );
}

/// Assert that a tweet action is blocked by the tracker (cooldown).
#[allow(dead_code)]
pub fn assert_tracker_blocks(tracker: &TweetActionTracker, tweet_id: &str) {
    assert!(
        !tracker.can_perform_action(&TweetId::from_unchecked(tweet_id)),
        "Tracker should block action on tweet '{tweet_id}'"
    );
}

// ============================================================================
// EngagementLimits Assertions
// ============================================================================

/// Assert that all default action types are allowed.
pub fn assert_all_actions_allowed(limits: &EngagementLimits, counters: &EngagementCounters) {
    assert!(limits.can_like(counters), "like should be allowed");
    assert!(limits.can_retweet(counters), "retweet should be allowed");
    assert!(limits.can_follow(counters), "follow should be allowed");
    assert!(limits.can_reply(counters), "reply should be allowed");
    assert!(limits.can_dive(counters), "dive should be allowed");
    assert!(limits.can_bookmark(counters), "bookmark should be allowed");
    assert!(
        limits.can_quote_tweet(counters),
        "quote_tweet should be allowed"
    );
}

/// Assert that all actions are blocked.
pub fn assert_all_actions_blocked(limits: &EngagementLimits, counters: &EngagementCounters) {
    assert!(!limits.can_like(counters), "like should be blocked");
    assert!(!limits.can_retweet(counters), "retweet should be blocked");
    assert!(!limits.can_follow(counters), "follow should be blocked");
    assert!(!limits.can_reply(counters), "reply should be blocked");
    assert!(!limits.can_dive(counters), "dive should be blocked");
    assert!(!limits.can_bookmark(counters), "bookmark should be blocked");
    assert!(
        !limits.can_quote_tweet(counters),
        "quote_tweet should be blocked"
    );
}

// ============================================================================
// Error Classification Helpers
// ============================================================================

/// Create a transient error message (for retry testing).
#[must_use]
pub fn transient_error(message: &str) -> String {
    format!("stale element reference: {message}")
}

/// Create a permanent error message.
#[must_use]
pub fn permanent_error(message: &str) -> String {
    format!("invalid selector syntax: {message}")
}

/// Create a fatal error message.
#[must_use]
pub fn fatal_error(message: &str) -> String {
    format!("browser disconnected: {message}")
}

// ============================================================================
// Async Test Helpers
// ============================================================================

/// Run an async block synchronously for tests that need it.
/// Uses `tokio::runtime::Runtime` for single-threaded execution.
#[allow(dead_code)]
pub fn run_async<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime for test");
    rt.block_on(future)
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Generate a simple unique ID for test tweets.
fn fast_random_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

// ============================================================================
// Tests for the Helpers Themselves
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tdd_red_build_tweet_creates_valid_tweet() {
        let tweet = build_tweet("test content", "test_author");
        assert_eq!(tweet["author"], "test_author");
        assert_eq!(tweet["text"], "test content");
        assert!(tweet["tweet_id"].as_str().unwrap().starts_with("tweet_"));
    }

    #[test]
    fn tdd_green_build_positive_tweet_contains_positive_language() {
        let tweet = build_positive_tweet();
        let text = tweet["text"].as_str().unwrap().to_lowercase();
        assert!(
            text.contains("amazing") || text.contains("love"),
            "Positive tweet should contain positive language"
        );
    }

    #[test]
    fn tdd_green_test_session_state_creation() {
        let session = test_session_state();
        assert_session_valid(&session, 10);
    }

    #[test]
    fn tdd_green_test_persona_weights_have_defaults() {
        let weights = test_persona_weights();
        assert!((0.0..=1.0).contains(&weights.like_prob));
        assert!((0.0..=1.0).contains(&weights.retweet_prob));
    }

    #[test]
    fn tdd_green_test_counters_with_actions_works() {
        let counters = test_counters_with_actions(3, 2, 1, 1);
        assert_eq!(counters.likes, 3);
        assert_eq!(counters.retweets, 2);
        assert_eq!(counters.follows, 1);
        assert_eq!(counters.replies, 1);
        assert_eq!(counters.total_actions(), 7);
    }

    #[test]
    fn tdd_green_test_tweet_with_replies_includes_replies() {
        let tweet = build_tweet_with_replies(3);
        let replies = tweet["replies"].as_array().unwrap();
        assert_eq!(replies.len(), 3);
    }

    #[test]
    fn tdd_green_assert_all_actions_allowed_works_on_fresh_counters() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();
        assert_all_actions_allowed(&limits, &counters);
    }

    #[test]
    fn tdd_green_assert_all_actions_blocked_works_on_full_counters() {
        let limits = EngagementLimits::with_limits(0, 0, 0, 0, 0, 0, 0, 0);
        let counters = EngagementCounters::new();
        assert_all_actions_blocked(&limits, &counters);
    }
}
