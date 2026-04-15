use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;
use log::{info, warn};
use crate::utils::{navigation, scroll, timing, mouse};
use rand;

pub async fn run(session_id: &str, page: &Page, payload: Value) -> Result<()> {
    info!("[{}][pageview] Task started", session_id);

    // Extract URL from payload
    let url = extract_url_from_payload(&payload)?;
    info!("[{}][pageview] Visiting URL: {}", session_id, url);

    // Navigate to URL
    if let Err(e) = navigation::goto(&page, &url, 30000).await {
        warn!("[{}][pageview] Failed to navigate to {}: {}", session_id, url, e);
        return Err(e);
    }

    // Wait for page load
    if let Err(e) = navigation::wait_for_load(&page, 10000).await {
        warn!("[{}][pageview] Failed to wait for page load {}: {}", session_id, url, e);
        return Err(e);
    }

    // Perform human-like page viewing behavior
    perform_pageview_behavior(page).await?;

    info!("[{}][pageview] Task completed successfully for: {}", session_id, url);
    Ok(())
}

async fn perform_pageview_behavior(page: &Page) -> Result<()> {
    // Initial pause to simulate page loading and user attention
    timing::human_pause(2000, 40).await;

    // Random scrolling to simulate reading content
    for _ in 0..rand::random::<u32>() % 3 + 1 {
        scroll::random_scroll(page).await?;
        timing::human_pause(3000, 50).await;
    }

    // Occasionally move mouse to simulate user interaction
    if rand::random::<bool>() {
        // Move mouse to a random position on the page
        let target_x = rand::random::<f64>() * 800.0; // Assume 800px width
        let target_y = rand::random::<f64>() * 600.0; // Assume 600px height
        mouse::human_mouse_move_to(page, target_x, target_y).await?;
        timing::human_pause(1000, 30).await;
    }

    // Final pause before task completion
    timing::human_pause(2000, 40).await;

    Ok(())
}

fn extract_url_from_payload(payload: &Value) -> Result<String> {
    // Try to extract URL from payload
    if let Some(url) = payload.get("url") {
        if let Some(url_str) = url.as_str() {
            return Ok(url_str.to_string());
        }
    }

    // Fallback: try "value" field
    if let Some(value) = payload.get("value") {
        if let Some(value_str) = value.as_str() {
            return Ok(value_str.to_string());
        }
    }

    Err(anyhow::anyhow!("No URL found in payload: {:?}", payload))
}