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
use crate::utils::math::{random_in_range, gaussian};
use crate::utils::scroll;
use crate::utils::timing::human_pause;
use crate::utils::page_size::get_viewport;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::time::{timeout, Duration};

static MOUSE_OVERLAY_ENABLED: AtomicBool = AtomicBool::new(true);
static CURSOR_X: AtomicU64 = AtomicU64::new(0);
static CURSOR_Y: AtomicU64 = AtomicU64::new(0);

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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
}

fn cursor_position() -> (f64, f64) {
    (
        f64::from_bits(CURSOR_X.load(Ordering::Relaxed)),
        f64::from_bits(CURSOR_Y.load(Ordering::Relaxed)),
    )
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
    let start_x = viewport.width / 2.0;
    let start_y = viewport.height / 2.0;

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

    Ok(())
}

pub async fn cursor_move_to_immediate(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    dispatch_mousemove(page, target_x, target_y).await
}

async fn dispatch_mousemove(page: &Page, x: f64, y: f64) -> Result<()> {
    set_cursor_position(x, y);
    dispatch_mousemove_dom(page, x, y).await?;

    Ok(())
}

async fn dispatch_mousemove_dom(_page: &Page, x: f64, y: f64) -> Result<()> {
    set_cursor_position(x, y);
    Ok(())
}

pub async fn sync_cursor_overlay(page: &Page) -> Result<()> {
    if !is_overlay_enabled() {
        return Ok(());
    }

    let (x, y) = cursor_position();
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
        let point = bezier_point(start.clone(), cp1.clone(), cp2.clone(), end.clone(), t);
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
        points.push(bezier_point(start.clone(), control.clone(), control.clone(), end.clone(), t));
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
        start.clone(),
        Point::new(overshoot_x, overshoot_y),
        end.clone(),
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

    let mut current = start.clone();

    for _ in 0..max_steps {
        let dx = end.x - current.x;
        let dy = end.y - current.y;
        let dist = (dx.powi(2) + dy.powi(2)).sqrt();

        if dist < tolerance {
            points.push(end.clone());
            break;
        }

        let kp = 0.8;
        let step_size = dist.min(50.0) * kp;
        let next_x = current.x + (dx / dist) * step_size;
        let next_y = current.y + (dy / dist) * step_size;

        let jitter = gaussian(0.0, 0.8, -2.0, 2.0);
        current = Point::new(next_x + jitter, next_y + jitter);
        points.push(current.clone());
    }

    points
}

pub async fn click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    left_click_at(page, x, y).await
}

pub async fn click_at_without_move(page: &Page, x: f64, y: f64) -> Result<()> {
    dispatch_click(page, x, y, MouseButton::Left).await
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

    let eval = page.evaluate(format!(
        "(function() {{
            const el = document.elementFromPoint({}, {});
            if (!el) return;
            
            const downEvent = new MouseEvent('mousedown', {{
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {},
                button: {}
            }});
            el.dispatchEvent(downEvent);
            
            const upEvent = new MouseEvent('mouseup', {{
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {},
                button: {}
            }});
            el.dispatchEvent(upEvent);
            
            const clickEvent = new MouseEvent('click', {{
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {},
                button: {}
            }});
            el.dispatchEvent(clickEvent);
        }})();",
        x, y, x, y, button_idx, x, y, button_idx, x, y, button_idx
    ));

    timeout(Duration::from_secs(2), eval)
    .await
    .map_err(|_| anyhow::anyhow!("dispatch_click timed out"))??;
    Ok(())
}

#[allow(dead_code)]
pub async fn click_selector(page: &Page, selector: &str) -> Result<()> {
    scroll::scroll_into_view(page, selector).await?;
    let (x, y) = crate::utils::page_size::get_element_center(page, selector).await?;
    click_at(page, x, y).await
}

#[allow(dead_code)]
pub fn fitts_law_optimal_size(distance: f64, time: f64) -> f64 {
    let id = time / 100.0;
    2.0 * distance / (2.0_f64.powf(id))
}
