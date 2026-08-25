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

/// The cross-origin mini-app iframe (Telegram Web K client).
const MINIAPP_IFRAME: &str = "iframe.payment-verification";

/// Scans the mini-app state: busy modal/text, tasks tab active, and the
/// visible Go/Claim task buttons.
const STATE_SCAN_JS: &str = r#"(() => {
    const q = s => document.querySelector(s);
    // Truly visible: has layout size AND is not hidden via display/visibility/opacity.
    // Hidden modal shells (e.g. #withdrawModal with visibility:hidden) still have
    // a non-zero rect — size alone is not enough.
    const visible = el => {
        if (!el) return false;
        const r = el.getBoundingClientRect();
        if (r.width <= 0 || r.height <= 0) return false;
        const s = window.getComputedStyle(el);
        return s.display !== 'none' && s.visibility !== 'hidden' && Number(s.opacity) !== 0;
    };
    const btns = [...document.querySelectorAll('.btn-small')].filter(visible);
    const text = b => (b.textContent || '').trim();
    const claimAction = q('.btn-main.state-claim#actionBtn');
    const tasksTab = q('#tab-tasks');
    const busyEl = [...document.querySelectorAll('[class*="modal"],[id*="modal"],[class*="dialog"],[id*="dialog"],[class*="popup"],[id*="popup"]')]
        .find(visible);
    const body = (document.body ? document.body.innerText : '').replace(/\s+/g, ' ').trim();
    return JSON.stringify({
        claimAction: !!claimAction && visible(claimAction) && text(claimAction) === 'CLAIM',
        tasksActive: !!tasksTab && tasksTab.classList.contains('active'),
        goButtons: btns.filter(b => text(b) === 'Go').map(b => b.id),
        claimButtons: btns.filter(b => text(b) === 'Claim').map(b => b.id),
        busyModal: !!busyEl,
        busyModalId: busyEl ? (busyEl.id || busyEl.className || busyEl.tagName).toString().slice(0, 80) : '',
        busyText: /Processing|Please wait/i.test(body)
    });
})()"#;

/// Best-effort modal dismissal: click any visible close/backdrop element.
const DISMISS_MODAL_JS: &str = r#"(() => {
    const candidates = [...document.querySelectorAll(
        '[class*="close"],[id*="close"],[aria-label*="Close"],[data-dismiss],[class*="backdrop"],[class*="overlay"]'
    )].filter(e => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; });
    for (const el of candidates) { try { el.click(); } catch (_) {} }
    return candidates.length;
})()"#;

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

    api.wait(3_000, 6_000).await;

    // ── Interaction steps (each independent) ──────────────────────────────
    // Every step reads the current mini-app state before acting, and skips
    // gracefully when its button is not present. Add / remove / reorder steps
    // freely without touching the others.

    // Step 4a: click the "Claim hourly miner" CLAIM action button 2-4 times
    // with a small random offset each time (not the exact same position),
    // random 0.5-1s between clicks. No verification.
    // (the button is `<button class="btn-main state-claim" id="actionBtn">CLAIM</button>`)
    info!("[Step 4a] Starting . . .");
    if let Some((cx, cy)) = api
        .iframe_center(
            MINIAPP_IFRAME,
            ".btn-main.state-claim#actionBtn",
            MINIAPP_ACTION_TIMEOUT_MS,
        )
        .await?
    {
        let clicks = random_in_range(1, 3);
        info!("[Step 4a] Claim button found, clicking {clicks}x");
        for i in 0..clicks {
            let jx = random_in_range(0, 16) as f64 - 8.0;
            let jy = random_in_range(0, 16) as f64 - 8.0;
            let outcome = api
                .iframe_click_at(MINIAPP_IFRAME, cx + jx, cy + jy, 5_000)
                .await?;
            info!(
                "[Step 4a] Claim click #{}/{} result: {}",
                i + 1,
                clicks,
                outcome.summary()
            );
            if i + 1 < clicks {
                api.wait(500, 1_000).await;
            }
        }
    } else {
        info!("[Step 4a] Claim button not found — skipping");
    }
    api.wait(500, 2_000).await;

    // Step 4b: activate the Tasks tab. Click exactly ONCE — it always succeeds,
    // and the click is harmless if the tab is already active.
    info!("[Step 4b] Opening Tasks tab");
    let outcome = api
        .iframe_click(
            MINIAPP_IFRAME,
            "[onclick=\"switchTab('tasks')\"]",
            MINIAPP_ACTION_TIMEOUT_MS,
        )
        .await
        .map_err(|e| anyhow::anyhow!("[Step 4b] Tasks tab click target not found: {e}"))?;
    info!("[Step 4b] Tasks tab click: {}", outcome.summary());
    api.wait(1_000, 2_000).await;
    ensure_tasks_visible(api).await?;

    // Steps 5-8: click each "Go" task button.
    click_task_button(api, ".btn-small#btn-youtube_like_comment", "Go", "[Step 5]").await?;
    click_task_button(api, ".btn-small#btn-twitter_retweet", "Go", "[Step 6]").await?;
    click_task_button(api, ".btn-small#btn-website_visit", "Go", "[Step 7]").await?;
    click_task_button(
        api,
        ".btn-small#btn-telegram_react_latest",
        "Go",
        "[Step 8]",
    )
    .await?;

    // Wait for the mini-app to validate the tasks (buttons become "Claim").
    info!("Waiting random 30-40s until all task synced");
    api.wait(30_000, 35_000).await;

    // Steps 9-12: click each "Claim" task button.
    click_task_button(
        api,
        ".btn-small#btn-youtube_like_comment",
        "Claim",
        "[Step 9]",
    )
    .await?;
    click_task_button(api, ".btn-small#btn-twitter_retweet", "Claim", "[Step 10]").await?;
    click_task_button(api, ".btn-small#btn-website_visit", "Claim", "[Step 11]").await?;
    click_task_button(
        api,
        ".btn-small#btn-telegram_react_latest",
        "Claim",
        "[Step 12]",
    )
    .await?;

    api.wait(500, 2_000).await;

    // Settle pause so the mining app opens before the task ends. DO NOT REMOVE THIS
    info!("Finalizing Tasks");
    api.pause(200_000).await;

    info!("ATF task completed");
    Ok(())
}

/// Wait for a modal/"Processing" busy state to clear (up to ~20s).
async fn wait_not_busy(api: &TaskContext) -> Result<()> {
    let start = std::time::Instant::now();
    loop {
        let raw = api
            .iframe_eval(MINIAPP_IFRAME, STATE_SCAN_JS, 10_000)
            .await?;
        let value = match raw {
            Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),
            other => other,
        };
        let modal = value["busyModal"].as_bool().unwrap_or(false);
        let text = value["busyText"].as_bool().unwrap_or(false);
        if !modal && !text {
            return Ok(());
        }
        let modal_id = value["busyModalId"].as_str().unwrap_or("");
        info!("[busy] waiting for state to clear (modal={modal} id='{modal_id}' text={text})");
        let _ = api
            .iframe_eval(MINIAPP_IFRAME, DISMISS_MODAL_JS, 5_000)
            .await;
        if start.elapsed().as_millis() >= 5_000 {
            break;
        }
        api.wait(1_000, 1_500).await;
    }
    warn!("[busy] state did not clear within 5s — proceeding anyway");
    Ok(())
}

/// Wait until the Tasks buttons are actually visible (have non-zero size).
/// If the Tasks tab is not active, invokes `window.switchTab('tasks')` inside
/// the iframe to guarantee tab activation.
async fn ensure_tasks_visible(api: &TaskContext) -> Result<()> {
    for attempt in 0..5 {
        let raw = api
            .iframe_eval(MINIAPP_IFRAME, STATE_SCAN_JS, 10_000)
            .await?;
        let value = match raw {
            Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),
            other => other,
        };
        let tasks_active = value["tasksActive"].as_bool().unwrap_or(false);
        let go = value["goButtons"].as_array().map(|a| a.len()).unwrap_or(0);
        let claim = value["claimButtons"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        if tasks_active && (go > 0 || claim > 0) {
            return Ok(());
        }
        if !tasks_active {
            info!(
                "[tasks] Tasks tab not active (attempt {}), invoking switchTab('tasks')",
                attempt + 1
            );
            let _ = api
                .iframe_eval(
                    MINIAPP_IFRAME,
                    "(() => { if (typeof window.switchTab === 'function') { window.switchTab('tasks'); return true; } return false; })()",
                    5_000,
                )
                .await;
        }
        api.wait(1_000, 2_000).await;
    }
    info!("[tasks] tasks not visible after retries — proceeding");
    Ok(())
}

/// Click a task button (by selector + exact text) until its state changes.
///
/// Robust per-step helper: waits for busy to clear, ensures the Tasks tab is
/// visible, probes before AND after each click, and stops when the button is
/// gone/disabled/busy or its text no longer matches. Max 5 clicks, 0.5-2s
/// between attempts. Skips cleanly if the button is never present (returns false).
async fn click_task_button(
    api: &TaskContext,
    selector: &str,
    text: &str,
    label: &str,
) -> Result<bool> {
    const MAX_RETRY: u32 = 3;
    const TIMEOUT_MS: u64 = 5_000;
    wait_not_busy(api).await?;
    ensure_tasks_visible(api).await?;
    let mut clicks = 0u32;
    loop {
        if !api.iframe_has_text(MINIAPP_IFRAME, selector, text).await? {
            info!("[{label}] skipped — no '{text}' button");
            return Ok(clicks > 0);
        }
        // The click itself re-resolves the element (scrollIntoView + hit-test).
        // If it can't be found (blank area / iframe changed), skip the step
        // instead of crashing the whole task.
        let outcome = match api
            .iframe_click_text(MINIAPP_IFRAME, selector, text, TIMEOUT_MS)
            .await
        {
            Ok(o) => o,
            Err(e) => {
                info!("[{label}] click target not found — skipping (clicked {clicks}x): {e}");
                return Ok(clicks > 0);
            }
        };
        clicks += 1;
        info!("[{label}] click #{clicks}: {}", outcome.summary());
        if !matches!(
            outcome.click,
            crate::utils::mouse::types::ClickStatus::Success
        ) {
            info!("[{label}] click did not land — moving on (clicked {clicks}x)");
            return Ok(clicks > 0);
        }
        // Allow mini-app JS time to process click event and update DOM state
        // (e.g. 'Go' -> 'Processing' / 'Claim' / disabled).
        api.wait(600, 900).await;
        // Post-click probe: stop as soon as the state changes away from 'text'.
        if !api.iframe_has_text(MINIAPP_IFRAME, selector, text).await? {
            info!(
                "[{label}] button state changed away from '{text}' — done after {clicks} click(s)"
            );
            return Ok(true);
        }
        if clicks >= MAX_RETRY {
            info!("[{label}] max retries ({clicks}x) — moving on");
            return Ok(true);
        }
        api.wait(500, 1_000).await;
    }
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
