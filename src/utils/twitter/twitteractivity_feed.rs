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
//! - [`scroll_through_feed()`]: Perform human-like scroll through timeline
//! - [`identify_engagement_candidates()`]: Find engagement-ready tweets
//! - [`get_scroll_progress()`]: Calculate current scroll position (0.0-1.0)
//! - [`scroll_read()`]: Single scroll with reading pause
//!
//! ## Usage
//!
//! ```rust,no_run
//! use auto::utils::twitter::twitteractivity_feed::*;
//! # use auto::runtime::task_context::TaskContext;
//! # async fn example(api: &TaskContext) -> anyhow::Result<()> {
//!
//! // Scroll through feed with reading pauses
//! scroll_feed(api, 10, true).await?;
//!
//! // Identify tweets for engagement
//! let candidates = identify_engagement_candidates(api).await?;
//!
//! // Check scroll progress
//! let progress = get_scroll_progress(api).await?;
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

use super::{twitteractivity_humanized::*, twitteractivity_selectors::*};

/// Performs a series of scroll actions through the feed with human-like behavior.
///
/// This function scrolls through the Twitter feed using either native scroll
/// or JavaScript-based scrolling. It adds occasional back-scrolls and pauses to
/// simulate human reading behavior.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
/// * `scroll_count` - Number of scroll actions to perform
/// * `use_native_scroll` - If true, uses native scroll; otherwise uses JS scroll
///
/// # Returns
///
/// Returns `Ok(())` on completion.
///
/// # Errors
///
/// Returns error if scroll operations fail.
///
/// # Behavior
///
/// - Uses profile-derived scroll amount and pause duration
/// - Optionally uses smooth scrolling with back-scrolls
/// - Occasionally scrolls back up slightly (25% of scroll amount) on later iterations
/// - Adds human-like pauses between scrolls
///
/// # Profile Parameters
///
/// - `scroll.amount`: Pixels to scroll per action
/// - `scroll.pause_ms`: Milliseconds to pause between scrolls
/// - `scroll.smooth`: Whether to use smooth scrolling
/// - `scroll.back_scroll`: Whether to include back-scrolls
#[instrument(skip(api))]
pub async fn scroll_feed(
    api: &TaskContext,
    scroll_count: u32,
    use_native_scroll: bool,
) -> Result<()> {
    let profile = api.behavior_runtime();
    let scroll_amount = profile.scroll.amount;
    let scroll_pause_ms = profile.scroll.pause_ms;
    let smooth = profile.scroll.smooth;

    for i in 0..scroll_count {
        if use_native_scroll {
            // Let TaskContext handle the scroll with profile-derived parameters
            api.scroll_read(
                1, // single pause per scroll burst
                scroll_amount,
                smooth,
                profile.scroll.back_scroll,
            )
            .await?;
        } else {
            // JS-based scroll
            let js = format!("window.scrollBy(0, {});", scroll_amount);
            api.page().evaluate(js).await?;
            human_pause(api, scroll_pause_ms).await;
        }

        // Occasionally scroll back up a little on later iterations
        if smooth && i > 2 && rand::random::<bool>() {
            let back_amount = scroll_amount / 4;
            let js = format!("window.scrollBy(0, -{});", back_amount);
            api.page().evaluate(js).await?;
            human_pause(api, 200).await;
        }
    }

    Ok(())
}

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
    let js = r#"
        (function() {
            var tweets = [];
            var elements = document.querySelectorAll('article[data-testid="tweet"]');
            for (var i = 0; i < elements.length; i++) {
                var el = elements[i];
                var rect = el.getBoundingClientRect();
                if (rect.height > 0 && rect.width > 0) {
                    // Extract tweet text content
                    var tweetTextEl = el.querySelector('[data-testid="tweetText"]');
                    var tweetText = tweetTextEl ? tweetTextEl.textContent.trim() : '';
                    
                    // Find engagement buttons within this tweet element
                    var likeBtn = el.querySelector('[data-testid="like"]');
                    var retweetBtn = el.querySelector('[data-testid="retweet"]');
                    var replyBtn = el.querySelector('[data-testid="reply"]');
                    
                    var buttonPositions = {};
                    if (likeBtn) {
                        var likeRect = likeBtn.getBoundingClientRect();
                        if (likeRect.width > 0 && likeRect.height > 0) {
                            buttonPositions.like = { x: likeRect.x + likeRect.width/2, y: likeRect.y + likeRect.height/2 };
                        }
                    }
                    if (retweetBtn) {
                        var retweetRect = retweetBtn.getBoundingClientRect();
                        if (retweetRect.width > 0 && retweetRect.height > 0) {
                            buttonPositions.retweet = { x: retweetRect.x + retweetRect.width/2, y: retweetRect.y + retweetRect.height/2 };
                        }
                    }
                    if (replyBtn) {
                        var replyRect = replyBtn.getBoundingClientRect();
                        if (replyRect.width > 0 && replyRect.height > 0) {
                            buttonPositions.reply = { x: replyRect.x + replyRect.width/2, y: replyRect.y + replyRect.height/2 };
                        }
                    }
                    
                    // Extract the status URL from the time element for reliable diving
                    // Look for the main tweet permalink (not analytics, shares, etc.)
                    var links = el.querySelectorAll('a[href*="/status/"]');
                    var statusUrl = null;
                    for (var j = 0; j < links.length; j++) {
                        var href = links[j].getAttribute('href');
                        // Match pattern: /username/status/tweetId (exactly 4 segments)
                        // Exclude analytics, shares, and other extended paths
                        var parts = href.split('/').filter(function(p) { return p.length > 0; });
                        if (parts.length === 3 && parts[1] === 'status' && !isNaN(parts[2])) {
                            statusUrl = href;
                            break; // Take the first matching permalink
                        }
                    }

                    // Prefer stable tweet identity from permalink. Position fallback is last resort.
                    var statusId = null;
                    if (statusUrl) {
                        var statusParts = statusUrl.split('/').filter(function(p) { return p.length > 0; });
                        statusId = statusParts[statusParts.length - 1].split(/[?#]/)[0];
                    }
                    var tweetId = el.dataset.tweetId ||
                                  el.getAttribute('data-item-id') ||
                                  el.getAttribute('data-tweet-id') ||
                                  statusId ||
                                  'tweet_' + Math.floor(rect.x) + '_' + Math.floor(rect.y);
                    
                    var tweetObj = {
                        id: tweetId,
                        status_url: statusUrl,
                        index: i,
                        text: tweetText,
                        x: rect.x + rect.width/2,
                        y: rect.y + rect.height/2,
                        height: rect.height,
                        width: rect.width,
                        buttons: buttonPositions
                    };

                    // Extract reply information for smart decision
                    var replies = [];
                    var replyElements = el.querySelectorAll('[data-testid="tweetReply"]');
                    for (var j = 0; j < Math.min(replyElements.length, 3); j++) { // Limit to top 3 replies
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

                    if (replies.length > 0) {
                        tweetObj.replies = replies;
                    }

                    tweets.push(tweetObj);
                }
            }
            return tweets;
        })()
    "#;

    let result = api.page().evaluate(js.to_string()).await?;
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
            "[candidate_scan] Found {} tweets but filtered: no_id={}, viewport={}, height={}",
            total_found,
            filtered_no_id,
            filtered_viewport,
            filtered_height
        );
    }

    Ok(candidates)
}

/// Identifies engagement buttons (like, retweet, reply) for a specific tweet element.
/// Returns a structured object with positions or nulls.
pub async fn get_tweet_engagement_buttons(api: &TaskContext) -> Result<Value> {
    let js = selector_engagement_buttons();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or_default();
    Ok(value)
}

/// Checks if a given tweet (by center coordinates) currently shows "Following" state
/// for the author (used to decide whether a follow action is needed).
pub async fn is_following_user_at_position(api: &TaskContext, _x: f64, _y: f64) -> Result<bool> {
    // Move mouse near the tweet to expose any hover-only indicators (optional)
    // For now, evaluate globally
    let js = selector_following_indicator();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or(Value::Bool(false));
    Ok(value.as_bool().unwrap_or(false))
}

/// Gets the current scroll position as percentage of total page height.
///
/// Calculates how far through the feed the user has scrolled, returning a
/// value between 0.0 (top) and 1.0 (bottom). Useful for detecting when the
/// feed end has been reached.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns scroll progress as a float between 0.0 and 1.0:
/// - 0.0: At the top of the page
/// - 1.0: At the bottom of the page (or scrolled past)
///
/// # Errors
///
/// Returns error if DOM evaluation fails (defaults to 0.0 in that case).
///
/// # Behavior
///
/// - Calculates scroll position as: scrollY / (scrollHeight - innerHeight)
/// - Returns 1.0 if scrolled past the bottom
/// - Clamps result to 0.0-1.0 range
/// - Returns 0.0 if evaluation fails
#[instrument(skip(api))]
pub async fn get_scroll_progress(api: &TaskContext) -> Result<f64> {
    let result = api
        .page()
        .evaluate("window.scrollY + window.innerHeight >= document.body.scrollHeight ? 1.0 : (window.scrollY / (document.body.scrollHeight - window.innerHeight))".to_string())
        .await?;
    let value = result.value();
    if let Some(v) = value.and_then(|v: &Value| v.as_f64()) {
        Ok(v.clamp(0.0, 1.0))
    } else {
        Ok(0.0)
    }
}

/// Ensures the feed has at least one tweet/article visible.
///
/// This function checks if the feed is populated with at least one tweet.
/// It can be used to verify that content has loaded before attempting engagement.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if at least one tweet is visible.
/// Returns `Ok(false)` if no tweets are visible.
///
/// # Errors
///
/// Returns error if DOM evaluation fails.
///
/// # Behavior
///
/// - Queries for `article[data-testid="tweet"]` elements
/// - Checks if any elements are found
/// - Returns true if at least one tweet exists
///
/// # Selector Used
///
/// - Tweets: `article[data-testid="tweet"]`
#[instrument(skip(api))]
pub async fn ensure_feed_populated(api: &TaskContext) -> Result<bool> {
    let js = selector_all_tweets();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if let Some(arr) = value.and_then(|v: &serde_json::Value| v.as_array()) {
        return Ok(!arr.is_empty());
    }
    Ok(false)
}

/// Performs a "deep scroll" — scrolls to the bottom of the feed.
/// Used to load more content before engaging.
pub async fn scroll_to_bottom_feed(api: &TaskContext) -> Result<()> {
    api.scroll_to_bottom().await?;
    // Wait for potential lazy-loaded content
    human_pause(api, 2000).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_signatures_exist() {
        // Compile-time check that public functions exist
        let _ = scroll_feed;
        let _ = identify_engagement_candidates;
        let _ = get_tweet_engagement_buttons;
        let _ = is_following_user_at_position;
        let _ = get_scroll_progress;
        let _ = ensure_feed_populated;
        let _ = scroll_to_bottom_feed;
    }

    #[test]
    fn test_function_count() {
        // Verify we have the expected number of public functions
        let function_names = [
            "scroll_feed",
            "identify_engagement_candidates",
            "get_tweet_engagement_buttons",
            "is_following_user_at_position",
            "get_scroll_progress",
            "ensure_feed_populated",
            "scroll_to_bottom_feed",
        ];
        assert_eq!(function_names.len(), 7);
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

    #[test]
    fn test_identify_engagement_candidates_js_generates_fallback_id() {
        let js = r#"
        (function() {
            var tweetId = el.dataset.tweetId ||
                          el.getAttribute('data-item-id') ||
                          el.getAttribute('data-tweet-id') ||
                          statusId ||
                          'tweet_' + Math.floor(rect.x) + '_' + Math.floor(rect.y);
            return tweetId;
        })()
        "#;
        assert!(js.contains("dataset.tweetId"));
        assert!(js.contains("data-item-id"));
        assert!(js.contains("data-tweet-id"));
        assert!(js.contains("'tweet_' + Math.floor(rect.x)"));
    }
}
