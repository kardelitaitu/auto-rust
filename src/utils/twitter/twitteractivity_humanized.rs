//! Human-like interaction pattern utilities for Twitter automation.
//! Wraps `TaskContext` methods with profile-aware defaults and higher-level
//! human-like behaviors (variable pauses, micro-movements, etc.).

use crate::prelude::TaskContext;
use crate::utils::math::gaussian;
use rand::Rng;
use serde_json::Value;
use std::time::{Duration, Instant};

use super::twitteractivity_selectors::*;

/// Human pause with variance based on profile action delay behavior.
pub async fn human_pause(api: &TaskContext, base_ms: u64) {
    let runtime = api.behavior_runtime();
    api
        .pause_with_variance(base_ms, runtime.action_delay.variance_pct.round() as u32)
        .await;
}

/// Short micro-pause typical of human hesitation between actions.
pub async fn micro_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let avg = runtime.action_delay.min_ms;
    let jitter = ((avg as f64) * 0.3) as u64; // ±30%
    let min = avg.saturating_sub(jitter).max(50);
    let max = avg.saturating_add(jitter).max(min + 50);
    let pause_ms = rand::thread_rng().gen_range(min..=max);
    api.pause(pause_ms).await;
}

/// Brief pause after a navigation-like action.
pub async fn after_navigation_pause(api: &TaskContext) {
    // Typical human pause after page load: 1–3 seconds
    let ms = rand::thread_rng().gen_range(1000..3000);
    api.pause(ms).await;
}

/// Brief pause after clicking a button (reaction delay).
pub async fn after_click_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.click.reaction_delay_ms;
    let variance = 30;
    api.pause_with_variance(base, variance).await;
}

/// Sleep using Tokio directly (blocking sleep for fixed periods).
/// Used when TaskContext.pause() is not appropriate (e.g., fixed timeout after an action).
pub async fn fixed_sleep(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// Random duration between two bounds using human-like distribution.
pub fn random_duration(min_ms: u64, max_ms: u64) -> Duration {
    let min = min_ms as f64;
    let max = max_ms as f64;
    let mean = (min + max) / 2.0;
    let stddev = (max - min) / 4.0; // 95% within range
    let duration_ms = gaussian(mean, stddev, min, max);
    Duration::from_millis(duration_ms as u64)
}

/// Simulates a human checking that an element is actually visible and interactable.
/// Hovers over the element briefly to activate hover states.
pub async fn verify_element_hover(api: &TaskContext, x: f64, y: f64) -> Result<(), anyhow::Error> {
    // Move to position with slower, more deliberate motion
    api.move_mouse_to(x, y).await?;
    human_pause(api, 300).await;
    Ok(())
}

/// Simulates a human reading content by pausing and occasionally scrolling a small amount.
/// Returns after approximately `duration_ms`.
pub async fn read_content_for(api: &TaskContext, duration_ms: u64) -> Result<(), anyhow::Error> {
    let deadline = std::time::Instant::now() + Duration::from_millis(duration_ms);
    let mut rng = rand::thread_rng();

    while std::time::Instant::now() < deadline {
        // Random short pause (reading)
        let read_pause = rng.gen_range(500..2000);
        api.pause(read_pause).await;

        // Random tiny scroll (like shifting eyes or slight page adjustment)
        if rng.gen_bool(0.3) {
            let tiny_scroll = rng.gen_range(20..100);
            let mut js = String::new();
            js.push_str(&format!("window.scrollBy(0, {});", tiny_scroll));
            api.page().evaluate(js).await?;
            api.pause(200).await;
        }

        // Skip if near deadline
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.as_millis() < 500 {
            break;
        }
    }

    Ok(())
}

/// Simulates a human closing a popup: move to X button and click.
/// Returns true if a popup was found and closed.
pub async fn attempt_close_popup(api: &TaskContext) -> Result<bool, anyhow::Error> {
    let js = selector_close_button();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();

    if let Some(v) = value {
        if let Some(obj) = v.as_object() {
            if let (Some(x), Some(y)) = (
                obj.get("x").and_then(|v: &Value| v.as_f64()),
                obj.get("y").and_then(|v: &Value| v.as_f64()),
            ) {
                api.move_mouse_to(x, y).await?;
                human_pause(api, 150).await;
                api.click_at(x, y).await?;
                human_pause(api, 500).await;
                return Ok(true);
            }
        }
    }

    Ok(false)
}



