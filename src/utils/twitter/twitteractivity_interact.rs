//! Interaction helpers for Twitter/X automation.
//!
//! This module provides functions for performing common Twitter engagement actions
//! including liking, retweeting, following, replying, and bookmarking tweets. All
//! interactions use human-like timing and cursor movements to avoid detection.
//!
//! ## Key Components
//!
//! - **Engagement Actions**: Like, retweet, follow, reply, bookmark
//! - **Human-like Timing**: Randomized pauses and cursor movements
//! - **Reply/Quote**: Compose and send replies with text input
//!
//! ## Key Functions
//!
//! - [`click_like_button()`]: Like a tweet
//! - [`click_retweet_button()`]: Open retweet menu
//! - [`confirm_retweet()`]: Confirm retweet from modal
//! - [`retweet_tweet()`]: Complete retweet action
//! - [`follow_from_tweet()`]: Follow a tweet author
//! - [`reply_to_tweet()`]: Reply to a tweet
//! - [`quote_tweet()`]: Quote a tweet (in twitteractivity_llm module)
//! - [`bookmark_tweet()`]: Bookmark a tweet
//!
//! ## Usage
//!
//! ```rust,no_run
//! use rust_orchestrator::utils::twitter::twitteractivity_interact::*;
//!
//! // Like a tweet
//! click_like_button(api).await?;
//!
//! // Retweet with confirmation
//! retweet_tweet(api).await?;
//!
//! // Reply to a tweet
//! reply_to_tweet(api, "Great point!").await?;
//! ```
//!
//! ## Timing and Humanization
//!
//! All functions use randomized pauses to simulate human behavior:
//! - 200-500ms pauses before/after clicks
//! - 1-2s pauses for confirmation actions
//! - Random variation in timing to avoid patterns

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
///
/// This function scrolls the like button into view and clicks it to like a tweet.
/// It uses the selector-based approach for reliability and adds a human-like pause
/// after the interaction.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if the like button was clicked successfully.
/// Returns `Ok(false)` if scrolling or clicking fails.
///
/// # Errors
///
/// Returns error if the scroll or click operation fails unexpectedly.
///
/// # Behavior
///
/// - Scrolls the like button into view
/// - Clicks the button using selector
/// - Adds 500ms human-like pause after clicking
///
/// # Selector Used
///
/// - Like button: `LIKE_BUTTON_SELECTOR` (defined in twitteractivity.rs)
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

/// Clicks the "retweet" button on the current tweet to open the retweet menu.
///
/// This function finds the retweet button by filtering for elements with
/// data-testid containing "retweet" (but not "unretweet"), then clicks it.
/// It does not confirm the retweet - that's handled by `confirm_retweet()`.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if the retweet button was clicked successfully.
/// Returns `Ok(false)` if the button is not found or click fails.
///
/// # Errors
///
/// Returns error if the DOM evaluation or click operation fails unexpectedly.
///
/// # Behavior
///
/// - Searches for buttons with data-testid containing "retweet"
/// - Excludes buttons containing "unretweet" (already retweeted)
/// - Validates button has visible dimensions
/// - Moves mouse to button and clicks
/// - Adds 250ms pause before click, 600ms after click
///
/// # Selector Strategy
///
/// Uses broad search: `button[data-testid], a[data-testid]`
/// Filters for: data-testid includes "retweet" but not "unretweet"
#[instrument(skip(api))]
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
///
/// This function clicks the "Retweet" confirm button in the modal that appears
/// after clicking the retweet button. It scrolls the button into view first for
/// reliability.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if the confirm button was clicked successfully.
/// Returns `Ok(false)` if the button is not found or click fails.
///
/// # Errors
///
/// Returns error if the DOM evaluation or click operation fails unexpectedly.
///
/// # Behavior
///
/// - Finds the retweet confirm button by data-testid
/// - Scrolls the button into view
/// - Moves mouse to button and clicks
/// - Adds 200ms pause before click, 800ms after click
///
/// # Selector Used
///
/// - Confirm button: `button[data-testid="retweetConfirm"]`
#[instrument(skip(api))]
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

/// Full retweet action: click retweet button then confirm in modal.
///
/// This is a convenience function that combines clicking the retweet button and
/// confirming the retweet in the modal. It handles scrolling and timing for both steps.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if both steps (click and confirm) succeed.
/// Returns `Ok(false)` if either step fails.
///
/// # Errors
///
/// Returns error if the scroll or click operations fail unexpectedly.
///
/// # Behavior
///
/// - Scrolls retweet button into view and clicks
/// - Waits 1-2s (randomized) before confirming
/// - Scrolls confirm button into view and clicks
/// - Waits 800ms after confirmation
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

/// Clicks the "reply" button on the current tweet to open the reply composer.
///
/// This function finds the reply button by filtering for elements with data-testid
/// containing "reply" or "comment", then clicks it to open the reply composer.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if the reply button was clicked successfully.
/// Returns `Ok(false)` if the button is not found or click fails.
///
/// # Errors
///
/// Returns error if the DOM evaluation or click operation fails unexpectedly.
///
/// # Behavior
///
/// - Searches for buttons with data-testid containing "reply" or "comment"
/// - Validates button has visible dimensions
/// - Moves mouse to button and clicks
/// - Adds 250ms pause before click, 500ms after click
///
/// # Selector Strategy
///
/// Uses broad search: `button[data-testid], a[data-testid]`
/// Filters for: data-testid includes "reply" or "comment"
#[instrument(skip(api))]
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
///
/// This function focuses the reply textarea, types the provided text, and clicks
/// the reply button to send. All operations have timeouts to prevent hanging.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
/// * `reply_text` - The text to type into the reply composer
///
/// # Returns
///
/// Returns `Ok(true)` if the reply was sent successfully.
/// Returns `Ok(false)` if any step (focus, type, or send) fails.
///
/// # Errors
///
/// Returns error if the operations fail unexpectedly.
///
/// # Behavior
///
/// - Focuses the reply textarea with 5s timeout
/// - Types the reply text with 10s timeout
/// - Finds and clicks the reply button with 5s timeout
/// - Adds human-like pauses (300-400ms) between steps
///
/// # Selectors Used
///
/// - Textarea: `[data-testid="tweetTextarea_0"]`
/// - Reply button: `[data-testid="tweetButtonInline"]`
///
/// # Timeouts
///
/// - Focus: 5 seconds
/// - Typing: 10 seconds
/// - Button find: 5 seconds
/// - Mouse move: 5 seconds
/// - Button click: 5 seconds
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
            info!("Timeout finding reply button");
            return Ok(false);
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
                            info!("Timeout clicking reply button");
                            return Ok(false);
                        }
                    }
                }
                Err(_) => {
                    info!("Timeout moving mouse to reply button");
                    return Ok(false);
                }
            }
        } else {
            info!("Reply button coordinates not found");
            return Ok(false);
        }
    } else {
        info!("Reply button not found");
        return Ok(false);
    }

    human_pause(api, 1000).await;
    info!("Reply send completed");
    Ok(true)
}

/// Full reply flow: open composer, type text, send.
///
/// This is a convenience function that combines clicking the reply button and
/// sending the reply text. It handles the complete reply interaction.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
/// * `reply_text` - The text to send as a reply
///
/// # Returns
///
/// Returns `Ok(true)` if the reply was sent successfully.
/// Returns `Ok(false)` if either step (click reply or send) fails.
///
/// # Errors
///
/// Returns error if the operations fail unexpectedly.
///
/// # Behavior
///
/// - Clicks the reply button to open composer
/// - Types the reply text into the textarea
/// - Clicks the send button to post the reply
#[instrument(skip(api))]
pub async fn reply_to_tweet(api: &TaskContext, reply_text: &str) -> Result<bool> {
    if !click_reply_button(api).await? {
        return Ok(false);
    }
    send_reply(api, reply_text).await
}

/// Clicks the "follow" button after simulating reading replies and scrolling up.
///
/// This function simulates human-like behavior by scrolling down to read replies,
/// then scrolling back up to access the follow button. It checks if the user is
/// already following by looking for a subscribe button.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if the follow button was clicked successfully.
/// Returns `Ok(false)` if already following or button not found/click fails.
///
/// # Errors
///
/// Returns error if the scroll or click operations fail unexpectedly.
///
/// # Behavior
///
/// - Scrolls down 200px to simulate reading replies
/// - Pauses 2s to "read"
/// - Scrolls to top to access follow button
/// - Checks for subscribe button (indicates already following)
/// - Clicks follow button if not already following
/// - Waits 1s after clicking
///
/// # Selectors Used
///
/// - Follow button: `FOLLOW_BUTTON_SELECTOR` (defined in twitteractivity.rs)
/// - Subscribe check: `button[data-testid*="-subscribe"]` with aria-label
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
///
/// This function scrolls the bookmark button into view and clicks it to bookmark
/// a tweet. It uses the selector-based approach for reliability.
///
/// # Arguments
///
/// * `api` - Task context with page and browser automation capabilities
///
/// # Returns
///
/// Returns `Ok(true)` if the bookmark button was clicked successfully.
/// Returns `Ok(false)` if scrolling or clicking fails.
///
/// # Errors
///
/// Returns error if the scroll or click operation fails unexpectedly.
///
/// # Behavior
///
/// - Scrolls the bookmark button into view
/// - Clicks the button using selector
/// - Adds 500ms human-like pause after clicking
///
/// # Selector Used
///
/// - Bookmark button: `BOOKMARK_BUTTON_SELECTOR` (defined in twitteractivity.rs)
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
