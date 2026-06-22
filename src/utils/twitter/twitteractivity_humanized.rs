//! Human-like interaction pattern utilities for Twitter automation.
//! Wraps `TaskContext` methods with profile-aware defaults and higher-level
//! human-like behaviors (variable pauses, micro-movements, etc.).

use crate::prelude::TaskContext;
use crate::utils::math::gaussian;
use crate::utils::timing::clustered_pause;
use rand::Rng;
use std::time::Duration;
use tracing::instrument;

use super::state::parse_button_coordinates;
use super::twitteractivity_selectors::selector_close_button;

// =========================================================================
// Constants
// =========================================================================

/// Variance percentage for click-related pauses (30%).
pub const CLICK_PAUSE_VARIANCE: u32 = 30;

/// Jitter percentage for micro-pauses (±30%).
pub const MICRO_PAUSE_JITTER_PCT: f64 = 0.3;

/// Navigation pause range (1–3 seconds).
pub const NAVIGATION_PAUSE_MIN_MS: u64 = 1000;
pub const NAVIGATION_PAUSE_MAX_MS: u64 = 3000;

// =========================================================================
// Pure functions extracted from async helpers for testability
// =========================================================================

/// Computes the min/max range for a micro-pause given the average action delay.
/// The range is avg ±30%, clamped to a minimum of 50ms.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_micro_pause_range(avg: u64) -> (u64, u64) {
    let jitter = ((avg as f64) * MICRO_PAUSE_JITTER_PCT) as u64;
    let min = avg.saturating_sub(jitter).max(50);
    let max = avg.saturating_add(jitter).max(min + 50);
    (min, max)
}

/// Rounds a variance percentage `f64` to a `u32`, clamping to at least 0.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
pub fn compute_human_pause_variance(variance_pct: f64) -> u32 {
    variance_pct.round().max(0.0) as u32
}

/// Computes the base pause duration as `base_value * multiplier`.
/// Simple helper to standardize the pattern used across all pause functions.
#[must_use]
pub fn compute_action_base(base_value: u64, multiplier: u64) -> u64 {
    base_value * multiplier
}

/// Computes a timing range `(min, max)` from a base delay and variance.
///
/// - `base = min_ms * multiplier`
/// - Range is `base ± variance%`, clamped to an absolute minimum of 500ms,
///   with max at least `min + 500`.
///
/// Pure function — no browser required.
#[must_use]
#[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
pub fn compute_timing_range(min_ms: u64, variance_pct: f64, multiplier: u64) -> (u64, u64) {
    let base = (min_ms * multiplier) as f64;
    let variance = variance_pct / 100.0;
    let min = (base * (1.0 - variance)).max(500.0) as u64;
    let max = (base * (1.0 + variance)).max((min + 500) as f64) as u64;
    (min, max)
}

// =========================================================================
// Async interaction helpers
// =========================================================================

/// Human pause with variance based on profile action delay behavior.
#[instrument(skip(api))]
pub async fn human_pause(api: &TaskContext, base_ms: u64) {
    let runtime = api.behavior_runtime();
    let variance = compute_human_pause_variance(runtime.action_delay.variance_pct);
    api.pause_human(base_ms, variance).await;
}

/// Short micro-pause typical of human hesitation between actions.
#[allow(clippy::cast_precision_loss)]
pub async fn micro_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let (min, max) = compute_micro_pause_range(runtime.action_delay.min_ms);
    let pause_ms = rand::thread_rng().gen_range(min..=max);
    api.pause(pause_ms).await;
}

/// Brief pause after a navigation-like action.
pub async fn after_navigation_pause(api: &TaskContext) {
    let ms = rand::thread_rng().gen_range(NAVIGATION_PAUSE_MIN_MS..=NAVIGATION_PAUSE_MAX_MS);
    api.pause(ms).await;
}

/// Brief pause after clicking a button (reaction delay).
pub async fn after_click_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = runtime.click.reaction_delay_ms;
    api.pause_human(base, CLICK_PAUSE_VARIANCE).await;
}

/// Sleep using Tokio directly (blocking sleep for fixed periods).
/// Used when `TaskContext.pause()` is not appropriate (e.g., fixed timeout after an action).
pub async fn fixed_sleep(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// Random duration between two bounds using human-like distribution.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn random_duration(min_ms: u64, max_ms: u64) -> Duration {
    let min = min_ms as f64;
    let max = max_ms as f64;
    let mean = f64::midpoint(min, max);
    let stddev = (max - min) / 4.0; // 95% within range
    let duration_ms = gaussian(mean, stddev, min, max);
    Duration::from_millis(duration_ms as u64)
}

/// Action-specific pause for scroll operations.
pub async fn scroll_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = compute_action_base(runtime.action_delay.min_ms, 2);
    api.pause_human(
        base,
        compute_human_pause_variance(runtime.action_delay.variance_pct),
    )
    .await;
}

/// Action-specific pause after an engagement action (like, retweet, follow).
pub async fn engagement_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = compute_action_base(runtime.action_delay.min_ms, 3);
    api.pause_human(
        base,
        compute_human_pause_variance(runtime.action_delay.variance_pct),
    )
    .await;
}

/// Action-specific pause after a reply or quote tweet.
pub async fn reply_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = compute_action_base(runtime.action_delay.min_ms, 4);
    api.pause_human(
        base,
        compute_human_pause_variance(runtime.action_delay.variance_pct),
    )
    .await;
}

/// Clustered pause with micro-movements between engagement actions.
/// Breaks rhythmic patterns by splitting pause into 2-3 segments with micro-jitters.
/// Ideal for transitions between different action types (like → retweet → reply).
pub async fn clustered_engagement_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = compute_action_base(runtime.action_delay.min_ms, 2);
    let variance = compute_human_pause_variance(runtime.action_delay.variance_pct);
    clustered_pause(base, variance, 2, 3).await;
}

/// Clustered pause specifically for reply actions (longer, more natural).
/// Simulates human thinking time before/after composing a reply.
pub async fn clustered_reply_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = compute_action_base(runtime.action_delay.min_ms, 3);
    let variance = compute_human_pause_variance(runtime.action_delay.variance_pct);
    clustered_pause(base, variance, 1, 3).await;
}

/// Action-specific pause before clicking (move-to-click delay).
pub async fn click_prep_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = compute_action_base(runtime.click.reaction_delay_ms, 4);
    api.pause_human(
        base,
        compute_human_pause_variance(runtime.action_delay.variance_pct),
    )
    .await;
}

/// Action-specific pause after clicking.
pub async fn click_post_pause(api: &TaskContext) {
    let runtime = api.behavior_runtime();
    let base = compute_action_base(runtime.click.reaction_delay_ms, 8);
    api.pause_human(
        base,
        compute_human_pause_variance(runtime.action_delay.variance_pct),
    )
    .await;
}

/// Simulates a human closing a popup: move to X button and click.
/// Returns true if a popup was found and closed.
#[instrument(skip(api))]
pub async fn attempt_close_popup(api: &TaskContext) -> Result<bool, anyhow::Error> {
    let js = selector_close_button();
    let result = api.page().evaluate(js.to_string()).await?;

    if let Some((x, y)) = result.value().and_then(parse_button_coordinates) {
        api.move_mouse_to(x, y).await?;
        human_pause(api, 150).await;
        api.click_at(x, y).await?;
        human_pause(api, 500).await;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod timing_computation_tests {
    use super::*;

    // ====================================================================
    // compute_micro_pause_range
    // ====================================================================

    #[test]
    fn micro_pause_normal_value() {
        // avg=500 → jitter=150, min=350, max=650
        let (min, max) = compute_micro_pause_range(500);
        assert_eq!(min, 350);
        assert_eq!(max, 650);
    }

    #[test]
    fn micro_pause_zero_avg() {
        // avg=0 → jitter=0, min=max(0,50)=50, max=max(0,100)=100
        let (min, max) = compute_micro_pause_range(0);
        assert_eq!(min, 50);
        assert_eq!(max, 100);
    }

    #[test]
    fn micro_pause_small_avg() {
        // avg=10 → jitter=3, min=max(7,50)=50, max=max(13,100)=100
        let (min, max) = compute_micro_pause_range(10);
        assert_eq!(min, 50);
        assert_eq!(max, 100);
    }

    #[test]
    fn micro_pause_very_large_avg() {
        // avg=1_000_000 → jitter=300_000, min=700_000, max=1_300_000
        let (min, max) = compute_micro_pause_range(1_000_000);
        assert_eq!(min, 700_000);
        assert_eq!(max, 1_300_000);
    }

    #[test]
    fn micro_pause_min_always_at_least_50() {
        for avg in [0, 1, 5, 10, 50, 71] {
            let (min, _) = compute_micro_pause_range(avg);
            assert!(min >= 50, "avg={avg} produced min={min} < 50");
        }
    }

    #[test]
    fn micro_pause_max_at_least_min_plus_50() {
        for avg in [0, 1, 10, 50, 100, 1000] {
            let (min, max) = compute_micro_pause_range(avg);
            assert!(max >= min + 50, "avg={avg} produced min={min}, max={max}");
        }
    }

    #[test]
    fn micro_pause_avg_167_gives_min_117_max_217() {
        // avg=167 → jitter=trunc(167*0.3)=50, min=167-50=117, max=167+50=217
        let (min, max) = compute_micro_pause_range(167);
        assert_eq!(min, 117);
        assert_eq!(max, 217);
    }

    // ====================================================================
    // compute_human_pause_variance
    // ====================================================================

    #[test]
    fn variance_rounds_normally() {
        assert_eq!(compute_human_pause_variance(20.4), 20);
        assert_eq!(compute_human_pause_variance(20.5), 21);
        assert_eq!(compute_human_pause_variance(99.9), 100);
        assert_eq!(compute_human_pause_variance(0.0), 0);
    }

    #[test]
    fn variance_clamps_below_zero() {
        assert_eq!(compute_human_pause_variance(-5.0), 0);
        assert_eq!(compute_human_pause_variance(-0.1), 0);
    }

    #[test]
    fn variance_handles_large_values() {
        assert_eq!(compute_human_pause_variance(1_000_000.0), 1_000_000);
    }

    #[test]
    fn variance_handles_fractional_values() {
        assert_eq!(compute_human_pause_variance(33.33), 33);
        assert_eq!(compute_human_pause_variance(66.66), 67);
    }

    // ====================================================================
    // compute_action_base
    // ====================================================================

    #[test]
    fn action_base_normal() {
        assert_eq!(compute_action_base(500, 2), 1000);
        assert_eq!(compute_action_base(500, 3), 1500);
        assert_eq!(compute_action_base(500, 4), 2000);
    }

    #[test]
    fn action_base_zero() {
        assert_eq!(compute_action_base(0, 2), 0);
        assert_eq!(compute_action_base(500, 0), 0);
    }

    #[test]
    fn action_base_unit_multiplier() {
        assert_eq!(compute_action_base(500, 1), 500);
    }

    #[test]
    fn action_base_large_values() {
        assert_eq!(compute_action_base(1_000_000, 8), 8_000_000);
    }

    // ====================================================================
    // compute_timing_range
    // ====================================================================

    #[test]
    fn timing_range_normal() {
        // min_ms=500, variance=30%, multiplier=8
        // base=4000, min=4000*0.7=2800, max=4000*1.3=5200
        let (min, max) = compute_timing_range(500, 30.0, 8);
        assert_eq!(min, 2800);
        assert_eq!(max, 5200);
    }

    #[test]
    fn timing_range_min_clamped_to_500() {
        // min_ms=10, variance=30%, multiplier=2
        // base=20, min=20*0.7=14 -> clamped to 500, max=500+500=1000
        let (min, max) = compute_timing_range(10, 30.0, 2);
        assert_eq!(min, 500);
        assert_eq!(max, 1000);
    }

    #[test]
    fn timing_range_zero_min_ms() {
        // min_ms=0, variance=50%, multiplier=8
        // base=0, min=0 -> clamped to 500, max=min+500=1000
        let (min, max) = compute_timing_range(0, 50.0, 8);
        assert_eq!(min, 500);
        assert_eq!(max, 1000);
    }

    #[test]
    fn timing_range_large_values() {
        // min_ms=10_000, variance=20%, multiplier=8
        // base=80_000, min=80_000*0.8=64_000, max=80_000*1.2=96_000
        let (min, max) = compute_timing_range(10_000, 20.0, 8);
        assert_eq!(min, 64_000);
        assert_eq!(max, 96_000);
    }

    #[test]
    fn timing_range_zero_variance() {
        // min_ms=1000, variance=0%, multiplier=4
        // base=4000, min=4000*1.0=4000, max=4000*1.0=4000 (but max >= min+500)
        let (min, max) = compute_timing_range(1000, 0.0, 4);
        assert_eq!(min, 4000);
        assert_eq!(max, 4500);
    }

    #[test]
    fn timing_range_high_variance() {
        // min_ms=1000, variance=90%, multiplier=2
        // base=2000, min=2000*0.1=200 -> clamped to 500, max=2000*1.9=3800
        let (min, max) = compute_timing_range(1000, 90.0, 2);
        assert_eq!(min, 500);
        assert_eq!(max, 3800);
    }

    #[test]
    fn timing_range_always_has_min_leq_max() {
        for min_ms in [0, 1, 100, 500, 1000, 10_000] {
            for variance in [0.0, 10.0, 30.0, 50.0, 100.0] {
                for mult in [1, 2, 4, 8] {
                    let (min, max) = compute_timing_range(min_ms, variance, mult);
                    assert!(
                        min <= max,
                        "min_ms={min_ms} variance={variance} mult={mult}: min={min} > max={max}"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod duration_tests {
    use super::random_duration;

    #[test]
    fn random_duration_stays_within_bounds() {
        for _ in 0..50 {
            let duration = random_duration(100, 200);
            let ms = duration.as_millis();
            assert!((100..=200).contains(&ms));
        }
    }

    #[test]
    fn random_duration_handles_identical_bounds() {
        let duration = random_duration(100, 100);
        let ms = duration.as_millis();
        assert!((90..=110).contains(&ms)); // Allow some variance
    }

    #[test]
    fn random_duration_handles_zero_bounds() {
        let duration = random_duration(0, 0);
        assert_eq!(duration.as_millis(), 0);
    }

    #[test]
    fn random_duration_handles_large_bounds() {
        let duration = random_duration(1000, 5000);
        let ms = duration.as_millis();
        assert!((1000..=5000).contains(&ms));
    }

    #[test]
    fn random_duration_has_reasonable_distribution() {
        let durations: Vec<u64> = (0..100)
            .map(|_| random_duration(100, 200).as_millis() as u64)
            .collect();

        for duration in &durations {
            assert!((100..=200).contains(duration));
        }

        let avg = durations.iter().sum::<u64>() as f64 / durations.len() as f64;
        assert!((130.0..=170.0).contains(&avg));
    }

    #[test]
    fn random_duration_is_variable() {
        let mut durations = Vec::new();
        for _ in 0..10 {
            durations.push(random_duration(100, 200).as_millis());
        }
        let min = *durations.iter().min().unwrap();
        let max = *durations.iter().max().unwrap();
        assert!(max > min, "Expected variance across 10 random samples");
    }

    #[test]
    fn random_duration_with_small_bounds_remains_safe() {
        let duration = random_duration(0, 10);
        let ms = duration.as_millis();
        assert!(ms <= 10, "Duration {}ms exceeded max bound of 10ms", ms);
    }

    #[test]
    fn random_duration_very_small_bounds_are_valid() {
        let duration = random_duration(1, 5);
        let ms = duration.as_millis();
        assert!((0..=10).contains(&ms));
    }
}

#[cfg(test)]
mod selector_tests {
    use super::super::twitteractivity_selectors::{
        js_extract_username_from_url, js_get_current_url, selector_all_tweets,
        selector_close_button, selector_element_center, selector_engagement_buttons,
        selector_feed_visible, selector_follow_button, selector_follow_confirm_modal,
        selector_following_indicator, selector_health_check, selector_login_flow,
        selector_popup_overlay, selector_tweet_user_avatar,
    };

    #[test]
    fn selector_close_button_returns_js() {
        let js = selector_close_button();
        assert!(js.contains("querySelector"));
        assert!(js.contains("aria-label"));
        assert!(js.contains("Close"));
    }

    #[test]
    fn selector_functions_return_valid_js() {
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
    fn selector_element_center_formats_correctly() {
        let js = selector_element_center("div.test");
        assert!(js.contains("div.test"));
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("x"));
        assert!(js.contains("y"));
    }

    #[test]
    fn selector_element_center_escapes_quotes() {
        let js = selector_element_center("div.test\"class");
        assert!(js.contains("\\\""));
        assert!(!js.contains("\"test\""));
    }

    #[test]
    fn selector_element_center_with_complex_selector() {
        let js = selector_element_center("[data-testid=\"tweet\"]");
        assert!(js.contains("data-testid"));
        assert!(js.contains("tweet"));
    }

    #[test]
    fn selector_functions_return_non_empty() {
        assert!(!selector_feed_visible().is_empty());
        assert!(!selector_all_tweets().is_empty());
        assert!(!selector_follow_button().is_empty());
        assert!(!selector_engagement_buttons().is_empty());
    }

    #[test]
    fn selector_functions_contain_function_keyword() {
        assert!(selector_feed_visible().contains("function"));
        assert!(selector_all_tweets().contains("function"));
        assert!(selector_follow_button().contains("function"));
    }

    #[test]
    fn selector_element_center_empty_selector() {
        let js = selector_element_center("");
        assert!(js.contains("querySelector"));
    }
}
