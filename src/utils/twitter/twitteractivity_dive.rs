//! Thread dive and deep-read helpers.
//! Expands tweet threads and scrolls through them for full context.

use crate::prelude::TaskContext;
use anyhow::Result;

use super::twitteractivity_feed::get_scroll_progress;
use super::{twitteractivity_selectors::*, twitteractivity_humanized::*};

/// Clicks on a tweet to open it in the thread/detail view.
/// Some Twitter UIs expand inline; this uses the "Show this thread" approach.
pub async fn dive_into_thread(api: &TaskContext, x: f64, y: f64) -> Result<()> {
    // First move to the location (with human-like motion)
    api.move_mouse_to(x, y).await?;
    human_pause(api, 300).await;

    // Click
    api.click_at(x, y).await?;
    human_pause(api, 800).await;

    // After click, check if a thread modal/overlay opened
    // Wait for thread content to be visible
    let selectors = [
        "article[data-testid='tweet']",
        "div[data-testid='tweetThread']",
        "[aria-label='Thread']",
    ];
    let _ = api
        .wait_for_any_visible_selector(&selectors, 10_000)
        .await
        .ok();

    Ok(())
}

/// Reads the full thread by scrolling through it.
/// Returns after `max_scrolls` iterations or end of thread reached.
pub async fn read_full_thread(
    api: &TaskContext,
    max_scrolls: u32,
) -> Result<()> {
    for i in 0..max_scrolls {
        // Check if we've reached the end
        let progress: f64 = get_scroll_progress(api).await.unwrap_or(0.0);
        if progress >= 0.95 {
            break;
        }

        // Scroll a small amount, mimicking reading
        api.scroll_read(
            1, // single pause
            300, // small incremental scroll
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


