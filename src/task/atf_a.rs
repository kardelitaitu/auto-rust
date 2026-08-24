//! ATF airdrop mining task — Telegram A (web) bot automation.
//!
//! Clone of the `atf` task for the Telegram **A** web client. Navigates to a
//! direct chat on `web.telegram.org/a`, waits a short random interval for the
//! page to settle, then mouse-clicks the "Start Mining" bot command to begin a
//! mining session.
//!
//! # Flow
//! 1. Navigate to `https://web.telegram.org/a/#8233119648`
//! 2. Wait a random 2–3 seconds (uniform variance around 2.5s)
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

/// Default target URL — Telegram Web A client, direct chat with the ATF bot.
const DEFAULT_URL: &str = "https://web.telegram.org/a/#8233119648";

/// Default task runtime budget in milliseconds.
pub const DEFAULT_ATF_A_TASK_DURATION_MS: u64 = 150_000;

/// How long to wait for the "Start Mining" button to appear, in milliseconds.
const BUTTON_VISIBILITY_TIMEOUT_MS: u64 = 30_000;

/// Telegram A client: the bot-command (Start Mining) button in the composer.
const START_MINING_SELECTOR: &str = "#MiddleColumn > div.messages-layout > div.Transition > div > div.middle-column-footer > div.Composer.is-chat-composer.shown.mounted > div.composer-wrapper > div > button.Button.composer-action-button.bot-menu.open.default.translucent.round";

/// Telegram A client: the mini-app iframe lives in the browser modal under
/// `#portals` (the K client used a stable `iframe.payment-verification` class;
/// the A client's iframe has only a generated class, so the DOM path is used).
const MINIAPP_IFRAME: &str = "#portals > div:nth-child(4) > div > div > div.modal-dialog.browser-modal-dialog > div.modal-content.custom-scroll > div > iframe";

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
    run_with_timeout(duration_ms, "atf_a", run_inner(api, payload)).await
}

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_ATF_A_TASK_DURATION_MS, 20)
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    info!("ATF-A task started");

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
    api.pause(2_000).await;

    // Step 3: make sure the "Start Mining" command button is present, then
    // mouse-click it with the native cursor. The Telegram A client has no
    // `.new-message-bot-commands` class — the button lives in the composer.
    if !api
        .wait_for_visible(START_MINING_SELECTOR, BUTTON_VISIBILITY_TIMEOUT_MS)
        .await?
    {
        warn!("Start Mining button did not appear within {BUTTON_VISIBILITY_TIMEOUT_MS}ms");
    }
    info!("Mouse-clicking Start Mining");
    let outcome = api.click(START_MINING_SELECTOR).await?;
    info!("Click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("Start Mining click failed: {}", outcome.summary());
    }

    api.pause(2_000).await;

    // Step 4: click the "Task Menu" tab inside the same iframe.
    info!("Clicking Task Menu tab inside iframe");
    let outcome = api
        .iframe_click(
            MINIAPP_IFRAME,
            "[onclick=\"switchTab('tasks')\"]",
            MINIAPP_ACTION_TIMEOUT_MS,
        )
        .await?;
    info!("Task Menu click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("Task Menu click failed: {}", outcome.summary());
    }
    api.pause(2_000).await;

    // Step 5: click each "Go" task button inside the iframe. The mini-app
    // scrolls its task list; `iframe_click` scrolls each button into view.
    // Track the last clicked local point and skip it on the next iteration so
    // a button that stays visible (marked "done") isn't clicked repeatedly.
    const GO_LOOP_TIMEOUT_MS: u64 = 8_000;
    let total_go = api
        .iframe_count(MINIAPP_IFRAME, "[id^=\"btn-telegram_\"]", 5_000)
        .await
        .unwrap_or(0);
    info!("Found {total_go} Go button(s) to click");
    let mut go_clicks = 0u32;
    let mut skip: Option<(f64, f64)> = None;
    loop {
        let timeout = if go_clicks == 0 {
            MINIAPP_ACTION_TIMEOUT_MS
        } else {
            GO_LOOP_TIMEOUT_MS
        };
        let outcome = match skip {
            None => {
                api.iframe_click(MINIAPP_IFRAME, "[id^=\"btn-telegram_\"]", timeout)
                    .await
            }
            Some((sx, sy)) => {
                api.iframe_click_skip(MINIAPP_IFRAME, "[id^=\"btn-telegram_\"]", sx, sy, timeout)
                    .await
            }
        };
        match outcome {
            Ok(outcome) => {
                go_clicks += 1;
                info!("Go click #{go_clicks} result: {}", outcome.summary());
                if !matches!(
                    outcome.click,
                    crate::utils::mouse::types::ClickStatus::Success
                ) {
                    bail!("Task click 'Go' failed: {}", outcome.summary());
                }
                // Convert the clicked absolute point back to iframe-local coords
                // so the next iteration skips this button.
                if let Ok(rect) = api.get_element_rect(MINIAPP_IFRAME).await {
                    skip = Some((outcome.x - rect.x, outcome.y - rect.y));
                }
            }
            Err(e) => {
                info!("No more Go buttons (clicked {go_clicks}): {e}");
                break;
            }
        }
        api.pause(1_500).await;
    }
    api.pause(2_000).await;

    // Settle pause so the mining app opens before the task ends. DO NOT REMOVE THIS
    api.pause(200_000).await;

    info!("ATF-A task completed");
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::{
        task_duration_ms, DEFAULT_ATF_A_TASK_DURATION_MS, DEFAULT_URL, MINIAPP_IFRAME,
        START_MINING_SELECTOR,
    };

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        let min = DEFAULT_ATF_A_TASK_DURATION_MS * 80 / 100; // 20% variance floor
        let max = DEFAULT_ATF_A_TASK_DURATION_MS * 120 / 100; // 20% variance ceiling
        assert!(
            (min..=max).contains(&duration_ms),
            "duration {duration_ms} outside {min}..={max}"
        );
    }

    #[test]
    fn default_url_is_telegram_a_chat() {
        assert_eq!(DEFAULT_URL, "https://web.telegram.org/a/#8233119648");
    }

    #[test]
    fn start_mining_selector_targets_composer_button() {
        // Telegram A composer bot-menu button — must anchor on MiddleColumn.
        assert!(START_MINING_SELECTOR.starts_with("#MiddleColumn > "));
        assert!(START_MINING_SELECTOR.contains("composer-action-button"));
        assert!(START_MINING_SELECTOR.contains("bot-menu"));
    }

    #[test]
    fn miniapp_iframe_selector_targets_portals_modal() {
        // Telegram A hosts the mini-app in the #portals browser modal,
        // NOT the K client's `iframe.payment-verification` class.
        assert!(MINIAPP_IFRAME.starts_with("#portals > "));
        assert!(MINIAPP_IFRAME.contains("browser-modal-dialog"));
        assert!(!MINIAPP_IFRAME.contains("payment-verification"));
    }
}
