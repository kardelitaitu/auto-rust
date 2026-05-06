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

use super::twitteractivity_feed::get_scroll_progress;
use super::{twitteractivity_humanized::*, twitteractivity_selectors::*};

/// Cached thread data for LLM processing.
/// Contains tweet author, text, and up to 20 replies collected during thread reading.
#[derive(Debug, Clone, Default)]
pub struct ThreadCache {
    pub tweet_author: String,
    pub tweet_text: String,
    pub replies: Vec<(String, String)>, // (author, text)
}

impl ThreadCache {
    /// Add a reply to the cache, up to 20 replies maximum.
    pub fn add_reply(&mut self, author: String, text: String) {
        if self.replies.len() < 20 {
            self.replies.push((author, text));
        }
    }

    /// Check if cache has valid data.
    pub fn is_valid(&self) -> bool {
        !self.tweet_author.is_empty() || !self.tweet_text.is_empty()
    }
}

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

/// Reads the full thread by scrolling through it with incremental reply extraction.
///
/// This function simulates human-like reading of a Twitter thread by scrolling
/// incrementally and extracting replies visible at each scroll position. Replies
/// are added to the provided cache for later LLM processing.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
/// * `max_scrolls` - Maximum number of scroll iterations to perform
/// * `cache` - Mutable thread cache to populate with extracted replies
///
/// # Returns
///
/// Returns `Ok(())` on completion. Errors are logged but don't stop execution.
///
/// # Behavior
///
/// - Extracts visible replies before each scroll
/// - Scrolls in small increments (300px) to simulate reading
/// - Adds human-like pauses between scrolls (500-1500ms)
/// - Stops early if scroll progress reaches 95% (end of thread)
/// - Limits cached replies to 20 total
/// - Every 3rd scroll includes a longer pause (1500ms) to simulate reading
///
/// # Scroll Progress
///
/// The function monitors scroll progress and stops when:
/// - `max_scrolls` iterations completed
/// - Scroll progress reaches 95% (end of thread)
///
/// # Example
///
/// ```rust,no_run
/// use auto::utils::twitter::twitteractivity_dive::*;
/// # use auto::runtime::task_context::TaskContext;
/// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
///
/// let mut cache = ThreadCache::default();
/// read_full_thread(api, 10, &mut cache).await?;
/// # Ok(())
/// # }
/// ```
#[instrument(skip(api, cache))]
pub async fn read_full_thread(
    api: &TaskContext,
    max_scrolls: u32,
    cache: &mut ThreadCache,
) -> Result<()> {
    for i in 0..max_scrolls {
        // Check if we've reached the end
        let progress: f64 = get_scroll_progress(api).await.unwrap_or_else(|e| {
            log::warn!("Failed to get scroll progress, defaulting to 0: {}", e);
            0.0
        });
        if progress >= 0.95 {
            break;
        }

        // Extract replies before scrolling (capture what's currently visible)
        if cache.replies.len() < 20 {
            match extract_visible_replies(api, cache).await {
                Ok(count) => {
                    if count > 0 {
                        info!(
                            "Extracted {} new replies (total: {})",
                            count,
                            cache.replies.len()
                        );
                    }
                }
                Err(e) => {
                    log::debug!("Failed to extract replies on scroll {}: {}", i, e);
                }
            }
        }

        // Scroll a small amount, mimicking reading
        api.scroll_read(
            1,    // single pause
            300,  // small incremental scroll
            true, // smooth
            false,
        )
        .await?;

        human_pause(api, 500).await;

        // Occasionally pause longer to "read"
        if i % 3 == 2 {
            human_pause(api, 1500).await;
        }
    }

    info!(
        "Thread reading complete. Cached {} replies",
        cache.replies.len()
    );
    Ok(())
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

/// Extracts initial tweet data (author and text) from the current thread view.
///
/// This function extracts the root tweet's author and text immediately after a thread
/// opens. It uses DOM queries to find the tweet elements and returns a ThreadCache
/// with the extracted data.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `ThreadCache` containing:
/// - `tweet_author`: The username of the tweet author
/// - `tweet_text`: The text content of the tweet
/// - `replies`: Empty vector (replies are added incrementally during scrolling)
///
/// # Errors
///
/// Returns error if DOM evaluation fails or data extraction fails.
///
/// # Behavior
///
/// - Queries for the first tweet in thread view
/// - Extracts author from `[data-testid="tweet"] [dir="auto"]`
/// - Extracts text from `[data-testid="tweetText"]`
/// - Returns "unknown" for author if not found
/// - Returns empty string for text if not found
///
/// # Selectors Used
///
/// - Author: `[data-testid="tweet"] [dir="auto"]`
/// - Text: `[data-testid="tweetText"]`
#[instrument(skip(api))]
pub async fn extract_initial_thread_data(api: &TaskContext) -> Result<ThreadCache> {
    let js = r#"
        (function() {
            // Extract tweet author from the first tweet in thread
            var authorEl = document.querySelector('[data-testid="tweet"] [dir="auto"]');
            var author = authorEl ? authorEl.textContent.trim() : 'unknown';
            
            // Extract tweet text
            var tweetEl = document.querySelector('[data-testid="tweetText"]');
            var text = tweetEl ? tweetEl.textContent.trim() : '';
            
            return {
                author: author,
                text: text
            };
        })()
    "#;

    let result = api.page().evaluate(js).await?;
    let value = result
        .value()
        .context("Failed to extract initial thread data")?;

    if let Some(obj) = value.as_object() {
        let author = obj
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let text = obj
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        info!(
            "Extracted initial thread data: author='{}', text_len={}",
            author,
            text.len()
        );
        Ok(ThreadCache {
            tweet_author: author,
            tweet_text: text,
            replies: Vec::new(),
        })
    } else {
        anyhow::bail!("Invalid initial thread data format")
    }
}

/// Extracts visible replies from the current thread view and adds them to the cache.
///
/// This function queries the DOM for all reply tweets in the current view and
/// extracts their author and text. Replies are deduplicated and added to the
/// provided cache up to a maximum of 20 total replies.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
/// * `cache` - Mutable thread cache to populate with extracted replies
///
/// # Returns
///
/// Returns the number of new replies added to the cache.
///
/// # Errors
///
/// Returns error if DOM evaluation fails or data extraction fails.
///
/// # Behavior
///
/// - Queries for all tweet elements in thread view
/// - Skips the first element (root tweet)
/// - Extracts author and text from each reply
/// - Deduplicates replies by author+text combination
/// - Limits total cached replies to 20
/// - Only adds replies with non-empty text
///
/// # Selectors Used
///
/// - Tweets: `article[data-testid="tweet"]`
/// - Author: `[dir="auto"]` within tweet
/// - Text: `[data-testid="tweetText"]` within tweet
#[instrument(skip(api, cache))]
pub async fn extract_visible_replies(api: &TaskContext, cache: &mut ThreadCache) -> Result<u32> {
    let js = r#"
        (function() {
            // Extract all reply tweets (skip the first one which is the root tweet)
            var replyEls = document.querySelectorAll('article[data-testid="tweet"]');
            var replies = [];
            
            for (var i = 1; i < replyEls.length; i++) {
                var tweetEl = replyEls[i];
                
                // Extract author
                var authorEl = tweetEl.querySelector('[dir="auto"]');
                var author = authorEl ? authorEl.textContent.trim() : 'unknown';
                
                // Extract text
                var textEl = tweetEl.querySelector('[data-testid="tweetText"]');
                var text = textEl ? textEl.textContent.trim() : '';
                
                if (text && text.length > 0) {
                    replies.push({ author: author, text: text });
                }
            }
            
            return replies;
        })()
    "#;

    let result = api.page().evaluate(js).await?;
    let value = result
        .value()
        .context("Failed to extract visible replies")?;

    if let Some(arr) = value.as_array() {
        let mut new_count = 0;
        for item in arr {
            if let Some(obj) = item.as_object() {
                let author = obj
                    .get("author")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let text = obj
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Only add if not already in cache (simple deduplication)
                let already_cached = cache
                    .replies
                    .iter()
                    .any(|(a, t)| a == &author && t == &text);
                if !already_cached && !text.is_empty() {
                    cache.add_reply(author, text);
                    new_count += 1;
                }
            }
        }
        Ok(new_count)
    } else {
        anyhow::bail!("Invalid replies data format")
    }
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
    fn test_thread_cache_default() {
        let cache = ThreadCache::default();
        assert!(cache.tweet_author.is_empty());
        assert!(cache.tweet_text.is_empty());
        assert!(cache.replies.is_empty());
    }

    #[test]
    fn test_thread_cache_add_reply() {
        let mut cache = ThreadCache::default();
        cache.add_reply("user1".to_string(), "reply1".to_string());
        assert_eq!(cache.replies.len(), 1);
        assert_eq!(
            cache.replies[0],
            ("user1".to_string(), "reply1".to_string())
        );
    }

    #[test]
    fn test_thread_cache_add_reply_limit() {
        let mut cache = ThreadCache::default();
        for i in 0..25 {
            cache.add_reply(format!("user{}", i), format!("reply{}", i));
        }
        assert_eq!(cache.replies.len(), 20);
    }

    #[test]
    fn test_thread_cache_is_valid_empty() {
        let cache = ThreadCache::default();
        assert!(!cache.is_valid());
    }

    #[test]
    fn test_thread_cache_is_valid_with_author() {
        let cache = ThreadCache {
            tweet_author: "testuser".to_string(),
            ..Default::default()
        };
        assert!(cache.is_valid());
    }

    #[test]
    fn test_thread_cache_is_valid_with_text() {
        let cache = ThreadCache {
            tweet_text: "test tweet".to_string(),
            ..Default::default()
        };
        assert!(cache.is_valid());
    }

    #[test]
    fn test_dive_into_thread_outcome_default() {
        let outcome = DiveIntoThreadOutcome::default();
        assert!(!outcome.opened);
        assert!(!outcome.used_fallback_target);
    }
}
