//! Circuit breaker implementation for browser operations
//!
//! This module provides circuit breaker functionality to prevent cascading failures
//! when browser connections become unavailable or unhealthy.

use do_over::circuit_breaker::CircuitBreakerPolicy;
use do_over::error::DoOverError;
use std::future::Future;
use std::time::Duration;
use tracing::{debug, warn};

/// Circuit breaker specifically designed for browser operations
#[derive(Clone)]
pub struct BrowserCircuitBreaker {
    /// The underlying circuit breaker policy
    policy: CircuitBreakerPolicy,
}

impl BrowserCircuitBreaker {
    /// Creates a new browser circuit breaker with the specified failure threshold and timeout
    ///
    /// # Arguments
    /// * `failure_threshold` - Number of consecutive failures before opening the circuit
    /// * `timeout_secs` - Duration to wait before attempting to close the circuit (half-open state)
    pub fn new(failure_threshold: usize, timeout_secs: u64) -> Self {
        let policy = CircuitBreakerPolicy::new(failure_threshold)
            .timeout(Duration::from_secs(timeout_secs))
            .on_open(|state| {
                warn!(
                    "Browser circuit breaker OPEN: {} consecutive failures",
                    state.num_consecutive_failures()
                );
            })
            .on_half_open(|_| {
                debug!("Browser circuit breaker HALF_OPEN: testing connection recovery");
            })
            .on_closed(|_| {
                debug!("Browser circuit breaker CLOSED: connection restored");
            });

        Self { policy }
    }

    /// Executes a future with circuit breaker protection
    ///
    /// # Arguments
    /// * `future` - Future representing the browser operation to execute
    ///
    /// # Returns
    /// * Result containing either the successful output or a DoOverError
    pub async fn call<F, T, E>(&self, mut future: F) -> Result<T, DoOverError<E>>
    where
        F: Future<Output = Result<T, E>>,
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        self.policy.call_async(&mut future).await
    }
}

impl Default for BrowserCircuitBreaker {
    fn default() -> Self {
        // Use sensible defaults matching existing configuration
        Self::new(5, 30) // 5 failures, 30 second timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    use tokio::time::sleep;

    fn io_error(message: &'static str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, message)
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_and_short_circuits_future() {
        let cb = BrowserCircuitBreaker::new(2, 1);
        let calls = Arc::new(AtomicUsize::new(0));

        let result = cb
            .call({
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(io_error("failed"))
                }
            })
            .await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));

        let result = cb
            .call({
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err::<(), _>(io_error("failed"))
                }
            })
            .await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        let result = cb
            .call({
                let calls = Arc::clone(&calls);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, std::io::Error>("should not run")
                }
            })
            .await;
        assert!(matches!(result, Err(DoOverError::CircuitOpen)));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovers_after_timeout() {
        let cb = BrowserCircuitBreaker::new(1, 1);

        let result = cb.call(async { Err::<(), _>(io_error("failed")) }).await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));

        let result = cb.call(async { Ok::<_, std::io::Error>(()) }).await;
        assert!(matches!(result, Err(DoOverError::CircuitOpen)));

        sleep(Duration::from_millis(1100)).await;

        let result = cb
            .call(async { Ok::<_, std::io::Error>("recovered") })
            .await;
        assert_eq!(result.unwrap(), "recovered");
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_resets_failure_count() {
        let cb = BrowserCircuitBreaker::new(3, 1);

        let result = cb.call(async { Err::<(), _>(io_error("failed-1")) }).await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));
        let result = cb.call(async { Err::<(), _>(io_error("failed-2")) }).await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));

        let result = cb.call(async { Ok::<_, std::io::Error>(()) }).await;
        assert!(result.is_ok());

        // Three fresh consecutive failures are required after the success.
        let result = cb.call(async { Err::<(), _>(io_error("failed-3")) }).await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));
        let result = cb.call(async { Err::<(), _>(io_error("failed-4")) }).await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));
        let result = cb.call(async { Err::<(), _>(io_error("failed-5")) }).await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));

        let result = cb.call(async { Ok::<_, std::io::Error>(()) }).await;
        assert!(matches!(result, Err(DoOverError::CircuitOpen)));
    }

    #[tokio::test]
    async fn test_circuit_breaker_clone_has_independent_state() {
        let cb1 = BrowserCircuitBreaker::new(1, 1);
        let cb2 = cb1.clone();

        let result = cb1.call(async { Err::<(), _>(io_error("failed")) }).await;
        assert!(matches!(result, Err(DoOverError::Inner(_))));

        let result = cb1.call(async { Ok::<_, std::io::Error>(()) }).await;
        assert!(matches!(result, Err(DoOverError::CircuitOpen)));

        let result = cb2.call(async { Ok::<_, std::io::Error>("ok") }).await;
        assert_eq!(result.unwrap(), "ok");
    }

    #[tokio::test]
    async fn test_circuit_breaker_default_allows_success() {
        let cb = BrowserCircuitBreaker::default();
        let result = cb.call(async { Ok::<_, std::io::Error>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }
}
