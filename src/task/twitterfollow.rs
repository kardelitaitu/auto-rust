use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::time::Duration;

use crate::prelude::TaskContext;
use crate::utils::math::random_in_range;
use crate::utils::mouse::{ClickOutcome, ClickStatus};
use crate::utils::twitter::{
    close_active_popup, twitteractivity_humanized::human_pause, twitteractivity_selectors::*,
};

const DEFAULT_NAVIGATE_TIMEOUT_MS: u64 = 30_000;
const MAX_ATTEMPTS: u32 = 5;
const POST_RELOAD_ATTEMPTS: u32 = 2;
const VERIFY_TIMEOUT_MS: u64 = 20_000;

/// Retry delay: base 3s + attempt*1s, with ±500ms jitter
fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(value) = payload.get("url") {
        if let Some(url_str) = value.as_str() {
            return Ok(normalize_url(url_str));
        }
    }
    if let Some(value) = payload.get("value") {
        if let Some(url_str) = value.as_str() {
            return Ok(normalize_url(url_str));
        }
    }
    // Check for default_url in payload
    if let Some(default_url) = payload.get("default_url") {
        if let Some(url_str) = default_url.as_str() {
            return Ok(normalize_url(url_str));
        }
    }
    for (key, val) in payload
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("payload not an object"))?
    {
        if key != "url" && key != "value" && key != "default_url" {
            if let Some(v) = val.as_str() {
                if !v.is_empty() && (v.contains("x.com") || v.contains("twitter.com")) {
                    return Ok(normalize_url(v));
                }
            }
        }
    }
    Err(anyhow::anyhow!("No URL found in payload"))
}

fn backoff_delay(attempt: u32) -> u64 {
    let base = 3000u64 + (attempt as u64) * 1000;
    let jitter = random_in_range(500, 1000);
    base + jitter
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let input_url = extract_url_from_payload(&payload)?;
    let username;

    if is_tweet_url(&input_url) {
        username = tweet_to_profile_flow(api, &input_url).await?;
    } else {
        username = extract_username_from_payload(&payload)?;
        let profile_url = format!("https://x.com/{}", username);
        info!("[twitterfollow] Starting: target=@{}", username);
        api.navigate(&profile_url, DEFAULT_NAVIGATE_TIMEOUT_MS)
            .await?;
        info!("[twitterfollow] Navigated to {}", profile_url);
    }

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

    loop {
        attempt += 1;

        // Exhausted pre-reload attempts → reload
        if attempt > MAX_ATTEMPTS {
            if has_reloaded || POST_RELOAD_ATTEMPTS == 0 {
                return Ok(false);
            }
            info!("[twitterfollow] Reloading page for retry...");
            api.navigate(
                &format!("https://x.com/{}", username),
                DEFAULT_NAVIGATE_TIMEOUT_MS,
            )
            .await?;
            human_pause(api, random_in_range(5000, 10000)).await;
            has_reloaded = true;
            continue;
        }

        info!(
            "[twitterfollow] Attempt {}/{}",
            attempt,
            MAX_ATTEMPTS
                + if has_reloaded {
                    POST_RELOAD_ATTEMPTS
                } else {
                    0
                }
        );

        // Wait for page to settle, scroll to make sure button is visible
        api.pause(500).await;
        api.scroll_to_top().await?;
        api.pause(500).await;

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

        // Locate and click follow button
        match find_and_click_follow_button(api).await {
            Ok(true) => {
                info!("[twitterfollow] Clicked follow button");
            }
            Ok(false) => {
                warn!("[twitterfollow] Follow button not visible");
                maybe_backoff(api, attempt).await;
                continue;
            }
            Err(e) => {
                warn!("[twitterfollow] Error clicking button: {}", e);
                maybe_backoff(api, attempt).await;
                continue;
            }
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
            var signals = [
                'rate limit',
                'too many attempts',
                'try again later',
                'you have been rate limited',
                'temporary restriction',
                'something went wrong',
                'unable to follow'
            ];
            for (var i = 0; i < signals.length; i++) {
                if (body.includes(signals[i])) {
                    return true;
                }
            }
            return false;
        })()
    "#;
    let result = api.page().evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v: &Value| v.as_bool())
        .unwrap_or(false))
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

async fn find_and_click_follow_button(api: &TaskContext) -> Result<bool> {
    let page = api.page();

    // Find follow button: role=button + aria-label="Follow @username" + span="Follow"
    let follow_js = r#"
        (function() {
            var buttons = document.querySelectorAll('[role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = btn.getAttribute('aria-label') || '';
                if (!ariaLabel.toLowerCase().startsWith('follow @')) continue;
                
                var spans = btn.querySelectorAll('span');
                for (var j = 0; j < spans.length; j++) {
                    var txt = (spans[j].textContent || spans[j].innerText || '').toLowerCase().trim();
                    if (txt === 'follow') {
                        var rect = btn.getBoundingClientRect();
                        if (rect.width > 0 && rect.height > 0) {
                            btn.click();
                            return true;
                        }
                    }
                }
            }
            return false;
        })()
    "#;
    let result = page.evaluate(follow_js).await?;
    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

async fn is_already_following(api: &TaskContext) -> Result<bool> {
    let page = api.page();

    // Check for multiple "already following" indicators:
    // 1. Unfollow button (aria-label starts with "unfollow")
    // 2. Following button (aria-label equals "following")
    // 3. Button with data-testid containing "-unfollow" (Twitter's following state)
    let js = r#"
        (function() {
            var buttons = document.querySelectorAll('[role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var btn = buttons[i];
                var ariaLabel = (btn.getAttribute('aria-label') || '').toLowerCase();
                var dataTestId = (btn.getAttribute('data-testid') || '').toLowerCase();

                // Check for Unfollow or Following aria-label
                if (ariaLabel.startsWith('unfollow') || ariaLabel === 'following') {
                    var rect = btn.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return true;
                    }
                }

                // Check for data-testid containing "-unfollow" (e.g., "1720490042765012992-unfollow")
                if (dataTestId.includes('-unfollow')) {
                    var rect = btn.getBoundingClientRect();
                    if (rect.width > 0 && rect.height > 0) {
                        return true;
                    }
                }
            }

            // Also check for span text "Following" inside a button with unfollow testid
            var allButtons = document.querySelectorAll('button, [role="button"]');
            for (var i = 0; i < allButtons.length; i++) {
                var testId = (allButtons[i].getAttribute('data-testid') || '').toLowerCase();
                if (testId.includes('-unfollow')) {
                    var spans = allButtons[i].querySelectorAll('span');
                    for (var j = 0; j < spans.length; j++) {
                        var txt = (spans[j].textContent || spans[j].innerText || '').trim().toLowerCase();
                        if (txt === 'following') {
                            var rect = allButtons[i].getBoundingClientRect();
                            if (rect.width > 0 && rect.height > 0) {
                                return true;
                            }
                        }
                    }
                }
            }

            return false;
        })()
    "#;
    let result = page.evaluate(js).await?;
    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

struct ButtonInfo {
    #[allow(dead_code)]
    x: f64,
    #[allow(dead_code)]
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
            let text = obj
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let label = obj
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
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

        api.pause(500).await;
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

fn normalize_url(url: &str) -> String {
    let mut url = url.trim().to_string();
    if !url.starts_with("http") {
        url = format!("https://{}", url);
    }
    url.replace("www.twitter.com/", "x.com/")
        .replace("www.x.com/", "x.com/")
}

fn is_tweet_url(url: &str) -> bool {
    let normalized = normalize_url(url);
    normalized.contains("/status/") && normalized.contains("x.com/")
}

fn extract_username_from_tweet_url(url: &str) -> Option<String> {
    let normalized = normalize_url(url);
    let path = normalized
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("x.com/");

    path.find("/status/").map(|idx| path[..idx].to_string())
}

async fn tweet_to_profile_flow(api: &TaskContext, tweet_url: &str) -> Result<String> {
    let username = extract_username_from_tweet_url(tweet_url)
        .ok_or_else(|| anyhow::anyhow!("Could not extract username from tweet URL"))?;

    info!("[twitterfollow] Navigating to tweet: {}", tweet_url);
    api.navigate(tweet_url, DEFAULT_NAVIGATE_TIMEOUT_MS).await?;

    api.pause(2000).await;

    info!("[twitterfollow] Scrolling down...");
    api.scroll_to_bottom().await?;
    api.pause(1000).await;

    info!("[twitterfollow] Scrolling up to top...");
    api.scroll_to_top().await?;
    api.pause(1000).await;

    info!("[twitterfollow] Clicking user avatar...");
    let avatar_click = click_tweet_avatar(api).await?;
    info!("[twitterfollow] Avatar {}", avatar_click.summary());

    api.pause(1500).await;

    let profile_url = format!("https://x.com/{}", username);
    info!("[twitterfollow] Navigated to profile: {}", profile_url);

    Ok(username)
}

/// Click on tweet user avatar using JS selector with fallback strategies
async fn click_tweet_avatar(api: &TaskContext) -> Result<ClickOutcome> {
    let js = selector_tweet_user_avatar();
    let result = api.page().evaluate(js).await?;

    if let Some(coords) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            coords.get("x").and_then(|v| v.as_f64()),
            coords.get("y").and_then(|v| v.as_f64()),
        ) {
            api.click_at(x, y).await?;
            // Return a synthetic ClickOutcome since click_at doesn't return one
            return Ok(ClickOutcome {
                click: ClickStatus::Success,
                x,
                y,
            });
        }
    }

    // Fallback to direct selector click
    api.click("[data-testid=\"Tweet-User-Avatar\"]").await
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_url_adds_https() {
        assert_eq!(normalize_url("x.com/foo"), "https://x.com/foo");
        assert_eq!(normalize_url("twitter.com/foo"), "https://twitter.com/foo");
    }

    #[test]
    fn test_normalize_url_handles_www() {
        assert_eq!(normalize_url("www.twitter.com/foo"), "https://x.com/foo");
        assert_eq!(normalize_url("www.x.com/foo"), "https://x.com/foo");
    }

    #[test]
    fn test_normalize_url_preserves_https() {
        assert_eq!(normalize_url("https://x.com/foo"), "https://x.com/foo");
    }

    #[test]
    fn test_is_tweet_url_detects_status() {
        assert!(is_tweet_url("https://x.com/user/status/123"));
        assert!(is_tweet_url("x.com/user/status/456"));
        assert!(!is_tweet_url("https://x.com/user"));
        assert!(!is_tweet_url("https://twitter.com/user"));
    }

    #[test]
    fn test_extract_username_from_tweet_url() {
        assert_eq!(
            extract_username_from_tweet_url("https://x.com/testuser/status/123"),
            Some("testuser".to_string())
        );
        assert_eq!(
            extract_username_from_tweet_url("x.com/anotheruser/status/456"),
            Some("anotheruser".to_string())
        );
        assert_eq!(extract_username_from_tweet_url("https://x.com/user"), None);
    }

    #[test]
    fn test_extract_username_from_payload_url() {
        let payload = json!({"url": "https://x.com/testuser"});
        assert_eq!(extract_username_from_payload(&payload).unwrap(), "testuser");
    }

    #[test]
    fn test_extract_username_from_payload_value() {
        let payload = json!({"value": "username123"});
        assert_eq!(
            extract_username_from_payload(&payload).unwrap(),
            "username123"
        );
    }

    #[test]
    fn test_extract_username_from_payload_username() {
        let payload = json!({"username": "myuser"});
        assert_eq!(extract_username_from_payload(&payload).unwrap(), "myuser");
    }

    #[test]
    fn test_extract_username_from_payload_fallback() {
        let payload = json!({"other_field": "fallback_user"});
        assert_eq!(
            extract_username_from_payload(&payload).unwrap(),
            "fallback_user"
        );
    }

    #[test]
    fn test_extract_username_from_payload_empty_fails() {
        let payload = json!({});
        assert!(extract_username_from_payload(&payload).is_err());
    }

    #[test]
    fn test_backoff_delay_increases_with_attempt() {
        let delay0 = backoff_delay(0);
        let delay1 = backoff_delay(1);
        let delay2 = backoff_delay(2);

        assert!(delay1 > delay0);
        assert!(delay2 > delay1);

        // Base delay is 3000ms + attempt*1000ms + jitter(500-1000ms)
        assert!((3500..=4000).contains(&delay0));
        assert!((4500..=5000).contains(&delay1));
        assert!((5500..=6000).contains(&delay2));
    }
}
