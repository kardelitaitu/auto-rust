//! Default implementations for config types that reference sibling types.
//!
//! These Default impls are separated from types.rs because they reference
//! other config struct types (e.g., `BrowserConfig::default()` calls
//! `CircuitBreakerConfig::default()`). Kept in a sibling module to avoid
//! import ordering issues.

use crate::session::DurationMs;
use std::collections::BTreeMap;

use super::types::{
    BraveConfig, BrowserConfig, BrowserProfile, ChromeConfig, CircuitBreakerConfig,
    EngagementLimitsConfig, IxbrowserConfig, NativeInteractionConfig, OrchestratorConfig,
    RoxybrowserConfig, ShardbrowserConfig, TwitterActivityConfig, TwitterLLMConfig,
    TwitterProbabilitiesConfig,
};

impl Default for TwitterActivityConfig {
    fn default() -> Self {
        Self {
            feed_scan_duration_ms: super::types::default_feed_scan_duration(),
            feed_scroll_count: super::types::default_feed_scroll_count(),
            engagement_candidate_count: super::types::default_engagement_candidate_count(),
            scroll_amount_pixels: super::types::default_twitter_scroll_amount(),
            candidate_scan_interval_ms: super::types::default_candidate_scan_interval_ms(),
            max_consecutive_scroll_failures: super::types::default_max_consecutive_scroll_failures(
            ),
            max_consecutive_empty_scans: super::types::default_max_consecutive_empty_scans(),
            persona_file_path: None,
            probabilities: TwitterProbabilitiesConfig::default(),
            engagement_limits: EngagementLimitsConfig::default(),
            llm: TwitterLLMConfig::default(),
            persistence_enabled: false,
        }
    }
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            connection_timeout_ms: DurationMs::new_const(30000),
            max_discovery_retries: 3,
            discovery_retry_delay_ms: DurationMs::new_const(500),
            circuit_breaker: CircuitBreakerConfig::default(),
            profiles: vec![],
            roxybrowser: RoxybrowserConfig::default(),
            ixbrowser: IxbrowserConfig::default(),
            shardbrowser: ShardbrowserConfig::default(),
            chrome: ChromeConfig::default(),
            brave: BraveConfig::default(),
            user_agent: None,
            extra_http_headers: BTreeMap::new(),
            cursor_overlay_ms: 0,
            cursor_overlay_color: "#ff6600".to_string(),
            cursor_overlay_show_trail: true,
            native_interaction: NativeInteractionConfig::default(),
            max_workers_per_session: 5,
            enable_learning_persistence: true,
            learning_ttl_days: 30,
            random_screen_size_brave_and_chrome: true,
        }
    }
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_global_concurrency: 5,
            task_timeout_ms: DurationMs::new_const(60000),
            group_timeout_ms: DurationMs::new_const(300000),
            worker_wait_timeout_ms: DurationMs::new_const(10000),
            task_stagger_delay_ms: 500,
            max_retries: 3,
            retry_delay_ms: DurationMs::new_const(2000),
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: 5,
            success_threshold: 3,
            half_open_time_ms: DurationMs::new_const(30000),
        }
    }
}

impl Default for RoxybrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: "http://localhost:4444".to_string(),
            api_key: String::new(),
        }
    }
}

impl Default for IxbrowserConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_url: "http://127.0.0.1:53200".to_string(),
        }
    }
}

impl Default for ShardbrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: "http://127.0.0.1:40325".to_string(),
            api_key: String::new(),
        }
    }
}

impl Default for BrowserProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            r#type: "chrome".to_string(),
            ws_endpoint: String::new(),
        }
    }
}
