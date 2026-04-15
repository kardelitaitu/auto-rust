use chromiumoxide::Page;
use anyhow::Result;
use crate::utils::math::{random_in_range, gaussian};
use crate::utils::timing::human_pause;

// Bezier curve control point
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

pub async fn human_mouse_move_to(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    // Get current mouse position (assume starting from center of viewport for simplicity)
    let _viewport_size = page.evaluate("({width: window.innerWidth, height: window.innerHeight})").await?;
    let start_x = 800.0; // Assume 800x600 viewport for simplicity
    let start_y = 300.0;

    // Generate Bezier curve points
    let points = generate_bezier_curve(Point::new(start_x, start_y), Point::new(target_x, target_y));

    // Move mouse along the curve with timing
    for point in points {
        // Move to point
        page.evaluate(format!("
            if (window.mouse) {{
                window.mouse.move({}, {});
            }} else {{
                // Fallback for browsers without mouse API
                document.dispatchEvent(new MouseEvent('mousemove', {{
                    clientX: {},
                    clientY: {},
                    bubbles: true
                }}));
            }}
        ", point.x, point.y, point.x, point.y)).await?;

        // Small pause between movements
        human_pause(10, 100).await;
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    // Move to position first
    human_mouse_move_to(page, x, y).await?;

    // Pause before clicking (human reaction time)
    human_pause(50, 50).await;

    // Perform click
    page.evaluate(format!("
        const element = document.elementFromPoint({}, {});
        if (element) {{
            const clickEvent = new MouseEvent('click', {{
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {},
                button: 0
            }});
            element.dispatchEvent(clickEvent);
        }}
    ", x, y, x, y)).await?;

    Ok(())
}

fn generate_bezier_curve(start: Point, end: Point) -> Vec<Point> {
    let mut points = Vec::new();

    // Generate control points with some randomness
    let cp1 = Point::new(
        gaussian((start.x + end.x) / 2.0, 50.0, start.x.min(end.x), start.x.max(end.x)),
        gaussian((start.y + end.y) / 2.0, 50.0, start.y.min(end.y), start.y.max(end.y))
    );

    let cp2 = Point::new(
        gaussian((start.x + end.x) / 2.0, 30.0, start.x.min(end.x), start.x.max(end.x)),
        gaussian((start.y + end.y) / 2.0, 30.0, start.y.min(end.y), start.y.max(end.y))
    );

    // Generate points along the curve (simplified quadratic Bezier)
    let steps = random_in_range(10, 20);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let point = quadratic_bezier_point(start.clone(), cp1.clone(), cp2.clone(), end.clone(), t);
        points.push(point);
    }

    points
}

fn quadratic_bezier_point(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    // Cubic Bezier curve
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

// Fitts's Law calculation for optimal click target size
#[allow(dead_code)]
pub fn fitts_law_optimal_size(distance: f64, time: f64) -> f64 {
    // Fitts's Law: ID = log2(2D/W)
    // Solving for W: W = 2D / 2^ID
    // Where ID = time / some constant (typically ~100ms/bit)
    let id = time / 100.0; // Rough approximation
    2.0 * distance / (2.0_f64.powf(id))
}