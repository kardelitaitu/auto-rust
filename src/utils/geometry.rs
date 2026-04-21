//! Shared geometry primitives used across interaction utilities.

/// Bounding box of a DOM element (matches browser API shape).
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    /// Check if this bbox is approximately equal to another (delta < threshold)
    pub fn approx_eq(&self, other: &BoundingBox, threshold: f64) -> bool {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        dx + dy < threshold
    }
}
