//! Browser discovery, connection, and management module.
//!
//! Handles:
//! - Browser profile discovery from configuration
//! - Establishing WebSocket connections to browser instances
//! - Managing browser lifecycle and health checks
//! - Integration with RoxyBrowser API for cloud-hosted browsers

use crate::config::Config;
use crate::session::Session;
use anyhow::{Result, bail};
use log::{info, debug, warn};
use std::time::Duration;
use futures::stream::{self, StreamExt};

pub async fn discover_browsers(config: &Config) -> Result<Vec<Session>> {
    let mut sessions = Vec::new();

    info!("Starting browser discovery...");

    for attempt in 1..=config.browser.max_discovery_retries {
        info!("Discovery attempt {}/{}", attempt, config.browser.max_discovery_retries);

        // Try configured profiles
        for profile in &config.browser.profiles {
            match connect_to_browser(profile, config).await {
                Ok(session) => {
                    info!("Connected to configured browser: {}", profile.name);
                    sessions.push(session);
                }
                Err(e) => {
                    debug!("Failed to connect to {}: {}", profile.name, e);
                }
            }
        }

        // Try Roxybrowser discovery (always enabled)
        let roxy_sessions = discover_roxybrowser(config).await?;
        for session in roxy_sessions {
            info!("Discovered Roxybrowser: {}", session.name);
            sessions.push(session);
        }

        // Try auto-discovery for local browsers
        let discovered_sessions = discover_local_browsers(config).await?;
        for session in discovered_sessions {
            info!("Auto-discovered browser: {}", session.name);
            sessions.push(session);
        }

        if !sessions.is_empty() {
            break;
        }

        if attempt < config.browser.max_discovery_retries {
            tokio::time::sleep(
                std::time::Duration::from_millis(config.browser.discovery_retry_delay_ms)
            ).await;
        }
    }

    Ok(sessions)
}

async fn connect_to_browser(profile: &crate::config::BrowserProfile, _config: &Config) -> Result<Session> {
    let ws_endpoint = &profile.ws_endpoint;
    
    if ws_endpoint.is_empty() {
        bail!("Empty WebSocket endpoint for profile: {}", profile.name);
    }
    
    info!("Connecting to configured browser: {}", profile.name);
    
    let (browser, handler) = chromiumoxide::Browser::connect(ws_endpoint).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}: {}", profile.name, e))?;
    
    let session = Session::new(
        format!("config-{}", profile.name),
        profile.name.clone(),
        profile.r#type.clone(),
        browser,
        handler,
        5, // max_workers
    );
    
    info!("Connected to configured browser: {}", profile.name);
    Ok(session)
}

/// Auto-discover local browsers (Brave, Chrome, etc.)
async fn discover_local_browsers(config: &Config) -> Result<Vec<Session>> {
    info!("Scanning for local Brave browsers...");

    let ports: Vec<u16> = (9001..=9050).collect();

    let results: Vec<Option<Session>> = stream::iter(ports)
        .map(|port| async move {
            discover_brave_on_port(port, config).await.ok().flatten()
        })
        .buffer_unordered(50)
        .collect()
        .await;

    let sessions: Vec<Session> = results.into_iter().flatten().collect();
    Ok(sessions)
}

/// Discover a Brave browser instance on a specific port
async fn discover_brave_on_port(port: u16, _config: &Config) -> Result<Option<Session>> {
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

                        // Try to connect to chromiumoxide
                        match chromiumoxide::Browser::connect(ws_str).await {
                            Ok((browser, handler)) => {
                                let session = Session::new(
                                    format!("brave-{port}"),
                                    format!("Brave on port {port}"),
                                    "localBrave".to_string(),
                                    browser,
                                    handler,
                                    5, // max_workers
                                );
                                return Ok(Some(session));
                            }
                            Err(e) => {
                                warn!("Failed to connect to Brave on port {port}: {e}");
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

/// Discover Roxybrowser instances via API
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
    
    let response: RoxyResponse = client.get_with_key("browser/connection_info", api_key).await?;

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
        let ws_url = profile.get("ws")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);

        let http_url = profile.get("http")
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
        let profile_id = profile.get("windowName")
            .and_then(|w| w.as_str())
            .map(|s| format!("roxy-{s}"))
            .unwrap_or_else(|| format!("roxy-{i}"));

        let profile_name = profile.get("name")
            .or_else(|| profile.get("windowName"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Roxybrowser-{i}"));

        debug!("Connecting to Roxybrowser: {profile_name} ({ws_url})");

        match chromiumoxide::Browser::connect(&ws_url).await {
            Ok((browser, handler)) => {
                let session = Session::new(
                    profile_id,
                    profile_name.clone(),
                    "roxybrowser".to_string(),
                    browser,
                    handler,
                    5,
                );
                sessions.push(session);
                info!("Connected to Roxybrowser: {profile_name}");
            }
            Err(e) => {
                warn!("Failed to connect to Roxybrowser {profile_name}: {e}");
            }
        }
    }

    Ok(sessions)
}
