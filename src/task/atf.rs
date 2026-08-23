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

/// Selector for the "Start Mining" bot command button.
const START_MINING_SELECTOR: &str = "div.new-message-bot-commands.is-view";

/// Default task runtime budget in milliseconds.
pub const DEFAULT_ATF_TASK_DURATION_MS: u64 = 60_000;

/// How long to wait for the "Start Mining" button to appear, in milliseconds.
const BUTTON_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

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

    // Make sure the "Start Mining" command is actually present before clicking.
    if !api
        .wait_for_visible(START_MINING_SELECTOR, BUTTON_VISIBILITY_TIMEOUT_MS)
        .await?
    {
        warn!("Start Mining button did not appear within {BUTTON_VISIBILITY_TIMEOUT_MS}ms");
    }

    // Step 3: mouse-click the bot command with the native cursor.
    info!("Mouse-clicking Start Mining");
    let outcome = api.nativeclick(START_MINING_SELECTOR).await?;
    info!("Click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("Start Mining click failed: {}", outcome.summary());
    }

    // Brief settle pause after the click.
    api.pause(1_000).await;

    info!("ATF task completed");
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::{
        task_duration_ms, DEFAULT_ATF_TASK_DURATION_MS, DEFAULT_URL, START_MINING_SELECTOR,
    };

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

    #[test]
    fn start_mining_selector_targets_bot_command() {
        assert_eq!(
            START_MINING_SELECTOR,
            "div.new-message-bot-commands.is-view"
        );
    }
}
