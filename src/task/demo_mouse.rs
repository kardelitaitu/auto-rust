use crate::prelude::TaskContext;
use crate::utils::timing::duration_with_variance;
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;

pub const DEFAULT_DEMO_MOUSE_TASK_DURATION_MS: u64 = 60_000;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    timeout(Duration::from_millis(duration_ms), run_inner(api, payload))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "[demo-mouse] Task exceeded duration budget of {}ms",
                duration_ms
            )
        })?
}

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_DEMO_MOUSE_TASK_DURATION_MS, 20)
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    info!("Task started");

    let url = extract_url_from_payload(&payload)?;
    info!("Navigating to: {}", url);

    api.navigate(&url, 30000).await?;

    if let Err(e) = api.wait_for_load(10000).await {
        warn!("Wait for load warning: {}", e);
    }

    let overlay_enabled = payload
        .get("overlay")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    crate::capabilities::mouse::set_overlay_enabled(overlay_enabled);
    info!("Mouse overlay in mouse.rs enabled={}", overlay_enabled);
    api.pause(500).await;

    let viewport = api.viewport().await?;
    info!(
        "Viewport: {}x{}",
        viewport.width as i32, viewport.height as i32
    );
    let mut failed_steps = Vec::new();

    // Test 1: Move cursor
    info!("Test 1: Move to 100:100...");
    if !run_step_with_timeout("test1_move", api.move_mouse_fast(100.0, 100.0)).await {
        failed_steps.push("test1_move".to_string());
    }
    info!("Done");
    api.pause(300).await;

    // Test 2: Move to center
    info!("Test 2: Move to center...");
    let cx = viewport.width / 2.0;
    let cy = viewport.height / 2.0;
    if !run_step_with_timeout("test2_move", api.move_mouse_fast(cx, cy)).await {
        failed_steps.push("test2_move".to_string());
    }
    info!("Done");
    api.pause(300).await;

    // Test 3: Left click
    info!("Test 3: Left click at center...");
    if !run_step_with_timeout("test3_left_click", api.left_click_fast(cx, cy)).await {
        failed_steps.push("test3_left_click".to_string());
    }
    info!("Done");
    api.pause(300).await;

    // Test 4: Right click
    info!("Test 4: Right click at 400:400...");
    if !run_step_with_timeout("test4_move", api.move_mouse_fast(400.0, 400.0)).await {
        failed_steps.push("test4_move".to_string());
    }
    api.pause(100).await;
    if !run_step_with_timeout("test4_right_click", api.right_click_fast(400.0, 400.0)).await {
        failed_steps.push("test4_right_click".to_string());
    }
    info!("Done");
    api.pause(300).await;

    // Test 5: Scroll
    info!("Test 5: Human scroll read...");
    api.scroll_read(3, 420, true, true).await?;
    info!("Done");
    api.pause(300).await;

    // Test 6: Multiple clicks (slower)
    info!("Test 6: Multiple clicks...");
    for i in 0..3 {
        let x = 200.0 + (i as f64 * 100.0);
        let y = 200.0;
        info!("Click {} at {:.0}:{:.0}", i + 1, x, y);
        if !run_step_with_timeout("test6_move", api.move_mouse_fast(x, y)).await {
            failed_steps.push(format!("test6_move_{}", i + 1));
        }
        api.pause(200).await;
        if !run_step_with_timeout("test6_left_click", api.left_click_fast(x, y)).await {
            failed_steps.push(format!("test6_left_click_{}", i + 1));
        }
        api.pause(300).await;
    }
    info!("Done");

    api.pause(500).await;

    if failed_steps.is_empty() {
        info!("Integration result: all mouse checks passed");
    } else {
        warn!(
            "Integration result: failed steps = {}",
            failed_steps.join(", ")
        );
        return Err(anyhow::anyhow!(
            "demo-mouse integration failed at steps: {}",
            failed_steps.join(", ")
        ));
    }

    info!("Task completed");
    Ok(())
}

fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(url) = payload.get("url") {
        if let Some(url_str) = url.as_str() {
            return Ok(url_str.to_string());
        }
    }

    if let Some(value) = payload.get("value") {
        if let Some(value_str) = value.as_str() {
            return Ok(value_str.to_string());
        }
    }

    // Check for default_url in payload
    if let Some(default_url) = payload.get("default_url") {
        if let Some(url_str) = default_url.as_str() {
            return Ok(url_str.to_string());
        }
    }

    Ok("https://uvi.gg/keyboard-tester/".to_string())
}

async fn run_step_with_timeout<T, F>(label: &str, fut: F) -> bool
where
    F: Future<Output = Result<T>>,
{
    match timeout(Duration::from_secs(8), fut).await {
        Ok(Ok(_)) => true,
        Ok(Err(e)) => {
            warn!("[demo-mouse] {label} failed: {e}");
            false
        }
        Err(_) => {
            warn!("[demo-mouse] {label} timed out");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // =========================================================================
    // extract_url_from_payload Tests
    // =========================================================================

    #[test]
    fn test_extract_url_from_payload_with_url_field() {
        let payload = json!({"url": "https://example.com"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://example.com");
    }

    #[test]
    fn test_extract_url_from_payload_with_value_field() {
        let payload = json!({"value": "https://test.com/page"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://test.com/page");
    }

    #[test]
    fn test_extract_url_from_payload_with_default_url_field() {
        let payload = json!({"default_url": "https://default.example.com"});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://default.example.com");
    }

    #[test]
    fn test_extract_url_from_payload_url_priority_over_value() {
        // url field takes priority over value
        let payload = json!({
            "url": "https://priority.com",
            "value": "https://secondary.com"
        });
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://priority.com");
    }

    #[test]
    fn test_extract_url_from_payload_uses_default_when_empty() {
        let payload = json!({});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://uvi.gg/keyboard-tester/");
    }

    #[test]
    fn test_extract_url_from_payload_invalid_url_type() {
        // URL is a number, should fall back to default
        let payload = json!({"url": 12345});
        let result = extract_url_from_payload(&payload).unwrap();
        assert_eq!(result, "https://uvi.gg/keyboard-tester/");
    }

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!(duration_ms >= 48_000 && duration_ms <= 72_000);
    }
}
