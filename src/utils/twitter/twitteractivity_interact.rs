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
//! use auto::utils::twitter::twitteractivity_interact::*;
//! # use auto::runtime::task_context::TaskContext;
//! # async fn example(api: &TaskContext) -> anyhow::Result<()> {
//!
//! // Like a tweet
//! like_tweet(api).await?;
//!
//! // Retweet with confirmation
//! retweet_tweet(api).await?;
//!
//! // Reply to a tweet
//! reply_to_tweet(api, "Great point!").await?;
//! # Ok(())
//! # }
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
use super::twitteractivity_selectors::selector_follow_button;

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

fn root_tweet_button_center_js(selector: &str) -> Result<String> {
    let selector_json = serde_json::to_string(selector)?;
    Ok(format!(
        r#"
        (function() {{
            var selector = {selector_json};
            function visible(el) {{
                if (!el) return false;
                var rect = el.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0;
            }}
            function center(el) {{
                var rect = el.getBoundingClientRect();
                return {{ x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 }};
            }}

            var articles = Array.prototype.slice.call(
                document.querySelectorAll('article[data-testid="tweet"]')
            ).filter(visible);
            var statusMatch = window.location.pathname.match(/\/status\/(\d+)/);
            var targetStatusId = statusMatch ? statusMatch[1] : null;
            var targetArticle = null;
            if (targetStatusId) {{
                for (var i = 0; i < articles.length; i++) {{
                    if (articles[i].querySelector('a[href*="/status/' + targetStatusId + '"]')) {{
                        targetArticle = articles[i];
                        break;
                    }}
                }}
            }}
            var scopes = articles.length > 0
                ? [targetArticle || articles[0]]
                : [document.querySelector('main'), document.body].filter(Boolean);

            for (var i = 0; i < scopes.length; i++) {{
                var button = scopes[i].querySelector(selector);
                if (visible(button)) return center(button);
            }}
            return null;
        }})()
        "#
    ))
}

async fn click_root_tweet_button(
    api: &TaskContext,
    selector: &str,
    action_name: &str,
) -> Result<bool> {
    let js = root_tweet_button_center_js(selector)?;
    let result = api.page().evaluate(js).await?;

    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            info!(
                "Found root tweet {} button at ({:.1}, {:.1})",
                action_name, x, y
            );
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            human_pause(api, 500).await;
            return Ok(true);
        }
    }

    info!("Root tweet {} button not found", action_name);
    Ok(false)
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

    if click_root_tweet_button(api, LIKE_BUTTON_SELECTOR, "like").await? {
        info!("Clicked root tweet like button");
        return Ok(true);
    }

    Ok(false)
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
    use crate::task::twitteractivity::RETWEET_BUTTON_SELECTOR;

    if click_root_tweet_button(api, RETWEET_BUTTON_SELECTOR, "retweet").await? {
        return Ok(true);
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
    if !click_root_tweet_button(api, RETWEET_BUTTON_SELECTOR, "retweet").await? {
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
    if click_root_tweet_button(api, r#"button[data-testid="reply"]"#, "reply").await? {
        return Ok(true);
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

    // Focus the specific reply textarea.
    let textarea_js = r##"
        (function() {
            var textboxes = document.querySelectorAll('[data-testid="tweetTextarea_0"][role="textbox"], [data-testid="tweetTextarea_0"]');
            for (var i = 0; i < textboxes.length; i++) {
                var ta = textboxes[i];
                var rect = ta.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) continue;
                ta.focus();
                ta.click();
                return { found: true };
            }
            return { found: false };
        })()
    "##;

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

    // Click the Reply submit button in the composer.
    let reply_button_js = r#"
        (function() {
            var buttons = document.querySelectorAll('button[data-testid="tweetButtonInline"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var rect = btn.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) continue;
                if (btn.disabled || btn.getAttribute('aria-disabled') === 'true') continue;
                var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
                if (text !== 'reply') continue;
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
            return null;
        })()
    "#;

    info!("Finding reply button");
    let button_result =
        match timeout(Duration::from_secs(5), api.page().evaluate(reply_button_js)).await {
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

    // Verify reply was sent by checking if textarea is cleared or composer closed
    let verify_js = r#"
        (function() {
            const textarea = document.querySelector('[data-testid="tweetTextarea_0"]');
            if (!textarea) return { sent: true, reason: "composer closed" }; // Composer closed, likely sent
            const text = textarea.textContent || textarea.value || '';
            if (text.trim() === '') return { sent: true, reason: "textarea cleared" }; // Text cleared, likely sent
            return { sent: false, reason: "textarea still has text" };
        })()
    "#;

    let verify_result = api.page().evaluate(verify_js).await?;
    if let Some(obj) = verify_result.value().and_then(|v| v.as_object()) {
        if let Some(sent) = obj.get("sent").and_then(|v| v.as_bool()) {
            if sent {
                info!("Reply send completed and verified");
                Ok(true)
            } else {
                let reason = obj
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                info!("Reply send verification failed: {}", reason);
                Ok(false)
            }
        } else {
            info!("Reply send completed (unable to verify)");
            Ok(false)
        }
    } else {
        info!("Reply send completed (verification failed)");
        Ok(false)
    }
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
    // Simulate reading replies by scrolling down a bit more
    info!("Simulating reading replies before following...");
    api.scroll_read(1, 200, true, false).await?;
    human_pause(api, 2000).await; // Pause to "read"

    // Scroll to top to bring follow button into view
    info!("Scrolling to top to access follow button...");
    api.scroll_to_top().await?;
    human_pause(api, 1000).await;

    // Check if already following.
    let following_js = r#"
        (function() {
            var buttons = document.querySelectorAll('button, [role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var text = (btn.textContent || btn.innerText || '').trim().toLowerCase();
                var label = (btn.getAttribute('aria-label') || '').toLowerCase();
                var dataTestId = (btn.getAttribute('data-testid') || '').toLowerCase();
                if (text === 'following' ||
                    label.includes('following @') ||
                    label.includes('unfollow @') ||
                    dataTestId.includes('unfollow')) {
                    return true;
                }
            }
            return false;
        })()
    "#;

    let following_result = api.page().evaluate(following_js).await?;
    if following_result
        .value()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        info!("Already following tweet author");
        return Ok(false);
    }

    let follow_result = api.page().evaluate(selector_follow_button()).await?;
    if let Some(obj) = follow_result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            info!("Found scoped follow button at ({:.1}, {:.1})", x, y);
            api.move_mouse_to(x, y).await?;
            human_pause(api, 250).await;
            api.click_at(x, y).await?;
            human_pause(api, 1000).await;
            return Ok(true);
        }
    }

    info!("Scoped follow button not found");
    Ok(false)
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

    if click_root_tweet_button(api, BOOKMARK_BUTTON_SELECTOR, "bookmark").await? {
        info!("Clicked root tweet bookmark button");
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_tweet_button_center_js_scopes_to_first_visible_tweet() {
        let js = root_tweet_button_center_js(r#"button[data-testid="reply"]"#).unwrap();

        assert!(js.contains("article[data-testid=\"tweet\"]"));
        assert!(js.contains("targetStatusId"));
        assert!(js.contains("articles[0]"));
        assert!(js.contains("querySelector(selector)"));
        assert!(js.contains(r#"button[data-testid=\"reply\"]"#));
    }

    #[test]
    fn test_root_tweet_button_center_js_includes_visibility_check() {
        let js = root_tweet_button_center_js(r#"button[data-testid="like"]"#).unwrap();
        assert!(js.contains("visible(el)"));
        assert!(js.contains("getBoundingClientRect"));
    }

    #[test]
    fn test_root_tweet_button_center_js_includes_center_function() {
        let js = root_tweet_button_center_js(r#"button[data-testid="retweet"]"#).unwrap();
        assert!(js.contains("function center(el)"));
        assert!(js.contains("rect.x + rect.width / 2"));
        assert!(js.contains("rect.y + rect.height / 2"));
    }

    #[test]
    fn test_root_tweet_button_center_js_handles_status_id_extraction() {
        let js = root_tweet_button_center_js(r#"button[data-testid="bookmark"]"#).unwrap();
        assert!(js.contains("window.location.pathname"));
        assert!(js.contains("/status/"));
    }

    #[test]
    fn test_root_tweet_button_center_js_escapes_selector_json() {
        let js = root_tweet_button_center_js(r#"button[data-testid="test\"quote"]"#).unwrap();
        assert!(js.contains("\\\""));
    }

    #[test]
    fn test_root_tweet_button_center_js_with_complex_selector() {
        let js = root_tweet_button_center_js(r#"[data-testid="tweet"] button[aria-label="Like"]"#)
            .unwrap();
        assert!(js.contains("data-testid"));
        assert!(js.contains("aria-label"));
    }

    #[test]
    fn test_root_tweet_button_center_js_returns_null_on_failure() {
        let js = root_tweet_button_center_js(r#"button[data-testid="test"]"#).unwrap();
        assert!(js.contains("return null"));
    }

    #[test]
    fn test_root_tweet_button_center_js_filters_visible_elements() {
        let js = root_tweet_button_center_js(r#"button[data-testid="follow"]"#).unwrap();
        assert!(js.contains(".filter(visible)"));
    }

    #[test]
    fn test_root_tweet_button_center_js_scopes_to_main_or_body() {
        let js = root_tweet_button_center_js(r#"button[data-testid="reply"]"#).unwrap();
        assert!(js.contains("document.querySelector('main')"));
        assert!(js.contains("document.body"));
    }
}
