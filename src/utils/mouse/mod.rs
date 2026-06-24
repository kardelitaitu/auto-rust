//! Mouse simulation and human-computer interaction utilities.
//!
//! Provides functions for simulating realistic mouse movements and clicks:
//! - Human-like mouse movement using Bezier curves and various path styles
//! - Click simulation with proper timing and precision
//! - Fitts's Law calculations for optimal target sizing
//! - Configurable velocity and trajectory randomization
//! - Utilities for human-computer interaction studies

use crate::config::NativeInteractionConfig;
use crate::utils::geometry::BoundingBox;
use crate::utils::math::{gaussian, random_in_range};
use crate::utils::native_input;
use crate::utils::scroll;
use crate::utils::timing::human_pause;
use anyhow::Result;
use chromiumoxide::Page;
use log::debug;
use std::time::Duration;
use tokio::time::timeout;

// Submodules
pub mod adaptive;
pub mod cdp;
pub mod curves;
pub mod native;
pub mod overlay;
pub mod trajectory;
pub mod types;

// Re-export types for backward compatibility
pub use adaptive::{hover_before_click, is_element_clickable};
pub use cdp::{
    dispatch_mouse_action, dispatch_mouse_event_cdp, dispatch_pointer_event,
    dispatch_single_mouse_event,
};
pub use curves::{
    click_at, dispatch_click, left_click_at, left_click_at_without_move, middle_click_at,
    right_click_at, right_click_at_without_move,
};
pub use native::{
    acquire_native_input_lock, browser_content_origin, browser_scale,
    cached_native_click_calibration, clear_nativeclick_forced_calibration_for_tests,
    clear_nativeclick_trace_hooks, get_forced_calibration, native_click_calibration_from_metrics,
    native_click_fingerprint, native_input_lock_metrics_snapshot, nativeclick_add_trace_hook,
    next_nativeclick_trace_id, record_nativeclick_trace_phase, screen_point_from_calibration,
    set_nativeclick_forced_calibration_for_tests, solve_calibration_from_probe_samples,
    store_native_click_calibration, take_nativeclick_trace_hooks, validate_native_calibration,
    BrowserWindowMetrics, NativeClickCalibration, NativeClickCalibrationEntry,
    NativeClickFingerprint, NativeClickProbeSample, NativeCursorCandidate, NativeDispatchOptions,
    NativeInputLockGuard, NativeInputLockMetricsSnapshot, ScreenPoint,
    FORCED_NATIVECLICK_CALIBRATION, NATIVECLICK_TRACE_HOOKS, NATIVE_CLICK_CALIBRATION_CACHE,
    NATIVE_CLICK_LOCK, NATIVE_CLICK_PROBE_HIT_FLAG, NATIVE_CLICK_PROBE_ID,
};
pub(crate) use overlay::run_cursor_overlay_background;
pub use overlay::{
    cursor_move_to, cursor_move_to_immediate, cursor_move_to_with_config, is_overlay_enabled,
    set_overlay_enabled, sync_cursor_overlay, sync_cursor_overlay_force, trigger_click_flash,
    CursorMovementConfig, PathStyle, Precision, Speed,
};
pub use trajectory::Point;
pub use types::{
    ClickOutcome, ClickStatus, HoverOutcome, HoverStatus, MouseButton, NativeCursorOutcome,
};

/// Wait for an element to become stable (not animating/layout-shifting).
/// Polls the element's bounding box every 100ms; returns when position
/// stabilizes (delta < 2px) for 3 consecutive checks, or times out.
pub async fn wait_for_stable_element(
    page: &Page,
    selector: &str,
    max_wait_ms: u64,
    required_stable_checks: u32,
    stability_threshold_px: f64,
) -> Result<Option<BoundingBox>> {
    let start_time = std::time::Instant::now();
    let mut prev_box: Option<BoundingBox> = None;
    let mut stable_count = 0u32;

    while start_time.elapsed().as_millis() < u128::from(max_wait_ms) {
        let js = format!(
            r"(function() {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.x, y: r.y, width: r.width, height: r.height }};
            }})()",
            selector.replace('\'', "\\'")
        );

        let result = if let Ok(Ok(eval_result)) =
            timeout(Duration::from_millis(500), page.evaluate(js)).await
        {
            eval_result
        } else {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };

        let obj_opt = result.value().and_then(|v| v.as_object());

        let bbox = if let Some(obj) = obj_opt {
            let x = obj
                .get("x")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let y = obj
                .get("y")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let width = obj
                .get("width")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let height = obj
                .get("height")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            BoundingBox {
                x,
                y,
                width,
                height,
            }
        } else {
            prev_box = None;
            stable_count = 0;
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };

        if bbox.width <= 0.0 || bbox.height <= 0.0 {
            prev_box = None;
            stable_count = 0;
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        if let Some(prev) = prev_box {
            if bbox.approx_eq(&prev, stability_threshold_px) {
                stable_count += 1;
                if stable_count >= required_stable_checks {
                    return Ok(Some(bbox));
                }
            } else {
                stable_count = 0;
            }
        }

        prev_box = Some(bbox);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(prev_box)
}

// ── Public high-level selector-based mouse functions ─────────────────

pub async fn hover_selector_human(
    page: &Page,
    selector: &str,
    hover_delay_ms: u64,
    hover_delay_variance_pct: u32,
    click_offset_px: i32,
) -> Result<HoverOutcome> {
    use super::adaptive::is_element_clickable;

    if !is_element_clickable(page, selector).await? {
        return Err(anyhow::anyhow!("Element '{selector}' not hoverable"));
    }

    scroll::scroll_into_view(page, selector).await?;

    if !is_in_viewport_internal(page, selector).await? {
        return Err(anyhow::anyhow!(
            "Element '{selector}' not in viewport after scroll"
        ));
    }

    let bbox = resolve_selector_bbox_with_retry(page, selector, 3).await?;
    let (x, y) = choose_click_point(&bbox, click_offset_px);
    overlay::cursor_move_to(page, x, y).await?;
    human_pause(hover_delay_ms, hover_delay_variance_pct).await;

    Ok(HoverOutcome {
        hover: HoverStatus::Success,
        x,
        y,
    })
}

pub async fn native_move_cursor_human(
    page: &Page,
    session_id: &str,
    query: Option<&str>,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    native_interaction: &NativeInteractionConfig,
) -> Result<NativeCursorOutcome> {
    let trace_id = next_nativeclick_trace_id();
    let settle_ms = (reaction_delay_ms / 4)
        .clamp(40, 200)
        .max(native_interaction.settle_ms);
    let settle_variance = (reaction_delay_variance_pct / 3).max(10);
    let attention_pause_ms = (reaction_delay_ms / 4).clamp(40, 200);

    let _native_click_guard = acquire_native_input_lock(session_id, trace_id, "nativecursor").await;
    page.bring_to_front().await.map_err(|err| {
        anyhow::anyhow!("trace={trace_id} nativecursor bring_to_front failed: {err}")
    })?;
    human_pause(attention_pause_ms, reaction_delay_variance_pct.min(45)).await;

    let candidate =
        resolve_native_cursor_candidate(page, trace_id, native_interaction, query).await?;
    let point = content_point_to_screen_point(
        page,
        session_id,
        trace_id,
        candidate.x,
        candidate.y,
        native_interaction,
    )
    .await
    .map_err(|err| anyhow::anyhow!("trace={trace_id} nativecursor mapping failed: {err}"))?;
    overlay::sync_native_overlay_position(page, candidate.x, candidate.y).await;

    page.bring_to_front().await.map_err(|err| {
        anyhow::anyhow!("trace={trace_id} nativecursor bring_to_front failed: {err}")
    })?;
    native_move_to_point(
        trace_id,
        point.x,
        point.y,
        NativeDispatchOptions {
            backend: native_interaction.native_input_backend,
            reaction_delay_ms,
            reaction_delay_variance_pct,
            settle_ms,
            settle_variance_pct: settle_variance,
        },
    )
    .await?;
    overlay::sync_native_overlay_position(page, candidate.x, candidate.y).await;

    Ok(NativeCursorOutcome {
        target: candidate.label.unwrap_or_default(),
        x: candidate.x,
        y: candidate.y,
        screen_x: Some(point.x),
        screen_y: Some(point.y),
    })
}

pub async fn middle_click_selector_human(
    page: &Page,
    selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    click_offset_px: i32,
) -> Result<ClickOutcome> {
    click_selector_with_button(
        page,
        selector,
        reaction_delay_ms,
        reaction_delay_variance_pct,
        click_offset_px,
        MouseButton::Middle,
    )
    .await
}

pub async fn click_selector_human(
    page: &Page,
    selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    click_offset_px: i32,
) -> Result<ClickOutcome> {
    use super::adaptive::{
        detect_element_type, is_element_clickable, move_cursor_collision_avoidant,
        wait_for_element_stability,
    };

    if !wait_for_element_stability(page, selector, 5000).await? {
        return Err(anyhow::anyhow!("Element '{selector}' not stable within 5s"));
    }

    scroll::scroll_into_view(page, selector).await?;

    if !is_element_clickable(page, selector).await? {
        return Err(anyhow::anyhow!("Element '{selector}' not clickable"));
    }

    if !is_in_viewport_internal(page, selector).await? {
        return Err(anyhow::anyhow!(
            "Element '{selector}' not in viewport after scroll"
        ));
    }

    let bbox = timeout(
        Duration::from_secs(2),
        resolve_selector_bbox_with_retry(page, selector, 3),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!("click resolve_selector_bbox timeout for selector={selector}")
    })??;

    let (x, y) = choose_click_point(&bbox, click_offset_px);
    move_cursor_collision_avoidant(page, x, y).await?;

    let element_type = detect_element_type(selector);
    adaptive::hover_before_click(page, x, y, &element_type).await?;

    timeout(
        Duration::from_secs(2),
        curves::dispatch_click(page, x, y, MouseButton::Left),
    )
    .await
    .map_err(|_| anyhow::anyhow!("click dispatch_click timeout for selector={selector}"))??;

    let settle_ms = (reaction_delay_ms / 4).clamp(40, 200);
    let settle_variance = (reaction_delay_variance_pct / 3).max(10);
    human_pause(settle_ms, settle_variance).await;

    Ok(ClickOutcome {
        click: ClickStatus::Success,
        x,
        y,
        screen_x: None,
        screen_y: None,
    })
}

pub async fn native_click_selector_human(
    page: &Page,
    session_id: &str,
    selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    click_offset_px: i32,
    native_interaction: &NativeInteractionConfig,
) -> Result<ClickOutcome> {
    use super::adaptive::{is_element_clickable, wait_for_element_stability};
    use super::overlay::nativeclick_debug;

    let trace_id = next_nativeclick_trace_id();
    let stability_wait_ms = native_interaction
        .stability_wait_ms
        .get()
        .clamp(1_000, 30_000);
    if !wait_for_element_stability(page, selector, stability_wait_ms).await? {
        return Err(anyhow::anyhow!("trace={trace_id} nativeclick element '{selector}' not stable within {stability_wait_ms}ms"));
    }

    let settle_ms = (reaction_delay_ms / 4)
        .clamp(40, 200)
        .max(native_interaction.settle_ms);
    let settle_variance = (reaction_delay_variance_pct / 3).max(10);
    let attention_pause_ms = (reaction_delay_ms / 4).clamp(40, 200);

    let _native_click_guard = acquire_native_input_lock(session_id, trace_id, "nativeclick").await;
    page.bring_to_front().await.map_err(|err| {
        anyhow::anyhow!("trace={trace_id} nativeclick bring_to_front failed: {err}")
    })?;
    human_pause(attention_pause_ms, reaction_delay_variance_pct.min(45)).await;
    nativeclick_debug(session_id, trace_id, selector, "scroll-into-view", "start");
    scroll::scroll_into_view(page, selector).await?;

    if !is_element_clickable(page, selector).await? {
        return Err(anyhow::anyhow!(
            "trace={trace_id} nativeclick element '{selector}' not clickable"
        ));
    }

    if !is_in_viewport_internal(page, selector).await? {
        return Err(anyhow::anyhow!(
            "trace={trace_id} nativeclick element '{selector}' not in viewport after scroll"
        ));
    }

    let bbox = timeout(
        Duration::from_millis(
            native_interaction
                .resolve_timeout_ms
                .get()
                .clamp(250, 30_000),
        ),
        resolve_selector_bbox_with_retry(page, selector, 3),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "trace={trace_id} nativeclick resolve_selector_bbox timeout for selector={selector}"
        )
    })??;

    nativeclick_debug(
        session_id,
        trace_id,
        selector,
        "bbox",
        format!(
            "x={:.1} y={:.1} w={:.1} h={:.1}",
            bbox.x, bbox.y, bbox.width, bbox.height
        ),
    );

    let (content_x, content_y) =
        resolve_native_click_point(page, session_id, trace_id, selector, click_offset_px, &bbox)
            .await?;
    let point = content_point_to_screen_point(
        page,
        session_id,
        trace_id,
        content_x,
        content_y,
        native_interaction,
    )
    .await
    .map_err(|err| anyhow::anyhow!("trace={trace_id} nativeclick mapping failed: {err}"))?;
    overlay::sync_native_overlay_position(page, content_x, content_y).await;
    page.bring_to_front().await.map_err(|err| {
        anyhow::anyhow!("trace={trace_id} nativeclick bring_to_front failed: {err}")
    })?;
    nativeclick_debug(
        session_id,
        trace_id,
        selector,
        "dispatch",
        format!("screen=({}, {})", point.x, point.y),
    );
    native_move_and_click_point(
        trace_id,
        point.x,
        point.y,
        NativeDispatchOptions {
            backend: native_interaction.native_input_backend,
            reaction_delay_ms,
            reaction_delay_variance_pct,
            settle_ms,
            settle_variance_pct: settle_variance,
        },
    )
    .await
    .map_err(|err| {
        anyhow::anyhow!("trace={trace_id} nativeclick dispatch failed for '{selector}': {err}")
    })?;

    let verified = verify_click_target(page, selector, content_x, content_y)
        .await
        .map_err(|err| {
            anyhow::anyhow!("trace={trace_id} nativeclick verification check failed: {err}")
        })?;
    overlay::sync_native_overlay_position(page, content_x, content_y).await;
    if !verified {
        return Err(anyhow::anyhow!(
            "trace={trace_id} nativeclick verification failed for '{selector}'"
        ));
    }

    Ok(ClickOutcome {
        click: ClickStatus::Success,
        x: content_x,
        y: content_y,
        screen_x: Some(point.x),
        screen_y: Some(point.y),
    })
}

pub async fn right_click_selector_human(
    page: &Page,
    selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    click_offset_px: i32,
) -> Result<ClickOutcome> {
    click_selector_with_button(
        page,
        selector,
        reaction_delay_ms,
        reaction_delay_variance_pct,
        click_offset_px,
        MouseButton::Right,
    )
    .await
}

pub async fn double_click_selector_human(
    page: &Page,
    selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    click_offset_px: i32,
) -> Result<ClickOutcome> {
    let first = click_selector_with_button(
        page,
        selector,
        reaction_delay_ms,
        reaction_delay_variance_pct,
        click_offset_px,
        MouseButton::Left,
    )
    .await?;
    human_pause(40, 20).await;
    let second = click_selector_with_button(
        page,
        selector,
        reaction_delay_ms / 2,
        reaction_delay_variance_pct,
        click_offset_px,
        MouseButton::Left,
    )
    .await?;

    Ok(ClickOutcome {
        click: if matches!(first.click, ClickStatus::Success)
            && matches!(second.click, ClickStatus::Success)
        {
            ClickStatus::Success
        } else {
            ClickStatus::Failed
        },
        x: second.x,
        y: second.y,
        screen_x: None,
        screen_y: None,
    })
}

pub async fn drag_selector_to_selector(
    page: &Page,
    from_selector: &str,
    to_selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
) -> Result<()> {
    scroll::scroll_into_view(page, from_selector).await?;
    scroll::scroll_into_view(page, to_selector).await?;

    let from_box = resolve_selector_bbox(page, from_selector).await?;
    let to_box = resolve_selector_bbox(page, to_selector).await?;
    let (start_x, start_y) = choose_click_point(&from_box, 6);
    let (end_x, end_y) = choose_click_point(&to_box, 6);

    overlay::cursor_move_to(page, start_x, start_y).await?;
    human_pause(reaction_delay_ms, reaction_delay_variance_pct).await;
    cdp::dispatch_mouse_action(page, start_x, start_y, 0, "mousedown").await?;

    let mid_x = f64::midpoint(start_x, end_x);
    let mid_y = f64::midpoint(start_y, end_y);
    overlay::cursor_move_to(page, mid_x, mid_y).await?;
    overlay::cursor_move_to(page, end_x, end_y).await?;

    cdp::dispatch_mouse_action(page, end_x, end_y, 0, "mouseup").await?;
    Ok(())
}

pub async fn drag_between_points_human(
    page: &Page,
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
) -> Result<()> {
    overlay::cursor_move_to(page, start_x, start_y).await?;
    human_pause(reaction_delay_ms, reaction_delay_variance_pct).await;
    cdp::dispatch_mouse_action(page, start_x, start_y, 0, "mousedown").await?;

    let mid_x = f64::midpoint(start_x, end_x);
    let mid_y = f64::midpoint(start_y, end_y);
    overlay::cursor_move_to(page, mid_x, mid_y).await?;
    overlay::cursor_move_to(page, end_x, end_y).await?;
    cdp::dispatch_mouse_action(page, end_x, end_y, 0, "mouseup").await?;
    Ok(())
}

// ── Native click calibration & coordinate mapping ──────────────────

pub(crate) async fn content_point_to_screen_point(
    page: &Page,
    session_id: &str,
    trace_id: u64,
    x: f64,
    y: f64,
    native_interaction: &NativeInteractionConfig,
) -> Result<ScreenPoint> {
    let metrics = browser_window_metrics(page).await?;
    let mut calibration =
        calibrate_native_click(page, session_id, trace_id, &metrics, native_interaction).await?;
    if let Ok(map) = FORCED_NATIVECLICK_CALIBRATION.lock() {
        if let Some(forced) = map.get(session_id).copied() {
            calibration = forced;
        }
    }
    validate_native_calibration(&calibration)
        .map_err(|err| anyhow::anyhow!("nativeclick calibration invalid: {err}"))?;
    Ok(screen_point_from_calibration(&metrics, &calibration, x, y))
}

// ── Internal helpers ──────────────────────────────────────────────────

fn choose_click_point(bbox: &BoundingBox, click_offset_px: i32) -> (f64, f64) {
    let center_x = bbox.x + bbox.width / 2.0;
    let center_y = bbox.y + bbox.height / 2.0;
    let min_x = bbox.x + 1.0;
    let min_y = bbox.y + 1.0;
    let max_x = (bbox.x + bbox.width - 1.0).max(min_x);
    let max_y = (bbox.y + bbox.height - 1.0).max(min_y);
    let spread = f64::from(click_offset_px.abs()).max(4.0);
    let spread_x = spread.min((bbox.width / 3.0).max(4.0));
    let spread_y = spread.min((bbox.height / 3.0).max(4.0));
    let x = gaussian(center_x, spread_x, min_x, max_x);
    let y = gaussian(center_y, spread_y, min_y, max_y);
    (x, y)
}

#[allow(clippy::cast_precision_loss)]
fn native_click_center_bounds(
    bbox: &BoundingBox,
    click_offset_px: i32,
) -> Option<(i32, i32, i32, i32)> {
    let center_x = bbox.x + bbox.width / 2.0;
    let center_y = bbox.y + bbox.height / 2.0;
    let spread = f64::from(click_offset_px.abs()).max(4.0);
    let min_x = ((center_x - spread).ceil()).max((bbox.x + 1.0).ceil());
    let max_x = ((center_x + spread).floor()).min((bbox.x + bbox.width - 1.0).floor());
    let min_y = ((center_y - spread).ceil()).max((bbox.y + 1.0).ceil());
    let max_y = ((center_y + spread).floor()).min((bbox.y + bbox.height - 1.0).floor());
    if min_x > max_x || min_y > max_y {
        return None;
    }
    Some((min_x as i32, max_x as i32, min_y as i32, max_y as i32))
}

#[allow(clippy::cast_precision_loss)]
fn native_click_random_center_point(bbox: &BoundingBox, click_offset_px: i32) -> (f64, f64) {
    if let Some((min_x, max_x, min_y, max_y)) = native_click_center_bounds(bbox, click_offset_px) {
        let x = random_in_range(min_x as u64, max_x as u64) as f64;
        let y = random_in_range(min_y as u64, max_y as u64) as f64;
        return (x, y);
    }
    let center_x = bbox.x + bbox.width / 2.0;
    let center_y = bbox.y + bbox.height / 2.0;
    (
        center_x.clamp(bbox.x + 1.0, bbox.x + bbox.width - 1.0),
        center_y.clamp(bbox.y + 1.0, bbox.y + bbox.height - 1.0),
    )
}

async fn point_hits_selector(
    page: &Page,
    trace_id: u64,
    selector: &str,
    x: f64,
    y: f64,
) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{ const el = document.querySelector({selector_js}); if (!el) return false; const hit = document.elementFromPoint({x}, {y}); if (!hit) return false; return el === hit || el.contains(hit) || hit.contains(el); }})()"
    );
    let result = timeout(Duration::from_millis(400), page.evaluate(js))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "trace={trace_id} nativeclick point hit-test timeout for selector={selector}"
            )
        })??;
    Ok(result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

async fn resolve_native_click_point(
    page: &Page,
    session_id: &str,
    trace_id: u64,
    selector: &str,
    click_offset_px: i32,
    bbox: &BoundingBox,
) -> Result<(f64, f64)> {
    use super::overlay::nativeclick_debug;
    for _ in 0..6 {
        let (x, y) = native_click_random_center_point(bbox, click_offset_px);
        if point_hits_selector(page, trace_id, selector, x, y)
            .await
            .unwrap_or(false)
        {
            nativeclick_debug(
                session_id,
                trace_id,
                selector,
                "resolved-point",
                format!("content_point=({x:.1},{y:.1})"),
            );
            return Ok((x, y));
        }
    }
    anyhow::bail!(
        "trace={trace_id} nativeclick could not resolve a verified point for selector={selector}"
    );
}

async fn resolve_native_cursor_candidate(
    page: &Page,
    trace_id: u64,
    native_interaction: &NativeInteractionConfig,
    query: Option<&str>,
) -> Result<NativeCursorCandidate> {
    let scope = query.unwrap_or("*");
    let query_js = match query {
        Some(value) => serde_json::to_string(value)?,
        None => "null".to_string(),
    };
    let js = format!(
        r"(() => {{ const query = {query_js}; const root = document.body || document.documentElement; if (!root) return null; const pickPoint = (rect) => {{ const centerX = Math.round(rect.left + rect.width / 2); const centerY = Math.round(rect.top + rect.height / 2); const minX = Math.max(Math.ceil(rect.left + 1), centerX - 4); const maxX = Math.min(Math.floor(rect.right - 1), centerX + 4); const minY = Math.max(Math.ceil(rect.top + 1), centerY - 4); const maxY = Math.min(Math.floor(rect.bottom - 1), centerY + 4); if (minX > maxX || minY > maxY) return null; return {{ x: minX + Math.floor(Math.random() * (maxX - minX + 1)), y: minY + Math.floor(Math.random() * (maxY - minY + 1)) }}; }}; const labelFor = (el) => {{ const tag = (el.tagName || 'element').toLowerCase(); return el.id ? `${{tag}}#${{el.id}}` : tag; }}; const matches = []; let nodes = []; try {{ nodes = query ? Array.from(document.querySelectorAll(query)) : Array.from(root.querySelectorAll('*')); }} catch (err) {{ return {{ error: String(err && err.message ? err.message : err) }}; }} for (const el of nodes) {{ if (!(el instanceof Element)) continue; if (el.id === '__auto_rust_mouse_overlay' || el.id === '__auto_rust_nativeclick_probe') continue; const rect = el.getBoundingClientRect(); if (rect.width < 8 || rect.height < 8) continue; if (rect.bottom <= 0 || rect.right <= 0) continue; if (rect.top >= window.innerHeight || rect.left >= window.innerWidth) continue; const style = window.getComputedStyle(el); if (!style) continue; if (style.display === 'none' || style.visibility === 'hidden') continue; if (Number.parseFloat(style.opacity || '1') === 0) continue; if (el.getAttribute('aria-hidden') === 'true') continue; const point = pickPoint(rect); if (!point) continue; const hit = document.elementFromPoint(point.x, point.y); if (!hit) continue; if (!(el === hit || el.contains(hit) || hit.contains(el))) continue; matches.push({{ label: labelFor(el), x: point.x, y: point.y }}); }} if (!matches.length) return null; return matches[Math.floor(Math.random() * matches.length)]; }})()",
    );
    let result = timeout(
        Duration::from_millis(
            native_interaction
                .resolve_timeout_ms
                .get()
                .clamp(250, 30_000),
        ),
        page.evaluate(js),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!("trace={trace_id} nativecursor candidate lookup timed out for '{scope}'")
    })??;
    let value = result.value().cloned().ok_or_else(|| {
        anyhow::anyhow!("trace={trace_id} nativecursor found no visible candidates for '{scope}'")
    })?;
    if value.is_null() {
        anyhow::bail!("trace={trace_id} nativecursor found no visible candidates for '{scope}'");
    }
    if let Some(error) = value.get("error").and_then(|v| v.as_str()) {
        anyhow::bail!(
            "trace={} nativecursor invalid selector '{}': {}",
            trace_id,
            query.unwrap_or(""),
            error
        );
    }
    Ok(serde_json::from_value(value)?)
}

async fn resolve_selector_bbox(page: &Page, selector: &str) -> Result<BoundingBox> {
    match wait_for_stable_element(page, selector, 2_000, 3, 2.0).await? {
        Some(bbox) => Ok(bbox),
        None => get_selector_bbox_once(page, selector).await,
    }
}

async fn is_in_viewport_internal(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{ const el = document.querySelector({selector_js}); if (!el) return false; const rect = el.getBoundingClientRect(); const windowHeight = window.innerHeight || document.documentElement.clientHeight; const windowWidth = window.innerWidth || document.documentElement.clientWidth; return rect.top < windowHeight && rect.bottom > 0 && rect.left < windowWidth && rect.right > 0; }})()"
    );
    let result = page
        .evaluate(js)
        .await?
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    Ok(result)
}

async fn resolve_selector_bbox_with_retry(
    page: &Page,
    selector: &str,
    max_retries: u32,
) -> Result<BoundingBox> {
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..=max_retries {
        match resolve_selector_bbox(page, selector).await {
            Ok(bbox) => {
                if bbox.width > 0.0 && bbox.height > 0.0 {
                    return Ok(bbox);
                }
                if attempt < max_retries {
                    human_pause(50, 20).await;
                    continue;
                }
                anyhow::bail!(
                    "Element '{selector}' has invalid bounds after {max_retries} retries"
                );
            }
            Err(e) => {
                last_err = Some(e);
                if attempt < max_retries {
                    human_pause(50, 20).await;
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        anyhow::anyhow!("Failed to resolve bbox for '{selector}' after {max_retries} retries")
    }))
}

async fn get_selector_bbox_once(page: &Page, selector: &str) -> Result<BoundingBox> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{ const el = document.querySelector({selector_js}); if (!el) return null; const r = el.getBoundingClientRect(); return {{ x: r.x, y: r.y, width: r.width, height: r.height }}; }})()"
    );
    let result = timeout(Duration::from_millis(800), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("bbox lookup timeout for selector={selector}"))??;
    let obj = result
        .value()
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("Element not found: {selector}"))?;
    let bbox = BoundingBox {
        x: obj
            .get("x")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0),
        y: obj
            .get("y")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0),
        width: obj
            .get("width")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0),
        height: obj
            .get("height")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0),
    };
    if bbox.width <= 0.0 || bbox.height <= 0.0 {
        anyhow::bail!("Element has invalid bounds: {selector}");
    }
    Ok(bbox)
}

async fn click_selector_with_button(
    page: &Page,
    selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    click_offset_px: i32,
    button: MouseButton,
) -> Result<ClickOutcome> {
    use super::adaptive::{detect_element_type, is_element_clickable};
    if !is_element_clickable(page, selector).await? {
        return Err(anyhow::anyhow!("Element '{selector}' not clickable"));
    }
    scroll::scroll_into_view(page, selector).await?;
    if !is_in_viewport_internal(page, selector).await? {
        return Err(anyhow::anyhow!(
            "Element '{selector}' not in viewport after scroll"
        ));
    }
    let bbox = resolve_selector_bbox_with_retry(page, selector, 3).await?;
    let (x, y) = choose_click_point(&bbox, click_offset_px);
    overlay::cursor_move_to(page, x, y).await?;
    let element_type = detect_element_type(selector);
    adaptive::hover_before_click(page, x, y, &element_type).await?;
    curves::dispatch_click(page, x, y, button).await?;
    let settle_ms = (reaction_delay_ms / 4).clamp(40, 200);
    let settle_variance = (reaction_delay_variance_pct / 3).max(10);
    human_pause(settle_ms, settle_variance).await;
    let verified = verify_click_target(page, selector, x, y)
        .await
        .unwrap_or(false);
    if !verified {
        debug!("click target verification was inconclusive for selector={selector}");
    }
    Ok(ClickOutcome {
        click: if verified {
            ClickStatus::Success
        } else {
            ClickStatus::Failed
        },
        x,
        y,
        screen_x: None,
        screen_y: None,
    })
}

async fn verify_click_target(page: &Page, selector: &str, x: f64, y: f64) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{ const el = document.querySelector({selector_js}); if (!el) return false; const rect = el.getBoundingClientRect(); if (rect.width <= 0 || rect.height <= 0) return false; const hit = document.elementFromPoint({x}, {y}); if (!hit) return false; return el === hit || el.contains(hit) || hit.contains(el); }})()"
    );
    let result = timeout(Duration::from_millis(500), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("click verification timeout"))??;
    Ok(result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

async fn browser_window_metrics(page: &Page) -> Result<BrowserWindowMetrics> {
    let js = r"(() => ({ screen_x: window.screenX ?? window.screenLeft ?? 0, screen_y: window.screenY ?? window.screenTop ?? 0, outer_width: window.outerWidth ?? window.innerWidth, outer_height: window.outerHeight ?? window.innerHeight, inner_width: window.innerWidth, inner_height: window.innerHeight, device_pixel_ratio: window.devicePixelRatio ?? 1, visual_viewport_scale: window.visualViewport ? window.visualViewport.scale : 1, visual_viewport_offset_left: window.visualViewport ? window.visualViewport.offsetLeft : 0, visual_viewport_offset_top: window.visualViewport ? window.visualViewport.offsetTop : 0, }))()";
    let result = timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("nativeclick browser metrics timeout"))??;
    let value = result
        .value()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("nativeclick browser metrics missing"))?;
    let metrics: BrowserWindowMetrics = serde_json::from_value(value)?;
    if metrics.inner_width <= 0.0 || metrics.inner_height <= 0.0 {
        anyhow::bail!("nativeclick browser metrics invalid");
    }
    Ok(metrics)
}

async fn calibrate_native_click(
    _page: &Page,
    _session_id: &str,
    _trace_id: u64,
    metrics: &BrowserWindowMetrics,
    native_interaction: &NativeInteractionConfig,
) -> Result<NativeClickCalibration> {
    Ok(native_click_calibration_from_metrics(
        metrics,
        native_interaction.calibration_mode,
    ))
}

async fn native_move_and_click_point(
    trace_id: u64,
    x: i32,
    y: i32,
    opts: NativeDispatchOptions,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        native_input::ensure_native_input_ready(opts.backend);
        native_input::native_move_and_click_point_blocking(
            opts.backend,
            x,
            y,
            opts.reaction_delay_ms,
            opts.reaction_delay_variance_pct,
            opts.settle_ms,
            opts.settle_variance_pct,
        )
    })
    .await
    .map_err(|err| anyhow::anyhow!("trace={trace_id} nativeclick join error: {err}"))?
}

async fn native_move_to_point(
    trace_id: u64,
    x: i32,
    y: i32,
    opts: NativeDispatchOptions,
) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        native_input::ensure_native_input_ready(opts.backend);
        native_input::native_move_to_point_blocking(
            opts.backend,
            x,
            y,
            opts.reaction_delay_ms,
            opts.reaction_delay_variance_pct,
            opts.settle_ms,
            opts.settle_variance_pct,
        )
    })
    .await
    .map_err(|err| anyhow::anyhow!("trace={trace_id} nativecursor join error: {err}"))?
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NativeClickCalibrationMode;
    use crate::utils::native_input::jittered_delay_ms;
    use crate::utils::trajectory::bezier_point;

    #[test]
    fn test_path_style_variants() {
        assert_eq!(PathStyle::Bezier, PathStyle::default());
        assert_ne!(PathStyle::Bezier, PathStyle::Arc);
    }

    #[test]
    fn test_precision_variants() {
        assert_eq!(Precision::Safe, Precision::default());
    }

    #[test]
    fn test_speed_variants() {
        assert_eq!(Speed::Normal, Speed::default());
    }

    #[test]
    fn test_mouse_button_as_button_index() {
        assert_eq!(MouseButton::Left.as_button_index(), 0);
        assert_eq!(MouseButton::Middle.as_button_index(), 1);
        assert_eq!(MouseButton::Right.as_button_index(), 2);
    }

    #[test]
    fn test_cursor_movement_config_defaults() {
        let config = CursorMovementConfig::default();
        assert_eq!(config.speed_multiplier, 1.0);
        assert_eq!(config.curve_spread, 50.0);
        assert_eq!(config.path_style, PathStyle::Bezier);
        assert_eq!(config.precision, Precision::Safe);
        assert_eq!(config.speed, Speed::Normal);
    }

    #[test]
    fn test_cursor_movement_config_with_speed() {
        let config = CursorMovementConfig::default().with_speed(Speed::Fast);
        assert_eq!(config.speed, Speed::Fast);
    }

    #[test]
    fn test_cursor_movement_config_with_precision() {
        let config = CursorMovementConfig::default().with_precision(Precision::Exact);
        assert_eq!(config.precision, Precision::Exact);
    }

    #[test]
    fn test_cursor_movement_config_with_path_style() {
        let config = CursorMovementConfig::default().with_path_style(PathStyle::Zigzag);
        assert_eq!(config.path_style, PathStyle::Zigzag);
    }

    #[test]
    fn test_speed_config_fast() {
        let config = CursorMovementConfig::default().with_speed(Speed::Fast);
        let (mult, delay, _) = config.speed_config();
        assert_eq!(mult, 0.1);
        assert_eq!(delay, (1, 3));
    }

    #[test]
    fn test_speed_config_slow() {
        let config = CursorMovementConfig::default().with_speed(Speed::Slow);
        let (mult, delay, _) = config.speed_config();
        assert_eq!(mult, 1.0);
        assert_eq!(delay, (5, 10));
    }

    #[test]
    fn test_point_new() {
        let point = Point::new(100.0, 200.0);
        assert_eq!(point.x, 100.0);
        assert_eq!(point.y, 200.0);
    }

    #[test]
    fn test_bezier_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);
        let config = CursorMovementConfig::default();
        let points = overlay::generate_bezier_curve_with_config(&start, &end, &config);
        assert!(!points.is_empty());
        assert_eq!(points.first().map(|p| p.x), Some(0.0));
        assert_eq!(points.last().map(|p| p.x), Some(100.0));
    }

    #[test]
    fn test_arc_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let points = overlay::generate_arc_curve(&start, &end);
        assert!(!points.is_empty());
    }

    #[test]
    fn test_zigzag_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);
        let points = overlay::generate_zigzag_curve(&start, &end);
        assert!(!points.is_empty());
    }

    #[test]
    fn test_overshoot_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(50.0, 50.0);
        let points = overlay::generate_overshoot_curve(&start, &end);
        assert_eq!(points.len(), 3);
    }

    #[test]
    fn test_stopped_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 200.0);
        let points = overlay::generate_stopped_curve(&start, &end);
        assert!(points.len() >= 2);
    }

    #[test]
    fn test_muscle_path_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);
        let points = overlay::generate_muscle_path(&start, &end);
        assert!(!points.is_empty());
    }

    fn fitts_law_optimal_size(distance: f64, time: f64) -> f64 {
        let id = time / 100.0;
        2.0 * distance / (2.0_f64.powf(id))
    }

    #[test]
    fn test_fitts_law_optimal_size() {
        let size = fitts_law_optimal_size(100.0, 500.0);
        assert!(size > 0.0);
    }

    #[test]
    fn test_fitts_law_zero_time() {
        let size = fitts_law_optimal_size(100.0, 0.0);
        assert_eq!(size, 200.0);
    }

    #[test]
    fn test_choose_click_point_stays_within_bbox() {
        let bbox = BoundingBox {
            x: 100.0,
            y: 200.0,
            width: 120.0,
            height: 60.0,
        };
        for _ in 0..50 {
            let (x, y) = choose_click_point(&bbox, 8);
            assert!(x >= bbox.x + 1.0 && x <= bbox.x + bbox.width - 1.0);
            assert!(y >= bbox.y + 1.0 && y <= bbox.y + bbox.height - 1.0);
        }
    }

    #[test]
    fn test_bezier_point_exact() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(50.0, 50.0);
        let p2 = Point::new(50.0, 50.0);
        let p3 = Point::new(100.0, 100.0);
        let mid = bezier_point(p0, p1, p2, p3, 0.5);
        assert!((mid.x - 50.0).abs() < 0.1);
        assert!((mid.y - 50.0).abs() < 0.1);
    }

    #[test]
    fn test_browser_content_origin_uses_window_chrome_offsets() {
        let metrics = BrowserWindowMetrics {
            screen_x: 100.0,
            screen_y: 120.0,
            outer_width: 1400.0,
            outer_height: 920.0,
            inner_width: 1320.0,
            inner_height: 860.0,
            device_pixel_ratio: 1.0,
            visual_viewport_scale: 1.0,
            visual_viewport_offset_left: 0.0,
            visual_viewport_offset_top: 0.0,
        };
        let (x, y) =
            browser_content_origin(&metrics, 1.0, 1.0, NativeClickCalibrationMode::Windows);
        assert_eq!(x, 140.0);
        assert_eq!(y, 180.0);
    }

    #[test]
    fn test_screen_point_from_metrics_applies_scale() {
        let metrics = BrowserWindowMetrics {
            screen_x: 100.0,
            screen_y: 120.0,
            outer_width: 1400.0,
            outer_height: 920.0,
            inner_width: 1320.0,
            inner_height: 860.0,
            device_pixel_ratio: 2.0,
            visual_viewport_scale: 1.0,
            visual_viewport_offset_left: 0.0,
            visual_viewport_offset_top: 0.0,
        };
        let calibration =
            native_click_calibration_from_metrics(&metrics, NativeClickCalibrationMode::Windows);
        let point = screen_point_from_calibration(&metrics, &calibration, 20.0, 10.0);
        assert_eq!(point.x, 180);
        assert_eq!(point.y, 200);
    }

    #[test]
    fn test_screen_point_from_calibration_uses_live_viewport_offset() {
        let metrics = BrowserWindowMetrics {
            screen_x: 100.0,
            screen_y: 120.0,
            outer_width: 1400.0,
            outer_height: 920.0,
            inner_width: 1320.0,
            inner_height: 860.0,
            device_pixel_ratio: 2.0,
            visual_viewport_scale: 1.0,
            visual_viewport_offset_left: 0.0,
            visual_viewport_offset_top: 0.0,
        };
        let calibration =
            native_click_calibration_from_metrics(&metrics, NativeClickCalibrationMode::Windows);
        let base_point = screen_point_from_calibration(&metrics, &calibration, 20.0, 10.0);
        let scrolled = BrowserWindowMetrics {
            visual_viewport_offset_left: 12.0,
            visual_viewport_offset_top: 24.0,
            ..metrics
        };
        let scrolled_point = screen_point_from_calibration(&scrolled, &calibration, 20.0, 10.0);
        assert_ne!(base_point, scrolled_point);
    }

    #[test]
    fn test_native_click_candidate_points_prefer_center() {
        let bbox = BoundingBox {
            x: 100.0,
            y: 200.0,
            width: 120.0,
            height: 60.0,
        };
        let bounds = native_click_center_bounds(&bbox, 0).expect("Should compute center bounds");
        assert_eq!(bounds, (156, 164, 226, 234));
    }

    #[test]
    fn test_native_click_candidate_points_stay_within_bbox() {
        let bbox = BoundingBox {
            x: 100.0,
            y: 200.0,
            width: 120.0,
            height: 60.0,
        };
        let (x, y) = native_click_random_center_point(&bbox, 0);
        assert!((156.0..=164.0).contains(&x));
        assert!((226.0..=234.0).contains(&y));
        assert!(x >= bbox.x && x <= bbox.x + bbox.width);
        assert!(y >= bbox.y && y <= bbox.y + bbox.height);
    }

    #[test]
    fn test_native_cursor_outcome_summary() {
        let outcome = NativeCursorOutcome {
            target: "button#submit".to_string(),
            x: 120.0,
            y: 240.0,
            screen_x: Some(640),
            screen_y: Some(480),
        };
        assert_eq!(
            outcome.summary(),
            "nativecursor button#submit (120.0,240.0)"
        );
    }

    #[test]
    fn test_native_click_fingerprint_changes_with_zoom() {
        let base = BrowserWindowMetrics {
            screen_x: 100.0,
            screen_y: 120.0,
            outer_width: 1400.0,
            outer_height: 920.0,
            inner_width: 1320.0,
            inner_height: 860.0,
            device_pixel_ratio: 1.0,
            visual_viewport_scale: 1.0,
            visual_viewport_offset_left: 0.0,
            visual_viewport_offset_top: 0.0,
        };
        let zoomed = BrowserWindowMetrics {
            visual_viewport_scale: 1.25,
            ..base
        };
        assert_ne!(
            native_click_fingerprint(&base, NativeClickCalibrationMode::Windows),
            native_click_fingerprint(&zoomed, NativeClickCalibrationMode::Windows)
        );
    }

    #[test]
    fn test_native_click_fingerprint_ignores_viewport_offsets() {
        let base = BrowserWindowMetrics {
            screen_x: 100.0,
            screen_y: 120.0,
            outer_width: 1400.0,
            outer_height: 920.0,
            inner_width: 1320.0,
            inner_height: 860.0,
            device_pixel_ratio: 1.0,
            visual_viewport_scale: 1.0,
            visual_viewport_offset_left: 0.0,
            visual_viewport_offset_top: 0.0,
        };
        let scrolled = BrowserWindowMetrics {
            visual_viewport_offset_left: 16.0,
            visual_viewport_offset_top: 32.0,
            ..base
        };
        assert_eq!(
            native_click_fingerprint(&base, NativeClickCalibrationMode::Windows),
            native_click_fingerprint(&scrolled, NativeClickCalibrationMode::Windows)
        );
    }

    #[test]
    fn test_native_click_mac_origin_uses_screen_left() {
        let metrics = BrowserWindowMetrics {
            screen_x: 100.0,
            screen_y: 120.0,
            outer_width: 1400.0,
            outer_height: 920.0,
            inner_width: 1320.0,
            inner_height: 860.0,
            device_pixel_ratio: 1.0,
            visual_viewport_scale: 1.0,
            visual_viewport_offset_left: 0.0,
            visual_viewport_offset_top: 0.0,
        };
        let (x, y) = browser_content_origin(&metrics, 1.0, 1.0, NativeClickCalibrationMode::Mac);
        assert!((x - 100.0).abs() < 0.001);
        assert!((y - 180.0).abs() < 0.001);
    }

    #[test]
    fn test_jittered_delay_stays_within_bounds() {
        for _ in 0..64 {
            let delay = jittered_delay_ms(100, 20);
            assert!((80..=120).contains(&delay));
        }
    }

    #[test]
    fn test_jittered_delay_zero_base() {
        let delay = jittered_delay_ms(0, 20);
        assert_eq!(delay, 0);
    }

    #[test]
    fn test_jittered_delay_zero_variance() {
        let delay = jittered_delay_ms(100, 0);
        assert_eq!(delay, 100);
    }

    #[test]
    fn test_click_status_variants() {
        assert_eq!(ClickStatus::Success, ClickStatus::Success);
        assert_eq!(ClickStatus::Failed, ClickStatus::Failed);
    }

    #[test]
    fn test_hover_status_variants() {
        assert_eq!(HoverStatus::Success, HoverStatus::Success);
        assert_eq!(HoverStatus::Failed, HoverStatus::Failed);
    }

    #[test]
    fn test_click_outcome_summary_success() {
        let outcome = ClickOutcome {
            click: ClickStatus::Success,
            x: 100.0,
            y: 200.0,
            screen_x: None,
            screen_y: None,
        };
        assert_eq!(outcome.summary(), "Clicked (100.0,200.0)");
    }

    #[test]
    fn test_click_outcome_summary_failed() {
        let outcome = ClickOutcome {
            click: ClickStatus::Failed,
            x: 100.0,
            y: 200.0,
            screen_x: None,
            screen_y: None,
        };
        assert_eq!(outcome.summary(), "Click failed (100.0,200.0)");
    }

    #[test]
    fn test_hover_outcome_summary_success() {
        let outcome = HoverOutcome {
            hover: HoverStatus::Success,
            x: 150.0,
            y: 250.0,
        };
        assert_eq!(outcome.summary(), "hover:success (150.0,250.0)");
    }

    #[test]
    fn test_hover_outcome_summary_failed() {
        let outcome = HoverOutcome {
            hover: HoverStatus::Failed,
            x: 150.0,
            y: 250.0,
        };
        assert_eq!(outcome.summary(), "hover:failed (150.0,250.0)");
    }

    #[test]
    fn test_set_overlay_enabled() {
        set_overlay_enabled(true);
        let _ = is_overlay_enabled();
        set_overlay_enabled(false);
        let _ = is_overlay_enabled();
    }

    #[test]
    fn test_is_overlay_enabled_default() {
        let _ = is_overlay_enabled();
    }
}
