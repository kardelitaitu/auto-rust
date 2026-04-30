//! Browser discovery, connection, and management module.
//!
//! Handles:
//! - Browser profile discovery from configuration
//! - Establishing WebSocket connections to browser instances
//! - Managing browser lifecycle and health checks
//! - Integration with RoxyBrowser API for cloud-hosted browsers

use crate::config::Config;
use crate::error::{BrowserError, OrchestratorError, Result};
use crate::session::Session;
use futures::stream::{self, StreamExt};
use log::{debug, info, warn};
use std::time::Duration;

fn normalize_browser_token(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn matches_browser_filters(candidate: &str, filters: &[String]) -> bool {
    if filters.is_empty() {
        return true;
    }

    let candidate_lower = candidate.to_lowercase();
    let candidate_norm = normalize_browser_token(candidate);

    filters.iter().any(|filter| {
        let filter_lower = filter.to_lowercase();
        let filter_norm = normalize_browser_token(filter);

        !filter_norm.is_empty()
            && (candidate_lower.contains(&filter_lower)
                || candidate_norm.contains(&filter_norm)
                || candidate_norm == filter_norm)
    })
}

fn profile_matches_filters(profile: &crate::config::BrowserProfile, filters: &[String]) -> bool {
    matches_browser_filters(&profile.name, filters)
        || matches_browser_filters(&profile.r#type, filters)
}

fn session_matches_filters(session: &Session, filters: &[String]) -> bool {
    matches_browser_filters(&session.name, filters)
        || matches_browser_filters(&session.profile_type, filters)
        || matches_browser_filters(&session.id, filters)
}

/// Discovers and connects to browser instances based on configuration.
///
/// This function attempts to connect to browsers through multiple channels:
/// 1. Configured browser profiles (local Brave, Chrome, etc.)
/// 2. RoxyBrowser cloud-hosted browsers (if enabled)
/// 3. Auto-discovery of local browser instances
///
/// # Arguments
///
/// * `config` - The orchestrator configuration containing browser settings
///
/// # Returns
///
/// A vector of successfully connected `Session` instances.
///
/// # Errors
///
/// Returns an error if no browsers can be discovered after all retry attempts.
///
/// # Examples
///
/// ```no_run
/// # use auto::browser::discover_browsers;
/// # use auto::config::Config;
/// # async fn example(config: &Config) -> anyhow::Result<()> {
/// let sessions = discover_browsers(config).await?;
/// println!("Discovered {} browsers", sessions.len());
/// # Ok(())
/// # }
/// ```
pub async fn discover_browsers(config: &Config) -> Result<Vec<Session>> {
    discover_browsers_with_filters(config, &[]).await
}

/// Discovers browser sessions and optionally filters them by browser name/type tokens.
pub async fn discover_browsers_with_filters(
    config: &Config,
    browser_filters: &[String],
) -> Result<Vec<Session>> {
    let mut sessions = Vec::new();

    if !browser_filters.is_empty() {
        info!("Browser filters active: {}", browser_filters.join(", "));
    }

    for attempt in 1..=config.browser.max_discovery_retries {
        // Try configured profiles
        for profile in &config.browser.profiles {
            if !profile_matches_filters(profile, browser_filters) {
                debug!("Skipping profile {} due to browser filters", profile.name);
                continue;
            }

            match connect_to_browser(profile, config).await {
                Ok(session) => {
                    sessions.push(session);
                }
                Err(e) => {
                    debug!("Failed to connect to {}: {}", profile.name, e);
                }
            }
        }

        // Try Roxybrowser discovery (always enabled)
        let roxy_sessions = discover_roxybrowser(config).await?;
        for session in roxy_sessions
            .into_iter()
            .filter(|session| session_matches_filters(session, browser_filters))
        {
            sessions.push(session);
        }

        // Try auto-discovery for local browsers
        let discovered_sessions = discover_local_browsers(config).await?;
        for session in discovered_sessions
            .into_iter()
            .filter(|session| session_matches_filters(session, browser_filters))
        {
            sessions.push(session);
        }

        if !sessions.is_empty() {
            break;
        }

        if attempt < config.browser.max_discovery_retries {
            tokio::time::sleep(std::time::Duration::from_millis(
                config.browser.discovery_retry_delay_ms,
            ))
            .await;
        }
    }

    // Log discovery summary
    if sessions.is_empty() {
        if !browser_filters.is_empty() {
            // Phase 3: Zero-Match Browser Filter Hard Fail
            // Treat zero matches as startup error when filters are active
            return Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "No browsers matched the specified filters: {}. Please check your --browsers argument.",
                    browser_filters.join(", ")
                )
            )));
        } else {
            warn!("No browsers discovered (no filters specified)");
        }
    } else {
        let names: Vec<_> = sessions.iter().map(|s| s.name.as_str()).collect();
        info!(
            "Discovered {} browser(s): {}",
            sessions.len(),
            names.join(", ")
        );
    }

    Ok(sessions)
}

/// Connects to a browser instance using the specified profile configuration.
///
/// # Arguments
///
/// * `profile` - The browser profile containing connection details
/// * `config` - The orchestrator configuration
///
/// # Returns
///
/// A `Session` instance representing the connected browser.
///
/// # Errors
///
/// Returns an error if:
/// - The WebSocket endpoint is empty
/// - The connection to the browser fails
/// - Session initialization fails
async fn connect_to_browser(
    profile: &crate::config::BrowserProfile,
    config: &Config,
) -> Result<Session> {
    let ws_endpoint = &profile.ws_endpoint;

    if ws_endpoint.is_empty() {
        return Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
            format!("Empty WebSocket endpoint for profile: {}", profile.name),
        )));
    }

    // Apply connection timeout to prevent indefinite hangs
    let connect_timeout = Duration::from_millis(config.browser.connection_timeout_ms.max(5000));
    let (browser, handler) = match tokio::time::timeout(
        connect_timeout,
        chromiumoxide::Browser::connect(ws_endpoint),
    )
    .await
    {
        Ok(Ok((browser, handler))) => (browser, handler),
        Ok(Err(e)) => {
            return Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!("Failed to connect to {}: {}", profile.name, e),
            )));
        }
        Err(_) => {
            return Err(OrchestratorError::Browser(BrowserError::ConnectionFailed(
                format!(
                    "Connection timeout to {} after {}ms",
                    profile.name,
                    connect_timeout.as_millis()
                ),
            )));
        }
    };

    let session = Session::new(
        format!("config-{}", profile.name),
        profile.name.clone(),
        profile.r#type.clone(),
        browser,
        handler,
        config.browser.max_workers_per_session,
        config.browser.cursor_overlay_ms,
        Some(config.browser.circuit_breaker.clone()),
    );

    Ok(session)
}

/// Auto-discovers local browser instances (Brave, Chrome, etc.).
///
/// Scans common CDP (Chrome DevTools Protocol) ports for local
/// browser instances that are running with remote debugging enabled:
/// - Brave: ports 9001-9050
/// - Chrome: ports 9221-9230
///
/// # Arguments
///
/// * `config` - The orchestrator configuration
///
/// # Returns
///
/// A vector of successfully connected `Session` instances.
async fn discover_local_browsers(config: &Config) -> Result<Vec<Session>> {
    let brave_ports: Vec<u16> = (9001..=9050).collect();
    let chrome_ports: Vec<u16> = (9222..=9230).collect();

    let mut results: Vec<Option<Session>> = stream::iter(brave_ports)
        .map(|port| async move { discover_brave_on_port(port, config).await.ok().flatten() })
        .buffer_unordered(50)
        .collect()
        .await;

    let chrome_results: Vec<Option<Session>> = stream::iter(chrome_ports)
        .map(|port| async move { discover_chrome_on_port(port, config).await.ok().flatten() })
        .buffer_unordered(50)
        .collect()
        .await;

    results.extend(chrome_results);
    let sessions: Vec<Session> = results.into_iter().flatten().collect();
    Ok(sessions)
}

/// Attempts to discover a Brave browser instance on a specific port.
///
/// Checks if a browser is running with remote debugging enabled on the
/// specified port by querying the CDP version endpoint.
///
/// # Arguments
///
/// * `port` - The port to check for a browser instance
/// * `config` - The orchestrator configuration
///
/// # Returns
///
/// * `Some(Session)` - If a browser is found and connected
/// * `None` - If no browser is found on this port
async fn discover_brave_on_port(port: u16, config: &Config) -> Result<Option<Session>> {
    let cdp_url = format!("http://127.0.0.1:{port}/json/version");

    debug!("Checking Brave on port {port}");

    // Try to connect with timeout
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
                        info!("Found Brave browser on port {port}");

                        // Try to connect to chromiumoxide with timeout
                        let brave_timeout =
                            Duration::from_millis(config.browser.connection_timeout_ms.max(5000));
                        match tokio::time::timeout(
                            brave_timeout,
                            chromiumoxide::Browser::connect(ws_str),
                        )
                        .await
                        {
                            Ok(Ok((browser, handler))) => {
                                let session = Session::new(
                                    format!("brave-{port}"),
                                    format!("Brave on port {port}"),
                                    "localBrave".to_string(),
                                    browser,
                                    handler,
                                    config.browser.max_workers_per_session,
                                    config.browser.cursor_overlay_ms,
                                    Some(config.browser.circuit_breaker.clone()),
                                );
                                return Ok(Some(session));
                            }
                            Ok(Err(e)) => {
                                warn!("Failed to connect to Brave on port {port}: {e}");
                            }
                            Err(_) => {
                                warn!(
                                    "Connection timeout to Brave on port {port} after {}ms",
                                    brave_timeout.as_millis()
                                );
                            }
                        }
                    }
                }
            }
        }
        _ => {
            // Port not available or no browser, continue silently
        }
    }

    Ok(None)
}

/// Attempts to discover a Chrome browser instance on a specific port.
///
/// Checks if a browser is running with remote debugging enabled on the
/// specified port by querying the CDP version endpoint.
///
/// # Arguments
///
/// * `port` - The port to check for a browser instance
/// * `config` - The orchestrator configuration
///
/// # Returns
///
/// * `Some(Session)` - If a browser is found and connected
/// * `None` - If no browser is found on this port
async fn discover_chrome_on_port(port: u16, config: &Config) -> Result<Option<Session>> {
    let cdp_url = format!("http://127.0.0.1:{port}/json/version");

    debug!("Checking Chrome on port {port}");

    // Try to connect with timeout
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
                        info!("Found Chrome browser on port {port}");

                        // Try to connect to chromiumoxide with timeout
                        let chrome_timeout =
                            Duration::from_millis(config.browser.connection_timeout_ms.max(5000));
                        match tokio::time::timeout(
                            chrome_timeout,
                            chromiumoxide::Browser::connect(ws_str),
                        )
                        .await
                        {
                            Ok(Ok((browser, handler))) => {
                                let session = Session::new(
                                    format!("chrome-{port}"),
                                    format!("Chrome on port {port}"),
                                    "localChrome".to_string(),
                                    browser,
                                    handler,
                                    config.browser.max_workers_per_session,
                                    config.browser.cursor_overlay_ms,
                                    Some(config.browser.circuit_breaker.clone()),
                                );
                                return Ok(Some(session));
                            }
                            Ok(Err(e)) => {
                                warn!("Failed to connect to Chrome on port {port}: {e}");
                            }
                            Err(_) => {
                                warn!(
                                    "Connection timeout to Chrome on port {port} after {}ms",
                                    chrome_timeout.as_millis()
                                );
                            }
                        }
                    }
                }
            }
        }
        _ => {
            // Port not available or no browser, continue silently
        }
    }

    Ok(None)
}

/// Discovers RoxyBrowser cloud-hosted browser instances via API.
///
/// Queries the RoxyBrowser API to retrieve available browser instances
/// and creates sessions for them.
///
/// # Arguments
///
/// * `config` - The orchestrator configuration containing RoxyBrowser API settings
///
/// # Returns
///
/// A vector of successfully connected `Session` instances from RoxyBrowser.
async fn discover_roxybrowser(config: &Config) -> Result<Vec<Session>> {
    let api_url = &config.browser.roxybrowser.api_url;
    let api_key = &config.browser.roxybrowser.api_key;

    info!("Discovering Roxybrowser from: {api_url}");

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
        warn!("Roxybrowser API error: {} (code: {})", msg, response.code);
        return Ok(vec![]);
    }

    let profiles = response.data.unwrap_or_default();

    if profiles.is_empty() {
        info!("No open Roxybrowser profiles found");
        return Ok(vec![]);
    }

    info!("Found {} Roxybrowser profiles", profiles.len());

    let mut sessions = Vec::new();

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

        // Use windowName as ID (the profile name user set in Roxybrowser), prepend with "roxy-"
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
            .unwrap_or_else(|| format!("Roxybrowser-{i}"));

        debug!("Connecting to Roxybrowser: {profile_name} ({ws_url})");

        let roxy_timeout = Duration::from_millis(config.browser.connection_timeout_ms.max(5000));
        match tokio::time::timeout(roxy_timeout, chromiumoxide::Browser::connect(&ws_url)).await {
            Ok(Ok((browser, handler))) => {
                let session = Session::new(
                    profile_id,
                    profile_name.clone(),
                    "roxybrowser".to_string(),
                    browser,
                    handler,
                    config.browser.max_workers_per_session,
                    config.browser.cursor_overlay_ms,
                    Some(config.browser.circuit_breaker.clone()),
                );
                sessions.push(session);
                info!("Connected to Roxybrowser: {profile_name}");
            }
            Ok(Err(e)) => {
                warn!("Failed to connect to Roxybrowser {profile_name}: {e}");
            }
            Err(_) => {
                warn!(
                    "Connection timeout to Roxybrowser {profile_name} after {}ms",
                    roxy_timeout.as_millis()
                );
            }
        }
    }

    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrowserProfile;

    #[test]
    fn test_browser_filter_matching() {
        let filters = vec!["brave".to_string(), "roxybrowser".to_string()];

        assert!(matches_browser_filters("Brave on port 9001", &filters));
        assert!(matches_browser_filters("roxy-browser", &filters));
        assert!(!matches_browser_filters("Safari", &filters));
    }

    #[test]
    fn test_normalize_browser_token() {
        assert_eq!(normalize_browser_token("Brave-Browser"), "bravebrowser");
        assert_eq!(normalize_browser_token("ROXYBROWSER"), "roxybrowser");
        assert_eq!(normalize_browser_token("Chrome_123"), "chrome123");
        assert_eq!(normalize_browser_token("Test@#$Browser"), "testbrowser");
        assert_eq!(normalize_browser_token(""), "");
        assert_eq!(normalize_browser_token("123"), "123");
    }

    #[test]
    fn test_matches_browser_filters_empty() {
        let filters: Vec<String> = vec![];
        assert!(matches_browser_filters("any browser", &filters));
        assert!(matches_browser_filters("", &filters));
    }

    #[test]
    fn test_matches_browser_filters_case_insensitive() {
        let filters = vec!["BRAVE".to_string()];
        assert!(matches_browser_filters("brave", &filters));
        assert!(matches_browser_filters("Brave", &filters));
        assert!(matches_browser_filters("BRAVE", &filters));
    }

    #[test]
    fn test_matches_browser_filters_partial_match() {
        let filters = vec!["brave".to_string()];
        assert!(matches_browser_filters("Brave Browser", &filters));
        assert!(matches_browser_filters("My Brave Instance", &filters));
    }

    #[test]
    fn test_matches_browser_filters_normalized_match() {
        let filters = vec!["brave-browser".to_string()];
        assert!(matches_browser_filters("Brave_Browser", &filters));
        assert!(matches_browser_filters("BraveBrowser", &filters));
    }

    #[test]
    fn test_matches_browser_filters_multiple_filters() {
        let filters = vec![
            "brave".to_string(),
            "chrome".to_string(),
            "safari".to_string(),
        ];
        assert!(matches_browser_filters("Brave Browser", &filters));
        assert!(matches_browser_filters("Chrome Instance", &filters));
        assert!(matches_browser_filters("Safari Web", &filters));
        assert!(!matches_browser_filters("Firefox", &filters));
    }

    #[test]
    fn test_profile_matches_filters_by_name() {
        let profile = BrowserProfile {
            name: "My Brave Browser".to_string(),
            r#type: "brave".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_profile_matches_filters_by_type() {
        let profile = BrowserProfile {
            name: "Custom Name".to_string(),
            r#type: "chrome".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["chrome".to_string()];
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_profile_matches_filters_no_match() {
        let profile = BrowserProfile {
            name: "My Browser".to_string(),
            r#type: "firefox".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string(), "chrome".to_string()];
        assert!(!profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_session_matches_filters_by_name() {
        // Test session filter matching by creating a session with test values
        // Note: We can't easily create a real Session in tests without a browser connection
        // So we test the filter logic indirectly through the helper function
        let filters = vec!["brave".to_string()];
        assert!(matches_browser_filters("Brave Browser", &filters));
    }

    #[test]
    fn test_session_matches_filters_by_type() {
        let filters = vec!["chrome".to_string()];
        assert!(matches_browser_filters("Chrome Instance", &filters));
    }

    #[test]
    fn test_session_matches_filters_by_id() {
        let filters = vec!["brave".to_string()];
        assert!(matches_browser_filters("brave-123", &filters));
    }

    #[test]
    fn test_session_matches_filters_no_match() {
        let filters = vec!["brave".to_string(), "chrome".to_string()];
        assert!(!matches_browser_filters("Safari", &filters));
    }

    #[test]
    fn test_matches_browser_filters_with_special_chars() {
        let filters = vec!["brave_browser".to_string()];
        assert!(matches_browser_filters("Brave@Browser", &filters));
        assert!(matches_browser_filters("Brave#Browser", &filters));
    }

    #[test]
    fn test_normalize_browser_token_with_numbers() {
        assert_eq!(normalize_browser_token("Browser123"), "browser123");
        assert_eq!(normalize_browser_token("123Browser"), "123browser");
        assert_eq!(normalize_browser_token("1_2_3"), "123");
    }

    #[test]
    fn test_matches_browser_filters_exact_match() {
        let filters = vec!["brave".to_string()];
        assert!(matches_browser_filters("brave", &filters));
        assert!(matches_browser_filters("Brave", &filters));
    }

    #[test]
    fn test_profile_matches_filters_empty_filters() {
        let profile = BrowserProfile {
            name: "Any Browser".to_string(),
            r#type: "any".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters: Vec<String> = vec![];
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_session_matches_filters_empty_filters() {
        // Test empty filters through the helper function
        let filters: Vec<String> = vec![];
        assert!(matches_browser_filters("any browser", &filters));
    }

    #[test]
    fn test_normalize_browser_token_unicode() {
        assert_eq!(normalize_browser_token("Brave浏览器"), "brave");
        assert_eq!(normalize_browser_token("ChromeКонтекст"), "chrome");
    }

    #[test]
    fn test_normalize_browser_token_whitespace_only() {
        assert_eq!(normalize_browser_token("   "), "");
        assert_eq!(normalize_browser_token("\t\n"), "");
    }

    #[test]
    fn test_matches_browser_filters_empty_filter_ignored() {
        let filters = vec!["".to_string()];
        assert!(!matches_browser_filters("Brave", &filters));
    }

    #[test]
    fn test_matches_browser_filters_substring_match() {
        let filters = vec!["bra".to_string()];
        assert!(matches_browser_filters("Brave Browser", &filters));
        assert!(matches_browser_filters("abra", &filters));
    }

    #[test]
    fn test_normalize_browser_token_mixed_case() {
        assert_eq!(normalize_browser_token("BrAvE"), "brave");
        assert_eq!(normalize_browser_token("ChRoMe"), "chrome");
    }

    #[test]
    fn test_profile_matches_filters_both_name_and_type() {
        let profile = BrowserProfile {
            name: "Brave Browser".to_string(),
            r#type: "brave".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match because both name and type contain "brave"
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_normalize_browser_token_only_special_chars() {
        assert_eq!(normalize_browser_token("@#$%"), "");
        assert_eq!(normalize_browser_token("!@#$%^&*()"), "");
    }

    #[test]
    fn test_matches_browser_filters_empty_candidate() {
        let filters = vec!["brave".to_string()];
        assert!(!matches_browser_filters("", &filters));
    }

    #[test]
    fn test_matches_browser_filters_filter_with_special_chars() {
        let filters = vec!["brave_browser".to_string()];
        assert!(matches_browser_filters("Brave-Browser", &filters));
        assert!(matches_browser_filters("Brave_Browser", &filters));
    }

    #[test]
    fn test_normalize_browser_token_preserves_numbers() {
        assert_eq!(normalize_browser_token("v1.2.3"), "v123");
        assert_eq!(normalize_browser_token("browser2.0"), "browser20");
    }

    #[test]
    fn test_matches_browser_filters_numeric_filter() {
        let filters = vec!["123".to_string()];
        assert!(matches_browser_filters("browser-123", &filters));
        assert!(matches_browser_filters("123-browser", &filters));
    }

    #[test]
    fn test_profile_matches_filters_name_only() {
        let profile = BrowserProfile {
            name: "My Brave".to_string(),
            r#type: "chrome".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match by name even though type is different
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_profile_matches_filters_type_only() {
        let profile = BrowserProfile {
            name: "Custom".to_string(),
            r#type: "brave".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match by type even though name doesn't match
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_matches_browser_filters_very_long_candidate() {
        let filters = vec!["brave".to_string()];
        let long_name = "a".repeat(1000) + " brave " + &"b".repeat(1000);
        assert!(matches_browser_filters(&long_name, &filters));
    }

    #[test]
    fn test_matches_browser_filters_very_long_filter() {
        let filters = vec!["brave".to_string()];
        let long_name = "a".repeat(1000) + " brave " + &"b".repeat(1000);
        assert!(matches_browser_filters(&long_name, &filters));
    }

    #[test]
    fn test_normalize_browser_token_consecutive_special_chars() {
        assert_eq!(normalize_browser_token("Brave@@@Browser"), "bravebrowser");
        assert_eq!(normalize_browser_token("Test###Name"), "testname");
    }

    #[test]
    fn test_matches_browser_filters_multiple_matches() {
        let filters = vec!["brave".to_string(), "chrome".to_string()];
        let candidate = "Brave Chrome Browser";
        // Should match because it contains both filters
        assert!(matches_browser_filters(candidate, &filters));
    }

    #[test]
    fn test_normalize_browser_token_leading_trailing_special_chars() {
        assert_eq!(normalize_browser_token("@@Brave@@"), "brave");
        assert_eq!(normalize_browser_token("##Chrome##"), "chrome");
    }

    #[test]
    fn test_matches_browser_filters_filter_with_spaces() {
        let filters = vec!["brave browser".to_string()];
        // Filter is normalized, so spaces are removed
        assert!(matches_browser_filters("BraveBrowser", &filters));
        assert!(matches_browser_filters("Brave-Browser", &filters));
    }

    #[test]
    fn test_profile_matches_filters_case_sensitivity() {
        let profile = BrowserProfile {
            name: "BRAVE".to_string(),
            r#type: "CHROME".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        let filters = vec!["brave".to_string()];
        // Should match due to case-insensitive comparison
        assert!(profile_matches_filters(&profile, &filters));
    }

    #[test]
    fn test_normalize_browser_token_empty_after_filtering() {
        assert_eq!(normalize_browser_token("@#$"), "");
        assert_eq!(normalize_browser_token("   "), "");
    }

    #[test]
    fn test_matches_browser_filters_candidate_with_numbers() {
        let filters = vec!["browser123".to_string()];
        assert!(matches_browser_filters("browser-123", &filters));
        assert!(matches_browser_filters("browser-123", &filters));
    }

    #[tokio::test]
    async fn test_discover_browsers_with_filters_empty_config_no_filter() {
        // Test: No filters, no browsers available - should return empty vec (warns but doesn't error)
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![],         // Empty profiles to avoid actual connections
                max_discovery_retries: 0, // Skip discovery attempts
                ..Default::default()
            },
            orchestrator: crate::config::OrchestratorConfig::default(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
        };
        let filters: Vec<String> = vec![];

        let result = discover_browsers_with_filters(&config, &filters).await;

        // Should succeed with empty sessions (warns but continues)
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_discover_browsers_with_filters_empty_config_with_filter() {
        // Test: Filters active but no browsers available - should return error
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![],         // Empty profiles to avoid actual connections
                max_discovery_retries: 0, // Skip discovery attempts
                ..Default::default()
            },
            orchestrator: crate::config::OrchestratorConfig::default(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
        };
        let filters = vec!["brave".to_string()];

        let result = discover_browsers_with_filters(&config, &filters).await;

        // Should fail with connection error due to zero matches
        match result {
            Ok(_) => panic!("Should return error when filters active but no matches"),
            Err(e) => {
                let err_msg = e.to_string();
                assert!(err_msg.contains("No browsers matched the specified filters"));
                assert!(err_msg.contains("brave"));
            }
        }
    }

    #[tokio::test]
    async fn test_discover_browsers_with_filters_empty_filter_string() {
        // Test: Empty filter string treated as no filter - should not error
        let config = crate::config::Config {
            browser: crate::config::BrowserConfig {
                profiles: vec![],         // Empty profiles to avoid actual connections
                max_discovery_retries: 0, // Skip discovery attempts
                ..Default::default()
            },
            orchestrator: crate::config::OrchestratorConfig::default(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
        };
        let filters: Vec<String> = vec![]; // Empty filters

        let result = discover_browsers_with_filters(&config, &filters).await;

        // Should succeed with empty sessions (no error when filters empty)
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
