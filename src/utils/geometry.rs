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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_creation() {
        let bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 50.0);
    }

    #[test]
    fn test_bounding_box_clone() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = bbox1.clone();
        assert_eq!(bbox1.x, bbox2.x);
        assert_eq!(bbox1.y, bbox2.y);
        assert_eq!(bbox1.width, bbox2.width);
        assert_eq!(bbox1.height, bbox2.height);
    }

    #[test]
    fn test_bounding_box_copy() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = bbox1;
        assert_eq!(bbox1.x, bbox2.x);
        assert_eq!(bbox1.y, bbox2.y);
        assert_eq!(bbox1.width, bbox2.width);
        assert_eq!(bbox1.height, bbox2.height);
    }

    #[test]
    fn test_approx_eq_identical() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_within_threshold() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.05,
            y: 20.03,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_exceeds_threshold() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 15.0,
            y: 25.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(!bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_zero_threshold() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.0));
    }

    #[test]
    fn test_approx_eq_negative_coordinates() {
        let bbox1 = BoundingBox {
            x: -10.0,
            y: -20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: -9.95,
            y: -20.03,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_large_threshold() {
        let bbox1 = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 100.0,
            y: 200.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 500.0));
    }

    #[test]
    fn test_approx_eq_x_difference_only() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.05,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_y_difference_only() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.0,
            y: 20.05,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_bounding_box_debug() {
        let bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let debug_str = format!("{:?}", bbox);
        assert!(debug_str.contains("BoundingBox"));
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("20"));
    }
}
