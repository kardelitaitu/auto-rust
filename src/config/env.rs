//! Environment variable override logic and alternative config loading paths.
//!
//! Provides `load_dotenv_defaults()` for `.env` file loading,
//! `load_code_config()` for hardcoded fallback config,
//! and `apply_env_overrides()` for post-load env-var patching.

use crate::error::Result;
use crate::session::DurationMs;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

use super::types::{
    BraveConfig, BrowserConfig, ChromeConfig, CircuitBreakerConfig, Config, IxbrowserConfig,
    NativeClickCalibrationMode, NativeInputBackend, NativeInteractionConfig, OrchestratorConfig,
    RoxybrowserConfig, ShardbrowserConfig, TaskDiscoveryConfig, TracingConfig,
    TwitterActivityConfig,
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
        if key.is_empty() {
            continue;
        }

        let mut value = raw_value.trim().to_string();
        // Skip if value is empty after trimming
        if value.is_empty() {
            continue;
        }
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = value[1..value.len() - 1].to_string();
        }

        // Only set the variable if it's not already defined in the environment.
        if env::var_os(key).is_none() {
            env::set_var(key, value);
        }
    }
}

/// Build a minimal `Config` from hardcoded defaults + `ROXYBROWSER_*` env vars.
/// Used as a fallback when no `config/default.toml` is found.
pub(crate) fn load_code_config() -> Result<Config> {
    let roxybrowser_url =
        env::var("ROXYBROWSER_API_URL").unwrap_or_else(|_| "http://127.0.0.1:50000/".to_string());
    let roxybrowser_key = env::var("ROXYBROWSER_API_KEY")
        .unwrap_or_else(|_| "c6ae203adfe0327a63ccc9174c178dec".to_string());
    let roxybrowser_enabled = env::var("ROXYBROWSER_ENABLED")
        .or_else(|_| env::var("BROWSER_ROXYBROWSER_ENABLED"))
        .ok()
        .and_then(|v| parse_env_bool(&v))
        .unwrap_or(true);

    let ixbrowser_url =
        env::var("IXBROWSER_API_URL").unwrap_or_else(|_| "http://127.0.0.1:53200".to_string());
    let ixbrowser_enabled = env::var("IXBROWSER_ENABLED")
        .or_else(|_| env::var("BROWSER_IXBROWSER_ENABLED"))
        .ok()
        .and_then(|v| parse_env_bool(&v))
        .unwrap_or(true);

    let shardbrowser_url =
        env::var("SHARDBROWSER_API_URL").unwrap_or_else(|_| "http://127.0.0.1:40325".to_string());
    let shardbrowser_key = env::var("SHARDBROWSER_API_KEY").unwrap_or_default();
    let shardbrowser_enabled = env::var("SHARDBROWSER_ENABLED")
        .or_else(|_| env::var("BROWSER_SHARDBROWSER_ENABLED"))
        .ok()
        .and_then(|v| parse_env_bool(&v))
        .unwrap_or(!shardbrowser_key.is_empty());

    let chrome_enabled = env::var("CHROME_ENABLED")
        .or_else(|_| env::var("BROWSER_CHROME_ENABLED"))
        .ok()
        .and_then(|v| parse_env_bool(&v))
        .unwrap_or(true);

    let brave_enabled = env::var("BRAVE_ENABLED")
        .or_else(|_| env::var("BROWSER_BRAVE_ENABLED"))
        .ok()
        .and_then(|v| parse_env_bool(&v))
        .unwrap_or(true);

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
                enabled: roxybrowser_enabled,
                api_url: roxybrowser_url,
                api_key: roxybrowser_key,
            },
            ixbrowser: IxbrowserConfig {
                enabled: ixbrowser_enabled,
                api_url: ixbrowser_url,
            },
            shardbrowser: ShardbrowserConfig {
                enabled: shardbrowser_enabled,
                api_url: shardbrowser_url,
                api_key: shardbrowser_key,
            },
            chrome: ChromeConfig {
                enabled: chrome_enabled,
                ..ChromeConfig::default()
            },
            brave: BraveConfig {
                enabled: brave_enabled,
                ..BraveConfig::default()
            },
            user_agent: None,
            extra_http_headers: BTreeMap::new(),
            cursor_overlay_ms: 0,
            cursor_overlay_color: "#ff6600".to_string(),
            cursor_overlay_show_trail: true,
            native_interaction: NativeInteractionConfig::default(),
            max_workers_per_session: 5,
            enable_learning_persistence: true,
            learning_ttl_days: 90,
            random_screen_size_brave_and_chrome: true,
        },
        orchestrator: OrchestratorConfig {
            max_global_concurrency: 20,
            task_timeout_ms: DurationMs::new_const(1_200_000),
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

/// Helper function to parse boolean value from environment variable string.
/// Recognizes `true`, `1`, `yes`, `on` as true, and `false`, `0`, `no`, `off` as false (case-insensitive).
pub(crate) fn parse_env_bool(val: &str) -> Option<bool> {
    let clean = val.split('#').next().unwrap_or(val).trim();
    match clean.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
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
    if let Ok(enabled) =
        env::var("ROXYBROWSER_ENABLED").or_else(|_| env::var("BROWSER_ROXYBROWSER_ENABLED"))
    {
        if let Some(b) = parse_env_bool(&enabled) {
            config.browser.roxybrowser.enabled = b;
        }
    }
    if let Ok(url) = env::var("IXBROWSER_API_URL") {
        config.browser.ixbrowser.api_url = url;
    }
    if let Ok(enabled) =
        env::var("IXBROWSER_ENABLED").or_else(|_| env::var("BROWSER_IXBROWSER_ENABLED"))
    {
        if let Some(b) = parse_env_bool(&enabled) {
            config.browser.ixbrowser.enabled = b;
        }
    }
    if let Ok(url) = env::var("SHARDBROWSER_API_URL") {
        if !url.is_empty() {
            config.browser.shardbrowser.enabled = true;
        }
        config.browser.shardbrowser.api_url = url;
    }
    if let Ok(key) = env::var("SHARDBROWSER_API_KEY") {
        if !key.is_empty() {
            config.browser.shardbrowser.enabled = true;
        }
        config.browser.shardbrowser.api_key = key;
    }
    if let Ok(enabled) =
        env::var("SHARDBROWSER_ENABLED").or_else(|_| env::var("BROWSER_SHARDBROWSER_ENABLED"))
    {
        if let Some(b) = parse_env_bool(&enabled) {
            config.browser.shardbrowser.enabled = b;
        }
    }
    if let Ok(enabled) = env::var("CHROME_ENABLED").or_else(|_| env::var("BROWSER_CHROME_ENABLED"))
    {
        if let Some(b) = parse_env_bool(&enabled) {
            config.browser.chrome.enabled = b;
        }
    }
    if let Ok(start) = env::var("CHROME_PORT_START") {
        if let Ok(p) = start.parse() {
            config.browser.chrome.port_start = p;
        }
    }
    if let Ok(end) = env::var("CHROME_PORT_END") {
        if let Ok(p) = end.parse() {
            config.browser.chrome.port_end = p;
        }
    }
    if let Ok(enabled) = env::var("BRAVE_ENABLED").or_else(|_| env::var("BROWSER_BRAVE_ENABLED")) {
        if let Some(b) = parse_env_bool(&enabled) {
            config.browser.brave.enabled = b;
        }
    }
    if let Ok(start) = env::var("BRAVE_PORT_START") {
        if let Ok(p) = start.parse() {
            config.browser.brave.port_start = p;
        }
    }
    if let Ok(end) = env::var("BRAVE_PORT_END") {
        if let Ok(p) = end.parse() {
            config.browser.brave.port_end = p;
        }
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
    if let Ok(stagger) = env::var("TASK_STAGGER_DELAY_MS") {
        config.orchestrator.task_stagger_delay_ms = stagger
            .parse()
            .unwrap_or(config.orchestrator.task_stagger_delay_ms);
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
    if let Ok(color) = env::var("CURSOR_OVERLAY_COLOR") {
        if !color.is_empty() {
            config.browser.cursor_overlay_color = color;
        }
    }
    if let Ok(show_trail) = env::var("CURSOR_OVERLAY_SHOW_TRAIL") {
        config.browser.cursor_overlay_show_trail = show_trail
            .parse()
            .unwrap_or(config.browser.cursor_overlay_show_trail);
    }
    if let Ok(val) = env::var("RANDOM_SCREEN_SIZE_BRAVE__AND_CHROME")
        .or_else(|_| env::var("RANDOM_SCREEN_SIZE_BRAVE_AND_CHROME"))
    {
        config.browser.random_screen_size_brave_and_chrome = val
            .trim()
            .parse()
            .unwrap_or(config.browser.random_screen_size_brave_and_chrome);
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
        &mut config
            .twitter_activity
            .probabilities
            .thread_dive_probability,
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
        config.task_discovery.enabled = enabled.parse().unwrap_or(config.task_discovery.enabled);
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

/// Shared mutex for every test in the `config` module that mutates the
/// process environment. `std::env` is process-global state, so all such
/// tests — in `env.rs` and in `tests.rs` (which aliases this as
/// `config_test_lock`) — must serialize through this single lock.
#[cfg(test)]
pub(crate) fn env_test_lock() -> &'static std::sync::Mutex<()> {
    use std::sync::{Mutex, OnceLock};
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_dotenv_defaults_parses_key_value() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "MY_TEST_VAR=hello\n").unwrap();
        let old = env::var("MY_TEST_VAR").ok();
        env::remove_var("MY_TEST_VAR");
        let orig_dir = env::current_dir().ok();
        env::set_current_dir(tmp.path()).ok();
        load_dotenv_defaults();
        assert_eq!(env::var("MY_TEST_VAR").unwrap(), "hello");
        if let Some(v) = old {
            env::set_var("MY_TEST_VAR", v);
        } else {
            env::remove_var("MY_TEST_VAR");
        }
        if let Some(d) = orig_dir {
            env::set_current_dir(d).ok();
        }
    }

    #[test]
    fn load_dotenv_defaults_skips_existing() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "MY_EXISTING_VAR=from_file\n").unwrap();
        let old = env::var("MY_EXISTING_VAR").ok();
        env::set_var("MY_EXISTING_VAR", "from_env");
        let orig_dir = env::current_dir().ok();
        env::set_current_dir(tmp.path()).ok();
        load_dotenv_defaults();
        assert_eq!(env::var("MY_EXISTING_VAR").unwrap(), "from_env");
        if let Some(v) = old {
            env::set_var("MY_EXISTING_VAR", v);
        } else {
            env::remove_var("MY_EXISTING_VAR");
        }
        if let Some(d) = orig_dir {
            env::set_current_dir(d).ok();
        }
    }

    #[test]
    fn load_dotenv_defaults_skips_comments_and_empty() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "# this is a comment\n\nKEY=value\n").unwrap();
        let old = env::var("KEY").ok();
        env::remove_var("KEY");
        let orig_dir = env::current_dir().unwrap();
        env::set_current_dir(tmp.path()).unwrap();
        load_dotenv_defaults();
        let result = env::var("KEY");
        env::set_current_dir(&orig_dir).unwrap();
        if let Some(v) = old {
            env::set_var("KEY", v);
        } else {
            env::remove_var("KEY");
        }
        assert_eq!(result.unwrap(), "value");
    }

    #[test]
    fn load_dotenv_defaults_strips_quotes() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "QUOTED=\"hello world\"\nSINGLE='test'\n").unwrap();
        let old_quoted = env::var("QUOTED").ok();
        let old_single = env::var("SINGLE").ok();
        env::remove_var("QUOTED");
        env::remove_var("SINGLE");
        let orig_dir = env::current_dir().ok();
        env::set_current_dir(tmp.path()).ok();
        load_dotenv_defaults();
        assert_eq!(env::var("QUOTED").unwrap(), "hello world");
        assert_eq!(env::var("SINGLE").unwrap(), "test");
        if let Some(v) = old_quoted {
            env::set_var("QUOTED", v);
        } else {
            env::remove_var("QUOTED");
        }
        if let Some(v) = old_single {
            env::set_var("SINGLE", v);
        } else {
            env::remove_var("SINGLE");
        }
        if let Some(d) = orig_dir {
            env::set_current_dir(d).ok();
        }
    }

    #[test]
    fn test_parse_env_bool_variants() {
        assert_eq!(parse_env_bool("true"), Some(true));
        assert_eq!(parse_env_bool("TRUE"), Some(true));
        assert_eq!(parse_env_bool("1"), Some(true));
        assert_eq!(parse_env_bool("yes # comment"), Some(true));
        assert_eq!(parse_env_bool("on"), Some(true));

        assert_eq!(parse_env_bool("false"), Some(false));
        assert_eq!(parse_env_bool("0"), Some(false));
        assert_eq!(parse_env_bool("no"), Some(false));
        assert_eq!(parse_env_bool("off # disabled"), Some(false));

        assert_eq!(parse_env_bool("invalid"), None);
    }

    #[test]
    fn apply_env_overrides_roxybrowser() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let old_url = env::var("ROXYBROWSER_API_URL").ok();
        let old_key = env::var("ROXYBROWSER_API_KEY").ok();

        env::set_var("ROXYBROWSER_API_URL", "http://override:9999/");
        env::set_var("ROXYBROWSER_API_KEY", "override-key");

        let mut config = load_code_config().unwrap();
        config = apply_env_overrides(config).unwrap();

        assert_eq!(config.browser.roxybrowser.api_url, "http://override:9999/");
        assert_eq!(config.browser.roxybrowser.api_key, "override-key");

        if let Some(v) = old_url {
            env::set_var("ROXYBROWSER_API_URL", v);
        } else {
            env::remove_var("ROXYBROWSER_API_URL");
        }
        if let Some(v) = old_key {
            env::set_var("ROXYBROWSER_API_KEY", v);
        } else {
            env::remove_var("ROXYBROWSER_API_KEY");
        }
    }

    #[test]
    fn apply_env_overrides_browser_enabled() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let old_roxy = env::var("ROXYBROWSER_ENABLED").ok();
        let old_ix = env::var("IXBROWSER_ENABLED").ok();
        let old_shard = env::var("SHARDBROWSER_ENABLED").ok();

        env::set_var("ROXYBROWSER_ENABLED", "false");
        env::set_var("IXBROWSER_ENABLED", "0");
        env::set_var("SHARDBROWSER_ENABLED", "true");

        let mut config = Config::default();
        config = apply_env_overrides(config).unwrap();

        assert!(!config.browser.roxybrowser.enabled);
        assert!(!config.browser.ixbrowser.enabled);
        assert!(config.browser.shardbrowser.enabled);

        if let Some(v) = old_roxy {
            env::set_var("ROXYBROWSER_ENABLED", v);
        } else {
            env::remove_var("ROXYBROWSER_ENABLED");
        }
        if let Some(v) = old_ix {
            env::set_var("IXBROWSER_ENABLED", v);
        } else {
            env::remove_var("IXBROWSER_ENABLED");
        }
        if let Some(v) = old_shard {
            env::set_var("SHARDBROWSER_ENABLED", v);
        } else {
            env::remove_var("SHARDBROWSER_ENABLED");
        }
    }

    #[test]
    fn apply_env_overrides_shardbrowser() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let old_url = env::var("SHARDBROWSER_API_URL").ok();
        let old_key = env::var("SHARDBROWSER_API_KEY").ok();

        env::set_var("SHARDBROWSER_API_URL", "http://shard:1234");
        env::set_var("SHARDBROWSER_API_KEY", "shard-key");

        let mut config = load_code_config().unwrap();
        config = apply_env_overrides(config).unwrap();

        assert_eq!(config.browser.shardbrowser.api_url, "http://shard:1234");
        assert_eq!(config.browser.shardbrowser.api_key, "shard-key");
        assert!(config.browser.shardbrowser.enabled);

        if let Some(v) = old_url {
            env::set_var("SHARDBROWSER_API_URL", v);
        } else {
            env::remove_var("SHARDBROWSER_API_URL");
        }
        if let Some(v) = old_key {
            env::set_var("SHARDBROWSER_API_KEY", v);
        } else {
            env::remove_var("SHARDBROWSER_API_KEY");
        }
    }

    #[test]
    fn apply_env_overrides_orchestrator() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let old_retries = env::var("MAX_RETRIES").ok();
        let old_stagger = env::var("TASK_STAGGER_DELAY_MS").ok();
        let old_timeout = env::var("TASK_TIMEOUT_MS").ok();

        env::set_var("MAX_RETRIES", "5");
        env::set_var("TASK_STAGGER_DELAY_MS", "100");
        env::set_var("TASK_TIMEOUT_MS", "120000");

        let mut config = load_code_config().unwrap();
        config = apply_env_overrides(config).unwrap();

        assert_eq!(config.orchestrator.max_retries, 5);
        assert_eq!(config.orchestrator.task_stagger_delay_ms, 100);
        assert_eq!(config.orchestrator.task_timeout_ms.get(), 120000);

        if let Some(v) = old_retries {
            env::set_var("MAX_RETRIES", v);
        } else {
            env::remove_var("MAX_RETRIES");
        }
        if let Some(v) = old_stagger {
            env::set_var("TASK_STAGGER_DELAY_MS", v);
        } else {
            env::remove_var("TASK_STAGGER_DELAY_MS");
        }
        if let Some(v) = old_timeout {
            env::set_var("TASK_TIMEOUT_MS", v);
        } else {
            env::remove_var("TASK_TIMEOUT_MS");
        }
    }

    #[test]
    fn apply_env_overrides_twitter_limits() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let old_likes = env::var("TWITTER_MAX_LIKES").ok();
        let old_retweets = env::var("TWITTER_MAX_RETWEETS").ok();
        let old_total = env::var("TWITTER_MAX_TOTAL_ACTIONS").ok();

        env::set_var("TWITTER_MAX_LIKES", "10");
        env::set_var("TWITTER_MAX_RETWEETS", "5");
        env::set_var("TWITTER_MAX_TOTAL_ACTIONS", "25");

        let mut config = load_code_config().unwrap();
        config = apply_env_overrides(config).unwrap();

        assert_eq!(config.twitter_activity.engagement_limits.max_likes, 10);
        assert_eq!(config.twitter_activity.engagement_limits.max_retweets, 5);
        assert_eq!(
            config.twitter_activity.engagement_limits.max_total_actions,
            25
        );

        if let Some(v) = old_likes {
            env::set_var("TWITTER_MAX_LIKES", v);
        } else {
            env::remove_var("TWITTER_MAX_LIKES");
        }
        if let Some(v) = old_retweets {
            env::set_var("TWITTER_MAX_RETWEETS", v);
        } else {
            env::remove_var("TWITTER_MAX_RETWEETS");
        }
        if let Some(v) = old_total {
            env::set_var("TWITTER_MAX_TOTAL_ACTIONS", v);
        } else {
            env::remove_var("TWITTER_MAX_TOTAL_ACTIONS");
        }
    }

    #[test]
    fn apply_env_overrides_twitter_llm() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let old_enabled = env::var("TWITTER_LLM_ENABLED").ok();
        let old_provider = env::var("TWITTER_LLM_PROVIDER").ok();
        let old_model = env::var("TWITTER_LLM_MODEL").ok();

        env::set_var("TWITTER_LLM_ENABLED", "true");
        env::set_var("TWITTER_LLM_PROVIDER", "openrouter");
        env::set_var("TWITTER_LLM_MODEL", "gpt-4");

        let mut config = load_code_config().unwrap();
        config = apply_env_overrides(config).unwrap();

        assert!(config.twitter_activity.llm.enabled);
        assert_eq!(config.twitter_activity.llm.provider, "openrouter");
        assert_eq!(config.twitter_activity.llm.model, "gpt-4");

        if let Some(v) = old_enabled {
            env::set_var("TWITTER_LLM_ENABLED", v);
        } else {
            env::remove_var("TWITTER_LLM_ENABLED");
        }
        if let Some(v) = old_provider {
            env::set_var("TWITTER_LLM_PROVIDER", v);
        } else {
            env::remove_var("TWITTER_LLM_PROVIDER");
        }
        if let Some(v) = old_model {
            env::set_var("TWITTER_LLM_MODEL", v);
        } else {
            env::remove_var("TWITTER_LLM_MODEL");
        }
    }

    #[test]
    fn apply_env_overrides_cursor_overlay() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let old_ms = env::var("CURSOR_OVERLAY_MS").ok();
        let old_color = env::var("CURSOR_OVERLAY_COLOR").ok();

        env::set_var("CURSOR_OVERLAY_MS", "50");
        env::set_var("CURSOR_OVERLAY_COLOR", "#00ff00");

        let mut config = load_code_config().unwrap();
        config = apply_env_overrides(config).unwrap();

        assert_eq!(config.browser.cursor_overlay_ms, 50);
        assert_eq!(config.browser.cursor_overlay_color, "#00ff00");

        if let Some(v) = old_ms {
            env::set_var("CURSOR_OVERLAY_MS", v);
        } else {
            env::remove_var("CURSOR_OVERLAY_MS");
        }
        if let Some(v) = old_color {
            env::set_var("CURSOR_OVERLAY_COLOR", v);
        } else {
            env::remove_var("CURSOR_OVERLAY_COLOR");
        }
    }

    #[test]
    fn load_dotenv_defaults_empty_value_skipped() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let env_path = tmp.path().join(".env");
        std::fs::write(&env_path, "EMPTY_VAR=\n").unwrap();
        let old = env::var("EMPTY_VAR").ok();
        env::remove_var("EMPTY_VAR");
        let orig_dir = env::current_dir().ok();
        env::set_current_dir(tmp.path()).ok();
        load_dotenv_defaults();
        assert!(
            env::var("EMPTY_VAR").is_err(),
            "Empty value should not be set"
        );
        if let Some(v) = old {
            env::set_var("EMPTY_VAR", v);
        }
        if let Some(d) = orig_dir {
            env::set_current_dir(d).ok();
        }
    }

    #[test]
    fn load_dotenv_defaults_no_file_is_noop() {
        let _guard = env_test_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let orig_dir = env::current_dir().ok();
        env::set_current_dir(tmp.path()).ok();
        load_dotenv_defaults();
        if let Some(d) = orig_dir {
            env::set_current_dir(d).ok();
        }
    }
}
