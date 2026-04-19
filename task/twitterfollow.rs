use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

use crate::prelude::TaskContext;
use crate::utils::math::random_in_range;
use crate::utils::mouse::GhostCursor;
use crate::utils::twitter::{
    close_active_popup,
    twitteractivity_selectors::*,
    twitteractivity_humanized::human_pause,
};

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const MAX_ATTEMPTS: u32 = 5;
const POST_RELOAD_ATTEMPTS: u32 = 2;
const VERIFY_TIMEOUT_MS: u64 = 20_000;

/// Retry delay: base 3s + attempt*1s, with ±500ms jitter
fn backoff_delay(attempt: u32) -> u64 {
    let base = 3000 + attempt * 1000;
    let jitter = random_in_range(0, 1000) - 500; // -500 to +500
    (base as i64 + jitter as i64).max(500) as u64 // min 500ms
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let username = extract_username_from_payload(&payload)?;
    let profile_url = format!("https://x.com/{}", username);

    info!("[twitterfollow] Starting: target=@{}", username);

    api.navigate(&profile_url, DEFAULT_NAVIGATE_TIMEOUT_MS).await?;
    info!("[twitterfollow] Navigated to {}", profile_url);

    verify_current_profile(api, &username).await?;

    // Already following pre-check
    if is_already_following(api).await? {
        info!("[twitterfollow] Already following @{}", username);
        return Ok(());
    }

    // Humanized read delay
    human_pause(api, random_in_range(8000, 15000)).await;

    let followed = robust_follow(api, &username).await?;

    if followed {
        info!("[twitterfollow] ✅ Successfully followed @{}", username);
    } else {
        info!("[twitterfollow] ℹ️ No action needed");
    }

    info!("[twitterfollow] Task complete");
    Ok(())
}

async fn robust_follow(api: &TaskContext, username: &str) -> Result<bool> {
    let mut attempt = 0u32;
    let mut has_reloaded = false;
    let mut ghost = GhostCursor::new(api.page_arc());

    loop {
        attempt += 1;

        // Exhausted pre-reload attempts → reload
        if attempt > MAX_ATTEMPTS {
            if has_reloaded || POST_RELOAD_ATTEMPTS == 0 {
                return Ok(false);
            }
            info!("[twitterfollow] Reloading page for retry...");
            api.navigate(&format!("https://x.com/{}", username), DEFAULT_NAVIGATE_TIMEOUT_MS).await?;
            human_pause(api, random_in_range(5000, 10000)).await;
            has_reloaded = true;
            continue;
        }

        info!("[twitterfollow] Attempt {}/{}",
            attempt,
            MAX_ATTEMPTS + if has_reloaded { POST_RELOAD_ATTEMPTS } else { 0 });

        // Dismiss overlays
        let _ = api.press("Escape").await;
        let _ = close_active_popup(api).await;

        // Soft error check
        if check_soft_error(api).await? {
            human_pause(api, random_in_range(5000, 8000)).await;
            continue;
        }

        // Pre-check: already following (includes pending handling)
        if handle_pending_state(api, username).await? {
            return Ok(true);
        }

        // Locate button
        let coords = match find_follow_button_coords(api).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                warn!("[twitterfollow] Follow button not visible");
                maybe_backoff(api, attempt).await;
                continue;
            }
            Err(e) => {
                warn!("[twitterfollow] Error locating button: {}", e);
                maybe_backoff(api, attempt).await;
                continue;
            }
        };

        // Safety: button still says "Follow"?
        if !button_says_follow(api).await? {
            info!("[twitterfollow] Button state changed to following");
            return Ok(true);
        }

        // Check actionability (not covered by overlay)
        if !is_element_actionable(api, coords.0, coords.1).await? {
            warn!("[twitterfollow] Button not actionable (possibly covered), retrying...");
            maybe_backoff(api, attempt).await;
            continue;
        }

        // Click with GhostCursor robustness
        if let Err(e) = ghost.click_at(coords.0, coords.1).await {
            warn!("[twitterfollow] GhostCursor click failed: {}", e);
            maybe_backoff(api, attempt).await;
            continue;
        }

        info!("[twitterfollow] Click successful, verifying...");

        // Verify
        match poll_for_follow_success(api).await {
            Ok(true) => {
                info!("[twitterfollow] ✅ Follow verified");
                return Ok(true);
            }
            Ok(false) => {
                warn!("[twitterfollow] Follow not verified");
            }
            Err(e) => {
                warn!("[twitterfollow] Verification error: {}", e);
            }
        }

        maybe_backoff(api, attempt).await;
    }
}

// ---- Helper functions (unchanged) ----

/// Check for soft errors (rate limits, suspended, etc.)
async fn check_soft_error(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var body = document.body.innerText.toLowerCase();
            if (body.includes('rate limit') || body.includes('too many attempts')) {
                return true;
            }
            return false;
        })()
    "#;
    let result = api.page().evaluate(js).await?;
    Ok(result.value().and_then(|v: &Value| v.as_bool()).unwrap_or(false))
}

/// Handle pending state: if button says "pending", wait 3s and re-check
async fn handle_pending_state(api: &TaskContext, _username: &str) -> Result<bool> {
    if is_already_following(api).await? {
        return Ok(true);
    }
    let info = match get_follow_button_info(api).await? {
        Some(i) => i,
        None => return Ok(false),
    };
    let txt = info.text.to_lowercase();
    if txt.contains("pending") {
        info!("[twitterfollow] Button in 'pending' state, waiting 3s...");
        human_pause(api, 3000).await;
        if is_already_following(api).await? {
            info!("[twitterfollow] Pending resolved to following");
            return Ok(true);
        }
    }
    Ok(false)
}

/// Backoff delay before next attempt
async fn maybe_backoff(api: &TaskContext, attempt: u32) {
    if attempt < MAX_ATTEMPTS {
        let delay = backoff_delay(attempt);
        human_pause(api, delay).await;
    }
}

/// Check if element center point is not covered by another element
async fn is_element_actionable(api: &TaskContext, x: f64, y: f64) -> Result<bool> {
    let js = format!(
        r#"
        (function() {{
            var el = document.elementFromPoint({}, {});
            if (!el) return false;
            var rect = el.getBoundingClientRect();
            var centerX = rect.left + rect.width / 2;
            var centerY = rect.top + rect.height / 2;
            var dist = Math.sqrt(Math.pow({} - centerX, 2) + Math.pow({} - centerY, 2));
            return dist < 5;
        }})()
        "#,
        x, y, x, y
    );
    let result = api.page().evaluate(js).await?;
    Ok(result.value().and_then(|v: &Value| v.as_bool()).unwrap_or(false))
}

async fn is_already_following(api: &TaskContext) -> Result<bool> {
    match get_follow_button_info(api).await {
        Ok(Some(info)) => {
            let t = info.text.to_lowercase();
            let l = info.label.to_lowercase();
            Ok(t.contains("following") || t.contains("unfollow") || l.contains("following") || l.contains("unfollow"))
        }
        _ => Ok(false),
    }
}

async fn find_follow_button_coords(api: &TaskContext) -> Result<Option<(f64, f64)>> {
    match get_follow_button_info(api).await {
        Ok(Some(info)) => Ok(Some((info.x, info.y))),
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

async fn button_says_follow(api: &TaskContext) -> Result<bool> {
    match get_follow_button_info(api).await {
        Ok(Some(info)) => {
            let t = info.text.to_lowercase();
            let l = info.label.to_lowercase();
            Ok(!(t.contains("following") || t.contains("unfollow") || l.contains("following") || l.contains("unfollow")))
        }
        Ok(None) => Ok(false),
        Err(e) => Err(e),
    }
}

struct ButtonInfo {
    x: f64,
    y: f64,
    text: String,
    label: String,
}

async fn get_follow_button_info(api: &TaskContext) -> Result<Option<ButtonInfo>> {
    let js = selector_follow_button();
    let result = api.page().evaluate(js).await?;
    let value = result.value();
    if let Some(obj) = value.and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
            let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let label = obj.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
            return Ok(Some(ButtonInfo { x, y, text, label }));
        }
    }
    Ok(None)
}

async fn poll_for_follow_success(api: &TaskContext) -> Result<bool> {
    let deadline = std::time::Instant::now() + Duration::from_millis(VERIFY_TIMEOUT_MS);
    loop {
        // Check 1: following indicator (unfollow button)
        let js_unfollow = selector_following_indicator();
        let result = api.page().evaluate(js_unfollow.to_string()).await?;
        if let Some(true) = result.value().and_then(|v: &Value| v.as_bool()) {
            return Ok(true);
        }

        // Check 2: follow button text changed to "following"
        if check_follow_button_says_following(api).await? {
            return Ok(true);
        }

        if std::time::Instant::now() >= deadline {
            return Ok(false);
        }

        sleep(Duration::from_millis(500)).await;
    }
}

async fn check_follow_button_says_following(api: &TaskContext) -> Result<bool> {
    match get_follow_button_info(api).await {
        Ok(Some(info)) => {
            let t = info.text.to_lowercase();
            let l = info.label.to_lowercase();
            Ok(t.contains("following") || l.contains("following"))
        }
        Ok(None) => Ok(false),
        Err(e) => Err(e),
    }
}

async fn verify_current_profile(api: &TaskContext, expected: &str) -> Result<()> {
    let js = r#"window.location.pathname"#;
    let result = api.page().evaluate(js.to_string()).await?;
    if let Some(path) = result.value().and_then(|v: &Value| v.as_str()) {
        let clean = path.trim_matches('/');
        if clean == expected || clean.to_lowercase() == expected.to_lowercase() {
            return Ok(());
        }
        anyhow::bail!("Profile mismatch: expected '{}', got '{}'", expected, path);
    }
    anyhow::bail!("Could not read current URL pathname");
}

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
        if let Some(v) = value.as_str() {
            return Ok(v.to_string());
        }
    }
    if let Some(value) = payload.get("username") {
        if let Some(v) = value.as_str() {
            return Ok(v.to_string());
        }
    }
    if let Some(obj) = payload.as_object() {
        for (_, val) in obj {
            if let Some(v) = val.as_str() {
                if !v.is_empty() {
                    return Ok(v.to_string());
                }
            }
        }
    }
    anyhow::bail!("No username found in payload");
}

