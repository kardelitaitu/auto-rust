//! ATF airdrop mining task — Telegram A (web) bot automation.
//!
//! Clone of the `atf` task for the Telegram **A** web client. Navigates to a
//! direct chat on `web.telegram.org/a`, waits a short random interval for the
//! page to settle, then mouse-clicks the "Start Mining" bot command to begin a
//! mining session.
//!
//! # Flow
//! 1. Navigate to `https://web.telegram.org/a/#8233119648`
//! 2. Wait a random 2–5 seconds (uniform) for the page to settle
//! 3. Mouse-click the "Start Mining" command button (no confirmation handling)
//! 4. `iframe_click` — click the "Tasks" tab inside the cross-origin mini-app
//!    iframe (attaches an OOPIF CDP session; no navigation, no new tab)
//! 5. `iframe_click` — click the YouTube task's "Go" button (targeted by
//!    class + id, scrolled into view)
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

    // Step 2: wait a random 2–4 seconds for the Telegram UI to settle.
    info!("Waiting random 2-5s for the page to settle");
    api.wait(2_000, 4_000).await;

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

    api.wait(2_000, 5_000).await;

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
    api.wait(1_000, 3_000).await;

    // Step 5: Click Go on Youtube Like — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-youtube_like_comment">Go</button>`).
    // Repeat clicks until the "Go" text is gone (the mini-app replaces it after
    // the task starts), with a random 1-2s interval, max 5 retries.
    info!("Starting STEP 5 --- CLICKING GO button on Youtube Like");
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
            info!("YouTube task 'Go' button gone (clicked {clicks}x)");
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
        info!("YouTube task click #{clicks} result: {}", outcome.summary());
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("YouTube task click failed: {}", outcome.summary());
        }
        if clicks >= GO_RETRY_MAX {
            info!("Reached max {GO_RETRY_MAX} clicks on YouTube task button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 6: Click Go on Twitter Retweet — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-twitter_retweet">Go</button>`).
    // Repeat clicks until the "Go" text is gone, random 0.5-2s interval, max 5.
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
            info!("Retweet task 'Go' button gone (clicked {clicks}x)");
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
        info!("Retweet task click #{clicks} result: {}", outcome.summary());
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("Retweet task click failed: {}", outcome.summary());
        }
        if clicks >= RETWEET_TASK_RETRY_MAX {
            info!("Reached max {RETWEET_TASK_RETRY_MAX} clicks on Retweet task button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 7: Click Go on Visit Website — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-btn-website_visit">Go</button>`).
    // Repeat clicks until the "Go" text is gone, random 0.5-2s interval, max 5.
    const WEBSITE_TASK_SELECTOR: &str = ".btn-small#btn-btn-website_visit";
    const WEBSITE_TASK_RETRY_MAX: u32 = 5;
    const WEBSITE_TASK_CHECK_TIMEOUT_MS: u64 = 2_000;
    let mut clicks = 0u32;
    loop {
        // Probe first — skip instantly when the "Go" button is gone.
        if !api
            .iframe_has_text(MINIAPP_IFRAME, WEBSITE_TASK_SELECTOR, "Go")
            .await?
        {
            info!("Website task 'Go' button gone (clicked {clicks}x)");
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
        info!("Website task click #{clicks} result: {}", outcome.summary());
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("Website task click failed: {}", outcome.summary());
        }
        if clicks >= WEBSITE_TASK_RETRY_MAX {
            info!("Reached max {WEBSITE_TASK_RETRY_MAX} clicks on Website task button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    api.wait(500, 2_000).await;

    // Step 8: Click Go on React Telegram Post — target by class + id + text content
    // (the button is `<button class="btn-small" id="btn-telegram_react_latest">Go</button>`).
    // Repeat clicks until the "Go" text is gone, random 0.5-2s interval, max 5.
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
            info!("React task 'Go' button gone (clicked {clicks}x)");
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
        info!("React task click #{clicks} result: {}", outcome.summary());
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            bail!("React task click failed: {}", outcome.summary());
        }
        if clicks >= REACT_TASK_RETRY_MAX {
            info!("Reached max {REACT_TASK_RETRY_MAX} clicks on React task button");
            break;
        }
        info!("Waiting random 0.5s-2s before next attempt");
        api.wait(500, 2_000).await;
    }

    // Switch focus back to the Telegram tab — earlier tasks (visit website /
    // youtube) may have opened a new tab and moved focus to it.
    api.focus_tab().await?;

    // long wait until all task validated
    api.wait(30_000, 40_000).await;

    // Step 9: Click Claim on YouTube Like — same button, text now "Claim".
    // Repeat clicks until the "Claim" text is gone, random 0.5-2s, max 5.
    const CLAIM_RETRY_MAX: u32 = 5;
    const CLAIM_CHECK_TIMEOUT_MS: u64 = 2_000;
    let mut clicks = 0u32;
    loop {
        match api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-youtube_like_comment",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await
        {
            Ok(outcome) => {
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
            }
            Err(e) => {
                info!("YouTube 'Claim' button gone (clicked {clicks}x): {e}");
                break;
            }
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
        match api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-twitter_retweet",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await
        {
            Ok(outcome) => {
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
            }
            Err(e) => {
                info!("Retweet 'Claim' button gone (clicked {clicks}x): {e}");
                break;
            }
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
        match api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-btn-website_visit",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await
        {
            Ok(outcome) => {
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
            }
            Err(e) => {
                info!("Website 'Claim' button gone (clicked {clicks}x): {e}");
                break;
            }
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
        match api
            .iframe_click_text(
                MINIAPP_IFRAME,
                ".btn-small#btn-telegram_react_latest",
                "Claim",
                CLAIM_CHECK_TIMEOUT_MS,
            )
            .await
        {
            Ok(outcome) => {
                clicks += 1;
                info!("React claim click #{clicks} result: {}", outcome.summary());
                if !matches!(
                    outcome.click,
                    crate::utils::mouse::types::ClickStatus::Success
                ) {
                    bail!("React claim click failed: {}", outcome.summary());
                }
            }
            Err(e) => {
                info!("React 'Claim' button gone (clicked {clicks}x): {e}");
                break;
            }
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
