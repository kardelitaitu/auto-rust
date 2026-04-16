//! Timing and delay utility functions for human-like automation.
//!
//! Provides functions for:
//! - Random delays within specified ranges
//! - Human-like pauses with Gaussian distribution
//! - Utilities for simulating realistic timing in automated tasks

use tokio::time::{sleep, Duration};
use crate::utils::math::{random_in_range, gaussian};

/// Pauses execution for a random duration within the specified range.
/// Useful for simulating human-like timing variations in automated tasks.
///
/// # Arguments
/// * `min_ms` - Minimum delay in milliseconds (inclusive)
/// * `max_ms` - Maximum delay in milliseconds (inclusive)
///
/// # Details
/// The delay is uniformly distributed between min_ms and max_ms.
/// This function is asynchronous and uses Tokio's sleep timer.
#[allow(dead_code)]
pub async fn random_delay(min_ms: u64, max_ms: u64) {
    let delay = random_in_range(min_ms, max_ms);
    sleep(Duration::from_millis(delay)).await;
}

/// Pauses execution for a human-like duration with Gaussian distribution.
/// Creates a delay that mimics human reaction and thinking time with variability.
///
/// # Arguments
/// * `base_ms` - Base delay in milliseconds
/// * `variance_pct` - Percentage variance to apply to the base delay
///
/// # Details
/// Uses a Gaussian (normal) distribution centered on `base_ms` with standard deviation
/// calculated as `(base_ms * variance_pct / 100)`. The result is clamped to a
/// reasonable range (10ms to 30s) to prevent extreme values.
///
/// # Examples
/// ```
/// // Pause for approximately 500ms with ±20% variability
/// human_pause(500, 20).await;
/// ```
#[allow(dead_code)]
pub async fn human_pause(base_ms: u64, variance_pct: u32) {
    let std_dev = (base_ms as f64) * (variance_pct as f64 / 100.0);
    let min_delay = (base_ms as f64 * 0.1).max(10.0); // Minimum 10ms
    let max_delay = (base_ms as f64 * 3.0).min(30000.0); // Maximum 30s

    let delay = gaussian(base_ms as f64, std_dev, min_delay, max_delay);
    sleep(Duration::from_millis(delay as u64)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_random_delay_bounds() {
        // Test that random_delay works without panicking
        // We don't test exact timing due to async scheduling variability
        let start = std::time::Instant::now();

        // Test with minimal delay
        random_delay(0, 1).await;

        let elapsed = start.elapsed();
        // Should complete reasonably quickly (allow for async overhead)
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_human_pause_bounds() {
        let start = std::time::Instant::now();

        // Test with minimal variance to make test faster
        human_pause(10, 1).await;

        let elapsed = start.elapsed();
        // Should complete in reasonable time
        assert!(elapsed.as_millis() < 50); // Allow some scheduling overhead
    }
}