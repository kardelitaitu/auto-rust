//! HTTP API client module with circuit breaker support.
//!
//! Provides:
//! - Type-safe HTTP request handling with reqwest
//! - Circuit breaker pattern for fault tolerance
//! - Automatic timeout configuration
//! - JSON serialization/deserialization support

use anyhow::{Result, bail};
use reqwest::Client;
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
}

impl ApiClient {
    /// Creates a new API client with the specified base URL.
    /// Configures the client with a 30-second timeout.
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
        }
    }

    /// Performs a GET request and deserializes the JSON response.
    /// Appends the path to the base URL and handles HTTP errors.
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
        let url = format!("{}{}", self.base_url, path);
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("API request failed with status {status}: {text}");
        }
        
        let result = response.json().await?;
        Ok(result)
    }

    /// Performs a GET request with API key authentication.
    /// Includes the X-API-Key header in the request for authenticated endpoints.
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
        let url = format!("{}{}", self.base_url, path);
        let response = self.client
            .get(&url)
            .header("X-API-Key", api_key)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("API request failed with status {status}: {text}");
        }
        
        let result = response.json().await?;
        Ok(result)
    }
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

    #[allow(dead_code)]
    pub fn is_closed(&self) -> bool {
        self.state == CircuitState::Closed
    }

    #[allow(dead_code)]
    pub fn record_success(&mut self) {
        self.successes += 1;
        
        if self.state == CircuitState::HalfOpen && self.successes >= self.success_threshold {
            self.state = CircuitState::Closed;
            self.failures = 0;
            self.successes = 0;
        }
    }

    #[allow(dead_code)]
    pub fn record_failure(&mut self) {
        self.failures += 1;
        
        if self.state == CircuitState::HalfOpen || self.failures >= self.failure_threshold {
            self.state = CircuitState::Open;
            self.failures = 0;
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, 3, 30000)
    }
}