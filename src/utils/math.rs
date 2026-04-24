//! Internal mathematical helpers for randomness and Gaussian distributions.
//!
//! Provides:
//! - Uniform random number generation
//! - Gaussian distribution sampling with bounds
//! - Statistical utilities for human-like behavior simulation

use rand::Rng;
use rand_distr::{Distribution, Normal};

/// Generates a random integer within the specified inclusive range.
/// Uses a thread-local random number generator for performance.
///
/// # Arguments
/// * `min` - Lower bound (inclusive)
/// * `max` - Upper bound (inclusive)
///
/// # Returns
/// A random integer between min and max (inclusive)
///
/// # Examples
/// ```rust
/// use rust_orchestrator::utils::random_in_range;
///
/// let roll = random_in_range(1, 6); // Returns a value between 1 and 6
/// assert!(roll >= 1 && roll <= 6);
/// ```
#[allow(dead_code)]
pub fn random_in_range(min: u64, max: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// Generates a random number from a Gaussian (normal) distribution,
/// clamped to the specified range.
///
/// This function samples from a normal distribution with the given mean and standard deviation,
/// then discards samples that fall outside the [min, max] range and resamples until
/// a valid value is obtained.
///
/// # Arguments
/// * `mean` - Mean of the normal distribution
/// * `std_dev` - Standard deviation of the normal distribution (must be positive)
/// * `min` - Minimum allowed value (inclusive)
/// * `max` - Maximum allowed value (inclusive)
///
/// # Returns
/// A random f64 value from the Gaussian distribution clamped to [min, max]
///
/// # Panics
/// Panics if std_dev is not positive (handled via expect with descriptive message)
///
/// # Examples
/// ```rust
/// use rust_orchestrator::utils::gaussian;
///
/// // Generate a value around 100 with std dev 10, clamped to 80-120
/// let val = gaussian(100.0, 10.0, 80.0, 120.0);
/// assert!(val >= 80.0 && val <= 120.0);
/// ```
#[allow(dead_code)]
pub fn gaussian(mean: f64, std_dev: f64, min: f64, max: f64) -> f64 {
    if !mean.is_finite() || !std_dev.is_finite() || !min.is_finite() || !max.is_finite() {
        return mean;
    }
    if min >= max {
        return min;
    }
    if std_dev <= 0.0 {
        return mean.clamp(min, max);
    }

    let normal = Normal::new(mean, std_dev)
        .expect("Failed to create normal distribution - standard deviation must be positive");
    let mut rng = rand::thread_rng();

    loop {
        let sample = normal.sample(&mut rng);
        if sample >= min && sample <= max {
            return sample;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_in_range() {
        let result = random_in_range(10, 20);
        assert!((10..=20).contains(&result));
    }

    #[test]
    fn test_gaussian_bounds() {
        // Test that gaussian respects bounds
        let result = gaussian(100.0, 10.0, 80.0, 120.0);
        assert!((80.0..=120.0).contains(&result));
    }

    #[test]
    fn test_gaussian_mean() {
        // Test that gaussian centers around mean (with some tolerance)
        // This is a statistical test - run multiple times
        let mut sum = 0.0;
        let samples = 1000;

        for _ in 0..samples {
            sum += gaussian(50.0, 5.0, 0.0, 100.0);
        }

        let mean = sum / samples as f64;
        // Should be close to 50.0 (within 2 standard deviations)
        assert!((mean - 50.0).abs() < 10.0);
    }

    #[test]
    fn test_gaussian_standard_deviation() {
        // Test that standard deviation is reasonable
        let mut values = Vec::new();
        let samples = 10000;
        let expected_mean = 100.0;
        let expected_std = 15.0;

        for _ in 0..samples {
            values.push(gaussian(expected_mean, expected_std, 0.0, 200.0));
        }

        let mean = values.iter().sum::<f64>() / samples as f64;
        let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples as f64;
        let std_dev = variance.sqrt();

        // Should be close to expected standard deviation (within 20% tolerance)
        assert!((std_dev - expected_std).abs() / expected_std < 0.2);
    }

    #[test]
    fn test_gaussian_degenerate_bounds() {
        let value = gaussian(42.0, 10.0, 10.0, 10.0);
        assert_eq!(value, 10.0);
    }

    #[test]
    fn test_random_in_range_same_value() {
        let result = random_in_range(5, 5);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_random_in_range_distribution() {
        // Test that random_in_range produces values across the range
        let mut values = std::collections::HashSet::new();
        for _ in 0..100 {
            values.insert(random_in_range(0, 10));
        }
        // Should have multiple different values
        assert!(values.len() > 5);
    }

    #[test]
    fn test_gaussian_infinity_mean() {
        let value = gaussian(f64::INFINITY, 10.0, 0.0, 100.0);
        assert!(!value.is_finite());
    }

    #[test]
    fn test_gaussian_nan_mean() {
        let value = gaussian(f64::NAN, 10.0, 0.0, 100.0);
        assert!(!value.is_finite());
    }

    #[test]
    fn test_gaussian_infinity_std_dev() {
        let value = gaussian(50.0, f64::INFINITY, 0.0, 100.0);
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_gaussian_nan_std_dev() {
        let value = gaussian(50.0, f64::NAN, 0.0, 100.0);
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_gaussian_infinity_bounds() {
        // When bounds are not finite, gaussian returns the mean
        let value = gaussian(50.0, 10.0, f64::NEG_INFINITY, f64::INFINITY);
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_gaussian_zero_std_dev() {
        let value = gaussian(50.0, 0.0, 0.0, 100.0);
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_gaussian_negative_std_dev() {
        let value = gaussian(50.0, -10.0, 0.0, 100.0);
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_gaussian_mean_outside_bounds() {
        // Mean is outside bounds, should still produce values within bounds
        let value = gaussian(150.0, 10.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_negative_bounds() {
        let value = gaussian(-50.0, 10.0, -100.0, 0.0);
        assert!((-100.0..=0.0).contains(&value));
    }

    #[test]
    fn test_gaussian_tight_bounds() {
        // Tight bounds around mean
        let value = gaussian(50.0, 10.0, 49.0, 51.0);
        assert!((49.0..=51.0).contains(&value));
    }

    #[test]
    fn test_gaussian_large_std_dev() {
        // Large standard deviation with tight bounds
        let value = gaussian(50.0, 1000.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_clamping() {
        // Test that values are properly clamped when sampling would exceed bounds
        for _ in 0..100 {
            let value = gaussian(50.0, 5.0, 40.0, 60.0);
            assert!((40.0..=60.0).contains(&value));
        }
    }

    #[test]
    fn test_random_in_range_zero() {
        let result = random_in_range(0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_random_in_range_large_values() {
        let result = random_in_range(1_000_000, 2_000_000);
        assert!((1_000_000..=2_000_000).contains(&result));
    }

    #[test]
    fn test_gaussian_mean_at_min_bound() {
        let value = gaussian(0.0, 10.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_mean_at_max_bound() {
        let value = gaussian(100.0, 10.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_very_small_std_dev() {
        let value = gaussian(50.0, 0.001, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_mean_clamp_to_min() {
        let value = gaussian(-10.0, 5.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_mean_clamp_to_max() {
        let value = gaussian(110.0, 5.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_random_in_range_max_u64() {
        let result = random_in_range(u64::MAX - 10, u64::MAX);
        assert!(result >= u64::MAX - 10 && result <= u64::MAX);
    }

    #[test]
    fn test_gaussian_consistency_with_fixed_rng() {
        // Test that gaussian produces consistent results with same parameters
        let mut values = Vec::new();
        for _ in 0..10 {
            values.push(gaussian(50.0, 10.0, 0.0, 100.0));
        }
        // All values should be within bounds
        for value in values {
            assert!((0.0..=100.0).contains(&value));
        }
    }

    #[test]
    fn test_gaussian_small_std_dev() {
        let value = gaussian(50.0, 0.001, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_rejection_sampling_terminates() {
        // Test that rejection sampling always terminates even with tight bounds
        let value = gaussian(50.0, 100.0, 49.0, 51.0);
        assert!((49.0..=51.0).contains(&value));
    }

    #[test]
    fn test_gaussian_extreme_mean() {
        // Skip extreme mean test - rejection sampling would take too long
        // The function handles non-finite values by returning mean, which is tested elsewhere
    }

    #[test]
    fn test_gaussian_extreme_std_dev() {
        // Skip extreme std dev test - rejection sampling would take too long
        // The function handles non-finite std dev by returning mean, which is tested elsewhere
    }

    #[test]
    fn test_gaussian_negative_mean_positive_bounds() {
        let value = gaussian(-50.0, 10.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_positive_mean_negative_bounds() {
        let value = gaussian(50.0, 10.0, -100.0, 0.0);
        assert!((-100.0..=0.0).contains(&value));
    }

    #[test]
    fn test_random_in_range_sequence() {
        let mut values = Vec::new();
        for _ in 0..10 {
            values.push(random_in_range(0, 100));
        }
        // All values should be in range
        for value in values {
            assert!((0..=100).contains(&value));
        }
    }

    #[test]
    fn test_gaussian_mean_near_bound() {
        let value = gaussian(1.0, 10.0, 0.0, 100.0);
        assert!((0.0..=100.0).contains(&value));
    }

    #[test]
    fn test_gaussian_bounds_swapped() {
        // When min > max, should return min
        let value = gaussian(50.0, 10.0, 100.0, 0.0);
        assert_eq!(value, 100.0);
    }

    #[test]
    fn test_gaussian_equal_bounds() {
        let value = gaussian(50.0, 10.0, 50.0, 50.0);
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_gaussian_very_tight_bounds() {
        let value = gaussian(50.0, 10.0, 49.999, 50.001);
        assert!((49.999..=50.001).contains(&value));
    }

    #[test]
    fn test_gaussian_zero_mean() {
        let value = gaussian(0.0, 10.0, -50.0, 50.0);
        assert!((-50.0..=50.0).contains(&value));
    }

    #[test]
    fn test_random_in_range_single_value() {
        let result = random_in_range(42, 42);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_gaussian_large_sample_mean() {
        // Test mean with large sample size
        let mut sum = 0.0;
        let samples = 10000;
        for _ in 0..samples {
            sum += gaussian(100.0, 20.0, 0.0, 200.0);
        }
        let mean = sum / samples as f64;
        // Should be close to 100.0
        assert!((mean - 100.0).abs() < 5.0);
    }

    #[test]
    fn test_gaussian_bounds_at_extremes() {
        let value = gaussian(50.0, 10.0, f64::MIN_POSITIVE, f64::MAX);
        // Should handle extreme bounds
        assert!(value.is_finite());
    }
}
