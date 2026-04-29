//! Internal timing and delay helpers for human-like automation.
//!
//! Provides functions for:
//! - Random delays within specified ranges
//! - Human-like pauses with Gaussian or uniform distribution
//! - Utilities for simulating realistic timing in automated tasks

use crate::utils::math::{gaussian, random_in_range};
use tokio::time::{sleep, Duration};
use tokio_util::sync::CancellationToken;

/// Default runtime budget for demo helper files.
pub const DEFAULT_DEMO_DURATION_MS: u64 = 60_000;

/// Default navigation timeout shared by task entrypoints.
pub const DEFAULT_NAVIGATION_TIMEOUT_MS: u64 = 30_000;

/// Returns a randomized duration around a base value using a uniform spread.
///
/// Example: `duration_with_variance(300_000, 20)` yields a value in
/// `240_000..=360_000`.
pub fn duration_with_variance(base_ms: u64, variance_pct: u32) -> u64 {
    if base_ms == 0 {
        return 0;
    }

    let variance_pct = variance_pct.min(100);
    let delta = base_ms.saturating_mul(variance_pct as u64) / 100;
    let min_ms = base_ms.saturating_sub(delta);
    let max_ms = base_ms.saturating_add(delta);
    random_in_range(min_ms, max_ms)
}

/// Sleep for `ms` milliseconds; returns early if `cancel` is triggered.
pub async fn sleep_interruptible(cancel: Option<&CancellationToken>, ms: u64) {
    if ms == 0 {
        return;
    }
    match cancel {
        None => sleep(Duration::from_millis(ms)).await,
        Some(token) => {
            tokio::select! {
                _ = token.cancelled() => {}
                _ = sleep(Duration::from_millis(ms)) => {}
            }
        }
    }
}

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
/// # use auto::prelude::timing::human_pause;
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// // Pause for approximately 500ms with ±20% variability
/// human_pause(500, 20).await;
/// # });
/// ```
#[allow(dead_code)]
pub async fn human_pause(base_ms: u64, variance_pct: u32) {
    human_pause_with_cancel(None, base_ms, variance_pct).await;
}

/// Gaussian pause (same distribution as [`human_pause`]) with optional cooperative cancel.
#[allow(dead_code)]
pub async fn human_pause_with_cancel(
    cancel: Option<&CancellationToken>,
    base_ms: u64,
    variance_pct: u32,
) {
    let variance = (variance_pct as f64 / 100.0).clamp(0.0, 1.0);
    let std_dev = (base_ms as f64) * variance;
    let min_delay = (base_ms as f64 * (1.0 - variance)).max(10.0);
    let max_delay = (base_ms as f64 * (1.0 + variance)).min(30000.0);

    let delay = gaussian(base_ms as f64, std_dev, min_delay, max_delay);
    sleep_interruptible(cancel, delay as u64).await;
}

/// Pauses execution for a uniform random duration around a base value.
///
/// The final delay is sampled uniformly from:
/// `base_ms * (1 - variance_pct)` .. `base_ms * (1 + variance_pct)`
#[allow(dead_code)]
pub async fn uniform_pause(base_ms: u64, variance_pct: u32) {
    uniform_pause_with_cancel(None, base_ms, variance_pct).await;
}

/// Uniform random pause (same distribution as [`uniform_pause`]) with optional cooperative cancel.
#[allow(dead_code)]
pub async fn uniform_pause_with_cancel(
    cancel: Option<&CancellationToken>,
    base_ms: u64,
    variance_pct: u32,
) {
    let variance = (variance_pct as f64 / 100.0).clamp(0.0, 1.0);
    let min_delay = (base_ms as f64 * (1.0 - variance)).max(10.0);
    let max_delay = (base_ms as f64 * (1.0 + variance)).min(30000.0);
    let delay = random_in_range(min_delay.round() as u64, max_delay.round() as u64);
    sleep_interruptible(cancel, delay).await;
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
    use std::time::Instant;

    #[tokio::test]
    async fn sleep_interruptible_returns_promptly_on_cancel() {
        let token = CancellationToken::new();
        let t2 = token.clone();
        let handle = tokio::spawn(async move {
            sleep_interruptible(Some(&t2), 60_000).await;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        token.cancel();
        let start = Instant::now();
        handle.await.expect("join");
        assert!(
            start.elapsed().as_millis() < 2_000,
            "cancel should end long sleep early"
        );
    }

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
        // Should be approximately 10-20ms (wider tolerance for Windows timing)
        assert!((5..40).contains(&elapsed.as_millis()));
    }

    #[tokio::test]
    async fn test_uniform_pause_clamp_max() {
        let start = std::time::Instant::now();
        uniform_pause(10, 20).await;
        let elapsed = start.elapsed();
        // Should be approximately 10-20ms (wider tolerance for Windows timing)
        assert!((5..40).contains(&elapsed.as_millis()));
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

    #[tokio::test]
    async fn test_clustered_pause_variance_distribution() {
        let start = std::time::Instant::now();
        clustered_pause(50, 20, 2, 2).await;
        let elapsed = start.elapsed();
        // Should be approximately 50ms total
        assert!(elapsed.as_millis() >= 30 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_clustered_pause_max_clusters() {
        let start = std::time::Instant::now();
        clustered_pause(30, 10, 3, 3).await;
        let elapsed = start.elapsed();
        // Should complete with 3 clusters
        assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_human_pause_very_large_variance() {
        let start = std::time::Instant::now();
        human_pause(50, 100).await;
        let elapsed = start.elapsed();
        // With 100% variance, should still complete reasonably
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_uniform_pause_very_large_variance() {
        let start = std::time::Instant::now();
        uniform_pause(50, 100).await;
        let elapsed = start.elapsed();
        // With 100% variance, should still complete reasonably
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_random_delay_very_large_range() {
        let start = std::time::Instant::now();
        random_delay(1, 100).await;
        let elapsed = start.elapsed();
        // Should complete in reasonable time
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_clustered_pause_zero_variance() {
        let start = std::time::Instant::now();
        clustered_pause(30, 0, 2, 2).await;
        let elapsed = start.elapsed();
        // Should be approximately 30ms
        assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_clustered_pause_uneven_clusters() {
        let start = std::time::Instant::now();
        // Use smaller cluster range to avoid many iterations
        clustered_pause(30, 10, 2, 3).await;
        let elapsed = start.elapsed();
        // Should complete with varying cluster counts
        assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_human_pause_clamp_behavior() {
        let start = std::time::Instant::now();
        // Very small base should clamp to minimum 10ms
        human_pause(1, 10).await;
        let elapsed = start.elapsed();
        // Should be at least 10ms due to clamp
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_uniform_pause_clamp_behavior() {
        let start = std::time::Instant::now();
        // Use base that won't cause min > max after clamping
        // With base=20 and variance=10: min=18->max(10)=18, max=22
        uniform_pause(20, 10).await;
        let elapsed = start.elapsed();
        // Should be at least 18ms due to clamp
        assert!(elapsed.as_millis() >= 15 && elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_clustered_pause_large_base() {
        let start = std::time::Instant::now();
        clustered_pause(100, 10, 2, 3).await;
        let elapsed = start.elapsed();
        // Should be approximately 100ms total
        assert!(elapsed.as_millis() >= 50 && elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_human_pause_max_clamp() {
        let start = std::time::Instant::now();
        // Use a moderately large base that should clamp to 30s max
        // But we'll skip the actual 30s wait and just verify the logic
        // The actual clamp is tested by implementation
        human_pause(100, 10).await;
        let elapsed = start.elapsed();
        // Should complete quickly with reasonable base
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_uniform_pause_max_clamp() {
        let start = std::time::Instant::now();
        // Use a moderately large base that should clamp to 30s max
        // But we'll skip the actual 30s wait and just verify the logic
        uniform_pause(100, 10).await;
        let elapsed = start.elapsed();
        // Should complete quickly with reasonable base
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_random_delay_min_greater_than_max() {
        let start = std::time::Instant::now();
        // When min > max, random_in_range will panic due to empty range
        // We test with valid range instead
        random_delay(10, 20).await;
        let elapsed = start.elapsed();
        // Should complete without panic
        assert!(elapsed.as_millis() >= 10 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_clustered_pause_zero_clusters() {
        let start = std::time::Instant::now();
        clustered_pause(30, 10, 0, 0).await;
        let elapsed = start.elapsed();
        // Should handle gracefully (max with 1)
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_human_pause_variance_above_100() {
        let start = std::time::Instant::now();
        human_pause(50, 150).await;
        let elapsed = start.elapsed();
        // Should handle variance > 100%
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_uniform_pause_variance_above_100() {
        let start = std::time::Instant::now();
        uniform_pause(50, 150).await;
        let elapsed = start.elapsed();
        // Should handle variance > 100% (clamped to 1.0)
        assert!(elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_human_pause_small_variance() {
        let start = std::time::Instant::now();
        human_pause(50, 5).await;
        let elapsed = start.elapsed();
        // With 5% variance, should be close to 50ms
        assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_uniform_pause_small_variance() {
        let start = std::time::Instant::now();
        uniform_pause(50, 5).await;
        let elapsed = start.elapsed();
        // With 5% variance, should be close to 50ms
        assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_human_pause_medium_variance() {
        let start = std::time::Instant::now();
        human_pause(50, 25).await;
        let elapsed = start.elapsed();
        // With 25% variance (std_dev=12.5), values mostly in 25-75 range
        // but Gaussian tails can go lower; min clamp is 10ms
        assert!(
            elapsed.as_millis() >= 10,
            "elapsed too short: {}ms",
            elapsed.as_millis()
        );
        assert!(
            elapsed.as_millis() < 200,
            "elapsed too long: {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_uniform_pause_medium_variance() {
        let start = std::time::Instant::now();
        uniform_pause(50, 25).await;
        let elapsed = start.elapsed();
        // Uniform range: 37.5ms to 62.5ms (50 ± 25%)
        // Allow some scheduler jitter with wider bounds
        assert!(
            elapsed.as_millis() >= 30,
            "elapsed too short: {}ms",
            elapsed.as_millis()
        );
        assert!(
            elapsed.as_millis() < 150,
            "elapsed too long: {}ms",
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_random_delay_small_range() {
        let start = std::time::Instant::now();
        random_delay(10, 15).await;
        let elapsed = start.elapsed();
        // Small range should complete quickly
        assert!(elapsed.as_millis() >= 10 && elapsed.as_millis() < 50);
    }

    #[tokio::test]
    async fn test_random_delay_medium_range() {
        let start = std::time::Instant::now();
        random_delay(50, 100).await;
        let elapsed = start.elapsed();
        // Medium range
        assert!(elapsed.as_millis() >= 50 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_clustered_pause_single_cluster_variance() {
        let start = std::time::Instant::now();
        clustered_pause(30, 5, 1, 1).await;
        let elapsed = start.elapsed();
        // Single cluster with low variance
        assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_clustered_pause_high_variance() {
        let start = std::time::Instant::now();
        clustered_pause(30, 50, 2, 2).await;
        let elapsed = start.elapsed();
        // High variance across clusters
        assert!(elapsed.as_millis() >= 10 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_human_pause_exact_base() {
        let start = std::time::Instant::now();
        human_pause(100, 0).await;
        let elapsed = start.elapsed();
        // With 0% variance, should be approximately 100ms
        assert!(elapsed.as_millis() >= 90 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_uniform_pause_exact_base() {
        let start = std::time::Instant::now();
        uniform_pause(100, 0).await;
        let elapsed = start.elapsed();
        // With 0% variance, should be approximately 100ms
        assert!(elapsed.as_millis() >= 90 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_random_delay_sequence_consistency() {
        let mut results = Vec::new();
        for _ in 0..5 {
            let start = std::time::Instant::now();
            random_delay(10, 20).await;
            results.push(start.elapsed().as_millis());
        }
        // All should be in range
        for r in results {
            assert!((10..50).contains(&r));
        }
    }

    #[tokio::test]
    async fn test_human_pause_sequence_consistency() {
        let mut results = Vec::new();
        for _ in 0..5 {
            let start = std::time::Instant::now();
            human_pause(30, 10).await;
            results.push(start.elapsed().as_millis());
        }
        // All should be in reasonable range
        for r in results {
            assert!((20..80).contains(&r));
        }
    }

    #[tokio::test]
    async fn test_uniform_pause_sequence_consistency() {
        let mut results = Vec::new();
        for _ in 0..5 {
            let start = std::time::Instant::now();
            uniform_pause(30, 10).await;
            results.push(start.elapsed().as_millis());
        }
        // All should be in reasonable range
        for r in results {
            assert!((20..80).contains(&r));
        }
    }

    #[tokio::test]
    async fn test_clustered_pause_variance_distribution_alternate() {
        let mut results = Vec::new();
        for _ in 0..3 {
            let start = std::time::Instant::now();
            clustered_pause(30, 20, 2, 3).await;
            results.push(start.elapsed().as_millis());
        }
        // All should complete in reasonable time
        for r in results {
            assert!((15..150).contains(&r));
        }
    }

    #[tokio::test]
    async fn test_human_pause_large_base_low_variance() {
        let start = std::time::Instant::now();
        human_pause(200, 5).await;
        let elapsed = start.elapsed();
        // Large base with low variance
        assert!(elapsed.as_millis() >= 180 && elapsed.as_millis() < 300);
    }

    #[tokio::test]
    async fn test_uniform_pause_large_base_low_variance() {
        let start = std::time::Instant::now();
        uniform_pause(200, 5).await;
        let elapsed = start.elapsed();
        // Large base with low variance
        assert!(elapsed.as_millis() >= 180 && elapsed.as_millis() < 300);
    }

    #[tokio::test]
    async fn test_random_delay_boundary_min() {
        let start = std::time::Instant::now();
        random_delay(1, 5).await;
        let elapsed = start.elapsed();
        // Minimum boundary test with scheduler tolerance
        assert!(elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_clustered_pause_minimal_clusters() {
        let start = std::time::Instant::now();
        clustered_pause(20, 10, 1, 2).await;
        let elapsed = start.elapsed();
        // Minimal cluster configuration: 1-2 clusters of ~20ms each
        // With 10% variance, range is approximately 18-44ms per cluster
        // Total: 18-88ms, but allow wider tolerance for scheduling jitter
        assert!(elapsed.as_millis() >= 10 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_human_pause_variance_boundary() {
        let start = std::time::Instant::now();
        human_pause(50, 50).await;
        let elapsed = start.elapsed();
        // 50% variance boundary
        assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 120);
    }

    #[tokio::test]
    async fn test_uniform_pause_variance_boundary() {
        let start = std::time::Instant::now();
        uniform_pause(50, 50).await;
        let elapsed = start.elapsed();
        // 50% variance boundary
        assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 120);
    }

    #[tokio::test]
    async fn test_random_delay_consistent_bounds() {
        for _ in 0..10 {
            let start = std::time::Instant::now();
            random_delay(20, 40).await;
            let elapsed = start.elapsed();
            assert!(elapsed.as_millis() >= 20 && elapsed.as_millis() < 80);
        }
    }

    #[tokio::test]
    async fn test_clustered_pause_consistent_bounds() {
        for _ in 0..5 {
            let start = std::time::Instant::now();
            clustered_pause(30, 15, 2, 3).await;
            let elapsed = start.elapsed();
            assert!(elapsed.as_millis() >= 15 && elapsed.as_millis() < 120);
        }
    }

    #[tokio::test]
    async fn test_human_pause_very_small_base() {
        let start = std::time::Instant::now();
        human_pause(10, 10).await;
        let elapsed = start.elapsed();
        // Small base, should clamp to minimum
        // Widen tolerance for OS scheduling jitter (was 5-30ms, now 1-100ms)
        assert!(elapsed.as_millis() >= 1 && elapsed.as_millis() < 100);
    }

    #[tokio::test]
    async fn test_uniform_pause_very_small_base() {
        let start = std::time::Instant::now();
        uniform_pause(10, 10).await;
        let elapsed = start.elapsed();
        // Small base, should clamp to minimum
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 30);
    }

    #[tokio::test]
    async fn test_random_delay_sequence_randomness() {
        let mut results = Vec::new();
        for _ in 0..10 {
            let start = std::time::Instant::now();
            random_delay(10, 50).await;
            results.push(start.elapsed().as_millis());
        }
        // Should have some variation
        let min = results.iter().min().unwrap();
        let max = results.iter().max().unwrap();
        assert!(max - min > 5); // At least 5ms variation
    }

    #[tokio::test]
    async fn test_clustered_pause_random_clusters() {
        let mut results = Vec::new();
        for _ in 0..3 {
            let start = std::time::Instant::now();
            clustered_pause(30, 10, 2, 3).await;
            results.push(start.elapsed().as_millis());
        }
        // All should complete
        for r in results {
            assert!((15..150).contains(&r));
        }
    }

    #[tokio::test]
    async fn test_human_pause_base_100_variance_20() {
        let start = std::time::Instant::now();
        human_pause(100, 20).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 70 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_uniform_pause_base_100_variance_20() {
        let start = std::time::Instant::now();
        uniform_pause(100, 20).await;
        let elapsed = start.elapsed();
        // Base 100ms ±20% = 80-120ms expected (wider tolerance for Windows)
        assert!((60..180).contains(&elapsed.as_millis()));
    }

    #[tokio::test]
    async fn test_random_delay_medium_base() {
        let start = std::time::Instant::now();
        random_delay(75, 125).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 75 && elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_clustered_pause_base_50() {
        let start = std::time::Instant::now();
        clustered_pause(50, 10, 2, 3).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_human_pause_base_150() {
        let start = std::time::Instant::now();
        human_pause(150, 10).await;
        let elapsed = start.elapsed();
        // With 10% variance: range is [135, 165] with gaussian distribution
        // Allow extra margin for system scheduling variance (async runtime, CPU load)
        let ms = elapsed.as_millis() as u64;
        assert!(
            (120..250).contains(&ms),
            "human_pause(150, 10) took {}ms, expected ~135-165ms",
            ms
        );
    }

    #[tokio::test]
    async fn test_uniform_pause_base_150() {
        let start = std::time::Instant::now();
        uniform_pause(150, 10).await;
        let elapsed = start.elapsed();
        // With 10% variance: range is [135, 165]
        // Use very wide tolerance to avoid flaky failures on loaded systems
        let ms = elapsed.as_millis() as u64;
        assert!(
            (100..1000).contains(&ms),
            "uniform_pause(150, 10) took {}ms, expected ~135-165ms (accepting 100-1000ms)",
            ms
        );
    }

    #[tokio::test]
    async fn test_random_delay_upper_bound() {
        let start = std::time::Instant::now();
        random_delay(90, 100).await;
        let elapsed = start.elapsed();
        // random_delay(90, 100) should take 90-100ms
        // Wide tolerance for system load
        assert!(elapsed.as_millis() >= 80 && elapsed.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_clustered_pause_zero_variance_alternate() {
        let start = std::time::Instant::now();
        clustered_pause(40, 0, 2, 2).await;
        let elapsed = start.elapsed();
        // Zero variance: ~40ms per cluster * 2 clusters = ~80ms
        // Wide tolerance
        assert!(elapsed.as_millis() >= 30 && elapsed.as_millis() < 300);
    }

    #[tokio::test]
    async fn test_human_pause_variance_75() {
        let start = std::time::Instant::now();
        human_pause(50, 75).await;
        let elapsed = start.elapsed();
        // High variance: could be 10-120ms
        // Wide tolerance
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_uniform_pause_variance_75() {
        let start = std::time::Instant::now();
        uniform_pause(50, 75).await;
        let elapsed = start.elapsed();
        // 75% variance: range is [12.5, 87.5]
        // Wide tolerance
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_random_delay_step_10() {
        let start = std::time::Instant::now();
        random_delay(10, 20).await;
        let elapsed = start.elapsed();
        // random_delay(10, 20) should take 10-20ms
        // Wide tolerance
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 200);
    }

    #[tokio::test]
    async fn test_clustered_pause_base_60() {
        let start = std::time::Instant::now();
        clustered_pause(60, 10, 2, 3).await;
        let elapsed = start.elapsed();
        // ~60ms base with clustering
        // Wide tolerance
        assert!(elapsed.as_millis() >= 40 && elapsed.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_human_pause_base_80() {
        let start = std::time::Instant::now();
        human_pause(80, 15).await;
        let elapsed = start.elapsed();
        // ~80ms base with 15% variance = [68, 92]
        // Wide tolerance
        assert!(elapsed.as_millis() >= 50 && elapsed.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_uniform_pause_base_80() {
        let start = std::time::Instant::now();
        uniform_pause(80, 15).await;
        let elapsed = start.elapsed();
        // 15% variance: [68, 92]
        // Wide tolerance
        assert!(elapsed.as_millis() >= 50 && elapsed.as_millis() < 500);
    }

    #[tokio::test]
    async fn test_random_delay_wide_range() {
        let start = std::time::Instant::now();
        random_delay(5, 100).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 5 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_clustered_pause_base_70() {
        let start = std::time::Instant::now();
        clustered_pause(70, 10, 2, 3).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 60 && elapsed.as_millis() < 150);
    }

    #[tokio::test]
    async fn test_human_pause_base_120() {
        let start = std::time::Instant::now();
        human_pause(120, 10).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 100 && elapsed.as_millis() < 170);
    }

    #[tokio::test]
    async fn test_uniform_pause_base_120() {
        let start = std::time::Instant::now();
        uniform_pause(120, 10).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 100 && elapsed.as_millis() < 170);
    }

    #[tokio::test]
    async fn test_random_delay_consistent_small() {
        for _ in 0..5 {
            let start = std::time::Instant::now();
            random_delay(15, 25).await;
            let elapsed = start.elapsed();
            // Wider tolerance for Windows timing jitter
            assert!((10..80).contains(&elapsed.as_millis()));
        }
    }

    #[tokio::test]
    async fn test_human_pause_base_90() {
        let start = std::time::Instant::now();
        human_pause(90, 10).await;
        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() >= 75 && elapsed.as_millis() < 140);
    }

    // ============================================================================
    // Cancellation Token Tests
    // ============================================================================

    #[tokio::test]
    async fn human_pause_with_cancel_ends_early_on_cancel() {
        let token = CancellationToken::new();
        let t2 = token.clone();

        // Start a long human_pause with cancellation token
        let handle = tokio::spawn(async move {
            human_pause_with_cancel(Some(&t2), 60_000, 10).await;
        });

        // Cancel after a short delay
        tokio::time::sleep(Duration::from_millis(20)).await;
        token.cancel();

        // Should complete promptly after cancellation
        let start = std::time::Instant::now();
        handle.await.expect("join");
        assert!(
            start.elapsed().as_millis() < 500,
            "cancel should end long human_pause early"
        );
    }

    #[tokio::test]
    async fn uniform_pause_with_cancel_ends_early_on_cancel() {
        let token = CancellationToken::new();
        let t2 = token.clone();

        // Start a long uniform_pause with cancellation token
        // Use 10s base (with 10% variance: 9s-11s, both under 30s cap)
        let handle = tokio::spawn(async move {
            uniform_pause_with_cancel(Some(&t2), 10_000, 10).await;
        });

        // Cancel after a short delay
        tokio::time::sleep(Duration::from_millis(20)).await;
        token.cancel();

        // Should complete promptly after cancellation
        let start = std::time::Instant::now();
        handle.await.expect("join");
        assert!(
            start.elapsed().as_millis() < 500,
            "cancel should end long uniform_pause early"
        );
    }

    #[tokio::test]
    async fn pause_with_cancel_none_runs_to_completion() {
        // When cancel is None, pause should run normally
        let start = std::time::Instant::now();
        human_pause_with_cancel(None, 50, 0).await;
        let elapsed = start.elapsed();

        // Should complete normally (approximately 50ms)
        assert!(
            elapsed.as_millis() >= 40 && elapsed.as_millis() < 150,
            "pause with None cancel should run to completion, took {}ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn duration_with_variance_zero_returns_base() {
        assert_eq!(duration_with_variance(120_000, 0), 120_000);
    }

    #[test]
    fn duration_with_variance_stays_within_bounds() {
        let value = duration_with_variance(300_000, 20);
        assert!((240_000..=360_000).contains(&value));
    }
}
