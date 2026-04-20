//! Mouse simulation and human-computer interaction utilities.
//!
//! Provides functions for simulating realistic mouse movements and clicks:
//! - Human-like mouse movement using Bezier curves and various path styles
//! - Click simulation with proper timing and precision
//! - Fitts's Law calculations for optimal target sizing
//! - Configurable velocity and trajectory randomization
//! - Utilities for human-computer interaction studies

use chromiumoxide::Page;
use anyhow::Result;
use crate::utils::geometry::BoundingBox;
use crate::utils::math::{random_in_range, gaussian};
use crate::utils::scroll;
use crate::utils::timing::human_pause;
use crate::utils::page_size::get_viewport;
use log::debug;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use tokio::time::{timeout, Duration, sleep};

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
}

#[derive(Debug, Clone, Copy)]
pub struct HoverOutcome {
    pub hover: HoverStatus,
    pub x: f64,
    pub y: f64,
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
            BoundingBox { x, y, width, height }
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


static MOUSE_OVERLAY_ENABLED: AtomicBool = AtomicBool::new(true);
static CURSOR_INITIALIZED: AtomicBool = AtomicBool::new(false);
static CURSOR_X: AtomicU64 = AtomicU64::new(0);
static CURSOR_Y: AtomicU64 = AtomicU64::new(0);
static LAST_OVERLAY_SYNC_MS: AtomicU64 = AtomicU64::new(0);
const OVERLAY_SYNC_INTERVAL_MS: u64 = 100;

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


#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
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
    MOUSE_OVERLAY_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn is_overlay_enabled() -> bool {
    MOUSE_OVERLAY_ENABLED.load(Ordering::Relaxed)
}

fn set_cursor_position(x: f64, y: f64) {
    CURSOR_X.store(x.to_bits(), Ordering::Relaxed);
    CURSOR_Y.store(y.to_bits(), Ordering::Relaxed);
    CURSOR_INITIALIZED.store(true, Ordering::Relaxed);
}

fn cursor_position() -> (f64, f64) {
    (
        f64::from_bits(CURSOR_X.load(Ordering::Relaxed)),
        f64::from_bits(CURSOR_Y.load(Ordering::Relaxed)),
    )
}

fn cursor_position_snapshot() -> Option<(f64, f64)> {
    if CURSOR_INITIALIZED.load(Ordering::Relaxed) {
        Some(cursor_position())
    } else {
        None
    }
}

fn cursor_start_position(viewport: &crate::utils::page_size::Viewport) -> (f64, f64) {
    if let Some((x, y)) = cursor_position_snapshot() {
        if x.is_finite()
            && y.is_finite()
            && x >= 0.0
            && y >= 0.0
            && x <= viewport.width
            && y <= viewport.height
        {
            return (x, y);
        }
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
    let (start_x, start_y) = cursor_start_position(&viewport);

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
            let delay = (config.min_step_delay_ms as f64 / config.speed_multiplier / move_multiplier) as u64;
            let variance = (config.max_step_delay_variance_ms as f64 / config.speed_multiplier / move_multiplier) as u32;
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
    set_cursor_position(x, y);
    dispatch_mousemove_dom(page, x, y).await?;
    sync_cursor_overlay(page).await.ok();

    Ok(())
}

async fn dispatch_mousemove_dom(page: &Page, x: f64, y: f64) -> Result<()> {
    set_cursor_position(x, y);
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

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn claim_overlay_sync_slot(now_ms: u64, force: bool) -> bool {
    loop {
        let last = LAST_OVERLAY_SYNC_MS.load(Ordering::Relaxed);
        if !force && now_ms.saturating_sub(last) < OVERLAY_SYNC_INTERVAL_MS {
            return false;
        }
        if LAST_OVERLAY_SYNC_MS
            .compare_exchange(last, now_ms, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return true;
        }
    }
}

async fn sync_cursor_overlay_with_mode(page: &Page, force: bool) -> Result<()> {
    if !is_overlay_enabled() {
        return Ok(());
    }

    let (x, y) = if let Some((x, y)) = cursor_position_snapshot() {
        (x, y)
    } else {
        // Initialize overlay position at viewport center so the cursor dot is visible
        // before the first explicit mouse movement.
        let viewport = timeout(Duration::from_millis(500), get_viewport(page))
            .await
            .map_err(|_| anyhow::anyhow!("sync_cursor_overlay viewport timeout"))??;
        let cx = viewport.width / 2.0;
        let cy = viewport.height / 2.0;
        set_cursor_position(cx, cy);
        (cx, cy)
    };
    let now_ms = now_unix_ms();
    if !claim_overlay_sync_slot(now_ms, force) {
        return Ok(());
    }

    let eval = page.evaluate(format!(
        "(function() {{
            let dot = document.getElementById('__auto_rust_mouse_overlay');
            if (!dot) {{
                dot = document.createElement('div');
                dot.id = '__auto_rust_mouse_overlay';
                dot.style.position = 'fixed';
                dot.style.width = '12px';
                dot.style.height = '12px';
                dot.style.borderRadius = '50%';
                dot.style.background = '#00ff00';
                dot.style.border = '1px solid #00cc00';
                dot.style.boxShadow = '0 0 6px #00ff00';
                dot.style.pointerEvents = 'none';
                dot.style.zIndex = '2147483647';
                document.body.appendChild(dot);
            }}
            dot.style.left = '{}px';
            dot.style.top = '{}px';
            dot.style.display = 'block';
        }})();",
        x - 6.0,
        y - 6.0
    ));

    timeout(Duration::from_millis(500), eval)
        .await
        .map_err(|_| anyhow::anyhow!("sync_cursor_overlay timed out"))??;
    Ok(())
}

fn generate_bezier_curve_with_config(start: &Point, end: &Point, config: &CursorMovementConfig) -> Vec<Point> {
    let mut points = Vec::new();

    let spread = config.curve_spread;
    let cp1 = Point::new(
        gaussian((start.x + end.x) / 2.0, spread, start.x.min(end.x), start.x.max(end.x)),
        gaussian((start.y + end.y) / 2.0, spread, start.y.min(end.y), start.y.max(end.y))
    );

    let cp2 = Point::new(
        gaussian((start.x + end.x) / 2.0, spread * 0.6, start.x.min(end.x), start.x.max(end.x)),
        gaussian((start.y + end.y) / 2.0, spread * 0.6, start.y.min(end.y), start.y.max(end.y))
    );

    let steps = config.steps.unwrap_or_else(|| random_in_range(10, 20) as u32);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let point = bezier_point(*start, cp1, cp2, *end, t);
        points.push(point);
    }

    points
}

fn bezier_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let x = (1.0 - t).powi(3) * p0.x +
            3.0 * (1.0 - t).powi(2) * t * p1.x +
            3.0 * (1.0 - t) * t.powi(2) * p2.x +
            t.powi(3) * p3.x;
    let y = (1.0 - t).powi(3) * p0.y +
            3.0 * (1.0 - t).powi(2) * t * p1.y +
            3.0 * (1.0 - t) * t.powi(2) * p2.y +
            t.powi(3) * p3.y;
    Point::new(x, y)
}

fn generate_arc_curve(start: &Point, end: &Point) -> Vec<Point> {
    let mid_x = (start.x + end.x) / 2.0;
    let distance = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
    let mid_y = (start.y + end.y) / 2.0 - distance * 0.3 * if random_in_range(0, 2) == 0 { 1.0 } else { -1.0 };

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

        let perp_x = -(end.y - start.y) / distance * zigzag_amount * if i % 2 == 0 { 1.0 } else { -1.0 };
        let perp_y = (end.x - start.x) / distance * zigzag_amount * if i % 2 == 0 { 1.0 } else { -1.0 };

        points.push(Point::new(base_x + perp_x, base_y + perp_y));
    }
    points
}

fn generate_overshoot_curve(start: &Point, end: &Point) -> Vec<Point> {
    let overshoot_scale = 1.2;
    let overshoot_x = start.x + (end.x - start.x) * overshoot_scale;
    let overshoot_y = start.y + (end.y - start.y) * overshoot_scale;

    vec![
        *start,
        Point::new(overshoot_x, overshoot_y),
        *end,
    ]
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
        anyhow::bail!("Coordinates ({}, {}) outside viewport ({}x{})", x, y, viewport.width, viewport.height);
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
    let button_idx = button.as_button_index();
    dispatch_mouse_action(page, x, y, button_idx, "mousedown").await?;
    dispatch_mouse_action(page, x, y, button_idx, "mouseup").await?;
    dispatch_mouse_action(page, x, y, button_idx, "click").await?;
    Ok(())
}

async fn dispatch_mouse_action(
    page: &Page,
    x: f64,
    y: f64,
    button_idx: u16,
    event_type: &str,
) -> Result<()> {
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

#[allow(dead_code)]
pub async fn click_selector(page: &Page, selector: &str) -> Result<()> {
    click_selector_human(page, selector, 60, 25, 6).await.map(|_| ())
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
    let selector_js = serde_json::to_string(selector)?;
    let native_scroll_js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;
            el.scrollIntoView({{behavior: 'auto', block: 'center', inline: 'nearest'}});
            return true;
        }})()"#
    );
    timeout(Duration::from_millis(1500), page.evaluate(native_scroll_js))
        .await
        .map_err(|_| anyhow::anyhow!("click native scroll timeout for selector={selector}"))??;

    let bbox = timeout(Duration::from_secs(2), resolve_selector_bbox(page, selector))
        .await
        .map_err(|_| anyhow::anyhow!("click resolve_selector_bbox timeout for selector={selector}"))??;

    let (x, y) = choose_click_point(&bbox, click_offset_px);
    let _ = timeout(Duration::from_millis(1200), cursor_move_to_immediate(page, x, y)).await;
    human_pause(reaction_delay_ms, reaction_delay_variance_pct).await;
    timeout(Duration::from_secs(2), dispatch_click(page, x, y, MouseButton::Left))
        .await
        .map_err(|_| anyhow::anyhow!("click dispatch_click timeout for selector={selector}"))??;

    let settle_ms = (reaction_delay_ms / 4).clamp(40, 200);
    let settle_variance = (reaction_delay_variance_pct / 3).max(10);
    human_pause(settle_ms, settle_variance).await;

    Ok(ClickOutcome {
        click: ClickStatus::Success,
        x,
        y,
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
        click: if matches!(first.click, ClickStatus::Success) && matches!(second.click, ClickStatus::Success) {
            ClickStatus::Success
        } else {
            ClickStatus::Failed
        },
        x: second.x,
        y: second.y,
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

async fn resolve_selector_bbox(page: &Page, selector: &str) -> Result<BoundingBox> {
    match wait_for_stable_element(page, selector, 2_000, 3, 2.0).await? {
        Some(bbox) => Ok(bbox),
        None => get_selector_bbox_once(page, selector).await,
    }
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
    human_pause(reaction_delay_ms, reaction_delay_variance_pct).await;
    dispatch_click(page, x, y, button).await?;

    let settle_ms = (reaction_delay_ms / 4).clamp(40, 200);
    let settle_variance = (reaction_delay_variance_pct / 3).max(10);
    human_pause(settle_ms, settle_variance).await;

    let verified = verify_click_target(page, selector, x, y).await.unwrap_or(false);
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

#[allow(dead_code)]
pub fn fitts_law_optimal_size(distance: f64, time: f64) -> f64 {
    let id = time / 100.0;
    2.0 * distance / (2.0_f64.powf(id))
}

// ============================================================================
// GhostCursor — High-level human-like mouse controller with per-session state
// ============================================================================

/// GhostCursor provides high-level mouse operations that mimic human behavior,
/// including stability checks, trajectory planning, retry logic, and fallbacks.
/// It maintains per-instance cursor position state, avoiding global mutable state.
///
/// This mirrors the Node.js `GhostCursor` class from `api/utils/ghostCursor.js`.
pub struct GhostCursor {
    page: Arc<Page>,
    previous_pos: (f64, f64),
}

impl GhostCursor {
    /// Create a new GhostCursor with random initial cursor position.
    pub fn new(page: Arc<Page>) -> Self {
        let prev = (
            random_in_range(50, 500) as f64,
            random_in_range(50, 500) as f64,
        );
        Self { page, previous_pos: prev }
    }

    /// Get the current page reference.
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// Get the current known cursor position (last set after move/click).
    pub fn previous_pos(&self) -> (f64, f64) {
        self.previous_pos
    }

    /// Set the cursor position state manually (rarely needed).
    pub fn set_previous_pos(&mut self, x: f64, y: f64) {
        self.previous_pos = (x, y);
    }

    /// Low-level movement along a cubic Bezier path with Gaussian tremor.
    /// Mirrors Node's `performMove(start, end, durationMs)`.
    async fn perform_move(&mut self, start: (f64, f64), end: (f64, f64), duration_ms: u64) -> Result<()> {
        let (start_x, start_y) = start;
        let (end_x, end_y) = end;

        // Guard against degenerate points
        if !start_x.is_finite() || !start_y.is_finite() || !end_x.is_finite() || !end_y.is_finite() {
            return Ok(());
        }

        let dx = end_x - start_x;
        let dy = end_y - start_y;
        let distance = (dx * dx + dy * dy).sqrt();

        // If distance is tiny, just move directly
        if distance < 0.5 {
            dispatch_mousemove(&self.page, end_x, end_y).await?;
            self.previous_pos = end;
            return Ok(());
        }

        // Arc amount for control point randomness (same as Node: 20..min(200, distance*0.5))
        let max_arc = (distance * 0.5).min(200.0);
        let arc_amount = if max_arc >= 20.0 {
            rand::thread_rng().gen_range(20.0..max_arc)
        } else {
            max_arc // fallback
        };

        // Control points (Gaussian noise)
        let p0 = Point::new(start_x, start_y);
        let p3 = Point::new(end_x, end_y);

        // p1: 30% along + gaussian(0, arc_amount)
        let p1 = Point::new(
            start_x + dx * 0.3 + gaussian(0.0, arc_amount, -arc_amount * 3.0, arc_amount * 3.0),
            start_y + dy * 0.3 + gaussian(0.0, arc_amount, -arc_amount * 3.0, arc_amount * 3.0),
        );

        // p2: 70% along + gaussian(0, arc_amount*0.6)
        let p2 = Point::new(
            start_x + dx * 0.7 + gaussian(0.0, arc_amount * 0.6, -arc_amount * 2.0, arc_amount * 2.0),
            start_y + dy * 0.7 + gaussian(0.0, arc_amount * 0.6, -arc_amount * 2.0, arc_amount * 2.0),
        );

        let start_time = std::time::Instant::now();
        let mut loop_flag = true;

        while loop_flag {
            let elapsed = start_time.elapsed().as_millis() as f64;
            let mut progress = elapsed / duration_ms as f64;
            if progress >= 1.0 {
                progress = 1.0;
                loop_flag = false;
            }

            // EaseOutCubic
            let eased_t = 1.0 - (1.0 - progress).powi(3);
            let pos = bezier_point(p0, p1, p2, p3, eased_t);

            // Tremor scale decreases as we approach target
            let tremor_scale = (1.0 - eased_t) * 1.5;
            let noisy_x = pos.x + (rand::thread_rng().gen::<f64>() - 0.5) * tremor_scale;
            let noisy_y = pos.y + (rand::thread_rng().gen::<f64>() - 0.5) * tremor_scale;

            dispatch_mousemove(&self.page, noisy_x, noisy_y).await?;

            if loop_flag {
                // Small random delay (0-8 ms) between moves
                sleep(Duration::from_millis(rand::thread_rng().gen_range(0..8))).await;
            }
        }

        self.previous_pos = end;
        Ok(())
    }

    /// High-level move to target with distance-based duration and overshoot.
    /// Mirrors `GhostCursor.move()` with Fitts's Law timing and probabilistic overshoot.
    pub async fn move_to(&mut self, target_x: f64, target_y: f64) -> Result<()> {
        let start = self.previous_pos;
        let end = (target_x, target_y);
        let dx = end.0 - start.0;
        let dy = end.1 - start.1;
        let distance = (dx * dx + dy * dy).sqrt();

        // Target duration: 250ms + 0.4*dist ±50ms
        let base_duration = 250.0 + distance * 0.4;
        let jitter = rand::thread_rng().gen_range(-50.0..50.0);
        let duration_ms = (base_duration + jitter).max(50.0) as u64; // min 50ms

        // Overshoot for long moves (20% chance if distance > 500px)
        let should_overshoot = distance > 500.0 && rand::thread_rng().gen_bool(0.2);
        if should_overshoot {
            let overshoot_scale = rand::thread_rng().gen_range(1.05..1.15);
            let error_lateral = gaussian(0.0, 20.0, -60.0, 60.0);
            let overshoot_x = start.0 + dx * overshoot_scale + error_lateral;
            let overshoot_y = start.1 + dy * overshoot_scale + error_lateral;
            // First leg to overshoot (use 80% of duration)
            let first_leg_duration = (duration_ms as f64 * 0.8) as u64;
            self.perform_move(start, (overshoot_x, overshoot_y), first_leg_duration).await?;
            // Brief pause 80-300ms
            sleep(Duration::from_millis(rand::thread_rng().gen_range(80..300))).await;
            // Second leg to target with shorter duration
            let second_leg_duration = rand::thread_rng().gen_range(150..300);
            self.perform_move((overshoot_x, overshoot_y), end, second_leg_duration).await?;
        } else {
            self.perform_move(start, end, duration_ms).await?;
        }

        Ok(())
    }

    /// Move to target with hesitation: for distance > 400px, split into two moves
    /// with a 100-300ms pause at the 40% waypoint.
    pub async fn move_with_hesitation(&mut self, target_x: f64, target_y: f64) -> Result<()> {
        let start = self.previous_pos;
        let dx = target_x - start.0;
        let dy = target_y - start.1;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance > 400.0 {
            // Midpoint at 40% of path
            let mid_x = start.0 + dx * 0.4;
            let mid_y = start.1 + dy * 0.4;
            // First leg to midpoint
            self.move_to(mid_x, mid_y).await?;
            // Hesitation pause
            sleep(Duration::from_millis(rand::thread_rng().gen_range(100..300))).await;
            // Second leg to target
            self.move_to(target_x, target_y).await?;
        } else {
            self.move_to(target_x, target_y).await?;
        }

        Ok(())
    }

    /// Hover at a point with micro-drift noise for a variable duration.
    /// Used to simulate human finger hovering before clicking.
    pub async fn hover_with_drift(&mut self, x: f64, y: f64, min_ms: u64, max_ms: u64) -> Result<()> {
        let duration = rand::thread_rng().gen_range(min_ms..=max_ms);
        let start_time = std::time::Instant::now();
        let drift_range = 1.0; // ±1px drift

        while start_time.elapsed().as_millis() < duration as u128 {
            let drift_x = (rand::thread_rng().gen::<f64>() - 0.5) * 2.0 * drift_range;
            let drift_y = (rand::thread_rng().gen::<f64>() - 0.5) * 2.0 * drift_range;
            dispatch_mousemove(&self.page, x + drift_x, y + drift_y).await?;

            // Occasional longer pause
            if rand::thread_rng().gen_bool(0.2) {
                sleep(Duration::from_millis(rand::thread_rng().gen_range(50..150))).await;
            }
            sleep(Duration::from_millis(rand::thread_rng().gen_range(50..100))).await;
        }

        self.previous_pos = (x, y);
        Ok(())
    }
    /// Click at absolute viewport coordinates using a default profile.
    /// This is a convenience wrapper around `click_with_profile` but without a selector.
    /// The caller should ensure the target is actionable.
    pub async fn click_at(&mut self, x: f64, y: f64) -> Result<bool> {
        let profile = ClickProfile::default();

        // Move to target with hesitation
        if let Err(_e) = self.move_with_hesitation(x, y).await {
            return Ok(false);
        }

        // Hover with drift
        if let Err(_e) = self.hover_with_drift(x, y, profile.hover_min_ms, profile.hover_max_ms).await {
            return Ok(false);
        }

        if profile.hesitation {
            sleep(Duration::from_millis(rand::thread_rng().gen_range(40..120))).await;
        }

        if profile.micro_move {
            let micro_x = x + rand::thread_rng().gen_range(-2.0..2.0);
            let micro_y = y + rand::thread_rng().gen_range(-2.0..2.0);
            let _ = dispatch_mousemove(self.page(), micro_x, micro_y).await;
            sleep(Duration::from_millis(rand::thread_rng().gen_range(20..50))).await;
        }

        let hold_ms = gaussian(60.0, 20.0, 20.0, 150.0) as u64;

        // Mouse down
        let down_js = format!(
            r#"(function() {{
                const el = document.elementFromPoint({}, {});
                if (!el) return;
                const evt = new MouseEvent('mousedown', {{
                    bubbles: true,
                    cancelable: true,
                    clientX: {},
                    clientY: {},
                    button: 0
                }});
                el.dispatchEvent(evt);
            }})()"#,
            x, y, x, y
        );
        let _ = self.page.evaluate(down_js).await;

        sleep(Duration::from_millis(hold_ms)).await;

        // Mouse up
        let up_js = format!(
            r#"(function() {{
                const el = document.elementFromPoint({}, {});
                if (!el) return;
                const evt = new MouseEvent('mouseup', {{
                    bubbles: true,
                    cancelable: true,
                    clientX: {},
                    clientY: {},
                    button: 0
                }});
                el.dispatchEvent(evt);
            }})()"#,
            x, y, x, y
        );
        let _ = self.page.evaluate(up_js).await;

        // Click event
        let click_js = format!(
            r#"(function() {{
                const el = document.elementFromPoint({}, {});
                if (!el) return;
                const evt = new MouseEvent('click', {{
                    bubbles: true,
                    cancelable: true,
                    clientX: {},
                    clientY: {},
                    button: 0
                }});
                el.dispatchEvent(evt);
            }})()"#,
            x, y, x, y
        );
        let _ = self.page.evaluate(click_js).await;

         Ok(true)
     }
 }
/// Click profile parameters (mirrors Node's profiles).
#[derive(Debug, Clone, Copy)]
pub struct ClickProfile {
    pub hover_min_ms: u64,
    pub hover_max_ms: u64,
    pub hold_ms: u64,
    pub hesitation: bool,
    pub micro_move: bool,
}

impl Default for ClickProfile {
    fn default() -> Self {
        Self {
            hover_min_ms: 200,
            hover_max_ms: 800,
            hold_ms: 80,
            hesitation: false,
            micro_move: false,
        }
    }
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
    fn test_overlay_enabled_default() {
        assert!(is_overlay_enabled());
    }

    #[test]
    fn test_overlay_can_be_disabled() {
        set_overlay_enabled(false);
        assert!(!is_overlay_enabled());
        set_overlay_enabled(true);
    }

    #[test]
    fn test_cursor_position_start_at_zero() {
        let (x, y) = cursor_position();
        assert_eq!(x, 0.0);
        assert_eq!(y, 0.0);
    }

    #[test]
    fn test_cursor_position_snapshot_starts_empty() {
        assert!(cursor_position_snapshot().is_none());
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
}

