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
}