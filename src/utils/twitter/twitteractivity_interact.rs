//! Interaction helpers for Twitter/X automation.
//! Like, retweet, follow, and reply operations with human-like timing.

use crate::prelude::TaskContext;
use anyhow::Result;
use log::info;
use rand;
use tracing::instrument;

use super::twitteractivity_humanized::*;

/// Gets the current page URL.
#[instrument(skip(api))]
pub async fn get_current_url(api: &TaskContext) -> Result<String> {
    let js = r#"
        (function() {
            return window.location.href;
        })()
    "#;
    let result = api.page().evaluate(js).await?;
    result
        .value()
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to get current URL"))
}

/// Checks if we're on the home feed.
#[instrument(skip(api))]
pub async fn is_on_home_feed(api: &TaskContext) -> Result<bool> {
    let url = get_current_url(api).await?;
    Ok(url.contains("x.com/home") || url.contains("twitter.com/home"))
}

/// Checks if we're on a tweet detail page.
#[instrument(skip(api))]
pub async fn is_on_tweet_page(api: &TaskContext) -> Result<bool> {
    let url = get_current_url(api).await?;
    Ok(url.contains("/status/") || url.contains("x.com/") && url.contains("/status/"))
}

/// Navigates to tweet by moving mouse to it and clicking.
/// Simplified to avoid scroll-related hanging issues.
#[instrument(skip(api))]
pub async fn navigate_to_tweet(api: &TaskContext, x: f64, y: f64) -> Result<bool> {
    // Just move mouse and click - no scrolling
    // The tweet should already be in viewport from the scan
    api.move_mouse_to(x, y).await?;
    human_pause(api, 200).await;
    api.click_at(x, y).await?;
    human_pause(api, 400).await;

    Ok(true)
}

/// Clicks the "like" (heart) button on the current tweet.
/// Uses api.click() with selector for reliability.
/// Returns true if the click succeeds.
#[instrument(skip(api))]
pub async fn like_tweet(api: &TaskContext) -> Result<bool> {
    use crate::task::twitteractivity::LIKE_BUTTON_SELECTOR;

    // Scroll like button into view before clicking
    if let Err(e) = api.scroll_into_view(LIKE_BUTTON_SELECTOR).await {
        info!("Failed to scroll like button into view: {}", e);
        return Ok(false);
    }
    if let Err(e) = api.click(LIKE_BUTTON_SELECTOR).await {
        info!("Failed to click like button: {}", e);
        return Ok(false);
    }
    info!("Clicked like button");
    human_pause(api, 500).await;
    Ok(true)
}

/// Clicks the "retweet" button on the current tweet.
/// Uses proper filtering then Coords + api.click_at.
/// Note: Does not confirm the retweet in the modal; just opens the retweet menu.
pub async fn click_retweet_button(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var el = buttons[i];
                var testId = (el.getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('retweet') && !testId.includes('unretweet')) {
                    var rect = el.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                    }
                }
            }
            return null;
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            info!("Found retweet button at ({:.1}, {:.1})", x, y);
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            info!("Clicked retweet button, waiting for menu...");
            human_pause(api, 600).await;
            return Ok(true);
        }
    }
    info!("Retweet button not found");
    Ok(false)
}

/// Confirms a retweet from the retweet modal.
pub async fn confirm_retweet(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var btn = document.querySelector('button[data-testid="retweetConfirm"]');
            if (!btn) return null;
            var rect = btn.getBoundingClientRect();
            return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            info!("Found retweet confirm button at ({:.1}, {:.1})", x, y);
            api.move_mouse_to(x, y).await?;
            human_pause(api, 200).await;
            api.click_at(x, y).await?;
            info!("Clicked retweet confirm");
            human_pause(api, 800).await;
            return Ok(true);
        }
    }
    info!("Retweet confirm button not found");
    Ok(false)
}

/// Full retweet action: click retweet then confirm.
#[instrument(skip(api))]
pub async fn retweet_tweet(api: &TaskContext) -> Result<bool> {
    use crate::task::twitteractivity::RETWEET_BUTTON_SELECTOR;
    use crate::task::twitteractivity::RETWEET_CONFIRM_SELECTOR;

    info!("Starting retweet action");
    // Scroll retweet button into view before clicking
    if let Err(e) = api.scroll_into_view(RETWEET_BUTTON_SELECTOR).await {
        info!("Failed to scroll retweet button into view: {}", e);
        return Ok(false);
    }
    // Click retweet button to open menu
    if let Err(e) = api.click(RETWEET_BUTTON_SELECTOR).await {
        info!("Failed to click retweet button: {}", e);
        return Ok(false);
    }
    info!("Clicked retweet button, waiting for menu...");

    // Random pause 1-2s before confirming
    let pause_ms = rand::random::<u64>() % 1000 + 1000; // 1-2s
    human_pause(api, pause_ms).await;

    // Scroll retweet confirm button into view before clicking
    if let Err(e) = api.scroll_into_view(RETWEET_CONFIRM_SELECTOR).await {
        info!("Failed to scroll retweet confirm button into view: {}", e);
        return Ok(false);
    }
    // Click confirm button
    if let Err(e) = api.click(RETWEET_CONFIRM_SELECTOR).await {
        info!("Failed to click retweet confirm: {}", e);
        return Ok(false);
    }
    info!("Retweet confirmed");

    // Wait for action to complete
    human_pause(api, 800).await;
    Ok(true)
}

/// Clicks the "reply" button on the current tweet.
/// Uses proper filtering then Coords + api.click_at.
pub async fn click_reply_button(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid], a[data-testid]');
            for (var i = 0; i < buttons.length; i++) {
                var el = buttons[i];
                var testId = (el.getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('reply') || testId.includes('comment')) {
                    var rect = el.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
                    }
                }
            }
            return null;
        })()
    "#;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            human_pause(api, 500).await;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Types text into the currently focused reply composer and sends it.
pub async fn send_reply(api: &TaskContext, reply_text: &str) -> Result<bool> {
    use std::time::Duration;
    use tokio::time::timeout;

    info!("Starting send_reply with text: '{}'", reply_text);

    // Focus the specific reply textarea
    let textarea_js = r##"(function() { var ta = document.querySelector('[data-testid="tweetTextarea_0"]'); if (ta) { ta.focus(); ta.click(); return { found: true }; } return { found: false }; })()"##;

    info!("Focusing reply textarea");
    match timeout(Duration::from_secs(5), api.page().evaluate(textarea_js)).await {
        Ok(result) => {
            let textarea_result = result?;
            let found = textarea_result
                .value()
                .and_then(|v| v.as_object())
                .and_then(|o| o.get("found"))
                .and_then(|fv| fv.as_bool())
                .unwrap_or(false);

            if !found {
                info!("Reply textarea not found");
                return Ok(false);
            }
        }
        Err(_) => {
            info!("Timeout focusing reply textarea");
            return Ok(false);
        }
    }

    info!("Reply textarea focused");
    human_pause(api, 300).await;

    // Type the reply text
    info!("Typing reply text");
    match timeout(Duration::from_secs(10), api.type_text(reply_text)).await {
        Ok(_) => {}
        Err(_) => {
            info!("Timeout typing reply text");
            return Ok(false);
        }
    }
    human_pause(api, 400).await;

    // Click the Reply button
    let reply_button_js = r#"
        (function() {
            var btn = document.querySelector('[data-testid="tweetButtonInline"]');
            if (btn) {
                var rect = btn.getBoundingClientRect();
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
            return null;
        })()
    "#;

    info!("Finding reply button");
    let button_result = match timeout(Duration::from_secs(5), api.page().evaluate(reply_button_js)).await {
        Ok(result) => result?,
        Err(_) => {
            info!("Timeout finding reply button, using Enter fallback");
            let _ = api.press("Enter").await;
            human_pause(api, 1000).await;
            return Ok(true);
        }
    };

    if let Some(coords) = button_result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            coords.get("x").and_then(|v| v.as_f64()),
            coords.get("y").and_then(|v| v.as_f64()),
        ) {
            info!("Found reply button at ({:.1}, {:.1})", x, y);
            match timeout(Duration::from_secs(5), api.move_mouse_to(x, y)).await {
                Ok(_) => {
                    human_pause(api, 200).await;
                    match timeout(Duration::from_secs(5), api.click_at(x, y)).await {
                        Ok(_) => {
                            info!("Clicked Reply button successfully");
                        }
                        Err(_) => {
                            info!("Timeout clicking reply button, using Enter fallback");
                            let _ = api.press("Enter").await;
                        }
                    }
                }
                Err(_) => {
                    info!("Timeout moving mouse to reply button, using Enter fallback");
                    let _ = api.press("Enter").await;
                }
            }
        } else {
            info!("Reply button coordinates not found, using Enter fallback");
            let _ = api.press("Enter").await;
        }
    } else {
        info!("Reply button not found, using Enter fallback");
        let _ = api.press("Enter").await;
    }

    human_pause(api, 1000).await;
    info!("Reply send completed");
    Ok(true)
}

/// Full reply flow: open composer, type text, send.
#[instrument(skip(api))]
pub async fn reply_to_tweet(api: &TaskContext, reply_text: &str) -> Result<bool> {
    if !click_reply_button(api).await? {
        return Ok(false);
    }
    send_reply(api, reply_text).await
}

/// Clicks the "follow" button after simulating reading replies and scrolling up.
/// Checks for subscribe button first to detect if already following.
#[instrument(skip(api))]
pub async fn follow_from_tweet(api: &TaskContext) -> Result<bool> {
    use crate::task::twitteractivity::FOLLOW_BUTTON_SELECTOR;

    // Simulate reading replies by scrolling down a bit more
    info!("Simulating reading replies before following...");
    api.scroll_read(1, 200, true, false).await?;
    human_pause(api, 2000).await; // Pause to "read"

    // Scroll to top to bring follow button into view
    info!("Scrolling to top to access follow button...");
    api.scroll_to_top().await?;
    human_pause(api, 1000).await;

    // Check if already following (subscribe button present)
    let subscribe_js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid*="-subscribe"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = btn.getAttribute('aria-label') || '';
                if (ariaLabel.includes('Subscribe to @')) {
                    var username = ariaLabel.replace('Subscribe to @', '').split(' ')[0];
                    return username;
                }
            }
            return null;
        })()
    "#;

    let subscribe_result = api.page().evaluate(subscribe_js).await?;
    if let Some(username_val) = subscribe_result.value() {
        if let Some(username) = username_val.as_str() {
            info!("Already following @{}", username);
            return Ok(false); // Not an error, just already following
        }
    }

    // Click the follow button
    if let Err(e) = api.click(FOLLOW_BUTTON_SELECTOR).await {
        info!("Failed to click follow button: {}", e);
        return Ok(false);
    }
    info!("Clicked follow button");

    // Wait for action to complete
    human_pause(api, 1000).await;
    Ok(true)
}

/// Clicks the "bookmark" button on the current tweet.
/// Uses api.click() with selector for reliability.
/// Returns true if the click succeeds.
#[instrument(skip(api))]
pub async fn bookmark_tweet(api: &TaskContext) -> Result<bool> {
    use crate::task::twitteractivity::BOOKMARK_BUTTON_SELECTOR;

    // Scroll bookmark button into view before clicking
    if let Err(e) = api.scroll_into_view(BOOKMARK_BUTTON_SELECTOR).await {
        info!("Failed to scroll bookmark button into view: {}", e);
        return Ok(false);
    }
    if let Err(e) = api.click(BOOKMARK_BUTTON_SELECTOR).await {
        info!("Failed to click bookmark button: {}", e);
        return Ok(false);
    }
    info!("Clicked bookmark button");
    human_pause(api, 500).await;
    Ok(true)
}
