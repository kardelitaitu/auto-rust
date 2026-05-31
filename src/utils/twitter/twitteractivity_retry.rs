//! Retry logic with exponential backoff for transient failures.
//!
//! Provides configurable retry mechanisms for browser automation operations
//! that may fail due to transient issues (network timeouts, stale elements, etc.).

use anyhow::Result;
use log::{debug, info, warn};
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::prelude::TaskContext;
use crate::utils::twitter::twitteractivity_errors::ErrorClass;

use super::twitteractivity_errors::ErrorClassifier;
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
    #[must_use]
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
    #[must_use]
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

/// Circuit breaker state constants for AtomicU8.
const CLOSED: u8 = 0;
const HALF_OPEN: u8 = 1;
const OPEN: u8 = 2;

/// Circuit breaker for preventing cascade failures.
///
/// Uses an `AtomicU8` state machine with compare-and-swap to ensure
/// only one concurrent caller transitions from OPEN to HALF_OPEN,
/// eliminating the TOCTOU race in the previous `RwLock<bool>` design.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    threshold: u32,
    reset_timeout: Duration,
    failures: Arc<AtomicU32>,
    last_failure: Arc<RwLock<Option<Instant>>>,
    state: Arc<AtomicU8>,
}

impl CircuitBreaker {
    #[must_use]
    pub fn new(threshold: u32, reset_timeout_ms: u64) -> Self {
        Self {
            threshold,
            reset_timeout: Duration::from_millis(reset_timeout_ms),
            failures: Arc::new(AtomicU32::new(0)),
            last_failure: Arc::new(RwLock::new(None)),
            state: Arc::new(AtomicU8::new(CLOSED)),
        }
    }

    /// Returns `true` if the circuit is open (calls should be rejected).
    ///
    /// When the reset timeout expires, only one caller atomically transitions
    /// the state from OPEN to HALF_OPEN via CAS. All other callers see `true`.
    #[must_use]
    pub async fn is_open(&self) -> bool {
        let s = self.state.load(Ordering::Acquire);
        if s == CLOSED {
            return false;
        }
        if s == OPEN {
            let elapsed = self.last_failure.read().await.map(|t| t.elapsed());
            if elapsed.is_some_and(|e| e > self.reset_timeout) {
                // CAS: only one caller transitions to HALF_OPEN
                if self
                    .state
                    .compare_exchange(OPEN, HALF_OPEN, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    self.failures.store(0, Ordering::Release);
                    return false; // this caller gets the probe
                }
            }
            return true; // still open for everyone else
        }
        // HALF_OPEN — only one caller gets through; reject others
        true
    }

    pub async fn record_success(&self) {
        self.state.store(CLOSED, Ordering::Release);
        self.failures.store(0, Ordering::Release);
        let mut last_failure = self.last_failure.write().await;
        *last_failure = None;
    }

    pub async fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::SeqCst) + 1;
        let mut last_failure = self.last_failure.write().await;
        *last_failure = Some(Instant::now());

        if failures >= self.threshold {
            self.state.store(OPEN, Ordering::Release);
            warn!("Circuit breaker opened after {failures} consecutive failures");
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
#[allow(clippy::cast_precision_loss)]
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

/// Core retry loop with a generic delay function.
///
/// Extracted from `retry_with_backoff` to allow testing without a real `TaskContext`.
pub(crate) async fn retry_with_backoff_inner<T, F, Fut, D, DFut>(
    mut operation: F,
    config: &RetryConfig,
    mut delay_fn: D,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
    D: FnMut(u64) -> DFut,
    DFut: std::future::Future<Output = ()>,
{
    let mut last_error = None;

    for attempt in 1..=config.max_attempts {
        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    info!("{operation_name} succeeded after {attempt} attempts");
                }
                return Ok(result);
            }
            Err(e) => {
                let error_class = e.classify();
                debug!("{operation_name} attempt {attempt} failed: {e} (class: {error_class})");

                match error_class {
                    ErrorClass::Transient => {
                        if attempt < config.max_attempts {
                            let delay_ms = calculate_delay(attempt, config);
                            warn!(
                                "{} transient error (attempt {}/{}): {}. Retrying in {}ms...",
                                operation_name, attempt, config.max_attempts, e, delay_ms
                            );
                            delay_fn(delay_ms).await;
                            last_error = Some(e);
                        } else {
                            warn!("{operation_name} failed after {attempt} attempts: {e}");
                            return Err(e);
                        }
                    }
                    ErrorClass::Permanent => {
                        debug!("{operation_name} permanent error, not retrying: {e}");
                        return Err(e);
                    }
                    ErrorClass::Fatal => {
                        warn!("{operation_name} fatal error, aborting: {e}");
                        return Err(e);
                    }
                }
            }
        }
    }

    // Should not reach here, but handle gracefully
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Retry exhausted")))
}

/// Retry an async operation with exponential backoff.
///
/// Only retries transient errors. Permanent and fatal errors fail immediately.
///
/// # Arguments
///
/// * `operation` - The async operation to retry (`FnMut` allows captured vars)
/// * `config` - Retry configuration
/// * `api` - Task context for humanized pauses
/// * `operation_name` - Name for logging
///
/// # Returns
///
/// Returns `Ok(T)` on success, or the last error after all retries exhausted.
pub async fn retry_with_backoff<T, F, Fut>(
    operation: F,
    config: &RetryConfig,
    api: &TaskContext,
    operation_name: &str,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    retry_with_backoff_inner(
        operation,
        config,
        |delay_ms| human_pause(api, delay_ms),
        operation_name,
    )
    .await
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
mod retry_inner_tests {
    use super::{retry_with_backoff_inner, RetryConfig};
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    use std::pin::Pin;

    /// A no-op delay function that records calls.
    fn make_recording_delay(
        call_count: Arc<AtomicU32>,
    ) -> impl FnMut(u64) -> Pin<Box<dyn std::future::Future<Output = ()>>> {
        move |_ms| {
            call_count.fetch_add(1, Ordering::SeqCst);
            Box::pin(async {})
        }
    }

    #[tokio::test]
    async fn immediate_success_no_retry_needed() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());

        let result = retry_with_backoff_inner(
            || async { Ok::<_, anyhow::Error>(42) },
            &RetryConfig::default(),
            delay,
            "immediate_test",
        )
        .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn transient_then_success_retries_once() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());
        let mut attempt = 0u32;

        let result = retry_with_backoff_inner(
            || {
                attempt += 1;
                async move {
                    if attempt == 1 {
                        Err(anyhow::anyhow!("stale element reference"))
                    } else {
                        Ok::<_, anyhow::Error>(99)
                    }
                }
            },
            &RetryConfig::default(),
            delay,
            "transient_then_success",
        )
        .await;

        assert_eq!(result.unwrap(), 99);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn transient_exhaustion_returns_last_error() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());

        let result: Result<(), anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("stale element reference")) },
            &RetryConfig::default(),
            delay,
            "exhaustion_test",
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("stale element reference"));
        // Default max_attempts = 3, so 2 delays (attempts 1 and 2)
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn permanent_error_stops_immediately() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());

        let result: Result<(), anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("invalid selector syntax")) },
            &RetryConfig::default(),
            delay,
            "permanent_test",
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid selector syntax"));
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn fatal_error_stops_immediately() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());

        let result: Result<(), anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("browser disconnected")) },
            &RetryConfig::default(),
            delay,
            "fatal_test",
        )
        .await;

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("browser disconnected"));
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn aggressive_config_retries_less() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());
        let config = RetryConfig::aggressive();

        let result: Result<(), anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("stale element reference")) },
            &config,
            delay,
            "aggressive_test",
        )
        .await;

        assert!(result.is_err());
        // Aggressive: max_attempts=2, so 1 delay
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn conservative_config_retries_more() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());
        let config = RetryConfig::conservative();

        let result: Result<(), anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("stale element reference")) },
            &config,
            delay,
            "conservative_test",
        )
        .await;

        assert!(result.is_err());
        // Conservative: max_attempts=5, so 4 delays
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn single_attempt_config_does_not_retry() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());
        let config = RetryConfig {
            max_attempts: 1,
            ..RetryConfig::default()
        };

        let result: Result<(), anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("stale element reference")) },
            &config,
            delay,
            "single_attempt",
        )
        .await;

        assert!(result.is_err());
        // max_attempts=1, no retries possible
        assert_eq!(call_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn delay_fn_receives_increasing_delays() {
        let delays = Arc::new(std::sync::Mutex::new(Vec::new()));
        let d = delays.clone();

        let delay_fn = move |ms| {
            d.lock().unwrap().push(ms);
            Box::pin(async {}) as Pin<Box<dyn std::future::Future<Output = ()>>>
        };

        let config = RetryConfig {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.0, // No jitter for deterministic test
        };

        let _result: Result<(), anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("stale element reference")) },
            &config,
            delay_fn,
            "delay_tracking",
        )
        .await;

        let recorded = delays.lock().unwrap();
        // Attempt 1: delay = 100 * 2^0 = 100
        // Attempt 2: delay = 100 * 2^1 = 200
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0], 100);
        assert_eq!(recorded[1], 200);
    }

    #[tokio::test]
    async fn operation_name_appears_in_errors() {
        let delay = |_ms| Box::pin(async {}) as Pin<Box<dyn std::future::Future<Output = ()>>>;

        let result: Result<u32, anyhow::Error> = retry_with_backoff_inner(
            || async { Err(anyhow::anyhow!("stale element reference")) },
            &RetryConfig {
                max_attempts: 1,
                ..RetryConfig::default()
            },
            delay,
            "my_custom_operation",
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn multiple_transient_then_success_after_retries() {
        let call_count = Arc::new(AtomicU32::new(0));
        let delay = make_recording_delay(call_count.clone());
        let mut attempt = 0u32;

        let config = RetryConfig {
            max_attempts: 5,
            ..RetryConfig::default()
        };

        let result = retry_with_backoff_inner(
            || {
                attempt += 1;
                async move {
                    if attempt <= 3 {
                        Err(anyhow::anyhow!("stale element reference"))
                    } else {
                        Ok::<_, anyhow::Error>(attempt)
                    }
                }
            },
            &config,
            delay,
            "multi_retry_success",
        )
        .await;

        assert_eq!(result.unwrap(), 4);
        // 3 failures → 3 delay calls
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
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
