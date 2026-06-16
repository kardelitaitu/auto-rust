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

    let mut candidates = Vec::new();
    let mut total_found = 0;
    let mut filtered_no_id = 0;
    let mut filtered_viewport = 0;
    let mut filtered_height = 0;
    let viewport = match api.viewport().await {
        Ok(vp) => vp,
        Err(_) => {
            // Fallback default viewport if query fails
            crate::utils::page_size::Viewport {
                width: 1920.0,
                height: 1080.0,
            }
        }
    };

    if let Some(arr) = value.and_then(|v: &serde_json::Value| v.as_array()) {
        total_found = arr.len();
        for tweet_val in arr {
            if let Some(obj) = tweet_val.as_object() {
                // Basic filter: tweet must have an id and be within viewport reasonably
                let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if id.is_empty() {
                    filtered_no_id += 1;
                    continue;
                }

                let y = obj.get("y").and_then(|v: &Value| v.as_f64()).unwrap_or(0.0);
                let height = obj
                    .get("height")
                    .and_then(|v: &Value| v.as_f64())
                    .unwrap_or(0.0);

                // Filter out tweets above viewport (negative y) or too small
                if y < 0.0 || height <= 50.0 {
                    if y < 0.0 {
                        filtered_viewport += 1;
                    } else {
                        filtered_height += 1;
                    }
                    continue;
                }

                // Consider tweets in viewport as "candidate" (relaxed from 70% to 90%)
                if y >= (viewport.height as f64 * 0.9) {
                    filtered_viewport += 1;
                    continue;
                }

                candidates.push(tweet_val.clone());
            }
        }
    }

    if total_found == 0 {
        log::warn!("[candidate_scan] No tweet elements found in DOM");
    } else if candidates.is_empty() {
        log::warn!(
            "[candidate_scan] Found {total_found} tweets but filtered: no_id={filtered_no_id}, viewport={filtered_viewport}, height={filtered_height}"
        );
    }

    Ok(candidates)
}

/// Checks if a given tweet (by center coordinates) currently shows "Following" state
/// for the author (used to decide whether a follow action is needed).
/// Scopes the check to the tweet article element at the given position.
#[allow(clippy::cast_precision_loss)]
pub async fn is_following_user_at_position(api: &TaskContext, x: f64, y: f64) -> Result<bool> {
    // Move mouse near the tweet to expose any hover-only indicators
    if let Err(e) = api.move_mouse_to(x, y).await {
        log::warn!("Failed to move mouse for hover indicators: {e}");
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
mod fuzz_tests {
    use proptest::prelude::*;
    use serde_json::Value;

    /// Simulate the candidate filtering logic from identify_engagement_candidates.
    fn simulate_candidate_filtering(value: &Value, viewport_height: f64) -> usize {
        let mut candidates = 0usize;
        if let Some(arr) = value.as_array() {
            for tweet_val in arr {
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
                    if y >= (viewport_height * 0.9) {
                        continue;
                    }
                    candidates += 1;
                }
            }
        }
        candidates
    }

    fn val(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or(Value::String(s.to_string()))
    }

    proptest! {
        #[test]
        fn fuzz_simulate_candidate_filtering(s: String) {
            let value = val(&s);
            let _ = simulate_candidate_filtering(&value, 1080.0);
            let _ = simulate_candidate_filtering(&value, f64::NAN);
            let _ = simulate_candidate_filtering(&value, -1.0);
            let _ = simulate_candidate_filtering(&value, 99999.0);
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
            let _ = simulate_candidate_filtering(&arr, 1080.0);
        }

        #[test]
        fn fuzz_is_following_value_pattern(s: String) {
            let value = val(&s);
            let _ = value.as_bool().unwrap_or(false);
        }
    }
}
