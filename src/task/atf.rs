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
//! 4. **State-driven loop** — read/scan/parse the mini-app iframe each iteration
//!    and act on what's actually present:
//!    - busy (modal/"Processing") → wait for it to clear
//!    - CLAIM action button (hourly miner) → click 2–4× with jitter
//!    - Tasks tab not active → activate it
//!    - `.btn-small` "Go" buttons → click the first one
//!    - `.btn-small` "Claim" buttons → click the first one
//!    - nothing actionable → wait 30–40s for validation, then finish
//!
//! Assumes the Telegram Web session is already logged in and the bot chat
//! renders its command bar without extra interaction.

use anyhow::{bail, Result};
use log::{info, warn};
use serde_json::Value;

use crate::prelude::TaskContext;
use crate::utils::math::random_in_range;
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

    // ── State-driven interaction loop ─────────────────────────────────────
    // Read / scan / parse the mini-app state each iteration and act on what
    // is actually present, adapting to whatever state the app is in.
    const MINIAPP_IFRAME: &str = "iframe.payment-verification";
    const STATE_SCAN_JS: &str = r#"(() => {
        const q = s => document.querySelector(s);
        const visible = el => { const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; };
        // Only VISIBLE buttons — hidden tabs (friends/profile/miners) also have
        // .btn-small buttons in the DOM; we must not act on them.
        const btns = [...document.querySelectorAll('.btn-small')].filter(visible);
        const text = b => (b.textContent || '').trim();
        const claimAction = q('.btn-main.state-claim#actionBtn');
        const tasksTab = q('#tab-tasks');
        const busyEl = [...document.querySelectorAll('[class*="modal"],[id*="modal"],[class*="overlay"],[id*="overlay"]')]
            .find(visible);
        const body = (document.body ? document.body.innerText : '').replace(/\s+/g, ' ').trim();
        return JSON.stringify({
            claimAction: !!claimAction && visible(claimAction) && text(claimAction) === 'CLAIM',
            tasksActive: !!tasksTab && tasksTab.classList.contains('active'),
            goButtons: btns.filter(b => text(b) === 'Go').map(b => b.id),
            claimButtons: btns.filter(b => text(b) === 'Claim').map(b => b.id),
            // Busy = a visible modal/overlay, or task-processing text. Keep the
            // text narrow — broad words like 'Connecting' match persistent UI.
            busyModal: !!busyEl,
            busyText: /Processing|Please wait/i.test(body)
        });
    })()"#;

    const MAX_STATE_ROUNDS: u32 = 120;
    const MAX_BUSY_ROUNDS: u32 = 20;
    let mut rounds = 0u32;
    let mut idle_rounds = 0u32;
    let mut busy_rounds = 0u32;
    loop {
        rounds += 1;
        if rounds > MAX_STATE_ROUNDS {
            warn!("[state] max rounds reached — stopping interaction loop");
            break;
        }
        let raw = match api.iframe_eval(MINIAPP_IFRAME, STATE_SCAN_JS, 15_000).await {
            Ok(v) => v,
            Err(e) => {
                // Transient iframe glitch (modal re-render) — retry instead of
                // aborting the whole task.
                warn!("[state] iframe scan failed: {e} — waiting 2s");
                api.wait(1_000, 2_000).await;
                continue;
            }
        };
        let value = match raw {
            Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),
            other => other,
        };
        let claim_action = value["claimAction"].as_bool().unwrap_or(false);
        let tasks_active = value["tasksActive"].as_bool().unwrap_or(false);
        let go_buttons: Vec<String> = value["goButtons"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let claim_buttons: Vec<String> = value["claimButtons"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let busy_modal = value["busyModal"].as_bool().unwrap_or(false);
        let busy_text = value["busyText"].as_bool().unwrap_or(false);

        // 1. Busy (modal / processing) — wait for it to clear, but cap the wait
        //    so a persistent modal (e.g. a leftover overlay) can't hang the task.
        if busy_modal || busy_text {
            busy_rounds += 1;
            info!("[state] busy (modal={busy_modal} text={busy_text}) round {busy_rounds}");
            if busy_rounds > MAX_BUSY_ROUNDS {
                warn!("[state] busy persisted — proceeding anyway");
                busy_rounds = 0;
            } else {
                api.wait(1_000, 2_000).await;
                idle_rounds = 0;
                continue;
            }
        } else {
            busy_rounds = 0;
        }
        // 2. CLAIM action button (hourly miner) — click with jitter.
        if claim_action {
            if let Some((cx, cy)) = api
                .iframe_center(MINIAPP_IFRAME, ".btn-main.state-claim#actionBtn", 10_000)
                .await?
            {
                let jx = random_in_range(0, 16) as f64 - 8.0;
                let jy = random_in_range(0, 16) as f64 - 8.0;
                let outcome = api
                    .iframe_click_at(MINIAPP_IFRAME, cx + jx, cy + jy, 5_000)
                    .await?;
                info!("[state] CLAIM action clicked: {}", outcome.summary());
            } else {
                info!("[state] CLAIM action gone — skip");
            }
            api.wait(500, 1_000).await;
            idle_rounds = 0;
            continue;
        }
        // 3. Tasks tab not active — activate it.
        if !tasks_active {
            info!("[state] opening Tasks tab");
            let outcome = api
                .iframe_click(
                    MINIAPP_IFRAME,
                    "[onclick=\"switchTab('tasks')\"]",
                    MINIAPP_ACTION_TIMEOUT_MS,
                )
                .await?;
            info!("[state] Tasks tab click: {}", outcome.summary());
            api.wait(1_000, 2_000).await;
            idle_rounds = 0;
            continue;
        }
        // 4. Go task buttons — click the first one.
        if let Some(id) = go_buttons.first() {
            let selector = format!(".btn-small#{id}");
            info!("[state] Go task '{id}'");
            match api
                .iframe_click_text(MINIAPP_IFRAME, &selector, "Go", 5_000)
                .await
            {
                Ok(outcome) => info!("[state] Go '{id}' clicked: {}", outcome.summary()),
                Err(e) => info!("[state] Go '{id}' gone: {e}"),
            }
            api.wait(500, 2_000).await;
            idle_rounds = 0;
            continue;
        }
        // 5. Claim task buttons — click the first one.
        if let Some(id) = claim_buttons.first() {
            let selector = format!(".btn-small#{id}");
            info!("[state] Claim task '{id}'");
            match api
                .iframe_click_text(MINIAPP_IFRAME, &selector, "Claim", 5_000)
                .await
            {
                Ok(outcome) => info!("[state] Claim '{id}' clicked: {}", outcome.summary()),
                Err(e) => info!("[state] Claim '{id}' gone: {e}"),
            }
            api.wait(500, 2_000).await;
            idle_rounds = 0;
            continue;
        }
        // 6. Nothing actionable — first idle waits for validation, second is done.
        idle_rounds += 1;
        if idle_rounds == 1 {
            info!("[state] nothing actionable — waiting 30-40s for validation");
            api.wait(30_000, 40_000).await;
            continue;
        }
        info!("[state] nothing actionable after validation wait — done");
        break;
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
