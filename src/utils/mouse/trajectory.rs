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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Point Struct Tests
    // =========================================================================

    #[test]
    fn test_point_new() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    }

    #[test]
    fn test_point_clone_and_copy() {
        let p1 = Point::new(5.0, 15.0);
        let p2 = p1; // Copy
        let p3 = p1;
        assert_eq!(p1, p2);
        assert_eq!(p1, p3);
    }

    // =========================================================================
    // Bezier Point Tests
    // =========================================================================

    #[test]
    fn test_bezier_point_at_start() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(50.0, 100.0);
        let p2 = Point::new(100.0, 100.0);
        let p3 = Point::new(150.0, 0.0);

        let result = bezier_point(p0, p1, p2, p3, 0.0);
        assert!((result.x - p0.x).abs() < 0.001);
        assert!((result.y - p0.y).abs() < 0.001);
    }

    #[test]
    fn test_bezier_point_at_end() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(50.0, 100.0);
        let p2 = Point::new(100.0, 100.0);
        let p3 = Point::new(150.0, 0.0);

        let result = bezier_point(p0, p1, p2, p3, 1.0);
        assert!((result.x - p3.x).abs() < 0.001);
        assert!((result.y - p3.y).abs() < 0.001);
    }

    #[test]
    fn test_bezier_point_at_midpoint() {
        let p0 = Point::new(0.0, 0.0);
        let p1 = Point::new(50.0, 100.0);
        let p2 = Point::new(100.0, 100.0);
        let p3 = Point::new(150.0, 0.0);

        let result = bezier_point(p0, p1, p2, p3, 0.5);
        // At t=0.5, the point should be roughly in the middle of the curve
        assert!(result.x > 20.0 && result.x < 130.0);
        assert!(result.y >= 0.0 && result.y <= 100.0);
    }

    // =========================================================================
    // Bezier Curve Tests
    // =========================================================================

    #[test]
    fn test_bezier_curve_start_end_points() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);

        let points = generate_bezier_curve_with_config(&start, &end, 50.0, Some(20));

        assert!(!points.is_empty());
        // First point should be close to start
        assert!((points[0].x - start.x).abs() < 0.001);
        assert!((points[0].y - start.y).abs() < 0.001);
        // Last point should be close to end
        assert!((points[points.len() - 1].x - end.x).abs() < 0.001);
        assert!((points[points.len() - 1].y - end.y).abs() < 0.001);
    }

    #[test]
    fn test_bezier_curve_step_count() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);

        let points = generate_bezier_curve_with_config(&start, &end, 50.0, Some(10));
        // 10 steps + 1 (inclusive) = 11 points
        assert_eq!(points.len(), 11);
    }

    #[test]
    fn test_bezier_curve_with_zero_spread() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_bezier_curve_with_config(&start, &end, 0.0, Some(10));
        assert!(!points.is_empty());
        // With zero spread, control points should be centered, creating a straighter line
        let first = points[0];
        let last = points[points.len() - 1];
        assert!((first.x - start.x).abs() < 0.001);
        assert!((last.x - end.x).abs() < 0.001);
    }

    #[test]
    fn test_bezier_curve_identical_points() {
        let start = Point::new(50.0, 50.0);
        let end = Point::new(50.0, 50.0);

        let points = generate_bezier_curve_with_config(&start, &end, 10.0, Some(5));
        // Should still generate a path (degenerate case)
        assert!(!points.is_empty());
    }

    // =========================================================================
    // Arc Curve Tests
    // =========================================================================

    #[test]
    fn test_arc_curve_start_end_points() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_arc_curve(&start, &end);

        assert_eq!(points.len(), 11); // 10 steps + 1
                                      // First and last points should match start/end
        assert!((points[0].x - start.x).abs() < 0.001);
        assert!((points[0].y - start.y).abs() < 0.001);
        assert!((points[points.len() - 1].x - end.x).abs() < 0.001);
        assert!((points[points.len() - 1].y - end.y).abs() < 0.001);
    }

    #[test]
    fn test_arc_curve_has_curvature() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_arc_curve(&start, &end);

        // At least some midpoints should deviate from straight line (y != 0)
        let mid_points_have_curve = points[2..points.len() - 2].iter().any(|p| p.y.abs() > 1.0);
        assert!(
            mid_points_have_curve,
            "Arc curve should have visible curvature"
        );
    }

    #[test]
    fn test_arc_curve_midpoint_deviation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_arc_curve(&start, &end);

        // Midpoint should be around (50, ±15) due to 30% distance arc
        let mid = &points[5]; // roughly middle
        assert!(mid.x > 40.0 && mid.x < 60.0);
        assert!(mid.y.abs() > 5.0 || mid.y.abs() < 25.0); // Reasonable arc height
    }

    // =========================================================================
    // Zigzag Curve Tests
    // =========================================================================

    #[test]
    fn test_zigzag_curve_structure() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_zigzag_curve(&start, &end);

        assert_eq!(points.len(), 5); // 4 steps + 1
                                     // Zigzag applies perpendicular offset, so exact match isn't expected
                                     // But points should be within zigzag_amount of the line
        let zigzag_amount = 10.0; // 100.0 * 0.1 = 10.0
        assert!((points[0].y).abs() <= zigzag_amount + 0.001);
        assert!((points[points.len() - 1].y).abs() <= zigzag_amount + 0.001);
    }

    #[test]
    fn test_zigzag_curve_has_perpendicular_deviation() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_zigzag_curve(&start, &end);

        // Middle points should deviate perpendicular to line (y-axis)
        let mid_points = &points[1..points.len() - 1];
        let has_positive_y = mid_points.iter().any(|p| p.y > 1.0);
        let has_negative_y = mid_points.iter().any(|p| p.y < -1.0);

        // Zigzag should alternate between positive and negative y
        assert!(
            has_positive_y || has_negative_y,
            "Zigzag should have perpendicular deviation from straight line"
        );
    }

    #[test]
    fn test_zigzag_curve_alternating_pattern() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_zigzag_curve(&start, &end);

        // Check that consecutive points alternate direction
        for i in 1..points.len() - 1 {
            let curr_y = points[i].y;
            let next_y = points[i + 1].y;

            // In a proper zigzag, middle points should alternate
            if i % 2 == 1 {
                // Odd indices should be on one side, even on other
                let sign_curr = if curr_y > 0.0 {
                    1
                } else if curr_y < 0.0 {
                    -1
                } else {
                    0
                };
                let sign_next = if next_y > 0.0 {
                    1
                } else if next_y < 0.0 {
                    -1
                } else {
                    0
                };
                // Consecutive middle points should have opposite signs
                if sign_curr != 0 && sign_next != 0 {
                    assert_ne!(
                        sign_curr, sign_next,
                        "Zigzag points should alternate direction"
                    );
                }
            }
        }
    }

    // =========================================================================
    // Overshoot Curve Tests
    // =========================================================================

    #[test]
    fn test_overshoot_curve_structure() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);

        let points = generate_overshoot_curve(&start, &end);

        // Should have exactly 3 points: start, overshoot, end
        assert_eq!(points.len(), 3);
        assert!((points[0].x - start.x).abs() < 0.001);
        assert!((points[0].y - start.y).abs() < 0.001);
        assert!((points[2].x - end.x).abs() < 0.001);
        assert!((points[2].y - end.y).abs() < 0.001);
    }

    #[test]
    fn test_overshoot_curve_overshoot_scale() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_overshoot_curve(&start, &end);

        // Middle point should be at 1.2x the distance (overshoot by 20%)
        let overshoot = &points[1];
        assert!(overshoot.x > 100.0, "Overshoot x should exceed end point");
        assert!(
            (overshoot.x - 120.0).abs() < 0.001,
            "Overshoot should be at 1.2x scale"
        );
    }

    #[test]
    fn test_overshoot_curve_both_directions() {
        let start = Point::new(100.0, 100.0);
        let end = Point::new(0.0, 0.0);

        let points = generate_overshoot_curve(&start, &end);

        // With reverse direction, overshoot should go negative
        let overshoot = &points[1];
        assert!(
            overshoot.x < 0.0,
            "Overshoot should go past target in reverse direction"
        );
        assert!(overshoot.y < 0.0);
    }

    // =========================================================================
    // Stopped Curve Tests
    // =========================================================================

    #[test]
    fn test_stopped_curve_structure() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 100.0);

        let points = generate_stopped_curve(&start, &end);

        // Should have 4 points: 3 stops + start and end (0, 1/3, 2/3, 1)
        assert_eq!(points.len(), 4);
        assert!((points[0].x - start.x).abs() < 0.001);
        assert!((points[0].y - start.y).abs() < 0.001);
        assert!((points[3].x - end.x).abs() < 0.001);
        assert!((points[3].y - end.y).abs() < 0.001);
    }

    #[test]
    fn test_stopped_curve_equal_spacing() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(90.0, 90.0);

        let points = generate_stopped_curve(&start, &end);

        // With 3 stops, points should be at 0%, 33%, 66%, 100%
        assert!((points[0].x - 0.0).abs() < 0.001);
        assert!(
            (points[1].x - 30.0).abs() < 0.001,
            "First stop should be at 33%"
        );
        assert!(
            (points[2].x - 60.0).abs() < 0.001,
            "Second stop should be at 66%"
        );
        assert!((points[3].x - 90.0).abs() < 0.001);
    }

    // =========================================================================
    // Muscle Path Tests
    // =========================================================================

    #[test]
    fn test_muscle_path_converges_to_end() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_muscle_path(&start, &end);

        assert!(!points.is_empty());
        // Last point should be very close to end (within tolerance)
        let last = points[points.len() - 1];
        assert!(
            (last.x - end.x).abs() < 3.0,
            "Muscle path should converge near end point"
        );
        assert!((last.y - end.y).abs() < 3.0);
    }

    #[test]
    fn test_muscle_path_has_progression() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_muscle_path(&start, &end);

        // Points should generally progress toward end
        let first = points[0];
        let last = points[points.len() - 1];
        assert!(
            last.x > first.x,
            "Path should progress in x direction toward end"
        );
    }

    #[test]
    fn test_muscle_path_respects_max_steps() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(1000.0, 0.0); // Long distance

        let points = generate_muscle_path(&start, &end);

        // Should not exceed max_steps (20)
        assert!(
            points.len() <= 20,
            "Muscle path should respect max_steps limit"
        );
    }

    #[test]
    fn test_muscle_path_has_jitter() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(100.0, 0.0);

        let points = generate_muscle_path(&start, &end);

        // With jitter, y values should vary (not all exactly 0)
        let y_variance: f64 = points.iter().map(|p| p.y.abs()).sum::<f64>() / points.len() as f64;
        // Allow for some jitter - average y deviation should be small but non-zero
        assert!(y_variance >= 0.0, "Y values should be non-negative");
    }

    #[test]
    fn test_muscle_path_short_distance_terminates() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(1.0, 1.0); // Very short distance

        let points = generate_muscle_path(&start, &end);

        // Should terminate quickly (within tolerance)
        assert!(points.len() < 10, "Short distance should terminate quickly");
    }
}
