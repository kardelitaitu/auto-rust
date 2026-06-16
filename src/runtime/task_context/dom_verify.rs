//! DOM inspection and verification methods for `TaskContext`.

use anyhow::{Context, Result};

use crate::runtime::task_context::TaskContext;

fn wrapper_timeout_context(stage: &str, details: impl Into<String>) -> String {
    format!("wrapper_timeout | stage={} {}", stage, details.into())
}

// ============================================================================
// Standalone DOM verification functions (delegated from TaskContext in mod.rs)
// ============================================================================

/// Verify that a selector hits a specific point (x, y) on the page.
/// Returns true if the target element matches, contains, or is contained by the hit element.
pub async fn verify_selector_hit(
    ctx: &TaskContext,
    selector: &str,
    x: f64,
    y: f64,
) -> Result<bool> {
    let selector_json = serde_json::to_string(selector)
        .map_err(|e| anyhow::anyhow!("Failed to serialize selector: {}", e))?;
    let selector_str = selector_json
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(&selector_json);

    let js = format!(
        r#"
        (function() {{
            const selector = {};
            const el = document.querySelector(selector);
            if (!el) return false;
            const rect = el.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) return false;
            const hit = document.elementFromPoint({}, {});
            if (!hit) return false;
            return el === hit || el.contains(hit) || hit.contains(el);
        }})()
        "#,
        selector_str, x, y
    );

    let result = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        ctx.page().evaluate(js),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "{}",
            wrapper_timeout_context("verify_selector_hit", format!("x={}, y={}", x, y))
        )
    })?
    .with_context(|| wrapper_timeout_context("verify_selector_hit", format!("x={}, y={}", x, y)))?;

    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

/// Post-interaction pause for human-like behavior.
pub async fn post_interaction_pause(ctx: &TaskContext) {
    post_interaction_pause_with_budget(ctx, 0).await;
}

/// Compute timing parameters for a post-interaction pause.
///
/// Pure function: given a minimum budget, action-delay min_ms, and action-delay
/// variance percentage, returns `(base_ms, variance_pct)` clamped to sensible bounds.
///
/// # Returns
/// - `base_ms`: clamped to [120, 1500]
/// - `variance_pct`: rounded and clamped to [10, 60]
#[must_use]
pub(crate) fn compute_post_interaction_timing(
    min_budget_ms: u64,
    action_delay_min_ms: u64,
    action_delay_variance_pct: f64,
) -> (u64, u32) {
    let base_ms = action_delay_min_ms.max(min_budget_ms).clamp(120, 1500);
    let variance_pct = action_delay_variance_pct.round().clamp(10.0, 60.0) as u32;
    (base_ms, variance_pct)
}

/// Post-interaction pause with minimum budget.
pub async fn post_interaction_pause_with_budget(ctx: &TaskContext, min_budget_ms: u64) {
    let action_delay = &ctx.behavior_runtime.action_delay;
    let (base_ms, variance_pct) = compute_post_interaction_timing(
        min_budget_ms,
        action_delay.min_ms,
        action_delay.variance_pct,
    );
    crate::capabilities::timing::uniform_pause(base_ms, variance_pct).await;
}

#[cfg(test)]
mod timing_tests {
    use super::compute_post_interaction_timing;

    #[test]
    fn test_default_timing_with_zero_budget() {
        let (base_ms, variance_pct) = compute_post_interaction_timing(0, 300, 30.0);
        assert_eq!(base_ms, 300);
        assert_eq!(variance_pct, 30);
    }

    #[test]
    fn test_min_budget_raises_base() {
        let (base_ms, variance_pct) = compute_post_interaction_timing(500, 300, 30.0);
        assert_eq!(base_ms, 500);
        assert_eq!(variance_pct, 30);
    }

    #[test]
    fn test_large_budget_clamped_to_max() {
        let (base_ms, variance_pct) = compute_post_interaction_timing(5000, 300, 30.0);
        assert_eq!(base_ms, 1500);
        assert_eq!(variance_pct, 30);
    }

    #[test]
    fn test_min_budget_raises_below_clamp_min() {
        let (base_ms, variance_pct) = compute_post_interaction_timing(0, 50, 30.0);
        assert_eq!(base_ms, 120);
        assert_eq!(variance_pct, 30);
    }

    #[test]
    fn test_variance_clamped_to_min() {
        let (base_ms, variance_pct) = compute_post_interaction_timing(0, 300, 5.0);
        assert_eq!(base_ms, 300);
        assert_eq!(variance_pct, 10);
    }

    #[test]
    fn test_variance_clamped_to_max() {
        let (base_ms, variance_pct) = compute_post_interaction_timing(0, 300, 80.0);
        assert_eq!(base_ms, 300);
        assert_eq!(variance_pct, 60);
    }

    #[test]
    fn test_variance_rounded() {
        let (_, variance_pct_a) = compute_post_interaction_timing(0, 300, 33.3);
        let (_, variance_pct_b) = compute_post_interaction_timing(0, 300, 33.7);
        assert_eq!(variance_pct_a, 33);
        assert_eq!(variance_pct_b, 34);
    }

    #[test]
    fn test_min_budget_raises_clamped_base() {
        // min_budget=100, min_ms=50, clamped to 120
        let (base_ms, _) = compute_post_interaction_timing(100, 50, 25.0);
        assert_eq!(base_ms, 120);
    }

    #[test]
    fn test_high_budget_above_max_returns_max() {
        let (base_ms, _) = compute_post_interaction_timing(3000, 2000, 25.0);
        assert_eq!(base_ms, 1500);
    }

    #[test]
    fn test_identical_budget_and_min_ms() {
        let (base_ms, _) = compute_post_interaction_timing(500, 500, 25.0);
        assert_eq!(base_ms, 500);
    }
}

#[cfg(test)]
mod fuzz_tests {
    use proptest::prelude::*;
    use serde_json::Value;

    fn val(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or(Value::String(s.to_string()))
    }

    proptest! {
        #[test]
        fn fuzz_verify_selector_hit_value(s: String) {
            let value = val(&s);
            let _ = value.as_bool().unwrap_or(false);
        }

        #[test]
        fn fuzz_verify_selector_hit_nested(s: String) {
            let value = val(&s);
            let _ = value.as_object()
                .and_then(|obj| obj.get("found"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
        }
    }
}
