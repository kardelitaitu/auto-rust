//! Configuration management module.
//!
//! Handles loading, validating, and applying configuration settings for the orchestrator.
//! Supports TOML configuration files with environment variable overrides.

use anyhow::{bail, Result};
use log::{info, warn};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::path::Path;

/// Top-level configuration structure for the Rust Orchestrator.
/// Contains all settings for browser connections, task orchestration, and system behavior.
/// Configuration can be loaded from TOML files and overridden with environment variables.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Browser connection and discovery settings
    pub browser: BrowserConfig,
    /// Task orchestration and concurrency settings
    pub orchestrator: OrchestratorConfig,
}

/// Configuration for browser connections and management.
/// Defines how the orchestrator discovers, connects to, and manages browser instances.
#[derive(Debug, Deserialize, Clone)]
pub struct BrowserConfig {
    /// List of browser connector types to use (currently unused)
    #[allow(dead_code)]
    pub connectors: Vec<String>,
    /// Timeout for establishing browser connections in milliseconds (currently unused)
    #[allow(dead_code)]
    pub connection_timeout_ms: u64,
    /// Maximum number of attempts to discover available browsers
    pub max_discovery_retries: u32,
    /// Delay between browser discovery attempts in milliseconds
    pub discovery_retry_delay_ms: u64,
    /// Circuit breaker configuration for fault tolerance (currently unused)
    #[allow(dead_code)]
    pub circuit_breaker: CircuitBreakerConfig,
    /// List of browser profiles available for task execution
    pub profiles: Vec<BrowserProfile>,
    /// RoxyBrowser API integration settings
    pub roxybrowser: RoxybrowserConfig,
    /// Optional browser user-agent override applied to new pages
    #[serde(default)]
    pub user_agent: Option<String>,
    /// Optional extra HTTP headers applied to new pages
    #[serde(default)]
    pub extra_http_headers: BTreeMap<String, String>,
}

/// Configuration for circuit breaker pattern implementation.
/// Circuit breakers prevent cascading failures by temporarily stopping requests
/// to services that are failing repeatedly.
#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct CircuitBreakerConfig {
    /// Whether the circuit breaker is enabled
    pub enabled: bool,
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes needed to close the circuit
    pub success_threshold: u32,
    /// Time to wait before trying to close the circuit again (in milliseconds)
    pub half_open_time_ms: u64,
}

/// Defines a browser profile for task execution.
/// Each profile represents a browser instance that can be used to run tasks.
#[derive(Debug, Deserialize, Clone)]
pub struct BrowserProfile {
    /// Human-readable name for this browser profile
    pub name: String,
    /// Type of browser (e.g., "chrome", "brave", "firefox") (currently unused)
    #[allow(dead_code)]
    pub r#type: String,
    /// WebSocket endpoint URL for connecting to the browser (currently unused)
    #[allow(dead_code)]
    pub ws_endpoint: String,
}

/// Configuration for RoxyBrowser API integration.
/// RoxyBrowser provides cloud-hosted browser instances for automation tasks.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct RoxybrowserConfig {
    /// Whether RoxyBrowser integration is enabled
    pub enabled: bool,
    /// Base URL for the RoxyBrowser API
    pub api_url: String,
    /// API authentication key for RoxyBrowser
    pub api_key: String,
}

/// Configuration for task orchestration and execution behavior.
/// Controls concurrency, timeouts, and retry policies for the orchestrator.
#[derive(Debug, Deserialize, Clone)]
pub struct OrchestratorConfig {
    /// Maximum number of tasks that can run concurrently across all sessions
    pub max_global_concurrency: usize,
    /// Timeout for individual task execution in milliseconds
    pub task_timeout_ms: u64,
    /// Timeout for task groups (sequential tasks) in milliseconds
    pub group_timeout_ms: u64,
    /// Timeout for acquiring a worker/session for task execution
    pub worker_wait_timeout_ms: u64,
    /// Threshold for detecting stuck workers in milliseconds
    #[allow(dead_code)]
    pub stuck_worker_threshold_ms: u64,
    /// Delay between starting consecutive tasks in milliseconds
    pub task_stagger_delay_ms: u64,
    /// Maximum number of retry attempts for failed tasks
    pub max_retries: u32,
    /// Delay between retry attempts in milliseconds
    pub retry_delay_ms: u64,
}

/// Loads configuration from file and environment variables.
/// Attempts to load from `config/default.toml` first, then applies environment
/// variable overrides. Falls back to hardcoded defaults if no config file exists.
///
/// # Environment Variables
/// - `ROXYBROWSER_API_URL`: Override the RoxyBrowser API URL
/// - `ROXYBROWSER_API_KEY`: Override the RoxyBrowser API key
///
/// # Returns
/// A complete Config struct with all settings resolved
pub fn load_config() -> Result<Config> {
    // Try to load from config/default.toml first
    let config_path = Path::new("config/default.toml");

    if config_path.exists() {
        info!("Loading config from {}", config_path.display());
        let content = std::fs::read_to_string(config_path)?;
        let file_config: Config = toml::from_str(&content)?;

        // Apply environment variable overrides
        return apply_env_overrides(file_config);
    }

    // Fall back to code-based config with env overrides
    apply_env_overrides(load_code_config()?)
}

fn load_code_config() -> Result<Config> {
    let roxybrowser_url =
        env::var("ROXYBROWSER_API_URL").unwrap_or_else(|_| "http://127.0.0.1:50000/".to_string());
    let roxybrowser_key = env::var("ROXYBROWSER_API_KEY")
        .unwrap_or_else(|_| "c6ae203adfe0327a63ccc9174c178dec".to_string());

    Ok(Config {
        browser: BrowserConfig {
            connectors: vec![],
            connection_timeout_ms: 10000,
            max_discovery_retries: 3,
            discovery_retry_delay_ms: 5000,
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 5,
                success_threshold: 3,
                half_open_time_ms: 30000,
            },
            profiles: vec![],
            roxybrowser: RoxybrowserConfig {
                enabled: true,
                api_url: roxybrowser_url,
                api_key: roxybrowser_key,
            },
            user_agent: None,
            extra_http_headers: BTreeMap::new(),
        },
        orchestrator: OrchestratorConfig {
            max_global_concurrency: 20,
            task_timeout_ms: 600_000,
            group_timeout_ms: 600_000,
            worker_wait_timeout_ms: 10000,
            stuck_worker_threshold_ms: 120_000,
            task_stagger_delay_ms: 2000,
            max_retries: 2,
            retry_delay_ms: 500,
        },
    })
}

fn apply_env_overrides(mut config: Config) -> Result<Config> {
    // Environment variable overrides
    if let Ok(url) = env::var("ROXYBROWSER_API_URL") {
        config.browser.roxybrowser.api_url = url;
    }
    if let Ok(key) = env::var("ROXYBROWSER_API_KEY") {
        config.browser.roxybrowser.api_key = key;
    }
    if let Ok(user_agent) = env::var("BROWSER_USER_AGENT") {
        config.browser.user_agent = Some(user_agent);
    }
    if let Ok(concurrency) = env::var("MAX_GLOBAL_CONCURRENCY") {
        config.orchestrator.max_global_concurrency = concurrency
            .parse()
            .unwrap_or(config.orchestrator.max_global_concurrency);
    }
    if let Ok(timeout) = env::var("TASK_TIMEOUT_MS") {
        config.orchestrator.task_timeout_ms = timeout
            .parse()
            .unwrap_or(config.orchestrator.task_timeout_ms);
    }
    if let Ok(retries) = env::var("MAX_RETRIES") {
        config.orchestrator.max_retries =
            retries.parse().unwrap_or(config.orchestrator.max_retries);
    }
    if let Ok(raw_headers) = env::var("BROWSER_EXTRA_HTTP_HEADERS") {
        config.browser.extra_http_headers = raw_headers
            .split(';')
            .filter_map(|pair| pair.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect();
    }

    Ok(config)
}

/// Validates configuration settings to ensure they are reasonable and safe.
/// Performs startup-time validation of critical configuration values to prevent
/// runtime issues. Logs warnings for potentially problematic values.
///
/// # Arguments
/// * `config` - The configuration to validate
///
/// # Returns
/// Ok(()) if validation passes, Err if critical issues are found
///
/// # Errors
/// Returns an error if any required values are zero or invalid
pub fn validate_config(config: &Config) -> Result<()> {
    if config.orchestrator.max_global_concurrency == 0 {
        bail!("max_global_concurrency must be > 0");
    }
    if config.orchestrator.task_timeout_ms == 0 {
        bail!("task_timeout_ms must be > 0");
    }
    if config.orchestrator.group_timeout_ms == 0 {
        bail!("group_timeout_ms must be > 0");
    }
    if config.orchestrator.max_retries > 10 {
        warn!("max_retries > 10 may cause long running times");
    }

    info!("Config validation passed");
    Ok(())
}
