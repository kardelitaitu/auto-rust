//! Configuration loading entry point — orchestrates TOML parsing, env overrides, and validation.
//!
//! Re-exports all types, defaults, env overrides, and validation from submodules.

// last audited 26-06-26 by Buffy

use crate::error::{ConfigError, OrchestratorError, Result};
use log::{info, warn};
use std::path::Path;

mod defaults;
mod env;
#[cfg(test)]
mod tests;
mod types;
pub mod validation;

pub(crate) use env::*;
pub use types::*;
pub use validation::*;

/// Loads configuration from file and environment variables.
/// Attempts to load from `config/default.toml` first, then applies environment
/// variable overrides. Falls back to hardcoded defaults if no config file exists.
///
/// # Environment Variables
/// - `ROXYBROWSER_API_URL`: Override the `RoxyBrowser` API URL
/// - `ROXYBROWSER_API_KEY`: Override the `RoxyBrowser` API key
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

/// Runs all validation checks on a loaded config.
///
/// # Example
/// ```ignore
/// use auto::config::{load_config, validate_config};
/// fn example() -> anyhow::Result<()> {
///     let config = load_config()?;
///     validate_config(&config)?;
///     println!("Configuration is valid");
///     Ok(())
/// }
/// ```
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
    #[must_use]
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
        if config.task_timeout_ms.get() == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "task_timeout_ms".to_string(),
                value: config.task_timeout_ms.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.task_timeout_ms.get() < 5_000 {
            warn!(
                "task_timeout_ms ({}) is very low. Tasks may timeout before completing.",
                config.task_timeout_ms
            );
        }
        if config.task_timeout_ms.get() > 3_600_000 {
            warn!(
                "task_timeout_ms ({}) is very high (>1 hour). Consider breaking tasks into smaller units.",
                config.task_timeout_ms
            );
        }

        if config.group_timeout_ms.get() == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "group_timeout_ms".to_string(),
                value: config.group_timeout_ms.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.group_timeout_ms.get() < config.task_timeout_ms.get() {
            warn!(
                "group_timeout_ms ({}) is less than task_timeout_ms ({}). \
                 This may cause group timeouts before individual tasks complete.",
                config.group_timeout_ms, config.task_timeout_ms
            );
        }

        // Worker timeout validation
        if config.worker_wait_timeout_ms.get() == 0 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "worker_wait_timeout_ms".to_string(),
                value: config.worker_wait_timeout_ms.to_string(),
                reason: "must be > 0".to_string(),
            }));
        }
        if config.worker_wait_timeout_ms.get() < 1_000 {
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
        if config.retry_delay_ms.get() == 0 {
            warn!("retry_delay_ms is 0. Consider adding a delay to avoid tight retry loops.");
        }
        if config.retry_delay_ms.get() > 30_000 {
            warn!(
                "retry_delay_ms ({}) is very high. This may cause long delays between retries.",
                config.retry_delay_ms
            );
        }

        // Cross-field validation: total retry time should not exceed task timeout
        let total_retry_time = config.retry_delay_ms * u64::from(config.max_retries);
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

        if config.discovery_retry_delay_ms.get() > 60_000 {
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
                    "RoxyBrowser API URL does not end with '/'. This may cause incorrect API paths. Got: {url}"
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

        if config.half_open_time_ms.get() < 5_000 {
            warn!(
                "circuit_breaker.half_open_time_ms ({}) is very low. Circuit may close prematurely.",
                config.half_open_time_ms
            );
        }
        if config.half_open_time_ms.get() > 300_000 {
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
        if config.feed_scan_duration_ms.get() < 10_000 {
            warn!(
                "twitter_activity.feed_scan_duration_ms ({}) is very low (<10s). \
                 Feed scan may not capture enough content.",
                config.feed_scan_duration_ms.get()
            );
        }
        if config.feed_scan_duration_ms.get() > 1_800_000 {
            return Err(OrchestratorError::Config(ConfigError::InvalidValue {
                field: "twitter_activity.feed_scan_duration_ms".to_string(),
                value: config.feed_scan_duration_ms.get().to_string(),
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
                warn!("twitter_activity.persona_file_path does not exist: {path}");
            }
        }

        // Consecutive failure threshold validation
        if config.max_consecutive_scroll_failures == 0 {
            warn!(
                "twitter_activity.max_consecutive_scroll_failures is 0. The feed loop will stop on the first scroll failure. Consider setting >= 1."
            );
        }
        if config.max_consecutive_scroll_failures > 20 {
            warn!(
                "twitter_activity.max_consecutive_scroll_failures ({}) is very high. This may cause excessive retries on persistent scroll failures.",
                config.max_consecutive_scroll_failures
            );
        }
        if config.max_consecutive_empty_scans == 0 {
            warn!(
                "twitter_activity.max_consecutive_empty_scans is 0. The feed loop will stop on the first empty scan. Consider setting >= 1."
            );
        }
        if config.max_consecutive_empty_scans > 20 {
            warn!(
                "twitter_activity.max_consecutive_empty_scans ({}) is very high. This may cause extended loops when no candidates are found.",
                config.max_consecutive_empty_scans
            );
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
