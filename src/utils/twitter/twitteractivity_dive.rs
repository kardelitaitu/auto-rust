//! Thread dive and deep-read helpers.
//! Expands tweet threads and scrolls through them for full context.

use crate::prelude::TaskContext;
use anyhow::Result;
use log::info;
use tracing::instrument;

use super::twitteractivity_feed::get_scroll_progress;
use super::{twitteractivity_humanized::*, twitteractivity_selectors::*};

#[derive(Debug, Clone, Copy, Default)]
pub struct DiveIntoThreadOutcome {
    pub opened: bool,
    pub used_fallback_target: bool,
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

    Ok(DiveIntoThreadOutcome {
        opened: thread_opened,
        used_fallback_target: false,
    })
}

/// Reads the full thread by scrolling through it.
/// Returns after `max_scrolls` iterations or end of thread reached.
#[instrument(skip(api))]
pub async fn read_full_thread(api: &TaskContext, max_scrolls: u32) -> Result<()> {
    for i in 0..max_scrolls {
        // Check if we've reached the end
        let progress: f64 = get_scroll_progress(api).await.unwrap_or_else(|e| {
            log::warn!("Failed to get scroll progress, defaulting to 0: {}", e);
            0.0
        });
        if progress >= 0.95 {
            break;
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
