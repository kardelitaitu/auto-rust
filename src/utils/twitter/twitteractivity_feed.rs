//! Feed and timeline interaction helpers.
//! Scrolling through the home feed, identifying tweets for engagement.

use crate::prelude::TaskContext;
use anyhow::Result;
use serde_json::Value;

use super::{twitteractivity_selectors::*, twitteractivity_humanized::*};

/// Performs a series of scroll actions through the feed.
/// Mimics a user slowly reading their timeline.
///
/// # Arguments
/// * `ctx` - Task context
/// * `scroll_count` - Number of scroll actions to perform
/// * `use_native_scroll` - If true, uses `ctx.scroll_to_bottom()`-style actions; else evaluate JS
pub async fn scroll_feed(
    ctx: &TaskContext,
    scroll_count: u32,
    use_native_scroll: bool,
) -> Result<()> {
    let profile = ctx.behavior_runtime();
    let scroll_amount = profile.scroll.amount;
    let scroll_pause_ms = profile.scroll.pause_ms;
    let smooth = profile.scroll.smooth;

    for i in 0..scroll_count {
        if use_native_scroll {
            // Let TaskContext handle the scroll with profile-derived parameters
            ctx.scroll_read(
                1, // single pause per scroll burst
                scroll_amount,
                smooth,
                profile.scroll.back_scroll,
            )
            .await?;
        } else {
            // JS-based scroll
            let js = format!("window.scrollBy(0, {});", scroll_amount);
            ctx.page().evaluate(js).await?;
            human_pause(ctx, scroll_pause_ms).await;
        }

        // Occasionally scroll back up a little on later iterations
        if smooth && i > 2 && rand::random::<bool>() {
            let back_amount = scroll_amount / 4;
            let js = format!("window.scrollBy(0, -{});", back_amount);
            ctx.page().evaluate(js).await?;
            human_pause(ctx, 200).await;
        }
    }

    Ok(())
}

/// Scans the current viewport for tweet articles that are good engagement candidates.
/// Returns an array of tweet objects with id and position info.
pub async fn identify_engagement_candidates(ctx: &TaskContext) -> Result<Vec<Value>> {
    let js = selector_all_tweets();
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value();

    let mut candidates = Vec::new();

    if let Some(arr) = value.and_then(|v: &serde_json::Value| v.as_array()) {
        for tweet_val in arr {
            if let Some(obj) = tweet_val.as_object() {
                // Basic filter: tweet must have an id and be within viewport reasonably
                if obj.get("id").is_some() {
                    let y = obj.get("y").and_then(|v: &Value| v.as_f64()).unwrap_or(0.0);
                    let height = obj.get("height").and_then(|v: &Value| v.as_f64()).unwrap_or(0.0);
                    // Consider near top half of viewport as "candidate"
                    let viewport = match ctx.viewport().await {
                        Ok(vp) => vp,
                        Err(_) => {
                            // Fallback default viewport if query fails
                            crate::utils::page_size::Viewport { width: 1920.0, height: 1080.0 }
                        }
                    };
                    if y < (viewport.height as f64 * 0.7) && height > 50.0 {
                        candidates.push(tweet_val.clone());
                    }
                }
            }
        }
    }

    Ok(candidates)
}

/// Identifies engagement buttons (like, retweet, reply) for a specific tweet element.
/// Returns a structured object with positions or nulls.
pub async fn get_tweet_engagement_buttons(ctx: &TaskContext) -> Result<Value> {
    let js = selector_engagement_buttons();
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or_default();
    Ok(value)
}

/// Checks if a given tweet (by center coordinates) currently shows "Following" state
/// for the author (used to decide whether a follow action is needed).
pub async fn is_following_user_at_position(ctx: &TaskContext, _x: f64, _y: f64) -> Result<bool> {
    // Move mouse near the tweet to expose any hover-only indicators (optional)
    // For now, evaluate globally
    let js = selector_following_indicator();
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or(Value::Bool(false));
    Ok(value.as_bool().unwrap_or(false))
}

/// Gets the current scroll position as percentage of total page height.
/// Returns 0.0–1.0 (0 = top, 1 = bottom)
pub async fn get_scroll_progress(ctx: &TaskContext) -> Result<f64> {
    let result = ctx
        .page()
        .evaluate("window.scrollY + window.innerHeight >= document.body.scrollHeight ? 1.0 : (window.scrollY / (document.body.scrollHeight - window.innerHeight))".to_string())
        .await?;
    let value = result.value();
    if let Some(v) = value.and_then(|v: &Value| v.as_f64()) {
        Ok(v.clamp(0.0, 1.0))
    } else {
        Ok(0.0)
    }
}

/// Ensures the feed has at least one tweet/article visible.
/// Returns true if feed appears populated.
pub async fn ensure_feed_populated(ctx: &TaskContext) -> Result<bool> {
    let js = selector_all_tweets();
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if let Some(arr) = value.and_then(|v: &serde_json::Value| v.as_array()) {
        return Ok(!arr.is_empty());
    }
    Ok(false)
}

/// Performs a "deep scroll" — scrolls to the bottom of the feed.
/// Used to load more content before engaging.
pub async fn scroll_to_bottom_feed(ctx: &TaskContext) -> Result<()> {
    ctx.scroll_to_bottom().await?;
    // Wait for potential lazy-loaded content
    human_pause(ctx, 2000).await;
    Ok(())
}
