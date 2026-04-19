//! HTTP API client module with circuit breaker and retry support.
//!
//! Provides:
//! - Type-safe HTTP request handling with reqwest
//! - Circuit breaker pattern for fault tolerance
//! - Automatic timeout configuration
//! - Exponential backoff with jitter for retries
//! - JSON serialization/deserialization support

use anyhow::{Result, anyhow, bail};
use log::{warn, info};
use parking_lot::Mutex;
use rand::Rng;
use reqwest::{Client, Method, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use serde::de::DeserializeOwned;

/// HTTP client for making API requests with consistent error handling.
/// Provides a wrapper around reqwest with timeout and retry capabilities.
/// Used for communicating with external APIs like RoxyBrowser.
pub struct ApiClient {
    /// Underlying HTTP client instance
    client: Client,
    /// Base URL for all API requests
    base_url: String,
    /// Optional circuit breaker for fault tolerance
    circuit_breaker: Option<Arc<Mutex<CircuitBreaker>>>,
    /// Retry policy for transient API failures
    retry_policy: RetryPolicy,
}

impl ApiClient {
    /// Creates a new API client with the specified base URL.
    /// Configures the client with a 30-second timeout.
    /// Does not use a circuit breaker.
    ///
    /// # Arguments
    /// * `base_url` - Base URL for all API requests (e.g., "<https://api.example.com>")
    ///
    /// # Returns
    /// A new ApiClient instance ready for making requests
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url,
            circuit_breaker: None,
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Creates a new API client with circuit breaker protection.
    ///
    /// # Arguments
    /// * `base_url` - Base URL for all API requests
    /// * `circuit_breaker` - Circuit breaker instance for fault tolerance
    ///
    /// # Returns
    /// A new ApiClient instance with circuit breaker enabled
    pub fn with_circuit_breaker(base_url: String, circuit_breaker: CircuitBreaker) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url,
            circuit_breaker: Some(Arc::new(Mutex::new(circuit_breaker))),
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Creates a new API client with a custom retry policy.
    #[allow(dead_code)]
    pub fn with_retry_policy(base_url: String, retry_policy: RetryPolicy) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url,
            circuit_breaker: None,
            retry_policy,
        }
    }

    /// Creates a new API client with both circuit breaker and custom retry policy.
    #[allow(dead_code)]
    pub fn with_circuit_breaker_and_retry_policy(
        base_url: String,
        circuit_breaker: CircuitBreaker,
        retry_policy: RetryPolicy,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            client,
            base_url,
            circuit_breaker: Some(Arc::new(Mutex::new(circuit_breaker))),
            retry_policy,
        }
    }

    /// Check if the circuit breaker allows the request
    fn can_execute(&self) -> bool {
        match &self.circuit_breaker {
            Some(cb) => cb.lock().can_execute(),
            None => true,
        }
    }

    /// Performs a GET request and deserializes the JSON response.
    /// Appends the path to the base URL and handles HTTP errors.
    /// Checks circuit breaker before executing and records success/failure.
    ///
    /// # Type Parameters
    /// * `T` - Type to deserialize the response into (must implement DeserializeOwned)
    ///
    /// # Arguments
    /// * `path` - API endpoint path (will be appended to base_url)
    ///
    /// # Returns
    /// Deserialized response data or an error
    #[allow(dead_code)]
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request_json(Method::GET, path, None).await
    }

    /// Performs a GET request with API key authentication.
    /// Includes the X-API-Key header in the request for authenticated endpoints.
    /// Checks circuit breaker before executing and records success/failure.
    ///
    /// # Type Parameters
    /// * `T` - Type to deserialize the response into
    ///
    /// # Arguments
    /// * `path` - API endpoint path
    /// * `api_key` - API key for authentication
    ///
    /// # Returns
    /// Deserialized response data or an error
    pub async fn get_with_key<T: DeserializeOwned>(&self, path: &str, api_key: &str) -> Result<T> {
        self.request_json(Method::GET, path, Some(api_key.to_string())).await
    }

    async fn request_json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        api_key: Option<String>,
    ) -> Result<T> {
        if !self.can_execute() {
            bail!("Circuit breaker is open - request blocked");
        }

        let url = format!("{}{}", self.base_url, path);
        let client = self.client.clone();
        let circuit_breaker = self.circuit_breaker.clone();
        let retry_policy = self.retry_policy.clone();
        let method_for_attempt = method;

        let result = retry_policy
            .execute(
                move || {
                    let client = client.clone();
                    let url = url.clone();
                    let api_key = api_key.clone();
                    let circuit_breaker = circuit_breaker.clone();
                    let method = method_for_attempt.clone();

                    async move {
                        if let Some(cb) = &circuit_breaker {
                            if !cb.lock().can_execute() {
                                return Err(ApiCallError::permanent(
                                    "Circuit breaker is open - request blocked",
                                ));
                            }
                        }

                        let mut request = client.request(method, &url);
                        if let Some(key) = api_key {
                            request = request.header("X-API-Key", key);
                        }

                        let response = request.send().await.map_err(|e| {
                            ApiCallError::retryable(format!("API request failed: {e}"))
                        })?;

                        let status = response.status();
                        if !status.is_success() {
                            let text = response.text().await.unwrap_or_default();
                            let message = format!("API request failed with status {status}: {text}");
                            let error = if is_retryable_status(status) {
                                ApiCallError::retryable(message)
                            } else {
                                ApiCallError::permanent(message)
                            };
                            if let Some(cb) = &circuit_breaker {
                                cb.lock().record_failure();
                            }
                            return Err(error);
                        }

                        let parsed = response.json().await.map_err(|e| {
                            ApiCallError::retryable(format!("Failed to parse JSON response: {e}"))
                        })?;

                        if let Some(cb) = &circuit_breaker {
                            cb.lock().record_success();
                        }

                        Ok(parsed)
                    }
                },
                |err: &ApiCallError| err.retryable,
            )
            .await;

        result.map_err(|err| anyhow!(err))
    }
}

#[derive(Debug)]
struct ApiCallError {
    message: String,
    retryable: bool,
}

impl ApiCallError {
    fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
        }
    }

    fn permanent(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
        }
    }
}

impl std::fmt::Display for ApiCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ApiCallError {}

fn is_retryable_status(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::REQUEST_TIMEOUT || status == StatusCode::TOO_MANY_REQUESTS
}

/// States of a circuit breaker for fault tolerance.
/// Circuit breakers prevent cascading failures by temporarily stopping
/// requests to services that are failing repeatedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum CircuitState {
    /// Circuit is closed - requests are allowed through normally
    Closed,
    /// Circuit is open - requests are blocked to prevent further failures
    Open,
    /// Circuit is testing if the service has recovered - limited requests allowed
    HalfOpen,
}

/// Implements circuit breaker pattern for fault-tolerant API calls.
/// Monitors request success/failure rates and temporarily blocks requests
/// to failing services to prevent cascading failures.
#[allow(dead_code)]
pub struct CircuitBreaker {
    /// Number of consecutive failures before opening the circuit
    failure_threshold: u32,
    /// Number of consecutive successes needed to close the circuit
    success_threshold: u32,
    /// Time to wait before trying to close the circuit again (milliseconds)
    half_open_timeout_ms: u64,
    /// Current count of consecutive failures
    failures: u32,
    /// Current count of consecutive successes (in half-open state)
    successes: u32,
    /// Current state of the circuit breaker
    state: CircuitState,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker with the specified thresholds.
    /// Starts in the Closed state, allowing requests through normally.
    ///
    /// # Arguments
    /// * `failure_threshold` - Consecutive failures before opening circuit
    /// * `success_threshold` - Consecutive successes to close circuit
    /// * `half_open_timeout_ms` - Timeout before retrying in half-open state
    ///
    /// # Returns
    /// A new CircuitBreaker instance in Closed state
    #[allow(dead_code)]
    pub const fn new(failure_threshold: u32, success_threshold: u32, half_open_timeout_ms: u64) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            half_open_timeout_ms,
            failures: 0,
            successes: 0,
            state: CircuitState::Closed,
        }
    }

    /// Check if a request can be executed based on circuit state
    pub fn can_execute(&self) -> bool {
        match self.state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true, // Allow limited requests in half-open state
        }
    }

    #[allow(dead_code)]
    pub fn is_closed(&self) -> bool {
        self.state == CircuitState::Closed
    }

    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        self.state == CircuitState::Open
    }

    #[allow(dead_code)]
    pub fn state(&self) -> CircuitState {
        self.state
    }

    #[allow(dead_code)]
    pub fn record_success(&mut self) {
        self.successes += 1;

        if self.state == CircuitState::HalfOpen && self.successes >= self.success_threshold {
            self.state = CircuitState::Closed;
            self.failures = 0;
            self.successes = 0;
            info!("Circuit breaker closed after {} consecutive successes", self.success_threshold);
        }
    }

    #[allow(dead_code)]
    pub fn record_failure(&mut self) {
        self.failures += 1;

        if self.state == CircuitState::HalfOpen || self.failures >= self.failure_threshold {
            self.state = CircuitState::Open;
            self.failures = 0;
            self.successes = 0;
            warn!("Circuit breaker opened after {} consecutive failures", self.failure_threshold);
        }
    }

    /// Reset the circuit breaker to closed state
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.failures = 0;
        self.successes = 0;
        self.state = CircuitState::Closed;
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, 3, 30000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_default_values() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
        assert_eq!(policy.initial_delay, Duration::from_millis(200));
        assert_eq!(policy.max_delay, Duration::from_secs(10));
        assert!((policy.factor - 2.0).abs() < f64::EPSILON);
        assert!((policy.jitter - 0.3).abs() < f64::EPSILON);
    }

    #[test]
    fn test_delay_for_attempt_zero_returns_zero() {
        let policy = RetryPolicy::default();
        let delay = policy.delay_for_attempt(0);
        assert_eq!(delay, Duration::ZERO);
    }

    #[test]
    fn test_delay_for_attempt_increases_exponentially() {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            factor: 2.0,
            jitter: 0.0, // disable jitter for deterministic test
        };

        let d1 = policy.delay_for_attempt(1);
        let d2 = policy.delay_for_attempt(2);
        let d3 = policy.delay_for_attempt(3);

        assert_eq!(d1, Duration::from_millis(200));
        assert_eq!(d2, Duration::from_millis(400));
        assert_eq!(d3, Duration::from_millis(800));
    }

    #[test]
    fn test_delay_capped_by_max() {
        let policy = RetryPolicy {
            max_retries: 10,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(1000), // 1 second max
            factor: 2.0,
            jitter: 0.0,
        };

        // Attempt 5: 100 * 2^5 = 3,200ms -> capped to 1000ms
        let delay = policy.delay_for_attempt(5);
        assert_eq!(delay, Duration::from_millis(1000));
    }

    #[test]
    fn test_circuit_breaker_default_values() {
        let cb = CircuitBreaker::default();
        assert_eq!(cb.failure_threshold, 5);
        assert_eq!(cb.success_threshold, 3);
        assert_eq!(cb.half_open_timeout_ms, 30000);
        assert!(cb.is_closed());
    }

    #[test]
    fn test_circuit_breaker_opens_on_failures() {
        let mut cb = CircuitBreaker::new(3, 2, 30000);
        
        assert!(cb.can_execute());
        assert!(cb.is_closed());
        
        cb.record_failure();
        assert!(cb.can_execute());
        assert!(cb.is_closed());
        
        cb.record_failure();
        assert!(cb.can_execute());
        assert!(cb.is_closed());
        
        cb.record_failure();
        assert!(!cb.can_execute());
        assert!(cb.is_open());
    }

    #[test]
    fn test_circuit_breaker_closes_on_successes() {
        let mut cb = CircuitBreaker::new(2, 2, 30000);
        
        // Open the circuit
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());
        assert!(!cb.can_execute());
        
        // Manually set to half-open (in real usage, this would happen after timeout)
        cb.state = CircuitState::HalfOpen;
        
        // Record successes to close
        cb.record_success();
        assert!(!cb.is_closed()); // Still half-open
        
        cb.record_success();
        assert!(cb.is_closed());
        assert!(cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new(2, 2, 30000);
        
        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_open());
        
        cb.reset();
        assert!(cb.is_closed());
        assert!(cb.can_execute());
        assert_eq!(cb.failure_threshold, 2); // Settings preserved
    }

    #[test]
    fn test_retryable_status_classification() {
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!is_retryable_status(StatusCode::BAD_REQUEST));
        assert!(!is_retryable_status(StatusCode::UNAUTHORIZED));
    }
}

/// Configuration for exponential backoff retry policy.
/// Controls how failed requests are retried with increasing delays.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts (0 = no retries)
    pub max_retries: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Exponential multiplier (2.0 = double each retry)
    pub factor: f64,
    /// Jitter factor (0.0–1.0) to randomize delays
    pub jitter: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(10),
            factor: 2.0,
            jitter: 0.3, // 30% jitter
        }
    }
}

impl RetryPolicy {
    /// Calculates the sleep duration for a given retry attempt with jitter.
    ///
    /// Uses "full jitter" strategy: delay = random(0, min(initial * factor^attempt, max))
    /// If `jitter` is 0.0, returns the exact capped exponential delay (deterministic).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }
        let exponential = self.initial_delay.as_millis() as f64
            * self.factor.powi(attempt as i32);
        let capped = exponential.min(self.max_delay.as_millis() as f64);

        let delay_ms = if self.jitter <= 0.0 {
            capped
        } else {
            rand::thread_rng().gen_range(0.0..=capped)
        };
        Duration::from_millis(delay_ms as u64)
    }

    /// Executes an async operation with exponential backoff retry.
    ///
    /// # Arguments
    /// * `operation` - Async closure that returns Result<T, E>
    /// * `is_retryable` - Predicate to determine if error should be retried
    ///
    /// # Returns
    /// Ok(T) if any attempt succeeds, or Err(E) with last error if all retries exhausted
    pub async fn execute<F, T, E>(
        &self,
        mut operation: impl FnMut() -> F,
        is_retryable: impl Fn(&E) -> bool,
    ) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match operation().await {
                Ok(value) => return Ok(value),
                Err(e) => {
                    last_error = Some(e);
                    if attempt >= self.max_retries {
                        break;
                    }
                    if !is_retryable(last_error.as_ref().unwrap()) {
                        break;
                    }
                    let delay = self.delay_for_attempt(attempt + 1);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(last_error.unwrap())
    }
}
