//! Mathematical utility functions for randomness and Gaussian distributions.
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
/// ```
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
/// ```
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
}
