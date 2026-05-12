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

use super::twitteractivity_selectors::*;

#[derive(Debug, Clone, Default)]
pub struct DiveIntoThreadOutcome {
    pub opened: bool,
    pub used_fallback_target: bool,
}

fn status_id_from_url(status_url: &str) -> Option<&str> {
    status_url
        .split("/status/")
        .nth(1)
        .and_then(|tail| tail.split(['?', '/', '#']).next())
        .filter(|id| !id.is_empty())
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
    let link_selector = format!("a[href='{}']", status_url);
    info!("Clicking tweet link selector: {}", link_selector);
    if let Err(e) = api.click(&link_selector).await {
        info!("Dive failed: click on link failed: {}", e);
        return Ok(DiveIntoThreadOutcome {
            opened: false,
            used_fallback_target: false,
        });
    }
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
    let url_matches = target_status_id
        .map(|id| current_url.contains(&format!("/status/{id}")))
        .unwrap_or_else(|| current_url.contains(status_url));
    let tweet_article_visible = api
        .wait_for_any_visible_selector(&[r#"article[data-testid="tweet"]"#], 1_000)
        .await
        .unwrap_or(false);
    let thread_opened = url_matches && (detail_visible || tweet_article_visible);

    if thread_opened {
        info!("Thread view opened successfully");
    } else {
        info!(
            "Thread view did not open within timeout or URL mismatch (detail_visible={}, tweet_article_visible={}, url_matches={}, current_url={})",
            detail_visible, tweet_article_visible, url_matches, current_url
        );
    }

    Ok(DiveIntoThreadOutcome {
        opened: thread_opened,
        used_fallback_target: false,
    })
}

/// Identifies engageable replies in the current thread view.
/// Returns a list of reply candidates with metadata and coordinates.
#[instrument(skip(api))]
pub async fn identify_thread_replies(api: &TaskContext) -> Result<Vec<serde_json::Value>> {
    let js = js_identify_thread_replies();
    let result = api.page().evaluate(js).await?;
    let value = result
        .value()
        .context("Failed to identify thread replies")?;

    if let Some(arr) = value.as_array() {
        Ok(arr.clone())
    } else {
        Ok(Vec::new())
    }
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

    #[test]
    fn test_status_id_from_relative_url() {
        assert_eq!(status_id_from_url("/user/status/12345"), Some("12345"));
    }

    #[test]
    fn test_status_id_from_absolute_url_with_query() {
        assert_eq!(
            status_id_from_url("https://x.com/user/status/12345?lang=en"),
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
            status_id_from_url("/user/status/12345#reply-1"),
            Some("12345")
        );
    }

    #[test]
    fn test_status_id_from_url_with_trailing_slash() {
        assert_eq!(status_id_from_url("/user/status/12345/"), Some("12345"));
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
}
