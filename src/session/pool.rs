//! Session pool manager for parallel discovery and retry coordination.
//!
//! Manages the discovery and connection lifecycle across multiple
//! browser sources with retry policies and parallel execution.

use crate::config::Config;
use crate::error::{BrowserError, OrchestratorError, Result};
use crate::session::connector::{BrowserCapabilities, ConnectorRegistry};
use crate::session::factory::SessionFactory;
use crate::session::Session;
use log::{debug, info, warn};

/// Manages a pool of browser sessions with discovery and retry logic.
///
/// The pool manager coordinates discovery across multiple connectors,
/// handles retry logic, and maintains the active session pool.
pub struct SessionPoolManager {
    registry: ConnectorRegistry,
    factory: SessionFactory,
    max_retries: u32,
}

impl SessionPoolManager {
    /// Creates a new session pool manager from configuration.
    ///
    /// # Arguments
    /// * `config` - The orchestrator configuration
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        Self {
            registry: ConnectorRegistry::standard(),
            factory: SessionFactory::from_config(config),
            max_retries: config.browser.max_discovery_retries,
        }
    }

    /// Creates a pool manager with custom components.
    ///
    /// # Arguments
    /// * `registry` - The connector registry to use
    /// * `factory` - The session factory to use
    /// * `max_retries` - Maximum discovery retry attempts
    #[must_use]
    pub fn new(registry: ConnectorRegistry, factory: SessionFactory, max_retries: u32) -> Self {
        Self {
            registry,
            factory,
            max_retries,
        }
    }

    /// Discovers available browsers across all connectors.
    ///
    /// Queries all available connectors for browser capabilities
    /// without establishing connections.
    ///
    /// # Arguments
    /// * `config` - The orchestrator configuration
    ///
    /// # Returns
    /// A list of discovered browser capabilities from all sources
    pub async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>> {
        let mut all_capabilities = Vec::new();

        for connector in self.registry.available(config) {
            match connector.discover(config).await {
                Ok(caps) => {
                    debug!("Connector discovered {} browser(s)", caps.len());
                    all_capabilities.extend(caps);
                }
                Err(e) => {
                    warn!("Connector discovery failed: {e}");
                }
            }
        }

        info!(
            "Total discovered browsers: {} (from {} available connectors)",
            all_capabilities.len(),
            self.registry.available(config).len()
        );

        Ok(all_capabilities)
    }

    /// Connects to discovered browsers and creates sessions.
    ///
    /// Attempts to connect to all discovered capabilities in parallel
    /// and returns successfully created sessions.
    ///
    /// # Arguments
    /// * `capabilities` - List of browser capabilities to connect to
    ///
    /// # Returns
    /// A vector of successfully created sessions
    pub async fn connect_all(&self, capabilities: &[BrowserCapabilities]) -> Vec<Session> {
        self.factory.create_sessions_parallel(capabilities).await
    }

    /// Discovers and connects to browsers with retry logic.
    ///
    /// Attempts discovery and connection up to `max_retries` times,
    /// returning as soon as at least one session is established.
    ///
    /// # Arguments
    /// * `config` - The orchestrator configuration
    ///
    /// # Returns
    /// A vector of successfully created sessions
    ///
    /// # Errors
    /// Returns an error if no sessions can be established after all retries
    pub async fn discover_and_connect(&self, config: &Config) -> Result<Vec<Session>> {
        for attempt in 1..=self.max_retries {
            debug!("Discovery attempt {}/{}", attempt, self.max_retries);

            match self.discover(config).await {
                Ok(caps) if !caps.is_empty() => {
                    let sessions = self.connect_all(&caps).await;
                    if !sessions.is_empty() {
                        info!(
                            "Established {} session(s) on attempt {}",
                            sessions.len(),
                            attempt
                        );
                        return Ok(sessions);
                    }
                    warn!("No sessions established on attempt {attempt}");
                }
                Ok(_) => {
                    debug!("No browsers discovered on attempt {attempt}");
                }
                Err(e) => {
                    warn!("Discovery failed on attempt {attempt}: {e}");
                }
            }

            if attempt < self.max_retries {
                let delay = std::time::Duration::from_millis(1000 * u64::from(attempt));
                debug!("Retrying after {delay:?}...");
                tokio::time::sleep(delay).await;
            }
        }

        Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
            format!(
                "No browsers discovered after {} retry attempts",
                self.max_retries
            ),
        )))
    }

    /// Discovers browsers with optional filtering.
    ///
    /// Filters discovered capabilities by browser name/type before
    /// establishing connections.
    ///
    /// # Arguments
    /// * `config` - The orchestrator configuration
    /// * `filters` - Optional list of browser name/type filters
    ///
    /// # Returns
    /// A vector of sessions matching the filters
    ///
    /// # Errors
    /// Returns an error if no matching browsers are found
    pub async fn discover_with_filters(
        &self,
        config: &Config,
        filters: &[String],
    ) -> Result<Vec<Session>> {
        if !filters.is_empty() {
            info!("Browser filters active: {}", filters.join(", "));
        }

        for attempt in 1..=self.max_retries {
            let caps = self.discover(config).await?;

            // Filter capabilities
            let filtered_caps: Vec<_> = caps
                .into_iter()
                .filter(|cap| self.capability_matches_filters(cap, filters))
                .collect();

            if !filtered_caps.is_empty() {
                let sessions = self.connect_all(&filtered_caps).await;
                if !sessions.is_empty() {
                    let names: Vec<_> = sessions.iter().map(|s| s.name.as_str()).collect();
                    info!(
                        "Discovered {} browser(s) on attempt {}: {}",
                        sessions.len(),
                        attempt,
                        names.join(", ")
                    );
                    return Ok(sessions);
                }
            }

            if attempt < self.max_retries {
                let delay = std::time::Duration::from_millis(1000 * u64::from(attempt));
                tokio::time::sleep(delay).await;
            }
        }

        if !filters.is_empty() {
            return Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "No browsers matched the specified filters: {}. Please check your --browsers argument.",
                    filters.join(", ")
                ),
            )));
        }

        warn!("No browsers discovered (no filters specified)");
        Ok(vec![])
    }

    /// Checks if a capability matches the given filters.
    fn capability_matches_filters(
        &self,
        capability: &BrowserCapabilities,
        filters: &[String],
    ) -> bool {
        if filters.is_empty() {
            return true;
        }

        let candidate = format!(
            "{} {} {}",
            capability.name, capability.browser_type, capability.id
        )
        .to_lowercase();

        filters.iter().any(|filter| {
            let filter_lower = filter.to_lowercase();
            let filter_norm = normalize_browser_token(filter);

            !filter_norm.is_empty()
                && (candidate.contains(&filter_lower)
                    || normalize_browser_token(&candidate).contains(&filter_norm))
        })
    }

    /// Returns the number of available connectors.
    #[must_use]
    pub fn connector_count(&self, config: &Config) -> usize {
        self.registry.available(config).len()
    }

    /// Returns the maximum retry count.
    #[must_use]
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }
}

impl Default for SessionPoolManager {
    fn default() -> Self {
        Self {
            registry: ConnectorRegistry::standard(),
            factory: SessionFactory::default(),
            max_retries: 3,
        }
    }
}

use crate::utils::normalize_browser_token;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::connector::{BrowserCapabilities, BrowserSource, ConnectorRegistry};
    use crate::session::factory::SessionFactory;

    #[test]
    fn test_session_pool_manager_default() {
        let manager = SessionPoolManager::default();
        assert_eq!(manager.max_retries(), 3);
    }

    #[test]
    fn test_session_pool_manager_from_config() {
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                max_discovery_retries: 5,
                ..Default::default()
            },
            ..Default::default()
        };

        let manager = SessionPoolManager::from_config(&config);
        assert_eq!(manager.max_retries(), 5);
    }

    #[test]
    fn test_capability_matches_filters_empty() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "test".to_string(),
            name: "Test Browser".to_string(),
            browser_type: "brave".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Configured,
        };

        // Empty filters should match everything
        assert!(manager.capability_matches_filters(&cap, &[]));
    }

    #[test]
    fn test_capability_matches_filters_by_name() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "test".to_string(),
            name: "My Brave Browser".to_string(),
            browser_type: "chrome".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Configured,
        };

        assert!(manager.capability_matches_filters(&cap, &["brave".to_string()]));
        assert!(manager.capability_matches_filters(&cap, &["Brave".to_string()]));
    }

    #[test]
    fn test_capability_matches_filters_by_type() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "test".to_string(),
            name: "Custom Name".to_string(),
            browser_type: "roxybrowser".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::RoxyBrowser,
        };

        assert!(manager.capability_matches_filters(&cap, &["roxybrowser".to_string()]));
        assert!(manager.capability_matches_filters(&cap, &["roxy".to_string()]));
    }

    #[test]
    fn test_capability_matches_filters_by_id() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "brave-123".to_string(),
            name: "Test".to_string(),
            browser_type: "chrome".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Local,
        };

        assert!(manager.capability_matches_filters(&cap, &["brave".to_string()]));
    }

    #[test]
    fn test_capability_no_match() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "chrome-1".to_string(),
            name: "Chrome".to_string(),
            browser_type: "localChrome".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Local,
        };

        assert!(!manager.capability_matches_filters(&cap, &["brave".to_string()]));
        assert!(!manager.capability_matches_filters(&cap, &["firefox".to_string()]));
    }

    #[test]
    fn test_capability_matches_multiple_filters() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "test".to_string(),
            name: "Brave Browser".to_string(),
            browser_type: "chrome".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Configured,
        };

        // Should match if any filter matches
        assert!(
            manager.capability_matches_filters(&cap, &["brave".to_string(), "firefox".to_string()])
        );
    }

    #[test]
    fn test_normalize_browser_token() {
        assert_eq!(normalize_browser_token("Brave-Browser"), "bravebrowser");
        assert_eq!(normalize_browser_token("Chrome_123"), "chrome123");
        assert_eq!(normalize_browser_token("Test@#$Browser"), "testbrowser");
        assert_eq!(normalize_browser_token(""), "");
        assert_eq!(
            normalize_browser_token("MixedCase_Example"),
            "mixedcaseexample"
        );
        assert_eq!(normalize_browser_token("123Numbers456"), "123numbers456");
        assert_eq!(
            normalize_browser_token("Only-Special!@#$%^&*()"),
            "onlyspecial"
        );
        assert_eq!(normalize_browser_token("a"), "a");
        assert_eq!(normalize_browser_token("A_B-C"), "abc");
    }

    #[test]
    fn test_connector_count() {
        let manager = SessionPoolManager::default();
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![],
                ..Default::default()
            },
            ..Default::default()
        };

        // Should have 0 available connectors with empty config
        assert_eq!(manager.connector_count(&config), 0);
    }

    #[test]
    fn test_capability_matches_normalized_filter() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "test".to_string(),
            name: "Brave_Browser".to_string(),
            browser_type: "chrome".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Configured,
        };

        // Should match even with different separators
        assert!(manager.capability_matches_filters(&cap, &["bravebrowser".to_string()]));
    }

    #[tokio::test]
    async fn test_discover_with_filters_empty_discovery_no_filters() {
        let manager =
            SessionPoolManager::new(ConnectorRegistry::empty(), SessionFactory::default(), 1);
        let config = crate::config::Config::default();

        let result = manager.discover_with_filters(&config, &[]).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_discover_with_filters_empty_discovery_with_filters() {
        let manager =
            SessionPoolManager::new(ConnectorRegistry::empty(), SessionFactory::default(), 1);
        let config = crate::config::Config::default();
        let filters = vec!["brave".to_string()];

        let result = manager.discover_with_filters(&config, &filters).await;

        match result {
            Ok(_) => panic!("should return an error when filters match nothing"),
            Err(err) => {
                let err_msg = err.to_string();
                assert!(err_msg.contains("No browsers matched the specified filters"));
                assert!(err_msg.contains("brave"));
            }
        }
    }

    // ========================================================================
    // TDD Tests — Coverage Expansion
    // ========================================================================

    #[tokio::test]
    async fn tdd_green_discover_empty_registry_returns_empty() {
        let manager =
            SessionPoolManager::new(ConnectorRegistry::empty(), SessionFactory::default(), 3);
        let config = crate::config::Config::default();
        let result = manager.discover(&config).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn tdd_green_connector_count_with_profiles() {
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![
                    crate::config::BrowserProfile {
                        name: "alpha".to_string(),
                        r#type: "brave".to_string(),
                        ws_endpoint: "ws://a:9222".to_string(),
                    },
                    crate::config::BrowserProfile {
                        name: "beta".to_string(),
                        r#type: "chrome".to_string(),
                        ws_endpoint: "ws://b:9222".to_string(),
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        // ConfiguredProfileConnector is available when profiles is non-empty
        let manager = SessionPoolManager::default();
        assert_eq!(manager.connector_count(&config), 1);
    }

    #[test]
    fn tdd_green_max_retries_constructors() {
        // Test max_retries from different constructors
        let manager_default = SessionPoolManager::default();
        assert_eq!(manager_default.max_retries(), 3);

        let manager_zero =
            SessionPoolManager::new(ConnectorRegistry::empty(), SessionFactory::default(), 0);
        assert_eq!(manager_zero.max_retries(), 0);

        let manager_five =
            SessionPoolManager::new(ConnectorRegistry::empty(), SessionFactory::default(), 5);
        assert_eq!(manager_five.max_retries(), 5);

        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                max_discovery_retries: 10,
                ..Default::default()
            },
            ..Default::default()
        };
        let manager_from_config = SessionPoolManager::from_config(&config);
        assert_eq!(manager_from_config.max_retries(), 10);
    }

    #[tokio::test]
    async fn tdd_red_discover_and_connect_zero_retries_fails_immediately() {
        let manager =
            SessionPoolManager::new(ConnectorRegistry::empty(), SessionFactory::default(), 0);
        let config = crate::config::Config::default();
        let result = manager.discover_and_connect(&config).await;
        match result {
            Err(err) => {
                let err_msg = err.to_string();
                assert!(
                    err_msg.contains("No browsers discovered"),
                    "error: {}",
                    err_msg
                );
            }
            Ok(sessions) => panic!(
                "expected error with zero retries, got {} sessions",
                sessions.len()
            ),
        }
    }

    #[tokio::test]
    async fn tdd_red_discover_and_connect_single_retry_fails_on_empty() {
        let manager =
            SessionPoolManager::new(ConnectorRegistry::empty(), SessionFactory::default(), 1);
        let config = crate::config::Config::default();
        let result = manager.discover_and_connect(&config).await;
        match result {
            Err(err) => {
                let err_msg = err.to_string();
                assert!(
                    err_msg.contains("No browsers discovered"),
                    "error: {}",
                    err_msg
                );
            }
            Ok(sessions) => panic!(
                "expected error with empty registry, got {} sessions",
                sessions.len()
            ),
        }
    }

    #[test]
    fn tdd_green_capability_matches_partial_across_fields() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "node-42".to_string(),
            name: "Firefox Dev".to_string(),
            browser_type: "firefox".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Local,
        };
        // The filter "node42" matches the normalized concatenation of
        // name+type+id: "firefoxdevfirefoxnode42" after normalization
        assert!(manager.capability_matches_filters(&cap, &["node42".to_string()]));
        // "firefox" matches the browser_type directly
        assert!(manager.capability_matches_filters(&cap, &["firefox".to_string()]));
    }

    #[test]
    fn tdd_green_normalize_unicode_non_ascii() {
        assert_eq!(normalize_browser_token("Bräve"), "brve");
        assert_eq!(normalize_browser_token("Chromé_123"), "chrom123");
        assert_eq!(normalize_browser_token("中文测试"), "");
        assert_eq!(normalize_browser_token("🦀 Rust"), "rust");
        assert_eq!(normalize_browser_token("café_browser"), "cafbrowser");
        assert_eq!(normalize_browser_token("ñ"), "");
    }

    #[test]
    fn tdd_green_capability_matches_special_chars_in_ids() {
        let manager = SessionPoolManager::default();
        let cap = BrowserCapabilities {
            id: "test!!".to_string(),
            name: "Chrome@Home".to_string(),
            browser_type: "chrome#beta".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Configured,
        };
        // Filter "chrome" matches the browser_type substring after normalization
        assert!(manager.capability_matches_filters(&cap, &["chrome".to_string()]));
        // Filter "home" matches the normalized name "chromehome"
        assert!(manager.capability_matches_filters(&cap, &["home".to_string()]));
    }

    #[test]
    fn tdd_green_connector_count_returns_zero_with_empty_registry() {
        let registry = ConnectorRegistry::empty();
        let config = crate::config::Config::default();
        assert_eq!(registry.available(&config).len(), 0);
    }
}
