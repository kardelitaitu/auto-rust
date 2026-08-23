//! Browser connector abstraction for discovery and connection.
//!
//! Provides a trait-based interface for connecting to different browser sources:
//! - Configured browser profiles
//! - `RoxyBrowser` cloud instances
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
    /// From `RoxyBrowser` API
    RoxyBrowser,
    /// From `IxBrowser` API
    IxBrowser,
    /// From `ShardBrowser` (shardx-launcher) API
    ShardBrowser,
    /// Auto-discovered local browser
    Local,
}

impl std::fmt::Display for BrowserSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserSource::Configured => write!(f, "configured"),
            BrowserSource::RoxyBrowser => write!(f, "roxybrowser"),
            BrowserSource::IxBrowser => write!(f, "ixbrowser"),
            BrowserSource::ShardBrowser => write!(f, "shardbrowser"),
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
    #[must_use]
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

        let connect_timeout =
            Duration::from_millis(config.browser.connection_timeout_ms.get().max(5000));

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

/// Connector for `RoxyBrowser` cloud instances.
pub struct RoxyBrowserConnector;

impl RoxyBrowserConnector {
    /// Creates a new `RoxyBrowser` connector.
    #[must_use]
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

        if !api_reachable(api_url).await {
            debug!("RoxyBrowser API not reachable at {api_url}, skipping discovery");
            return Ok(vec![]);
        }

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
                .map_or_else(|| format!("roxy-{i}"), |s| format!("roxy-{s}"));

            let profile_name = profile
                .get("name")
                .or_else(|| profile.get("windowName"))
                .and_then(|n| n.as_str())
                .map_or_else(
                    || format!("RoxyBrowser-{i}"),
                    std::string::ToString::to_string,
                );

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
        let connect_timeout =
            Duration::from_millis(config.browser.connection_timeout_ms.get().max(5000));

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

/// Budget for the TCP reachability probe used before HTTP discovery calls.
///
/// Live local browser APIs accept TCP connections in single-digit ms, while a
/// dead endpoint on Windows can take ~2s to refuse. Probing with a short budget
/// lets discovery skip unreachable services almost instantly instead of paying
/// the full HTTP connect cost per connector.
const API_REACHABILITY_TIMEOUT: Duration = Duration::from_millis(500);

/// Quickly checks whether the host:port of an http(s) base URL accepts TCP
/// connections within `API_REACHABILITY_TIMEOUT`.
#[must_use]
async fn api_reachable(base_url: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(base_url) else {
        return false;
    };
    let Some(host) = url.host_str() else {
        return false;
    };
    let port = url.port_or_known_default().unwrap_or(80);

    tokio::time::timeout(
        API_REACHABILITY_TIMEOUT,
        tokio::net::TcpStream::connect((host, port)),
    )
    .await
    .map(|res| res.is_ok())
    .unwrap_or(false)
}

/// Helper to resolve a WebSocket debugger URL from a host:port or URL string.
async fn resolve_ws_url(address: &str) -> Option<String> {
    let address = address.trim();
    if address.is_empty() {
        return None;
    }

    // If it's already a ws/wss URL, return it
    if address.starts_with("ws://") || address.starts_with("wss://") {
        return Some(address.to_string());
    }

    // Prepare HTTP URL for json/version
    let http_url = if address.starts_with("http://") || address.starts_with("https://") {
        if address.ends_with("/json/version") {
            address.to_string()
        } else if address.ends_with('/') {
            format!("{}json/version", address)
        } else {
            format!("{}/json/version", address)
        }
    } else {
        format!("http://{}/json/version", address)
    };

    let client = reqwest::Client::new();
    let response = client
        .get(&http_url)
        .timeout(Duration::from_millis(1500))
        .send()
        .await
        .ok()?;

    if response.status().is_success() {
        if let Ok(version_data) = response.json::<serde_json::Value>().await {
            if let Some(ws_url) = version_data
                .get("webSocketDebuggerUrl")
                .and_then(serde_json::Value::as_str)
            {
                return Some(ws_url.to_string());
            }
        }
    }
    None
}

/// Connector for `IxBrowser` instances.
pub struct IxBrowserConnector;

impl IxBrowserConnector {
    /// Creates a new `IxBrowser` connector.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for IxBrowserConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "windows")]
fn get_running_ixbrowser_ports() -> std::collections::HashMap<String, String> {
    let mut ports = std::collections::HashMap::new();
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"name = 'chrome.exe'\" | Where-Object { $_.CommandLine -like '*--protected-userid=*' } | Select-Object -ExpandProperty CommandLine"
        ])
        .output();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if line.contains("--protected-userid=") && line.contains("--remote-debugging-port=") {
                let mut uid = None;
                let mut port = None;
                if let Some(idx) = line.find("--protected-userid=") {
                    let sub = &line[idx + "--protected-userid=".len()..];
                    let end = sub.find(' ').unwrap_or(sub.len());
                    uid = Some(sub[..end].trim_matches('"').to_string());
                }
                if let Some(idx) = line.find("--remote-debugging-port=") {
                    let sub = &line[idx + "--remote-debugging-port=".len()..];
                    let end = sub.find(' ').unwrap_or(sub.len());
                    port = Some(sub[..end].trim_matches('"').to_string());
                }
                if let (Some(u), Some(p)) = (uid, port) {
                    ports.insert(u, p);
                }
            }
        }
    }
    ports
}

#[cfg(not(target_os = "windows"))]
fn get_running_ixbrowser_ports() -> std::collections::HashMap<String, String> {
    std::collections::HashMap::new()
}

#[async_trait]
impl BrowserConnector for IxBrowserConnector {
    fn is_available(&self, config: &Config) -> bool {
        config.browser.ixbrowser.enabled && !config.browser.ixbrowser.api_url.is_empty()
    }

    async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>> {
        let api_url = &config.browser.ixbrowser.api_url;

        // Normalize base URL to ensure trailing slash and include /api/v2/ if not present
        let mut base_url = api_url.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }
        let base_url = if base_url.contains("/api/v2") {
            base_url
        } else {
            format!("{}api/v2/", base_url)
        };

        info!("Discovering IxBrowser from: {base_url}");

        if !api_reachable(api_url).await {
            debug!("IxBrowser API not reachable at {api_url}, skipping discovery");
            return Ok(vec![]);
        }

        let client = crate::api::ApiClient::new(base_url.clone());

        #[derive(serde::Deserialize)]
        struct IxBrowserError {
            code: i64,
            message: Option<String>,
        }

        #[derive(serde::Deserialize)]
        struct IxBrowserResponse {
            error: IxBrowserError,
            data: Option<serde_json::Value>,
        }

        // Hit the "profile-opened-list" POST endpoint to get active profiles
        let response: IxBrowserResponse = match client.post("profile-opened-list").await {
            Ok(resp) => resp,
            Err(e) => {
                warn!("IxBrowser API request failed: {}", e);
                return Ok(vec![]);
            }
        };

        if response.error.code != 0 && response.error.code != 200 {
            let msg = response.error.message.as_deref().unwrap_or("unknown");
            warn!(
                "IxBrowser API error: {} (code: {})",
                msg, response.error.code
            );
            return Ok(vec![]);
        }

        let profiles = match response.data {
            Some(serde_json::Value::Array(arr)) => arr,
            Some(serde_json::Value::Object(obj)) => {
                if let Some(serde_json::Value::Array(arr)) =
                    obj.get("list").or_else(|| obj.get("data"))
                {
                    arr.clone()
                } else {
                    vec![]
                }
            }
            _ => vec![],
        };

        if profiles.is_empty() {
            info!("No active/opened IxBrowser profiles found");
            return Ok(vec![]);
        }

        info!("Found {} active/opened IxBrowser profiles", profiles.len());

        let process_ports = get_running_ixbrowser_ports();

        let mut capabilities = Vec::new();

        for (i, profile) in profiles.iter().enumerate() {
            let profile_id_opt = profile
                .get("profile_id")
                .or_else(|| profile.get("profileId"))
                .or_else(|| profile.get("id"));

            let profile_id = match profile_id_opt {
                Some(serde_json::Value::Number(num)) => num.to_string(),
                Some(serde_json::Value::String(s)) => s.clone(),
                _ => format!("ixbrowser-{}", i),
            };

            let profile_id_val = if let Ok(id_int) = profile_id.parse::<i64>() {
                serde_json::Value::Number(id_int.into())
            } else {
                serde_json::Value::String(profile_id.clone())
            };

            // Only proceed if this profile is actually running locally
            let Some(port) = process_ports.get(&profile_id) else {
                info!("Profile ID {} is not running locally, skipping", profile_id);
                continue;
            };

            // Query profile details from profile-list to get the profile's user-friendly name
            let mut profile_name = format!("IxBrowserProfile-{}", profile_id);
            #[derive(serde::Serialize)]
            struct ListRequest {
                profile_id: serde_json::Value,
            }
            let list_req = ListRequest {
                profile_id: profile_id_val.clone(),
            };
            if let Ok(list_resp) = client
                .post_json::<_, IxBrowserResponse>("profile-list", &list_req)
                .await
            {
                if list_resp.error.code == 0 || list_resp.error.code == 200 {
                    if let Some(list_data) = list_resp.data {
                        if let Some(serde_json::Value::Array(arr)) = list_data.get("data") {
                            if let Some(profile_obj) = arr.first() {
                                if let Some(name_str) =
                                    profile_obj.get("name").and_then(serde_json::Value::as_str)
                                {
                                    profile_name = name_str.to_string();
                                }
                            }
                        }
                    }
                }
            }

            // Resolve WebSocket URL from the debugging port
            let ws_url = resolve_ws_url(&format!("127.0.0.1:{}", port)).await;

            let Some(ws_url) = ws_url else {
                warn!(
                    "Profile '{}' has no active debugging address or WebSocket URL, skipping",
                    profile_name
                );
                continue;
            };

            capabilities.push(BrowserCapabilities {
                id: format!("ixbrowser-{}", profile_id),
                name: profile_name,
                browser_type: "ixbrowser".to_string(),
                ws_url,
                source: BrowserSource::IxBrowser,
            });
        }

        Ok(capabilities)
    }

    async fn connect(&self, capability: &BrowserCapabilities, config: &Config) -> Result<Session> {
        let connect_timeout =
            Duration::from_millis(config.browser.connection_timeout_ms.get().max(5000));

        match tokio::time::timeout(
            connect_timeout,
            chromiumoxide::Browser::connect(&capability.ws_url),
        )
        .await
        {
            Ok(Ok((browser, handler))) => {
                info!("Connected to IxBrowser: {}", capability.name);
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
                format!("Failed to connect to IxBrowser {}: {}", capability.name, e),
            ))),
            Err(_) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Connection timeout to IxBrowser {} after {}ms",
                    capability.name,
                    connect_timeout.as_millis()
                ),
            ))),
        }
    }
}

/// Fetch a JSON value from the ShardBrowser launcher API with Bearer auth.
/// Returns `None` on HTTP/network failure (the caller decides whether to retry
/// or skip).
async fn fetch_json(base_url: &str, path: &str, auth: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = format!("{base_url}{path}");
    let response = client
        .get(&url)
        .header("Authorization", auth)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        warn!(
            "ShardBrowser API returned status {} for {url}",
            response.status()
        );
        return None;
    }
    response.json::<serde_json::Value>().await.ok()
}

/// Fetch a JSON array from the ShardBrowser launcher API.
/// Handles both bare arrays and objects wrapping the list under `"data"`/`"list"`.
fn extract_json_array(value: &serde_json::Value) -> Vec<serde_json::Value> {
    match value {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(obj) => obj
            .get("data")
            .or_else(|| obj.get("list"))
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default(),
        _ => vec![],
    }
}

/// Extract a CDP connection hint from a ShardBrowser object (a `/running`
/// instance or a `/profiles` entry).
///
/// The launcher exposes a `cdp` field on open profiles, either as an object
/// `{ http_url, port, web_socket_debugger_url }` or as a bare ws/http string.
/// Returns the most useful hint (`web_socket_debugger_url`, then `http_url`,
/// then `port`-based address), or `None` when no CDP endpoint is exposed.
fn extract_cdp_hint(profile: &serde_json::Value) -> Option<String> {
    match profile.get("cdp") {
        Some(serde_json::Value::Object(map)) => map
            .get("web_socket_debugger_url")
            .or_else(|| map.get("http_url"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                map.get("port")
                    .and_then(serde_json::Value::as_u64)
                    .map(|port| format!("127.0.0.1:{port}"))
            }),
        Some(serde_json::Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

/// Connector for `ShardBrowser` (shardx-launcher) instances.
pub struct ShardBrowserConnector;

impl ShardBrowserConnector {
    /// Creates a new `ShardBrowser` connector.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for ShardBrowserConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserConnector for ShardBrowserConnector {
    fn is_available(&self, config: &Config) -> bool {
        config.browser.shardbrowser.enabled && !config.browser.shardbrowser.api_url.is_empty()
    }

    async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>> {
        let api_url = &config.browser.shardbrowser.api_url;
        let api_key = &config.browser.shardbrowser.api_key;

        info!("Discovering ShardBrowser from: {api_url}");

        if !api_reachable(api_url).await {
            debug!("ShardBrowser API not reachable at {api_url}, skipping discovery");
            return Ok(vec![]);
        }

        let base_url = api_url.trim_end_matches('/');
        let auth = format!("Bearer {api_key}");

        // Only profiles that are actually OPEN are candidates. The launcher's
        // `/running` endpoint reports exactly those — do not iterate every
        // created profile.
        let running = fetch_json(base_url, "/running", &auth)
            .await
            .map(|v| extract_json_array(&v))
            .unwrap_or_default();

        if running.is_empty() {
            info!("No ShardBrowser profiles are open");
            return Ok(vec![]);
        }

        // Fetch profile metadata (names, cdp fallback) once and index by id.
        let profiles = fetch_json(base_url, "/profiles", &auth)
            .await
            .map(|v| extract_json_array(&v))
            .unwrap_or_default();
        let by_id: std::collections::HashMap<&str, &serde_json::Value> = profiles
            .iter()
            .filter_map(|p| {
                p.get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(|id| (id, p))
            })
            .collect();

        let mut capabilities = Vec::new();

        for (i, instance) in running.iter().enumerate() {
            let profile_id = instance
                .get("profile_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            let profile_meta = if !profile_id.is_empty() {
                by_id.get(profile_id).copied()
            } else {
                None
            }
            .unwrap_or(instance);

            let profile_name = profile_meta
                .get("name")
                .and_then(serde_json::Value::as_str)
                .map_or_else(|| format!("ShardBrowser-{i}"), str::to_string);

            // CDP hint comes from the running instance when the launcher
            // exposes it; fall back to the profile metadata.
            let Some(cdp_hint) =
                extract_cdp_hint(instance).or_else(|| extract_cdp_hint(profile_meta))
            else {
                warn!(
                    "ShardBrowser profile '{profile_name}' is open but exposes no CDP endpoint (profiles launched from the app UI lack remote debugging)"
                );
                continue;
            };

            // `cdp` may already be a ws:// URL or an http address needing resolution.
            let ws_url = if cdp_hint.starts_with("ws://") || cdp_hint.starts_with("wss://") {
                Some(cdp_hint)
            } else {
                resolve_ws_url(&cdp_hint).await
            };

            let Some(ws_url) = ws_url else {
                warn!(
                    "ShardBrowser profile '{profile_name}' has no resolvable CDP address, skipping"
                );
                continue;
            };

            capabilities.push(BrowserCapabilities {
                id: format!("shardbrowser-{profile_id}"),
                name: profile_name,
                browser_type: "shardbrowser".to_string(),
                ws_url,
                source: BrowserSource::ShardBrowser,
            });
        }

        info!(
            "Found {} open ShardBrowser profile(s) with CDP",
            capabilities.len()
        );

        Ok(capabilities)
    }

    async fn connect(&self, capability: &BrowserCapabilities, config: &Config) -> Result<Session> {
        let connect_timeout =
            Duration::from_millis(config.browser.connection_timeout_ms.get().max(5000));

        match tokio::time::timeout(
            connect_timeout,
            chromiumoxide::Browser::connect(&capability.ws_url),
        )
        .await
        {
            Ok(Ok((browser, handler))) => {
                info!("Connected to ShardBrowser: {}", capability.name);
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
                    "Failed to connect to ShardBrowser {}: {}",
                    capability.name, e
                ),
            ))),
            Err(_) => Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Connection timeout to ShardBrowser {} after {}ms",
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            brave_port_start: DEFAULT_BRAVE_PORT_START,
            brave_port_end: DEFAULT_BRAVE_PORT_END,
            chrome_port_start: DEFAULT_CHROME_PORT_START,
            chrome_port_end: DEFAULT_CHROME_PORT_END,
        }
    }

    /// Creates a connector with custom port ranges from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            brave_port_start: Self::parse_port_env("BRAVE_PORT_START", DEFAULT_BRAVE_PORT_START),
            brave_port_end: Self::parse_port_env("BRAVE_PORT_END", DEFAULT_BRAVE_PORT_END),
            chrome_port_start: Self::parse_port_env("CHROME_PORT_START", DEFAULT_CHROME_PORT_START),
            chrome_port_end: Self::parse_port_env("CHROME_PORT_END", DEFAULT_CHROME_PORT_END),
        }
    }

    /// Parse a port value from a string, returning `None` on parse failure.
    ///
    /// This is the pure parsing core of `parse_port_env`, extracted for testability.
    #[must_use]
    fn parse_port_value(val: &str) -> Option<u16> {
        val.parse::<u16>().ok()
    }

    fn parse_port_env(var_name: &str, default: u16) -> u16 {
        match std::env::var(var_name) {
            Ok(val) => match Self::parse_port_value(&val) {
                Some(port) => port,
                None => {
                    warn!(
                    "[browser] Invalid port value in {var_name}: '{val}'. Using default: {default}"
                );
                    default
                }
            },
            Err(_) => default,
        }
    }

    /// Construct the CDP version URL for a given port.
    #[must_use]
    fn cdp_version_url(port: u16) -> String {
        format!("http://127.0.0.1:{port}/json/version")
    }

    /// Extract the WebSocket debugger URL from a `/json/version` response.
    #[must_use]
    fn extract_ws_url_from_version(value: &serde_json::Value) -> Option<String> {
        value
            .get("webSocketDebuggerUrl")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    }

    /// Build a `BrowserCapabilities` for a locally discovered browser.
    #[must_use]
    fn make_local_browser_capability(
        port: u16,
        browser_type: &str,
        ws_url: &str,
    ) -> BrowserCapabilities {
        BrowserCapabilities {
            id: format!("{browser_type}-{port}"),
            name: format!("{browser_type} on port {port}"),
            browser_type: format!("local{browser_type}"),
            ws_url: ws_url.to_string(),
            source: BrowserSource::Local,
        }
    }

    async fn check_port(
        &self,
        port: u16,
        browser_type: &str,
        _config: &Config,
    ) -> Option<BrowserCapabilities> {
        let cdp_url = Self::cdp_version_url(port);

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
                    if let Some(ws_url) = Self::extract_ws_url_from_version(&version_data) {
                        info!("Found {browser_type} browser on port {port}");
                        return Some(Self::make_local_browser_capability(
                            port,
                            browser_type,
                            &ws_url,
                        ));
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
        let connect_timeout =
            Duration::from_millis(config.browser.connection_timeout_ms.get().max(5000));

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
    #[must_use]
    pub fn standard() -> Self {
        Self {
            connectors: vec![
                Box::new(ConfiguredProfileConnector::new()),
                Box::new(RoxyBrowserConnector::new()),
                Box::new(IxBrowserConnector::new()),
                Box::new(ShardBrowserConnector::new()),
                Box::new(LocalBrowserConnector::from_env()),
            ],
        }
    }

    /// Creates a new empty registry.
    #[must_use]
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
    #[must_use]
    pub fn available(&self, config: &Config) -> Vec<&dyn BrowserConnector> {
        self.connectors
            .iter()
            .filter(|c| c.is_available(config))
            .map(std::convert::AsRef::as_ref)
            .collect()
    }

    /// Returns all connectors regardless of availability.
    #[must_use]
    pub fn all(&self) -> Vec<&dyn BrowserConnector> {
        self.connectors
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect()
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
        assert_eq!(BrowserSource::IxBrowser.to_string(), "ixbrowser");
        assert_eq!(BrowserSource::ShardBrowser.to_string(), "shardbrowser");
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
        assert_eq!(registry.all().len(), 5);
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
    fn test_shard_connector_available_when_enabled() {
        let connector = ShardBrowserConnector::new();
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                shardbrowser: crate::config::ShardbrowserConfig {
                    enabled: true,
                    api_url: "http://127.0.0.1:40325".to_string(),
                    api_key: "test-key".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(connector.is_available(&config));
    }

    #[test]
    fn test_shard_connector_not_available_when_disabled() {
        let connector = ShardBrowserConnector::new();
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                shardbrowser: crate::config::ShardbrowserConfig {
                    enabled: false,
                    api_url: "http://127.0.0.1:40325".to_string(),
                    api_key: "test-key".to_string(),
                },
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!connector.is_available(&config));
    }

    #[test]
    fn test_extract_cdp_hint_object_with_ws() {
        let profile = serde_json::json!({
            "id": "abc",
            "name": "test",
            "running": true,
            "cdp": {
                "http_url": "http://127.0.0.1:37370",
                "port": 37370,
                "web_socket_debugger_url": "ws://127.0.0.1:37370/devtools/browser/uuid"
            }
        });
        assert_eq!(
            extract_cdp_hint(&profile),
            Some("ws://127.0.0.1:37370/devtools/browser/uuid".to_string())
        );
    }

    #[test]
    fn test_extract_cdp_hint_object_http_only() {
        let profile = serde_json::json!({
            "cdp": { "http_url": "http://127.0.0.1:8080", "port": 8080 }
        });
        assert_eq!(
            extract_cdp_hint(&profile),
            Some("http://127.0.0.1:8080".to_string())
        );
    }

    #[test]
    fn test_extract_cdp_hint_object_port_only() {
        let profile = serde_json::json!({ "cdp": { "port": 9000 } });
        assert_eq!(
            extract_cdp_hint(&profile),
            Some("127.0.0.1:9000".to_string())
        );
    }

    #[test]
    fn test_extract_cdp_hint_string() {
        let profile = serde_json::json!({ "cdp": "ws://127.0.0.1:5555/devtools" });
        assert_eq!(
            extract_cdp_hint(&profile),
            Some("ws://127.0.0.1:5555/devtools".to_string())
        );
    }

    #[test]
    fn test_extract_cdp_hint_missing_or_empty() {
        assert_eq!(extract_cdp_hint(&serde_json::json!({})), None);
        assert_eq!(extract_cdp_hint(&serde_json::json!({ "cdp": null })), None);
        assert_eq!(extract_cdp_hint(&serde_json::json!({ "cdp": "" })), None);
        assert_eq!(extract_cdp_hint(&serde_json::json!({ "cdp": 42 })), None);
    }

    #[tokio::test]
    async fn test_api_reachable_invalid_url_is_false() {
        assert!(!api_reachable("").await);
        assert!(!api_reachable("not a url").await);
        assert!(!api_reachable("ftp://127.0.0.1:21").await);
    }

    #[tokio::test]
    async fn test_api_reachable_alive_port_is_true() {
        // A live listener on loopback resolves within the 500ms budget.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        assert!(api_reachable(&format!("http://127.0.0.1:{port}/")).await);
    }

    #[test]
    fn test_local_connector_default_ports() {
        let connector = LocalBrowserConnector::new();
        assert_eq!(connector.brave_port_start, 9001);
        assert_eq!(connector.brave_port_end, 9050);
        assert_eq!(connector.chrome_port_start, 9222);
        assert_eq!(connector.chrome_port_end, 9230);
    }

    // =========================================================================
    // parse_port_value tests
    // =========================================================================

    #[test]
    fn test_parse_port_value_valid_mid_range() {
        assert_eq!(LocalBrowserConnector::parse_port_value("9222"), Some(9222));
    }

    #[test]
    fn test_parse_port_value_valid_brave_start() {
        assert_eq!(LocalBrowserConnector::parse_port_value("9001"), Some(9001));
    }

    #[test]
    fn test_parse_port_value_valid_zero() {
        assert_eq!(LocalBrowserConnector::parse_port_value("0"), Some(0));
    }

    #[test]
    fn test_parse_port_value_valid_one() {
        assert_eq!(LocalBrowserConnector::parse_port_value("1"), Some(1));
    }

    #[test]
    fn test_parse_port_value_valid_max() {
        assert_eq!(
            LocalBrowserConnector::parse_port_value("65535"),
            Some(65535)
        );
    }

    #[test]
    fn test_parse_port_value_valid_leading_zeros() {
        assert_eq!(
            LocalBrowserConnector::parse_port_value("008080"),
            Some(8080)
        );
    }

    #[test]
    fn test_parse_port_value_invalid_negative() {
        assert_eq!(LocalBrowserConnector::parse_port_value("-1"), None);
    }

    #[test]
    fn test_parse_port_value_invalid_too_large() {
        assert_eq!(LocalBrowserConnector::parse_port_value("65536"), None);
    }

    #[test]
    fn test_parse_port_value_invalid_empty_string() {
        assert_eq!(LocalBrowserConnector::parse_port_value(""), None);
    }

    #[test]
    fn test_parse_port_value_invalid_non_numeric() {
        assert_eq!(LocalBrowserConnector::parse_port_value("abc"), None);
    }

    #[test]
    fn test_parse_port_value_invalid_float() {
        assert_eq!(LocalBrowserConnector::parse_port_value("9222.5"), None);
    }

    #[test]
    fn test_parse_port_value_invalid_whitespace() {
        assert_eq!(LocalBrowserConnector::parse_port_value(" 9222 "), None);
    }

    #[test]
    fn test_parse_port_value_invalid_special_chars() {
        assert_eq!(LocalBrowserConnector::parse_port_value("9@22"), None);
    }

    #[test]
    fn test_parse_port_value_invalid_hex() {
        assert_eq!(LocalBrowserConnector::parse_port_value("0xABCD"), None);
    }

    #[test]
    fn test_parse_port_value_overflow_u16_plus_one() {
        assert_eq!(LocalBrowserConnector::parse_port_value("70000"), None);
    }

    #[test]
    fn test_parse_port_value_valid_u16_max() {
        assert_eq!(
            LocalBrowserConnector::parse_port_value("65535"),
            Some(u16::MAX)
        );
    }

    // =========================================================================
    // parse_port_env end-to-end tests
    // =========================================================================

    #[test]
    fn test_parse_port_env_var_not_set_returns_default() {
        // When env var is not set, should return the default
        // Use a unique var name to avoid collision with real env
        let result =
            LocalBrowserConnector::parse_port_env("_TEST_UNSET_VAR_SHOULD_NOT_EXIST_", 8080);
        assert_eq!(result, 8080);
    }

    #[test]
    fn test_parse_port_env_var_empty_returns_default() {
        // When env var is set to empty, parse_port_value returns None -> default
        let result = LocalBrowserConnector::parse_port_env("_TEST_EMPTY_PARSE_PORT_", 8080);
        // This depends on whether the env var is actually set; it's fine to use default
        assert_eq!(result, 8080);
    }

    // =========================================================================
    // check_port pure logic tests
    // =========================================================================

    #[test]
    fn test_cdp_version_url() {
        assert_eq!(
            LocalBrowserConnector::cdp_version_url(9222),
            "http://127.0.0.1:9222/json/version"
        );
        assert_eq!(
            LocalBrowserConnector::cdp_version_url(9001),
            "http://127.0.0.1:9001/json/version"
        );
        assert_eq!(
            LocalBrowserConnector::cdp_version_url(65535),
            "http://127.0.0.1:65535/json/version"
        );
    }

    #[test]
    fn test_extract_ws_url_from_version_found() {
        let json = serde_json::json!({
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/ABC123"
        });
        assert_eq!(
            LocalBrowserConnector::extract_ws_url_from_version(&json),
            Some("ws://127.0.0.1:9222/devtools/page/ABC123".to_string())
        );
    }

    #[test]
    fn test_extract_ws_url_from_version_missing_key() {
        let json = serde_json::json!({
            "other_key": "value"
        });
        assert_eq!(
            LocalBrowserConnector::extract_ws_url_from_version(&json),
            None
        );
    }

    #[test]
    fn test_extract_ws_url_from_version_null_value() {
        let json = serde_json::json!({
            "webSocketDebuggerUrl": null
        });
        assert_eq!(
            LocalBrowserConnector::extract_ws_url_from_version(&json),
            None
        );
    }

    #[test]
    fn test_extract_ws_url_from_version_wrong_type() {
        let json = serde_json::json!({
            "webSocketDebuggerUrl": 12345
        });
        assert_eq!(
            LocalBrowserConnector::extract_ws_url_from_version(&json),
            None
        );
    }

    #[test]
    fn test_extract_ws_url_from_version_empty_object() {
        let json = serde_json::json!({});
        assert_eq!(
            LocalBrowserConnector::extract_ws_url_from_version(&json),
            None
        );
    }

    #[test]
    fn test_extract_ws_url_from_version_array_response() {
        let json = serde_json::json!([]);
        assert_eq!(
            LocalBrowserConnector::extract_ws_url_from_version(&json),
            None
        );
    }

    #[test]
    fn test_make_local_browser_capability_brave() {
        let caps = LocalBrowserConnector::make_local_browser_capability(
            9001,
            "Brave",
            "ws://127.0.0.1:9001/devtools/page/ABC",
        );
        assert_eq!(caps.id, "Brave-9001");
        assert_eq!(caps.name, "Brave on port 9001");
        assert_eq!(caps.browser_type, "localBrave");
        assert_eq!(caps.ws_url, "ws://127.0.0.1:9001/devtools/page/ABC");
        assert_eq!(caps.source, BrowserSource::Local);
    }

    #[test]
    fn test_make_local_browser_capability_chrome() {
        let caps = LocalBrowserConnector::make_local_browser_capability(
            9222,
            "Chrome",
            "ws://127.0.0.1:9222/devtools/page/DEF",
        );
        assert_eq!(caps.id, "Chrome-9222");
        assert_eq!(caps.name, "Chrome on port 9222");
        assert_eq!(caps.browser_type, "localChrome");
        assert_eq!(caps.ws_url, "ws://127.0.0.1:9222/devtools/page/DEF");
        assert_eq!(caps.source, BrowserSource::Local);
    }

    #[test]
    fn test_make_local_browser_capability_edge_port() {
        let caps = LocalBrowserConnector::make_local_browser_capability(
            1,
            "Edge",
            "ws://127.0.0.1:1/devtools",
        );
        assert_eq!(caps.id, "Edge-1");
        assert_eq!(caps.name, "Edge on port 1");
        assert_eq!(caps.browser_type, "localEdge");
        assert_eq!(caps.ws_url, "ws://127.0.0.1:1/devtools");
        assert_eq!(caps.source, BrowserSource::Local);
    }

    #[test]
    fn test_make_local_browser_capability_max_port() {
        let caps = LocalBrowserConnector::make_local_browser_capability(
            65535,
            "Chrome",
            "ws://127.0.0.1:65535/devtools",
        );
        assert_eq!(caps.id, "Chrome-65535");
        assert_eq!(caps.name, "Chrome on port 65535");
        assert_eq!(caps.browser_type, "localChrome");
        assert_eq!(caps.ws_url, "ws://127.0.0.1:65535/devtools");
        assert_eq!(caps.source, BrowserSource::Local);
    }

    #[test]
    fn test_make_local_browser_capability_ws_url_special_chars() {
        let caps = LocalBrowserConnector::make_local_browser_capability(
            9222,
            "Chrome",
            "ws://127.0.0.1:9222/devtools/page/abc-123_def",
        );
        assert_eq!(caps.ws_url, "ws://127.0.0.1:9222/devtools/page/abc-123_def");
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
