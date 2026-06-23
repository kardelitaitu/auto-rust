//! Adaptive mouse behavior — human-like cursor movement with collision avoidance.
//!
//! Contains element detection, movement speed adaptation (Fitts' Law), collision-aware
//! path generation, attention drift simulation, and high-level click/hover selectors.

use super::cdp;
use super::overlay::{
    dispatch_mousemove, generate_bezier_curve_with_config, CursorMovementConfig, PathStyle,
};
use super::trajectory::Point;
use super::types::MouseButton;
use crate::utils::math::{gaussian, random_in_range};
use crate::utils::page_size::get_viewport;
use crate::utils::timing::human_pause;
use anyhow::Result;
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};

/// Detects element type from selector for hover duration customization.
/// This is a heuristic-based detection.
pub(crate) fn detect_element_type(selector: &str) -> String {
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
pub(crate) async fn wait_for_element_stability(
    page: &Page,
    selector: &str,
    timeout_ms: u64,
) -> Result<bool> {
    let start_time = std::time::Instant::now();
    let check_interval_ms = 100;
    let required_stable_checks = 3;
    let mut stable_count = 0;

    while start_time.elapsed().as_millis() < u128::from(timeout_ms) {
        // Check if element exists and is visible
        let exists_js = format!(
            r"(() => {{
                const el = document.querySelector({});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                return rect.width > 0 && rect.height > 0 && rect.top >= 0;
            }})()",
            serde_json::to_string(selector)?
        );

        let exists = page
            .evaluate(exists_js)
            .await?
            .value()
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if !exists {
            stable_count = 0;
            tokio::time::sleep(tokio::time::Duration::from_millis(check_interval_ms)).await;
            continue;
        }

        // Get current position
        let pos_js = format!(
            r"(() => {{
                const el = document.querySelector({});
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.left, y: rect.top, width: rect.width, height: rect.height }};
            }})()",
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
            let curr_x = curr
                .get("x")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let curr_y = curr
                .get("y")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let next_x = next
                .get("x")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let next_y = next
                .get("y")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);

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

pub async fn hover_before_click(page: &Page, x: f64, y: f64, element_type: &str) -> Result<()> {
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
    let _ = cdp::dispatch_pointer_event(page, "pointerenter", x, y, MouseButton::Left).await;

    // Variable hover duration with variance
    let variance = random_in_range(20, 40) as u32;
    human_pause(base_hover_ms, variance).await;

    // Subtle position shift during hover (humans aren't perfectly still)
    let hover_shift_x = gaussian(0.0, 1.5, -4.0, 4.0);
    let hover_shift_y = gaussian(0.0, 1.5, -4.0, 4.0);
    dispatch_mousemove(page, x + hover_shift_x, y + hover_shift_y).await?;

    // Fire pointerleave before click (to properly balance pointerenter)
    let _ = cdp::dispatch_pointer_event(page, "pointerleave", x, y, MouseButton::Left).await;

    Ok(())
}

/// Moves cursor to target coordinates with adaptive speed and collision avoidance
pub(crate) async fn move_cursor_collision_avoidant(
    page: &Page,
    target_x: f64,
    target_y: f64,
) -> Result<()> {
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

/// User experience level for adaptive speed calibration.
/// `Intermediate` is the default; `Novice` and `Expert` are reserved
/// for future profile-integration support (matched but never constructed).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum ExperienceLevel {
    Novice,
    Intermediate,
    Expert,
}

/// Target element priority for adaptive speed calibration.
/// `Normal` is the default; `Critical` and `Optional` are reserved
/// for future selector-integration support (matched but never constructed).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum ElementPriority {
    Critical,
    Normal,
    Optional,
}

/// Moves cursor along points with adaptive speed (slower near target)
#[allow(clippy::cast_precision_loss)]
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

/// Check if a single point collides with a UI element.
/// Returns true if the point is over an interactive element.
async fn check_point_collision(page: &Page, point: &Point) -> Result<bool> {
    let js = format!(
        r"(() => {{
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
                    return true;
                }}
            }}
            return false;
        }})()",
        point.x, point.y
    );

    if let Ok(result) = page.evaluate(js).await {
        if let Some(collision) = result.value().and_then(serde_json::Value::as_bool) {
            return Ok(collision);
        }
    }

    Ok(false)
}

/// Detects UI elements that would cause unwanted hovers along cursor path
async fn detect_ui_collisions_along_path(page: &Page, points: &[Point]) -> Result<Vec<Point>> {
    let mut collision_points = Vec::new();
    let sample_rate = 5; // Check every 5th point to optimize

    for (i, point) in points.iter().enumerate() {
        if i % sample_rate != 0 {
            continue; // Skip most points for performance
        }

        if check_point_collision(page, point).await? {
            collision_points.push(*point);
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
                f64::midpoint(prev.x, point.x) + (point.y - prev.y) * 0.3, // Perpendicular offset
                f64::midpoint(prev.y, point.y) - (point.x - prev.x) * 0.3,
            );
            safe_points.push(detour_point);
        }

        safe_points.push(point);
    }

    safe_points
}

/// Simulates human attention drift - cursor briefly moves away then corrects back
#[allow(clippy::cast_precision_loss)]
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
pub async fn is_element_clickable(page: &Page, selector: &str) -> Result<bool> {
    let js = format!(
        r"(() => {{
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
        }})()",
        serde_json::to_string(selector)?
    );

    page.evaluate(js)
        .await?
        .value()
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| anyhow::anyhow!("Failed to evaluate clickability"))
}
