//! Page size and viewport utilities for cursor positioning.
//!
//! Provides functions for querying browser viewport dimensions and
//! calculating valid cursor positions for mouse interactions.

use anyhow::Result;
use chromiumoxide::Page;
use serde::Deserialize;
use serde_json;

/// Represents the browser viewport dimensions.
/// Used for cursor position calculations and bounds checking.
#[derive(Debug, Clone, Deserialize)]
pub struct Viewport {
    /// Viewport width in pixels
    pub width: f64,
    /// Viewport height in pixels
    pub height: f64,
}

/// Fetches the current viewport size from the browser.
///
/// # Arguments
/// * `page` - The browser page to query
///
/// # Returns
/// Ok(Viewport) with current dimensions, Err if evaluation fails
pub async fn get_viewport(page: &Page) -> Result<Viewport> {
    let result = page
        .evaluate("({width: window.innerWidth, height: window.innerHeight})")
        .await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to get viewport value"))?;
    let viewport: Viewport = serde_json::from_value(value.clone())?;
    Ok(viewport)
}

/// Fetches the full document scrollable dimensions.
///
/// # Arguments
/// * `page` - The browser page to query
///
/// # Returns
/// Ok(DocumentSize) with scrollWidth and scrollHeight, Err if evaluation fails
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DocumentSize {
    pub scroll_width: f64,
    pub scroll_height: f64,
}

#[allow(dead_code)]
pub async fn get_document_size(page: &Page) -> Result<DocumentSize> {
    let result = page.evaluate(
        "({scroll_width: document.documentElement.scrollWidth, scroll_height: document.documentElement.scrollHeight})"
    ).await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to get document size value"))?;
    let doc: DocumentSize = serde_json::from_value(value.clone())?;
    Ok(doc)
}

/// Gets the center position of the viewport.
///
/// # Arguments
/// * `viewport` - The viewport dimensions
///
/// # Returns
/// (x, y) tuple representing the center coordinates
#[allow(dead_code)]
pub fn center_position(viewport: &Viewport) -> (f64, f64) {
    (viewport.width / 2.0, viewport.height / 2.0)
}

/// Generates a random position within the viewport bounds.
/// Use this for natural starting positions for cursor movement.
///
/// # Arguments
/// * `viewport` - The viewport dimensions
/// * `margin` - Optional margin in pixels to keep away from edges (default: 10)
///
/// # Returns
/// (x, y) tuple representing random coordinates within bounds
#[allow(dead_code)]
pub fn random_position(viewport: &Viewport, margin: f64) -> (f64, f64) {
    use crate::utils::math::random_in_range;
    let safe_margin_x = margin.max(1.0).min((viewport.width / 4.0).max(1.0));
    let safe_margin_y = margin.max(1.0).min((viewport.height / 4.0).max(1.0));
    let max_x = (viewport.width - safe_margin_x).max(safe_margin_x + 1.0);
    let max_y = (viewport.height - safe_margin_y).max(safe_margin_y + 1.0);
    let x = random_in_range(safe_margin_x as u64, max_x as u64) as f64;
    let y = random_in_range(safe_margin_y as u64, max_y as u64) as f64;
    (x, y)
}

/// Generates a random viewport position while avoiding the outer edge band.
/// `edge_ratio` is clamped to [0.0, 0.45], where 0.10 means avoid outer 10% on each side.
#[allow(dead_code)]
pub fn random_position_with_edge_ratio(viewport: &Viewport, edge_ratio: f64) -> (f64, f64) {
    use crate::utils::math::random_in_range;

    let clamped_ratio = edge_ratio.clamp(0.0, 0.45);
    let min_x = (viewport.width * clamped_ratio).max(1.0);
    let min_y = (viewport.height * clamped_ratio).max(1.0);
    let max_x = (viewport.width * (1.0 - clamped_ratio)).max(min_x + 1.0);
    let max_y = (viewport.height * (1.0 - clamped_ratio)).max(min_y + 1.0);

    let x = random_in_range(min_x as u64, max_x as u64) as f64;
    let y = random_in_range(min_y as u64, max_y as u64) as f64;
    (x, y)
}

/// Gets the center position of a target element.
/// Uses element.getBoundingClientRect() to find the element's position.
///
/// # Arguments
/// * `page` - The browser page
/// * `selector` - CSS selector for the target element
///
/// # Returns
/// Ok((x, y)) with center coordinates of the element, Err if element not found
pub async fn get_element_center(page: &Page, selector: &str) -> Result<(f64, f64)> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        "
        (() => {{
            const el = document.querySelector({});
            if (!el) return null;
            const rect = el.getBoundingClientRect();
            return {{
                x: rect.left + rect.width / 2,
                y: rect.top + rect.height / 2,
                width: rect.width,
                height: rect.height
            }};
        }})()
    ",
        selector_js
    );

    let result = page.evaluate(js).await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to get element center value"))?;
    let coords: ElementCoords = serde_json::from_value(value.clone())?;

    if !coords.is_valid() {
        anyhow::bail!("Element not found: {}", selector);
    }

    Ok((coords.x, coords.y))
}

#[derive(Deserialize)]
struct ElementCoords {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl ElementCoords {
    fn is_valid(&self) -> bool {
        !self.x.is_nan() && !self.y.is_nan() && self.width > 0.0 && self.height > 0.0
    }
}

/// Gets the bounds of a target element for cursor placement.
/// Returns (min_x, min_y, max_x, max_y) tuple.
///
/// # Arguments
/// * `page` - The browser page
/// * `selector` - CSS selector for the target element
///
/// # Returns
/// Ok((min_x, min_y, max_x, max_y)) with element bounds, Err if element not found
#[allow(dead_code)]
pub async fn get_element_bounds(page: &Page, selector: &str) -> Result<(f64, f64, f64, f64)> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        "
        (() => {{
            const el = document.querySelector({});
            if (!el) return null;
            const rect = el.getBoundingClientRect();
            return {{
                x: rect.left,
                y: rect.top,
                width: rect.width,
                height: rect.height
            }};
        }})()
    ",
        selector_js
    );

    let result = page.evaluate(js).await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to get element bounds value"))?;
    let coords: ElementCoords = serde_json::from_value(value.clone())?;

    if !coords.is_valid() {
        anyhow::bail!("Element not found: {}", selector);
    }

    Ok((
        coords.x,
        coords.y,
        coords.x + coords.width,
        coords.y + coords.height,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_center_position() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };
        let (x, y) = center_position(&viewport);
        assert_eq!(x, 500.0);
        assert_eq!(y, 400.0);
    }

    #[test]
    fn test_center_position_odd_dimensions() {
        let viewport = Viewport {
            width: 1001.0,
            height: 801.0,
        };
        let (x, y) = center_position(&viewport);
        assert_eq!(x, 500.5);
        assert_eq!(y, 400.5);
    }

    #[test]
    fn test_random_position_within_bounds() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };
        let margin = 10.0;

        for _ in 0..50 {
            let (x, y) = random_position(&viewport, margin);
            assert!(x >= margin);
            assert!(x <= viewport.width - margin);
            assert!(y >= margin);
            assert!(y <= viewport.height - margin);
        }
    }

    #[test]
    fn test_random_position_with_large_margin() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };
        let margin = 200.0;

        let (x, y) = random_position(&viewport, margin);
        assert!(x >= margin);
        assert!(x <= viewport.width - margin);
        assert!(y >= margin);
        assert!(y <= viewport.height - margin);
    }

    #[test]
    fn test_random_position_zero_margin() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };

        for _ in 0..20 {
            let (x, y) = random_position(&viewport, 0.0);
            assert!(x >= 1.0);
            assert!(x <= viewport.width - 1.0);
            assert!(y >= 1.0);
            assert!(y <= viewport.height - 1.0);
        }
    }

    #[test]
    fn test_random_position_with_edge_ratio() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };
        let edge_ratio = 0.10;

        for _ in 0..50 {
            let (x, y) = random_position_with_edge_ratio(&viewport, edge_ratio);
            // Use integer bounds to avoid floating-point precision issues
            // Function internally converts to u64, so we check against integer bounds
            let min_x_expected = (viewport.width * edge_ratio) as u64 as f64;
            let max_x_expected = (viewport.width * (1.0 - edge_ratio)) as u64 as f64;
            let min_y_expected = (viewport.height * edge_ratio) as u64 as f64;
            let max_y_expected = (viewport.height * (1.0 - edge_ratio)) as u64 as f64;

            assert!(x >= min_x_expected, "x={} < min_x={}", x, min_x_expected);
            assert!(x <= max_x_expected, "x={} > max_x={}", x, max_x_expected);
            assert!(y >= min_y_expected, "y={} < min_y={}", y, min_y_expected);
            assert!(y <= max_y_expected, "y={} > max_y={}", y, max_y_expected);
        }
    }

    #[test]
    fn test_random_position_with_edge_ratio_clamps_high_ratio() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };

        // Test that ratio is clamped to 0.45
        let (x, y) = random_position_with_edge_ratio(&viewport, 0.50);
        // Use integer bounds to match function's internal conversion
        let min_x_expected = (viewport.width * 0.45) as u64 as f64;
        let max_x_expected = (viewport.width * 0.55) as u64 as f64;
        let min_y_expected = (viewport.height * 0.45) as u64 as f64;
        let max_y_expected = (viewport.height * 0.55) as u64 as f64;

        assert!(x >= min_x_expected, "x={} < min_x={}", x, min_x_expected);
        assert!(x <= max_x_expected, "x={} > max_x={}", x, max_x_expected);
        assert!(y >= min_y_expected, "y={} < min_y={}", y, min_y_expected);
        assert!(y <= max_y_expected, "y={} > max_y={}", y, max_y_expected);
    }

    #[test]
    fn test_random_position_with_edge_ratio_zero() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };

        let (x, y) = random_position_with_edge_ratio(&viewport, 0.0);
        assert!(x >= 1.0);
        assert!(x <= viewport.width - 1.0);
        assert!(y >= 1.0);
        assert!(y <= viewport.height - 1.0);
    }

    #[test]
    fn test_element_coords_valid() {
        let coords = ElementCoords {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 30.0,
        };
        assert!(coords.is_valid());
    }

    #[test]
    fn test_element_coords_invalid_nan() {
        let coords = ElementCoords {
            x: f64::NAN,
            y: 200.0,
            width: 50.0,
            height: 30.0,
        };
        assert!(!coords.is_valid());
    }

    #[test]
    fn test_element_coords_invalid_zero_width() {
        let coords = ElementCoords {
            x: 100.0,
            y: 200.0,
            width: 0.0,
            height: 30.0,
        };
        assert!(!coords.is_valid());
    }

    #[test]
    fn test_element_coords_invalid_zero_height() {
        let coords = ElementCoords {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 0.0,
        };
        assert!(!coords.is_valid());
    }

    #[test]
    fn test_element_coords_invalid_negative_width() {
        let coords = ElementCoords {
            x: 100.0,
            y: 200.0,
            width: -10.0,
            height: 30.0,
        };
        assert!(!coords.is_valid());
    }

    #[test]
    fn test_viewport_struct_fields() {
        let viewport = Viewport {
            width: 1920.0,
            height: 1080.0,
        };
        assert_eq!(viewport.width, 1920.0);
        assert_eq!(viewport.height, 1080.0);
    }

    #[test]
    fn test_document_size_struct_fields() {
        let doc = DocumentSize {
            scroll_width: 1920.0,
            scroll_height: 5000.0,
        };
        assert_eq!(doc.scroll_width, 1920.0);
        assert_eq!(doc.scroll_height, 5000.0);
    }

    #[test]
    fn test_random_position_distribution() {
        let viewport = Viewport {
            width: 1000.0,
            height: 800.0,
        };

        // Test that distribution is reasonably spread
        let positions: Vec<(f64, f64)> =
            (0..100).map(|_| random_position(&viewport, 50.0)).collect();

        // Check that positions span the range
        let min_x = positions.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = positions
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = positions.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_y = positions
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);

        assert!(min_x < 200.0); // Should have some near the left edge
        assert!(max_x > 800.0); // Should have some near the right edge
        assert!(min_y < 200.0); // Should have some near the top edge
        assert!(max_y > 600.0); // Should have some near the bottom edge
    }

    #[test]
    fn test_random_position_small_viewport() {
        let viewport = Viewport {
            width: 100.0,
            height: 100.0,
        };

        let (x, y) = random_position(&viewport, 10.0);
        assert!(x >= 10.0);
        assert!(x <= 90.0);
        assert!(y >= 10.0);
        assert!(y <= 90.0);
    }

    #[test]
    fn test_random_position_with_edge_ratio_small_viewport() {
        let viewport = Viewport {
            width: 100.0,
            height: 100.0,
        };

        let (x, y) = random_position_with_edge_ratio(&viewport, 0.20);
        assert!(x >= 20.0);
        assert!(x <= 80.0);
        assert!(y >= 20.0);
        assert!(y <= 80.0);
    }
}
