//! Mouse simulation and human-computer interaction utilities.
//!
//! Provides functions for simulating realistic mouse movements and clicks:
//! - Human-like mouse movement using Bezier curves
//! - Click simulation with proper timing
//! - Fitts's Law calculations for optimal target sizing
//! - Configurable velocity and trajectory randomization
//! - Utilities for human-computer interaction studies

use chromiumoxide::Page;
use anyhow::Result;
use crate::utils::math::{random_in_range, gaussian};
use crate::utils::timing::human_pause;
use crate::utils::page_size::get_viewport;

/// Configuration for cursor movement behavior.
/// Allows customization of movement speed, trajectory, and timing.
#[derive(Debug, Clone)]
pub struct CursorMovementConfig {
    /// Base speed multiplier (higher = faster). Default: 1.0
    pub speed_multiplier: f64,
    /// Minimum delay between movement steps in milliseconds. Default: 10
    pub min_step_delay_ms: u64,
    /// Maximum additional delay variance in milliseconds. Default: 90
    pub max_step_delay_variance_ms: u64,
    /// Bezier curve control point spread (higher = more curved). Default: 50.0
    pub curve_spread: f64,
    /// Number of steps in the bezier curve (higher = smoother but slower). Default: None (random 10-20)
    pub steps: Option<u32>,
    /// Whether to add random micro-pauses during movement. Default: true
    pub add_micro_pauses: bool,
}

impl Default for CursorMovementConfig {
    fn default() -> Self {
        Self {
            speed_multiplier: 1.0,
            min_step_delay_ms: 10,
            max_step_delay_variance_ms: 90,
            curve_spread: 50.0,
            steps: None,
            add_micro_pauses: true,
        }
    }
}

/// Represents a 2D point for mouse movement calculations.
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

/// Moves the mouse cursor to a target position using a human-like Bezier curve trajectory.
/// Uses default configuration with sensible human-like parameters.
///
/// # Arguments
/// * `page` - The browser page to perform the mouse movement on
/// * `target_x` - Target X coordinate
/// * `target_y` - Target Y coordinate
///
/// # Returns
/// Ok(()) if the movement succeeds
pub async fn cursor_move_to(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    cursor_move_to_with_config(page, target_x, target_y, &CursorMovementConfig::default()).await
}

/// Moves the mouse cursor with configurable behavior.
///
/// # Arguments
/// * `page` - The browser page to perform the mouse movement on
/// * `target_x` - Target X coordinate
/// * `target_y` - Target Y coordinate
/// * `config` - Configuration for movement behavior
///
/// # Returns
/// Ok(()) if the movement succeeds
pub async fn cursor_move_to_with_config(
    page: &Page,
    target_x: f64,
    target_y: f64,
    config: &CursorMovementConfig,
) -> Result<()> {
    let viewport = get_viewport(page).await?;
    
    let start_x = viewport.width / 2.0;
    let start_y = viewport.height / 2.0;

    let start_point = Point::new(start_x, start_y);
    let end_point = Point::new(target_x, target_y);
    let points = generate_bezier_curve_with_config(&start_point, &end_point, config);

    for point in points {
        page.evaluate(format!(
            "if (window.mouse) {{window.mouse.move({}, {});}} else {{document.dispatchEvent(new MouseEvent('mousemove', {{clientX: {}, clientY: {}, bubbles: true}}));}}",
            point.x, point.y, point.x, point.y
        )).await?;

        let delay = (config.min_step_delay_ms as f64 / config.speed_multiplier) as u64;
        let variance = (config.max_step_delay_variance_ms as f64 / config.speed_multiplier) as u32;
        human_pause(delay, variance).await;

        if config.add_micro_pauses && random_in_range(0, 100) < 10 {
            human_pause(random_in_range(50, 200), 20).await;
        }
    }

    Ok(())
}

/// Generates a series of points along a Bezier curve between two points.
#[allow(dead_code)]
#[allow(dead_code)]
fn generate_bezier_curve(start: &Point, end: &Point) -> Vec<Point> {
    generate_bezier_curve_with_config(start, end, &CursorMovementConfig::default())
}

/// Generates Bezier curve with custom configuration.
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

/// Clicks at the specified coordinates by first moving the mouse and then performing a click.
///
/// # Arguments
/// * `page` - The browser page to perform the click on
/// * `x` - X coordinate of the click location
/// * `y` - Y coordinate of the click location
///
/// # Returns
/// Ok(()) if the click succeeds
#[allow(dead_code)]
pub async fn click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    left_click_at(page, x, y).await
}

/// Left click at coordinates.
pub async fn left_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click_event(page, x, y, 0).await
}

/// Middle click at coordinates.
#[allow(dead_code)]
#[allow(dead_code)]
pub async fn middle_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click_event(page, x, y, 1).await
}

/// Right click at coordinates.
#[allow(dead_code)]
#[allow(dead_code)]
pub async fn right_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click_event(page, x, y, 2).await
}

async fn dispatch_click_event(page: &Page, x: f64, y: f64, button: u16) -> Result<()> {
    page.evaluate(format!(
        "const el = document.elementFromPoint({}, {}); if (el) {{el.dispatchEvent(new MouseEvent('click', {{bubbles: true, cancelable: true, clientX: {}, clientY: {}, button: {}}}));}}",
        x, y, x, y, button
    )).await?;
    Ok(())
}

/// Clicks on a CSS selector element.
#[allow(dead_code)]
pub async fn click_selector(page: &Page, selector: &str) -> Result<()> {
    let (x, y) = crate::utils::page_size::get_element_center(page, selector).await?;
    click_at(page, x, y).await
}

/// Calculates the optimal target size for a clicking task using Fitts's Law.
#[allow(dead_code)]
pub fn fitts_law_optimal_size(distance: f64, time: f64) -> f64 {
    let id = time / 100.0;
    2.0 * distance / (2.0_f64.powf(id))
}
