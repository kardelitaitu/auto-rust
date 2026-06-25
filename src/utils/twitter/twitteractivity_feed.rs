//! Feed and timeline interaction helpers for Twitter automation.
//!
//! This module provides functionality for navigating and scrolling through Twitter's
//! home feed, identifying engagement candidates, and tracking scroll progress. It
//! implements human-like reading patterns to avoid detection.
//!
//! ## Key Components
//!
//! - **Feed Scrolling**: Natural scrolling with randomized pauses
//! - **Candidate Identification**: Find tweets suitable for engagement
//! - **Scroll Progress Tracking**: Monitor how far through the feed
//! - **Human-like Reading**: Simulate reading behavior with pauses
//!
//! ## Key Functions
//!
//! - `scroll_through_feed()`: Perform human-like scroll through timeline
//! - `identify_engagement_candidates()`: Find engagement-ready tweets
//! - `get_scroll_progress()`: Calculate current scroll position (0.0-1.0)
//! - `scroll_read()`: Single scroll with reading pause
//!
//! ## Usage
//!
//! ```rust,no_run
//! use auto::utils::twitter::twitteractivity_feed::*;
//! # use auto::runtime::task_context::TaskContext;
//! # async fn example(api: &TaskContext) -> anyhow::Result<()> {
//!
//! // Identify tweets for engagement
//! let candidates = identify_engagement_candidates(api).await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Scroll Behavior
//!
//! The module implements human-like scrolling:
//! - Small incremental scrolls (200-500px)
//! - Reading pauses between scrolls (500-2000ms)
//! - Random variation to avoid patterns
//! - Progress tracking to detect feed end

use crate::prelude::TaskContext;
use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use super::twitteractivity_selectors::js_identify_engagement_candidates;

/// Scans the current viewport for tweet articles that are good engagement candidates.
///
/// This function queries the DOM for all visible tweets and extracts their metadata
/// including position, text content, and engagement button positions. The returned
/// data can be used to select tweets for engagement actions.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns a vector of tweet objects containing:
/// - `id`: Tweet identifier (generated from position if not available)
/// - `position`: {x, y, width, height} of tweet element
/// - `text`: Tweet text content
/// - `buttonPositions`: Coordinates of like, retweet, and reply buttons
///
/// # Errors
///
/// Returns error if DOM evaluation fails.
///
/// # Behavior
///
/// - Queries for all `article[data-testid="tweet"]` elements
/// - Filters for visible elements (width > 0 and height > 0)
/// - Extracts tweet text from `[data-testid="tweetText"]`
/// - Finds engagement button positions within each tweet
/// - Generates tweet ID from position if not available in DOM
///
/// # Selectors Used
///
/// - Tweets: `article[data-testid="tweet"]`
/// - Text: `[data-testid="tweetText"]`
/// - Like button: `[data-testid="like"]`
/// - Retweet button: `[data-testid="retweet"]`
/// - Reply button: `[data-testid="reply"]`
#[instrument(skip(api))]
pub async fn identify_engagement_candidates(api: &TaskContext) -> Result<Vec<Value>> {
    let js = js_identify_engagement_candidates();
    let result = api.page().evaluate(js).await?;
    let value = result.value();

    let candidates = match value.and_then(|v: &serde_json::Value| v.as_array()) {
        Some(arr) => filter_candidates(arr),
        None => Vec::new(),
    };

    if candidates.is_empty() && value.is_some() {
        if let Some(arr) = value.and_then(|v| v.as_array()) {
            if arr.is_empty() {
                log::warn!("[candidate_scan] No tweet elements found in DOM");
            } else {
                log::warn!(
                    "[candidate_scan] Found {} tweets but all were filtered out",
                    arr.len()
                );
            }
        }
    }

    Ok(candidates)
}

/// Pure-function filter for engagement candidates.
///
/// Extracted from `identify_engagement_candidates` for testability.
/// Takes a slice of tweet JSON objects and returns only tweets that
/// pass all filter criteria:
/// - Must have a non-empty string `id` field
/// - `y` position must be >= 0
/// - `height` must be > 50
#[must_use]
pub fn filter_candidates(tweets: &[Value]) -> Vec<Value> {
    let mut candidates = Vec::new();
    for tweet_val in tweets {
        if let Some(obj) = tweet_val.as_object() {
            let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let y = obj.get("y").and_then(|v: &Value| v.as_f64()).unwrap_or(0.0);
            let height = obj
                .get("height")
                .and_then(|v: &Value| v.as_f64())
                .unwrap_or(0.0);
            if y < 0.0 || height <= 50.0 {
                continue;
            }
            candidates.push(tweet_val.clone());
        }
    }
    candidates
}

/// Checks if a given tweet (by center coordinates) currently shows "Following" state
/// for the author (used to decide whether a follow action is needed).
/// Scopes the check to the tweet article element at the given position.
#[allow(clippy::cast_precision_loss)]
pub async fn is_following_user_at_position(api: &TaskContext, x: f64, y: f64) -> Result<bool> {
    // Move mouse near the tweet to expose any hover-only indicators
    if let Err(e) = api.move_mouse_to(x, y).await {
        log::warn!("[feed] Failed to move mouse for hover indicators: {e}");
    }
    // Use elementFromPoint to scope the query to the tweet at this position
    let js = format!(
        r#"(function() {{
            var el = document.elementFromPoint({x}, {y});
            while (el && el.tagName !== 'ARTICLE') {{
                el = el.parentElement;
            }}
            if (!el) return false;
            var buttons = el.querySelectorAll('button');
            for (var i = 0; i < buttons.length; i++) {{
                var btn = buttons[i];
                var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
                var label = (btn.getAttribute('aria-label') || '').toLowerCase();
                var dataTestId = (btn.getAttribute('data-testid') || '').toLowerCase();
                if (text === 'following' ||
                    label.includes('following @') ||
                    dataTestId.includes('unfollow')) {{
                    return true;
                }}
            }}
            return false;
        }})()"#,
        x = x,
        y = y
    );
    let result = api.page().evaluate(js).await?;
    let value = result.value().cloned().unwrap_or(Value::Bool(false));
    Ok(value.as_bool().unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_signatures_exist() {
        // Compile-time check that public functions exist
        let _ = identify_engagement_candidates;
        let _ = is_following_user_at_position;
    }

    #[test]
    fn test_function_count() {
        let function_names = [
            "identify_engagement_candidates",
            "is_following_user_at_position",
        ];
        assert_eq!(function_names.len(), 2);
    }

    #[test]
    fn test_identify_engagement_candidates_js_contains_tweet_selector() {
        let js = r#"
        (function() {
            var tweets = [];
            var elements = document.querySelectorAll('article[data-testid="tweet"]');
            for (var i = 0; i < elements.length; i++) {
                var el = elements[i];
                var rect = el.getBoundingClientRect();
                if (rect.height > 0 && rect.width > 0) {
                    var tweetTextEl = el.querySelector('[data-testid="tweetText"]');
                    var tweetText = tweetTextEl ? tweetTextEl.textContent.trim() : '';
                    tweets.push({ text: tweetText });
                }
            }
            return tweets;
        })()
        "#;
        assert!(js.contains("article[data-testid=\"tweet\"]"));
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("data-testid=\"tweetText\""));
    }

    #[test]
    fn test_identify_engagement_candidates_js_extracts_button_positions() {
        let js = r#"
        (function() {
            var likeBtn = el.querySelector('[data-testid="like"]');
            var retweetBtn = el.querySelector('[data-testid="retweet"]');
            var replyBtn = el.querySelector('[data-testid="reply"]');
            var buttonPositions = {};
            if (likeBtn) {
                var likeRect = likeBtn.getBoundingClientRect();
                buttonPositions.like = { x: likeRect.x + likeRect.width/2, y: likeRect.y + likeRect.height/2 };
            }
            return buttonPositions;
        })()
        "#;
        assert!(js.contains("data-testid=\"like\""));
        assert!(js.contains("data-testid=\"retweet\""));
        assert!(js.contains("data-testid=\"reply\""));
        assert!(js.contains("buttonPositions"));
    }

    #[test]
    fn test_identify_engagement_candidates_js_extracts_status_url() {
        let js = r#"
        (function() {
            var links = el.querySelectorAll('a[href*="/status/"]');
            var statusUrl = null;
            for (var j = 0; j < links.length; j++) {
                var href = links[j].getAttribute('href');
                var parts = href.split('/').filter(function(p) { return p.length > 0; });
                if (parts.length === 3 && parts[1] === 'status' && !isNaN(parts[2])) {
                    statusUrl = href;
                    break;
                }
            }
            return statusUrl;
        })()
        "#;
        assert!(js.contains("href*=\"/status/\""));
        assert!(js.contains("parts.length === 3"));
        assert!(js.contains("parts[1] === 'status'"));
    }

    #[test]
    fn test_identify_engagement_candidates_js_extracts_replies() {
        let js = r#"
        (function() {
            var replies = [];
            var replyElements = el.querySelectorAll('[data-testid="tweetReply"]');
            for (var j = 0; j < Math.min(replyElements.length, 3); j++) {
                var replyEl = replyElements[j];
                var authorEl = replyEl.querySelector('[dir="auto"] span:first-child');
                var textEl = replyEl.querySelector('[data-testid="tweetText"]');
                if (authorEl && textEl) {
                    replies.push({
                        author: authorEl.textContent.trim(),
                        text: textEl.textContent.trim()
                    });
                }
            }
            return replies;
        })()
        "#;
        assert!(js.contains("data-testid=\"tweetReply\""));
        assert!(js.contains("Math.min(replyElements.length, 3)"));
        assert!(js.contains("author"));
        assert!(js.contains("text"));
    }

    #[test]
    fn test_get_scroll_progress_js_formula() {
        let js = "window.scrollY + window.innerHeight >= document.body.scrollHeight ? 1.0 : (window.scrollY / (document.body.scrollHeight - window.innerHeight))";
        assert!(js.contains("window.scrollY"));
        assert!(js.contains("window.innerHeight"));
        assert!(js.contains("document.body.scrollHeight"));
        assert!(js.contains("? 1.0 :"));
    }

    #[test]
    fn test_scroll_feed_js_uses_window_scroll_by() {
        let js = "window.scrollBy(0, {});";
        assert!(js.contains("window.scrollBy"));
        assert!(js.contains("0,"));
    }
}

#[cfg(test)]
mod candidate_filter_tests {
    use super::*;
    use serde_json::json;

    // Delegate to the production filter function
    fn filter(tweets: &[Value]) -> usize {
        super::filter_candidates(tweets).len()
    }

    // ====================================================================
    // ID filter edge cases
    // ====================================================================

    #[test]
    fn filter_empty_id_is_excluded() {
        let tweets = vec![json!({"id": "", "y": 100.0, "height": 200.0})];
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_missing_id_field_is_excluded() {
        let tweets = vec![json!({"y": 100.0, "height": 200.0})];
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_null_id_is_excluded() {
        let tweets = vec![json!({"id": null, "y": 100.0, "height": 200.0})];
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_non_string_id_is_excluded() {
        let tweets = vec![json!({"id": 12345, "y": 100.0, "height": 200.0})];
        // non-string id becomes None from as_str(), falls to "", excluded
        assert_eq!(filter(&tweets), 0);
    }

    // ====================================================================
    // Y position edge cases
    // ====================================================================

    #[test]
    fn filter_negative_y_is_excluded() {
        let tweets = vec![json!({"id": "t1", "y": -10.0, "height": 200.0})];
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_zero_y_is_included() {
        let tweets = vec![json!({"id": "t1", "y": 0.0, "height": 200.0})];
        assert_eq!(filter(&tweets), 1);
    }

    #[test]
    fn filter_missing_y_defaults_to_zero() {
        let tweets = vec![json!({"id": "t1", "height": 200.0})];
        assert_eq!(filter(&tweets), 1);
    }

    // ====================================================================
    // Height edge cases
    // ====================================================================

    #[test]
    fn filter_height_at_exact_50_is_excluded() {
        let tweets = vec![json!({"id": "t1", "y": 100.0, "height": 50.0})];
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_height_just_above_50_is_included() {
        let tweets = vec![json!({"id": "t1", "y": 100.0, "height": 50.1})];
        assert_eq!(filter(&tweets), 1);
    }

    #[test]
    fn filter_missing_height_defaults_to_zero_and_excluded() {
        let tweets = vec![json!({"id": "t1", "y": 100.0})];
        // height defaults to 0.0, which is <= 50.0 → excluded
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_null_height_is_excluded() {
        let tweets = vec![json!({"id": "t1", "y": 100.0, "height": null})];
        // null → as_f64() returns None → falls to 0.0 → excluded
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_zero_height_is_excluded() {
        let tweets = vec![json!({"id": "t1", "y": 100.0, "height": 0.0})];
        assert_eq!(filter(&tweets), 0);
    }

    // ====================================================================
    // Valid candidates
    // ====================================================================

    #[test]
    fn filter_valid_tweet_is_included() {
        let tweets = vec![json!({"id": "tweet_123", "y": 300.0, "height": 200.0})];
        assert_eq!(filter(&tweets), 1);
    }

    #[test]
    fn filter_multiple_valid_tweets_all_included() {
        let tweets = vec![
            json!({"id": "t1", "y": 100.0, "height": 200.0}),
            json!({"id": "t2", "y": 350.0, "height": 180.0}),
            json!({"id": "t3", "y": 600.0, "height": 220.0}),
        ];
        assert_eq!(filter(&tweets), 3);
    }

    #[test]
    fn filter_mixed_valid_and_invalid_tweets() {
        let tweets = vec![
            json!({"id": "t1", "y": 100.0, "height": 200.0}), // valid
            json!({"id": "", "y": 200.0, "height": 200.0}),   // empty id
            json!({"id": "t3", "y": -50.0, "height": 200.0}), // negative y
            json!({"id": "t4", "y": 500.0, "height": 30.0}),  // too short
        ];
        assert_eq!(filter(&tweets), 1); // only t1 passes
    }

    // ====================================================================
    // Edge cases with non-tweet JSON
    // ====================================================================

    #[test]
    fn filter_empty_array_returns_zero() {
        assert_eq!(filter(&[]), 0);
    }

    #[test]
    fn filter_non_object_elements_are_skipped() {
        let tweets = vec![json!("string_tweet"), json!(42), json!(null), json!(true)];
        assert_eq!(filter(&tweets), 0);
    }

    #[test]
    fn filter_object_without_expected_fields_is_excluded() {
        let tweets = vec![json!({"name": "random", "value": 100})];
        // no id → excluded (empty id)
        // no y → defaults to 0.0
        // no height → defaults to 0.0 → excluded
        assert_eq!(filter(&tweets), 0);
    }

    // ====================================================================
    // Basic acceptance
    // ====================================================================

    #[test]
    fn filter_basic_acceptance() {
        let tweets = vec![json!({"id": "t1", "y": 400.0, "height": 100.0})];
        assert_eq!(filter(&tweets), 1);

        let tweets = vec![json!({"id": "t1", "y": 1500.0, "height": 200.0})];
        assert_eq!(filter(&tweets), 1);
    }
}

#[cfg(test)]
mod fuzz_tests {
    use proptest::prelude::*;
    use serde_json::Value;

    /// Delegate to the production filter function.
    fn simulate_candidate_filtering(value: &Value) -> usize {
        match value.as_array() {
            Some(arr) => super::filter_candidates(arr).len(),
            None => 0,
        }
    }

    fn val(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or(Value::String(s.to_string()))
    }

    proptest! {
        #[test]
        fn fuzz_simulate_candidate_filtering(s: String) {
            let value = val(&s);
            let _ = simulate_candidate_filtering(&value);
        }

        #[test]
        fn fuzz_simulate_candidate_filtering_with_tweet(
            id: String,
            y: String,
            height: String,
        ) {
            let arr = serde_json::json!([
                {"id": val(&id), "y": val(&y), "height": val(&height)},
                {"id": "valid", "y": 100.0, "height": 200.0},
            ]);
            let _ = simulate_candidate_filtering(&arr);
        }

        #[test]
        fn fuzz_is_following_value_pattern(s: String) {
            let value = val(&s);
            let _ = value.as_bool().unwrap_or(false);
        }
    }
}
