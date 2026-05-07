//! Retry logic with exponential backoff for transient failures.
//!
//! Provides configurable retry mechanisms for browser automation operations
//! that may fail due to transient issues (network timeouts, stale elements, etc.).

use anyhow::Result;
use log::{debug, info, warn};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::prelude::TaskContext;
use crate::utils::twitter::twitteractivity_errors::ErrorClass;

use super::twitteractivity_errors::{ErrorClassifier};
use super::twitteractivity_humanized::human_pause;

/// Configuration for retry behavior.
#[derive(Debug, Clone, Copy)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (including initial).
    pub max_attempts: u32,
    /// Initial delay between retries in milliseconds.
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds.
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: f64,
    /// Add random jitter to avoid thundering herd (0.0-1.0).
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 500,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl RetryConfig {
    /// Conservative config for critical operations (more retries, longer delays).
    pub fn conservative() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            backoff_multiplier: 1.5,
            jitter_factor: 0.2,
        }
    }

    /// Aggressive config for fast operations (fewer retries, shorter delays).
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 2,
            base_delay_ms: 250,
            max_delay_ms: 2000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

/// Statistics for retry operations.
#[derive(Debug, Default, Clone)]
pub struct RetryStats {
    pub total_attempts: u32,
    pub transient_errors: u32,
    pub permanent_errors: u32,
    pub success_after_retry: u32,
}

/// Circuit breaker for preventing cascade failures.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    threshold: u32,
    reset_timeout: Duration,
    failures: Arc<AtomicU32>,
    last_failure: Arc<RwLock<Option<Instant>>>,
    is_open: Arc<RwLock<bool>>,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, reset_timeout_ms: u64) -> Self {
        Self {
            threshold,
            reset_timeout: Duration::from_millis(reset_timeout_ms),
            failures: Arc::new(AtomicU32::new(0)),
            last_failure: Arc::new(RwLock::new(None)),
            is_open: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn is_open(&self) -> bool {
        let is_open = *self.is_open.read().await;
        if is_open {
            let last_failure_lock = self.last_failure.read().await;
            if let Some(last_time) = *last_failure_lock {
                if last_time.elapsed() > self.reset_timeout {
                    drop(last_failure_lock);
                    let mut is_open_write = self.is_open.write().await;
                    *is_open_write = false;
                    self.failures.store(0, Ordering::SeqCst);
                    return false;
                }
            }
            return true;
        }
        false
    }

    pub async fn record_success(&self) {
        self.failures.store(0, Ordering::SeqCst);
        let mut last_failure = self.last_failure.write().await;
        *last_failure = None;
        let mut is_open = self.is_open.write().await;
        *is_open = false;
    }

    pub async fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        let mut last_failure = self.last_failure.write().await;
        *last_failure = Some(Instant::now());
        
        if failures >= self.threshold {
            let mut is_open = self.is_open.write().await;
            *is_open = true;
            warn!(
                "Circuit breaker opened after {} consecutive failures",
                failures
            );
        }
    }

    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        if self.is_open().await {
            return Err(anyhow::anyhow!("Circuit breaker is open"));
        }

        match operation().await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(e)
            }
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, 30000) // 5 failures, 30s timeout
    }
}

/// Calculate delay with exponential backoff and jitter.
fn calculate_delay(attempt: u32, config: &RetryConfig) -> u64 {
    let base = config.base_delay_ms as f64 * config.backoff_multiplier.powi(attempt as i32 - 1);
    let delay = base.min(config.max_delay_ms as f64);

    // Add jitter
    let jitter = if config.jitter_factor > 0.0 {
        let jitter_range = delay * config.jitter_factor;
        let random_jitter = rand::random::<f64>() * jitter_range;
        random_jitter - (jitter_range / 2.0)
    } else {
        0.0
    };

    (delay + jitter) as u64
}

/// Retry an async operation with exponential backoff.
///
/// Only retries transient errors. Permanent and fatal errors fail immediately.
///
/// # Arguments
///
/// * `operation` - The async operation to retry (FnMut allows captured vars)
/// * `config` - Retry configuration
/// * `api` - Task context for humanized pauses
/// * `operation_name` - Name for logging
///
/// # Returns
///
/// Returns `Ok(T)` on success, or the last error after all retries exhausted.
pub async fn retry_with_backoff<T, F, Fut>(
    mut operation: F,
    config: &RetryConfig,
    api: &TaskContext,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("{} succeeded after {} attempts", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                let error_class = e.classify();
                debug!(
                    "{} attempt {} failed: {} (class: {})",
                    operation_name, attempt, e, error_class
                );

                match error_class {
                    ErrorClass::Transient => {
                        if attempt < config.max_attempts {
                            let delay_ms = calculate_delay(attempt, config);
                            warn!(
                                "{} transient error (attempt {}/{}): {}. Retrying in {}ms...",
                                operation_name, attempt, config.max_attempts, e, delay_ms
                            );
                            human_pause(api, delay_ms).await;
                            last_error = Some(e);
                        } else {
                            warn!(
                                "{} failed after {} attempts: {}",
                                operation_name, attempt, e
                            );
                            return Err(e);
                        }
                    }
                    ErrorClass::Permanent => {
                        debug!("{} permanent error, not retrying: {}", operation_name, e);
                        return Err(e);
                    }
                    ErrorClass::Fatal => {
                        warn!("{} fatal error, aborting: {}", operation_name, e);
                        return Err(e);
                    }
                }
            }
        }
    }

    // Should not reach here, but handle gracefully
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Retry exhausted")))
}

/// Retry wrapper that returns Result with graceful degradation.
///
/// On failure, returns `Ok(false)` instead of error, allowing the caller
/// to continue execution without killing the session.
///
/// # Arguments
///
/// * `operation` - The async operation to retry
/// * `config` - Retry configuration
/// * `api` - Task context for humanized pauses
/// * `operation_name` - Name for logging
/// * `metric_counter_name` - Counter to increment on failure (optional)
///
/// # Returns
///
/// Returns `Ok(true)` on success, `Ok(false)` on failure (after retries).
pub async fn retry_with_fallback<T, F, Fut>(
    operation: F,
    config: &RetryConfig,
    api: &TaskContext,
    operation_name: &str,
) -> Result<bool>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    match retry_with_backoff(operation, config, api, operation_name).await {
        Ok(_) => Ok(true),
        Err(e) => {
            warn!("{} failed after retries, continuing: {}", operation_name, e);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::RetryConfig;

    #[test]
    fn retry_config_default_values() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.base_delay_ms, 500);
    }
}

#[cfg(test)]
mod delay_tests {
    use super::{calculate_delay, RetryConfig};

    #[test]
    fn calculate_delay_stays_in_expected_range() {
        let config = RetryConfig::default();

        // First attempt: base delay
        let d1 = calculate_delay(1, &config);
        assert!((400..=600).contains(&d1)); // with jitter

        // Second attempt: 2x base
        let d2 = calculate_delay(2, &config);
        assert!((900..=1100).contains(&d2));
    }
}

#[cfg(test)]
mod circuit_breaker_tests {
    use super::CircuitBreaker;

    #[tokio::test]
    async fn circuit_breaker_opens_and_closes_as_expected() {
        let cb = CircuitBreaker::new(3, 1000);

        assert!(!cb.is_open().await);

        cb.record_failure().await;
        cb.record_failure().await;
        assert!(!cb.is_open().await);

        cb.record_failure().await;
        assert!(cb.is_open().await);

        // After success, should be closed
        cb.record_success().await;
        assert!(!cb.is_open().await);
    }
}
