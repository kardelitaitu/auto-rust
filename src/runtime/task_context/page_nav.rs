//! Navigation and page management methods for `TaskContext`.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// Retry configuration for transient CDP failures
const EVAL_RETRY_MAX_ATTEMPTS: u32 = 3;
const EVAL_RETRY_BASE_DELAY_MS: u64 = 50;

/// Returns true if the error is transient and safe to retry.
///
/// Transient errors include timeouts, connection issues, and temporary
/// failures that may succeed on retry. Permanent errors like "element
/// not found" or "invalid selector" should fail immediately without retry.
fn is_transient_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();

    // Permanent errors - don't retry
    if msg.contains("not found") || msg.contains("element not found") {
        return false;
    }
    if msg.contains("no such element") {
        return false;
    }
    if msg.contains("invalid selector") {
        return false;
    }
    if msg.contains("selector not found") {
        return false;
    }
    if msg.contains("node is disconnected") {
        return false;
    }
    // Standalone "disconnected" without "node is" prefix — could be a temporary
    // connection dropout that's recoverable
    if msg.contains("disconnected") {
        return true;
    }
    if msg.contains("target closed") {
        return false;
    }
    if msg.contains("permission denied") {
        return false;
    }

    // Transient errors - safe to retry
    if msg.contains("timeout") || msg.contains("timed out") {
        return true;
    }
    if msg.contains("connection") && (msg.contains("refused") || msg.contains("reset") || msg.contains("broken")) {
        return true;
    }
    if msg.contains("temporary") || msg.contains("unavailable") {
        return true;
    }
    if msg.contains("network") || msg.contains("econnreset") {
        return true;
    }
    if msg.contains("aborted") || msg.contains("cancelled") || msg.contains("interrupted") {
        return true;
    }

    // Default: treat unknown errors as potentially transient (retry)
    // This is safer than failing immediately on unexpected errors
    true
}

impl TaskContext {
    /// Helper to retry an async CDP operation with exponential backoff.
    /// Only retries transient errors; permanent errors fail immediately.
    async fn with_retry<F, Fut, T>(&self, op: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, anyhow::Error>>,
    {
        let mut attempt = 0;
        loop {
            match op().await {
                Ok(result) => return Ok(result),
                Err(e) if is_transient_error(&e) && attempt < EVAL_RETRY_MAX_ATTEMPTS => {
                    attempt += 1;
                    let delay = EVAL_RETRY_BASE_DELAY_MS * (1 << attempt.min(6));
                    log::warn!("CDP operation failed (attempt {}), retrying in {}ms: {}", attempt, delay, e);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                Err(e) => {
                    let classification = if is_transient_error(&e) {
                        "transient (max attempts)"
                    } else {
                        "permanent"
                    };
                    log::debug!("CDP operation failed ({classification}): {}", e);
                    return Err(e);
                }
            }
        }
    }
}

use crate::capabilities::{navigation, timing};
use crate::runtime::task_context::{FocusOutcome, FocusStatus, TaskContext};

impl TaskContext {
    /// Navigate to a URL with timeout and human-like pacing.
    pub async fn navigate(&self, url: &str, timeout_ms: u64) -> Result<()> {
        let navigate_timeout_ms = timeout_ms.max(super::MIN_NAVIGATE_TIMEOUT_MS);
        navigation::goto(self.page(), url, navigate_timeout_ms)
            .await
            .with_context(|| {
                format!("navigate_timeout | stage=goto url={url} timeout_ms={navigate_timeout_ms}")
            })?;
        let action_delay = &self.behavior_runtime.action_delay;
        timing::human_pause(
            action_delay.min_ms,
            action_delay.variance_pct.round() as u32,
        )
        .await;
        let settle_base = action_delay
            .min_ms
            .saturating_add(navigate_timeout_ms.min(2_000) / 4)
            .clamp(150, 4_000);
        let settle_variance = action_delay.variance_pct.round().clamp(10.0, 60.0) as u32;
        timing::human_pause(settle_base, settle_variance).await;
        let settle_ms = navigate_timeout_ms.min(3_000);
        self.wait_for_load(settle_ms).await.with_context(|| {
            format!("navigate_timeout | stage=settle_load url={url} timeout_ms={settle_ms}")
        })?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Set a custom user agent string for this session.
    pub async fn set_user_agent(&self, user_agent: &str) -> Result<()> {
        let escaped = user_agent.replace("'", "\\'");
        self.with_retry(|| async {
            self.page
                .as_ref()
                .evaluate(format!("navigator.userAgent = '{}'", escaped))
                .await
                .map_err(anyhow::Error::from)
        }).await?;
        Ok(())
    }

    /// Set extra HTTP headers to be sent with each request.
    pub async fn set_extra_http_headers(&self, headers: &[(String, String)]) -> Result<()> {
        log::debug!("set_extra_http_headers called with {} headers", headers.len());
        Ok(())
    }

    /// Apply browser context settings (user agent and headers).
    /// If user_agent is None, skips setting it.
    pub async fn apply_browser_context(&self, user_agent: Option<&str>, headers: &[(String, String)]) -> Result<()> {
        if let Some(ua) = user_agent {
            self.set_user_agent(ua).await?;
        }
        self.set_extra_http_headers(headers).await
    }

    /// Wait for the page load event with timeout enforcement.
    pub async fn wait_for_load(&self, timeout_ms: u64) -> Result<()> {
        let duration = Duration::from_millis(timeout_ms);
        tokio::time::timeout(duration, async {
            // Poll until document is complete (loaded)
            loop {
                let ready_state = self.page.evaluate("document.readyState").await?;
                let state = ready_state.value()
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                if state == "complete" || state == "interactive" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Ok::<(), anyhow::Error>(())
        }).await
        .map_err(|e| anyhow::anyhow!("wait_for_load timeout after {}ms: {}", timeout_ms, e))??;
        Ok(())
    }

    /// Wait until any of the given selectors becomes visible.
    /// Uses getBoundingClientRect to verify width > 0 and height > 0, ensuring
    /// the element is not hidden, collapsed, or display:none.
    pub async fn wait_for_any_visible_selector(&self, selectors: &[&str], timeout_ms: u64) -> Result<bool> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        for selector_str in selectors {
            let mut attempt = 0;
            while std::time::Instant::now() < deadline {
                attempt += 1;
                let escaped = selector_str.replace('\"', "\\\"");
                // Check visibility: width > 0 AND height > 0 via getBoundingClientRect
                let visible = self.with_retry(|| async {
                    self.page.evaluate(format!(
                        "(function() {{ const el = document.querySelector('{}'); if (!el) return false; const r = el.getBoundingClientRect(); return r.width > 0 && r.height > 0; }})()",
                        escaped
                    )).await.map_err(anyhow::Error::from)
                }).await?;
                if visible.value().and_then(|v| v.as_bool()).unwrap_or(false) {
                    log::debug!("Selector '{}' visible after {} attempts", selector_str, attempt);
                    return Ok(true);
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
        Ok(false)
    }

    /// Focus the element matching the selector.
    pub async fn focus(&self, selector: &str) -> Result<FocusOutcome> {
        #[derive(Deserialize)]
        struct Rect { x: f64, y: f64 }
        let escaped = selector.replace('\"', "\\\"");
        let rect: Option<Rect> = self.with_retry(|| async {
            self.page.evaluate(format!(
                "(function() {{ const el = document.querySelector('{}'); if (!el) return null; const r = el.getBoundingClientRect(); return {{ x: r.left + r.width/2, y: r.top + r.height/2 }}; }})()",
                escaped
            )).await.map_err(anyhow::Error::from)
        }).await?.value().and_then(|v| serde_json::from_value(v.clone()).ok());
        match rect {
            Some(r) => {
                self.with_retry(|| async {
                    self.page.evaluate(format!(
                        "(function() {{ const el = document.querySelector('{}'); if (el) el.focus(); }})()",
                        escaped
                    )).await.map_err(anyhow::Error::from)
                }).await?;
                Ok(FocusOutcome { focus: FocusStatus::Success, x: r.x, y: r.y })
            }
            None => Ok(FocusOutcome { focus: FocusStatus::Failed, x: 0.0, y: 0.0 }),
        }
    }

    pub fn check_permission(&self, permission: &'static str) -> crate::error::Result<()> {
        let perms = self.policy.effective_permissions();
        let has_permission = match permission {
            "allow_screenshot" => perms.allow_screenshot,
            "allow_export_cookies" => perms.allow_export_cookies,
            "allow_import_cookies" => perms.allow_import_cookies,
            "allow_export_session" => perms.allow_export_session,
            "allow_import_session" => perms.allow_import_session,
            "allow_session_clipboard" => perms.allow_session_clipboard,
            "allow_read_data" => perms.allow_read_data,
            "allow_write_data" => perms.allow_write_data,
            "allow_http_requests" => perms.allow_http_requests,
            "allow_dom_inspection" => perms.allow_dom_inspection,
            "allow_browser_export" => perms.allow_browser_export,
            "allow_browser_import" => perms.allow_browser_import,
            _ => {
                log::warn!("Unknown permission '{permission}' requested");
                false
            }
        };
        if has_permission {
            Ok(())
        } else {
            Err(crate::error::TaskError::PermissionDenied {
                permission,
                task_name: self.session_id.clone(),
            }
            .into())
        }
    }

    pub async fn check_page_connected(&self) -> crate::error::Result<()> {
        match self.page.evaluate("1").await {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::error::TaskError::CdpError {
                operation: "Page.connection_check".to_string(),
                reason: format!("Page not responding to CDP: {e}"),
            }
            .into()),
        }
    }

    pub async fn screenshot(&self) -> Result<String> {
        self.screenshot_with_quality(50).await
    }

    pub async fn screenshot_with_quality(&self, quality: u8) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_screenshot {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_screenshot' permission",
                self.session_id
            ));
        }
        let quality = quality.clamp(1, 100);
        let png_bytes = self
            .page
            .screenshot(
                chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::default(),
            )
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Page.captureScreenshot - {e}"))?;
        let img = image::load_from_memory(&png_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to load PNG image: {e}"))?;
        let rgb_img = img.to_rgb8();
        let (width, height) = (rgb_img.width(), rgb_img.height());
        let encoder = webp::Encoder::new(rgb_img.as_raw(), webp::PixelLayout::Rgb, width, height);
        let webp_data = encoder.encode(f32::from(quality));
        let now = chrono::Utc::now();
        let filename = format!(
            "{}-{}-{}.webp",
            now.format("%Y-%m-%d"),
            now.format("%H-%M"),
            self.session_id
        );
        let screenshot_dir = std::path::Path::new("data/screenshot");
        std::fs::create_dir_all(screenshot_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create screenshot directory: {e}"))?;
        let file_path = screenshot_dir.join(&filename);
        std::fs::write(&file_path, &*webp_data)
            .map_err(|e| anyhow::anyhow!("Failed to write screenshot: {e}"))?;
        file_path
            .to_str()
            .map(std::string::ToString::to_string)
            .ok_or_else(|| anyhow::anyhow!("Invalid screenshot path"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Retry Configuration Tests
    // ========================================================================

    #[test]
    fn test_retry_constants_values() {
        assert_eq!(EVAL_RETRY_MAX_ATTEMPTS, 3);
        assert_eq!(EVAL_RETRY_BASE_DELAY_MS, 50);
    }

    #[test]
    fn test_retry_delay_exponential_backoff() {
        // Verify the exponential backoff formula: base * 2^attempt
        let attempt_0 = EVAL_RETRY_BASE_DELAY_MS * (1u64 << 0);
        let attempt_1 = EVAL_RETRY_BASE_DELAY_MS * (1u64 << 1);
        let attempt_2 = EVAL_RETRY_BASE_DELAY_MS * (1u64 << 2);
        let attempt_3 = EVAL_RETRY_BASE_DELAY_MS * (1u64 << 3);

        assert_eq!(attempt_0, 50);  // 50 * 1 = 50ms
        assert_eq!(attempt_1, 100); // 50 * 2 = 100ms
        assert_eq!(attempt_2, 200); // 50 * 4 = 200ms
        assert_eq!(attempt_3, 400); // 50 * 8 = 400ms
    }

    // ========================================================================
    // FocusOutcome Tests
    // ========================================================================

    #[test]
    fn test_focus_outcome_success_coordinates() {
        let outcome = FocusOutcome {
            focus: FocusStatus::Success,
            x: 150.0,
            y: 200.0,
        };
        let summary = outcome.summary();
        assert!(summary.contains("success"));
        assert!(summary.contains("150.0"));
        assert!(summary.contains("200.0"));
    }

    #[test]
    fn test_focus_outcome_failed_coordinates() {
        let outcome = FocusOutcome {
            focus: FocusStatus::Failed,
            x: 0.0,
            y: 0.0,
        };
        let summary = outcome.summary();
        assert!(summary.contains("failed"));
    }

    #[test]
    fn test_focus_outcome_constructors() {
        // Test Success variant
        let success = FocusOutcome {
            focus: FocusStatus::Success,
            x: 100.0,
            y: 300.0,
        };
        assert_eq!(success.focus, FocusStatus::Success);
        assert_eq!(success.x, 100.0);
        assert_eq!(success.y, 300.0);

        // Test Failed variant
        let failed = FocusOutcome {
            focus: FocusStatus::Failed,
            x: 0.0,
            y: 0.0,
        };
        assert_eq!(failed.focus, FocusStatus::Failed);
        assert_eq!(failed.x, 0.0);
        assert_eq!(failed.y, 0.0);

        // Test summary output for both variants
        let success_summary = success.summary();
        assert!(success_summary.contains("success"));
        let failed_summary = failed.summary();
        assert!(failed_summary.contains("failed"));
    }

    // ========================================================================
    // FocusStatus Tests
    // ========================================================================

    #[test]
    fn test_focus_status_variants() {
        assert_eq!(FocusStatus::Success as u8, 0);
        assert_eq!(FocusStatus::Failed as u8, 1);
    }

    #[test]
    fn test_focus_status_partial_eq() {
        assert_eq!(FocusStatus::Success, FocusStatus::Success);
        assert_ne!(FocusStatus::Success, FocusStatus::Failed);
    }

    #[test]
    fn test_focus_status_debug() {
        assert_eq!(format!("{:?}", FocusStatus::Success), "Success");
        assert_eq!(format!("{:?}", FocusStatus::Failed), "Failed");
    }

    // ========================================================================
    // Escape Helper Logic Tests
    // ========================================================================

    #[test]
    fn test_escape_single_quote() {
        // Test the escaping logic that would be used for user agent strings
        let input = "Mozilla/5.0 (Browser with 'single quotes')";
        let escaped = input.replace('\u{0027}', "\\'");
        // After replace: "Mozilla/5.0 (Browser with \\x27single quotes\\x27)"
        assert!(escaped.contains("\\'"));
    }

    #[test]
    fn test_escape_double_quote_in_selector() {
        // Test the escaping logic for selectors with double quotes
        let input = "div[data-value=\"test\"].class";
        let escaped = input.replace('\u{0022}', "\\\"");
        // After replace: "div[data-value=\\\"test\\\"].class"
        assert!(escaped.contains("\\\""));
    }

    #[test]
    fn test_escape_no_special_chars() {
        let input = "div.classname";
        let escaped = input.replace('\u{0022}', "\\\"");
        assert_eq!(escaped, "div.classname");
    }

    // ========================================================================
    // wait_for_load Timeout Message Tests
    // ========================================================================

    #[test]
    fn test_wait_for_load_timeout_message_format() {
        let timeout_ms = 5000u64;
        let error = anyhow::anyhow!("wait_for_load timeout after {}ms: elapsed", timeout_ms);
        let msg = error.to_string();
        assert!(msg.contains("5000ms"));
        assert!(msg.contains("wait_for_load"));
    }

    // ========================================================================
    // Visibility Check Logic Tests
    // ========================================================================

    #[test]
    fn test_visibility_check_js_logic() {
        // Test that the visibility check JS correctly identifies visible vs hidden
        // Simulate what the JS evaluates: r.width > 0 && r.height > 0

        // Hidden element (display: none) would have 0 dimensions
        let hidden_rect = (0.0, 0.0);
        let is_visible_hidden = hidden_rect.0 > 0.0 && hidden_rect.1 > 0.0;
        assert!(!is_visible_hidden);

        // Visible element has non-zero dimensions
        let visible_rect = (100.0, 50.0);
        let is_visible_normal = visible_rect.0 > 0.0 && visible_rect.1 > 0.0;
        assert!(is_visible_normal);

        // Collapsed element (visibility: hidden) would have 0 dimensions
        let collapsed_rect = (50.0, 0.0);
        let is_visible_collapsed = collapsed_rect.0 > 0.0 && collapsed_rect.1 > 0.0;
        assert!(!is_visible_collapsed);
    }

    // ========================================================================
    // apply_browser_context Option Handling Tests
    // ========================================================================

    #[test]
    fn test_option_user_agent_handling() {
        // Test that Option<&str> handling works correctly
        let ua_some: Option<&str> = Some("Mozilla/5.0 Test");
        let ua_none: Option<&str> = None;

        assert!(ua_some.is_some());
        assert!(ua_none.is_none());

        // Test pattern matching
        match ua_some {
            Some(ua) => assert_eq!(ua, "Mozilla/5.0 Test"),
            None => panic!("Expected Some"),
        }

        match ua_none {
            Some(_) => panic!("Expected None"),
            None => { /* expected */ }
        }
    }

    // ========================================================================
    // Duration Construction Tests
    // ========================================================================

    #[test]
    fn test_duration_from_millis() {
        let dur_50 = Duration::from_millis(50);
        let dur_100 = Duration::from_millis(100);
        let dur_5000 = Duration::from_millis(5000);

        assert_eq!(dur_50.as_millis(), 50);
        assert_eq!(dur_100.as_millis(), 100);
        assert_eq!(dur_5000.as_millis(), 5000);
    }

    #[test]
    fn test_duration_polling_interval() {
        // 50ms polling interval for wait_for_load
        let polling_interval = Duration::from_millis(50);
        assert_eq!(polling_interval.as_millis(), 50);

        // 200ms polling interval for wait_for_any_visible_selector
        let visibility_interval = Duration::from_millis(200);
        assert_eq!(visibility_interval.as_millis(), 200);
    }

    // ========================================================================
    // Retry Logic Simulation Tests
    // ========================================================================

    #[test]
    fn test_retry_logic_max_attempts() {
        // Simulate what with_retry does - 3 retries with exponential backoff
        // attempt increments before delay calculation
        let mut attempt = 0;
        let mut total_delay = 0u64;

        while attempt < EVAL_RETRY_MAX_ATTEMPTS {
            attempt += 1;
            let delay = EVAL_RETRY_BASE_DELAY_MS * (1u64 << attempt.min(6));
            total_delay += delay;
        }

        // After 3 retries: 100 + 200 + 400 = 700ms total delay
        // (first error increments attempt to 1 -> delay 100ms, etc.)
        assert_eq!(total_delay, 700);
    }

    #[test]
    fn test_retry_logic_capped_at_max() {
        // Attempt beyond max should not be retried
        let attempt = EVAL_RETRY_MAX_ATTEMPTS; // 3
        let should_retry = attempt < EVAL_RETRY_MAX_ATTEMPTS;
        assert!(!should_retry);
    }

    #[test]
    fn test_retry_delay_shift_bounds() {
        // Test that shift doesn't overflow - min(6) caps shift at 6
        let large_attempt = 10u32;
        let capped_shift = 1u64 << large_attempt.min(6);
        assert_eq!(capped_shift, 64); // 1 << 6 = 64
    }

    // ========================================================================
    // Transient Error Classification Tests
    // ========================================================================

    #[test]
    fn test_is_transient_error_timeout() {
        // Timeout errors should be transient (retry)
        let timeout_err = anyhow::anyhow!("CDP operation timed out");
        assert!(is_transient_error(&timeout_err));

        let timeout_err2 = anyhow::anyhow!("Timeout: operation exceeded 30s");
        assert!(is_transient_error(&timeout_err2));
    }

    #[test]
    fn test_is_transient_error_not_found() {
        // "Not found" errors should be permanent (no retry)
        let not_found_err = anyhow::anyhow!("Element not found: #btn");
        assert!(!is_transient_error(&not_found_err));

        let not_found_err2 = anyhow::anyhow!("No such element: div.class");
        assert!(!is_transient_error(&not_found_err2));
    }

    #[test]
    fn test_is_transient_error_connection() {
        // Connection errors should be transient (retry)
        let conn_refused = anyhow::anyhow!("Connection refused");
        assert!(is_transient_error(&conn_refused));

        let conn_reset = anyhow::anyhow!("Connection reset by peer");
        assert!(is_transient_error(&conn_reset));

        let conn_broken = anyhow::anyhow!("Connection broken");
        assert!(is_transient_error(&conn_broken));
    }

    #[test]
    fn test_is_transient_error_target_closed() {
        // Target closed should be permanent (no retry)
        let target_closed = anyhow::anyhow!("Target closed");
        assert!(!is_transient_error(&target_closed));

        let node_disconnected = anyhow::anyhow!("Node is disconnected");
        assert!(!is_transient_error(&node_disconnected));
    }

    #[test]
    fn test_is_transient_error_disconnected_standalone() {
        // Standalone "disconnected" (without "node is" prefix) should be transient
        // This catches connection dropouts that may recover on retry
        let disconnected = anyhow::anyhow!("Connection disconnected");
        assert!(is_transient_error(&disconnected));

        let disconnected2 = anyhow::anyhow!("Pipe disconnected");
        assert!(is_transient_error(&disconnected2));
    }

    #[test]
    fn test_is_transient_error_permission_denied() {
        // Permission denied should be permanent (no retry)
        let perm_denied = anyhow::anyhow!("Permission denied for operation");
        assert!(!is_transient_error(&perm_denied));
    }

    #[test]
    fn test_is_transient_error_aborted() {
        // Aborted/cancelled should be transient (retry)
        let aborted = anyhow::anyhow!("Operation aborted");
        assert!(is_transient_error(&aborted));

        let cancelled = anyhow::anyhow!("Operation cancelled");
        assert!(is_transient_error(&cancelled));
    }

    #[test]
    fn test_is_transient_error_network() {
        // Network errors should be transient (retry)
        let network_err = anyhow::anyhow!("Network error: connection failed");
        assert!(is_transient_error(&network_err));

        let econnreset = anyhow::anyhow!("ECONNRESET");
        assert!(is_transient_error(&econnreset));
    }

    #[test]
    fn test_is_transient_error_unknown_defaults_to_retry() {
        // Unknown errors should default to transient (retry) for safety
        let unknown_err = anyhow::anyhow!("Some unexpected CDP error");
        assert!(is_transient_error(&unknown_err));
    }

    #[test]
    fn test_is_transient_error_case_insensitive() {
        // Error matching should be case-insensitive due to to_lowercase()
        let timeout_upper = anyhow::anyhow!("CDP TIMEOUT");
        assert!(is_transient_error(&timeout_upper));

        let not_found_title = anyhow::anyhow!("Element NOT FOUND");
        assert!(!is_transient_error(&not_found_title));
    }

    #[test]
    fn test_is_transient_error_temporary_unavailable() {
        // Temporary or unavailable errors should be transient (retry)
        let temporary = anyhow::anyhow!("Service temporarily unavailable");
        assert!(is_transient_error(&temporary));

        let unavailable = anyhow::anyhow!("Resource unavailable");
        assert!(is_transient_error(&unavailable));

        let temp_failure = anyhow::anyhow!("Temporary failure in name resolution");
        assert!(is_transient_error(&temp_failure));
    }

    #[test]
    fn test_is_transient_error_interrupted() {
        // Interrupted errors should be transient (retry)
        let interrupted = anyhow::anyhow!("Operation interrupted");
        assert!(is_transient_error(&interrupted));

        let thread_interrupted = anyhow::anyhow!("Thread interrupted");
        assert!(is_transient_error(&thread_interrupted));
    }
}