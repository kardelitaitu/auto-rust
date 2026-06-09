//! Environment variable override logic and alternative config loading paths.
//!
//! Provides `load_dotenv_defaults()` for `.env` file loading,
//! `load_code_config()` for hardcoded fallback config,
//! and `apply_env_overrides()` for post-load env-var patching.

use crate::error::Result;
use crate::session::DurationMs;
use log::info;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

use super::types::{
    BrowserConfig, CircuitBreakerConfig, Config, NativeClickCalibrationMode,
    NativeInputBackend, NativeInteractionConfig, OrchestratorConfig, RoxybrowserConfig,
    TaskDiscoveryConfig, TracingConfig, TwitterActivityConfig, TwitterLLMConfig,
};

/// Load `.env` defaults into the environment (only for keys not already set).
/// Called by `load_config()` before reading env overrides.
pub(crate) fn load_dotenv_defaults() {
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

/// Build a minimal `Config` from hardcoded defaults + `ROXYBROWSER_*` env vars.
/// Used as a fallback when no `config/default.toml` is found.
pub(crate) fn load_code_config() -> Result<Config> {
    let roxybrowser_url =
        env::var("ROXYBROWSER_API_URL").unwrap_or_else(|_| "http://127.0.0.1:50000/".to_string());
    let roxybrowser_key = env::var("ROXYBROWSER_API_KEY")
        .unwrap_or_else(|_| "c6ae203adfe0327a63ccc9174c178dec".to_string());

    Ok(Config {
        browser: BrowserConfig {
            connection_timeout_ms: DurationMs::new_const(10000),
            max_discovery_retries: 3,
            discovery_retry_delay_ms: DurationMs::new_const(5000),
            circuit_breaker: CircuitBreakerConfig {
                enabled: true,
                failure_threshold: 5,
                success_threshold: 3,
                half_open_time_ms: DurationMs::new_const(30000),
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
            enable_learning_persistence: true,
            learning_ttl_days: 30,
        },
        orchestrator: OrchestratorConfig {
            max_global_concurrency: 20,
            task_timeout_ms: DurationMs::new_const(600_000),
            group_timeout_ms: DurationMs::new_const(600_000),
            worker_wait_timeout_ms: DurationMs::new_const(10000),
            task_stagger_delay_ms: 2000,
            max_retries: 2,
            retry_delay_ms: DurationMs::new_const(500),
        },
        twitter_activity: TwitterActivityConfig::default(),
        tracing: TracingConfig::default(),
        task_discovery: TaskDiscoveryConfig::default(),
    })
}

/// Apply environment variable overrides to an already-loaded `Config`.
pub(crate) fn apply_env_overrides(mut config: Config) -> Result<Config> {
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
            .parse::<u64>()
            .ok()
            .and_then(DurationMs::new)
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
            .parse::<u64>()
            .ok()
            .and_then(DurationMs::new)
            .unwrap_or(config.browser.native_interaction.stability_wait_ms);
    }
    if let Ok(resolve_timeout_ms) = env::var("NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS") {
        config.browser.native_interaction.resolve_timeout_ms = resolve_timeout_ms
            .parse::<u64>()
            .ok()
            .and_then(DurationMs::new)
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
                    log::info!("Loaded {var_name} from env: '{prob_str}' -> {val:.3}");
                    *config = val;
                }
                Err(e) => log::warn!(
                    "Failed to parse {var_name} '{prob_str}' (cleaned: '{clean_prob}'): {e}"
                ),
            }
        } else {
            log::debug!("{var_name} not set in environment");
        }
    }

    // Twitter engagement probabilities
    parse_env_float(
        "TWITTER_LIKE_PROBABILITY",
        0.4,
        &mut config.twitter_activity.probabilities.like_probability,
    );
    parse_env_float(
        "TWITTER_RETWEET_PROBABILITY",
        0.15,
        &mut config.twitter_activity.probabilities.retweet_probability,
    );
    parse_env_float(
        "TWITTER_QUOTE_PROBABILITY",
        0.15,
        &mut config.twitter_activity.probabilities.quote_probability,
    );
    parse_env_float(
        "TWITTER_FOLLOW_PROBABILITY",
        0.05,
        &mut config.twitter_activity.probabilities.follow_probability,
    );
    parse_env_float(
        "TWITTER_REPLY_PROBABILITY",
        0.05,
        &mut config.twitter_activity.probabilities.reply_probability,
    );
    parse_env_float(
        "TWITTER_BOOKMARK_PROBABILITY",
        0.02,
        &mut config.twitter_activity.probabilities.bookmark_probability,
    );
    parse_env_float(
        "TWITTER_THREAD_DIVE_PROBABILITY",
        0.25,
        &mut config.twitter_activity.probabilities.thread_dive_probability,
    );

    // Twitter scroll/candidate scan interval overrides
    if let Ok(scroll) = env::var("TWITTER_SCROLL_AMOUNT_PIXELS") {
        config.twitter_activity.scroll_amount_pixels = scroll
            .parse()
            .unwrap_or(config.twitter_activity.scroll_amount_pixels);
    }
    if let Ok(candidate) = env::var("TWITTER_CANDIDATE_SCAN_INTERVAL_MS") {
        config.twitter_activity.candidate_scan_interval_ms = candidate
            .parse()
            .unwrap_or(config.twitter_activity.candidate_scan_interval_ms);
    }

    // Twitter consecutive threshold overrides
    if let Ok(scroll_failures) = env::var("TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES") {
        config.twitter_activity.max_consecutive_scroll_failures = scroll_failures
            .parse()
            .unwrap_or(config.twitter_activity.max_consecutive_scroll_failures);
    }
    if let Ok(empty_scans) = env::var("TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS") {
        config.twitter_activity.max_consecutive_empty_scans = empty_scans
            .parse()
            .unwrap_or(config.twitter_activity.max_consecutive_empty_scans);
    }

    // Twitter LLM overrides
    if let Ok(llm_enabled) = env::var("TWITTER_LLM_ENABLED") {
        config.twitter_activity.llm.enabled = llm_enabled
            .parse()
            .unwrap_or(config.twitter_activity.llm.enabled);
    }
    if let Ok(provider) = env::var("TWITTER_LLM_PROVIDER") {
        config.twitter_activity.llm.provider = provider;
    }
    if let Ok(model) = env::var("TWITTER_LLM_MODEL") {
        config.twitter_activity.llm.model = model;
    }
    parse_env_float(
        "TWITTER_LLM_REPLY_PROBABILITY",
        0.05,
        &mut config.twitter_activity.llm.reply_probability,
    );
    parse_env_float(
        "TWITTER_LLM_QUOTE_PROBABILITY",
        0.05,
        &mut config.twitter_activity.llm.quote_tweet_probability,
    );

    // Task discovery overrides
    if let Ok(enabled) = env::var("TASK_DISCOVERY_ENABLED") {
        config.task_discovery.enabled = enabled
            .parse()
            .unwrap_or(config.task_discovery.enabled);
    }
    if let Ok(roots) = env::var("TASK_DISCOVERY_ROOTS") {
        config.task_discovery.roots = roots
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if let Ok(extensions) = env::var("TASK_DISCOVERY_EXTENSIONS") {
        config.task_discovery.extensions = extensions
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    Ok(config)
}
