//! Browser connector abstraction for discovery and connection.
//!
//! Provides a trait-based interface for connecting to different browser sources:
//! - Configured browser profiles
//! - RoxyBrowser cloud instances
//! - Local browser discovery (Brave, Chrome on common ports)

use crate::config::Config;
use crate::error::{BrowserError, OrchestratorError, Result};
use crate::session::Session;
use async_trait::async_trait;
use log::{debug, info, warn};
use std::time::Duration;

/// Capabilities of a browser instance discovered by a connector.
///
/// This struct holds metadata about a browser that can be used
/// for session construction and downstream decision-making.
#[derive(Debug, Clone)]
pub struct BrowserCapabilities {
    /// Unique identifier for the browser instance
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Browser type (e.g., "brave", "chrome", "roxybrowser")
    pub browser_type: String,
    /// WebSocket debugger URL for CDP connection
    pub ws_url: String,
    /// Source of discovery (config, roxybrowser, local)
    pub source: BrowserSource,
}

/// Source of browser discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserSource {
    /// From configured browser profiles
    Configured,
    /// From RoxyBrowser API
    RoxyBrowser,
    /// Auto-discovered local browser
    Local,
}

impl std::fmt::Display for BrowserSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserSource::Configured => write!(f, "configured"),
            BrowserSource::RoxyBrowser => write!(f, "roxybrowser"),
            BrowserSource::Local => write!(f, "local"),
        }
    }
}

/// Trait for browser connectors.
///
/// Implementors provide discovery and connection capabilities
/// for specific browser sources.
#[async_trait]
pub trait BrowserConnector: Send + Sync {
    /// Returns true if this connector is available for the given config.
    fn is_available(&self, _config: &Config) -> bool {
        true
    }

    /// Discovers available browsers without connecting.
    ///
    /// Returns a list of browser capabilities that can be
    /// used to establish connections later.
    async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>>;

    /// Connects to a specific browser capability and creates a Session.
    ///
    /// # Arguments
    /// * `capability` - The browser capability to connect to
    /// * `config` - The orchestrator configuration
    ///
    /// # Returns
    /// A connected Session instance
    async fn connect(&self, capability: &BrowserCapabilities, config: &Config) -> Result<Session>;
}

/// Connector for configured browser profiles.
pub struct ConfiguredProfileConnector;

impl ConfiguredProfileConnector {
    /// Creates a new configured profile connector.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ConfiguredProfileConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserConnector for ConfiguredProfileConnector {
    fn is_available(&self, config: &Config) -> bool {
        !config.browser.profiles.is_empty()
    }

    async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>> {
        let mut capabilities = Vec::new();

        for profile in &config.browser.profiles {
            capabilities.push(BrowserCapabilities {
                id: format!("config-{}", profile.name),
                name: profile.name.clone(),
                browser_type: profile.r#type.clone(),
                ws_url: profile.ws_endpoint.clone(),
                source: BrowserSource::Configured,
            });
        }

        debug!(
            "Discovered {} configured browser profiles",
            capabilities.len()
        );
        Ok(capabilities)
    }

    async fn connect(&self, capability: &BrowserCapabilities, config: &Config) -> Result<Session> {
        if capability.ws_url.is_empty() {
            return Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!("Empty WebSocket endpoint for profile: {}", capability.name),
            )));
        }

        let connect_timeout = Duration::from_millis(config.browser.connection_timeout_ms.max(5000));

        match tokio::time::timeout(
            connect_timeout,
            chromiumoxide::Browser::connect(&capability.ws_url),
        )
        .await
        {
            Ok(Ok((browser, handler))) => {
                debug!("Connected to configured profile: {}", capability.name);
                Ok(Session::new(
                    capability.id.clone(),
                    capability.name.clone(),
                    capability.browser_type.clone(),
                    browser,
                    handler,
                    config.browser.max_workers_per_session,
                    config.browser.cursor_overlay_ms,
                    Some(config.browser.circuit_breaker.clone()),
                ))
            }
            Ok(Err(e)) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!("Failed to connect to {}: {}", capability.name, e),
            ))),
            Err(_) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Connection timeout to {} after {}ms",
                    capability.name,
                    connect_timeout.as_millis()
                ),
            ))),
        }
    }
}

/// Connector for RoxyBrowser cloud instances.
pub struct RoxyBrowserConnector;

impl RoxyBrowserConnector {
    /// Creates a new RoxyBrowser connector.
    pub fn new() -> Self {
        Self
    }
}

impl Default for RoxyBrowserConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserConnector for RoxyBrowserConnector {
    fn is_available(&self, config: &Config) -> bool {
        config.browser.roxybrowser.enabled && !config.browser.roxybrowser.api_url.is_empty()
    }

    async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>> {
        let api_url = &config.browser.roxybrowser.api_url;
        let api_key = &config.browser.roxybrowser.api_key;

        info!("Discovering RoxyBrowser from: {api_url}");

        let client = crate::api::ApiClient::new(api_url.clone());

        #[derive(serde::Deserialize)]
        struct RoxyResponse {
            code: i64,
            msg: Option<String>,
            data: Option<Vec<serde_json::Value>>,
        }

        let response: RoxyResponse = client
            .get_with_key("browser/connection_info", api_key)
            .await?;

        if response.code != 0 {
            let msg = response.msg.as_deref().unwrap_or("unknown");
            warn!("RoxyBrowser API error: {} (code: {})", msg, response.code);
            return Ok(vec![]);
        }

        let profiles = response.data.unwrap_or_default();

        if profiles.is_empty() {
            info!("No open RoxyBrowser profiles found");
            return Ok(vec![]);
        }

        info!("Found {} RoxyBrowser profiles", profiles.len());

        let mut capabilities = Vec::new();

        for (i, profile) in profiles.iter().enumerate() {
            let ws_url = profile
                .get("ws")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);

            let http_url = profile
                .get("http")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string);

            let ws_url = if let Some(url) = ws_url {
                url
            } else if let Some(http) = http_url {
                http.replace("http", "ws")
            } else {
                warn!("Profile {i} missing ws/http, skipping");
                continue;
            };

            let profile_id = profile
                .get("windowName")
                .and_then(|w| w.as_str())
                .map(|s| format!("roxy-{s}"))
                .unwrap_or_else(|| format!("roxy-{i}"));

            let profile_name = profile
                .get("name")
                .or_else(|| profile.get("windowName"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("RoxyBrowser-{i}"));

            capabilities.push(BrowserCapabilities {
                id: profile_id,
                name: profile_name,
                browser_type: "roxybrowser".to_string(),
                ws_url,
                source: BrowserSource::RoxyBrowser,
            });
        }

        Ok(capabilities)
    }

    async fn connect(&self, capability: &BrowserCapabilities, config: &Config) -> Result<Session> {
        let connect_timeout = Duration::from_millis(config.browser.connection_timeout_ms.max(5000));

        match tokio::time::timeout(
            connect_timeout,
            chromiumoxide::Browser::connect(&capability.ws_url),
        )
        .await
        {
            Ok(Ok((browser, handler))) => {
                info!("Connected to RoxyBrowser: {}", capability.name);
                Ok(Session::new(
                    capability.id.clone(),
                    capability.name.clone(),
                    capability.browser_type.clone(),
                    browser,
                    handler,
                    config.browser.max_workers_per_session,
                    config.browser.cursor_overlay_ms,
                    Some(config.browser.circuit_breaker.clone()),
                ))
            }
            Ok(Err(e)) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Failed to connect to RoxyBrowser {}: {}",
                    capability.name, e
                ),
            ))),
            Err(_) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Connection timeout to RoxyBrowser {} after {}ms",
                    capability.name,
                    connect_timeout.as_millis()
                ),
            ))),
        }
    }
}

/// Connector for local browser auto-discovery.
pub struct LocalBrowserConnector {
    brave_port_start: u16,
    brave_port_end: u16,
    chrome_port_start: u16,
    chrome_port_end: u16,
}

// Default port ranges for browser discovery
const DEFAULT_BRAVE_PORT_START: u16 = 9001;
const DEFAULT_BRAVE_PORT_END: u16 = 9050;
const DEFAULT_CHROME_PORT_START: u16 = 9222;
const DEFAULT_CHROME_PORT_END: u16 = 9230;
const MIN_PORT: u16 = 1024;
const MAX_PORT: u16 = 65535;

impl LocalBrowserConnector {
    /// Creates a new local browser connector with default port ranges.
    pub fn new() -> Self {
        Self {
            brave_port_start: DEFAULT_BRAVE_PORT_START,
            brave_port_end: DEFAULT_BRAVE_PORT_END,
            chrome_port_start: DEFAULT_CHROME_PORT_START,
            chrome_port_end: DEFAULT_CHROME_PORT_END,
        }
    }

    /// Creates a connector with custom port ranges from environment variables.
    pub fn from_env() -> Self {
        Self {
            brave_port_start: Self::parse_port_env("BRAVE_PORT_START", DEFAULT_BRAVE_PORT_START),
            brave_port_end: Self::parse_port_env("BRAVE_PORT_END", DEFAULT_BRAVE_PORT_END),
            chrome_port_start: Self::parse_port_env("CHROME_PORT_START", DEFAULT_CHROME_PORT_START),
            chrome_port_end: Self::parse_port_env("CHROME_PORT_END", DEFAULT_CHROME_PORT_END),
        }
    }

    fn parse_port_env(var_name: &str, default: u16) -> u16 {
        match std::env::var(var_name) {
            Ok(val) => match val.parse::<u16>() {
                Ok(port) => port,
                Err(_) => {
                    warn!(
                        "[browser] Invalid port value in {}: '{}'. Using default: {}",
                        var_name, val, default
                    );
                    default
                }
            },
            Err(_) => default,
        }
    }

    async fn check_port(
        &self,
        port: u16,
        browser_type: &str,
        _config: &Config,
    ) -> Option<BrowserCapabilities> {
        let cdp_url = format!("http://127.0.0.1:{port}/json/version");

        debug!("Checking {browser_type} on port {port}");

        let client = reqwest::Client::new();
        let response = client
            .get(&cdp_url)
            .timeout(Duration::from_millis(1000))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(version_data) = resp.json::<serde_json::Value>().await {
                    if let Some(ws_url) = version_data.get("webSocketDebuggerUrl") {
                        if let Some(ws_str) = ws_url.as_str() {
                            info!("Found {browser_type} browser on port {port}");
                            return Some(BrowserCapabilities {
                                id: format!("{browser_type}-{port}"),
                                name: format!("{} on port {}", browser_type, port),
                                browser_type: format!("local{browser_type}"),
                                ws_url: ws_str.to_string(),
                                source: BrowserSource::Local,
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        None
    }
}

impl Default for LocalBrowserConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserConnector for LocalBrowserConnector {
    fn is_available(&self, _config: &Config) -> bool {
        // Local connector is only available if we can actually discover browsers
        // For now, always return false since we can't check port availability
        // without actually scanning
        false
    }

    async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>> {
        use futures::stream::{self, StreamExt};

        let brave_ports: Vec<u16> =
            (self.brave_port_start..=self.brave_port_end.clamp(MIN_PORT, MAX_PORT)).collect();
        let chrome_ports: Vec<u16> =
            (self.chrome_port_start..=self.chrome_port_end.clamp(MIN_PORT, MAX_PORT)).collect();

        let brave_caps: Vec<Option<BrowserCapabilities>> = stream::iter(brave_ports)
            .map(|port| async move { self.check_port(port, "Brave", config).await })
            .buffer_unordered(50)
            .collect()
            .await;

        let chrome_caps: Vec<Option<BrowserCapabilities>> = stream::iter(chrome_ports)
            .map(|port| async move { self.check_port(port, "Chrome", config).await })
            .buffer_unordered(50)
            .collect()
            .await;

        let capabilities: Vec<BrowserCapabilities> = brave_caps
            .into_iter()
            .chain(chrome_caps)
            .flatten()
            .collect();

        info!(
            "Local discovery found {} browsers (Brave: {}-{}, Chrome: {}-{})",
            capabilities.len(),
            self.brave_port_start,
            self.brave_port_end,
            self.chrome_port_start,
            self.chrome_port_end
        );

        Ok(capabilities)
    }

    async fn connect(&self, capability: &BrowserCapabilities, config: &Config) -> Result<Session> {
        let connect_timeout = Duration::from_millis(config.browser.connection_timeout_ms.max(5000));

        match tokio::time::timeout(
            connect_timeout,
            chromiumoxide::Browser::connect(&capability.ws_url),
        )
        .await
        {
            Ok(Ok((browser, handler))) => {
                debug!("Connected to local browser: {}", capability.name);
                Ok(Session::new(
                    capability.id.clone(),
                    capability.name.clone(),
                    capability.browser_type.clone(),
                    browser,
                    handler,
                    config.browser.max_workers_per_session,
                    config.browser.cursor_overlay_ms,
                    Some(config.browser.circuit_breaker.clone()),
                ))
            }
            Ok(Err(e)) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!("Failed to connect to {}: {}", capability.name, e),
            ))),
            Err(_) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Connection timeout to {} after {}ms",
                    capability.name,
                    connect_timeout.as_millis()
                ),
            ))),
        }
    }
}

/// Registry of available browser connectors.
pub struct ConnectorRegistry {
    connectors: Vec<Box<dyn BrowserConnector>>,
}

impl ConnectorRegistry {
    /// Creates a new connector registry with all standard connectors.
    pub fn standard() -> Self {
        Self {
            connectors: vec![
                Box::new(ConfiguredProfileConnector::new()),
                Box::new(RoxyBrowserConnector::new()),
                Box::new(LocalBrowserConnector::from_env()),
            ],
        }
    }

    /// Creates a new empty registry.
    pub fn empty() -> Self {
        Self {
            connectors: Vec::new(),
        }
    }

    /// Adds a connector to the registry.
    pub fn add(&mut self, connector: Box<dyn BrowserConnector>) {
        self.connectors.push(connector);
    }

    /// Returns connectors that are available for the given config.
    pub fn available(&self, config: &Config) -> Vec<&dyn BrowserConnector> {
        self.connectors
            .iter()
            .filter(|c| c.is_available(config))
            .map(|c| c.as_ref())
            .collect()
    }

    /// Returns all connectors regardless of availability.
    pub fn all(&self) -> Vec<&dyn BrowserConnector> {
        self.connectors.iter().map(|c| c.as_ref()).collect()
    }
}

impl Default for ConnectorRegistry {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_source_display() {
        assert_eq!(BrowserSource::Configured.to_string(), "configured");
        assert_eq!(BrowserSource::RoxyBrowser.to_string(), "roxybrowser");
        assert_eq!(BrowserSource::Local.to_string(), "local");
    }

    #[test]
    fn test_connector_registry_empty() {
        let registry = ConnectorRegistry::empty();
        assert!(registry.all().is_empty());
    }

    #[test]
    fn test_connector_registry_standard() {
        let registry = ConnectorRegistry::standard();
        assert_eq!(registry.all().len(), 3);
    }

    #[test]
    fn test_configured_connector_available_with_profiles() {
        let connector = ConfiguredProfileConnector::new();
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![crate::config::BrowserProfile {
                    name: "test".to_string(),
                    r#type: "brave".to_string(),
                    ws_endpoint: "ws://localhost:9222".to_string(),
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(connector.is_available(&config));
    }

    #[test]
    fn test_configured_connector_not_available_empty() {
        let connector = ConfiguredProfileConnector::new();
        let config = crate::config::Config::default();
        assert!(!connector.is_available(&config));
    }

    #[test]
    fn test_roxy_connector_available_when_enabled() {
        let connector = RoxyBrowserConnector::new();
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                roxybrowser: crate::config::RoxybrowserConfig {
                    enabled: true,
                    api_url: "http://localhost:3000".to_string(),
                    api_key: "test".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(connector.is_available(&config));
    }

    #[test]
    fn test_roxy_connector_not_available_when_disabled() {
        let connector = RoxyBrowserConnector::new();
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                roxybrowser: crate::config::RoxybrowserConfig {
                    enabled: false,
                    api_url: "http://localhost:3000".to_string(),
                    api_key: "test".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!connector.is_available(&config));
    }

    #[test]
    fn test_local_connector_default_ports() {
        let connector = LocalBrowserConnector::new();
        assert_eq!(connector.brave_port_start, 9001);
        assert_eq!(connector.brave_port_end, 9050);
        assert_eq!(connector.chrome_port_start, 9222);
        assert_eq!(connector.chrome_port_end, 9230);
    }

    #[test]
    fn test_browser_capabilities_creation() {
        let caps = BrowserCapabilities {
            id: "test-1".to_string(),
            name: "Test Browser".to_string(),
            browser_type: "brave".to_string(),
            ws_url: "ws://localhost:9222".to_string(),
            source: BrowserSource::Configured,
        };

        assert_eq!(caps.id, "test-1");
        assert_eq!(caps.name, "Test Browser");
        assert_eq!(caps.browser_type, "brave");
        assert_eq!(caps.ws_url, "ws://localhost:9222");
        assert_eq!(caps.source, BrowserSource::Configured);
    }
}
