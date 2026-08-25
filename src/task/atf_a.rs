//! ATF airdrop mining task — Telegram A (web) bot automation.
//!
//! Clone of the `atf` task for the Telegram **A** web client. Navigates to a
//! direct chat on `web.telegram.org/a`, waits a short random interval for the
//! page to settle, then mouse-clicks the "Start Mining" bot command to begin a
//! mining session.
//!
//! # Flow
//! 1. Navigate to `https://web.telegram.org/a/#8233119648`
//! 2. Wait a random 5–10 seconds for the page to settle
//! 3. Mouse-click the "Start Mining" command button in composer
//! 4. Verify mini-app iframe is visible and wait for busy state to clear
//! 5. CLAIM action button (hourly miner) → click 1–3× with jitter
//! 6. Activate Tasks tab (with JS switchTab fallback)
//! 7. Click each "Go" task button (YouTube, Twitter, Website, Telegram)
//! 8. Click each "Claim" task button (YouTube, Twitter, Website, Telegram)
//! 9. Random Activities 1–5 (10% chance each to view Home/Friends/Profile/Miners tabs)
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

/// Default target URL — Telegram Web A client, direct chat with the ATF bot.
const DEFAULT_URL: &str = "https://web.telegram.org/a/#8233119648";

/// Default task runtime budget in milliseconds.
pub const DEFAULT_ATF_A_TASK_DURATION_MS: u64 = 180_000;

/// Navigation timeout for slow connections, in milliseconds.
const NAVIGATION_TIMEOUT_MS: u64 = 120_000;

/// How long to wait for the "Start Mining" button to appear, in milliseconds.
const BUTTON_VISIBILITY_TIMEOUT_MS: u64 = 45_000;

/// Telegram A client: the bot-command (Start Mining) button in the composer.
const START_MINING_SELECTOR: &str = "#MiddleColumn > div.messages-layout > div.Transition > div > div.middle-column-footer > div.Composer.is-chat-composer.shown.mounted > div.composer-wrapper > div > button.Button.composer-action-button.bot-menu.open.default.translucent.round";

/// Telegram A client: the mini-app iframe lives in the browser modal under
/// `#portals` (the K client used a stable `iframe.payment-verification` class;
/// the A client's iframe has only a generated class, so the DOM path is used).
const MINIAPP_IFRAME: &str = "#portals > div:nth-child(4) > div > div > div.modal-dialog.browser-modal-dialog > div.modal-content.custom-scroll > div > iframe";

/// Scans the mini-app state: busy modal/text, tasks tab active, and the
/// visible Go/Claim task buttons.
const STATE_SCAN_JS: &str = r#"(() => {
    const q = s => document.querySelector(s);
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
    const busyModalText = busyEl ? (busyEl.innerText || '') : '';
    return JSON.stringify({
        claimAction: !!claimAction && visible(claimAction) && text(claimAction) === 'CLAIM',
        tasksActive: !!tasksTab && tasksTab.classList.contains('active'),
        goButtons: btns.filter(b => text(b) === 'Go').map(b => b.id),
        claimButtons: btns.filter(b => text(b) === 'Claim').map(b => b.id),
        busyModal: !!busyEl,
        busyModalId: busyEl ? (busyEl.id || busyEl.className || busyEl.tagName).toString().slice(0, 80) : '',
        busyText: /Processing|Please wait/i.test(busyModalText)
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
    info!("[Step 1] Navigating to: {url}");
    api.navigate(url, NAVIGATION_TIMEOUT_MS).await?;

    // Step 2: wait 5–10 seconds for the Telegram UI to settle on slow connections.
    info!("[Step 2] Waiting random 5-10s for Telegram page to settle");
    api.wait(5_000, 10_000).await;

    // Step 3: make sure the "Start Mining" command button is present, then
    // mouse-click it with the native cursor.
    if !api
        .wait_for_visible(START_MINING_SELECTOR, BUTTON_VISIBILITY_TIMEOUT_MS)
        .await?
    {
        warn!(
            "[Step 3] Start Mining button did not appear within {BUTTON_VISIBILITY_TIMEOUT_MS}ms"
        );
    }
    let _ = api.focus_tab().await;
    info!("[Step 3] Mouse-clicking Start Mining");
    let outcome = api.click(START_MINING_SELECTOR).await?;
    info!("[Step 3] Click result: {}", outcome.summary());
    if !matches!(
        outcome.click,
        crate::utils::mouse::types::ClickStatus::Success
    ) {
        bail!("[Step 3] Start Mining click failed: {}", outcome.summary());
    }

    api.wait(2_000, 4_000).await;

    // Step 3b: Ensure the mini-app iframe is loaded and ready.
    info!("[Step 3b] Waiting for mini-app iframe to load . . .");
    if !api
        .wait_for_visible(MINIAPP_IFRAME, BUTTON_VISIBILITY_TIMEOUT_MS)
        .await?
    {
        warn!(
            "[Step 3b] Mini-app iframe '{MINIAPP_IFRAME}' did not appear within {BUTTON_VISIBILITY_TIMEOUT_MS}ms"
        );
    }
    wait_not_busy(api).await?;

    // Step 4a: click the "Claim hourly miner" CLAIM action button 1-3 times
    // with a small random offset each time.
    // Step 4a: click the "Claim hourly miner" CLAIM action button 2-4 times
    // with a small random offset each time (not the exact same position),
    // random 0.5-1s between clicks. No verification.
    // (the button is `<button class="btn-main state-claim" id="actionBtn">CLAIM</button>`)
    let _ = api.focus_tab().await;
    info!("[Step 4a] Starting . . .");
    if let Some((cx, cy)) = api
        .iframe_center(
            MINIAPP_IFRAME,
            ".btn-main.state-claim#actionBtn, .state-claim",
            3_000,
        )
        .await?
    {
        let clicks = random_in_range(1, 3);
        info!("[Step 4a] Claim button found, clicking {clicks}x");
        for i in 0..clicks {
            let jx = random_in_range(0, 16) as f64 - 8.0;
            let jy = random_in_range(0, 16) as f64 - 8.0;
            let _ = api.focus_tab().await;
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

    // [Random Activity 1] 10% chance to click Home tab and view it briefly.
    if random_in_range(1, 100) <= 10 {
        info!("[Random Activity 1] 10% chance triggered — clicking Home tab");
        let _ = switch_tab(api, "home").await;
        api.wait(2_000, 4_000).await;
    }

    // [Random Activity 2] 10% chance to click Friends tab and view it briefly.
    if random_in_range(1, 100) <= 10 {
        info!("[Random Activity 2] 10% chance triggered — clicking Friends tab");
        let _ = switch_tab(api, "friends").await;
        api.wait(2_000, 4_000).await;
    }

    // Step 4b: activate the Tasks tab. Try coordinate click first, with fallback to JS switchTab.
    info!("[Step 4b] Opening Tasks tab");
    switch_tab(api, "tasks").await?;
    api.wait(1_000, 2_000).await;
    ensure_tasks_visible(api).await?;

    // Steps 5-8: click each "Go" task button.
    click_task_button(
        api,
        ".btn-small#btn-youtube_like_comment, #btn-youtube_like_comment, [id*='youtube']",
        "Go",
        "[Step 5]",
    )
    .await?;
    click_task_button(
        api,
        ".btn-small#btn-twitter_retweet, #btn-twitter_retweet, [id*='twitter'], [id*='retweet']",
        "Go",
        "[Step 6]",
    )
    .await?;
    click_task_button(
        api,
        ".btn-small#btn-website_visit, #btn-website_visit, [id*='website'], [id*='visit']",
        "Go",
        "[Step 7]",
    )
    .await?;
    click_task_button(
        api,
        ".btn-small#btn-telegram_react_latest, #btn-telegram_react_latest, [id*='telegram'], [id*='react']",
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
        ".btn-small#btn-youtube_like_comment, #btn-youtube_like_comment, [id*='youtube']",
        "Claim",
        "[Step 9]",
    )
    .await?;
    click_task_button(
        api,
        ".btn-small#btn-twitter_retweet, #btn-twitter_retweet, [id*='twitter'], [id*='retweet']",
        "Claim",
        "[Step 10]",
    )
    .await?;
    click_task_button(
        api,
        ".btn-small#btn-website_visit, #btn-website_visit, [id*='website'], [id*='visit']",
        "Claim",
        "[Step 11]",
    )
    .await?;
    click_task_button(
        api,
        ".btn-small#btn-telegram_react_latest, #btn-telegram_react_latest, [id*='telegram'], [id*='react']",
        "Claim",
        "[Step 12]",
    )
    .await?;

    // [Random Activity 3] 10% chance to click Profile tab and view it briefly.
    if random_in_range(1, 100) <= 10 {
        info!("[Random Activity 3] 10% chance triggered — clicking Profile tab");
        let _ = switch_tab(api, "profile").await;
        api.wait(2_000, 4_000).await;
    }

    // [Random Activity 4] 10% chance to click Home tab and view it briefly.
    if random_in_range(1, 100) <= 10 {
        info!("[Random Activity 4] 10% chance triggered — clicking Home tab");
        let _ = switch_tab(api, "home").await;
        api.wait(2_000, 4_000).await;
    }

    // [Random Activity 5] 10% chance to click Miners tab and view it briefly.
    if random_in_range(1, 100) <= 10 {
        info!("[Random Activity 5] 10% chance triggered — clicking Miners tab");
        let _ = switch_tab(api, "miners").await;
        api.wait(2_000, 4_000).await;
    }

    info!("Finalizing Tasks");
    api.wait(500, 3_000).await;

    info!("ATF-A task completed");
    Ok(())
}

/// Activate a tab in the ATF mini-app by tab name (e.g., "tasks", "home", "friends", "profile", "miners").
/// First attempts fast DOM clicking with flexible selector variants (1.5s timeout per selector),
/// then falls back to `window.switchTab(name)` via JS.
async fn switch_tab(api: &TaskContext, tab_name: &str) -> Result<()> {
    let _ = api.focus_tab().await;

    let selectors = [
        format!("[onclick*=\"switchTab('{tab_name}')\"]"),
        format!("[onclick*=\"switchTab(\\\"{tab_name}\\\")\"]"),
        format!("[onclick*=\"{tab_name}\"]"),
        format!("[data-tab=\"{tab_name}\"]"),
        format!("#tab-{tab_name}"),
        format!(".tab-{tab_name}"),
    ];

    let mut clicked = false;
    for sel in &selectors {
        if let Ok(outcome) = api.iframe_click(MINIAPP_IFRAME, sel, 1_500).await {
            info!(
                "[tab] Activated '{tab_name}' tab via selector '{sel}': {}",
                outcome.summary()
            );
            clicked = true;
            break;
        }
    }

    if !clicked {
        let js = format!(
            "(() => {{ if (typeof window.switchTab === 'function') {{ window.switchTab('{tab_name}'); return true; }} return false; }})()"
        );
        match api.iframe_eval(MINIAPP_IFRAME, &js, 3_000).await {
            Ok(_) => info!("[tab] Activated '{tab_name}' tab via JS switchTab('{tab_name}')"),
            Err(e) => warn!("[tab] Failed to switch tab to '{tab_name}': {e}"),
        }
    }

    Ok(())
}

/// Wait for a modal/"Processing" busy state to clear (up to ~20s).
async fn wait_not_busy(api: &TaskContext) -> Result<()> {
    let start = std::time::Instant::now();
    loop {
        let raw = match api.iframe_eval(MINIAPP_IFRAME, STATE_SCAN_JS, 10_000).await {
            Ok(v) => v,
            Err(_) => break,
        };
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
        let raw = match api.iframe_eval(MINIAPP_IFRAME, STATE_SCAN_JS, 10_000).await {
            Ok(v) => v,
            Err(_) => {
                api.wait(1_000, 2_000).await;
                continue;
            }
        };
        let value = match raw {
            Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),
            other => other,
        };
        let tasks_active = value["tasksActive"].as_bool().unwrap_or(false);
        let go_buttons = value["goButtons"].as_array().cloned().unwrap_or_default();
        let claim_buttons = value["claimButtons"].as_array().cloned().unwrap_or_default();
        info!(
            "[tasks] Scan attempt {}: active={tasks_active}, goButtons={:?}, claimButtons={:?}",
            attempt + 1,
            go_buttons,
            claim_buttons
        );
        if tasks_active && (!go_buttons.is_empty() || !claim_buttons.is_empty()) {
            return Ok(());
        }
        if !tasks_active {
            info!(
                "[tasks] Tasks tab not active (attempt {}), invoking switchTab('tasks')",
                attempt + 1
            );
            let _ = switch_tab(api, "tasks").await;
        }
        api.wait(1_000, 2_000).await;
    }
    info!("[tasks] tasks scan completed — proceeding to task steps");
    Ok(())
}

/// Click a task button (by selector + exact text) until its state changes.
///
/// Robust per-step helper: directly tries `iframe_click_text` with a 3s poll
/// timeout so async DOM rendering never trips an instant false skip.
async fn click_task_button(
    api: &TaskContext,
    selector: &str,
    text: &str,
    label: &str,
) -> Result<bool> {
    const MAX_RETRY: u32 = 3;
    const TIMEOUT_MS: u64 = 3_000;
    wait_not_busy(api).await?;
    let mut clicks = 0u32;
    loop {
        let _ = api.focus_tab().await;
        let outcome = match api
            .iframe_click_text(MINIAPP_IFRAME, selector, text, TIMEOUT_MS)
            .await
        {
            Ok(o) => o,
            Err(e) => {
                if clicks == 0 {
                    info!("[{label}] skipped — no '{text}' button: {e}");
                } else {
                    info!("[{label}] done after {clicks} click(s) — button state changed away from '{text}'");
                }
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
        api.wait(600, 1_000).await;
        // Close any spawned tabs (e.g. YouTube / Twitter / Web) and return focus
        let _ = api.focus_tab().await;
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
