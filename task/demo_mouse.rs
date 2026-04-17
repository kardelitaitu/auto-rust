use anyhow::Result;
use serde_json::Value;
use log::{info, warn};
use std::future::Future;
use std::time::Duration;
use tokio::time::timeout;
use crate::prelude::TaskContext;

pub async fn run(ctx: &TaskContext, payload: Value) -> Result<()> {
    let session_id = ctx.session_id();
    info!("[{session_id}][demo-mouse] Task started");

    let url = extract_url_from_payload(&payload)?;
    info!("[{session_id}][demo-mouse] Navigating to: {}", url);

    ctx.navigate_to(&url, 30000).await?;

    if let Err(e) = ctx.wait_for_load(10000).await {
        warn!("[{session_id}][demo-mouse] Wait for load warning: {}", e);
    }

    let overlay_enabled = payload
        .get("overlay")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    crate::capabilities::mouse::set_overlay_enabled(overlay_enabled);
    info!(
        "[{session_id}][demo-mouse] Mouse overlay in mouse.rs enabled={}",
        overlay_enabled
    );
    ctx.pause(500, 20).await;

    let viewport = ctx.viewport().await?;
    info!("Viewport: {}x{}", viewport.width as i32, viewport.height as i32);
    let mut failed_steps = Vec::new();

    // Test 1: Move cursor
    info!("Test 1: Move to 100:100...");
    if !run_step_with_timeout("test1_move", ctx.move_mouse_fast(100.0, 100.0)).await {
        failed_steps.push("test1_move".to_string());
    }
    info!("Done");
    ctx.pause(300, 10).await;

    // Test 2: Move to center
    info!("Test 2: Move to center...");
    let cx = viewport.width / 2.0;
    let cy = viewport.height / 2.0;
    if !run_step_with_timeout("test2_move", ctx.move_mouse_fast(cx, cy)).await {
        failed_steps.push("test2_move".to_string());
    }
    info!("Done");
    ctx.pause(300, 10).await;

    // Test 3: Left click
    info!("Test 3: Left click at center...");
    if !run_step_with_timeout("test3_left_click", ctx.left_click_fast(cx, cy))
    .await
    {
        failed_steps.push("test3_left_click".to_string());
    }
    info!("Done");
    ctx.pause(300, 10).await;

    // Test 4: Right click
    info!("Test 4: Right click at 400:400...");
    if !run_step_with_timeout("test4_move", ctx.move_mouse_fast(400.0, 400.0)).await {
        failed_steps.push("test4_move".to_string());
    }
    ctx.pause(100, 10).await;
    if !run_step_with_timeout("test4_right_click", ctx.right_click_fast(400.0, 400.0))
    .await
    {
        failed_steps.push("test4_right_click".to_string());
    }
    info!("Done");
    ctx.pause(300, 10).await;

    // Test 5: Scroll
    info!("Test 5: Human scroll read...");
    ctx.scroll_read(3, 420, true, true).await?;
    info!("Done");
    ctx.pause(300, 10).await;

    // Test 6: Multiple clicks (slower)
    info!("Test 6: Multiple clicks...");
    for i in 0..3 {
        let x = 200.0 + (i as f64 * 100.0);
        let y = 200.0;
        info!("Click {} at {:.0}:{:.0}", i + 1, x, y);
        if !run_step_with_timeout("test6_move", ctx.move_mouse_fast(x, y)).await {
            failed_steps.push(format!("test6_move_{}", i + 1));
        }
        ctx.pause(200, 20).await;
        if !run_step_with_timeout("test6_left_click", ctx.left_click_fast(x, y))
        .await
        {
            failed_steps.push(format!("test6_left_click_{}", i + 1));
        }
        ctx.pause(300, 30).await;
    }
    info!("Done");

    ctx.pause(500, 50).await;

    if failed_steps.is_empty() {
        info!("[{session_id}][demo-mouse] Integration result: all mouse checks passed");
    } else {
        warn!(
            "[{session_id}][demo-mouse] Integration result: failed steps = {}",
            failed_steps.join(", ")
        );
        return Err(anyhow::anyhow!(
            "demo-mouse integration failed at steps: {}",
            failed_steps.join(", ")
        ));
    }

    info!("[{session_id}][demo-mouse] Task completed");
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
