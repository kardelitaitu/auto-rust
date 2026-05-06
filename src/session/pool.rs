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
                    warn!("Connector discovery failed: {}", e);
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
                    warn!("No sessions established on attempt {}", attempt);
                }
                Ok(_) => {
                    debug!("No browsers discovered on attempt {}", attempt);
                }
                Err(e) => {
                    warn!("Discovery failed on attempt {}: {}", attempt, e);
                }
            }

            if attempt < self.max_retries {
                let delay = std::time::Duration::from_millis(1000 * attempt as u64);
                debug!("Retrying after {:?}...", delay);
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
                let delay = std::time::Duration::from_millis(1000 * attempt as u64);
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
    pub fn connector_count(&self, config: &Config) -> usize {
        self.registry.available(config).len()
    }

    /// Returns the maximum retry count.
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

/// Normalizes a browser token for filter matching.
fn normalize_browser_token(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::connector::{BrowserCapabilities, BrowserSource};

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
}
