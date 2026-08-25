//! ATF airdrop mining task — Telegram bot automation.
//!
//! Navigates to the `ATF_AIRDROP` bot chat on Telegram Web, waits a short
//! random interval for the page to settle, then mouse-clicks the "Start Mining"
//! bot command to begin a mining session.
//!
//! # Flow
//! 1. Navigate to `https://web.telegram.org/k/#@ATF_AIRDROP_bot`
//! 2. Wait a random 2–5 seconds (uniform) for the page to settle
//! 3. Mouse-click the "Start Mining" command button (no confirmation handling)
//! 4. `iframe_click` — click the "Tasks" tab inside the cross-origin mini-app
//!    iframe (attaches an OOPIF CDP session; no navigation, no new tab)
//! 5. `iframe_click` — click each "Go" task button (scrolls through the list)
//!
//! Assumes the Telegram Web session is already logged in and the bot chat
//! renders its command bar without extra interaction.

use anyhow::{bail, Result};
use log::{info, warn};
use serde_json::Value;

use crate::prelude::TaskContext;
use crate::utils::timing::{duration_with_variance, run_with_timeout};

// ============================================================================
// Constants
// ============================================================================

/// Default target URL — Telegram Web chat with the ATF airdrop bot.
const DEFAULT_URL: &str = "https://web.telegram.org/k/#@ATF_AIRDROP_bot";

/// Default task runtime budget in milliseconds.
/// Large enough to cover the dev-hold pause (550-600s) plus the full flow.
pub const DEFAULT_ATF_TASK_DURATION_MS: u64 = 720_000;

/// How long to wait for the "Start Mining" button to appear, in milliseconds.
const BUTTON_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

/// How long to wait for a Telegram mini-app action button, in milliseconds.
const MINIAPP_ACTION_TIMEOUT_MS: u64 = 30_000;

// ============================================================================
// Task Entry Point
// ============================================================================

/// Main task entry point.
///
/// # Arguments
/// * `api` - `TaskContext` for browser automation
/// * `payload` - JSON payload; optional `url` key overrides the target chat
///
/// # Returns
/// * `Ok(())` - Mining started successfully
/// * `Err(e)` - Task failed with error
pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    run_with_timeout(duration_ms, "atf", run_inner(api, payload)).await
}

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_ATF_TASK_DURATION_MS, 20)
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    info!("ATF task started");

    // Step 1: navigate to the bot chat.
    let url = payload
        .get("url")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_URL);
    info!("Navigating to: {url}");
    api.navigate(url, crate::utils::timing::DEFAULT_NAVIGATION_TIMEOUT_MS)
        .await?;

    // Step 2: wait a random 2–5 seconds for the Telegram UI to settle.
    info!("Waiting random 2-4s for the page to settle");
    api.wait(2_000, 4_000).await;

    // Step 3: make sure the "Start Mining" command is present, then
    // mouse-click it with the native cursor.
    if !api
        .wait_for_visible(
            "div.new-message-bot-commands.is-view",
            BUTTON_VISIBILITY_TIMEOUT_MS,
        )
        .await?
    {
        warn!("Start Mining button did not appear within {BUTTON_VISIBILITY_TIMEOUT_MS}ms");
    }
    info!("Mouse-clicking Start Mining");
    let outcome = api.click("div.new-message-bot-commands.is-view").await?;
    info!("Click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("Start Mining click failed: {}", outcome.summary());
    }

    api.wait(1_000, 3_000).await;

    // The mini-app iframe has a stable class — select it by CSS rather than
    // a fragile positional XPath. (XPath is still supported for other cases.)
    const MINIAPP_IFRAME: &str = "iframe.payment-verification";

    // Step 4: click the "Task Menu" tab inside the same iframe.
    info!("[Step 4] Clicking Task Menu tab inside iframe");
    api.wait(2_000, 3_000).await;
    let outcome = api
        .iframe_click(
            MINIAPP_IFRAME,
            "[onclick=\"switchTab('tasks')\"]",
            MINIAPP_ACTION_TIMEOUT_MS,
        )
        .await?;
    info!("[Step 4] Task Menu click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("[Step 4] Task Menu click failed: {}", outcome.summary());
    }
    api.wait(1_000, 3_000).await;

    // Step 5: Click Go on Youtube Like — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-youtube_like_comment">Go</button>`).
    // Repeat clicks until the "Go" text is gone (the mini-app replaces it after
    // the task starts), with a random 1-2s interval, max 5 retries.
    info!("[Step 5] Starting . . .");
    const YOUTUBE_TASK_SELECTOR: &str = ".btn-small#btn-youtube_like_comment";
    const GO_RETRY_MAX: u32 = 5;
    const GO_CHECK_TIMEOUT_MS: u64 = 2_000;
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Go" button is gone, instead of
        // polling the full click timeout to discover it.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, YOUTUBE_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 5] YouTube task 'Go' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                YOUTUBE_TASK_SELECTOR,
                "Go",
                GO_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!(
            "[Step 5] YouTube task click #{clicks} result: {}",
            outcome.summary()
        );
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("[Step 5] YouTube task click failed: {}", outcome.summary());
        }
        // Post-click probe: detect the state change (busy/done) right away
        // instead of only noticing it on the next pre-click probe.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, YOUTUBE_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 5] YouTube task done (clicked {clicks}x)");
            break;
        }
        if clicks >= GO_RETRY_MAX {
            info!("[Step 5] Reached max {GO_RETRY_MAX} clicks on YouTube task button");
            break;
        }
        info!("[Step 5] Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 6: Click Go on Twitter Retweet — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-twitter_retweet">Go</button>`).
    // Repeat clicks until the "Go" text is gone, random 0.5-2s interval, max 5.
    // Step 5 may have opened a new tab (YouTube) — refocus the Telegram tab first.
    api.focus_tab().await?;
    info!("[Step 6] Starting . . .");
    const RETWEET_TASK_SELECTOR: &str = ".btn-small#btn-twitter_retweet";
    const RETWEET_TASK_RETRY_MAX: u32 = 5;
    const RETWEET_TASK_CHECK_TIMEOUT_MS: u64 = 2_000;
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Go" button is gone.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, RETWEET_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 6] Retweet task 'Go' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                RETWEET_TASK_SELECTOR,
                "Go",
                RETWEET_TASK_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!(
            "[Step 6] Retweet task click #{clicks} result: {}",
            outcome.summary()
        );
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("[Step 6] Retweet task click failed: {}", outcome.summary());
        }
        // Post-click probe: stop as soon as the task state changes.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, RETWEET_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 6] Retweet task done (clicked {clicks}x)");
            break;
        }
        if clicks >= RETWEET_TASK_RETRY_MAX {
            info!("[Step 6] Reached max {RETWEET_TASK_RETRY_MAX} clicks on Retweet task button");
            break;
        }
        info!("[Step 6] Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    // Wait for development inspection
    info!("Wait for development inspection");
    api.wait(550_000, 600_000).await;

    api.wait(500, 2_000).await;

    // Step 7: Click Go on Visit Website — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-website_visit">Go</button>`).
    // Repeat clicks until the "Go" text is gone, random 0.5-2s interval, max 5.
    api.focus_tab().await?;
    info!("[Step 7] Starting . . .");
    const WEBSITE_TASK_SELECTOR: &str = ".btn-small#btn-website_visit";
    const WEBSITE_TASK_RETRY_MAX: u32 = 5;
    const WEBSITE_TASK_CHECK_TIMEOUT_MS: u64 = 2_000;
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Go" button is gone.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, WEBSITE_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 7] Website task 'Go' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                WEBSITE_TASK_SELECTOR,
                "Go",
                WEBSITE_TASK_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!(
            "[Step 7] Website task click #{clicks} result: {}",
            outcome.summary()
        );
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("[Step 7] Website task click failed: {}", outcome.summary());
        }
        // Post-click probe: stop as soon as the task state changes.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, WEBSITE_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 7] Website task done (clicked {clicks}x)");
            break;
        }
        if clicks >= WEBSITE_TASK_RETRY_MAX {
            info!("[Step 7] Reached max {WEBSITE_TASK_RETRY_MAX} clicks on Website task button");
            break;
        }
        info!("[Step 7] Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 8: Click Go on React Telegram Post — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-telegram_react_latest">Go</button>`).
    // Repeat clicks until the "Go" text is gone, random 0.5-2s interval, max 5.
    // Step 7 (Visit Website) likely opened a new tab — refocus first.
    api.focus_tab().await?;
    info!("[Step 8] Starting . . .");
    const REACT_TASK_SELECTOR: &str = ".btn-small#btn-telegram_react_latest";
    const REACT_TASK_RETRY_MAX: u32 = 5;
    const REACT_TASK_CHECK_TIMEOUT_MS: u64 = 2_000;
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Go" button is gone.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, REACT_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 8] React task 'Go' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                REACT_TASK_SELECTOR,
                "Go",
                REACT_TASK_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!(
            "[Step 8] React task click #{clicks} result: {}",
            outcome.summary()
        );
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("[Step 8] React task click failed: {}", outcome.summary());
        }
        // Post-click probe: stop as soon as the task state changes.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, REACT_TASK_SELECTOR, "Go")
            .await?
        {
            info!("[Step 8] React task done (clicked {clicks}x)");
            break;
        }
        if clicks >= REACT_TASK_RETRY_MAX {
            info!("[Step 8] Reached max {REACT_TASK_RETRY_MAX} clicks on React task button");
            break;
        }
        info!("[Step 8] Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    // Switch focus back to the Telegram tab — earlier tasks (visit website /
    // youtube) may have opened a new tab and moved focus to it.
    api.focus_tab().await?;

    // long wait until all task validated
    info!("Waiting random 30-40s until all task synced");
    api.wait(30_000, 40_000).await;

    // Step 9: Click Claim on YouTube Like — same button, text now "Claim".
    // Repeat clicks until the "Claim" text is gone, random 0.5-2s, max 5.
    const CLAIM_RETRY_MAX: u32 = 5;
    const CLAIM_CHECK_TIMEOUT_MS: u64 = 2_000;
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Claim" button is gone.
        if !api
            .iframe_has_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-youtube_like_comment",
                "Claim",
            )
            .await?
        {
            info!("YouTube 'Claim' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-youtube_like_comment",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!(
            "YouTube claim click #{clicks} result: {}",
            outcome.summary()
        );
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("YouTube claim click failed: {}", outcome.summary());
        }
        // Post-click probe: stop as soon as the task state changes.
        if !api
            .iframe_has_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-youtube_like_comment",
                "Claim",
            )
            .await?
        {
            info!("YouTube claim done (clicked {clicks}x)");
            break;
        }
        if clicks >= CLAIM_RETRY_MAX {
            info!("Reached max {CLAIM_RETRY_MAX} clicks on YouTube claim button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 10: Click Claim on Twitter Retweet — target by class + id + "Claim" text.
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Claim" button is gone.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, ".btn-small#btn-twitter_retweet", "Claim")
            .await?
        {
            info!("Retweet 'Claim' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-twitter_retweet",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!(
            "Retweet claim click #{clicks} result: {}",
            outcome.summary()
        );
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("Retweet claim click failed: {}", outcome.summary());
        }
        // Post-click probe: stop as soon as the task state changes.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, ".btn-small#btn-twitter_retweet", "Claim")
            .await?
        {
            info!("Retweet claim done (clicked {clicks}x)");
            break;
        }
        if clicks >= CLAIM_RETRY_MAX {
            info!("Reached max {CLAIM_RETRY_MAX} clicks on Retweet claim button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 11: Click Claim on Visit Website — target by class + id + "Claim" text.
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Claim" button is gone.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, ".btn-small#btn-website_visit", "Claim")
            .await?
        {
            info!("Website 'Claim' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-website_visit",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!(
            "Website claim click #{clicks} result: {}",
            outcome.summary()
        );
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("Website claim click failed: {}", outcome.summary());
        }
        // Post-click probe: stop as soon as the task state changes.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, ".btn-small#btn-website_visit", "Claim")
            .await?
        {
            info!("Website claim done (clicked {clicks}x)");
            break;
        }
        if clicks >= CLAIM_RETRY_MAX {
            info!("Reached max {CLAIM_RETRY_MAX} clicks on Website claim button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 12: Click Claim on React Telegram Post — target by class + id + "Claim" text.
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Claim" button is gone.
        if !api
            .iframe_has_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-telegram_react_latest",
                "Claim",
            )
            .await?
        {
            info!("React 'Claim' button gone (clicked {clicks}x)");
            break;
        }
        let outcome = api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-telegram_react_latest",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await?;
        clicks += 1;
        info!("React claim click #{clicks} result: {}", outcome.summary());
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("React claim click failed: {}", outcome.summary());
        }
        // Post-click probe: stop as soon as the task state changes.
        if !api
            .iframe_has_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-telegram_react_latest",
                "Claim",
            )
            .await?
        {
            info!("React claim done (clicked {clicks}x)");
            break;
        }
        if clicks >= CLAIM_RETRY_MAX {
            info!("Reached max {CLAIM_RETRY_MAX} clicks on React claim button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Settle pause so the mining app opens before the task ends. DO NOT REMOVE THIS
    info!("Finalizing Tasks");
    api.pause(200_000).await;

    info!("ATF task completed");
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::{task_duration_ms, DEFAULT_ATF_TASK_DURATION_MS, DEFAULT_URL};

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        let min = DEFAULT_ATF_TASK_DURATION_MS * 80 / 100; // 20% variance floor
        let max = DEFAULT_ATF_TASK_DURATION_MS * 120 / 100; // 20% variance ceiling
        assert!(
            (min..=max).contains(&duration_ms),
            "duration {duration_ms} outside {min}..={max}"
        );
    }

    #[test]
    fn default_url_is_telegram_atf_bot() {
        assert_eq!(DEFAULT_URL, "https://web.telegram.org/k/#@ATF_AIRDROP_bot");
    }
}
