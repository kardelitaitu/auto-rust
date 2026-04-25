//! Configuration management module.
//!
//! Handles loading, validating, and applying configuration settings for the orchestrator.
//! Supports TOML configuration files with environment variable overrides.

use crate::error::{ConfigError, OrchestratorError, Result};
use log::{info, warn};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

/// Top-level configuration structure for the Rust Orchestrator.
/// Contains all settings for browser connections, task orchestration, and system behavior.
/// Configuration can be loaded from TOML files with environment variable overrides.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// Browser connection and discovery settings
    pub browser: BrowserConfig,
    /// Task orchestration and concurrency settings
    pub orchestrator: OrchestratorConfig,
    /// Twitter activity task configuration
    #[serde(default)]
    pub twitter_activity: TwitterActivityConfig,
    /// OpenTelemetry tracing configuration
    #[serde(default)]
    pub tracing: TracingConfig,
}

/// Configuration for browser connections and management.
/// Defines how the orchestrator discovers, connects to, and manages browser instances.
///
/// # Future Use
/// - `connectors`: Reserved for additional browser connector types beyond RoxyBrowser
/// - `connection_timeout_ms`: Not currently used - chromiumoxide::Browser::connect has no timeout param
#[derive(Debug, Deserialize, Clone)]
pub struct BrowserConfig {
    /// List of browser connector types to use
    #[allow(dead_code)]
    pub connectors: Vec<String>,
    /// Timeout for establishing browser connections in milliseconds
    /// Note: Not currently applied - chromiumoxide::Browser::connect() has no timeout
    #[allow(dead_code)]
    pub connection_timeout_ms: u64,
    /// Maximum number of attempts to discover available browsers
    pub max_discovery_retries: u32,
    /// Delay between browser discovery attempts in milliseconds
    pub discovery_retry_delay_ms: u64,
    /// Circuit breaker configuration for fault tolerance.
    /// Note: Validated but not yet wired to ApiClient. Future use for API fault tolerance.
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
    /// Cursor overlay sync interval in milliseconds (0 = disabled)
    #[serde(default)]
    pub cursor_overlay_ms: u64,
    /// Native input interaction settings for OS-level cursor actions
    #[serde(default)]
    pub native_interaction: NativeInteractionConfig,
    /// Maximum concurrent pages/workers per session
    #[serde(default = "default_max_workers_per_session")]
    pub max_workers_per_session: usize,
}

/// Calibration mode for native cursor and click coordinate mapping.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum NativeClickCalibrationMode {
    #[default]
    Windows,
    Mac,
    Linux,
}

impl NativeClickCalibrationMode {
    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "mac" | "darwin" | "osx" => Self::Mac,
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            other => {
                warn!(
                    "Invalid native click calibration mode '{}', falling back to windows",
                    other
                );
                Self::Windows
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Mac => "mac",
            Self::Linux => "linux",
        }
    }
}

/// Backend used for native OS input dispatch.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum NativeInputBackend {
    #[default]
    Enigo,
    Sendinput,
    Rdev,
}

impl NativeInputBackend {
    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "enigo" => Self::Enigo,
            "sendinput" | "send_input" | "win32" => Self::Sendinput,
            "rdev" => Self::Rdev,
            other => {
                warn!(
                    "Invalid native input backend '{}', falling back to enigo",
                    other
                );
                Self::Enigo
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enigo => "enigo",
            Self::Sendinput => "sendinput",
            Self::Rdev => "rdev",
        }
    }
}

/// Configuration for native input interactions.
#[derive(Debug, Deserialize, Clone)]
pub struct NativeInteractionConfig {
    #[serde(default)]
    pub calibration_mode: NativeClickCalibrationMode,
    #[serde(default)]
    pub native_input_backend: NativeInputBackend,
    #[serde(default = "default_native_interaction_stability_wait_ms")]
    pub stability_wait_ms: u64,
    #[serde(default = "default_native_interaction_resolve_timeout_ms")]
    pub resolve_timeout_ms: u64,
    #[serde(default = "default_native_interaction_settle_ms")]
    pub settle_ms: u64,
}

impl Default for NativeInteractionConfig {
    fn default() -> Self {
        Self {
            calibration_mode: NativeClickCalibrationMode::default(),
            native_input_backend: NativeInputBackend::default(),
            stability_wait_ms: default_native_interaction_stability_wait_ms(),
            resolve_timeout_ms: default_native_interaction_resolve_timeout_ms(),
            settle_ms: default_native_interaction_settle_ms(),
        }
    }
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
    /// Type of browser (e.g., "chrome", "brave", "firefox")
    pub r#type: String,
    /// WebSocket endpoint URL for connecting to the browser
    pub ws_endpoint: String,
}

/// Configuration for RoxyBrowser API integration.
/// RoxyBrowser provides cloud-hosted browser instances for automation tasks.
#[derive(Debug, Deserialize, Clone)]
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

/// Configuration for the Twitter/X activity task.
#[derive(Debug, Deserialize, Clone)]
pub struct TwitterActivityConfig {
    /// Duration of feed scanning phase in milliseconds (default: 60s)
    #[serde(default = "default_feed_scan_duration")]
    pub feed_scan_duration_ms: u64,
    /// Number of scroll actions during feed scanning (default: 10)
    #[serde(default = "default_feed_scroll_count")]
    pub feed_scroll_count: u32,
    /// Number of tweets to consider for engagement per scan (default: 5)
    #[serde(default = "default_engagement_candidate_count")]
    pub engagement_candidate_count: u32,
    /// Scroll amount override for Twitter activity (pixels, 0 = use profile default)
    #[serde(default = "default_twitter_scroll_amount")]
    pub scroll_amount_pixels: i32,
    /// Candidate scan interval override (ms, 0 = use default 2500ms)
    #[serde(default = "default_candidate_scan_interval_ms")]
    pub candidate_scan_interval_ms: u64,
    /// Path to persona file (optional)
    #[serde(default)]
    pub persona_file_path: Option<String>,

    /// Default engagement probabilities (can be overridden by task payload)
    #[serde(default)]
    pub probabilities: TwitterProbabilitiesConfig,

    /// Engagement limits for rate limit protection
    #[serde(default)]
    pub engagement_limits: EngagementLimitsConfig,

    /// LLM configuration for V2 features
    #[serde(default)]
    pub llm: TwitterLLMConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TwitterProbabilitiesConfig {
    #[serde(default = "default_like_probability")]
    pub like_probability: f64,
    #[serde(default = "default_retweet_probability")]
    pub retweet_probability: f64,
    #[serde(default = "default_quote_probability")]
    pub quote_probability: f64,
    #[serde(default = "default_follow_probability")]
    pub follow_probability: f64,
    #[serde(default = "default_reply_probability")]
    pub reply_probability: f64,
    #[serde(default = "default_bookmark_probability")]
    pub bookmark_probability: f64,
    #[serde(default = "default_thread_dive_probability")]
    pub thread_dive_probability: f64,
}

impl Default for TwitterProbabilitiesConfig {
    fn default() -> Self {
        Self {
            like_probability: default_like_probability(),
            retweet_probability: default_retweet_probability(),
            quote_probability: default_quote_probability(),
            follow_probability: default_follow_probability(),
            reply_probability: default_reply_probability(),
            bookmark_probability: default_bookmark_probability(),
            thread_dive_probability: default_thread_dive_probability(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TwitterLLMConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_temperature")]
    pub temperature: f64,
    #[serde(default = "default_llm_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_llm_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_reply_probability")]
    pub reply_probability: f64,
    #[serde(default = "default_quote_probability")]
    pub quote_tweet_probability: f64,
}

fn default_llm_provider() -> String {
    "ollama".to_string()
}
fn default_llm_model() -> String {
    "llama3.2:latest".to_string()
}
fn default_llm_temperature() -> f64 {
    0.7
}
fn default_llm_max_tokens() -> u32 {
    100
}
fn default_llm_timeout() -> u64 {
    30000
}

// Default engagement probabilities
fn default_like_probability() -> f64 {
    0.4
}
fn default_retweet_probability() -> f64 {
    0.15
}
fn default_follow_probability() -> f64 {
    0.05
}
fn default_bookmark_probability() -> f64 {
    0.02
}
fn default_thread_dive_probability() -> f64 {
    0.25
}
fn default_reply_probability() -> f64 {
    0.05
}
fn default_quote_probability() -> f64 {
    0.15
}

/// OpenTelemetry tracing configuration.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct TracingConfig {
    /// Whether OpenTelemetry tracing is enabled
    #[serde(default)]
    pub enabled: bool,
    /// OTLP endpoint URL (e.g., "http://localhost:4317")
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    /// Service name for tracing
    #[serde(default = "default_service_name")]
    pub service_name: String,
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}

fn default_service_name() -> String {
    "auto".to_string()
}

/// Engagement limits configuration for Twitter automation.
/// Prevents rate limits and account restrictions by capping actions per session.
#[derive(Debug, Deserialize, Clone)]
pub struct EngagementLimitsConfig {
    /// Maximum likes per session (default: 5)
    #[serde(default = "default_max_likes")]
    pub max_likes: u32,
    /// Maximum retweets per session (default: 3)
    #[serde(default = "default_max_retweets")]
    pub max_retweets: u32,
    /// Maximum follows per session (default: 2)
    #[serde(default = "default_max_follows")]
    pub max_follows: u32,
    /// Maximum replies per session (default: 1)
    #[serde(default = "default_max_replies")]
    pub max_replies: u32,
    /// Maximum thread dives per session (default: 3)
    #[serde(default = "default_max_thread_dives")]
    pub max_thread_dives: u32,
    /// Maximum bookmarks per session (default: 0, disabled in V1)
    #[serde(default = "default_max_bookmarks")]
    pub max_bookmarks: u32,
    /// Maximum quote tweets per session (default: 2)
    #[serde(default = "default_max_quote_tweets")]
    pub max_quote_tweets: u32,
    /// Maximum total engagement actions per session (default: 10)
    #[serde(default = "default_max_total_actions")]
    pub max_total_actions: u32,
}

impl Default for EngagementLimitsConfig {
    fn default() -> Self {
        Self {
            max_likes: default_max_likes(),
            max_retweets: default_max_retweets(),
            max_follows: default_max_follows(),
            max_replies: default_max_replies(),
            max_thread_dives: default_max_thread_dives(),
            max_bookmarks: default_max_bookmarks(),
            max_quote_tweets: default_max_quote_tweets(),
            max_total_actions: default_max_total_actions(),
        }
    }
}

fn default_max_likes() -> u32 {
    5
}

fn default_max_retweets() -> u32 {
    3
}

fn default_max_follows() -> u32 {
    2
}

fn default_max_replies() -> u32 {
    1
}

fn default_max_thread_dives() -> u32 {
    3
}

fn default_max_bookmarks() -> u32 {
    2
}

fn default_max_quote_tweets() -> u32 {
    2
}

fn default_max_total_actions() -> u32 {
    10
}

fn default_max_workers_per_session() -> usize {
    5
}

fn default_native_interaction_stability_wait_ms() -> u64 {
    5_000
}

fn default_native_interaction_resolve_timeout_ms() -> u64 {
    2_000
}

fn default_native_interaction_settle_ms() -> u64 {
    0
}

fn default_feed_scan_duration() -> u64 {
    60_000
}
fn default_feed_scroll_count() -> u32 {
    10
}
fn default_engagement_candidate_count() -> u32 {
    5
}
fn default_twitter_scroll_amount() -> i32 {
    0 // 0 means use profile default
}
fn default_candidate_scan_interval_ms() -> u64 {
    0 // 0 means use default 2500ms
}

impl Default for TwitterActivityConfig {
    fn default() -> Self {
        Self {
            feed_scan_duration_ms: default_feed_scan_duration(),
            feed_scroll_count: default_feed_scroll_count(),
            engagement_candidate_count: default_engagement_candidate_count(),
            scroll_amount_pixels: default_twitter_scroll_amount(),
            candidate_scan_interval_ms: default_candidate_scan_interval_ms(),
            persona_file_path: None,
            probabilities: TwitterProbabilitiesConfig::default(),
            engagement_limits: EngagementLimitsConfig::default(),
            llm: TwitterLLMConfig::default(),
        }
    }
}

#[allow(dead_code)]
impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            connectors: vec![],
            connection_timeout_ms: 30000,
            max_discovery_retries: 3,
            discovery_retry_delay_ms: 500,
            circuit_breaker: CircuitBreakerConfig::default(),
            profiles: vec![],
            roxybrowser: RoxybrowserConfig::default(),
            user_agent: None,
            extra_http_headers: BTreeMap::new(),
            cursor_overlay_ms: 0,
            native_interaction: NativeInteractionConfig::default(),
            max_workers_per_session: 5,
        }
    }
}

#[allow(dead_code)]
impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_global_concurrency: 5,
            task_timeout_ms: 60000,
            group_timeout_ms: 300000,
            worker_wait_timeout_ms: 10000,
            stuck_worker_threshold_ms: 120000,
            task_stagger_delay_ms: 500,
            max_retries: 3,
            retry_delay_ms: 2000,
        }
    }
}

#[allow(dead_code)]
impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            failure_threshold: 5,
            success_threshold: 3,
            half_open_time_ms: 30000,
        }
    }
}

#[allow(dead_code)]
impl Default for RoxybrowserConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: "http://localhost:4444".to_string(),
            api_key: String::new(),
        }
    }
}

#[allow(dead_code)]
impl Default for BrowserProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            r#type: "chrome".to_string(),
            ws_endpoint: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_defaults() {
        let config = BrowserConfig::default();
        assert_eq!(config.max_discovery_retries, 3);
        assert_eq!(config.discovery_retry_delay_ms, 500);
        assert!(config.profiles.is_empty());
        assert_eq!(
            config.native_interaction.calibration_mode,
            NativeClickCalibrationMode::Windows
        );
        assert_eq!(
            config.native_interaction.native_input_backend,
            NativeInputBackend::Enigo
        );
        assert_eq!(config.native_interaction.stability_wait_ms, 5000);
        assert_eq!(config.native_interaction.resolve_timeout_ms, 2000);
        assert_eq!(config.native_interaction.settle_ms, 0);
    }

    #[test]
    fn test_orchestrator_config_defaults() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.max_global_concurrency, 5);
        assert_eq!(config.task_timeout_ms, 60000);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_twitter_activity_config_defaults() {
        let config = TwitterActivityConfig::default();
        assert_eq!(config.feed_scan_duration_ms, 60000);
        assert_eq!(config.feed_scroll_count, 10);
        assert_eq!(config.engagement_candidate_count, 5);
        assert_eq!(config.engagement_limits.max_likes, 5);
        assert_eq!(config.engagement_limits.max_retweets, 3);
        assert_eq!(config.engagement_limits.max_follows, 2);
        assert_eq!(config.engagement_limits.max_total_actions, 10);
    }

    #[test]
    fn test_circuit_breaker_config_defaults() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.success_threshold, 3);
        assert_eq!(config.half_open_time_ms, 30000);
    }

    #[test]
    fn test_roxybrowser_config_defaults() {
        let config = RoxybrowserConfig::default();
        assert_eq!(config.api_url, "http://localhost:4444");
        assert!(!config.enabled);
    }

    #[test]
    fn test_native_click_calibration_mode_from_env() {
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("windows"),
            NativeClickCalibrationMode::Windows
        );
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("mac"),
            NativeClickCalibrationMode::Mac
        );
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("darwin"),
            NativeClickCalibrationMode::Mac
        );
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("linux"),
            NativeClickCalibrationMode::Linux
        );
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("invalid"),
            NativeClickCalibrationMode::Windows
        );
    }

    #[test]
    fn test_native_click_calibration_mode_as_str() {
        assert_eq!(NativeClickCalibrationMode::Windows.as_str(), "windows");
        assert_eq!(NativeClickCalibrationMode::Mac.as_str(), "mac");
        assert_eq!(NativeClickCalibrationMode::Linux.as_str(), "linux");
    }

    #[test]
    fn test_native_input_backend_from_env() {
        assert_eq!(
            NativeInputBackend::from_env_value("enigo"),
            NativeInputBackend::Enigo
        );
        assert_eq!(
            NativeInputBackend::from_env_value("sendinput"),
            NativeInputBackend::Sendinput
        );
        assert_eq!(
            NativeInputBackend::from_env_value("send_input"),
            NativeInputBackend::Sendinput
        );
        assert_eq!(
            NativeInputBackend::from_env_value("rdev"),
            NativeInputBackend::Rdev
        );
        assert_eq!(
            NativeInputBackend::from_env_value("invalid"),
            NativeInputBackend::Enigo
        );
    }

    #[test]
    fn test_native_input_backend_as_str() {
        assert_eq!(NativeInputBackend::Enigo.as_str(), "enigo");
        assert_eq!(NativeInputBackend::Sendinput.as_str(), "sendinput");
        assert_eq!(NativeInputBackend::Rdev.as_str(), "rdev");
    }

    #[test]
    fn test_native_interaction_config_defaults() {
        let config = NativeInteractionConfig::default();
        assert_eq!(config.calibration_mode, NativeClickCalibrationMode::Windows);
        assert_eq!(config.native_input_backend, NativeInputBackend::Enigo);
        assert_eq!(config.stability_wait_ms, 5000);
        assert_eq!(config.resolve_timeout_ms, 2000);
        assert_eq!(config.settle_ms, 0);
    }

    #[test]
    fn test_twitter_probabilities_config_defaults() {
        let config = TwitterProbabilitiesConfig::default();
        assert_eq!(config.like_probability, 0.4);
        assert_eq!(config.retweet_probability, 0.15);
        assert_eq!(config.quote_probability, 0.15);
        assert_eq!(config.follow_probability, 0.05);
        assert_eq!(config.reply_probability, 0.05);
        assert_eq!(config.bookmark_probability, 0.02);
        assert_eq!(config.thread_dive_probability, 0.25);
    }

    #[test]
    fn test_engagement_limits_config_defaults() {
        let config = EngagementLimitsConfig::default();
        assert_eq!(config.max_likes, 5);
        assert_eq!(config.max_retweets, 3);
        assert_eq!(config.max_follows, 2);
        assert_eq!(config.max_replies, 1);
        assert_eq!(config.max_thread_dives, 3);
        assert_eq!(config.max_bookmarks, 2);
        assert_eq!(config.max_quote_tweets, 2);
        assert_eq!(config.max_total_actions, 10);
    }

    #[test]
    fn test_twitter_llm_config_defaults() {
        let config = TwitterLLMConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.provider, "");
        assert_eq!(config.model, "");
        assert_eq!(config.temperature, 0.0);
        assert_eq!(config.max_tokens, 0);
        assert_eq!(config.timeout_ms, 0);
    }

    #[test]
    fn test_tracing_config_defaults() {
        let config = TracingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.otlp_endpoint, "");
        assert_eq!(config.service_name, "");
    }

    #[test]
    fn test_browser_profile_defaults() {
        let profile = BrowserProfile::default();
        assert_eq!(profile.name, "default");
        assert_eq!(profile.r#type, "chrome");
        assert!(profile.ws_endpoint.is_empty());
    }

    #[test]
    fn test_config_validation_report_new() {
        let report = ConfigValidationReport::new();
        assert!(report.errors.is_empty());
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn test_native_click_calibration_mode_case_insensitive() {
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("WINDOWS"),
            NativeClickCalibrationMode::Windows
        );
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("MAC"),
            NativeClickCalibrationMode::Mac
        );
        assert_eq!(
            NativeClickCalibrationMode::from_env_value("LINUX"),
            NativeClickCalibrationMode::Linux
        );
    }

    #[test]
    fn test_native_input_backend_case_insensitive() {
        assert_eq!(
            NativeInputBackend::from_env_value("ENIGO"),
            NativeInputBackend::Enigo
        );
        assert_eq!(
            NativeInputBackend::from_env_value("SENDINPUT"),
            NativeInputBackend::Sendinput
        );
        assert_eq!(
            NativeInputBackend::from_env_value("RDEV"),
            NativeInputBackend::Rdev
        );
    }

    #[test]
    fn test_twitter_activity_config_clone() {
        let config = TwitterActivityConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.feed_scan_duration_ms, config.feed_scan_duration_ms);
        assert_eq!(cloned.feed_scroll_count, config.feed_scroll_count);
    }

    #[test]
    fn test_orchestrator_config_clone() {
        let config = OrchestratorConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_global_concurrency, config.max_global_concurrency);
        assert_eq!(cloned.task_timeout_ms, config.task_timeout_ms);
    }

    #[test]
    fn test_browser_config_clone() {
        let config = BrowserConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_discovery_retries, config.max_discovery_retries);
        assert_eq!(cloned.discovery_retry_delay_ms, config.discovery_retry_delay_ms);
    }

    #[test]
    fn test_native_click_calibration_mode_partial_equality() {
        let mode1 = NativeClickCalibrationMode::Windows;
        let mode2 = NativeClickCalibrationMode::Windows;
        assert_eq!(mode1, mode2);
        assert!(mode1 == mode2);
    }

    #[test]
    fn test_native_input_backend_partial_equality() {
        let backend1 = NativeInputBackend::Enigo;
        let backend2 = NativeInputBackend::Enigo;
        assert_eq!(backend1, backend2);
        assert!(backend1 == backend2);
    }

    #[test]
    fn test_config_struct_debug() {
        let config = BrowserConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("BrowserConfig"));
    }

    #[test]
    fn test_config_validation_report_add_error() {
        let mut report = ConfigValidationReport::new();
        report.errors.push("Test error".to_string());
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0], "Test error");
    }

    #[test]
    fn test_config_validation_report_add_warning() {
        let mut report = ConfigValidationReport::new();
        report.warnings.push("Test warning".to_string());
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0], "Test warning");
    }

    #[test]
    fn test_config_validation_report_clone() {
        let mut report = ConfigValidationReport::new();
        report.errors.push("Error 1".to_string());
        report.warnings.push("Warning 1".to_string());
        let cloned = report.clone();
        assert_eq!(cloned.errors.len(), 1);
        assert_eq!(cloned.warnings.len(), 1);
    }

    #[test]
    fn test_twitter_activity_config_with_persona_path() {
        let mut config = TwitterActivityConfig::default();
        config.persona_file_path = Some("/path/to/persona.json".to_string());
        assert!(config.persona_file_path.is_some());
        assert_eq!(config.persona_file_path.unwrap(), "/path/to/persona.json");
    }

    #[test]
    fn test_twitter_activity_config_scroll_amount_override() {
        let mut config = TwitterActivityConfig::default();
        config.scroll_amount_pixels = 500;
        assert_eq!(config.scroll_amount_pixels, 500);
    }

    #[test]
    fn test_twitter_activity_config_candidate_scan_interval() {
        let mut config = TwitterActivityConfig::default();
        config.candidate_scan_interval_ms = 3000;
        assert_eq!(config.candidate_scan_interval_ms, 3000);
    }

    #[test]
    fn test_twitter_llm_config_with_custom_values() {
        let config = TwitterLLMConfig {
            enabled: true,
            provider: "openrouter".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.5,
            max_tokens: 200,
            timeout_ms: 60000,
            reply_probability: 0.1,
            quote_tweet_probability: 0.05,
        };
        assert!(config.enabled);
        assert_eq!(config.provider, "openrouter");
        assert_eq!(config.model, "gpt-4");
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_tokens, 200);
    }

    #[test]
    fn test_tracing_config_with_custom_values() {
        let config = TracingConfig {
            enabled: true,
            otlp_endpoint: "http://localhost:4318".to_string(),
            service_name: "my-service".to_string(),
        };
        assert!(config.enabled);
        assert_eq!(config.otlp_endpoint, "http://localhost:4318");
        assert_eq!(config.service_name, "my-service");
    }

    #[test]
    fn test_browser_config_with_user_agent() {
        let mut config = BrowserConfig::default();
        config.user_agent = Some("CustomAgent/1.0".to_string());
        assert!(config.user_agent.is_some());
        assert_eq!(config.user_agent.unwrap(), "CustomAgent/1.0");
    }

    #[test]
    fn test_browser_config_with_extra_headers() {
        let mut config = BrowserConfig::default();
        config.extra_http_headers.insert("X-Custom".to_string(), "Value".to_string());
        assert_eq!(config.extra_http_headers.len(), 1);
        assert_eq!(config.extra_http_headers.get("X-Custom"), Some(&"Value".to_string()));
    }

    #[test]
    fn test_browser_config_cursor_overlay_ms() {
        let mut config = BrowserConfig::default();
        config.cursor_overlay_ms = 100;
        assert_eq!(config.cursor_overlay_ms, 100);
    }

    #[test]
    fn test_browser_config_max_workers_per_session() {
        let config = BrowserConfig::default();
        assert_eq!(config.max_workers_per_session, 5);
    }

    #[test]
    fn test_roxybrowser_config_with_api_key() {
        let config = RoxybrowserConfig {
            enabled: true,
            api_url: "https://api.roxybrowser.com".to_string(),
            api_key: "test-key-123".to_string(),
        };
        assert!(config.enabled);
        assert_eq!(config.api_url, "https://api.roxybrowser.com");
        assert_eq!(config.api_key, "test-key-123");
    }

    #[test]
    fn test_circuit_breaker_config_custom_values() {
        let config = CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 10,
            success_threshold: 5,
            half_open_time_ms: 60000,
        };
        assert!(config.enabled);
        assert_eq!(config.failure_threshold, 10);
        assert_eq!(config.success_threshold, 5);
        assert_eq!(config.half_open_time_ms, 60000);
    }

    #[test]
    fn test_orchestrator_config_custom_values() {
        let config = OrchestratorConfig {
            max_global_concurrency: 10,
            task_timeout_ms: 120000,
            group_timeout_ms: 600000,
            worker_wait_timeout_ms: 20000,
            stuck_worker_threshold_ms: 240000,
            task_stagger_delay_ms: 1000,
            max_retries: 5,
            retry_delay_ms: 5000,
        };
        assert_eq!(config.max_global_concurrency, 10);
        assert_eq!(config.task_timeout_ms, 120000);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_browser_profile_custom_values() {
        let profile = BrowserProfile {
            name: "MyProfile".to_string(),
            r#type: "brave".to_string(),
            ws_endpoint: "ws://localhost:9222".to_string(),
        };
        assert_eq!(profile.name, "MyProfile");
        assert_eq!(profile.r#type, "brave");
        assert_eq!(profile.ws_endpoint, "ws://localhost:9222");
    }

    #[test]
    fn test_engagement_limits_config_custom_values() {
        let config = EngagementLimitsConfig {
            max_likes: 10,
            max_retweets: 5,
            max_follows: 3,
            max_replies: 2,
            max_thread_dives: 5,
            max_bookmarks: 3,
            max_quote_tweets: 4,
            max_total_actions: 20,
        };
        assert_eq!(config.max_likes, 10);
        assert_eq!(config.max_total_actions, 20);
    }

    #[test]
    fn test_twitter_probabilities_config_custom_values() {
        let config = TwitterProbabilitiesConfig {
            like_probability: 0.5,
            retweet_probability: 0.2,
            quote_probability: 0.1,
            follow_probability: 0.1,
            reply_probability: 0.1,
            bookmark_probability: 0.05,
            thread_dive_probability: 0.3,
        };
        assert_eq!(config.like_probability, 0.5);
        assert_eq!(config.thread_dive_probability, 0.3);
    }
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
    load_dotenv_defaults();

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

fn load_dotenv_defaults() {
    let dotenv_path = Path::new(".env");
    if !dotenv_path.exists() {
        return;
    }

    let Ok(content) = fs::read_to_string(dotenv_path) else {
        return;
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() || env::var_os(key).is_some() {
            continue;
        }

        let mut value = raw_value.trim().to_string();
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = value[1..value.len() - 1].to_string();
        }

        env::set_var(key, value);
    }
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
            cursor_overlay_ms: 0,
            native_interaction: NativeInteractionConfig::default(),
            max_workers_per_session: 5,
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
        twitter_activity: TwitterActivityConfig::default(),
        tracing: TracingConfig::default(),
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
    if let Ok(overlay_ms) = env::var("CURSOR_OVERLAY_MS") {
        config.browser.cursor_overlay_ms = overlay_ms
            .parse()
            .unwrap_or(config.browser.cursor_overlay_ms);
    }
    if let Ok(mode) = env::var("native_click_calibration") {
        config.browser.native_interaction.calibration_mode =
            NativeClickCalibrationMode::from_env_value(&mode);
    } else if let Ok(mode) = env::var("NATIVE_CLICK_CALIBRATION") {
        config.browser.native_interaction.calibration_mode =
            NativeClickCalibrationMode::from_env_value(&mode);
    }
    if let Ok(stability_wait_ms) = env::var("NATIVE_INTERACTION_STABILITY_WAIT_MS") {
        config.browser.native_interaction.stability_wait_ms = stability_wait_ms
            .parse()
            .unwrap_or(config.browser.native_interaction.stability_wait_ms);
    }
    if let Ok(resolve_timeout_ms) = env::var("NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS") {
        config.browser.native_interaction.resolve_timeout_ms = resolve_timeout_ms
            .parse()
            .unwrap_or(config.browser.native_interaction.resolve_timeout_ms);
    }
    if let Ok(settle_ms) = env::var("NATIVE_INTERACTION_SETTLE_MS") {
        config.browser.native_interaction.settle_ms = settle_ms
            .parse()
            .unwrap_or(config.browser.native_interaction.settle_ms);
    }
    if let Ok(backend) = env::var("NATIVE_INPUT_BACKEND") {
        config.browser.native_interaction.native_input_backend =
            NativeInputBackend::from_env_value(&backend);
    }

    // Twitter Activity engagement limits overrides
    if let Ok(max_likes) = env::var("TWITTER_MAX_LIKES") {
        config.twitter_activity.engagement_limits.max_likes = max_likes
            .parse()
            .unwrap_or(config.twitter_activity.engagement_limits.max_likes);
    }
    if let Ok(max_retweets) = env::var("TWITTER_MAX_RETWEETS") {
        config.twitter_activity.engagement_limits.max_retweets = max_retweets
            .parse()
            .unwrap_or(config.twitter_activity.engagement_limits.max_retweets);
    }
    if let Ok(max_follows) = env::var("TWITTER_MAX_FOLLOWS") {
        config.twitter_activity.engagement_limits.max_follows = max_follows
            .parse()
            .unwrap_or(config.twitter_activity.engagement_limits.max_follows);
    }
    if let Ok(max_replies) = env::var("TWITTER_MAX_REPLIES") {
        config.twitter_activity.engagement_limits.max_replies = max_replies
            .parse()
            .unwrap_or(config.twitter_activity.engagement_limits.max_replies);
    }
    if let Ok(max_total) = env::var("TWITTER_MAX_TOTAL_ACTIONS") {
        config.twitter_activity.engagement_limits.max_total_actions = max_total
            .parse()
            .unwrap_or(config.twitter_activity.engagement_limits.max_total_actions);
    }

    // Helper function to parse float from env var, stripping comments
    fn parse_env_float(var_name: &str, _default: f64, config: &mut f64) {
        if let Ok(prob_str) = env::var(var_name) {
            // Strip comments (everything after #)
            let clean_prob = prob_str.split('#').next().unwrap_or(&prob_str).trim();
            match clean_prob.parse::<f64>() {
                Ok(val) => {
                    log::info!("Loaded {} from env: '{}' -> {:.3}", var_name, prob_str, val);
                    *config = val;
                }
                Err(e) => log::warn!(
                    "Failed to parse {} '{}' (cleaned: '{}'): {}",
                    var_name,
                    prob_str,
                    clean_prob,
                    e
                ),
            }
        } else {
            log::debug!("{} not set in environment", var_name);
        }
    }

    // Twitter engagement probabilities overrides
    parse_env_float(
        "TWITTER_LIKE_PROBABILITY",
        config.twitter_activity.probabilities.like_probability,
        &mut config.twitter_activity.probabilities.like_probability,
    );
    parse_env_float(
        "TWITTER_RETWEET_PROBABILITY",
        config.twitter_activity.probabilities.retweet_probability,
        &mut config.twitter_activity.probabilities.retweet_probability,
    );
    parse_env_float(
        "TWITTER_QUOTE_PROBABILITY",
        config.twitter_activity.probabilities.quote_probability,
        &mut config.twitter_activity.probabilities.quote_probability,
    );
    parse_env_float(
        "TWITTER_FOLLOW_PROBABILITY",
        config.twitter_activity.probabilities.follow_probability,
        &mut config.twitter_activity.probabilities.follow_probability,
    );
    parse_env_float(
        "TWITTER_REPLY_PROBABILITY",
        config.twitter_activity.probabilities.reply_probability,
        &mut config.twitter_activity.probabilities.reply_probability,
    );
    parse_env_float(
        "TWITTER_BOOKMARK_PROBABILITY",
        config.twitter_activity.probabilities.bookmark_probability,
        &mut config.twitter_activity.probabilities.bookmark_probability,
    );
    parse_env_float(
        "TWITTER_THREAD_DIVE_PROBABILITY",
        config
            .twitter_activity
            .probabilities
            .thread_dive_probability,
        &mut config
            .twitter_activity
            .probabilities
            .thread_dive_probability,
    );

    // Twitter scroll behavior override
    if let Ok(amount) = env::var("TWITTER_SCROLL_AMOUNT_PIXELS") {
        config.twitter_activity.scroll_amount_pixels = amount
            .parse()
            .unwrap_or(config.twitter_activity.scroll_amount_pixels);
    }
    if let Ok(interval) = env::var("TWITTER_CANDIDATE_SCAN_INTERVAL_MS") {
        config.twitter_activity.candidate_scan_interval_ms = interval
            .parse()
            .unwrap_or(config.twitter_activity.candidate_scan_interval_ms);
    }

    // Twitter LLM config overrides (V2)
    if let Ok(enabled) = env::var("TWITTER_LLM_ENABLED") {
        config.twitter_activity.llm.enabled = enabled
            .parse()
            .unwrap_or(config.twitter_activity.llm.enabled);
    }
    if let Ok(provider) = env::var("TWITTER_LLM_PROVIDER") {
        config.twitter_activity.llm.provider = provider;
    }
    if let Ok(model) = env::var("TWITTER_LLM_MODEL") {
        config.twitter_activity.llm.model = model;
    }
    if let Ok(prob) = env::var("TWITTER_LLM_REPLY_PROBABILITY") {
        config.twitter_activity.llm.reply_probability = prob
            .parse()
            .unwrap_or(config.twitter_activity.llm.reply_probability);
    }
    if let Ok(prob) = env::var("TWITTER_LLM_QUOTE_PROBABILITY") {
        config.twitter_activity.llm.quote_tweet_probability = prob
            .parse()
            .unwrap_or(config.twitter_activity.llm.quote_tweet_probability);
    }

    log::info!("Final Twitter probabilities in config: like={:.3}, retweet={:.3}, quote={:.3}, follow={:.3}, reply={:.3}, bookmark={:.3}, dive={:.3}",
        config.twitter_activity.probabilities.like_probability,
        config.twitter_activity.probabilities.retweet_probability,
        config.twitter_activity.probabilities.quote_probability,
        config.twitter_activity.probabilities.follow_probability,
        config.twitter_activity.probabilities.reply_probability,
        config.twitter_activity.probabilities.bookmark_probability,
        config.twitter_activity.probabilities.thread_dive_probability);

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
    let report = ConfigValidationReport::new();

    // Validate orchestrator settings
    report.validate_orchestrator_config(&config.orchestrator)?;

    // Validate browser settings
    report.validate_browser_config(&config.browser)?;

    // Validate circuit breaker config
    report.validate_circuit_breaker(&config.browser.circuit_breaker)?;

    // Validate Twitter Activity config
    report.validate_twitter_activity_config(&config.twitter_activity)?;

    // Validate LLM config
    report.validate_llm_config(&config.twitter_activity.llm)?;

    // Validate tracing config
    report.validate_tracing_config(&config.tracing)?;

    info!("Config validation passed");
    Ok(())
}

/// Detailed validation report for configuration
#[derive(Debug, Clone)]
pub struct ConfigValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ConfigValidationReport {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Validate orchestrator configuration with range checks
    pub fn validate_orchestrator_config(&self, config: &OrchestratorConfig) -> Result<()> {
        // Concurrency validation (1-100 range)
        if config.max_global_concurrency == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "max_global_concurrency".to_string(),
                value: config.max_global_concurrency.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.max_global_concurrency > 100 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "max_global_concurrency".to_string(),
                value: config.max_global_concurrency.to_string(),
                reason: "exceeds maximum recommended value (100). Values this high may cause resource exhaustion.".to_string(),
            }));
        }
        if config.max_global_concurrency > 50 {
            warn!(
                "max_global_concurrency ({}) is high. Consider using a connection pool or \
                 rate limiting to avoid overwhelming target servers.",
                config.max_global_concurrency
            );
        }

        // Timeout validations
        if config.task_timeout_ms == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "task_timeout_ms".to_string(),
                value: config.task_timeout_ms.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.task_timeout_ms < 5_000 {
            warn!(
                "task_timeout_ms ({}) is very low. Tasks may timeout before completing.",
                config.task_timeout_ms
            );
        }
        if config.task_timeout_ms > 3_600_000 {
            warn!(
                "task_timeout_ms ({}) is very high (>1 hour). Consider breaking tasks into smaller units.",
                config.task_timeout_ms
            );
        }

        if config.group_timeout_ms == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "group_timeout_ms".to_string(),
                value: config.group_timeout_ms.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.group_timeout_ms < config.task_timeout_ms {
            warn!(
                "group_timeout_ms ({}) is less than task_timeout_ms ({}). \
                 This may cause group timeouts before individual tasks complete.",
                config.group_timeout_ms, config.task_timeout_ms
            );
        }

        // Worker timeout validation
        if config.worker_wait_timeout_ms == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "worker_wait_timeout_ms".to_string(),
                value: config.worker_wait_timeout_ms.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.worker_wait_timeout_ms < 1_000 {
            warn!(
                "worker_wait_timeout_ms ({}) is very low. Workers may timeout before acquiring resources.",
                config.worker_wait_timeout_ms
            );
        }

        // Retry validation
        if config.max_retries > 10 {
            warn!(
                "max_retries ({}) is high. This may cause long running times and \
                 excessive resource usage on persistent failures.",
                config.max_retries
            );
        }
        if config.retry_delay_ms == 0 {
            warn!("retry_delay_ms is 0. Consider adding a delay to avoid tight retry loops.");
        }
        if config.retry_delay_ms > 30_000 {
            warn!(
                "retry_delay_ms ({}) is very high. This may cause long delays between retries.",
                config.retry_delay_ms
            );
        }

        // Cross-field validation: total retry time should not exceed task timeout
        let total_retry_time = config.retry_delay_ms * config.max_retries as u64;
        if total_retry_time > config.task_timeout_ms {
            warn!(
                "Total retry time ({}ms) exceeds task_timeout_ms ({}ms). \
                 Tasks may timeout before all retries are attempted.",
                total_retry_time, config.task_timeout_ms
            );
        }

        // Stagger delay validation
        if config.task_stagger_delay_ms > 10_000 {
            warn!(
                "task_stagger_delay_ms ({}) is very high. This may cause slow group execution.",
                config.task_stagger_delay_ms
            );
        }

        Ok(())
    }

    /// Validate browser configuration
    pub fn validate_browser_config(&self, config: &BrowserConfig) -> Result<()> {
        // Discovery retry validation
        if config.max_discovery_retries == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "max_discovery_retries".to_string(),
                value: config.max_discovery_retries.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.max_discovery_retries > 10 {
            warn!(
                "max_discovery_retries ({}) is high. This may cause long startup delays.",
                config.max_discovery_retries
            );
        }

        if config.discovery_retry_delay_ms == 0 {
            warn!("discovery_retry_delay_ms is 0. Consider adding a delay between retries.");
        }
        if config.discovery_retry_delay_ms > 60_000 {
            warn!(
                "discovery_retry_delay_ms ({}) is very high. This may cause long startup delays.",
                config.discovery_retry_delay_ms
            );
        }

        if config.max_workers_per_session == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "max_workers_per_session".to_string(),
                value: config.max_workers_per_session.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.max_workers_per_session > 20 {
            warn!(
                "max_workers_per_session ({}) is high. Each worker uses a page.",
                config.max_workers_per_session
            );
        }

        // Profile name uniqueness validation
        let mut seen_names = std::collections::HashSet::new();
        for profile in &config.profiles {
            if !seen_names.insert(&profile.name) {
                return Err(OrchestratorError::Config(ConfigError::ValidationFailed(
                    format!(
                        "Duplicate browser profile name: '{}'. Profile names must be unique.",
                        profile.name
                    ),
                )));
            }

            // Validate profile name is not empty
            if profile.name.trim().is_empty() {
                return Err(OrchestratorError::Config(ConfigError::ValidationFailed(
                    "Browser profile name cannot be empty".to_string(),
                )));
            }
        }

        // RoxyBrowser API URL format validation
        if !config.roxybrowser.api_url.is_empty() {
            let url = &config.roxybrowser.api_url;
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                    field: "roxybrowser.api_url".to_string(),
                    value: url.clone(),
                    reason: "must start with http:// or https://".to_string(),
                }));
            }
            // Validate URL format
            if url.parse::<reqwest::Url>().is_err() {
                return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                    field: "roxybrowser.api_url".to_string(),
                    value: url.clone(),
                    reason: "invalid URL format".to_string(),
                }));
            }
            if !url.ends_with('/') {
                warn!(
                    "RoxyBrowser API URL does not end with '/'. This may cause incorrect API paths. Got: {}",
                    url
                );
            }
        }

        // API key validation (warn if empty but don't fail - might not be using RoxyBrowser)
        if config.roxybrowser.enabled && config.roxybrowser.api_key.is_empty() {
            warn!("RoxyBrowser is enabled but api_key is empty. API requests will fail.");
        }

        Ok(())
    }

    /// Validate circuit breaker configuration
    pub fn validate_circuit_breaker(&self, config: &CircuitBreakerConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        if config.failure_threshold == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "circuit_breaker.failure_threshold".to_string(),
                value: config.failure_threshold.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.failure_threshold > 20 {
            warn!(
                "circuit_breaker.failure_threshold ({}) is very high. Circuit may not trip on real failures.",
                config.failure_threshold
            );
        }

        if config.success_threshold == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "circuit_breaker.success_threshold".to_string(),
                value: config.success_threshold.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.success_threshold > 10 {
            warn!(
                "circuit_breaker.success_threshold ({}) is high. Circuit may take long to close.",
                config.success_threshold
            );
        }

        if config.half_open_time_ms < 5_000 {
            warn!(
                "circuit_breaker.half_open_time_ms ({}) is very low. Circuit may close prematurely.",
                config.half_open_time_ms
            );
        }
        if config.half_open_time_ms > 300_000 {
            warn!(
                "circuit_breaker.half_open_time_ms ({}) is very high. Recovery may take too long.",
                config.half_open_time_ms
            );
        }

        Ok(())
    }

    /// Validate Twitter Activity configuration
    pub fn validate_twitter_activity_config(&self, config: &TwitterActivityConfig) -> Result<()> {
        // Feed scan duration validation (10s - 30min range)
        if config.feed_scan_duration_ms < 10_000 {
            warn!(
                "twitter_activity.feed_scan_duration_ms ({}) is very low (<10s). \
                 Feed scan may not capture enough content.",
                config.feed_scan_duration_ms
            );
        }
        if config.feed_scan_duration_ms > 1_800_000 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.feed_scan_duration_ms".to_string(),
                value: config.feed_scan_duration_ms.to_string(),
                reason: "exceeds maximum (30min). Consider breaking into multiple shorter scans."
                    .to_string(),
            }));
        }

        // Feed scroll count validation
        if config.feed_scroll_count == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.feed_scroll_count".to_string(),
                value: config.feed_scroll_count.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.feed_scroll_count > 100 {
            warn!(
                "twitter_activity.feed_scroll_count ({}) is very high. This may trigger rate limiting.",
                config.feed_scroll_count
            );
        }

        // Engagement candidate count validation
        if config.engagement_candidate_count == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.engagement_candidate_count".to_string(),
                value: config.engagement_candidate_count.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.engagement_candidate_count > 20 {
            warn!(
                "twitter_activity.engagement_candidate_count ({}) is high. Consider a smaller number for more focused engagement.",
                config.engagement_candidate_count
            );
        }

        // Persona file path validation (if provided)
        if let Some(path) = &config.persona_file_path {
            if !std::path::Path::new(path).exists() {
                warn!(
                    "twitter_activity.persona_file_path does not exist: {}",
                    path
                );
            }
        }

        // Engagement limits validation
        let limits = &config.engagement_limits;

        if limits.max_total_actions == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.engagement_limits.max_total_actions".to_string(),
                value: limits.max_total_actions.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if limits.max_total_actions > 50 {
            warn!(
                "twitter_activity.engagement_limits.max_total_actions ({}) is very high. \
                 This may trigger rate limiting or account restrictions.",
                limits.max_total_actions
            );
        }

        // Check individual limits don't exceed total
        if limits.max_likes > limits.max_total_actions {
            warn!(
                "twitter_activity.engagement_limits.max_likes ({}) exceeds max_total_actions ({}). \
                 Like limit will be capped by total.",
                limits.max_likes, limits.max_total_actions
            );
        }
        if limits.max_retweets > limits.max_total_actions {
            warn!(
                "twitter_activity.engagement_limits.max_retweets ({}) exceeds max_total_actions ({}).",
                limits.max_retweets, limits.max_total_actions
            );
        }
        if limits.max_follows > limits.max_total_actions {
            warn!(
                "twitter_activity.engagement_limits.max_follows ({}) exceeds max_total_actions ({}).",
                limits.max_follows, limits.max_total_actions
            );
        }

        // Conservative limits warning
        if limits.max_likes > 10 {
            warn!(
                "twitter_activity.engagement_limits.max_likes ({}) is high. \
                 Twitter may flag this as automated behavior. Recommended: ≤5",
                limits.max_likes
            );
        }
        if limits.max_retweets > 5 {
            warn!(
                "twitter_activity.engagement_limits.max_retweets ({}) is high. \
                 Twitter may flag this as automated behavior. Recommended: ≤3",
                limits.max_retweets
            );
        }
        if limits.max_follows > 5 {
            warn!(
                "twitter_activity.engagement_limits.max_follows ({}) is high. \
                 Twitter may flag this as automated behavior. Recommended: ≤2",
                limits.max_follows
            );
        }

        Ok(())
    }

    /// Validate LLM configuration
    pub fn validate_llm_config(&self, config: &TwitterLLMConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        // Temperature validation (should be 0.0 - 2.0 for most models)
        if config.temperature < 0.0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.llm.temperature".to_string(),
                value: config.temperature.to_string(),
                reason: "must be non-negative".to_string(),
            }));
        }
        if config.temperature > 2.0 {
            warn!(
                "twitter_activity.llm.temperature ({}) is high (>2.0). This may produce less coherent responses.",
                config.temperature
            );
        }

        // Max tokens validation
        if config.max_tokens == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.llm.max_tokens".to_string(),
                value: config.max_tokens.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.max_tokens > 4096 {
            warn!(
                "twitter_activity.llm.max_tokens ({}) is high. Consider using smaller values for faster responses.",
                config.max_tokens
            );
        }

        // Timeout validation
        if config.timeout_ms == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.llm.timeout_ms".to_string(),
                value: config.timeout_ms.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.timeout_ms > 60000 {
            warn!(
                "twitter_activity.llm.timeout_ms ({}) is high (>2min). LLM requests may take long to timeout.",
                config.timeout_ms
            );
        }

        // Probability validation (should be 0.0 - 1.0)
        if config.reply_probability < 0.0 || config.reply_probability > 1.0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.llm.reply_probability".to_string(),
                value: config.reply_probability.to_string(),
                reason: "must be between 0.0 and 1.0".to_string(),
            }));
        }
        if config.quote_tweet_probability < 0.0 || config.quote_tweet_probability > 1.0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.llm.quote_tweet_probability".to_string(),
                value: config.quote_tweet_probability.to_string(),
                reason: "must be between 0.0 and 1.0".to_string(),
            }));
        }

        // Provider validation
        if config.provider.is_empty() {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.llm.provider".to_string(),
                value: config.provider.clone(),
                reason: "must not be empty".to_string(),
            }));
        }
        if config.model.is_empty() {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.llm.model".to_string(),
                value: config.model.clone(),
                reason: "must not be empty".to_string(),
            }));
        }

        Ok(())
    }

    /// Validate tracing configuration
    pub fn validate_tracing_config(&self, config: &TracingConfig) -> Result<()> {
        if !config.enabled {
            return Ok(());
        }

        // Validate OTLP endpoint
        if config.otlp_endpoint.is_empty() {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "tracing.otlp_endpoint".to_string(),
                value: config.otlp_endpoint.clone(),
                reason: "must not be empty when tracing is enabled".to_string(),
            }));
        }

        // Validate service name
        if config.service_name.is_empty() {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "tracing.service_name".to_string(),
                value: config.service_name.clone(),
                reason: "must not be empty".to_string(),
            }));
        }

        // Validate URL format for OTLP endpoint
        if config.otlp_endpoint.parse::<reqwest::Url>().is_err() {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "tracing.otlp_endpoint".to_string(),
                value: config.otlp_endpoint.clone(),
                reason: "invalid URL format".to_string(),
            }));
        }

        Ok(())
    }
}

impl Default for ConfigValidationReport {
    fn default() -> Self {
        Self::new()
    }
}
