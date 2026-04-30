//! Cursor trajectory and path generation for human-like mouse movement.
//!
//! Provides various curve generation algorithms for realistic cursor paths:
//! - Bezier curves with configurable spread
//! - Arc curves for curved movements
//! - Zigzag patterns for erratic movement
//! - Overshoot curves for correction patterns
//! - Stopped curves for pause patterns
//! - Muscle paths for simulation-based movement

use crate::utils::math::{gaussian, random_in_range};

/// A point in 2D space with floating-point coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point with the given coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Generate a Bezier curve with custom configuration.
/// Uses cubic Bezier with Gaussian-distributed control points.
pub fn generate_bezier_curve_with_config(
    start: &Point,
    end: &Point,
    spread: f64,
    steps: Option<u32>,
) -> Vec<Point> {
    let mut points = Vec::new();

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

    let step_count = steps.unwrap_or_else(|| random_in_range(10, 20) as u32);
    for i in 0..=step_count {
        let t = i as f64 / step_count as f64;
        let point = bezier_point(*start, cp1, cp2, *end, t);
        points.push(point);
    }

    points
}

/// Calculate a point on a cubic Bezier curve at parameter t.
pub fn bezier_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
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

/// Generate an arc curve between two points.
/// Creates a curved path with upward or downward arc.
pub fn generate_arc_curve(start: &Point, end: &Point) -> Vec<Point> {
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

/// Generate a zigzag curve for erratic movement.
/// Creates a perpendicular zigzag pattern along the path.
pub fn generate_zigzag_curve(start: &Point, end: &Point) -> Vec<Point> {
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

/// Generate an overshoot curve that goes past the target.
/// Simulates human correction behavior.
pub fn generate_overshoot_curve(start: &Point, end: &Point) -> Vec<Point> {
    let overshoot_scale = 1.2;
    let overshoot_x = start.x + (end.x - start.x) * overshoot_scale;
    let overshoot_y = start.y + (end.y - start.y) * overshoot_scale;

    vec![*start, Point::new(overshoot_x, overshoot_y), *end]
}

/// Generate a stopped curve with pauses along the path.
/// Creates intermediate stop points.
pub fn generate_stopped_curve(start: &Point, end: &Point) -> Vec<Point> {
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

/// Generate a muscle-based path using simulation.
/// Simulates muscle movement with jitter and step adjustments.
pub fn generate_muscle_path(start: &Point, end: &Point) -> Vec<Point> {
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
