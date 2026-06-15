//! Cursor overlay rendering and mouse movement configuration.
//!
//! Manages the visual cursor overlay injected into browser pages, along with
//! cursor movement configuration types and path-style enums.

use super::trajectory::{self, Point};
use super::types::MouseButton;
use crate::state::{
    are_all_overlays_enabled, overlay_for_page, set_overlay_enabled_for_all, SessionOverlayState,
};
use crate::utils::math::random_in_range;
use crate::utils::page_size::get_viewport;
use crate::utils::timing::human_pause;
use anyhow::Result;
use chromiumoxide::Page;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{timeout, Duration};

const OVERLAY_SYNC_INTERVAL_MS: u64 = 50;
const DEFAULT_OVERLAY_SIZE_PX: f64 = 12.0;
const MIN_OVERLAY_SIZE_PX: f64 = 4.0;
const MAX_OVERLAY_SIZE_PX: f64 = 64.0;

static OVERLAY_SIZE_PX: std::sync::LazyLock<f64> = std::sync::LazyLock::new(|| {
    std::env::var("MOUSE_OVERLAY_SIZE_PX")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .map_or(DEFAULT_OVERLAY_SIZE_PX, |value| {
            value.clamp(MIN_OVERLAY_SIZE_PX, MAX_OVERLAY_SIZE_PX)
        })
});

/// Internal helper for logging within nativeclick context.
pub(crate) fn with_nativeclick_log_context(session_id: &str, f: impl FnOnce()) {
    let mut ctx = crate::logger::get_log_context();
    ctx.session_id = Some(session_id.to_string());
    let _guard = crate::logger::scoped_log_context(ctx);
    f();
}

pub(crate) fn nativeclick_debug(
    session_id: &str,
    trace_id: u64,
    selector: &str,
    phase: &str,
    message: impl std::fmt::Display,
) {
    super::native::record_nativeclick_trace_phase(session_id, phase);
    with_nativeclick_log_context(session_id, || {
        log::debug!(
            "nativeclick trace={trace_id} session={session_id} selector={selector} phase={phase}: {message}"
        );
    });
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

// MouseButton impl - enum defined in types.rs
impl MouseButton {
    pub(crate) fn as_button_index(&self) -> u16 {
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
    pub fn with_speed(mut self, speed: Speed) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_precision(mut self, precision: Precision) -> Self {
        self.precision = precision;
        self
    }

    pub fn with_path_style(mut self, path_style: PathStyle) -> Self {
        self.path_style = path_style;
        self
    }

    pub(crate) fn speed_config(&self) -> (f64, (u64, u64), bool) {
        match self.speed {
            Speed::Fast => (0.1, (1, 3), true),
            Speed::Normal => (0.5, (2, 5), false),
            Speed::Slow => (1.0, (5, 10), false),
        }
    }
}

pub fn set_overlay_enabled(enabled: bool) {
    set_overlay_enabled_for_all(enabled);
}

#[must_use]
pub fn is_overlay_enabled() -> bool {
    are_all_overlays_enabled()
}

pub(crate) fn overlay_state_for_page(page: &Page) -> Option<Arc<SessionOverlayState>> {
    overlay_for_page(page.target_id().as_ref())
}

pub(crate) fn cursor_start_position(
    page: &Page,
    viewport: &crate::utils::page_size::Viewport,
) -> (f64, f64) {
    if let Some(overlay_state) = overlay_state_for_page(page) {
        return overlay_state.cursor_start_position(viewport);
    }
    (viewport.width / 2.0, viewport.height / 2.0)
}

pub async fn cursor_move_to(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    cursor_move_to_with_config(page, target_x, target_y, &CursorMovementConfig::default()).await
}

#[allow(clippy::cast_precision_loss)]
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

pub(crate) async fn dispatch_mousemove(page: &Page, x: f64, y: f64) -> Result<()> {
    dispatch_mousemove_dom(page, x, y).await?;
    sync_cursor_overlay(page).await.ok();
    Ok(())
}

pub(crate) async fn sync_native_overlay_position(page: &Page, x: f64, y: f64) {
    if let Some(overlay_state) = overlay_state_for_page(page) {
        overlay_state.set_cursor_position(x, y);
        sync_cursor_overlay_force(page).await.ok();
    }
}

async fn dispatch_mousemove_dom(page: &Page, x: f64, y: f64) -> Result<()> {
    if let Some(overlay_state) = overlay_state_for_page(page) {
        overlay_state.set_cursor_position(x, y);
    }

    if super::cdp::dispatch_mouse_event_cdp(
        page,
        chromiumoxide::cdp::browser_protocol::input::DispatchMouseEventType::MouseMoved,
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
        r"(function() {{
            const el = document.elementFromPoint({x}, {y});
            if (!el) return;
            const evt = new MouseEvent('mousemove', {{
                bubbles: true,
                cancelable: true,
                clientX: {x},
                clientY: {y},
                button: 0
            }});
            el.dispatchEvent(evt);
        }})()"
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
            log::debug!("[{session_id}] cursor overlay error: {e}");
        }
    }
}

pub(crate) fn now_unix_ms() -> u64 {
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

    log::debug!("Syncing cursor overlay to ({x}, {y})");
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

// Path generation functions delegate to trajectory module
pub(crate) fn generate_bezier_curve_with_config(
    start: &Point,
    end: &Point,
    config: &CursorMovementConfig,
) -> Vec<Point> {
    trajectory::generate_bezier_curve_with_config(start, end, config.curve_spread, config.steps)
}

pub(crate) fn generate_arc_curve(start: &Point, end: &Point) -> Vec<Point> {
    trajectory::generate_arc_curve(start, end)
}

pub(crate) fn generate_zigzag_curve(start: &Point, end: &Point) -> Vec<Point> {
    trajectory::generate_zigzag_curve(start, end)
}

pub(crate) fn generate_overshoot_curve(start: &Point, end: &Point) -> Vec<Point> {
    trajectory::generate_overshoot_curve(start, end)
}

pub(crate) fn generate_stopped_curve(start: &Point, end: &Point) -> Vec<Point> {
    trajectory::generate_stopped_curve(start, end)
}

pub(crate) fn generate_muscle_path(start: &Point, end: &Point) -> Vec<Point> {
    trajectory::generate_muscle_path(start, end)
}
