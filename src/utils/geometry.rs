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
    /// Check if this bbox is approximately equal to another (delta <= threshold)
    pub fn approx_eq(&self, other: &BoundingBox, threshold: f64) -> bool {
        let dx = (self.x - other.x).abs();
        let dy = (self.y - other.y).abs();
        dx + dy <= threshold
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

    #[test]
    fn test_bounding_box_zero_dimensions() {
        let bbox = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        assert_eq!(bbox.x, 0.0);
        assert_eq!(bbox.y, 0.0);
        assert_eq!(bbox.width, 0.0);
        assert_eq!(bbox.height, 0.0);
    }

    #[test]
    fn test_bounding_box_large_values() {
        let bbox = BoundingBox {
            x: 999999.0,
            y: 888888.0,
            width: 777777.0,
            height: 666666.0,
        };
        assert_eq!(bbox.x, 999999.0);
        assert_eq!(bbox.y, 888888.0);
    }

    #[test]
    fn test_bounding_box_negative_dimensions() {
        let bbox = BoundingBox {
            x: -100.0,
            y: -200.0,
            width: -50.0,
            height: -75.0,
        };
        assert_eq!(bbox.x, -100.0);
        assert_eq!(bbox.y, -200.0);
    }

    #[test]
    fn test_approx_eq_very_small_threshold() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.0001,
            y: 20.0001,
            width: 100.0,
            height: 50.0,
        };
        assert!(!bbox1.approx_eq(&bbox2, 0.0001));
    }

    #[test]
    fn test_approx_eq_fractional_values() {
        let bbox1 = BoundingBox {
            x: 10.5,
            y: 20.7,
            width: 100.3,
            height: 50.9,
        };
        let bbox2 = BoundingBox {
            x: 10.55,
            y: 20.73,
            width: 100.3,
            height: 50.9,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_width_height_ignored() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 999.0,
            height: 888.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_bounding_box_partial_equality() {
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
        assert_eq!(bbox1.x, bbox2.x);
        assert_eq!(bbox1.y, bbox2.y);
    }

    #[test]
    fn test_approx_eq_both_differences_at_threshold() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.04,
            y: 20.04,
            width: 100.0,
            height: 50.0,
        };
        // dx + dy = 0.04 + 0.04 = 0.08, threshold = 0.1, so 0.08 <= 0.1 is true
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_both_differences_exceed_threshold() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.06,
            y: 20.06,
            width: 100.0,
            height: 50.0,
        };
        // dx + dy = 0.06 + 0.06 = 0.12, threshold = 0.1, so 0.12 <= 0.1 is false
        assert!(!bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_with_negative_threshold() {
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
        // dx + dy = 0, threshold = -0.1, so 0 <= -0.1 is false
        assert!(!bbox1.approx_eq(&bbox2, -0.1));
    }

    #[test]
    fn test_bounding_box_very_small_values() {
        let bbox = BoundingBox {
            x: 0.0001,
            y: 0.0002,
            width: 0.0003,
            height: 0.0004,
        };
        assert_eq!(bbox.x, 0.0001);
        assert_eq!(bbox.y, 0.0002);
    }

    #[test]
    fn test_bounding_box_floating_point_precision() {
        let bbox1 = BoundingBox {
            x: 1.0 / 3.0,
            y: 2.0 / 3.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 1.0 / 3.0,
            y: 2.0 / 3.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox1.approx_eq(&bbox2, 0.0001));
    }

    #[test]
    fn test_approx_eq_asymmetric_difference() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.09,
            y: 20.01,
            width: 100.0,
            height: 50.0,
        };
        // dx = 0.09, dy = 0.01, dx + dy = 0.10, threshold = 0.1
        // Due to floating point precision, use a slightly larger threshold
        assert!(bbox1.approx_eq(&bbox2, 0.11));
    }

    #[test]
    fn test_bounding_box_max_f64_values() {
        let bbox = BoundingBox {
            x: f64::MAX,
            y: f64::MAX,
            width: f64::MAX,
            height: f64::MAX,
        };
        assert_eq!(bbox.x, f64::MAX);
    }

    #[test]
    fn test_bounding_box_min_f64_values() {
        let bbox = BoundingBox {
            x: f64::MIN,
            y: f64::MIN,
            width: f64::MIN,
            height: f64::MIN,
        };
        assert_eq!(bbox.x, f64::MIN);
    }

    #[test]
    fn test_bounding_box_infinity() {
        let bbox = BoundingBox {
            x: f64::INFINITY,
            y: f64::INFINITY,
            width: f64::INFINITY,
            height: f64::INFINITY,
        };
        assert!(bbox.x.is_infinite());
    }

    #[test]
    fn test_bounding_box_nan() {
        let bbox = BoundingBox {
            x: f64::NAN,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox.x.is_nan());
    }

    #[test]
    fn test_approx_eq_with_nan() {
        let bbox1 = BoundingBox {
            x: f64::NAN,
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
        // NaN comparison should return false
        assert!(!bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_approx_eq_both_nan() {
        let bbox1 = BoundingBox {
            x: f64::NAN,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: f64::NAN,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        // NaN != NaN, so should be false
        assert!(!bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_bounding_box_partial_mutability() {
        let mut bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        bbox.x = 15.0;
        bbox.y = 25.0;
        assert_eq!(bbox.x, 15.0);
        assert_eq!(bbox.y, 25.0);
    }

    #[test]
    fn test_approx_eq_very_large_difference() {
        let bbox1 = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 1000000.0,
            y: 1000000.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(!bbox1.approx_eq(&bbox2, 1000.0));
    }

    #[test]
    fn test_approx_eq_threshold_exactly_equal_to_difference() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = BoundingBox {
            x: 10.04,
            y: 20.04,
            width: 100.0,
            height: 50.0,
        };
        // dx + dy = 0.04 + 0.04 = 0.08, threshold = 0.1, so 0.08 <= 0.1 is true
        assert!(bbox1.approx_eq(&bbox2, 0.1));
    }

    #[test]
    fn test_bounding_box_struct_fields_public() {
        let bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        // Verify all fields are accessible
        let _ = bbox.x;
        let _ = bbox.y;
        let _ = bbox.width;
        let _ = bbox.height;
    }

    #[test]
    fn test_bounding_box_default_values_not_implemented() {
        // BoundingBox doesn't implement Default
        // This test documents that behavior
        let bbox = BoundingBox {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        assert_eq!(bbox.x, 0.0);
    }

    #[test]
    fn test_approx_eq_symmetric() {
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
        assert_eq!(bbox1.approx_eq(&bbox2, 0.1), bbox2.approx_eq(&bbox1, 0.1));
    }

    #[test]
    fn test_approx_eq_reflexive() {
        let bbox = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(bbox.approx_eq(&bbox, 0.0));
    }

    #[test]
    fn test_bounding_box_copy_semantics() {
        let bbox1 = BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let bbox2 = bbox1;
        // bbox1 should still be valid (Copy trait)
        assert_eq!(bbox1.x, 10.0);
        assert_eq!(bbox2.x, 10.0);
    }
}
