//! Feed and timeline interaction helpers.
//! Scrolling through the home feed, identifying tweets for engagement.

use crate::prelude::TaskContext;
use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use super::{twitteractivity_humanized::*, twitteractivity_selectors::*};

/// Performs a series of scroll actions through the feed.
/// Mimics a user slowly reading their timeline.
///
/// # Arguments
/// * `api` - Task context
/// * `scroll_count` - Number of scroll actions to perform
/// * `use_native_scroll` - If true, uses `api.scroll_to_bottom()`-style actions; else evaluate JS
#[instrument(skip(api))]
pub async fn scroll_feed(
    api: &TaskContext,
    scroll_count: u32,
    use_native_scroll: bool,
) -> Result<()> {
    let profile = api.behavior_runtime();
    let scroll_amount = profile.scroll.amount;
    let scroll_pause_ms = profile.scroll.pause_ms;
    let smooth = profile.scroll.smooth;

    for i in 0..scroll_count {
        if use_native_scroll {
            // Let TaskContext handle the scroll with profile-derived parameters
            api.scroll_read(
                1, // single pause per scroll burst
                scroll_amount,
                smooth,
                profile.scroll.back_scroll,
            )
            .await?;
        } else {
            // JS-based scroll
            let js = format!("window.scrollBy(0, {});", scroll_amount);
            api.page().evaluate(js).await?;
            human_pause(api, scroll_pause_ms).await;
        }

        // Occasionally scroll back up a little on later iterations
        if smooth && i > 2 && rand::random::<bool>() {
            let back_amount = scroll_amount / 4;
            let js = format!("window.scrollBy(0, -{});", back_amount);
            api.page().evaluate(js).await?;
            human_pause(api, 200).await;
        }
    }

    Ok(())
}

/// Scans the current viewport for tweet articles that are good engagement candidates.
/// Returns an array of tweet objects with id, position info, text content, and reply data.
pub async fn identify_engagement_candidates(api: &TaskContext) -> Result<Vec<Value>> {
    let js = r#"
        (function() {
            var tweets = [];
            var elements = document.querySelectorAll('article[data-testid="tweet"]');
            for (var i = 0; i < elements.length; i++) {
                var el = elements[i];
                var rect = el.getBoundingClientRect();
                if (rect.height > 0 && rect.width > 0) {
                    // Extract tweet text content
                    var tweetTextEl = el.querySelector('[data-testid="tweetText"]');
                    var tweetText = tweetTextEl ? tweetTextEl.textContent.trim() : '';
                    
                    // Extract basic tweet info
                    // Twitter may not have a tweet ID attribute, so generate one from position
                    var tweetId = el.dataset.tweetId || 
                                  el.getAttribute('data-item-id') || 
                                  el.getAttribute('data-tweet-id') || 
                                  'tweet_' + Math.floor(rect.x) + '_' + Math.floor(rect.y);
                    
                    // Find engagement buttons within this tweet element
                    var likeBtn = el.querySelector('[data-testid="like"]');
                    var retweetBtn = el.querySelector('[data-testid="retweet"]');
                    var replyBtn = el.querySelector('[data-testid="reply"]');
                    
                    var buttonPositions = {};
                    if (likeBtn) {
                        var likeRect = likeBtn.getBoundingClientRect();
                        if (likeRect.width > 0 && likeRect.height > 0) {
                            buttonPositions.like = { x: likeRect.x + likeRect.width/2, y: likeRect.y + likeRect.height/2 };
                        }
                    }
                    if (retweetBtn) {
                        var retweetRect = retweetBtn.getBoundingClientRect();
                        if (retweetRect.width > 0 && retweetRect.height > 0) {
                            buttonPositions.retweet = { x: retweetRect.x + retweetRect.width/2, y: retweetRect.y + retweetRect.height/2 };
                        }
                    }
                    if (replyBtn) {
                        var replyRect = replyBtn.getBoundingClientRect();
                        if (replyRect.width > 0 && replyRect.height > 0) {
                            buttonPositions.reply = { x: replyRect.x + replyRect.width/2, y: replyRect.y + replyRect.height/2 };
                        }
                    }
                    
                    // Extract the status URL from the time element for reliable diving
                    var timeEl = el.querySelector('a[href*="/status/"]');
                    var statusUrl = timeEl ? timeEl.getAttribute('href') : null;
                    
                    var tweetObj = {
                        id: tweetId,
                        status_url: statusUrl,
                        index: i,
                        text: tweetText,
                        x: rect.x + rect.width/2,
                        y: rect.y + rect.height/2,
                        height: rect.height,
                        width: rect.width,
                        buttons: buttonPositions
                    };

                    // Extract reply information for smart decision
                    var replies = [];
                    var replyElements = el.querySelectorAll('[data-testid="tweetReply"]');
                    for (var j = 0; j < Math.min(replyElements.length, 3); j++) { // Limit to top 3 replies
                        var replyEl = replyElements[j];
                        var authorEl = replyEl.querySelector('[dir="auto"] span:first-child');
                        var textEl = replyEl.querySelector('[data-testid="tweetText"]');
                        if (authorEl && textEl) {
                            replies.push({
                                author: authorEl.textContent.trim(),
                                text: textEl.textContent.trim()
                            });
                        }
                    }

                    if (replies.length > 0) {
                        tweetObj.replies = replies;
                    }

                    tweets.push(tweetObj);
                }
            }
            return tweets;
        })()
    "#;

    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();

    let mut candidates = Vec::new();
    let mut total_found = 0;
    let mut filtered_no_id = 0;
    let mut filtered_viewport = 0;
    let mut filtered_height = 0;
    let viewport = match api.viewport().await {
        Ok(vp) => vp,
        Err(_) => {
            // Fallback default viewport if query fails
            crate::utils::page_size::Viewport {
                width: 1920.0,
                height: 1080.0,
            }
        }
    };

    if let Some(arr) = value.and_then(|v: &serde_json::Value| v.as_array()) {
        total_found = arr.len();
        for tweet_val in arr {
            if let Some(obj) = tweet_val.as_object() {
                // Basic filter: tweet must have an id and be within viewport reasonably
                let id = obj.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if id.is_empty() {
                    filtered_no_id += 1;
                    continue;
                }

                let y = obj.get("y").and_then(|v: &Value| v.as_f64()).unwrap_or(0.0);
                let height = obj
                    .get("height")
                    .and_then(|v: &Value| v.as_f64())
                    .unwrap_or(0.0);

                // Filter out tweets above viewport (negative y) or too small
                if y < 0.0 || height <= 50.0 {
                    if y < 0.0 {
                        filtered_viewport += 1;
                    } else {
                        filtered_height += 1;
                    }
                    continue;
                }

                // Consider tweets in viewport as "candidate" (relaxed from 70% to 90%)
                if y >= (viewport.height as f64 * 0.9) {
                    filtered_viewport += 1;
                    continue;
                }

                candidates.push(tweet_val.clone());
            }
        }
    }

    if total_found == 0 {
        log::warn!("[candidate_scan] No tweet elements found in DOM");
    } else if candidates.is_empty() {
        log::warn!(
            "[candidate_scan] Found {} tweets but filtered: no_id={}, viewport={}, height={}",
            total_found,
            filtered_no_id,
            filtered_viewport,
            filtered_height
        );
    }

    Ok(candidates)
}

/// Identifies engagement buttons (like, retweet, reply) for a specific tweet element.
/// Returns a structured object with positions or nulls.
pub async fn get_tweet_engagement_buttons(api: &TaskContext) -> Result<Value> {
    let js = selector_engagement_buttons();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or_default();
    Ok(value)
}

/// Checks if a given tweet (by center coordinates) currently shows "Following" state
/// for the author (used to decide whether a follow action is needed).
pub async fn is_following_user_at_position(api: &TaskContext, _x: f64, _y: f64) -> Result<bool> {
    // Move mouse near the tweet to expose any hover-only indicators (optional)
    // For now, evaluate globally
    let js = selector_following_indicator();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or(Value::Bool(false));
    Ok(value.as_bool().unwrap_or(false))
}

/// Gets the current scroll position as percentage of total page height.
/// Returns 0.0–1.0 (0 = top, 1 = bottom)
pub async fn get_scroll_progress(api: &TaskContext) -> Result<f64> {
    let result = api
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
pub async fn ensure_feed_populated(api: &TaskContext) -> Result<bool> {
    let js = selector_all_tweets();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if let Some(arr) = value.and_then(|v: &serde_json::Value| v.as_array()) {
        return Ok(!arr.is_empty());
    }
    Ok(false)
}

/// Performs a "deep scroll" — scrolls to the bottom of the feed.
/// Used to load more content before engaging.
pub async fn scroll_to_bottom_feed(api: &TaskContext) -> Result<()> {
    api.scroll_to_bottom().await?;
    // Wait for potential lazy-loaded content
    human_pause(api, 2000).await;
    Ok(())
}
