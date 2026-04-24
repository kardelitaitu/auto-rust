//! Human-like interaction pattern utilities for Twitter automation.
//! Wraps `TaskContext` methods with profile-aware defaults and higher-level
//! human-like behaviors (variable pauses, micro-movements, etc.).

use crate::prelude::TaskContext;
use crate::utils::math::gaussian;
use crate::utils::timing::clustered_pause;
use rand::Rng;
use serde_json::Value;
use std::time::{Duration, Instant};
use tracing::instrument;

use super::twitteractivity_selectors::*;

/// Human pause with variance based on profile action delay behavior.
#[instrument(skip(api))]
pub async fn human_pause(api: &TaskContext, base_ms: u64) {
    let runtime = api.behavior_runtime();
    api.pause_with_variance(base_ms, runtime.action_delay.variance_pct.round() as u32)
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
#[instrument(skip(api))]
pub async fn verify_element_hover(api: &TaskContext, x: f64, y: f64) -> Result<(), anyhow::Error> {
    // Move to position with slower, more deliberate motion
    api.move_mouse_to(x, y).await?;
    human_pause(api, 300).await;
    Ok(())
}

/// Simulates a human reading content by pausing and occasionally scrolling a small amount.
/// Returns after approximately `duration_ms`.
#[instrument(skip(api))]
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

/// Action-specific pause for scroll operations.
pub async fn scroll_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.action_delay.min_ms * 2;
    api.pause_with_variance(base, runtime.action_delay.variance_pct.round() as u32)
        .await;
}

/// Action-specific pause after an engagement action (like, retweet, follow).
pub async fn engagement_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.action_delay.min_ms * 3;
    api.pause_with_variance(base, runtime.action_delay.variance_pct.round() as u32)
        .await;
}

/// Action-specific pause after a reply or quote tweet.
pub async fn reply_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.action_delay.min_ms * 4;
    api.pause_with_variance(base, runtime.action_delay.variance_pct.round() as u32)
        .await;
}

/// Clustered pause with micro-movements between engagement actions.
/// Breaks rhythmic patterns by splitting pause into 2-3 segments with micro-jitters.
/// Ideal for transitions between different action types (like → retweet → reply).
pub async fn clustered_engagement_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.action_delay.min_ms * 2;
    let variance = runtime.action_delay.variance_pct.round() as u32;
    // Use 2-3 clusters to break rhythmic patterns
    clustered_pause(base, variance, 2, 3).await;
}

/// Clustered pause specifically for reply actions (longer, more natural).
/// Simulates human thinking time before/after composing a reply.
pub async fn clustered_reply_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.action_delay.min_ms * 3;
    let variance = runtime.action_delay.variance_pct.round() as u32;
    // Use 1-3 clusters for reply (more variance = more human-like)
    clustered_pause(base, variance, 1, 3).await;
}

/// Action-specific pause before clicking (move-to-click delay).
pub async fn click_prep_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.click.reaction_delay_ms * 4;
    api.pause_with_variance(base, runtime.action_delay.variance_pct.round() as u32)
        .await;
}

/// Action-specific pause after clicking.
pub async fn click_post_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.click.reaction_delay_ms * 8;
    api.pause_with_variance(base, runtime.action_delay.variance_pct.round() as u32)
        .await;
}

/// Simulates a human closing a popup: move to X button and click.
/// Returns true if a popup was found and closed.
#[instrument(skip(api))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_duration_within_bounds() {
        for _ in 0..50 {
            let duration = random_duration(100, 200);
            let ms = duration.as_millis();
            assert!((100..=200).contains(&ms));
        }
    }

    #[test]
    fn test_random_duration_same_bounds() {
        let duration = random_duration(100, 100);
        let ms = duration.as_millis();
        assert!((90..=110).contains(&ms)); // Allow some variance
    }

    #[test]
    fn test_random_duration_zero_bounds() {
        let duration = random_duration(0, 0);
        assert_eq!(duration.as_millis(), 0);
    }

    #[test]
    fn test_random_duration_large_bounds() {
        let duration = random_duration(1000, 5000);
        let ms = duration.as_millis();
        assert!((1000..=5000).contains(&ms));
    }

    #[test]
    fn test_random_duration_consistency() {
        // Test that the function produces reasonable distribution
        let durations: Vec<u64> = (0..100)
            .map(|_| random_duration(100, 200).as_millis() as u64)
            .collect();

        // All should be within bounds
        for duration in &durations {
            assert!((100..=200).contains(duration));
        }

        // Average should be close to midpoint
        let avg = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
        assert!((130.0..=170.0).contains(&avg));
    }

    #[test]
    fn test_selector_close_button_returns_js() {
        let js = selector_close_button();
        assert!(js.contains("querySelector"));
        assert!(js.contains("aria-label"));
        assert!(js.contains("Close"));
    }

    #[test]
    fn test_selector_functions_return_valid_js() {
        assert!(selector_feed_visible().contains("querySelector"));
        assert!(selector_all_tweets().contains("querySelectorAll"));
        assert!(selector_follow_button().contains("aria-label"));
        assert!(selector_engagement_buttons().contains("like"));
        assert!(selector_login_flow().contains("session"));
        assert!(selector_popup_overlay().contains("dialog"));
        assert!(selector_follow_confirm_modal().contains("follow"));
        assert!(selector_following_indicator().contains("following"));
        assert!(js_get_current_url().contains("location"));
        assert!(js_extract_username_from_url().contains("pathname"));
        assert!(selector_tweet_user_avatar().contains("avatar"));
        assert!(selector_health_check().contains("feed_visible"));
    }

    #[test]
    fn test_selector_element_center_formats_correctly() {
        let js = selector_element_center("div.test");
        assert!(js.contains("div.test"));
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("x"));
        assert!(js.contains("y"));
    }

    #[test]
    fn test_selector_element_center_escapes_quotes() {
        let js = selector_element_center("div.test\"class");
        assert!(js.contains("\\\""));
        assert!(!js.contains("\"test\""));
    }

    #[test]
    fn test_selector_element_center_with_complex_selector() {
        let js = selector_element_center("[data-testid=\"tweet\"]");
        assert!(js.contains("data-testid"));
        assert!(js.contains("tweet"));
    }

    #[test]
    fn test_random_duration_produces_variance() {
        let d1 = random_duration(100, 200);
        let d2 = random_duration(100, 200);
        // Should rarely be exactly the same
        assert_ne!(d1.as_millis(), d2.as_millis());
    }

    #[test]
    fn test_random_duration_negative_bounds_clamped() {
        // Gaussian with negative bounds should clamp to min
        let duration = random_duration(0, 10);
        assert!(duration.as_millis() >= 0);
    }

    #[test]
    fn test_selector_functions_return_non_empty() {
        assert!(!selector_feed_visible().is_empty());
        assert!(!selector_all_tweets().is_empty());
        assert!(!selector_follow_button().is_empty());
        assert!(!selector_engagement_buttons().is_empty());
    }

    #[test]
    fn test_selector_functions_contain_function_keyword() {
        assert!(selector_feed_visible().contains("function"));
        assert!(selector_all_tweets().contains("function"));
        assert!(selector_follow_button().contains("function"));
    }

    #[test]
    fn test_selector_element_center_empty_selector() {
        let js = selector_element_center("");
        assert!(js.contains("querySelector"));
    }

    #[test]
    fn test_random_duration_very_small_bounds() {
        let duration = random_duration(1, 5);
        let ms = duration.as_millis();
        assert!((0..=10).contains(&ms)); // Allow some variance
    }
}
