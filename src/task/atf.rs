//! ATF airdrop mining task — Telegram bot automation.
//!
//! Navigates to the `ATF_AIRDROP` bot chat on Telegram Web, waits a short
//! random interval for the page to settle, then mouse-clicks the "Start Mining"
//! bot command to begin a mining session.
//!
//! # Flow
//! 1. Navigate to `https://web.telegram.org/k/#@ATF_AIRDROP_bot`
//! 2. Wait a random 2–3 seconds (uniform variance around 2.5s)
//! 3. Native mouse-click the "Start Mining" command button
//! 4. Wait for the "Launch" confirmation popup and mouse-click it
//! 5. Read the mini-app iframe `src` and navigate the tab to it directly
//!    (the mini-app is a cross-origin iframe that selectors cannot reach)
//! 6. Click the mini-app "Go" button
//! 7. Switch to the "Mine" tab
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
pub const DEFAULT_ATF_TASK_DURATION_MS: u64 = 60_000;

/// How long to wait for the "Start Mining" button to appear, in milliseconds.
const BUTTON_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

/// How long to wait for the "Launch" confirmation popup, in milliseconds.
const LAUNCH_POPUP_TIMEOUT_MS: u64 = 20_000;

/// How long to wait for a Telegram mini-app action button, in milliseconds.
const MINIAPP_ACTION_TIMEOUT_MS: u64 = 30_000;

/// Base wait before clicking, in milliseconds. 20% variance yields 2–3s.
const PRE_CLICK_WAIT_BASE_MS: u64 = 2_500;
const PRE_CLICK_WAIT_VARIANCE_PCT: u32 = 20;

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

    // Step 2: wait a random 2–3 seconds for the Telegram UI to settle.
    info!("Waiting random 2-3s for the page to settle");
    api.pause_with_variance(PRE_CLICK_WAIT_BASE_MS, PRE_CLICK_WAIT_VARIANCE_PCT)
        .await;

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

    api.pause(2_000).await;

    // Step 4: the bot shows a "Launch" confirmation popup — wait for it and
    // mouse-click it. The popup is a floating overlay that animates in.
    if !api
        .wait_for_visible("button.popup-button.primary", LAUNCH_POPUP_TIMEOUT_MS)
        .await?
    {
        warn!("Launch confirmation popup did not appear within {LAUNCH_POPUP_TIMEOUT_MS}ms");
    }
    info!("Mouse-clicking Launch");
    let outcome = api.click("button.popup-button.primary").await?;
    info!("Launch click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("Launch confirmation click failed: {}", outcome.summary());
    }

    api.pause(2_000).await;

    // Step 5: the mini-app opens in a cross-origin iframe — selectors can't
    // reach inside it. Read its `src` (readable cross-origin) and navigate
    // the tab directly to the mini-app URL, making it the top document.
    if !api.wait_for("iframe", MINIAPP_ACTION_TIMEOUT_MS).await? {
        warn!("Mini-app iframe did not appear within {MINIAPP_ACTION_TIMEOUT_MS}ms");
    }
    let miniapp_url = api
        .attr("iframe", "src")
        .await?
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Mini-app iframe has no src attribute"))?;
    info!(
        "Navigating to mini-app: {}",
        &miniapp_url[..miniapp_url.len().min(120)]
    );
    api.navigate(
        &miniapp_url,
        crate::utils::timing::DEFAULT_NAVIGATION_TIMEOUT_MS,
    )
    .await?;
    api.pause(2_000).await;

    // Step 6: inside the mini-app, click the "Go" action button.
    if !api
        .wait_for_visible("#btn-telegram_react_latest", MINIAPP_ACTION_TIMEOUT_MS)
        .await?
    {
        warn!("Mini-app Go button did not appear within {MINIAPP_ACTION_TIMEOUT_MS}ms");
    }
    info!("Mouse-clicking mini-app Go button");
    let outcome = api.click("#btn-telegram_react_latest").await?;
    info!("Go click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("Mini-app Go click failed: {}", outcome.summary());
    }

    // Step 7: switch to the "Mine" tab.
    api.pause(1_500).await;
    if !api
        .wait_for_visible("[onclick=\"switchTab('home')\"]", MINIAPP_ACTION_TIMEOUT_MS)
        .await?
    {
        warn!("Mine tab did not appear within {MINIAPP_ACTION_TIMEOUT_MS}ms");
    }
    info!("Mouse-clicking Mine tab");
    let outcome = api.click("[onclick=\"switchTab('home')\"]").await?;
    info!("Mine tab click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("Mine tab click failed: {}", outcome.summary());
    }

    // Settle pause so the mining app opens before the task ends. DO NOT REMOVE THIS
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
