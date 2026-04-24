//! Internal timing and delay helpers for human-like automation.
//!
//! Provides functions for:
//! - Random delays within specified ranges
//! - Human-like pauses with Gaussian or uniform distribution
//! - Utilities for simulating realistic timing in automated tasks

use crate::utils::math::{gaussian, random_in_range};
use tokio::time::{sleep, Duration};

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
/// ```no_run
/// # use rust_orchestrator::prelude::timing::human_pause;
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// // Pause for approximately 500ms with ±20% variability
/// human_pause(500, 20).await;
/// # });
/// ```
#[allow(dead_code)]
pub async fn human_pause(base_ms: u64, variance_pct: u32) {
    let std_dev = (base_ms as f64) * (variance_pct as f64 / 100.0);
    let min_delay = (base_ms as f64 * 0.1).max(10.0); // Minimum 10ms
    let max_delay = (base_ms as f64 * 3.0).min(30000.0); // Maximum 30s

    let delay = gaussian(base_ms as f64, std_dev, min_delay, max_delay);
    sleep(Duration::from_millis(delay as u64)).await;
}

/// Pauses execution for a uniform random duration around a base value.
///
/// The final delay is sampled uniformly from:
/// `base_ms * (1 - variance_pct)` .. `base_ms * (1 + variance_pct)`
#[allow(dead_code)]
pub async fn uniform_pause(base_ms: u64, variance_pct: u32) {
    let variance = (variance_pct as f64 / 100.0).clamp(0.0, 1.0);
    let min_delay = (base_ms as f64 * (1.0 - variance)).max(10.0);
    let max_delay = (base_ms as f64 * (1.0 + variance)).min(30000.0);
    let delay = random_in_range(min_delay.round() as u64, max_delay.round() as u64);
    sleep(Duration::from_millis(delay)).await;
}

/// Pauses execution in clustered segments with optional micro-movements.
/// Humans don't pause continuously - they pause, fidget slightly, pause again.
///
/// # Arguments
/// * `base_ms` - Total base delay to distribute across clusters
/// * `variance_pct` - Percentage variance to apply
/// * `min_clusters` - Minimum number of pause clusters (default: 1)
/// * `max_clusters` - Maximum number of pause clusters (default: 3)
///
/// # Details
/// Creates 1-3 pause clusters with micro-jitters between them.
/// This reduces rhythmic detection patterns that pure delays exhibit.
#[allow(dead_code)]
pub async fn clustered_pause(
    base_ms: u64,
    variance_pct: u32,
    min_clusters: u64,
    max_clusters: u64,
) {
    let clusters = random_in_range(min_clusters, max_clusters).max(1);
    let base_per_cluster = base_ms / clusters.max(1);
    let variance_per_cluster = variance_pct / 2; // Less variance per segment

    for i in 0..clusters {
        // Each cluster gets a portion of the total delay
        let cluster_delay = if i == clusters - 1 {
            // Last cluster gets remaining time to ensure total is approximately base_ms
            base_ms.saturating_sub(base_per_cluster * (clusters - 1))
        } else {
            base_per_cluster
        };

        // Skip zero-length clusters
        if cluster_delay > 0 {
            let cluster_variance = if i == clusters - 1 {
                variance_pct
            } else {
                variance_per_cluster
            };
            human_pause(cluster_delay, cluster_variance).await;
        }
    }
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

    #[tokio::test]
    async fn test_random_delay_zero_range() {
        let start = std::time::Instant::now();
        random_delay(0, 0).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_random_delay_same_min_max() {
        let start = std::time::Instant::now();
        random_delay(50, 50).await;
        let elapsed = start.elapsed();
        // Should be approximately 50ms (with tolerance)
        assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_random_delay_large_range() {
        let start = std::time::Instant::now();
        random_delay(10, 20).await;
        let elapsed = start.elapsed();
        // Should be between 10-20ms (with tolerance)
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_human_pause_zero_variance() {
        let start = std::time::Instant::now();
        human_pause(20, 0).await;
        let elapsed = start.elapsed();
        // Should be approximately 20ms (with tolerance)
        assert!(elapsed.as_millis() >= 10 && elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_human_pause_high_variance() {
        let start = std::time::Instant::now();
        human_pause(20, 50).await;
        let elapsed = start.elapsed();
        // With high variance, should complete reasonably
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_human_pause_zero_base() {
        let start = std::time::Instant::now();
        human_pause(0, 10).await;
        let elapsed = start.elapsed();
        // Should be very fast (minimum 10ms clamp)
        assert!(elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_human_pause_large_base() {
        let start = std::time::Instant::now();
        human_pause(100, 10).await;
        let elapsed = start.elapsed();
        // Should be approximately 100ms (with variance)
        assert!(elapsed.as_millis() >= 50 && elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_uniform_pause_zero_variance() {
        let start = std::time::Instant::now();
        uniform_pause(20, 0).await;
        let elapsed = start.elapsed();
        // Should be approximately 20ms
        assert!(elapsed.as_millis() >= 10 && elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_uniform_pause_high_variance() {
        let start = std::time::Instant::now();
        uniform_pause(20, 50).await;
        let elapsed = start.elapsed();
        // Should complete reasonably
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_uniform_pause_clamp_min() {
        let start = std::time::Instant::now();
        uniform_pause(10, 20).await;
        let elapsed = start.elapsed();
        // Should be between 10 and 20ms
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 30);
    }

    #[tokio::test]
    async fn test_uniform_pause_clamp_max() {
        let start = std::time::Instant::now();
        uniform_pause(10, 20).await;
        let elapsed = start.elapsed();
        // Should be between 10 and 20ms
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 30);
    }

    #[tokio::test]
    async fn test_clustered_pause_single_cluster() {
        let start = std::time::Instant::now();
        clustered_pause(20, 10, 1, 1).await;
        let elapsed = start.elapsed();
        // Should be approximately 20ms
        assert!(elapsed.as_millis() >= 10 && elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_clustered_pause_multiple_clusters() {
        let start = std::time::Instant::now();
        clustered_pause(30, 10, 2, 3).await;
        let elapsed = start.elapsed();
        // Should be approximately 30ms total
        assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_clustered_pause_zero_base() {
        let start = std::time::Instant::now();
        clustered_pause(0, 10, 1, 3).await;
        let elapsed = start.elapsed();
        // Should be very fast
        assert!(elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_clustered_pause_minimal() {
        // Test minimal parameters for quick completion
        clustered_pause(1, 0, 1, 1).await;
    }

    #[tokio::test]
    async fn test_random_delay_sequence() {
        let start = std::time::Instant::now();
        for _ in 0..5 {
            random_delay(1, 5).await;
        }
        let elapsed = start.elapsed();
        // Should complete in reasonable time
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_human_pause_sequence() {
        let start = std::time::Instant::now();
        for _ in 0..3 {
            human_pause(10, 5).await;
        }
        let elapsed = start.elapsed();
        // Should complete in reasonable time
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_uniform_pause_sequence() {
        let start = std::time::Instant::now();
        for _ in 0..3 {
            uniform_pause(10, 5).await;
        }
        let elapsed = start.elapsed();
        // Should complete in reasonable time
        assert!(elapsed.as_millis() < 100);
    }
}
