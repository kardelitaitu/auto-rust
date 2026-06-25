//! Configuration type definitions — all struct and enum types used by the config system.
//!
//! Includes type definitions, enum implementations, and `default_*()` helper functions
//! referenced by `#[serde(default = "...")]` attributes on the structs.

use crate::session::DurationMs;
use log::warn;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Top-level configuration structure for the Rust Orchestrator.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct Config {
    pub browser: BrowserConfig,
    pub orchestrator: OrchestratorConfig,
    #[serde(default)]
    pub twitter_activity: TwitterActivityConfig,
    #[serde(default)]
    pub tracing: TracingConfig,
    #[serde(default)]
    pub task_discovery: TaskDiscoveryConfig,
}

/// Configuration for browser connections and management.
#[derive(Debug, Deserialize, Clone)]
pub struct BrowserConfig {
    pub connection_timeout_ms: DurationMs,
    pub max_discovery_retries: u32,
    pub discovery_retry_delay_ms: DurationMs,
    pub circuit_breaker: CircuitBreakerConfig,
    pub profiles: Vec<BrowserProfile>,
    pub roxybrowser: RoxybrowserConfig,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub extra_http_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub cursor_overlay_ms: u64,
    #[serde(default = "default_cursor_overlay_color")]
    pub cursor_overlay_color: String,
    #[serde(default = "default_cursor_overlay_show_trail")]
    pub cursor_overlay_show_trail: bool,
    #[serde(default)]
    pub native_interaction: NativeInteractionConfig,
    #[serde(default = "default_max_workers_per_session")]
    pub max_workers_per_session: usize,
    #[serde(default = "default_enable_learning_persistence")]
    pub enable_learning_persistence: bool,
    #[serde(default = "default_learning_ttl_days")]
    pub learning_ttl_days: u32,
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
    #[must_use]
    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "mac" | "darwin" | "osx" => Self::Mac,
            "linux" => Self::Linux,
            "windows" => Self::Windows,
            other => {
                warn!("Invalid native click calibration mode '{other}', falling back to windows");
                Self::Windows
            }
        }
    }

    #[must_use]
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
    #[must_use]
    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "enigo" => Self::Enigo,
            "sendinput" | "send_input" | "win32" => Self::Sendinput,
            "rdev" => Self::Rdev,
            other => {
                warn!("Invalid native input backend '{other}', falling back to enigo");
                Self::Enigo
            }
        }
    }

    #[must_use]
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
    pub stability_wait_ms: DurationMs,
    #[serde(default = "default_native_interaction_resolve_timeout_ms")]
    pub resolve_timeout_ms: DurationMs,
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
#[derive(Debug, Deserialize, Clone)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub half_open_time_ms: DurationMs,
}

/// Defines a browser profile for task execution.
#[derive(Debug, Deserialize, Clone)]
pub struct BrowserProfile {
    pub name: String,
    pub r#type: String,
    pub ws_endpoint: String,
}

/// Configuration for `RoxyBrowser` API integration.
#[derive(Debug, Deserialize, Clone)]
pub struct RoxybrowserConfig {
    pub enabled: bool,
    pub api_url: String,
    pub api_key: String,
}

/// Configuration for task orchestration and execution behavior.
#[derive(Debug, Deserialize, Clone)]
pub struct OrchestratorConfig {
    pub max_global_concurrency: usize,
    pub task_timeout_ms: DurationMs,
    pub group_timeout_ms: DurationMs,
    pub worker_wait_timeout_ms: DurationMs,
    pub task_stagger_delay_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: DurationMs,
}

/// Configuration for the Twitter/X activity task.
#[derive(Debug, Deserialize, Clone)]
pub struct TwitterActivityConfig {
    #[serde(default = "default_feed_scan_duration")]
    pub feed_scan_duration_ms: DurationMs,
    #[serde(default = "default_feed_scroll_count")]
    pub feed_scroll_count: u32,
    #[serde(default = "default_engagement_candidate_count")]
    pub engagement_candidate_count: u32,
    #[serde(default = "default_twitter_scroll_amount")]
    pub scroll_amount_pixels: i32,
    #[serde(default = "default_candidate_scan_interval_ms")]
    pub candidate_scan_interval_ms: u64,
    #[serde(default = "default_max_consecutive_scroll_failures")]
    pub max_consecutive_scroll_failures: u32,
    #[serde(default = "default_max_consecutive_empty_scans")]
    pub max_consecutive_empty_scans: u32,
    #[serde(default)]
    pub persona_file_path: Option<String>,
    #[serde(default)]
    pub probabilities: TwitterProbabilitiesConfig,
    #[serde(default)]
    pub engagement_limits: EngagementLimitsConfig,
    #[serde(default)]
    pub llm: TwitterLLMConfig,
    #[serde(default)]
    pub persistence_enabled: bool,
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
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    #[serde(default = "default_service_name")]
    pub service_name: String,
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4317".to_string()
}

fn default_service_name() -> String {
    "auto".to_string()
}

/// Task discovery configuration (Phase 2).
#[derive(Debug, Deserialize, Clone)]
pub struct TaskDiscoveryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub roots: Vec<String>,
    #[serde(default = "default_task_extensions")]
    pub extensions: Vec<String>,
}

fn default_task_extensions() -> Vec<String> {
    vec!["task".to_string()]
}

impl Default for TaskDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            roots: Vec::new(),
            extensions: default_task_extensions(),
        }
    }
}

/// Engagement limits configuration for Twitter automation.
#[derive(Debug, Deserialize, Clone)]
pub struct EngagementLimitsConfig {
    #[serde(default = "default_max_likes")]
    pub max_likes: u32,
    #[serde(default = "default_max_retweets")]
    pub max_retweets: u32,
    #[serde(default = "default_max_follows")]
    pub max_follows: u32,
    #[serde(default = "default_max_replies")]
    pub max_replies: u32,
    #[serde(default = "default_max_thread_dives")]
    pub max_thread_dives: u32,
    #[serde(default = "default_max_bookmarks")]
    pub max_bookmarks: u32,
    #[serde(default = "default_max_quote_tweets")]
    pub max_quote_tweets: u32,
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
fn default_cursor_overlay_color() -> String {
    "#ff6600".to_string()
}

fn default_cursor_overlay_show_trail() -> bool {
    true
}

fn default_max_workers_per_session() -> usize {
    5
}
fn default_enable_learning_persistence() -> bool {
    true
}
fn default_learning_ttl_days() -> u32 {
    30
}
fn default_native_interaction_stability_wait_ms() -> DurationMs {
    DurationMs::new_const(5_000)
}
fn default_native_interaction_resolve_timeout_ms() -> DurationMs {
    DurationMs::new_const(2_000)
}
fn default_native_interaction_settle_ms() -> u64 {
    0
}
pub(crate) fn default_feed_scan_duration() -> DurationMs {
    DurationMs::new_const(60_000)
}
pub(crate) fn default_feed_scroll_count() -> u32 {
    10
}
pub(crate) fn default_engagement_candidate_count() -> u32 {
    5
}
pub(crate) fn default_twitter_scroll_amount() -> i32 {
    0
}
pub(crate) fn default_candidate_scan_interval_ms() -> u64 {
    0
}
pub(crate) fn default_max_consecutive_scroll_failures() -> u32 {
    3
}
pub(crate) fn default_max_consecutive_empty_scans() -> u32 {
    3
}
