//! Twitter Follow Task — Navigate to a user profile and click the follow button.
//!
//! This implementation is deterministic and scoped:
//! - Uses profile-header-specific selectors to avoid following wrong user
//! - Pre-checks for already-following state
//! - Retries with reload on failure (mirrors Node.js robustFollow)
//! - Verifies success by checking for "Following"/"Unfollow" button state

use log::{info, warn};
use anyhow::Result;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use crate::prelude::TaskContext;
use crate::utils::math::random_in_range;
use crate::utils::twitter::{
    close_active_popup,
    twitteractivity_selectors::*,
    twitteractivity_humanized::human_pause,
};

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const MAX_ATTEMPTS: u32 = 5;
const POST_RELOAD_ATTEMPTS: u32 = 2;
const RETRY_DELAY_BASE_MS: u64 = 500;
const VERIFY_TIMEOUT_MS: u64 = 20_000;

/// Main entry point
pub async fn run(ctx: &TaskContext, payload: Value) -> Result<()> {
    let username = extract_username_from_payload(&payload)?;
    let profile_url = format!("https://x.com/{}", username);

    info!("[twitterfollow] Starting: target=@{}", username);

    // 1. Navigate to profile
    ctx.navigate(&profile_url, DEFAULT_NAVIGATE_TIMEOUT_MS).await?;
    info!("[twitterfollow] Navigated to {}", profile_url);

    // 2. Verify we're on the correct profile
    verify_current_profile(ctx, &username).await?;

    // 3. Warm-up: read the profile (simulate human)
    human_pause(ctx, random_in_range(8000, 15000)).await;

    // 4. Core follow logic with retries
    let result = robust_follow(ctx, &username).await;

    match result {
        Ok(followed) => {
            if followed {
                info!("[twitterfollow] ✅ Successfully followed @{}", username);
            } else {
                info!("[twitterfollow] ℹ️ No follow action needed (already following or button not found)");
            }
        }
        Err(e) => {
            warn!("[twitterfollow] ❌ Failed to follow @{}: {}", username, e);
        }
    }

    info!("[twitterfollow] Task complete");
    Ok(())
}

/// Main follow loop with retries and page reload fallback
async fn robust_follow(ctx: &TaskContext, username: &str) -> Result<bool> {
    let mut attempt = 0;
    let mut has_reloaded = false;

    loop {
        attempt += 1;

        // Check if we've exhausted pre-reload attempts
        if attempt > MAX_ATTEMPTS {
            if has_reloaded || POST_RELOAD_ATTEMPTS == 0 {
                warn!("[twitterfollow] Exhausted all attempts");
                return Ok(false);
            }
            // Reload and continue with post-reload attempts
            info!("[twitterfollow] Reloading page to retry...");
            ctx.navigate(&format!("https://x.com/{}", username), DEFAULT_NAVIGATE_TIMEOUT_MS).await?;
            human_pause(ctx, random_in_range(5000, 10000)).await;
            has_reloaded = true;
            continue;
        }

        info!("[twitterfollow] Attempt {}/{}", attempt, MAX_ATTEMPTS + if has_reloaded { POST_RELOAD_ATTEMPTS } else { 0 });

        // Dismiss any interfering overlays (Escape + close active popup)
        let _ = ctx.press("Escape").await;
        let _ = close_active_popup(ctx).await;

        // Pre-check: already following?
        if check_already_following(ctx).await? {
            info!("[twitterfollow] Already following @{}", username);
            return Ok(true);
        }

        // Locate follow button (scoped to profile header)
        let button_coords = match find_follow_button_coords(ctx).await {
            Ok(Some(coords)) => coords,
            Ok(None) => {
                warn!("[twitterfollow] Follow button not found");
                // Retry with delay
                human_pause(ctx, random_in_range(1000, 3000)).await;
                continue;
            }
            Err(e) => {
                warn!("[twitterfollow] Error locating button: {}", e);
                human_pause(ctx, random_in_range(2000, 5000)).await;
                continue;
            }
        };

        // Safety re-check: button text still says "Follow"?
        if !button_says_follow(ctx).await? {
            info!("[twitterfollow] Button no longer says 'Follow' (already followed or state changed)");
            return Ok(true);
        }

        // Click
        humanized_click(ctx, button_coords.0, button_coords.1).await?;
        info!("[twitterfollow] Clicked follow button at ({:.0}, {:.0})", button_coords.0, button_coords.1);

        // Poll for follow state change (until unfollow button appears)
        match poll_for_follow_success(ctx).await {
            Ok(true) => {
                info!("[twitterfollow] Follow verified (unfollow button visible)");
                return Ok(true);
            }
            Ok(false) => {
                warn!("[twitterfollow] Follow not verified within timeout");
                // Retry
            }
            Err(e) => {
                warn!("[twitterfollow] Verification error: {}", e);
            }
        }

        // Wait before retry
        let delay = random_in_range(RETRY_DELAY_BASE_MS, RETRY_DELAY_BASE_MS * 2);
        human_pause(ctx, delay).await;
    }
}

/// Pre-check: Is the unfollow button already visible? If yes, we're already following.
async fn check_already_following(ctx: &TaskContext) -> Result<bool> {
    // The `selector_following_indicator()` returns true/false if any unfollow-like button exists
    let js = selector_following_indicator();
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if let Some(b) = value.and_then(|v| v.as_bool()) {
        return Ok(b);
    }
    Ok(false)
}

/// Find follow button coordinates using scoped selector.
/// Returns `Some((x,y))` for center of button, or `None` if not found.
async fn find_follow_button_coords(ctx: &TaskContext) -> Result<Option<(f64, f64)>> {
    let js = selector_follow_button();
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value();

    if let Some(obj) = value.and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            return Ok(Some((x, y)));
        }
    }
    Ok(None)
}

/// Check if the current follow button still says "Follow" (not "Following"/"Pending"/"Unfollow").
async fn button_says_follow(ctx: &TaskContext) -> Result<bool> {
    let js = selector_follow_button();
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value();

    if let Some(obj) = value.and_then(|v| v.as_object()) {
        if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
            let lower = text.to_lowercase();
            // If button says "following" or "unfollow", we shouldn't click
            if lower.contains("following") || lower.contains("unfollow") {
                return Ok(false);
            }
        }
        if let Some(label) = obj.get("label").and_then(|v| v.as_str()) {
            let lower = label.to_lowercase();
            if lower.contains("following") || lower.contains("unfollow") {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Perform a humanized click with mouse move and small pauses.
async fn humanized_click(ctx: &TaskContext, x: f64, y: f64) -> Result<()> {
    ctx.move_mouse_to(x, y).await?;
    human_pause(ctx, 300).await; // deliberate hover
    ctx.click(x, y).await?;
    human_pause(ctx, 800).await; // wait for any UI update
    Ok(())
}

/// Verify follow success by polling for the unfollow button.
/// Returns `Ok(true)` if unfollow button appears within timeout.
async fn poll_for_follow_success(ctx: &TaskContext) -> Result<bool> {
    let deadline = std::time::Instant::now() + Duration::from_millis(VERIFY_TIMEOUT_MS);

    loop {
        // Check 1: Is the unfollow button visible?
        let js_unfollow = selector_following_indicator(); // returns true if button says following/unfollow
        let result = ctx.page().evaluate(js_unfollow.to_string()).await?;
        let value = result.value();
        if let Some(true) = value.and_then(|v| v.as_bool()) {
            return Ok(true);
        }

        // Check 2: Re-evaluate follow button — did its text change to "Following"?
        let js_follow = selector_follow_button();
        let result2 = ctx.page().evaluate(js_follow.to_string()).await?;
        let value2 = result2.value();
        if let Some(obj) = value2.and_then(|v| v.as_object()) {
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                if text.to_lowercase().contains("following") {
                    return Ok(true);
                }
            }
            if let Some(label) = obj.get("label").and_then(|v| v.as_str()) {
                if label.to_lowercase().contains("following") {
                    return Ok(true);
                }
            }
        }

        // Timeout?
        if std::time::Instant::now() >= deadline {
            return Ok(false);
        }

        // Wait before next poll
        sleep(Duration::from_millis(500)).await;
    }
}

/// Verify that the current page's URL pathname contains the expected username.
async fn verify_current_profile(ctx: &TaskContext, expected_username: &str) -> Result<()> {
    let js = r#"
        (function() {
            return window.location.pathname;
        })()
    "#;
    let result = ctx.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if let Some(path) = value.and_then(|v| v.as_str()) {
        // path should be like "/username" or "/username/"
        let clean = path.trim_matches('/');
        if clean == expected_username {
            return Ok(());
        }
        // Could be case mismatch? Normalize
        if clean.to_lowercase() == expected_username.to_lowercase() {
            return Ok(());
        }
        anyhow::bail!("Profile mismatch: expected '{}', got path '{}'", expected_username, path);
    }
    anyhow::bail!("Could not read current URL pathname");
}

/// Extract username from payload (supports url/value/username fields)
fn extract_username_from_payload(payload: &Value) -> Result<String> {
    if let Some(value) = payload.get("url") {
        if let Some(url_str) = value.as_str() {
            let username = url_str
                .trim_start_matches("https://x.com/")
                .trim_start_matches("x.com/")
                .trim_start_matches("www.x.com/");
            if !username.is_empty() {
                return Ok(username.to_string());
            }
        }
    }

    if let Some(value) = payload.get("value") {
        if let Some(value_str) = value.as_str() {
            return Ok(value_str.to_string());
        }
    }

    if let Some(value) = payload.get("username") {
        if let Some(value_str) = value.as_str() {
            return Ok(value_str.to_string());
        }
    }

    // Fallback: any non-empty string field
    if let Some(obj) = payload.as_object() {
        for (key, val) in obj {
            if key != "url" && key != "value" && key != "username" {
                if let Some(v) = val.as_str() {
                    if !v.is_empty() {
                        return Ok(v.to_string());
                    }
                }
            }
        }
    }

    anyhow::bail!("No username found in payload");
}
