//! Thread dive and deep-read helpers.
//! Expands tweet threads and scrolls through them for full context.

use crate::prelude::TaskContext;
use anyhow::Result;
use std::time::Duration;
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
        return Ok(DiveIntoThreadOutcome {
            opened: false,
            used_fallback_target: false,
        });
    }

    // Find the tweet element by its status URL and scroll it into view
    let js = format!(
        r#"
        (function() {{
            var links = document.querySelectorAll('a[href*="/status/"]');
            for (var i = 0; i < links.length; i++) {{
                var href = links[i].getAttribute('href');
                if (href === '{}' || href.endsWith('{}')) {{
                    // Get coordinates for clicking (skip scrollIntoView to avoid hanging)
                    var rect = links[i].getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {{
                        return {{ 
                            success: true, 
                            x: rect.x + rect.width / 2, 
                            y: rect.y + rect.height / 2 
                        }};
                    }}
                }}
            }}
            return {{ success: false, x: 0, y: 0 }};
        }})()
        "#,
        status_url, status_url
    );

    let result = match tokio::time::timeout(
        Duration::from_secs(5),
        api.page().evaluate(js.as_str())
    ).await {
        Ok(r) => r,
        Err(_) => {
            return Ok(DiveIntoThreadOutcome {
                opened: false,
                used_fallback_target: false,
            });
        }
    }?;
    let (success, click_x, click_y) = if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        let success = obj.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
        let click_x = obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let click_y = obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
        (success, click_x, click_y)
    } else {
        (false, 0.0, 0.0)
    };

    if !success {
        return Ok(DiveIntoThreadOutcome {
            opened: false,
            used_fallback_target: false,
        });
    }

    // No pause needed since we're not scrolling
    // human_pause(api, 500).await;

    // Use Rust API for cursor movement and click (enables cursor overlay tracking)
    api.move_mouse_to(click_x, click_y).await?;
    human_pause(api, 250).await;
    api.click_at(click_x, click_y).await?;
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

    Ok(DiveIntoThreadOutcome {
        opened: true,
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
