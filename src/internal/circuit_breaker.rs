//! Circuit breaker implementation for browser operations
//! 
//! This module provides circuit breaker functionality to prevent cascading failures
//! when browser connections become unavailable or unhealthy.

use do_over::circuit_breaker::{CircuitBreaker, CircuitBreakerPolicy};
use do_over::error::DoOverError;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tracing::{debug, error, warn};

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
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_open_after_failures() {
        let cb = BrowserCircuitBreaker::new(2, 1); // Open after 2 failures, 1 sec timeout

        // First failure
        let result = cb.call(async { Err::<(), _>("failed") }).await;
        assert!(result.is_err());

        // Second failure - should open the circuit
        let result = cb.call(async { Err::<(), _>("failed") }).await;
        assert!(result.is_err());

        // Third call should be rejected immediately (circuit open)
        let result = cb.call(async { Ok(()) }).await;
        assert!(result.is_err());
        if let Err(DoOverError::Rejected(_)) = result {
            // Expected
        } else {
            panic!("Expected rejected error");
        }

        // Wait for timeout to expire
        sleep(Duration::from_secs(2)).await;

        // Should now allow a trial call (half-open)
        let result = cb.call(async { Ok("success") }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_does_not_open() {
        let cb = BrowserCircuitBreaker::new(3, 1);

        // Successes should not open the circuit
        for _ in 0..5 {
            let result = cb.call(async { Ok::<(), ()>(()) }).await;
            assert!(result.is_ok());
        }

        // Should still be closed
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_success_closes() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        // Open the circuit
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        // Should be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());

        // Wait for timeout
        sleep(Duration::from_secs(2)).await;

        // Success in half-open should close the circuit
        let result = cb.call(async { Ok("success") }).await;
        assert!(result.is_ok());

        // Should now be closed again
        let result = cb.call(async { Ok("success2") }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_failure_reopens() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        // Open the circuit
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        // Wait for timeout
        sleep(Duration::from_secs(2)).await;

        // Failure in half-open should reopen
        let result = cb.call(async { Err::<(), _>("half-open failed") }).await;
        assert!(result.is_err());

        // Should still be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_default() {
        let cb = BrowserCircuitBreaker::default();
        // Default should work
        let result = cb.call(async { Ok::<i32, ()>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_circuit_breaker_single_failure_threshold() {
        let cb = BrowserCircuitBreaker::new(1, 1);

        // Single failure should open
        let result = cb.call(async { Err::<(), _>("failed") }).await;
        assert!(result.is_err());

        // Should be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_high_threshold() {
        let cb = BrowserCircuitBreaker::new(10, 1);

        // 9 failures should not open
        for _ in 0..9 {
            let result = cb.call(async { Err::<(), _>("failed") }).await;
            assert!(result.is_err());
        }

        // Should still be closed
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_ok());

        // 10th failure should open
        let result = cb.call(async { Err::<(), _>("failed") }).await;
        assert!(result.is_err());

        // Should be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_zero_timeout() {
        let cb = BrowserCircuitBreaker::new(2, 0);

        // Open the circuit
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        // Should be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());

        // Zero timeout should allow immediate retry
        sleep(Duration::from_millis(10)).await;

        let result = cb.call(async { Ok("success") }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_long_timeout() {
        let cb = BrowserCircuitBreaker::new(2, 60);

        // Open the circuit
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        // Should be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());

        // Should still be open after short wait
        sleep(Duration::from_millis(100)).await;

        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_mixed_success_failure() {
        let cb = BrowserCircuitBreaker::new(3, 1);

        // Mix of success and failure
        let _ = cb.call(async { Ok::<(), ()>(()) }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Ok::<(), ()>(()) }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Ok::<(), ()>(()) }).await;

        // Should still be closed (no consecutive failures)
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_ok());

        // 3 consecutive failures should open
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_clone_independent() {
        let cb1 = BrowserCircuitBreaker::new(2, 1);
        let cb2 = cb1.clone();

        // Open cb1
        let _ = cb1.call(async { Err::<(), _>("failed") }).await;
        let _ = cb1.call(async { Err::<(), _>("failed") }).await;

        // cb1 should be open
        let result = cb1.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());

        // cb2 should be independent (closed)
        let result = cb2.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_concurrent_calls() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        // Concurrent failures
        let tasks: Vec<_> = (0..5)
            .map(|_| {
                let cb = cb.clone();
                tokio::spawn(async move {
                    cb.call(async { Err::<(), _>("failed") }).await
                })
            })
            .collect();

        for task in tasks {
            let _ = task.await;
        }

        // Circuit should be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_different_error_types() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        let _ = cb.call(async { Err::<(), &str>("error1") }).await;
        let _ = cb.call(async { Err::<(), &str>("error2") }).await;

        // Should be open regardless of error type
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery_after_timeout() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        // Open circuit
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        sleep(Duration::from_secs(2)).await;

        // Should recover
        let result = cb.call(async { Ok("recovered") }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_multiple_reopen_cycles() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        for cycle in 0..3 {
            // Open circuit
            let _ = cb.call(async { Err::<(), _>("failed") }).await;
            let _ = cb.call(async { Err::<(), _>("failed") }).await;

            // Should be open
            let result = cb.call(async { Ok::<(), ()>(()) }).await;
            assert!(result.is_err(), "Cycle {}: should be open", cycle);

            // Wait for recovery
            sleep(Duration::from_secs(2)).await;

            // Should recover
            let result = cb.call(async { Ok(format!("cycle {}", cycle)) }).await;
            assert!(result.is_ok(), "Cycle {}: should recover", cycle);
        }
    }

    #[tokio::test]
    async fn test_circuit_breaker_with_timeout_error() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        let _ = cb.call(async { Err::<(), _>("timeout") }).await;
        let _ = cb.call(async { Err::<(), _>("timeout") }).await;

        // Should be open
        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_success_after_partial_failures() {
        let cb = BrowserCircuitBreaker::new(3, 1);

        // 2 failures (below threshold)
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        // Success should reset failure count
        let result = cb.call(async { Ok("success") }).await;
        assert!(result.is_ok());

        // Need 3 more failures to open
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;
        let _ = cb.call(async { Err::<(), _>("failed") }).await;

        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_immediate_success() {
        let cb = BrowserCircuitBreaker::new(5, 10);

        let result = cb.call(async { Ok(123) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[tokio::test]
    async fn test_circuit_breaker_string_return_type() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        let result = cb.call(async { Ok("test string".to_string()) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test string");
    }

    #[tokio::test]
    async fn test_circuit_breaker_unit_return_type() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        let result = cb.call(async { Ok::<(), ()>(()) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circuit_breaker_complex_return_type() {
        let cb = BrowserCircuitBreaker::new(2, 1);

        let result = cb
            .call(async { Ok(vec![1, 2, 3, 4, 5]) })
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3, 4, 5]);
    }
}