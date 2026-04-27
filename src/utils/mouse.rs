//! Mouse simulation and human-computer interaction utilities.
//!
//! Provides functions for simulating realistic mouse movements and clicks:
//! - Human-like mouse movement using Bezier curves and various path styles
//! - Click simulation with proper timing and precision
//! - Fitts's Law calculations for optimal target sizing
//! - Configurable velocity and trajectory randomization
//! - Utilities for human-computer interaction studies

use crate::config::{NativeClickCalibrationMode, NativeInteractionConfig};
use crate::logger::scoped_log_context;
use crate::state::{
    are_all_overlays_enabled, overlay_for_page, set_overlay_enabled_for_all, SessionOverlayState,
};
use crate::utils::geometry::BoundingBox;
use crate::utils::math::{gaussian, random_in_range};
use crate::utils::native_input;
use crate::utils::page_size::get_viewport;
use crate::utils::scroll;
use crate::utils::timing::human_pause;
use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::input::{
    DispatchMouseEventParams, DispatchMouseEventType, MouseButton as CdpMouseButton,
};
use chromiumoxide::Page;
use log::{debug, info};
use once_cell::sync::Lazy;
#[cfg(test)]
use rand::Rng;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{sleep, timeout, Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoverStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct ClickOutcome {
    pub click: ClickStatus,
    pub x: f64,
    pub y: f64,
    pub screen_x: Option<i32>,
    pub screen_y: Option<i32>,
}

#[derive(Debug, Clone, Copy)]
pub struct HoverOutcome {
    pub hover: HoverStatus,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone)]
pub struct NativeCursorOutcome {
    pub target: String,
    pub x: f64,
    pub y: f64,
    pub screen_x: Option<i32>,
    pub screen_y: Option<i32>,
}

impl ClickOutcome {
    pub fn summary(&self) -> String {
        match self.click {
            ClickStatus::Success => format!("Clicked ({:.1},{:.1})", self.x, self.y),
            ClickStatus::Failed => format!("Click failed ({:.1},{:.1})", self.x, self.y),
        }
    }
}

impl HoverOutcome {
    pub fn summary(&self) -> String {
        let status = match self.hover {
            HoverStatus::Success => "success",
            HoverStatus::Failed => "failed",
        };
        format!("hover:{status} ({:.1},{:.1})", self.x, self.y)
    }
}

impl NativeCursorOutcome {
    pub fn summary(&self) -> String {
        format!("nativecursor {} ({:.1},{:.1})", self.target, self.x, self.y)
    }
}

/// Wait for an element to become stable (not animating/layout-shifting).
/// Polls the element's bounding box every 100ms; returns when position
/// stabilizes (delta < 2px) for 3 consecutive checks, or times out.
///
/// # Arguments
/// * `page` - Browser page
/// * `selector` - CSS selector for the target element
/// * `max_wait_ms` - Maximum wait time in milliseconds
/// * `required_stable_checks` - Number of consecutive stable readings required (default: 3)
/// * `stability_threshold_px` - Position delta threshold in pixels (default: 2.0)
///
/// # Returns
/// `Ok(Some(BoundingBox))` when stable, `Ok(None)` if element not found or timeout
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

    while start_time.elapsed().as_millis() < max_wait_ms as u128 {
        // Query bounding box via JavaScript evaluation
        let js = format!(
            r#"(function() {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return {{ x: r.x, y: r.y, width: r.width, height: r.height }};
            }})()"#,
            selector.replace('\'', "\\'")
        );

        let result = match timeout(Duration::from_millis(500), page.evaluate(js)).await {
            Ok(Ok(eval_result)) => eval_result,
            _ => {
                sleep(Duration::from_millis(100)).await;
                continue;
            }
        };

        // Extract object from serde_json::Value
        let obj_opt = result.value().and_then(|v| v.as_object());

        let bbox = if let Some(obj) = obj_opt {
            let x = obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let width = obj.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let height = obj.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);
            BoundingBox {
                x,
                y,
                width,
                height,
            }
        } else {
            prev_box = None;
            stable_count = 0;
            sleep(Duration::from_millis(100)).await;
            continue;
        };

        // Validate bbox dimensions
        if bbox.width <= 0.0 || bbox.height <= 0.0 {
            prev_box = None;
            stable_count = 0;
            sleep(Duration::from_millis(100)).await;
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
        sleep(Duration::from_millis(100)).await;
    }

    Ok(prev_box)
}

const OVERLAY_SYNC_INTERVAL_MS: u64 = 50;
const DEFAULT_OVERLAY_SIZE_PX: f64 = 12.0;
const MIN_OVERLAY_SIZE_PX: f64 = 4.0;
const MAX_OVERLAY_SIZE_PX: f64 = 64.0;

static OVERLAY_SIZE_PX: Lazy<f64> = Lazy::new(|| {
    std::env::var("MOUSE_OVERLAY_SIZE_PX")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.clamp(MIN_OVERLAY_SIZE_PX, MAX_OVERLAY_SIZE_PX))
        .unwrap_or(DEFAULT_OVERLAY_SIZE_PX)
});

static NATIVE_CLICK_LOCK: Lazy<TokioMutex<()>> = Lazy::new(|| TokioMutex::new(()));
static NATIVE_CLICK_CALIBRATION_CACHE: Lazy<
    std::sync::Mutex<HashMap<String, NativeClickCalibrationEntry>>,
> = Lazy::new(|| std::sync::Mutex::new(HashMap::new()));
static FORCED_NATIVECLICK_CALIBRATION: Lazy<
    std::sync::Mutex<HashMap<String, NativeClickCalibration>>,
> = Lazy::new(|| std::sync::Mutex::new(HashMap::new()));
static NATIVECLICK_TRACE_HOOKS: Lazy<std::sync::Mutex<HashMap<String, Vec<String>>>> =
    Lazy::new(|| std::sync::Mutex::new(HashMap::new()));
static NATIVE_CLICK_TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);
static NATIVE_LOCK_ACQUISITIONS: AtomicU64 = AtomicU64::new(0);
static NATIVE_LOCK_CONTENTIONS: AtomicU64 = AtomicU64::new(0);
static NATIVE_LOCK_TOTAL_WAIT_MS: AtomicU64 = AtomicU64::new(0);
static NATIVE_LOCK_MAX_WAIT_MS: AtomicU64 = AtomicU64::new(0);
static NATIVE_LOCK_TOTAL_HOLD_MS: AtomicU64 = AtomicU64::new(0);
static NATIVE_LOCK_MAX_HOLD_MS: AtomicU64 = AtomicU64::new(0);
const NATIVE_LOCK_CONTENTION_THRESHOLD_MS: u64 = 50;
const NATIVE_LOCK_WARN_WAIT_THRESHOLD_MS: u64 = 500;

#[derive(Debug, Clone, Copy)]
pub struct NativeInputLockMetricsSnapshot {
    pub acquisitions: u64,
    pub contentions: u64,
    pub total_wait_ms: u64,
    pub max_wait_ms: u64,
    pub avg_wait_ms: f64,
    pub total_hold_ms: u64,
    pub max_hold_ms: u64,
    pub avg_hold_ms: f64,
}

pub fn native_input_lock_metrics_snapshot() -> NativeInputLockMetricsSnapshot {
    let acquisitions = NATIVE_LOCK_ACQUISITIONS.load(Ordering::Relaxed);
    let total_wait_ms = NATIVE_LOCK_TOTAL_WAIT_MS.load(Ordering::Relaxed);
    let total_hold_ms = NATIVE_LOCK_TOTAL_HOLD_MS.load(Ordering::Relaxed);
    let denom = acquisitions.max(1) as f64;
    NativeInputLockMetricsSnapshot {
        acquisitions,
        contentions: NATIVE_LOCK_CONTENTIONS.load(Ordering::Relaxed),
        total_wait_ms,
        max_wait_ms: NATIVE_LOCK_MAX_WAIT_MS.load(Ordering::Relaxed),
        avg_wait_ms: total_wait_ms as f64 / denom,
        total_hold_ms,
        max_hold_ms: NATIVE_LOCK_MAX_HOLD_MS.load(Ordering::Relaxed),
        avg_hold_ms: total_hold_ms as f64 / denom,
    }
}

fn atomic_update_max(target: &AtomicU64, value: u64) {
    let mut current = target.load(Ordering::Relaxed);
    while value > current
        && target
            .compare_exchange(current, value, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
    {
        current = target.load(Ordering::Relaxed);
    }
}

struct NativeInputLockGuard<'a> {
    _guard: tokio::sync::MutexGuard<'a, ()>,
    acquired_at: Instant,
}

impl Drop for NativeInputLockGuard<'_> {
    fn drop(&mut self) {
        let hold_ms = self.acquired_at.elapsed().as_millis() as u64;
        NATIVE_LOCK_TOTAL_HOLD_MS.fetch_add(hold_ms, Ordering::Relaxed);
        atomic_update_max(&NATIVE_LOCK_MAX_HOLD_MS, hold_ms);
    }
}

async fn acquire_native_input_lock(
    session_id: &str,
    trace_id: u64,
    op: &'static str,
) -> NativeInputLockGuard<'static> {
    let wait_started = Instant::now();
    let guard = NATIVE_CLICK_LOCK.lock().await;
    let wait_ms = wait_started.elapsed().as_millis() as u64;
    NATIVE_LOCK_ACQUISITIONS.fetch_add(1, Ordering::Relaxed);
    NATIVE_LOCK_TOTAL_WAIT_MS.fetch_add(wait_ms, Ordering::Relaxed);
    atomic_update_max(&NATIVE_LOCK_MAX_WAIT_MS, wait_ms);
    if wait_ms >= NATIVE_LOCK_CONTENTION_THRESHOLD_MS {
        NATIVE_LOCK_CONTENTIONS.fetch_add(1, Ordering::Relaxed);
    }
    if wait_ms >= NATIVE_LOCK_WARN_WAIT_THRESHOLD_MS {
        with_nativeclick_log_context(session_id, || {
            info!(
                "native-input-lock trace={} op={} wait_ms={} acquisitions={} contentions={}",
                trace_id,
                op,
                wait_ms,
                NATIVE_LOCK_ACQUISITIONS.load(Ordering::Relaxed),
                NATIVE_LOCK_CONTENTIONS.load(Ordering::Relaxed)
            );
        });
    }
    NativeInputLockGuard {
        _guard: guard,
        acquired_at: Instant::now(),
    }
}

fn with_nativeclick_log_context(session_id: &str, f: impl FnOnce()) {
    let mut ctx = crate::logger::get_log_context();
    ctx.session_id = Some(session_id.to_string());
    let _guard = scoped_log_context(ctx);
    f();
}

fn next_nativeclick_trace_id() -> u64 {
    NATIVE_CLICK_TRACE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn nativeclick_debug(
    session_id: &str,
    trace_id: u64,
    selector: &str,
    phase: &str,
    message: impl std::fmt::Display,
) {
    record_nativeclick_trace_phase(session_id, phase);
    with_nativeclick_log_context(session_id, || {
        debug!(
            "nativeclick trace={} session={} selector={} phase={} {}",
            trace_id, session_id, selector, phase, message
        );
    });
}

fn record_nativeclick_trace_phase(session_id: &str, phase: &str) {
    if let Ok(mut hooks) = NATIVECLICK_TRACE_HOOKS.lock() {
        hooks
            .entry(session_id.to_string())
            .or_default()
            .push(phase.to_string());
    }
}

/// Clears captured nativeclick trace phases for the session.
/// Used by integration tests to assert pipeline ordering.
pub fn clear_nativeclick_trace_hooks(session_id: &str) {
    if let Ok(mut hooks) = NATIVECLICK_TRACE_HOOKS.lock() {
        hooks.remove(session_id);
    }
}

/// Returns and clears captured nativeclick trace phases for the session.
/// Used by integration tests to assert pipeline ordering.
pub fn take_nativeclick_trace_hooks(session_id: &str) -> Vec<String> {
    if let Ok(mut hooks) = NATIVECLICK_TRACE_HOOKS.lock() {
        return hooks.remove(session_id).unwrap_or_default();
    }
    Vec::new()
}

/// Sets a forced calibration override for integration tests.
/// This is intended for test harnesses that need deterministic error-path coverage.
pub fn set_nativeclick_forced_calibration_for_tests(
    session_id: &str,
    scale_x: f64,
    scale_y: f64,
    origin_adjust_x: f64,
    origin_adjust_y: f64,
    mode: NativeClickCalibrationMode,
) {
    if let Ok(mut map) = FORCED_NATIVECLICK_CALIBRATION.lock() {
        map.insert(
            session_id.to_string(),
            NativeClickCalibration {
                scale_x,
                scale_y,
                origin_adjust_x,
                origin_adjust_y,
                mode,
            },
        );
    }
}

/// Clears the forced calibration override used by integration tests.
pub fn clear_nativeclick_forced_calibration_for_tests(session_id: &str) {
    if let Ok(mut map) = FORCED_NATIVECLICK_CALIBRATION.lock() {
        map.remove(session_id);
    }
}

#[derive(Debug, Clone, Copy)]
struct NativeClickCalibrationEntry {
    fingerprint: NativeClickFingerprint,
    calibration: NativeClickCalibration,
}

#[derive(Debug, Clone, Deserialize)]
struct NativeCursorCandidate {
    label: String,
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeClickFingerprint {
    mode: NativeClickCalibrationMode,
    screen_x: i32,
    screen_y: i32,
    outer_width: i32,
    outer_height: i32,
    inner_width: i32,
    inner_height: i32,
    device_pixel_ratio_milli: i32,
    visual_viewport_scale_milli: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
#[derive(Default)]
pub enum PathStyle {
    #[default]
    Bezier,
    Arc,
    Zigzag,
    Overshoot,
    Stopped,
    Muscle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
#[derive(Default)]
pub enum Precision {
    Exact,
    #[default]
    Safe,
    Rough,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
#[derive(Default)]
pub enum Speed {
    Fast,
    #[default]
    Normal,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

impl MouseButton {
    fn as_button_index(&self) -> u16 {
        match self {
            MouseButton::Left => 0,
            MouseButton::Right => 2,
            MouseButton::Middle => 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CursorMovementConfig {
    pub speed_multiplier: f64,
    pub min_step_delay_ms: u64,
    pub max_step_delay_variance_ms: u64,
    pub curve_spread: f64,
    pub steps: Option<u32>,
    pub add_micro_pauses: bool,
    pub path_style: PathStyle,
    pub precision: Precision,
    pub speed: Speed,
}

impl Default for CursorMovementConfig {
    fn default() -> Self {
        Self {
            speed_multiplier: 1.0,
            min_step_delay_ms: 2,
            max_step_delay_variance_ms: 5,
            curve_spread: 50.0,
            steps: None,
            add_micro_pauses: true,
            path_style: PathStyle::Bezier,
            precision: Precision::Safe,
            speed: Speed::Normal,
        }
    }
}

impl CursorMovementConfig {
    #[allow(dead_code)]
    pub fn with_speed(mut self, speed: Speed) -> Self {
        self.speed = speed;
        self
    }

    #[allow(dead_code)]
    pub fn with_precision(mut self, precision: Precision) -> Self {
        self.precision = precision;
        self
    }

    #[allow(dead_code)]
    pub fn with_path_style(mut self, style: PathStyle) -> Self {
        self.path_style = style;
        self
    }

    fn speed_config(&self) -> (f64, (u64, u64), bool) {
        match self.speed {
            Speed::Fast => (0.1, (1, 3), true),
            Speed::Normal => (0.5, (2, 5), false),
            Speed::Slow => (1.0, (5, 10), false),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

pub fn set_overlay_enabled(enabled: bool) {
    set_overlay_enabled_for_all(enabled);
}

pub fn is_overlay_enabled() -> bool {
    are_all_overlays_enabled()
}

fn overlay_state_for_page(page: &Page) -> Option<Arc<SessionOverlayState>> {
    overlay_for_page(page.target_id().as_ref())
}

fn cursor_start_position(page: &Page, viewport: &crate::utils::page_size::Viewport) -> (f64, f64) {
    if let Some(overlay_state) = overlay_state_for_page(page) {
        return overlay_state.cursor_start_position(viewport);
    }

    (viewport.width / 2.0, viewport.height / 2.0)
}

pub async fn cursor_move_to(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    cursor_move_to_with_config(page, target_x, target_y, &CursorMovementConfig::default()).await
}

pub async fn cursor_move_to_with_config(
    page: &Page,
    target_x: f64,
    target_y: f64,
    config: &CursorMovementConfig,
) -> Result<()> {
    let viewport = timeout(Duration::from_secs(2), get_viewport(page))
        .await
        .map_err(|_| anyhow::anyhow!("cursor_move_to_with_config viewport timeout"))??;
    let (start_x, start_y) = cursor_start_position(page, &viewport);

    // Degenerate path guard: if source and target are effectively identical,
    // dispatch one move event and return to avoid zero-range sampling.
    let dx = target_x - start_x;
    let dy = target_y - start_y;
    if dx.hypot(dy) < 0.5 {
        dispatch_mousemove(page, target_x, target_y).await?;
        return Ok(());
    }

    let start_point = Point::new(start_x, start_y);
    let end_point = Point::new(target_x, target_y);

    let points = match config.path_style {
        PathStyle::Bezier => generate_bezier_curve_with_config(&start_point, &end_point, config),
        PathStyle::Arc => generate_arc_curve(&start_point, &end_point),
        PathStyle::Zigzag => generate_zigzag_curve(&start_point, &end_point),
        PathStyle::Overshoot => generate_overshoot_curve(&start_point, &end_point),
        PathStyle::Stopped => generate_stopped_curve(&start_point, &end_point),
        PathStyle::Muscle => generate_muscle_path(&start_point, &end_point),
    };

    let (move_multiplier, _, disable_human_path) = config.speed_config();
    let use_human_path = config.add_micro_pauses && !disable_human_path;

    for point in points {
        dispatch_mousemove(page, point.x, point.y).await?;

        if use_human_path {
            let delay = (config.min_step_delay_ms as f64
                / config.speed_multiplier
                / move_multiplier) as u64;
            let variance = (config.max_step_delay_variance_ms as f64
                / config.speed_multiplier
                / move_multiplier) as u32;
            human_pause(delay, variance).await;

            if random_in_range(0, 100) < 10 {
                human_pause(random_in_range(50, 200), 20).await;
            }
        }
    }
    // Always land overlay on the final cursor point even when throttling is active.
    sync_cursor_overlay_force(page).await.ok();

    Ok(())
}

pub async fn cursor_move_to_immediate(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    dispatch_mousemove(page, target_x, target_y).await?;
    sync_cursor_overlay_force(page).await.ok();
    Ok(())
}

async fn dispatch_mousemove(page: &Page, x: f64, y: f64) -> Result<()> {
    dispatch_mousemove_dom(page, x, y).await?;
    sync_cursor_overlay(page).await.ok();

    Ok(())
}

async fn sync_native_overlay_position(page: &Page, x: f64, y: f64) {
    if let Some(overlay_state) = overlay_state_for_page(page) {
        overlay_state.set_cursor_position(x, y);
        sync_cursor_overlay_force(page).await.ok();
    }
}

async fn dispatch_mousemove_dom(page: &Page, x: f64, y: f64) -> Result<()> {
    if let Some(overlay_state) = overlay_state_for_page(page) {
        overlay_state.set_cursor_position(x, y);
    }

    if dispatch_mouse_event_cdp(
        page,
        DispatchMouseEventType::MouseMoved,
        x,
        y,
        None,
        None,
        None,
    )
    .await
    .is_ok()
    {
        return Ok(());
    }

    // Fallback path for environments where CDP mouse dispatch fails.
    let eval = page.evaluate(format!(
        r#"(function() {{
            const el = document.elementFromPoint({}, {});
            if (!el) return;
            const evt = new MouseEvent('mousemove', {{
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {},
                button: 0
            }});
            el.dispatchEvent(evt);
        }})()"#,
        x, y, x, y
    ));
    timeout(Duration::from_secs(2), eval)
        .await
        .map_err(|_| anyhow::anyhow!("dispatch_mousemove timed out"))??;
    Ok(())
}

pub async fn sync_cursor_overlay(page: &Page) -> Result<()> {
    sync_cursor_overlay_with_mode(page, false).await
}

pub async fn sync_cursor_overlay_force(page: &Page) -> Result<()> {
    sync_cursor_overlay_with_mode(page, true).await
}

pub async fn run_cursor_overlay_background(
    overlay_state: Arc<SessionOverlayState>,
    interval_ms: u64,
    session_id: String,
) {
    let interval = Duration::from_millis(interval_ms);

    loop {
        tokio::time::sleep(interval).await;

        if !overlay_state.is_enabled() {
            continue;
        }

        let Some(active_page) = overlay_state.active_page() else {
            continue;
        };

        if let Err(e) = sync_cursor_overlay(&active_page).await {
            log::debug!("[{}] cursor overlay error: {}", session_id, e);
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

async fn sync_cursor_overlay_with_mode(page: &Page, force: bool) -> Result<()> {
    let Some(overlay_state) = overlay_state_for_page(page) else {
        return Ok(());
    };

    if !overlay_state.is_enabled() {
        return Ok(());
    }

    let (x, y) = if let Some((x, y)) = overlay_state.cursor_position_snapshot() {
        (x, y)
    } else {
        // Initialize overlay position at viewport center so the cursor dot is visible
        // before the first explicit mouse movement.
        let viewport = timeout(Duration::from_millis(500), get_viewport(page))
            .await
            .map_err(|_| anyhow::anyhow!("sync_cursor_overlay viewport timeout"))??;
        let cx = viewport.width / 2.0;
        let cy = viewport.height / 2.0;
        overlay_state.set_cursor_position(cx, cy);
        (cx, cy)
    };
    let now_ms = now_unix_ms();
    if !overlay_state.claim_sync_slot(now_ms, force, OVERLAY_SYNC_INTERVAL_MS) {
        log::debug!("Overlay sync skipped (throttled)");
        return Ok(());
    }

    log::debug!("Syncing cursor overlay to ({}, {})", x, y);
    let eval = page.evaluate(format!(
        "(function() {{
            let dot = document.getElementById('__auto_rust_mouse_overlay');
            if (!dot) {{
                dot = document.createElement('div');
                dot.id = '__auto_rust_mouse_overlay';
                dot.style.position = 'fixed';
                dot.style.width = '{}px';
                dot.style.height = '{}px';
                dot.style.background = '#ffffff';
                dot.style.border = '4px solid #ff6600';
                dot.style.pointerEvents = 'none';
                dot.style.zIndex = '2147483647';
                document.body.appendChild(dot);
            }}
            dot.style.left = '{}px';
            dot.style.top = '{}px';
        }})();",
        *OVERLAY_SIZE_PX,
        *OVERLAY_SIZE_PX,
        x - (*OVERLAY_SIZE_PX / 2.0),
        y - (*OVERLAY_SIZE_PX / 2.0)
    ));

    timeout(Duration::from_millis(500), eval)
        .await
        .map_err(|_| anyhow::anyhow!("sync_cursor_overlay timed out"))??;
    Ok(())
}

fn generate_bezier_curve_with_config(
    start: &Point,
    end: &Point,
    config: &CursorMovementConfig,
) -> Vec<Point> {
    let mut points = Vec::new();

    let spread = config.curve_spread;
    let cp1 = Point::new(
        gaussian(
            (start.x + end.x) / 2.0,
            spread,
            start.x.min(end.x),
            start.x.max(end.x),
        ),
        gaussian(
            (start.y + end.y) / 2.0,
            spread,
            start.y.min(end.y),
            start.y.max(end.y),
        ),
    );

    let cp2 = Point::new(
        gaussian(
            (start.x + end.x) / 2.0,
            spread * 0.6,
            start.x.min(end.x),
            start.x.max(end.x),
        ),
        gaussian(
            (start.y + end.y) / 2.0,
            spread * 0.6,
            start.y.min(end.y),
            start.y.max(end.y),
        ),
    );

    let steps = config
        .steps
        .unwrap_or_else(|| random_in_range(10, 20) as u32);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let point = bezier_point(*start, cp1, cp2, *end, t);
        points.push(point);
    }

    points
}

fn bezier_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let x = (1.0 - t).powi(3) * p0.x
        + 3.0 * (1.0 - t).powi(2) * t * p1.x
        + 3.0 * (1.0 - t) * t.powi(2) * p2.x
        + t.powi(3) * p3.x;
    let y = (1.0 - t).powi(3) * p0.y
        + 3.0 * (1.0 - t).powi(2) * t * p1.y
        + 3.0 * (1.0 - t) * t.powi(2) * p2.y
        + t.powi(3) * p3.y;
    Point::new(x, y)
}

fn generate_arc_curve(start: &Point, end: &Point) -> Vec<Point> {
    let mid_x = (start.x + end.x) / 2.0;
    let distance = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
    let mid_y = (start.y + end.y) / 2.0
        - distance
            * 0.3
            * if random_in_range(0, 2) == 0 {
                1.0
            } else {
                -1.0
            };

    let control = Point::new(mid_x, mid_y);
    let mut points = Vec::new();
    let steps = 10;

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        points.push(bezier_point(*start, control, control, *end, t));
    }
    points
}

fn generate_zigzag_curve(start: &Point, end: &Point) -> Vec<Point> {
    let mut points = Vec::new();
    let steps = 4;
    let distance = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
    let zigzag_amount = distance * 0.1;

    for i in 0..=steps {
        let progress = i as f64 / steps as f64;
        let base_x = start.x + (end.x - start.x) * progress;
        let base_y = start.y + (end.y - start.y) * progress;

        let perp_x =
            -(end.y - start.y) / distance * zigzag_amount * if i % 2 == 0 { 1.0 } else { -1.0 };
        let perp_y =
            (end.x - start.x) / distance * zigzag_amount * if i % 2 == 0 { 1.0 } else { -1.0 };

        points.push(Point::new(base_x + perp_x, base_y + perp_y));
    }
    points
}

fn generate_overshoot_curve(start: &Point, end: &Point) -> Vec<Point> {
    let overshoot_scale = 1.2;
    let overshoot_x = start.x + (end.x - start.x) * overshoot_scale;
    let overshoot_y = start.y + (end.y - start.y) * overshoot_scale;

    vec![*start, Point::new(overshoot_x, overshoot_y), *end]
}

fn generate_stopped_curve(start: &Point, end: &Point) -> Vec<Point> {
    let stops = 3;
    let mut points = Vec::new();

    for i in 0..=stops {
        let progress = i as f64 / stops as f64;
        let x = start.x + (end.x - start.x) * progress;
        let y = start.y + (end.y - start.y) * progress;
        points.push(Point::new(x, y));
    }
    points
}

fn generate_muscle_path(start: &Point, end: &Point) -> Vec<Point> {
    let mut points = Vec::new();
    let max_steps = 20;
    let tolerance = 2.0;

    let mut current = *start;

    for _ in 0..max_steps {
        let dx = end.x - current.x;
        let dy = end.y - current.y;
        let dist = (dx.powi(2) + dy.powi(2)).sqrt();

        if dist < tolerance {
            points.push(*end);
            break;
        }

        let kp = 0.8;
        let step_size = dist.min(50.0) * kp;
        let next_x = current.x + (dx / dist) * step_size;
        let next_y = current.y + (dy / dist) * step_size;

        let jitter = gaussian(0.0, 0.8, -2.0, 2.0);
        current = Point::new(next_x + jitter, next_y + jitter);
        points.push(current);
    }

    points
}

pub async fn click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    left_click_at(page, x, y).await
}

pub async fn left_click_at_without_move(page: &Page, x: f64, y: f64) -> Result<()> {
    dispatch_click(page, x, y, MouseButton::Left).await
}

pub async fn right_click_at_without_move(page: &Page, x: f64, y: f64) -> Result<()> {
    dispatch_click(page, x, y, MouseButton::Right).await
}

#[allow(dead_code)]
pub async fn click_at_with_options(
    page: &Page,
    x: f64,
    y: f64,
    button: MouseButton,
    move_to_first: bool,
    precision: Precision,
    hover_ms: u64,
) -> Result<()> {
    let viewport = get_viewport(page).await?;

    if x < 0.0 || x > viewport.width || y < 0.0 || y > viewport.height {
        anyhow::bail!(
            "Coordinates ({}, {}) outside viewport ({}x{})",
            x,
            y,
            viewport.width,
            viewport.height
        );
    }

    let mut target_x = x;
    let mut target_y = y;

    match precision {
        Precision::Rough => {
            target_x = x + random_in_range(0, 20) as f64 - 10.0;
            target_y = y + random_in_range(0, 20) as f64 - 10.0;
        }
        Precision::Safe => {
            target_x = x + random_in_range(0, 6) as f64 - 3.0;
            target_y = y + random_in_range(0, 6) as f64 - 3.0;
        }
        Precision::Exact => {}
    }

    if move_to_first {
        cursor_move_to(page, target_x, target_y).await?;
    } else {
        dispatch_mousemove(page, target_x, target_y).await?;
    }

    if hover_ms > 0 {
        human_pause(hover_ms, 20).await;
    }

    dispatch_click(page, target_x, target_y, button).await
}

pub async fn left_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click(page, x, y, MouseButton::Left).await
}

#[allow(dead_code)]
pub async fn middle_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click(page, x, y, MouseButton::Middle).await
}

#[allow(dead_code)]
pub async fn right_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click(page, x, y, MouseButton::Right).await
}

async fn dispatch_click(page: &Page, x: f64, y: f64, button: MouseButton) -> Result<()> {
    // Fire pointer events around the click for better browser compatibility
    // Real browsers fire these, but most automation tools skip them
    let _ = dispatch_pointer_event(page, "pointerover", x, y, button).await;
    let _ = dispatch_pointer_event(page, "pointerenter", x, y, button).await;

    // Small delay to simulate pointer capture
    crate::utils::timing::human_pause(15, 30).await;

    // Fire pointermove at final position
    let _ = dispatch_pointer_event(page, "pointermove", x, y, button).await;

    // Mouse events (the actual click)
    let button_idx = button.as_button_index();
    dispatch_mouse_action(page, x, y, button_idx, "mousedown").await?;

    // Brief press duration - real humans don't release immediately
    crate::utils::timing::human_pause(80, 25).await;

    dispatch_mouse_action(page, x, y, button_idx, "mouseup").await?;

    // Fire pointerout after click (cleanup)
    let _ = dispatch_pointer_event(page, "pointerout", x, y, button).await;

    Ok(())
}

async fn dispatch_mouse_action(
    page: &Page,
    x: f64,
    y: f64,
    button_idx: u16,
    event_type: &str,
) -> Result<()> {
    if let Some(cdp_type) = map_cdp_event_type(event_type) {
        let button = map_cdp_button(button_idx);
        let buttons = match cdp_type {
            DispatchMouseEventType::MousePressed => Some(mouse_button_mask(button_idx)),
            DispatchMouseEventType::MouseReleased => Some(0),
            DispatchMouseEventType::MouseMoved | DispatchMouseEventType::MouseWheel => None,
        };
        let click_count = if matches!(
            cdp_type,
            DispatchMouseEventType::MousePressed | DispatchMouseEventType::MouseReleased
        ) {
            Some(1)
        } else {
            None
        };

        if dispatch_mouse_event_cdp(page, cdp_type, x, y, button, buttons, click_count)
            .await
            .is_ok()
        {
            return Ok(());
        }
    } else if event_type == "click" {
        // Native click is produced by mousePressed + mouseReleased.
        return Ok(());
    }

    // Fallback path for environments where CDP mouse dispatch fails.
    let eval = page.evaluate(format!(
        "(function() {{
            const el = document.elementFromPoint({}, {});
            if (!el) return false;

            const evt = new MouseEvent('{}', {{
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {},
                button: {}
            }});
            el.dispatchEvent(evt);
            return true;
        }})();",
        x, y, event_type, x, y, button_idx
    ));

    let result = timeout(Duration::from_secs(2), eval)
        .await
        .map_err(|_| anyhow::anyhow!("dispatch_mouse_action timed out"))??;

    let did_dispatch = result.value().and_then(|v| v.as_bool()).unwrap_or(false);
    if !did_dispatch {
        anyhow::bail!("dispatch_mouse_action found no element at ({x:.1},{y:.1})");
    }

    Ok(())
}

fn map_cdp_button(button_idx: u16) -> Option<CdpMouseButton> {
    match button_idx {
        0 => Some(CdpMouseButton::Left),
        1 => Some(CdpMouseButton::Middle),
        2 => Some(CdpMouseButton::Right),
        _ => None,
    }
}

fn mouse_button_mask(button_idx: u16) -> i64 {
    match button_idx {
        0 => 1, // Left
        1 => 4, // Middle
        2 => 2, // Right
        _ => 0,
    }
}

/// Dispatch a pointer event for enhanced browser event coverage.
/// Pointer events (pointerover, pointerenter, pointermove, etc.) are fired
/// by real browsers but often skipped by automation tools.
///
/// # Arguments
/// * `page` - Browser page
/// * `event_type` - Pointer event type (pointerover, pointerenter, pointermove, pointerout, pointerleave)
/// * `x` - X coordinate
/// * `y` - Y coordinate
/// * `button` - Mouse button being pressed
async fn dispatch_pointer_event(
    page: &Page,
    event_type: &str,
    x: f64,
    y: f64,
    button: MouseButton,
) -> Result<()> {
    let pointer_id = 1; // Standard mouse pointer ID
    let button_idx = button.as_button_index();
    let buttons = mouse_button_mask(button_idx);

    let js = format!(
        r#"(function() {{
            const el = document.elementFromPoint({}, {});
            if (!el) return false;
            
            const evt = new PointerEvent('{}', {{
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {},
                pointerId: {},
                width: 1,
                height: 1,
                pressure: 0.5,
                tiltX: 0,
                tiltY: 0,
                pointerType: 'mouse',
                isPrimary: true,
                button: {},
                buttons: {}
            }});
            
            el.dispatchEvent(evt);
            return true;
        }})();"#,
        x, y, event_type, x, y, pointer_id, button_idx, buttons
    );

    let result = timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("dispatch_pointer_event timed out"))??;

    let did_dispatch = result.value().and_then(|v| v.as_bool()).unwrap_or(false);
    if !did_dispatch {
        // Non-fatal - some elements don't support pointer events
        debug!("dispatch_pointer_event: no element at ({}, {})", x, y);
    }

    Ok(())
}

fn map_cdp_event_type(event_type: &str) -> Option<DispatchMouseEventType> {
    match event_type {
        "mousedown" => Some(DispatchMouseEventType::MousePressed),
        "mouseup" => Some(DispatchMouseEventType::MouseReleased),
        "mousemove" => Some(DispatchMouseEventType::MouseMoved),
        _ => None,
    }
}

async fn dispatch_mouse_event_cdp(
    page: &Page,
    event_type: DispatchMouseEventType,
    x: f64,
    y: f64,
    button: Option<CdpMouseButton>,
    buttons: Option<i64>,
    click_count: Option<i64>,
) -> Result<()> {
    let params = DispatchMouseEventParams {
        r#type: event_type,
        x,
        y,
        modifiers: None,
        timestamp: None,
        button,
        buttons,
        click_count,
        force: None,
        tangential_pressure: None,
        tilt_x: None,
        tilt_y: None,
        twist: None,
        delta_x: None,
        delta_y: None,
        pointer_type: None,
    };
    timeout(Duration::from_secs(2), page.execute(params))
        .await
        .map_err(|_| anyhow::anyhow!("dispatch_mouse_event_cdp timed out"))??;
    Ok(())
}

#[allow(dead_code)]
pub async fn click_selector(page: &Page, selector: &str) -> Result<()> {
    click_selector_human(page, selector, 60, 25, 6)
        .await
        .map(|_| ())
}

pub async fn hover_selector_human(
    page: &Page,
    selector: &str,
    hover_delay_ms: u64,
    hover_delay_variance_pct: u32,
    click_offset_px: i32,
) -> Result<HoverOutcome> {
    scroll::scroll_into_view(page, selector).await?;

    let bbox = resolve_selector_bbox(page, selector).await?;
    let (x, y) = choose_click_point(&bbox, click_offset_px);
    cursor_move_to(page, x, y).await?;
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
        anyhow::anyhow!(
            "trace={} nativecursor bring_to_front failed: {}",
            trace_id,
            err
        )
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
    .map_err(|err| anyhow::anyhow!("trace={} nativecursor mapping failed: {}", trace_id, err))?;
    sync_native_overlay_position(page, candidate.x, candidate.y).await;

    page.bring_to_front().await.map_err(|err| {
        anyhow::anyhow!(
            "trace={} nativecursor bring_to_front failed: {}",
            trace_id,
            err
        )
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
    sync_native_overlay_position(page, candidate.x, candidate.y).await;

    Ok(NativeCursorOutcome {
        target: candidate.label,
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

pub async fn hover_before_click(
    page: &Page,
    _selector: &str,
    x: f64,
    y: f64,
    element_type: &str,
) -> Result<()> {
    // Different elements have different natural hover times based on human behavior
    let base_hover_ms: u64 = match element_type {
        "button" => random_in_range(80, 200),
        "link" => random_in_range(100, 350),
        "input" => random_in_range(50, 150),
        "checkbox" => random_in_range(120, 280),
        "radio" => random_in_range(100, 250),
        "dropdown" | "select" => random_in_range(150, 400),
        "menu" | "nav" => random_in_range(60, 180),
        _ => random_in_range(60, 180),
    };

    // Fire pointerenter at hover start
    let _ = dispatch_pointer_event(page, "pointerenter", x, y, MouseButton::Left).await;

    // Variable hover duration with variance
    let variance = random_in_range(20, 40) as u32;
    human_pause(base_hover_ms, variance).await;

    // Subtle position shift during hover (humans aren't perfectly still)
    let hover_shift_x = gaussian(0.0, 1.5, -4.0, 4.0);
    let hover_shift_y = gaussian(0.0, 1.5, -4.0, 4.0);
    dispatch_mousemove(page, x + hover_shift_x, y + hover_shift_y).await?;

    // Fire pointerleave before click (to properly balance pointerenter)
    let _ = dispatch_pointer_event(page, "pointerleave", x, y, MouseButton::Left).await;

    Ok(())
}

/// Detects element type from selector for hover duration customization.
/// This is a heuristic-based detection.
fn detect_element_type(selector: &str) -> String {
    let sel_lower = selector.to_lowercase();

    // Input elements
    if sel_lower.contains("input") {
        if sel_lower.contains("checkbox") {
            return "checkbox".to_string();
        }
        if sel_lower.contains("radio") {
            return "radio".to_string();
        }
        return "input".to_string();
    }

    // Form elements
    if sel_lower.contains("button") || sel_lower.contains("submit") {
        return "button".to_string();
    }
    if sel_lower.contains("select") || sel_lower.contains("dropdown") {
        return "dropdown".to_string();
    }

    // Navigation
    if sel_lower.contains("nav") || sel_lower.contains("menu") || sel_lower.contains("a[") {
        return "link".to_string();
    }

    // Generic link
    if sel_lower.starts_with("a ") || sel_lower.starts_with("a[") {
        return "link".to_string();
    }

    "default".to_string()
}

/// Waits for element to be stable (position not changing) before interaction
async fn wait_for_element_stability(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    let start_time = std::time::Instant::now();
    let check_interval_ms = 100;
    let required_stable_checks = 3;
    let mut stable_count = 0;

    while start_time.elapsed().as_millis() < timeout_ms as u128 {
        // Check if element exists and is visible
        let exists_js = format!(
            r#"(() => {{
                const el = document.querySelector({});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0 && rect.top >= 0;
            }})()"#,
            serde_json::to_string(selector)?
        );

        let exists = page
            .evaluate(exists_js)
            .await?
            .value()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !exists {
            stable_count = 0;
            tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
            continue;
        }

        // Get current position
        let pos_js = format!(
            r#"(() => {{
                const el = document.querySelector({});
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.left, y: rect.top, width: rect.width, height: rect.height }};
            }})()"#,
            serde_json::to_string(selector)?
        );

        let current_result = page.evaluate(pos_js.clone()).await?;
        let current_pos = current_result.value().and_then(|v| v.as_object());

        if current_pos.is_none() {
            stable_count = 0;
            tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
            continue;
        }

        // Wait a bit and check position again
        tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;

        let next_result = page.evaluate(pos_js).await?;
        let next_pos = next_result.value().and_then(|v| v.as_object());

        if let (Some(curr), Some(next)) = (current_pos, next_pos) {
            let curr_x = curr.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let curr_y = curr.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let next_x = next.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let next_y = next.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);

            // Check if position changed significantly (tolerance: 2px)
            let dx = (curr_x - next_x).abs();
            let dy = (curr_y - next_y).abs();

            if dx <= 2.0 && dy <= 2.0 {
                stable_count += 1;
                if stable_count >= required_stable_checks {
                    return Ok(true);
                }
            } else {
                stable_count = 0; // Reset on movement
            }
        } else {
            stable_count = 0;
        }
    }

    Ok(false)
}

/// Moves cursor to target coordinates with adaptive speed and collision avoidance
async fn move_cursor_collision_avoidant(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    // Get current cursor position (assume viewport center if unknown)
    let viewport = timeout(Duration::from_secs(1), get_viewport(page))
        .await
        .map_err(|_| anyhow::anyhow!("Failed to get viewport for collision avoidance"))??;

    let start_x = viewport.width as f64 / 2.0;
    let start_y = viewport.height as f64 / 2.0;

    // Calculate adaptive speed based on context
    let distance = ((target_x - start_x).powi(2) + (target_y - start_y).powi(2)).sqrt();
    let mut config = calculate_adaptive_cursor_config(
        distance,
        50.0,
        ExperienceLevel::Intermediate,
        ElementPriority::Normal,
    );

    let start_point = Point::new(start_x, start_y);
    let end_point = Point::new(target_x, target_y);

    let points = match config.path_style {
        PathStyle::Bezier => generate_bezier_curve_with_config(&start_point, &end_point, &config),
        _ => generate_bezier_curve_with_config(&start_point, &end_point, &config), // Default to bezier
    };

    // Check for potential collisions along the path
    let collision_points = detect_ui_collisions_along_path(page, &points).await?;

    let final_points = if collision_points.is_empty() {
        points
    } else {
        // Generate alternative path avoiding collisions
        generate_collision_free_path(&points, &collision_points, &config)
    };

    // Phase 2: Use adaptive speed during movement
    move_along_points_adaptive(page, &final_points, &mut config).await?;

    Ok(())
}

/// Calculates adaptive cursor configuration based on context
fn calculate_adaptive_cursor_config(
    distance: f64,
    target_size_px: f64,
    user_experience: ExperienceLevel,
    target_importance: ElementPriority,
) -> CursorMovementConfig {
    // Base speed depends on user experience
    let base_multiplier = match user_experience {
        ExperienceLevel::Novice => 0.7, // Slower, more deliberate
        ExperienceLevel::Intermediate => 1.0,
        ExperienceLevel::Expert => 1.3, // Faster, more confident
    };

    // Adjust for distance (Fitts' Law approximation)
    let distance_factor = 1.0 / (1.0 + distance.log10().max(0.0) * 0.1);

    // Adjust for target size (larger targets = faster movement)
    let size_factor = 1.0 + (target_size_px.sqrt() * 0.01).min(0.5);

    // Adjust for importance (important elements get more careful approach)
    let importance_factor = match target_importance {
        ElementPriority::Critical => 0.8, // More careful
        ElementPriority::Normal => 1.0,
        ElementPriority::Optional => 1.2, // Less careful
    };

    let final_multiplier = base_multiplier * distance_factor * size_factor * importance_factor;

    CursorMovementConfig {
        speed_multiplier: final_multiplier.clamp(0.3, 2.0),
        min_step_delay_ms: (2.0 / final_multiplier).round() as u64,
        max_step_delay_variance_ms: (5.0 / final_multiplier).round() as u64,
        ..Default::default()
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum ExperienceLevel {
    Novice,
    Intermediate,
    Expert,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum ElementPriority {
    Critical,
    Normal,
    Optional,
}

/// Moves cursor along points with adaptive speed (slower near target)
async fn move_along_points_adaptive(
    page: &Page,
    points: &[Point],
    config: &mut CursorMovementConfig,
) -> Result<()> {
    let total_points = points.len();

    for (i, point) in points.iter().enumerate() {
        dispatch_mousemove(page, point.x, point.y).await?;

        // Phase 2: Adaptive speed - slow down near target for precision
        let progress = i as f64 / total_points as f64;
        let speed_adjustment = if progress > 0.8 {
            // Slow down to 70% speed in final 20%
            0.7
        } else if progress > 0.6 {
            // Moderate slowdown in final 40%
            0.85
        } else {
            1.0
        };

        let adjusted_min_delay = (config.min_step_delay_ms as f64 / speed_adjustment) as u64;
        let adjusted_max_variance =
            (config.max_step_delay_variance_ms as f64 / speed_adjustment) as u32;

        human_pause(adjusted_min_delay, adjusted_max_variance).await;

        // Include attention simulation
        if random_in_range(0, 100) < 8 {
            // 8% chance
            simulate_attention_drift(page, point.x, point.y).await?;
        } else if random_in_range(0, 100) < 12 {
            // 12% chance
            human_pause(random_in_range(50, 200), 20).await;
        }
    }

    Ok(())
}

/// Detects UI elements that would cause unwanted hovers along cursor path
async fn detect_ui_collisions_along_path(page: &Page, points: &[Point]) -> Result<Vec<Point>> {
    let mut collision_points = Vec::new();
    let sample_rate = 5; // Check every 5th point to optimize

    for (i, point) in points.iter().enumerate() {
        if i % sample_rate != 0 {
            continue; // Skip most points for performance
        }

        let js = format!(
            r#"(() => {{
                const el = document.elementFromPoint({}, {});
                if (el && el !== document.body && el !== document.documentElement) {{
                    // Check if it's a significant UI element
                    const tag = el.tagName.toLowerCase();
                    const role = el.getAttribute('role');
                    const ariaLabel = el.getAttribute('aria-label');

                    // Consider it a collision if it's interactive or labeled
                    if (tag === 'button' || tag === 'a' || tag === 'input' ||
                        tag === 'select' || role === 'button' || role === 'link' ||
                        (ariaLabel && ariaLabel.trim().length > 0)) {{
                        return {{ x: {}, y: {}, significant: true }};
                    }}
                }}
                return null;
            }})()"#,
            point.x, point.y, point.x, point.y
        );

        if let Ok(result) = page.evaluate(js).await {
            if let Some(obj) = result.value().and_then(|v| v.as_object()) {
                if obj
                    .get("significant")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    collision_points.push(*point);
                }
            }
        }
    }

    Ok(collision_points)
}

/// Generates an alternative cursor path that avoids detected collisions
fn generate_collision_free_path(
    original_points: &[Point],
    collision_points: &[Point],
    _config: &CursorMovementConfig,
) -> Vec<Point> {
    if collision_points.is_empty() {
        return original_points.to_vec();
    }

    // Simple approach: add intermediate waypoints to avoid collision areas
    let mut safe_points = Vec::new();
    safe_points.push(original_points[0]); // Start point

    for (i, &point) in original_points.iter().enumerate().skip(1) {
        // Check if this point is near a collision
        let near_collision = collision_points.iter().any(|&collision| {
            let dx = point.x - collision.x;
            let dy = point.y - collision.y;
            (dx * dx + dy * dy).sqrt() < 50.0 // 50px avoidance radius
        });

        if near_collision && i > 0 && i < original_points.len() - 1 {
            // Insert intermediate points to detour around collision
            let prev = original_points[i - 1];
            let detour_point = Point::new(
                (prev.x + point.x) / 2.0 + (point.y - prev.y) * 0.3, // Perpendicular offset
                (prev.y + point.y) / 2.0 - (point.x - prev.x) * 0.3,
            );
            safe_points.push(detour_point);
        }

        safe_points.push(point);
    }

    safe_points
}

/// Moves cursor along a series of points with human-like timing and attention simulation
#[allow(dead_code)]
async fn move_along_points(
    page: &Page,
    points: &[Point],
    config: &CursorMovementConfig,
) -> Result<()> {
    for (i, point) in points.iter().enumerate() {
        dispatch_mousemove(page, point.x, point.y).await?;

        if config.add_micro_pauses {
            let delay = (config.min_step_delay_ms as f64 / config.speed_multiplier) as u64;
            let variance =
                (config.max_step_delay_variance_ms as f64 / config.speed_multiplier) as u32;
            human_pause(delay, variance).await;

            // Phase 2: Attention simulation - occasional drift and micro-breaks
            if random_in_range(0, 100) < 8 {
                // 8% chance of attention drift
                simulate_attention_drift(page, point.x, point.y).await?;
            } else if random_in_range(0, 100) < 12 {
                // 12% chance of micro-pause
                human_pause(random_in_range(50, 200), 20).await;
            }

            // Phase 2: Fatigue effects - occasional longer pauses when "tired"
            if i > points.len() / 2 && random_in_range(0, 100) < 5 {
                // 5% chance in second half
                human_pause(random_in_range(300, 800), 30).await; // Fatigue pause
            }
        }
    }

    Ok(())
}

/// Simulates human attention drift - cursor briefly moves away then corrects back
async fn simulate_attention_drift(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    // Generate a small drift (10-30 pixels away)
    let drift_distance = random_in_range(10, 30) as f64;
    let drift_angle = random_in_range(0, 360) as f64 * std::f64::consts::PI / 180.0;

    let drift_x = target_x + drift_distance * drift_angle.cos();
    let drift_y = target_y + drift_distance * drift_angle.sin();

    // Ensure drift stays within reasonable bounds
    let clamped_drift_x = drift_x.clamp(0.0, 1920.0); // Assume 1920px viewport
    let clamped_drift_y = drift_y.clamp(0.0, 1080.0);

    // Move to drift position
    dispatch_mousemove(page, clamped_drift_x, clamped_drift_y).await?;
    human_pause(random_in_range(150, 400), 25).await; // Brief hesitation

    // Correct back to target
    dispatch_mousemove(page, target_x, target_y).await?;
    human_pause(random_in_range(100, 250), 20).await; // Settle back

    Ok(())
}

/// Checks if element is visually clickable (not obscured, enabled, etc.)
async fn is_element_clickable(page: &Page, selector: &str) -> Result<bool> {
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({});
            if (!el) return false;

            // Check CSS visibility
            const style = window.getComputedStyle(el);
            if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') {{
                return false;
            }}

            // Check if disabled
            if (el.disabled || el.getAttribute('aria-disabled') === 'true') {{
                return false;
            }}

            // Check bounding rect
            const rect = el.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) {{
                return false;
            }}

            // Check if obscured by checking element at center point
            const centerX = rect.left + rect.width / 2;
            const centerY = rect.top + rect.height / 2;

            // Make sure center is within viewport
            if (centerX < 0 || centerY < 0 ||
                centerX > window.innerWidth || centerY > window.innerHeight) {{
                return false;
            }}

            const topElement = document.elementFromPoint(centerX, centerY);
            if (!topElement) return false;

            // Element is clickable if it's the top element or contains it
            return el === topElement || el.contains(topElement);
        }})()"#,
        serde_json::to_string(selector)?
    );

    page.evaluate(js)
        .await?
        .value()
        .and_then(|v| v.as_bool())
        .ok_or_else(|| anyhow::anyhow!("Failed to evaluate clickability"))
}

pub async fn click_selector_human(
    page: &Page,
    selector: &str,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    click_offset_px: i32,
) -> Result<ClickOutcome> {
    // Phase 1: Smart element waiting and stability check
    if !wait_for_element_stability(page, selector, 5000).await? {
        return Err(anyhow::anyhow!(
            "Element '{}' not stable within 5s",
            selector
        ));
    }

    // Phase 1: Visual clickability confirmation
    if !is_element_clickable(page, selector).await? {
        return Err(anyhow::anyhow!("Element '{}' not clickable", selector));
    }

    // Use the improved human-like scroll_into_view
    use crate::utils::scroll;
    scroll::scroll_into_view(page, selector).await?;

    // Phase 2: Verify element is in viewport after scroll
    if !is_in_viewport_internal(page, selector).await? {
        return Err(anyhow::anyhow!(
            "Element '{}' not in viewport after scroll",
            selector
        ));
    }

    // Phase 2: Re-resolve bbox after scroll (may have shifted, with retry for stale selectors)
    let bbox = timeout(
        Duration::from_secs(2),
        resolve_selector_bbox_with_retry(page, selector, 3),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!("click resolve_selector_bbox timeout for selector={selector}")
    })??;

    let (x, y) = choose_click_point(&bbox, click_offset_px);

    // Phase 1: Enhanced cursor movement with collision avoidance
    move_cursor_collision_avoidant(page, x, y).await?;

    // Add human-like hover before clicking
    let element_type = detect_element_type(selector);
    hover_before_click(page, selector, x, y, &element_type).await?;

    timeout(
        Duration::from_secs(2),
        dispatch_click(page, x, y, MouseButton::Left),
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
    let trace_id = next_nativeclick_trace_id();
    let stability_wait_ms = native_interaction.stability_wait_ms.clamp(1_000, 30_000);
    if !wait_for_element_stability(page, selector, stability_wait_ms).await? {
        return Err(anyhow::anyhow!(
            "trace={} nativeclick element '{}' not stable within {}ms",
            trace_id,
            selector,
            stability_wait_ms
        ));
    }

    if !is_element_clickable(page, selector).await? {
        return Err(anyhow::anyhow!(
            "trace={} nativeclick element '{}' not clickable",
            trace_id,
            selector
        ));
    }

    let settle_ms = (reaction_delay_ms / 4)
        .clamp(40, 200)
        .max(native_interaction.settle_ms);
    let settle_variance = (reaction_delay_variance_pct / 3).max(10);
    let attention_pause_ms = (reaction_delay_ms / 4).clamp(40, 200);

    // Serialize the full native-click sequence so concurrent browser sessions
    // do not fight over the single global cursor or foreground focus.
    let _native_click_guard = acquire_native_input_lock(session_id, trace_id, "nativeclick").await;
    page.bring_to_front().await.map_err(|err| {
        anyhow::anyhow!(
            "trace={} nativeclick bring_to_front failed: {}",
            trace_id,
            err
        )
    })?;
    human_pause(attention_pause_ms, reaction_delay_variance_pct.min(45)).await;
    nativeclick_debug(session_id, trace_id, selector, "scroll-into-view", "start");
    scroll::scroll_into_view(page, selector).await?;

    let bbox = timeout(
        Duration::from_millis(native_interaction.resolve_timeout_ms.clamp(250, 30_000)),
        resolve_selector_bbox(page, selector),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "trace={} nativeclick resolve_selector_bbox timeout for selector={selector}",
            trace_id
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
    .map_err(|err| anyhow::anyhow!("trace={} nativeclick mapping failed: {}", trace_id, err))?;
    sync_native_overlay_position(page, content_x, content_y).await;
    page.bring_to_front().await.map_err(|err| {
        anyhow::anyhow!(
            "trace={} nativeclick bring_to_front failed: {}",
            trace_id,
            err
        )
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
        anyhow::anyhow!(
            "trace={} nativeclick dispatch failed for '{}': {}",
            trace_id,
            selector,
            err
        )
    })?;

    let verified = verify_click_target(page, selector, content_x, content_y)
        .await
        .map_err(|err| {
            anyhow::anyhow!(
                "trace={} nativeclick verification check failed: {}",
                trace_id,
                err
            )
        })?;
    sync_native_overlay_position(page, content_x, content_y).await;
    if !verified {
        return Err(anyhow::anyhow!(
            "trace={} nativeclick verification failed for '{}'",
            trace_id,
            selector
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

    cursor_move_to(page, start_x, start_y).await?;
    human_pause(reaction_delay_ms, reaction_delay_variance_pct).await;
    dispatch_mouse_action(page, start_x, start_y, 0, "mousedown").await?;

    let mid_x = (start_x + end_x) / 2.0;
    let mid_y = (start_y + end_y) / 2.0;
    cursor_move_to(page, mid_x, mid_y).await?;
    cursor_move_to(page, end_x, end_y).await?;

    dispatch_mouse_action(page, end_x, end_y, 0, "mouseup").await?;
    Ok(())
}

fn choose_click_point(bbox: &BoundingBox, click_offset_px: i32) -> (f64, f64) {
    let center_x = bbox.x + bbox.width / 2.0;
    let center_y = bbox.y + bbox.height / 2.0;

    let min_x = bbox.x + 1.0;
    let min_y = bbox.y + 1.0;
    let max_x = (bbox.x + bbox.width - 1.0).max(min_x);
    let max_y = (bbox.y + bbox.height - 1.0).max(min_y);

    let spread = (click_offset_px.abs() as f64).max(4.0);
    let spread_x = spread.min((bbox.width / 3.0).max(4.0));
    let spread_y = spread.min((bbox.height / 3.0).max(4.0));

    let x = gaussian(center_x, spread_x, min_x, max_x);
    let y = gaussian(center_y, spread_y, min_y, max_y);
    (x, y)
}

fn native_click_center_bounds(
    bbox: &BoundingBox,
    click_offset_px: i32,
) -> Option<(i32, i32, i32, i32)> {
    let center_x = bbox.x + bbox.width / 2.0;
    let center_y = bbox.y + bbox.height / 2.0;
    let spread = (click_offset_px.abs() as f64).max(4.0);
    let min_x = ((center_x - spread).ceil()).max((bbox.x + 1.0).ceil());
    let max_x = ((center_x + spread).floor()).min((bbox.x + bbox.width - 1.0).floor());
    let min_y = ((center_y - spread).ceil()).max((bbox.y + 1.0).ceil());
    let max_y = ((center_y + spread).floor()).min((bbox.y + bbox.height - 1.0).floor());

    if min_x > max_x || min_y > max_y {
        return None;
    }

    Some((min_x as i32, max_x as i32, min_y as i32, max_y as i32))
}

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
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;
            const hit = document.elementFromPoint({x}, {y});
            if (!hit) return false;
            return el === hit || el.contains(hit) || hit.contains(el);
        }})()"#
    );

    let result = timeout(Duration::from_millis(400), page.evaluate(js))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "trace={} nativeclick point hit-test timeout for selector={selector}",
                trace_id
            )
        })??;

    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

async fn resolve_native_click_point(
    page: &Page,
    session_id: &str,
    trace_id: u64,
    selector: &str,
    click_offset_px: i32,
    bbox: &BoundingBox,
) -> Result<(f64, f64)> {
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
                format!("content_point=({:.1},{:.1})", x, y),
            );
            return Ok((x, y));
        }
    }

    anyhow::bail!(
        "trace={} nativeclick could not resolve a verified point for selector={selector}",
        trace_id
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
        r#"(() => {{
            const query = {query_js};
            const root = document.body || document.documentElement;
            if (!root) return null;

            const pickPoint = (rect) => {{
                const centerX = Math.round(rect.left + rect.width / 2);
                const centerY = Math.round(rect.top + rect.height / 2);
                const minX = Math.max(Math.ceil(rect.left + 1), centerX - 4);
                const maxX = Math.min(Math.floor(rect.right - 1), centerX + 4);
                const minY = Math.max(Math.ceil(rect.top + 1), centerY - 4);
                const maxY = Math.min(Math.floor(rect.bottom - 1), centerY + 4);
                if (minX > maxX || minY > maxY) return null;
                return {{
                    x: minX + Math.floor(Math.random() * (maxX - minX + 1)),
                    y: minY + Math.floor(Math.random() * (maxY - minY + 1)),
                }};
            }};

            const labelFor = (el) => {{
                const tag = (el.tagName || 'element').toLowerCase();
                return el.id ? `${{tag}}#${{el.id}}` : tag;
            }};

            const matches = [];
            let nodes = [];
            try {{
                nodes = query ? Array.from(document.querySelectorAll(query)) : Array.from(root.querySelectorAll('*'));
            }} catch (err) {{
                return {{ error: String(err && err.message ? err.message : err) }};
            }}

            for (const el of nodes) {{
                if (!(el instanceof Element)) continue;
                if (el.id === '__auto_rust_mouse_overlay' || el.id === '__auto_rust_nativeclick_probe') continue;
                const rect = el.getBoundingClientRect();
                if (rect.width < 8 || rect.height < 8) continue;
                if (rect.bottom <= 0 || rect.right <= 0) continue;
                if (rect.top >= window.innerHeight || rect.left >= window.innerWidth) continue;
                const style = window.getComputedStyle(el);
                if (!style) continue;
                if (style.display === 'none' || style.visibility === 'hidden') continue;
                if (Number.parseFloat(style.opacity || '1') === 0) continue;
                if (el.getAttribute('aria-hidden') === 'true') continue;

                const point = pickPoint(rect);
                if (!point) continue;
                const hit = document.elementFromPoint(point.x, point.y);
                if (!hit) continue;
                if (!(el === hit || el.contains(hit) || hit.contains(el))) continue;

                matches.push({{
                    label: labelFor(el),
                    x: point.x,
                    y: point.y,
                }});
            }}

            if (!matches.length) return null;
            return matches[Math.floor(Math.random() * matches.length)];
        }})()"#,
    );

    let result = timeout(
        Duration::from_millis(native_interaction.resolve_timeout_ms.clamp(250, 30_000)),
        page.evaluate(js),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "trace={} nativecursor candidate lookup timed out for '{}'",
            trace_id,
            scope
        )
    })??;
    let value = result.value().cloned().ok_or_else(|| {
        anyhow::anyhow!(
            "trace={} nativecursor found no visible candidates for '{scope}'",
            trace_id
        )
    })?;

    if value.is_null() {
        anyhow::bail!(
            "trace={} nativecursor found no visible candidates for '{scope}'",
            trace_id
        );
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

/// Check if element is in viewport (internal helper, no permissions required).
/// Unlike TaskContext::is_in_viewport(), this doesn't require allow_dom_inspection permission.
async fn is_in_viewport_internal(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;
            const rect = el.getBoundingClientRect();
            const windowHeight = window.innerHeight || document.documentElement.clientHeight;
            const windowWidth = window.innerWidth || document.documentElement.clientWidth;
            return rect.top < windowHeight && rect.bottom > 0 &&
                   rect.left < windowWidth && rect.right > 0;
        }})()"#
    );

    let result = page
        .evaluate(js)
        .await?
        .value()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(result)
}

/// Resolve selector bbox with retry logic (handles stale selectors after scroll).
async fn resolve_selector_bbox_with_retry(
    page: &Page,
    selector: &str,
    max_retries: u32,
) -> Result<BoundingBox> {
    let mut last_err: Option<anyhow::Error> = None;

    for attempt in 0..=max_retries {
        match resolve_selector_bbox(page, selector).await {
            Ok(bbox) => {
                // Verify bbox is valid (non-zero dimensions)
                if bbox.width > 0.0 && bbox.height > 0.0 {
                    return Ok(bbox);
                }
                if attempt < max_retries {
                    human_pause(50, 20).await;
                    continue;
                }
                anyhow::bail!("Element '{}' has invalid bounds after {} retries", selector, max_retries);
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
        anyhow::anyhow!("Failed to resolve bbox for '{}' after {} retries", selector, max_retries)
    }))
}

async fn get_selector_bbox_once(page: &Page, selector: &str) -> Result<BoundingBox> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const r = el.getBoundingClientRect();
            return {{ x: r.x, y: r.y, width: r.width, height: r.height }};
        }})()"#
    );

    let result = timeout(Duration::from_millis(800), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("bbox lookup timeout for selector={selector}"))??;

    let obj = result
        .value()
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow::anyhow!("Element not found: {selector}"))?;

    let bbox = BoundingBox {
        x: obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
        y: obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
        width: obj.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0),
        height: obj.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0),
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
    scroll::scroll_into_view(page, selector).await?;

    let bbox = resolve_selector_bbox(page, selector).await?;
    let (x, y) = choose_click_point(&bbox, click_offset_px);
    cursor_move_to(page, x, y).await?;

    // Add human-like hover before clicking
    let element_type = detect_element_type(selector);
    hover_before_click(page, selector, x, y, &element_type).await?;

    dispatch_click(page, x, y, button).await?;

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
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;
            const rect = el.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) return false;
            const hit = document.elementFromPoint({x}, {y});
            if (!hit) return false;
            return el === hit || el.contains(hit) || hit.contains(el);
        }})()"#
    );

    let result = timeout(Duration::from_millis(500), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("click verification timeout"))??;

    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
struct BrowserWindowMetrics {
    screen_x: f64,
    screen_y: f64,
    outer_width: f64,
    outer_height: f64,
    inner_width: f64,
    inner_height: f64,
    device_pixel_ratio: f64,
    visual_viewport_scale: f64,
    visual_viewport_offset_left: f64,
    visual_viewport_offset_top: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScreenPoint {
    x: i32,
    y: i32,
}

#[derive(Debug, Clone, Copy)]
struct NativeClickCalibration {
    scale_x: f64,
    scale_y: f64,
    origin_adjust_x: f64,
    origin_adjust_y: f64,
    mode: NativeClickCalibrationMode,
}

#[derive(Debug, Clone, Copy)]
struct NativeDispatchOptions {
    backend: crate::config::NativeInputBackend,
    reaction_delay_ms: u64,
    reaction_delay_variance_pct: u32,
    settle_ms: u64,
    settle_variance_pct: u32,
}

const NATIVE_CLICK_PROBE_ID: &str = "__auto_rust_nativeclick_probe";
const NATIVE_CLICK_PROBE_HIT_FLAG: &str = "__auto_rust_nativeclick_probe_hit";

fn browser_content_origin(
    metrics: &BrowserWindowMetrics,
    scale_x: f64,
    scale_y: f64,
    mode: NativeClickCalibrationMode,
) -> (f64, f64) {
    let chrome_y = (metrics.outer_height - metrics.inner_height).max(0.0);
    let chrome_x = match mode {
        NativeClickCalibrationMode::Windows => {
            ((metrics.outer_width - metrics.inner_width).max(0.0)) / 2.0
        }
        NativeClickCalibrationMode::Mac | NativeClickCalibrationMode::Linux => 0.0,
    };
    (
        metrics.screen_x + chrome_x + metrics.visual_viewport_offset_left * scale_x,
        metrics.screen_y + chrome_y + metrics.visual_viewport_offset_top * scale_y,
    )
}

fn browser_scale(metrics: &BrowserWindowMetrics, _mode: NativeClickCalibrationMode) -> f64 {
    (metrics.device_pixel_ratio / metrics.visual_viewport_scale.max(1.0)).clamp(0.5, 4.0)
}

fn native_click_calibration_from_metrics(
    metrics: &BrowserWindowMetrics,
    mode: NativeClickCalibrationMode,
) -> NativeClickCalibration {
    let scale = browser_scale(metrics, mode);
    NativeClickCalibration {
        scale_x: scale,
        scale_y: scale,
        origin_adjust_x: 0.0,
        origin_adjust_y: 0.0,
        mode,
    }
}

fn native_click_fingerprint(
    metrics: &BrowserWindowMetrics,
    mode: NativeClickCalibrationMode,
) -> NativeClickFingerprint {
    NativeClickFingerprint {
        mode,
        screen_x: metrics.screen_x.round() as i32,
        screen_y: metrics.screen_y.round() as i32,
        outer_width: metrics.outer_width.round() as i32,
        outer_height: metrics.outer_height.round() as i32,
        inner_width: metrics.inner_width.round() as i32,
        inner_height: metrics.inner_height.round() as i32,
        device_pixel_ratio_milli: (metrics.device_pixel_ratio * 1000.0).round() as i32,
        visual_viewport_scale_milli: (metrics.visual_viewport_scale * 1000.0).round() as i32,
    }
}

fn screen_point_from_calibration(
    metrics: &BrowserWindowMetrics,
    calibration: &NativeClickCalibration,
    x: f64,
    y: f64,
) -> ScreenPoint {
    let (origin_x, origin_y) = browser_content_origin(
        metrics,
        calibration.scale_x,
        calibration.scale_y,
        calibration.mode,
    );
    let screen_x = (origin_x + calibration.origin_adjust_x + x * calibration.scale_x).round();
    let screen_y = (origin_y + calibration.origin_adjust_y + y * calibration.scale_y).round();
    ScreenPoint {
        x: screen_x.clamp(i32::MIN as f64, i32::MAX as f64) as i32,
        y: screen_y.clamp(i32::MIN as f64, i32::MAX as f64) as i32,
    }
}

fn cached_native_click_calibration(
    cache_key: &str,
    fingerprint: NativeClickFingerprint,
) -> Option<NativeClickCalibration> {
    NATIVE_CLICK_CALIBRATION_CACHE
        .lock()
        .ok()
        .and_then(|cache| {
            cache.get(cache_key).and_then(|entry| {
                if entry.fingerprint == fingerprint {
                    Some(entry.calibration)
                } else {
                    None
                }
            })
        })
}

fn store_native_click_calibration(
    cache_key: &str,
    fingerprint: NativeClickFingerprint,
    calibration: NativeClickCalibration,
) {
    if let Ok(mut cache) = NATIVE_CLICK_CALIBRATION_CACHE.lock() {
        cache.insert(
            cache_key.to_string(),
            NativeClickCalibrationEntry {
                fingerprint,
                calibration,
            },
        );
    }
}

fn native_click_probe_scale_candidates(base_scale: f64) -> Vec<f64> {
    let mut scales: Vec<f64> = Vec::new();
    let mut push_scale = |scale: f64| {
        let scale = scale.clamp(0.5, 4.0);
        if !scales
            .iter()
            .any(|existing| (*existing - scale).abs() < 0.001)
        {
            scales.push(scale);
        }
    };

    for scale in [0.75, 0.88, 1.0, 1.12, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 3.0] {
        push_scale(scale);
    }

    for multiplier in [0.5, 0.66, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5] {
        push_scale(base_scale * multiplier);
    }

    scales.sort_by(|a, b| {
        let da = (a - base_scale).abs();
        let db = (b - base_scale).abs();
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });
    scales
}

async fn native_click_probe_point(page: &Page) -> Result<Option<(f64, f64)>> {
    let js = format!(
        r#"(function() {{
            const value = window.{probe_point_flag};
            if (!value) return null;
            return {{ x: value.x ?? null, y: value.y ?? null }};
        }})()"#,
        probe_point_flag = "__auto_rust_nativeclick_probe_point",
    );
    let result = timeout(Duration::from_millis(400), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("nativeclick probe point read timed out"))??;
    let value = result.value().cloned();
    let Some(obj) = value.and_then(|v| v.as_object().cloned()) else {
        return Ok(None);
    };
    let x = obj.get("x").and_then(|v| v.as_f64());
    let y = obj.get("y").and_then(|v| v.as_f64());
    Ok(match (x, y) {
        (Some(x), Some(y)) => Some((x, y)),
        _ => None,
    })
}

async fn inject_native_click_probe(page: &Page) -> Result<()> {
    inject_native_click_probe_at(page, 64.0, 64.0).await
}

async fn inject_native_click_probe_at(page: &Page, left: f64, top: f64) -> Result<()> {
    let js = format!(
        r#"(function() {{
            window.{hit_flag} = false;
            window.__auto_rust_nativeclick_probe_point = null;
            let probe = document.getElementById({probe_id});
            if (!probe) {{
                probe = document.createElement('button');
                probe.id = {probe_id};
                probe.type = 'button';
                probe.textContent = '';
                probe.style.position = 'fixed';
                probe.style.left = '{left}px';
                probe.style.top = '{top}px';
                probe.style.width = '48px';
                probe.style.height = '48px';
                probe.style.margin = '0';
                probe.style.padding = '0';
                probe.style.border = '0';
                probe.style.opacity = '0.01';
                probe.style.background = 'transparent';
                probe.style.zIndex = '2147483647';
                probe.style.pointerEvents = 'auto';
                probe.style.cursor = 'default';
                probe.onclick = (evt) => {{
                    window.{hit_flag} = true;
                    window.__auto_rust_nativeclick_probe_point = {{ x: evt.clientX, y: evt.clientY }};
                }};
                (document.body || document.documentElement).appendChild(probe);
            }} else {{
                probe.style.left = '{left}px';
                probe.style.top = '{top}px';
                probe.style.width = '48px';
                probe.style.height = '48px';
                probe.style.opacity = '0.01';
                probe.style.pointerEvents = 'auto';
                probe.style.zIndex = '2147483647';
                probe.onclick = (evt) => {{
                    window.{hit_flag} = true;
                    window.__auto_rust_nativeclick_probe_point = {{ x: evt.clientX, y: evt.clientY }};
                }};
            }}
            return true;
        }})()"#,
        probe_id = serde_json::to_string(NATIVE_CLICK_PROBE_ID)?,
        hit_flag = NATIVE_CLICK_PROBE_HIT_FLAG,
        left = left,
        top = top,
    );
    timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("nativeclick probe injection timed out"))??;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct NativeClickProbeSample {
    desired_x: f64,
    desired_y: f64,
    hit_x: f64,
    hit_y: f64,
}

async fn measure_native_click_probe(
    page: &Page,
    trace_id: u64,
    candidate: &NativeClickCalibration,
    metrics: &BrowserWindowMetrics,
    desired_x: f64,
    desired_y: f64,
    native_interaction: &NativeInteractionConfig,
) -> Result<Option<NativeClickProbeSample>> {
    inject_native_click_probe_at(page, desired_x - 24.0, desired_y - 24.0).await?;
    sync_native_overlay_position(page, desired_x, desired_y).await;
    let probe_point = screen_point_from_calibration(metrics, candidate, desired_x, desired_y);
    native_move_and_click_point(
        trace_id,
        probe_point.x,
        probe_point.y,
        NativeDispatchOptions {
            backend: native_interaction.native_input_backend,
            reaction_delay_ms: 10,
            reaction_delay_variance_pct: 5,
            settle_ms: 5,
            settle_variance_pct: 5,
        },
    )
    .await?;
    if !native_click_probe_hit(page).await.unwrap_or(false) {
        return Ok(None);
    }
    let Some((hit_x, hit_y)) = native_click_probe_point(page).await? else {
        return Ok(None);
    };
    Ok(Some(NativeClickProbeSample {
        desired_x,
        desired_y,
        hit_x,
        hit_y,
    }))
}

fn solve_calibration_from_probe_samples(
    metrics: &BrowserWindowMetrics,
    candidate: NativeClickCalibration,
    first: NativeClickProbeSample,
    second: NativeClickProbeSample,
) -> Option<NativeClickCalibration> {
    let desired_dx = second.desired_x - first.desired_x;
    let desired_dy = second.desired_y - first.desired_y;
    let hit_dx = second.hit_x - first.hit_x;
    let hit_dy = second.hit_y - first.hit_y;

    let scale_x = if hit_dx.abs() >= 1.0 && desired_dx.abs() >= 1.0 {
        candidate.scale_x * (desired_dx / hit_dx)
    } else {
        candidate.scale_x
    };
    let scale_y = if hit_dy.abs() >= 1.0 && desired_dy.abs() >= 1.0 {
        candidate.scale_y * (desired_dy / hit_dy)
    } else {
        candidate.scale_y
    };

    if !scale_x.is_finite() || !scale_y.is_finite() {
        return None;
    }

    let scale_x = scale_x.clamp(0.5, 4.0);
    let scale_y = scale_y.clamp(0.5, 4.0);
    let (candidate_origin_x, candidate_origin_y) = browser_content_origin(
        metrics,
        candidate.scale_x,
        candidate.scale_y,
        candidate.mode,
    );
    let (refined_origin_x, refined_origin_y) =
        browser_content_origin(metrics, scale_x, scale_y, candidate.mode);

    let origin_adjust_x = candidate.origin_adjust_x
        + (candidate_origin_x - refined_origin_x)
        + first.desired_x * candidate.scale_x
        - first.hit_x * scale_x;
    let origin_adjust_y = candidate.origin_adjust_y
        + (candidate_origin_y - refined_origin_y)
        + first.desired_y * candidate.scale_y
        - first.hit_y * scale_y;

    Some(NativeClickCalibration {
        scale_x,
        scale_y,
        origin_adjust_x,
        origin_adjust_y,
        mode: candidate.mode,
    })
}

async fn native_click_probe_hit(page: &Page) -> Result<bool> {
    let js = format!(
        r#"(function() {{
            return Boolean(window.{hit_flag});
        }})()"#,
        hit_flag = NATIVE_CLICK_PROBE_HIT_FLAG,
    );
    let result = timeout(Duration::from_millis(400), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("nativeclick probe verification timed out"))??;
    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

async fn remove_native_click_probe(page: &Page) -> Result<()> {
    let js = format!(
        r#"(function() {{
            const probe = document.getElementById({probe_id});
            if (probe && probe.parentNode) {{
                probe.parentNode.removeChild(probe);
            }}
            try {{
                delete window.{hit_flag};
            }} catch (err) {{
                window.{hit_flag} = false;
            }}
            try {{
                delete window.__auto_rust_nativeclick_probe_point;
            }} catch (err) {{
                window.__auto_rust_nativeclick_probe_point = null;
            }}
            return true;
        }})()"#,
        probe_id = serde_json::to_string(NATIVE_CLICK_PROBE_ID)?,
        hit_flag = NATIVE_CLICK_PROBE_HIT_FLAG,
    );
    timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("nativeclick probe cleanup timed out"))??;
    Ok(())
}

async fn calibrate_native_click(
    page: &Page,
    session_id: &str,
    trace_id: u64,
    metrics: &BrowserWindowMetrics,
    native_interaction: &NativeInteractionConfig,
) -> Result<NativeClickCalibration> {
    let mode = native_interaction.calibration_mode;
    let fingerprint = native_click_fingerprint(metrics, mode);
    let cache_key = format!("{}::{}", session_id, page.target_id().as_ref());

    if let Some(calibration) = cached_native_click_calibration(&cache_key, fingerprint) {
        nativeclick_debug(
            session_id,
            trace_id,
            "__calibration__",
            "cache-hit",
            format!("mode={} fingerprint={:?}", mode.as_str(), fingerprint),
        );
        return Ok(calibration);
    }

    if let Ok(cache) = NATIVE_CLICK_CALIBRATION_CACHE.lock() {
        if let Some(entry) = cache.get(&cache_key) {
            if entry.fingerprint != fingerprint {
                nativeclick_debug(
                    session_id,
                    trace_id,
                    "__calibration__",
                    "invalidated",
                    format!("old={:?} new={:?}", entry.fingerprint, fingerprint),
                );
            }
        }
    }

    let base_calibration = native_click_calibration_from_metrics(metrics, mode);
    inject_native_click_probe(page).await?;

    let probe_first_x = 88.0;
    let probe_first_y = 88.0;
    let probe_second_x = 248.0;
    let probe_second_y = 188.0;
    let candidate_scales = native_click_probe_scale_candidates(base_calibration.scale_x);
    let mut last_error: Option<anyhow::Error> = None;

    for scale in candidate_scales {
        let candidate = NativeClickCalibration {
            scale_x: scale,
            scale_y: scale,
            ..base_calibration
        };
        let first = match measure_native_click_probe(
            page,
            trace_id,
            &candidate,
            metrics,
            probe_first_x,
            probe_first_y,
            native_interaction,
        )
        .await
        {
            Ok(Some(sample)) => sample,
            Ok(None) => continue,
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };
        let second = match measure_native_click_probe(
            page,
            trace_id,
            &candidate,
            metrics,
            probe_second_x,
            probe_second_y,
            native_interaction,
        )
        .await
        {
            Ok(Some(sample)) => sample,
            Ok(None) => continue,
            Err(err) => {
                last_error = Some(err);
                continue;
            }
        };

        if let Some(refined) =
            solve_calibration_from_probe_samples(metrics, candidate, first, second)
        {
            nativeclick_debug(
                session_id,
                trace_id,
                "__calibration__",
                "probe-hit",
                format!(
                    "fingerprint={:?} first_hit=({:.1},{:.1}) second_hit=({:.1},{:.1}) scale=({:.3},{:.3}) adjust=({:.1},{:.1})",
                    fingerprint,
                    first.hit_x,
                    first.hit_y,
                    second.hit_x,
                    second.hit_y,
                    refined.scale_x,
                    refined.scale_y,
                    refined.origin_adjust_x,
                    refined.origin_adjust_y
                ),
            );
            store_native_click_calibration(&cache_key, fingerprint, refined);
            let _ = remove_native_click_probe(page).await;
            return Ok(refined);
        }
    }

    let _ = remove_native_click_probe(page).await;
    if let Some(err) = last_error {
        nativeclick_debug(
            session_id,
            trace_id,
            "__calibration__",
            "probe-failed",
            format!("using fallback metrics calibration: {err}"),
        );
    }

    nativeclick_debug(
        session_id,
        trace_id,
        "__calibration__",
        "fallback",
        format!(
            "fingerprint={:?} scale={:.3}",
            fingerprint, base_calibration.scale_x
        ),
    );
    Ok(base_calibration)
}

async fn browser_window_metrics(page: &Page) -> Result<BrowserWindowMetrics> {
    let js = r#"(() => ({
        screen_x: window.screenX ?? window.screenLeft ?? 0,
        screen_y: window.screenY ?? window.screenTop ?? 0,
        outer_width: window.outerWidth ?? window.innerWidth,
        outer_height: window.outerHeight ?? window.innerHeight,
        inner_width: window.innerWidth,
        inner_height: window.innerHeight,
        device_pixel_ratio: window.devicePixelRatio ?? 1,
        visual_viewport_scale: window.visualViewport ? window.visualViewport.scale : 1,
        visual_viewport_offset_left: window.visualViewport ? window.visualViewport.offsetLeft : 0,
        visual_viewport_offset_top: window.visualViewport ? window.visualViewport.offsetTop : 0,
    }))()"#;

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

async fn content_point_to_screen_point(
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
        .map_err(|err| anyhow::anyhow!("nativeclick calibration invalid: {}", err))?;
    Ok(screen_point_from_calibration(&metrics, &calibration, x, y))
}

fn validate_native_calibration(calibration: &NativeClickCalibration) -> Result<()> {
    let finite = calibration.scale_x.is_finite()
        && calibration.scale_y.is_finite()
        && calibration.origin_adjust_x.is_finite()
        && calibration.origin_adjust_y.is_finite();
    if !finite {
        anyhow::bail!("non-finite calibration value");
    }
    if calibration.scale_x <= 0.0 || calibration.scale_y <= 0.0 {
        anyhow::bail!("scale must be > 0");
    }
    if calibration.scale_x > 8.0 || calibration.scale_y > 8.0 {
        anyhow::bail!("scale exceeds max bound");
    }
    Ok(())
}

#[cfg(test)]
fn jittered_delay_ms(base_ms: u64, variance_pct: u32) -> u64 {
    if base_ms == 0 {
        return 0;
    }

    let mut rng = rand::thread_rng();
    let variance = ((base_ms as f64) * (variance_pct as f64 / 100.0)).round() as u64;
    if variance == 0 {
        return base_ms;
    }

    let lower = base_ms.saturating_sub(variance);
    let upper = base_ms.saturating_add(variance);
    rng.gen_range(lower..=upper)
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
    .map_err(|err| anyhow::anyhow!("trace={} nativeclick join error: {err}", trace_id))?
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
    .map_err(|err| anyhow::anyhow!("trace={} nativecursor join error: {err}", trace_id))?
}

#[allow(dead_code)]
pub fn fitts_law_optimal_size(distance: f64, time: f64) -> f64 {
    let id = time / 100.0;
    2.0 * distance / (2.0_f64.powf(id))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let points = generate_bezier_curve_with_config(&start, &end, &config);
        assert!(!points.is_empty());
        assert_eq!(points.first().map(|p| p.x), Some(0.0));
        assert_eq!(points.last().map(|p| p.x), Some(100.0));
    }

    #[test]
    fn test_arc_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);
        let points = generate_arc_curve(&start, &end);
        assert!(!points.is_empty());
    }

    #[test]
    fn test_zigzag_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);
        let points = generate_zigzag_curve(&start, &end);
        assert!(!points.is_empty());
    }

    #[test]
    fn test_overshoot_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(50.0, 50.0);
        let points = generate_overshoot_curve(&start, &end);
        assert_eq!(points.len(), 3);
    }

    #[test]
    fn test_stopped_curve_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 200.0);
        let points = generate_stopped_curve(&start, &end);
        assert!(points.len() >= 2);
    }

    #[test]
    fn test_muscle_path_generation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);
        let points = generate_muscle_path(&start, &end);
        assert!(!points.is_empty());
    }

    #[test]
    fn test_fitts_law_optimal_size() {
        let size = fitts_law_optimal_size(100.0, 500.0);
        assert!(size > 0.0);
    }

    #[test]
    fn test_fitts_law_zero_time() {
        let size = fitts_law_optimal_size(100.0, 0.0);
        // id = 0/100 = 0, so 2^0 = 1, 2*100/1 = 200
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

        let bounds = native_click_center_bounds(&bbox, 0).unwrap();
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
        assert_eq!(x, 100.0);
        assert_eq!(y, 180.0);
    }

    #[test]
    fn test_two_probe_solver_recovers_scale_and_origin_adjustment() {
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
        let candidate = NativeClickCalibration {
            scale_x: 1.0,
            scale_y: 1.0,
            origin_adjust_x: 0.0,
            origin_adjust_y: 0.0,
            mode: NativeClickCalibrationMode::Windows,
        };
        let first = NativeClickProbeSample {
            desired_x: 88.0,
            desired_y: 88.0,
            hit_x: 68.0,
            hit_y: 78.0,
        };
        let second = NativeClickProbeSample {
            desired_x: 248.0,
            desired_y: 188.0,
            hit_x: 148.0,
            hit_y: 128.0,
        };

        let solved =
            solve_calibration_from_probe_samples(&metrics, candidate, first, second).unwrap();

        assert!((solved.scale_x - 2.0).abs() < 0.001);
        assert!((solved.scale_y - 2.0).abs() < 0.001);
        assert!((solved.origin_adjust_x + 48.0).abs() < 0.001);
        assert!((solved.origin_adjust_y + 68.0).abs() < 0.001);
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
        // Test that the functions don't panic
        set_overlay_enabled(true);
        let _ = is_overlay_enabled();
        set_overlay_enabled(false);
        let _ = is_overlay_enabled();
    }

    #[test]
    fn test_is_overlay_enabled_default() {
        // Test that the function works without panicking
        let _ = is_overlay_enabled();
    }
}
