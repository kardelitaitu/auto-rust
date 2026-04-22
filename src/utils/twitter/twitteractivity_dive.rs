//! Thread dive and deep-read helpers.
//! Expands tweet threads and scrolls through them for full context.

use crate::prelude::TaskContext;
use anyhow::Result;
use tracing::instrument;

use super::twitteractivity_feed::get_scroll_progress;
use super::{twitteractivity_humanized::*, twitteractivity_selectors::*};

/// Clicks on a tweet to open it in the thread/detail view.
/// Uses selector-based clicking on the tweet text for reliability.
#[instrument(skip(api))]
pub async fn dive_into_thread(api: &TaskContext, _x: f64, _y: f64) -> Result<()> {
    // Find the first visible tweet and click on its text to open thread view
    let js = r#"
        (function() {
            var tweets = document.querySelectorAll('article[data-testid="tweet"]');
            for (var i = 0; i < tweets.length; i++) {
                var el = tweets[i];
                var rect = el.getBoundingClientRect();
                // Find a visible tweet in the viewport
                if (rect.height > 0 && rect.width > 0 && rect.y >= 0 && rect.y < window.innerHeight) {
                    // Click on the tweet text to open thread
                    var textEl = el.querySelector('[data-testid="tweetText"]');
                    if (textEl) {
                        textEl.click();
                        return { success: true };
                    }
                }
            }
            return { success: false };
        })()
    "#;

    let result = api.page().evaluate(js).await?;
    let success = result
        .value()
        .and_then(|v| v.as_object())
        .and_then(|obj| obj.get("success").and_then(|v| v.as_bool()))
        .unwrap_or(false);

    if !success {
        return Ok(()); // No visible tweet found, skip dive
    }

    human_pause(api, 800).await;

    // Wait for thread view to open (look for thread-specific indicators)
    let selectors = [
        "div[data-testid='tweetDetail']",  // Thread detail view
        "div[data-testid='tweetThread']",  // Thread view
        "[aria-label='Timeline: Thread']", // Thread timeline
    ];
    let _ = api
        .wait_for_any_visible_selector(&selectors, 5_000)
        .await
        .ok();

    Ok(())
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
