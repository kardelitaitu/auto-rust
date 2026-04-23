//! Thread dive and deep-read helpers.
//! Expands tweet threads and scrolls through them for full context.

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
    pub cache: Option<ThreadCache>,
}

/// Clicks on a tweet to open it in the thread/detail view.
/// Uses status URL to find the element, then clicks with cursor overlay tracking.
#[instrument(skip(api))]
pub async fn dive_into_thread(api: &TaskContext, status_url: &str) -> Result<DiveIntoThreadOutcome> {
    if status_url.is_empty() {
        info!("Dive skipped: empty status_url");
        return Ok(DiveIntoThreadOutcome {
            opened: false,
            used_fallback_target: false,
            cache: None,
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
            cache: None,
        });
    }
    info!("Clicked tweet link, waiting for thread view...");

    // Wait for thread/modal view to open (tweet detail or thread)
    let selectors = [
        r#"div[role="dialog"]"#,                    // Modal dialog (common for tweet details)
        r#"div[data-testid="tweetDetail"]"#,        // Thread detail view
        r#"div[data-testid="tweetThread"]"#,        // Thread view
        r#"[aria-label="Timeline: Thread"]"#,       // Thread timeline
        r#"article[data-testid="tweet"]"#,          // Tweet in detail view
    ];
    let thread_opened = api
        .wait_for_any_visible_selector(&selectors, 5_000)
        .await
        .unwrap_or(false);

    if thread_opened {
        info!("Thread view opened successfully");
    } else {
        info!("Thread view did not open within timeout");
    }

    let cache = if thread_opened {
        match extract_initial_thread_data(api).await {
            Ok(c) => Some(c),
            Err(e) => {
                info!("Failed to extract initial thread data: {}", e);
                None
            }
        }
    } else {
        None
    };

    Ok(DiveIntoThreadOutcome {
        opened: thread_opened,
        used_fallback_target: false,
        cache,
    })
}

/// Reads the full thread by scrolling through it.
/// Returns after `max_scrolls` iterations or end of thread reached.
/// Incrementally extracts replies into the provided cache.
#[instrument(skip(api, cache))]
pub async fn read_full_thread(api: &TaskContext, max_scrolls: u32, cache: &mut ThreadCache) -> Result<()> {
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
                        info!("Extracted {} new replies (total: {})", count, cache.replies.len());
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

    info!("Thread reading complete. Cached {} replies", cache.replies.len());
    Ok(())
}

/// Returns the current thread depth (number of visible tweets in thread view).
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
/// Called immediately after thread opens to capture the root tweet.
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
    let value = result.value().context("Failed to extract initial thread data")?;

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

        info!("Extracted initial thread data: author='{}', text_len={}", author, text.len());
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
/// Returns the number of new replies extracted.
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
    let value = result.value().context("Failed to extract visible replies")?;

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
                let already_cached = cache.replies.iter().any(|(a, t)| a == &author && t == &text);
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
