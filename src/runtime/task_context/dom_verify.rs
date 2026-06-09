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

/// Post-interaction pause with minimum budget.
pub async fn post_interaction_pause_with_budget(ctx: &TaskContext, min_budget_ms: u64) {
    let action_delay = &ctx.behavior_runtime.action_delay;
    let base_ms = action_delay.min_ms.max(min_budget_ms).clamp(120, 1500);
    let variance_pct = action_delay.variance_pct.round().clamp(10.0, 60.0) as u32;
    crate::capabilities::timing::uniform_pause(base_ms, variance_pct).await;
}
