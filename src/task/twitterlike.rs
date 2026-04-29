//! Twitter like task.
//! Likes a tweet with human-like cursor movement and timing.

use crate::prelude::TaskContext;
use crate::utils::math::random_in_range;
use crate::utils::timing::{duration_with_variance, DEFAULT_NAVIGATION_TIMEOUT_MS};
use crate::utils::twitter::{
    close_active_popup, twitteractivity_feed::identify_engagement_candidates,
    twitteractivity_humanized::human_pause, twitteractivity_navigation::goto_home,
};
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;

const POST_WAIT_MS: u64 = 3000;
pub const DEFAULT_TWITTERLIKE_TASK_DURATION_MS: u64 = 30_000;

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_TWITTERLIKE_TASK_DURATION_MS, 20)
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    timeout(Duration::from_millis(duration_ms), run_inner(api, payload))
        .await
        .map_err(|_| anyhow::anyhow!(
            "[twitterlike] Task exceeded duration budget of {}ms",
            duration_ms
        ))?
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    let tweet_url = extract_url_from_payload(&payload)?;
    let max_likes = payload
        .get("max_likes")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;
    let from_feed = payload
        .get("from_feed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    info!("[twitterlike] Task started");
    info!(
        "[twitterlike] Max likes: {}, From feed: {}",
        max_likes, from_feed
    );

    let mut likes_count = 0u32;

    if from_feed || tweet_url.is_empty() {
        // Like from feed
        info!("[twitterlike] Navigating to home feed...");
        goto_home(api).await?;
        api.pause(2000).await;

        // Dismiss popups
        let _ = close_active_popup(api).await;

        // Scan feed for tweets to like
        info!("[twitterlike] Scanning feed for tweets to like...");

        while likes_count < max_likes {
            let candidates = identify_engagement_candidates(api).await?;
            let candidates_vec: Vec<Value> = candidates;

            if candidates_vec.is_empty() {
                warn!("[twitterlike] No tweets found in feed");
                break;
            }

            // Take first candidate
            if let Some(tweet) = candidates_vec.first() {
                info!("[twitterlike] Liking tweet...");
                if tweet.get("x").is_some() && tweet.get("y").is_some() {
                    // Use the like_tweet helper from twitteractivity_interact
                    // For now, we'll click the like button directly
                    let like_js = r#"
                        (function() {
                            var buttons = document.querySelectorAll('[data-testid="like"]');
                            if (buttons.length > 0) {
                                var rect = buttons[0].getBoundingClientRect();
                                if (rect.width > 0 && rect.height > 0) {
                                    buttons[0].click();
                                    return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                                }
                            }
                            return null;
                        })()
                    "#;
                    let result = api.page().evaluate(like_js.to_string()).await?;
                    if result.value().is_some() {
                        info!(
                            "[twitterlike] Liked tweet {}/{}",
                            likes_count + 1,
                            max_likes
                        );
                        likes_count += 1;
                        human_pause(api, random_in_range(1000, 2000)).await;

                        // Scroll a bit to find new tweets
                        api.scroll_to_bottom().await?;
                        api.pause(1000).await;
                        api.scroll_to_top().await?;
                        api.pause(1000).await;
                    } else {
                        warn!("[twitterlike] Failed to click like button");
                        break;
                    }
                } else {
                    warn!("[twitterlike] Tweet has no position data");
                    break;
                }
            }
        }
    } else {
        // Like specific tweet
        info!("[twitterlike] Navigating to tweet: {}", tweet_url);
        api.navigate(&tweet_url, DEFAULT_NAVIGATION_TIMEOUT_MS).await?;
        api.pause(2000).await;

        // Dismiss popups
        let _ = close_active_popup(api).await;

        // Like the tweet up to max_likes times (for testing, usually just 1)
        for i in 0..max_likes {
            info!("[twitterlike] Liking tweet {}/{}", i + 1, max_likes);

            let like_js = r#"
                (function() {
                    var buttons = document.querySelectorAll('[data-testid="like"]');
                    if (buttons.length > 0) {
                        var rect = buttons[0].getBoundingClientRect();
                        if (rect.width > 0 && rect.height > 0) {
                            buttons[0].click();
                            return true;
                        }
                    }
                    return false;
                })()
            "#;
            let result = api.page().evaluate(like_js.to_string()).await?;
            let clicked = result.value().and_then(|v| v.as_bool()).unwrap_or(false);

            if clicked {
                info!("[twitterlike] Like clicked successfully");
                likes_count += 1;

                if i < max_likes - 1 {
                    human_pause(api, random_in_range(2000, 4000)).await;
                }
            } else {
                warn!("[twitterlike] Failed to click like button");
                break;
            }
        }
    }

    api.pause(POST_WAIT_MS).await;
    info!(
        "[twitterlike] Task completed - liked {} tweets",
        likes_count
    );
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
    Ok(String::new()) // Empty string means use feed
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
    fn extract_url_from_payload_empty() {
        let payload = json!({});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, ""); // Empty means use feed
    }

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!(duration_ms >= 24_000 && duration_ms <= 36_000);
    }
}
