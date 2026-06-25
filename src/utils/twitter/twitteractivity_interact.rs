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
//! - `click_like_button()`: Like a tweet
//! - `click_retweet_button()`: Open retweet menu
//! - `retweet_tweet()`: Complete retweet action
//! - `follow_from_tweet()`: Follow a tweet author
//! - `reply_to_tweet()`: Reply to a tweet
//! - `quote_tweet()`: Quote a tweet (in `twitteractivity_llm` module)
//! - `bookmark_tweet()`: Bookmark a tweet
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
use crate::utils::timing::TIMEOUT_SHORT_SECS;
use anyhow::Result;
use log::{info, warn};
use rand::Rng;
use tracing::instrument;

use super::state::{parse_button_coordinates, parse_following_result, parse_reply_verification};
use super::twitteractivity_humanized::human_pause;
use super::twitteractivity_selectors::{
    js_find_reply_submit_button, js_find_reply_textarea, js_root_tweet_button_center,
    selector_follow_button, REPLY_BUTTON_SELECTOR,
};
use super::{EngagementOutcome, FollowOutcome};

// =========================================================================
// Pure functions extracted from async interaction helpers for testability
// =========================================================================

/// Checks if a URL indicates we're on the home feed.
/// Pure function — no browser required.
#[must_use]
pub fn is_home_feed_url(url: &str) -> bool {
    url.contains("x.com/home") || url.contains("twitter.com/home")
}

/// Checks if a URL indicates we're on a tweet detail page.
/// Only checks the URL path for `/status/`.
/// Pure function — no browser required.
#[must_use]
pub fn is_tweet_page_url(url: &str) -> bool {
    url.contains("/status/")
}

// =========================================================================
// Async interaction helpers
// =========================================================================

/// Gets the current page URL.
#[instrument(skip(api))]
pub async fn get_current_url(api: &TaskContext) -> Result<String> {
    let js = r"
        (function() {
            return window.location.href;
        })()
    ";
    let result = api.page().evaluate(js).await?;
    result
        .value()
        .and_then(|v| v.as_str())
        .map(std::string::ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("Failed to get current URL"))
}

/// Checks if we're on the home feed.
#[instrument(skip(api))]
pub async fn is_on_home_feed(api: &TaskContext) -> Result<bool> {
    let url = get_current_url(api).await?;
    Ok(is_home_feed_url(&url))
}

/// Checks if we're on a tweet detail page.
#[instrument(skip(api))]
pub async fn is_on_tweet_page(api: &TaskContext) -> Result<bool> {
    let url = get_current_url(api).await?;
    if is_tweet_page_url(&url) {
        return Ok(true);
    }

    // Check for tweet detail modal visibility
    let js = r#"
        (function() {
            const modal = document.querySelector('div[role="dialog"]');
            if (modal) {
                const rect = modal.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) return true;
            }
            const detail = document.querySelector('div[data-testid="tweetDetail"]');
            if (detail) {
                const rect = detail.getBoundingClientRect();
                if (rect.width > 0 && rect.height > 0) return true;
            }
            return false;
        })()
    "#;

    let result = api.page().evaluate(js).await?;
    Ok(result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

fn root_tweet_button_center_js(selector: &str) -> String {
    js_root_tweet_button_center(selector)
}

async fn click_root_tweet_button(
    api: &TaskContext,
    selector: &str,
    action_name: &str,
) -> Result<EngagementOutcome> {
    let js = root_tweet_button_center_js(selector);
    let result = api.page().evaluate(js).await?;

    if let Some((x, y)) = result.value().and_then(parse_button_coordinates) {
        info!("[{action_name}] Found root tweet {action_name} button at ({x:.1}, {y:.1})");
        api.move_mouse_to(x, y).await?;
        human_pause(api, 250).await;
        api.click_at(x, y).await?;
        human_pause(api, 500).await;
        return Ok(EngagementOutcome::Completed);
    }

    info!("[{action_name}] Root tweet {action_name} button not found");
    Ok(EngagementOutcome::ElementNotFound)
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
pub async fn like_tweet(api: &TaskContext) -> Result<EngagementOutcome> {
    use super::twitteractivity_selectors::LIKE_BUTTON_SELECTOR;

    let outcome = click_root_tweet_button(api, LIKE_BUTTON_SELECTOR, "like").await?;
    if matches!(outcome, EngagementOutcome::Completed) {
        info!("[like] Clicked root tweet like button");
    }
    Ok(outcome)
}

/// Clicks the "retweet" button on the current tweet to open the retweet menu.
///
/// This function finds the retweet button by filtering for elements with
/// data-testid containing "retweet" (but not "unretweet"), then clicks it.
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
pub async fn click_retweet_button(api: &TaskContext) -> Result<EngagementOutcome> {
    use super::twitteractivity_selectors::RETWEET_BUTTON_SELECTOR;

    click_root_tweet_button(api, RETWEET_BUTTON_SELECTOR, "retweet").await
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
pub async fn retweet_tweet(api: &TaskContext) -> Result<EngagementOutcome> {
    use super::twitteractivity_selectors::RETWEET_BUTTON_SELECTOR;
    use super::twitteractivity_selectors::RETWEET_CONFIRM_SELECTOR;

    info!("[retweet] Starting retweet action");
    if click_root_tweet_button(api, RETWEET_BUTTON_SELECTOR, "retweet").await?
        != EngagementOutcome::Completed
    {
        return Ok(EngagementOutcome::ElementNotFound);
    }
    info!("[retweet] Clicked retweet button, waiting for menu...");

    // Random pause 1-2s before confirming
    let pause_ms = rand::thread_rng().gen_range(1000..2000);
    human_pause(api, pause_ms).await;

    // Find retweet confirm button coordinates
    let confirm_btn_js = super::twitteractivity_selectors::js_find_retweet_confirm_button();
    let result = api.page().evaluate(confirm_btn_js).await?;

    if let Some((x, y)) = result.value().and_then(parse_button_coordinates) {
        info!("[retweet] Found retweet confirm button at ({x:.1}, {y:.1})");
        api.move_mouse_to(x, y).await?;
        human_pause(api, 250).await;
        api.click_at(x, y).await?;
        info!("[retweet] Retweet confirmed");
        human_pause(api, 800).await;
        return Ok(EngagementOutcome::Completed);
    }

    // Fallback: direct click using selector
    warn!("[retweet] Coordinate search failed, attempting fallback selector click...");
    if let Err(e) = api.scroll_into_view(RETWEET_CONFIRM_SELECTOR).await {
        info!("[retweet] Failed to scroll retweet confirm button into view: {e}");
        return Ok(EngagementOutcome::Failed);
    }
    if let Err(e) = api.click(RETWEET_CONFIRM_SELECTOR).await {
        info!("[retweet] Failed to click retweet confirm: {e}");
        return Ok(EngagementOutcome::Failed);
    }
    info!("[retweet] Retweet confirmed via fallback");
    human_pause(api, 800).await;
    Ok(EngagementOutcome::Completed)
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
pub async fn click_reply_button(api: &TaskContext) -> Result<EngagementOutcome> {
    click_root_tweet_button(api, REPLY_BUTTON_SELECTOR, "reply").await
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
pub async fn send_reply(api: &TaskContext, reply_text: &str) -> Result<EngagementOutcome> {
    use std::time::Duration;
    use tokio::time::timeout;
    // Timeout constants imported from crate::utils::timing

    info!("[reply] Starting send_reply with text: '{reply_text}'");

    // Focus the specific reply textarea.
    let textarea_js = js_find_reply_textarea();

    info!("[reply] Focusing reply textarea");
    if let Ok(result) = timeout(
        Duration::from_secs(TIMEOUT_SHORT_SECS),
        api.page().evaluate(textarea_js),
    )
    .await
    {
        let textarea_result = result?;
        let found = textarea_result
            .value()
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("found"))
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if !found {
            info!("[reply] Reply textarea not found");
            return Ok(EngagementOutcome::ElementNotFound);
        }
    } else {
        info!("[reply] Timeout focusing reply textarea");
        return Ok(EngagementOutcome::Failed);
    }

    info!("[reply] Reply textarea focused");
    // Wait 2-3s before typing to let React render the composer and position cursor
    let pause_ms = rand::thread_rng().gen_range(2000..3001);
    human_pause(api, pause_ms).await;

    // Type the reply text with natural typing (includes typos and corrections)
    info!("[reply] Typing reply text (with typos)");
    let typing = api.behavior_runtime().typing;
    // Calculate dynamic timeout: allow up to 1.5 seconds per character for natural typing, plus base medium timeout
    let typing_timeout_secs =
        crate::utils::timing::TIMEOUT_MEDIUM_SECS + (reply_text.len() as u64 * 1500 / 1000);
    match timeout(
        Duration::from_secs(typing_timeout_secs),
        crate::utils::keyboard::natural_typing_profiled(
            api.page(),
            "[data-testid=\"tweetTextarea_0\"]",
            reply_text,
            &typing,
        ),
    )
    .await
    {
        Ok(Err(e)) => {
            info!("[reply] Typing failed due to error: {e}");
            return Ok(EngagementOutcome::Failed);
        }
        Err(_) => {
            info!("[reply] Timeout typing reply text");
            return Ok(EngagementOutcome::Failed);
        }
        Ok(Ok(())) => {}
    }
    // Verify typing completed by reading back the textarea content.
    // This waits for React to process all queued InputEvents before we
    // attempt to find the submit button (which is disabled while empty).
    info!("[reply] Verifying typed content...");
    {
        let check_js = r#"
            (function() {
                const el = document.querySelector('[data-testid="tweetTextarea_0"]');
                if (!el) return '';
                return el.innerText || el.textContent || '';
            })()
        "#;
        let mut received = String::new();
        for attempt in 1..=10 {
            if let Ok(Ok(val)) = timeout(
                Duration::from_secs(TIMEOUT_SHORT_SECS),
                api.page().evaluate(check_js),
            )
            .await
            {
                if let Some(text) = val.value().and_then(|v| v.as_str()) {
                    let normalized_received: String =
                        text.chars().filter(|c| !c.is_whitespace()).collect();
                    let normalized_expected: String =
                        reply_text.chars().filter(|c| !c.is_whitespace()).collect();

                    received = text.trim().to_string();
                    if !normalized_received.is_empty()
                        && normalized_expected.contains(&normalized_received)
                    {
                        info!(
                            "[reply] Typing verified (attempt {}/10): {} chars received",
                            attempt,
                            received.len()
                        );
                        break;
                    }
                }
            }
            info!(
                "[reply] Content not ready yet (attempt {}/10), waiting...",
                attempt
            );
            human_pause(api, 500).await;
        }
        if received.is_empty() {
            info!(
                "[reply] Typing verification: no content detected after 10 attempts, proceeding anyway..."
            );
        }
    }

    // 1. Scroll the reply submit button into view so its coordinates are stable.
    let scroll_js = r#"(function() {
             const btn = document.querySelector('button[data-testid="tweetButtonInline"], button[data-testid="tweetButton"]');
             if (btn) btn.scrollIntoView({ block: 'center', behavior: 'instant' });
         })()"#
        .to_string();
    let _ = api.page().evaluate(scroll_js).await;
    human_pause(api, 300).await;

    // 2. Click the Reply submit button by evaluating its actual post-scroll coordinates.
    // Retry up to 3 times with short pauses to allow React to process the input
    // and enable the button (it starts disabled when composer is empty).
    let reply_button_js = js_find_reply_submit_button();

    let button_coords = {
        let mut coords = None;
        for attempt in 1..=3 {
            info!("[reply] Finding reply button (attempt {}/3)", attempt);
            let button_result = match timeout(
                Duration::from_secs(TIMEOUT_SHORT_SECS),
                api.page().evaluate(reply_button_js),
            )
            .await
            {
                Ok(r) => r?,
                Err(_) => {
                    info!(
                        "[reply] Timeout finding reply button on attempt {}",
                        attempt
                    );
                    human_pause(api, 300).await;
                    continue;
                }
            };

            if let Some((x, y)) = button_result.value().and_then(parse_button_coordinates) {
                coords = Some((x, y));
                break;
            }
            info!(
                "[reply] Reply button not ready on attempt {}, retrying...",
                attempt
            );
            human_pause(api, 300).await;
        }
        coords
    };

    if let Some((x, y)) = button_coords {
        info!("[reply] Found reply button at ({x:.1}, {y:.1})");

        if timeout(
            Duration::from_secs(TIMEOUT_SHORT_SECS),
            api.move_mouse_to(x, y),
        )
        .await
        .is_ok()
        {
            human_pause(api, 200).await;
            if timeout(Duration::from_secs(TIMEOUT_SHORT_SECS), api.click_at(x, y))
                .await
                .is_ok()
            {
                info!("[reply] Clicked Reply button successfully");
            } else {
                info!("[reply] Timeout clicking reply button");
                return Ok(EngagementOutcome::Failed);
            }
        } else {
            info!("[reply] Timeout moving mouse to reply button");
            return Ok(EngagementOutcome::Failed);
        }
    } else {
        info!("[reply] Reply button not found after 3 attempts");
        return Ok(EngagementOutcome::Failed);
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
    let outcome = verify_result
        .value()
        .map(parse_reply_verification)
        .unwrap_or(EngagementOutcome::Unverified);
    match outcome {
        EngagementOutcome::Completed => {
            info!("[reply] Reply send completed and verified");
        }
        EngagementOutcome::Failed => {
            info!("[reply] Reply send verification failed");
        }
        EngagementOutcome::Unverified => {
            info!("[reply] Reply send completed (unable to verify)");
        }
        _ => {}
    }
    Ok(outcome)
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
pub async fn reply_to_tweet(api: &TaskContext, reply_text: &str) -> Result<EngagementOutcome> {
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
pub async fn follow_from_tweet(api: &TaskContext) -> Result<FollowOutcome> {
    // Simulate reading replies by scrolling down a bit more
    info!("[follow] Simulating reading replies before following...");
    api.scroll_read(1, 200, true, false).await?;
    human_pause(api, 2000).await; // Pause to "read"

    // Scroll to top to bring follow button into view
    info!("[follow] Scrolling to top to access follow button...");
    api.scroll_to_top().await?;
    human_pause(api, 1000).await;

    // Check if already following (scoped to the tweet detail article).
    let following_js = r#"
        (function() {
            // Scope to the first visible tweet article on the detail page
            var container = document.querySelector('article[data-testid="tweet"]');
            if (!container) return false;
            var buttons = container.querySelectorAll('button, [role="button"]');
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
        .map(parse_following_result)
        .unwrap_or(false)
    {
        info!("[follow] Already following tweet author");
        return Ok(FollowOutcome::AlreadyFollowing);
    }

    let follow_result = api.page().evaluate(selector_follow_button()).await?;
    if let Some((x, y)) = follow_result.value().and_then(parse_button_coordinates) {
        info!("[follow] Found scoped follow button at ({x:.1}, {y:.1})");
        api.move_mouse_to(x, y).await?;
        human_pause(api, 250).await;
        api.click_at(x, y).await?;
        human_pause(api, 1000).await;
        return Ok(FollowOutcome::Followed);
    }

    info!("[follow] Scoped follow button not found");
    Ok(FollowOutcome::ButtonNotFound)
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
pub async fn bookmark_tweet(api: &TaskContext) -> Result<EngagementOutcome> {
    use super::twitteractivity_selectors::BOOKMARK_BUTTON_SELECTOR;

    let outcome = click_root_tweet_button(api, BOOKMARK_BUTTON_SELECTOR, "bookmark").await?;
    if matches!(outcome, EngagementOutcome::Completed) {
        info!("[bookmark] Clicked root tweet bookmark button");
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_tweet_button_center_js_scopes_to_first_visible_tweet() {
        let js = root_tweet_button_center_js(r#"button[data-testid="reply"]"#);

        assert!(js.contains("article[data-testid=\"tweet\"]"));
        assert!(js.contains("targetStatusId"));
        assert!(js.contains("visibleArticles[0]"));
        assert!(js.contains(r#"button[data-testid=\"reply\"]"#));
    }

    #[test]
    fn test_root_tweet_button_center_js_includes_visibility_check() {
        let js = root_tweet_button_center_js(r#"button[data-testid="like"]"#);
        assert!(js.contains("visible(el)"));
        assert!(js.contains("getBoundingClientRect"));
    }

    #[test]
    fn test_root_tweet_button_center_js_includes_center_function() {
        let js = root_tweet_button_center_js(r#"button[data-testid="retweet"]"#);
        assert!(js.contains("function center(el)"));
        assert!(js.contains("rect.x + rect.width / 2"));
        assert!(js.contains("rect.y + rect.height / 2"));
    }

    #[test]
    fn test_root_tweet_button_center_js_handles_status_id_extraction() {
        let js = root_tweet_button_center_js(r#"button[data-testid="bookmark"]"#);
        assert!(js.contains("window.location.pathname"));
        assert!(js.contains("/status/"));
    }

    #[test]
    fn test_root_tweet_button_center_js_escapes_selector_json() {
        let js = root_tweet_button_center_js(r#"button[data-testid="test\"quote"]"#);
        assert!(js.contains("\\\""));
    }

    #[test]
    fn test_root_tweet_button_center_js_with_complex_selector() {
        let js = root_tweet_button_center_js(r#"[data-testid="tweet"] button[aria-label="Like"]"#);
        assert!(js.contains("data-testid"));
        assert!(js.contains("aria-label"));
    }

    #[test]
    fn test_root_tweet_button_center_js_returns_null_on_failure() {
        let js = root_tweet_button_center_js(r#"button[data-testid="test"]"#);
        assert!(js.contains("return null"));
    }

    #[test]
    fn test_root_tweet_button_center_js_filters_visible_elements() {
        let js = root_tweet_button_center_js(r#"button[data-testid="follow"]"#);
        assert!(js.contains(".filter(visible)"));
    }

    #[test]
    fn test_root_tweet_button_center_js_scopes_to_main_or_body() {
        let js = root_tweet_button_center_js(r#"button[data-testid="reply"]"#);
        assert!(js.contains("document.querySelector('main')"));
        assert!(js.contains("document.body"));
    }
}

#[cfg(test)]
mod pure_function_tests {
    use super::*;
    use serde_json::json;

    // ====================================================================
    // is_home_feed_url
    // ====================================================================

    #[test]
    fn home_feed_url_xcom() {
        assert!(is_home_feed_url("https://x.com/home"));
    }

    #[test]
    fn home_feed_url_twittercom() {
        assert!(is_home_feed_url("https://twitter.com/home"));
    }

    #[test]
    fn home_feed_url_with_query() {
        assert!(is_home_feed_url("https://x.com/home?t=123&ref=src"));
    }

    #[test]
    fn home_feed_url_tweet_page_is_not_home() {
        assert!(!is_home_feed_url("https://x.com/user/status/12345"));
    }

    #[test]
    fn home_feed_url_explore_is_not_home() {
        assert!(!is_home_feed_url("https://x.com/explore"));
    }

    #[test]
    fn home_feed_url_empty_string() {
        assert!(!is_home_feed_url(""));
    }

    #[test]
    fn home_feed_url_case_sensitive() {
        // URLs are case-sensitive, /Home is different from /home
        assert!(!is_home_feed_url("https://x.com/Home"));
    }

    // ====================================================================
    // is_tweet_page_url
    // ====================================================================

    #[test]
    fn tweet_page_url_xcom_status() {
        assert!(is_tweet_page_url("https://x.com/user/status/123456789"));
    }

    #[test]
    fn tweet_page_url_twittercom_status() {
        assert!(is_tweet_page_url(
            "https://twitter.com/user/status/123456789"
        ));
    }

    #[test]
    fn tweet_page_url_with_query() {
        assert!(is_tweet_page_url("https://x.com/user/status/12345?t=abc"));
    }

    #[test]
    fn tweet_page_url_home_is_not_tweet() {
        assert!(!is_tweet_page_url("https://x.com/home"));
    }

    #[test]
    fn tweet_page_url_explore_is_not_tweet() {
        assert!(!is_tweet_page_url("https://x.com/explore"));
    }

    #[test]
    fn tweet_page_url_empty_string() {
        assert!(!is_tweet_page_url(""));
    }

    #[test]
    fn tweet_page_url_path_contains_status_with_trailing_segment() {
        // URL with /status/ prefix but no numeric ID — still matches on /status/
        assert!(is_tweet_page_url("https://x.com/i/status/"));
    }

    // ====================================================================
    // parse_button_coordinates
    // ====================================================================

    #[test]
    fn parse_button_coords_valid() {
        let value = json!({"x": 100.5, "y": 200.3});
        assert_eq!(parse_button_coordinates(&value), Some((100.5, 200.3)));
    }

    #[test]
    fn parse_button_coords_missing_x() {
        let value = json!({"y": 200.3});
        assert_eq!(parse_button_coordinates(&value), None);
    }

    #[test]
    fn parse_button_coords_missing_y() {
        let value = json!({"x": 100.5});
        assert_eq!(parse_button_coordinates(&value), None);
    }

    #[test]
    fn parse_button_coords_empty_object() {
        let value = json!({});
        assert_eq!(parse_button_coordinates(&value), None);
    }

    #[test]
    fn parse_button_coords_null_values() {
        let value = json!({"x": null, "y": 200.3});
        assert_eq!(parse_button_coordinates(&value), None);
    }

    #[test]
    fn parse_button_coords_non_numeric() {
        let value = json!({"x": "abc", "y": 200.3});
        assert_eq!(parse_button_coordinates(&value), None);
    }

    #[test]
    fn parse_button_coords_non_object() {
        assert_eq!(parse_button_coordinates(&json!(42)), None);
        assert_eq!(parse_button_coordinates(&json!("string")), None);
        assert_eq!(parse_button_coordinates(&json!(null)), None);
    }

    #[test]
    fn parse_button_coords_integer_values() {
        let value = json!({"x": 100, "y": 200});
        let coords = parse_button_coordinates(&value);
        assert!(coords.is_some());
        let (x, y) = coords.unwrap();
        assert!((x - 100.0).abs() < f64::EPSILON);
        assert!((y - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_button_coords_negative() {
        // Negative coordinates should still parse (coordinates can be off-screen)
        let value = json!({"x": -50.0, "y": -100.0});
        let coords = parse_button_coordinates(&value);
        assert!(coords.is_some());
        let (x, y) = coords.unwrap();
        assert!((x - -50.0).abs() < f64::EPSILON);
        assert!((y - -100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn parse_button_coords_zero() {
        // Zero is valid (element at origin)
        let value = json!({"x": 0.0, "y": 0.0});
        assert!(parse_button_coordinates(&value).is_some());
    }

    #[test]
    fn parse_button_coords_nan_and_infinity_via_null() {
        // Note: serde_json cannot represent NaN/Infinity — f64::NAN and f64::INFINITY
        // both serialize to Value::Null via the json!() macro. So these tests verify
        // that null-like poison values are rejected (the actual NaN/Infinity case is
        // impossible to construct with default serde_json).
        let value = json!({"x": null, "y": 100.0});
        assert_eq!(parse_button_coordinates(&value), None);
        let value = json!({"x": 100.0, "y": null});
        assert_eq!(parse_button_coordinates(&value), None);
        let value = json!({"x": null, "y": null});
        assert_eq!(parse_button_coordinates(&value), None);
    }

    #[test]
    fn parse_button_coords_large_values() {
        // Very large but finite values should still parse
        let value = json!({"x": 1e8, "y": 1e8});
        assert!(parse_button_coordinates(&value).is_some());
    }

    #[test]
    fn parse_button_coords_extra_fields() {
        // Extra fields beyond x,y should be ignored
        let value = json!({"x": 100.0, "y": 200.0, "width": 50, "height": 30, "found": true});
        let coords = parse_button_coordinates(&value);
        assert!(coords.is_some());
        let (x, y) = coords.unwrap();
        assert!((x - 100.0).abs() < f64::EPSILON);
        assert!((y - 200.0).abs() < f64::EPSILON);
    }

    // ====================================================================
    // parse_reply_verification
    // ====================================================================

    #[test]
    fn reply_verification_sent_true() {
        let value = json!({"sent": true});
        assert_eq!(
            parse_reply_verification(&value),
            EngagementOutcome::Completed
        );
    }

    #[test]
    fn reply_verification_sent_false() {
        let value = json!({"sent": false});
        assert_eq!(parse_reply_verification(&value), EngagementOutcome::Failed);
    }

    #[test]
    fn reply_verification_with_reason() {
        let value = json!({"sent": true, "reason": "composer closed"});
        assert_eq!(
            parse_reply_verification(&value),
            EngagementOutcome::Completed
        );
    }

    #[test]
    fn reply_verification_sent_false_with_reason() {
        let value = json!({"sent": false, "reason": "textarea still has text"});
        assert_eq!(parse_reply_verification(&value), EngagementOutcome::Failed);
    }

    #[test]
    fn reply_verification_missing_sent() {
        let value = json!({"reason": "something"});
        assert_eq!(
            parse_reply_verification(&value),
            EngagementOutcome::Unverified
        );
    }

    #[test]
    fn reply_verification_null_sent() {
        let value = json!({"sent": null});
        assert_eq!(
            parse_reply_verification(&value),
            EngagementOutcome::Unverified
        );
    }

    #[test]
    fn reply_verification_string_sent() {
        let value = json!({"sent": "true"});
        assert_eq!(
            parse_reply_verification(&value),
            EngagementOutcome::Unverified
        );
    }

    #[test]
    fn reply_verification_non_object() {
        assert_eq!(
            parse_reply_verification(&json!(42)),
            EngagementOutcome::Unverified
        );
        assert_eq!(
            parse_reply_verification(&json!("string")),
            EngagementOutcome::Unverified
        );
        assert_eq!(
            parse_reply_verification(&json!(null)),
            EngagementOutcome::Unverified
        );
    }

    #[test]
    fn reply_verification_empty_object() {
        assert_eq!(
            parse_reply_verification(&json!({})),
            EngagementOutcome::Unverified
        );
    }

    // ====================================================================
    // parse_following_result
    // ====================================================================

    #[test]
    fn following_result_true() {
        assert!(parse_following_result(&json!(true)));
    }

    #[test]
    fn following_result_false() {
        assert!(!parse_following_result(&json!(false)));
    }

    #[test]
    fn following_result_null() {
        assert!(!parse_following_result(&json!(null)));
    }

    #[test]
    fn following_result_string() {
        assert!(!parse_following_result(&json!("true")));
    }

    #[test]
    fn following_result_number() {
        assert!(!parse_following_result(&json!(1)));
    }

    #[test]
    fn following_result_non_boolean() {
        assert!(!parse_following_result(&json!({"following": true})));
    }
}
