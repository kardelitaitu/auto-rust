//! Thread dive and deep-read helpers for Twitter automation.
//!
//! This module provides functionality for navigating into tweet threads and reading
//! their full content by scrolling through replies. It also includes a caching mechanism
//! to capture thread data (author, text, replies) for later LLM processing.
//!
//! ## Key Components
//!
//! - **Thread Diving**: Click into tweet threads to open detailed view
//! - **Thread Reading**: Scroll through threads with human-like pauses
//! - **Thread Caching**: Incrementally capture replies during scrolling
//!
//! ## Key Functions
//!
//! - [`dive_into_thread()`]: Click a tweet link to open thread detail view
//! - [`read_full_thread()`]: Scroll through a thread with optional caching
//! - [`extract_initial_thread_data()`]: Capture root tweet author and text
//! - [`extract_visible_replies()`]: Extract replies visible in current view
//!
//! ## Usage
//!
//! ```ignore
//! use auto::utils::twitter::twitteractivity_dive::*;
//! use auto::utils::twitter::twitteractivity_state::ThreadCache;
//! # use auto::runtime::task_context::TaskContext;
//! # async fn example(api: &TaskContext) -> anyhow::Result<()> {
//!
//! // Dive into a thread and read it
//! let status_url = "https://x.com/user/status/123";
//! let outcome = dive_into_thread(api, status_url).await?;
//! if outcome.opened {
//!     let mut cache = ThreadCache::default();
//!     read_full_thread(api, 10, &mut cache).await?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Thread Caching
//!
//! The [`ThreadCache`] struct captures thread data incrementally:
//! - Initial tweet data (author, text) captured after thread opens
//! - Replies extracted before each scroll (up to 20 total)
//! - Cache can be used for LLM reply/quote generation

use crate::prelude::TaskContext;
use anyhow::{Context, Result};
use log::info;
use tracing::instrument;

use super::twitteractivity_selectors::{
    js_extract_all_tweets, js_get_current_url, selector_all_tweets,
};
use super::twitteractivity_types::StatusUrl;

#[derive(Debug, Clone, Default)]
pub struct DiveIntoThreadOutcome {
    pub opened: bool,
    pub used_fallback_target: bool,
}

fn status_id_from_url(status_url: &str) -> Option<String> {
    StatusUrl::from_unchecked(status_url)
        .tweet_id()
        .map(String::from)
}

/// Clicks on a tweet to open it in the thread/detail view.
///
/// This function navigates into a tweet's thread by clicking on a tweet link
/// identified by its status URL. It waits for the thread view to open and
/// optionally captures initial thread data for caching.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
/// * `status_url` - The status URL of the tweet (e.g., "/username/status/123456")
///
/// # Returns
///
/// Returns `DiveIntoThreadOutcome` containing:
/// - `opened`: Whether the thread view opened successfully
/// - `used_fallback_target`: Whether a fallback selector was used
///
/// # Errors
///
/// Returns error if the click operation fails unexpectedly.
///
/// # Behavior
///
/// - Returns early with `opened: false` if `status_url` is empty
/// - Constructs a link selector from the status URL
/// - Clicks the link and waits for thread view to appear
/// - Uses multiple selector strategies to detect thread view opening
/// - Extracts initial thread data (author, text) if thread opens successfully
///
/// # Selectors Used
///
/// The function waits for any of these selectors to become visible:
/// - `div[role="dialog"]` - Modal dialog
/// - `div[data-testid="tweetDetail"]` - Thread detail view
/// - `div[data-testid="tweetThread"]` - Thread view
/// - `[aria-label="Timeline: Thread"]` - Thread timeline
/// - `article[data-testid="tweet"]` - Tweet in detail view
#[instrument(skip(api))]
pub async fn dive_into_thread(
    api: &TaskContext,
    status_url: &str,
) -> Result<DiveIntoThreadOutcome> {
    if status_url.is_empty() {
        info!("Dive skipped: empty status_url");
        return Ok(DiveIntoThreadOutcome {
            opened: false,
            used_fallback_target: false,
        });
    }

    info!("Attempting to dive into thread: {}", status_url);

    // Click the tweet link using the high-level API (handles scrolling, movement, clicking)
    // Escape single quotes in status_url to prevent CSS selector injection
    let escaped_url = status_url.replace('\'', "\\'");
    let link_selector = format!("a[href='{escaped_url}']");
    info!("Clicking tweet link selector: {}", link_selector);
    api.click(&link_selector)
        .await
        .context("Dive failed: click on link failed")?;
    info!("Clicked tweet link, waiting for thread view...");

    // Wait for thread/modal view to open (tweet detail or thread)
    let selectors = [
        r#"div[role="dialog"]"#, // Modal dialog (common for tweet details)
        r#"div[data-testid="tweetDetail"]"#, // Thread detail view
        r#"div[data-testid="tweetThread"]"#, // Thread view
        r#"[aria-label="Timeline: Thread"]"#, // Thread timeline
    ];
    let detail_visible = api
        .wait_for_any_visible_selector(&selectors, 5_000)
        .await
        .unwrap_or(false);
    let current_url = api
        .page()
        .evaluate(js_get_current_url())
        .await
        .ok()
        .and_then(|result| {
            result
                .value()
                .and_then(|value| value.as_str().map(str::to_owned))
        })
        .unwrap_or_default();
    let target_status_id = status_id_from_url(status_url);
    let url_matches = target_status_id.as_deref().map_or_else(
        || current_url.contains(status_url),
        |id| current_url.contains(&format!("/status/{id}")),
    );
    let tweet_article_visible = api
        .wait_for_any_visible_selector(&[r#"article[data-testid="tweet"]"#], 3_000)
        .await
        .unwrap_or(false);
    let thread_opened = url_matches && (detail_visible || tweet_article_visible);

    if thread_opened {
        info!("Thread view opened successfully");
    } else {
        info!(
            "Thread view did not open within timeout or URL mismatch (detail_visible={detail_visible}, tweet_article_visible={tweet_article_visible}, url_matches={url_matches}, current_url={current_url})"
        );
    }

    Ok(DiveIntoThreadOutcome {
        opened: thread_opened,
        used_fallback_target: false,
    })
}

/// Identifies engageable replies in the current thread view.
/// Returns a list of reply candidates with metadata and coordinates.
/// Delegates to the unified `js_extract_all_tweets()` and filters for
/// visible replies with like buttons.
#[instrument(skip(api))]
pub async fn identify_thread_replies(api: &TaskContext) -> Result<Vec<serde_json::Value>> {
    let js = js_extract_all_tweets();
    let result = api.page().evaluate(js).await?;
    let value = result
        .value()
        .context("Failed to identify thread replies")?;

    let replies = value
        .as_object()
        .and_then(|obj| obj.get("replies"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    // Filter to visible replies with like buttons (same as old js_identify_thread_replies)
    let engageable: Vec<serde_json::Value> = replies
        .into_iter()
        .filter(|r| {
            r.get("visible").and_then(|v| v.as_bool()).unwrap_or(false)
                && r.get("like_pos").is_some()
                && r.get("like_pos").and_then(|p| p.as_object()).is_some()
        })
        .take(8)
        .collect();

    Ok(engageable)
}

/// Returns the current thread depth (number of visible tweets in thread view).
///
/// Counts the number of tweet elements currently visible in the thread view.
/// Useful for determining how much of the thread has loaded.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns the count of visible tweet elements in the thread view.
#[instrument(skip(api))]
pub async fn get_thread_depth(api: &TaskContext) -> Result<u32> {
    let js = selector_all_tweets();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    let count = value
        .and_then(|v: &serde_json::Value| v.as_array().map(|arr| arr.len() as u32))
        .unwrap_or(0);
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_selectors;

    #[test]
    fn test_status_id_from_relative_url() {
        assert_eq!(
            status_id_from_url("/user/status/12345").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_absolute_url_with_query() {
        assert_eq!(
            status_id_from_url("https://x.com/user/status/12345?lang=en").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_non_status_url() {
        assert_eq!(status_id_from_url("https://x.com/home"), None);
    }

    #[test]
    fn test_status_id_from_url_with_fragment() {
        assert_eq!(
            status_id_from_url("/user/status/12345#reply-1").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_url_with_trailing_slash() {
        assert_eq!(
            status_id_from_url("/user/status/12345/").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_empty_url() {
        assert_eq!(status_id_from_url(""), None);
    }

    #[test]
    fn test_dive_into_thread_outcome_default() {
        let outcome = DiveIntoThreadOutcome::default();
        assert!(!outcome.opened);
        assert!(!outcome.used_fallback_target);
    }

    #[test]
    fn test_status_id_from_numeric_username_path() {
        // Edge case: username is numeric (e.g., user ID-based URL)
        assert_eq!(
            status_id_from_url("/12345/status/67890").as_deref(),
            Some("67890")
        );
    }

    #[test]
    fn test_status_id_from_url_with_special_chars() {
        // Edge case: URL with encoded characters
        assert_eq!(
            status_id_from_url("/user.name/status/12345?source=search").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_url_with_multiple_query_params() {
        assert_eq!(
            status_id_from_url("https://x.com/user/status/12345?lang=en&t=abc123&s=01").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_url_with_www_prefix() {
        // Edge case: www subdomain
        assert_eq!(
            status_id_from_url("https://www.x.com/user/status/12345").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_short_status_url() {
        // Edge case: minimal valid status URL
        assert_eq!(status_id_from_url("/status/1").as_deref(), Some("1"));
    }

    #[test]
    fn test_identify_thread_replies_js_includes_function_wrapper() {
        let js = twitteractivity_selectors::js_extract_all_tweets();
        assert!(js.starts_with("(function()"));
        assert!(js.trim().ends_with("})()"));
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("like_pos"));
        assert!(js.contains("visible"));
    }

    #[test]
    fn test_selector_all_tweets_returns_valid_js() {
        let js = selector_all_tweets();
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("article"));
        assert!(js.contains("data-testid"));
    }
}

#[cfg(test)]
mod tdd_tests {
    use super::*;

    #[test]
    fn tdd_red_dive_status_id_rejects_malformed_urls() {
        assert_eq!(status_id_from_url("not a url"), None);
        assert_eq!(status_id_from_url(""), None);
        assert_eq!(status_id_from_url("/status/"), None);
        assert_eq!(status_id_from_url("//status/"), None);
    }

    #[test]
    fn tdd_red_dive_status_id_accepts_minimal_valid_url() {
        assert_eq!(status_id_from_url("/status/0").as_deref(), Some("0"));
        assert_eq!(status_id_from_url("/status/1").as_deref(), Some("1"));
    }

    #[test]
    fn tdd_green_dive_status_id_with_multiple_slashes_in_path() {
        assert_eq!(
            status_id_from_url("/user/extra/status/12345").as_deref(),
            Some("12345")
        );
    }

    #[test]
    fn tdd_green_dive_outcome_default_clone_fields() {
        let outcome = DiveIntoThreadOutcome::default();
        assert!(!outcome.opened);
        assert!(!outcome.used_fallback_target);
        // Verify we can clone and modify
        let modified = DiveIntoThreadOutcome {
            opened: true,
            ..outcome
        };
        assert!(modified.opened);
        assert!(!modified.used_fallback_target);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Roundtrip: format!("/{user}/status/{id}") → status_id_from_url → Some(id)
        #[test]
        fn proptest_status_id_roundtrip(
            username in "[a-zA-Z0-9._-]{1,20}",
            id in "[a-zA-Z0-9_-]{1,30}",
        ) {
            let url = format!("/{username}/status/{id}");
            let result = status_id_from_url(&url);
            prop_assert_eq!(result.as_deref(), Some(id.as_str()),
                "Failed roundtrip for url={}", url);
        }

        /// status_id_from_url returns None for URLs without /status/.
        #[test]
        fn proptest_status_id_returns_none_for_other_urls(
            path in "[a-zA-Z0-9/_]{1,50}",
        ) {
            prop_assume!(!path.contains("/status/"));
            let result = status_id_from_url(&path);
            prop_assert_eq!(result, None,
                "Expected None for non-status URL: {}", path);
        }

        /// Absolute URLs with query strings still extract the tweet ID.
        #[test]
        fn proptest_status_id_with_query_string(
            username in "[a-zA-Z0-9_]{1,20}",
            id in "[0-9]{1,20}",
            query in "[a-zA-Z0-9=;&-]{0,30}",
        ) {
            let url = if query.is_empty() {
                format!("https://x.com/{username}/status/{id}")
            } else {
                format!("https://x.com/{username}/status/{id}?{query}")
            };
            let result = status_id_from_url(&url);
            prop_assert_eq!(result.as_deref(), Some(id.as_str()),
                "Failed for url={}", url);
        }
    }
}
