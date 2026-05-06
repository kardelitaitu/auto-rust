//! Session factory for constructing sessions from browser capabilities.
//!
//! The factory pattern separates session construction from discovery,
//! making it easier to test and extend session creation logic.

use crate::config::Config;
use crate::error::{BrowserError, OrchestratorError, Result};
use crate::session::connector::BrowserCapabilities;
use crate::session::Session;
use log::{debug, info};
use std::time::Duration;

/// Factory for creating browser sessions from discovered capabilities.
///
/// The factory handles the connection and session construction process,
/// attaching browser capability data to the resulting session.
pub struct SessionFactory {
    connection_timeout_ms: u64,
    max_workers: usize,
    cursor_overlay_ms: u64,
    circuit_breaker_config: crate::config::CircuitBreakerConfig,
}

impl SessionFactory {
    /// Creates a new session factory from configuration.
    ///
    /// # Arguments
    /// * `config` - The orchestrator configuration
    pub fn from_config(config: &Config) -> Self {
        Self {
            connection_timeout_ms: config.browser.connection_timeout_ms.max(5000),
            max_workers: config.browser.max_workers_per_session,
            cursor_overlay_ms: config.browser.cursor_overlay_ms,
            circuit_breaker_config: config.browser.circuit_breaker.clone(),
        }
    }

    /// Creates a factory with explicit settings.
    ///
    /// # Arguments
    /// * `connection_timeout_ms` - Connection timeout in milliseconds
    /// * `max_workers` - Maximum concurrent workers per session
    /// * `cursor_overlay_ms` - Cursor overlay sync interval (0 = disabled)
    /// * `circuit_breaker_config` - Circuit breaker configuration
    pub fn new(
        connection_timeout_ms: u64,
        max_workers: usize,
        cursor_overlay_ms: u64,
        circuit_breaker_config: crate::config::CircuitBreakerConfig,
    ) -> Self {
        Self {
            connection_timeout_ms: connection_timeout_ms.max(5000),
            max_workers,
            cursor_overlay_ms,
            circuit_breaker_config,
        }
    }

    /// Creates a session from browser capabilities.
    ///
    /// Establishes a WebSocket connection to the browser and constructs
    /// a fully initialized Session with capability metadata attached.
    ///
    /// # Arguments
    /// * `capability` - The browser capability to connect to
    ///
    /// # Returns
    /// A connected Session instance
    ///
    /// # Errors
    /// Returns an error if connection fails or times out
    pub async fn create_session(&self, capability: &BrowserCapabilities) -> Result<Session> {
        if capability.ws_url.is_empty() {
            return Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!("Empty WebSocket endpoint for: {}", capability.name),
            )));
        }

        debug!(
            "Connecting to {} (source: {}): {}",
            capability.name, capability.source, capability.ws_url
        );

        let connect_timeout = Duration::from_millis(self.connection_timeout_ms);

        match tokio::time::timeout(
            connect_timeout,
            chromiumoxide::Browser::connect(&capability.ws_url),
        )
        .await
        {
            Ok(Ok((browser, handler))) => {
                info!(
                    "Connected to {} (id: {}, type: {})",
                    capability.name, capability.id, capability.browser_type
                );

                Ok(Session::new(
                    capability.id.clone(),
                    capability.name.clone(),
                    capability.browser_type.clone(),
                    browser,
                    handler,
                    self.max_workers,
                    self.cursor_overlay_ms,
                    Some(self.circuit_breaker_config.clone()),
                ))
            }
            Ok(Err(e)) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Failed to connect to {} ({}): {}",
                    capability.name, capability.source, e
                ),
            ))),
            Err(_) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Connection timeout to {} ({}) after {}ms",
                    capability.name, capability.source, self.connection_timeout_ms
                ),
            ))),
        }
    }

    /// Creates sessions from multiple capabilities in parallel.
    ///
    /// # Arguments
    /// * `capabilities` - List of browser capabilities to connect to
    ///
    /// # Returns
    /// A vector of successfully created sessions
    pub async fn create_sessions_parallel(
        &self,
        capabilities: &[BrowserCapabilities],
    ) -> Vec<Session> {
        use futures::stream::{self, StreamExt};

        let results: Vec<Option<Session>> = stream::iter(capabilities)
            .map(|cap| async move { self.create_session(cap).await.ok() })
            .buffer_unordered(10)
            .collect()
            .await;

        results.into_iter().flatten().collect()
    }

    /// Returns the connection timeout in milliseconds.
    pub fn connection_timeout_ms(&self) -> u64 {
        self.connection_timeout_ms
    }

    /// Returns the maximum workers per session.
    pub fn max_workers(&self) -> usize {
        self.max_workers
    }

    /// Returns the cursor overlay interval in milliseconds.
    pub fn cursor_overlay_ms(&self) -> u64 {
        self.cursor_overlay_ms
    }
}

impl Default for SessionFactory {
    fn default() -> Self {
        Self {
            connection_timeout_ms: 30000,
            max_workers: 3,
            cursor_overlay_ms: 0,
            circuit_breaker_config: crate::config::CircuitBreakerConfig::default(),
        }
    }
}

/// Builder for constructing session factories with custom settings.
#[derive(Debug)]
pub struct SessionFactoryBuilder {
    connection_timeout_ms: u64,
    max_workers: usize,
    cursor_overlay_ms: u64,
    circuit_breaker_config: crate::config::CircuitBreakerConfig,
}

impl SessionFactoryBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the connection timeout.
    pub fn connection_timeout_ms(mut self, ms: u64) -> Self {
        self.connection_timeout_ms = ms.max(5000);
        self
    }

    /// Sets the maximum workers per session.
    pub fn max_workers(mut self, workers: usize) -> Self {
        self.max_workers = workers;
        self
    }

    /// Sets the cursor overlay sync interval.
    pub fn cursor_overlay_ms(mut self, ms: u64) -> Self {
        self.cursor_overlay_ms = ms;
        self
    }

    /// Sets the circuit breaker configuration.
    pub fn circuit_breaker_config(mut self, config: crate::config::CircuitBreakerConfig) -> Self {
        self.circuit_breaker_config = config;
        self
    }

    /// Builds the session factory.
    pub fn build(self) -> SessionFactory {
        SessionFactory::new(
            self.connection_timeout_ms,
            self.max_workers,
            self.cursor_overlay_ms,
            self.circuit_breaker_config,
        )
    }
}

impl Default for SessionFactoryBuilder {
    fn default() -> Self {
        Self {
            connection_timeout_ms: 30000,
            max_workers: 3,
            cursor_overlay_ms: 0,
            circuit_breaker_config: crate::config::CircuitBreakerConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_factory_default() {
        let factory = SessionFactory::default();
        assert_eq!(factory.connection_timeout_ms(), 30000);
        assert_eq!(factory.max_workers(), 3);
        assert_eq!(factory.cursor_overlay_ms(), 0);
    }

    #[test]
    fn test_session_factory_from_config() {
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                connection_timeout_ms: 45000,
                max_workers_per_session: 5,
                cursor_overlay_ms: 100,
                circuit_breaker: crate::config::CircuitBreakerConfig {
                    enabled: true,
                    failure_threshold: 10,
                    success_threshold: 3,
                    half_open_time_ms: 60000,
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let factory = SessionFactory::from_config(&config);
        assert_eq!(factory.connection_timeout_ms(), 45000);
        assert_eq!(factory.max_workers(), 5);
        assert_eq!(factory.cursor_overlay_ms(), 100);
    }

    #[test]
    fn test_session_factory_new() {
        let factory =
            SessionFactory::new(20000, 4, 50, crate::config::CircuitBreakerConfig::default());
        assert_eq!(factory.connection_timeout_ms(), 20000);
        assert_eq!(factory.max_workers(), 4);
        assert_eq!(factory.cursor_overlay_ms(), 50);
    }

    #[test]
    fn test_session_factory_min_timeout() {
        // Timeout should be clamped to minimum of 5000ms
        let factory =
            SessionFactory::new(1000, 3, 0, crate::config::CircuitBreakerConfig::default());
        assert_eq!(factory.connection_timeout_ms(), 5000);
    }

    #[test]
    fn test_factory_builder_default() {
        let builder = SessionFactoryBuilder::new();
        let factory = builder.build();
        assert_eq!(factory.connection_timeout_ms(), 30000);
        assert_eq!(factory.max_workers(), 3);
    }

    #[test]
    fn test_factory_builder_chaining() {
        let factory = SessionFactoryBuilder::new()
            .connection_timeout_ms(60000)
            .max_workers(10)
            .cursor_overlay_ms(200)
            .build();

        assert_eq!(factory.connection_timeout_ms(), 60000);
        assert_eq!(factory.max_workers(), 10);
        assert_eq!(factory.cursor_overlay_ms(), 200);
    }

    #[test]
    fn test_factory_builder_timeout_clamping() {
        let factory = SessionFactoryBuilder::new()
            .connection_timeout_ms(100) // Below minimum
            .build();

        assert_eq!(factory.connection_timeout_ms(), 5000);
    }
}
