//! Twitter thread dive task.
//! Reads a full tweet thread with human-like timing and scrolling.

use crate::prelude::*;
use crate::utils::math::random_in_range;
use crate::utils::timing::{duration_with_variance, DEFAULT_NAVIGATION_TIMEOUT_MS};
use crate::utils::twitter::twitteractivity_navigation::goto_home;
use anyhow::Result;
use log::info;
use serde_json::Value;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::time::timeout;

const DEFAULT_MAX_SCROLLS: u32 = 5;
pub const DEFAULT_TWITTERDIVE_DURATION_MS: u64 = 60_000;

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_TWITTERDIVE_DURATION_MS, 20)
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    timeout(Duration::from_millis(duration_ms), run_inner(api, payload))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "[twitterdive] Task exceeded duration budget of {}ms",
                duration_ms
            )
        })?
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    let max_scrolls = payload
        .get("max_scrolls")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_MAX_SCROLLS as u64) as u32;
    let duration_ms = payload
        .get("duration_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TWITTERDIVE_DURATION_MS);

    info!("[twitterdive] Task started");
    info!(
        "[twitterdive] Max scrolls: {}, Duration: {}ms",
        max_scrolls, duration_ms
    );

    // Navigate to tweet
    info!("[twitterdive] Navigating to tweet...");
    api.navigate(&tweet_url, DEFAULT_NAVIGATION_TIMEOUT_MS)
        .await?;
    api.pause_human(2000, 20).await;

    // Extract initial tweet info
    let (author, text) = extract_tweet_info(api)
        .await
        .unwrap_or_else(|_| ("unknown".to_string(), "N/A".to_string()));
    info!("[twitterdive] Thread by @{}: {} chars", author, text.len());

    // Read thread with human-like behavior
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let mut scrolls_done = 0u32;
    let mut unique_tweets = HashSet::new();

    info!("[twitterdive] Starting thread read...");

    // Initial reading pause
    api.pause_human(random_in_range(3000, 6000), 20).await;

    while scrolls_done < max_scrolls && Instant::now() < deadline {
        // Track unique tweets visible before scroll
        if let Ok(ids) = extract_visible_tweet_ids(api).await {
            for id in ids {
                unique_tweets.insert(id);
            }
        }

        // Scroll down to reveal more of thread
        info!("[twitterdive] Scroll {}/{}", scrolls_done + 1, max_scrolls);

        let scroll_amount = random_in_range(400, 800) as i32;
        let (_, start_y) = api.get_scroll_position().await.unwrap_or((0, 0));

        // Use human-like scrolling capability
        api.scroll_read(1, scroll_amount, true, true).await?;

        scrolls_done += 1;

        // Reading pause after scroll
        let read_pause = random_in_range(2000, 5000);
        api.pause_human(read_pause, 30).await;

        // Verify if scroll position changed (End-of-thread detection)
        let (_, end_y) = api.get_scroll_position().await.unwrap_or((0, 0));
        let at_bottom = end_y <= start_y; // Didn't move down

        // Check for "Show more" or end indicators
        let at_end = check_end_of_thread(api).await?;
        if at_bottom && at_end {
            info!("[twitterdive] Reached true end of thread (no further scroll possible)");
            break;
        }

        // Occasional micro-scroll back up (human behavior)
        if random_in_range(0, 100) < 20 {
            let back_scroll = random_in_range(100, 300) as i32;
            api.scroll_read(1, -back_scroll, true, false).await?;
            api.pause_human(1000, 20).await;
        }
    }

    // Final capture of visible tweets
    if let Ok(ids) = extract_visible_tweet_ids(api).await {
        for id in ids {
            unique_tweets.insert(id);
        }
    }

    // Final summary
    info!("[twitterdive] Thread reading complete");
    info!(
        "[twitterdive] Scrolls: {}, Unique tweets read: {}, Duration: {:.1}s",
        scrolls_done,
        unique_tweets.len(),
        (Instant::now() - deadline).as_secs_f64() + (duration_ms as f64 / 1000.0)
    );

    // Navigate back to home
    info!("[twitterdive] Returning to home feed...");
    goto_home(api).await?;
    api.pause_human(2000, 20).await;

    info!("[twitterdive] Task completed");
    Ok(())
}

fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(value) = payload.get("url") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }
    if let Some(value) = payload.get("value") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }
    // Check for default_url in payload
    if let Some(default_url) = payload.get("default_url") {
        if let Some(url_str) = default_url.as_str() {
            return Ok(url_str.to_string());
        }
    }
    for (key, val) in payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("payload not an object"))?
    {
        if key != "url" && key != "value" && key != "default_url" {
            if let Some(v) = val.as_str() {
                if !v.is_empty() && v.contains("x.com") {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err(anyhow::anyhow!("No URL found in payload"))
}

async fn extract_tweet_info(api: &TaskContext) -> Result<(String, String)> {
    let page = api.page();

    let result = page
        .evaluate(
            r#"
        (function() {
            var tweet = document.querySelector('article[data-testid="tweet"]');
            if (!tweet) return null;

            var nameLink = tweet.querySelector('a[href^="/"]');
            var username = nameLink ? nameLink.getAttribute('href').split('/')[1] : 'unknown';
            var textContent = tweet.querySelectorAll('[data-testid="tweetText"]');
            var text = textContent.length > 0 ? textContent[textContent.length - 1].innerText : '';

            return { username: username, text: text };
        })()
    "#,
        )
        .await?;

    let value = result.value();
    if let Some(v) = value {
        if let Some(obj) = v.as_object() {
            let username = obj
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("");
            return Ok((username.to_string(), text.to_string()));
        }
    }

    Err(anyhow::anyhow!("Could not extract tweet info"))
}

async fn extract_visible_tweet_ids(api: &TaskContext) -> Result<Vec<String>> {
    let page = api.page();
    let js = r#"
        (function() {
            var articles = document.querySelectorAll('article[data-testid="tweet"]');
            var ids = [];
            for (var i = 0; i < articles.length; i++) {
                var el = articles[i];
                var id = el.dataset.tweetId || 
                         el.getAttribute('data-item-id') || 
                         el.getAttribute('data-tweet-id');
                
                if (!id) {
                    var statusLink = el.querySelector('a[href*="/status/"]');
                    if (statusLink) {
                        var parts = statusLink.getAttribute('href').split('/');
                        id = parts[parts.length - 1].split('?')[0];
                    }
                }
                
                if (id) ids.push(id);
            }
            return ids;
        })()
    "#;

    let result = page.evaluate(js).await?;
    let value = result.value().and_then(|v| v.as_array());
    
    if let Some(arr) = value {
        Ok(arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
    } else {
        Ok(Vec::new())
    }
}

async fn check_end_of_thread(api: &TaskContext) -> Result<bool> {
    let page = api.page();

    let result = page
        .evaluate(
            r#"
        (function() {
            // Check for "Show more" or end indicators
            var showMore = document.querySelector('[data-testid="showMoreThread"]');
            
            // On Twitter, if we reach the end, there is often a specific marker or 
            // simply the absence of further articles being lazily loaded.
            return !showMore;
        })()
    "#,
        )
        .await?;

    let at_end = result.value().and_then(|v| v.as_bool()).unwrap_or(false);
    Ok(at_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_url_from_payload_url() {
        let payload = json!({"url": "https://x.com/user/status/123"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_url_from_payload_value() {
        let payload = json!({"value": "https://x.com/user/status/456"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert!(result.contains("x.com"));
    }

    #[test]
    fn extract_url_missing() {
        let payload = json!({});
        assert!(extract_url_from_payload(&payload).is_err());
    }

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!((48_000..=72_000).contains(&duration_ms));
    }
}
