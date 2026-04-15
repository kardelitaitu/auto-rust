use anyhow::{bail, Result};
use log::{info, warn};
use serde::Deserialize;
use std::env;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub browser: BrowserConfig,
    pub orchestrator: OrchestratorConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BrowserConfig {
    #[allow(dead_code)]
    pub connectors: Vec<String>,
    #[allow(dead_code)]
    pub connection_timeout_ms: u64,
    pub max_discovery_retries: u32,
    pub discovery_retry_delay_ms: u64,
    #[allow(dead_code)]
    pub circuit_breaker: CircuitBreakerConfig,
    pub profiles: Vec<BrowserProfile>,
    pub roxybrowser: RoxybrowserConfig,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub half_open_time_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BrowserProfile {
    pub name: String,
    #[allow(dead_code)]
    pub r#type: String,
    #[allow(dead_code)]
    pub ws_endpoint: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct RoxybrowserConfig {
    pub enabled: bool,
    pub api_url: String,
    pub api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OrchestratorConfig {
    pub max_global_concurrency: usize,
    pub task_timeout_ms: u64,
    pub group_timeout_ms: u64,
    pub worker_wait_timeout_ms: u64,
    #[allow(dead_code)]
    pub stuck_worker_threshold_ms: u64,
    pub task_stagger_delay_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
}

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
        },
        orchestrator: OrchestratorConfig {
            max_global_concurrency: 20,
            task_timeout_ms: 600000,
            group_timeout_ms: 600000,
            worker_wait_timeout_ms: 10000,
            stuck_worker_threshold_ms: 120000,
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

    Ok(config)
}

/// Validate configuration at startup
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
