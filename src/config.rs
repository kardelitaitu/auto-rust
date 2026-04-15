use anyhow::Result;
use serde::Deserialize;
use std::env;

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
}

pub fn load_config() -> Result<Config> {
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
        },
    })
}
