use super::*;
use crate::session::DurationMs;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn config_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn test_browser_config_defaults() {
    let config = BrowserConfig::default();
    assert_eq!(config.max_discovery_retries, 3);
    assert_eq!(config.discovery_retry_delay_ms.get(), 500);
    assert!(config.profiles.is_empty());
    assert_eq!(
        config.native_interaction.calibration_mode,
        NativeClickCalibrationMode::Windows
    );
    assert_eq!(
        config.native_interaction.native_input_backend,
        NativeInputBackend::Enigo
    );
    assert_eq!(config.native_interaction.stability_wait_ms.get(), 5000);
    assert_eq!(config.native_interaction.resolve_timeout_ms.get(), 2000);
    assert_eq!(config.native_interaction.settle_ms, 0);
}

#[test]
fn test_orchestrator_config_defaults() {
    let config = OrchestratorConfig::default();
    assert_eq!(config.max_global_concurrency, 5);
    assert_eq!(config.task_timeout_ms.get(), 60000);
    assert_eq!(config.max_retries, 3);
}

#[test]
fn test_twitter_activity_config_defaults() {
    let config = TwitterActivityConfig::default();
    assert_eq!(config.feed_scan_duration_ms.get(), 60000);
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
    assert_eq!(config.half_open_time_ms.get(), 30000);
}

#[test]
fn test_roxybrowser_config_defaults() {
    let config = RoxybrowserConfig::default();
    assert_eq!(config.api_url, "http://localhost:4444");
    assert!(!config.enabled);
}

#[test]
fn test_task_discovery_config_defaults() {
    let config = TaskDiscoveryConfig::default();
    assert!(!config.enabled);
    assert!(config.roots.is_empty());
    assert_eq!(config.extensions, vec!["task".to_string()]);
}

#[test]
fn test_load_config_defaults_task_discovery_when_omitted_in_toml() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    std::env::set_current_dir(cwd).unwrap();

    assert!(!config.task_discovery.enabled);
    assert!(config.task_discovery.roots.is_empty());
    assert_eq!(config.task_discovery.extensions, vec!["task".to_string()]);
}

#[test]
fn test_load_config_applies_task_discovery_env_overrides_from_dotenv() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[task_discovery]
enabled = false
roots = []
extensions = ["task"]
"#;
    let dotenv = "TASK_DISCOVERY_ENABLED=true\nTASK_DISCOVERY_ROOTS=./tasks;./extra-tasks\nTASK_DISCOVERY_EXTENSIONS=task;dsl\n";

    fs::write(config_dir.join("default.toml"), toml).unwrap();
    fs::write(temp_dir.path().join(".env"), dotenv).unwrap();

    let keys = [
        "TASK_DISCOVERY_ENABLED",
        "TASK_DISCOVERY_ROOTS",
        "TASK_DISCOVERY_EXTENSIONS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    assert!(config.task_discovery.enabled);
    assert_eq!(
        config.task_discovery.roots,
        vec!["./tasks".to_string(), "./extra-tasks".to_string()]
    );
    assert_eq!(
        config.task_discovery.extensions,
        vec!["task".to_string(), "dsl".to_string()]
    );
}

#[test]
fn test_load_config_prefers_explicit_env_over_dotenv_for_task_discovery() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[task_discovery]
enabled = false
roots = []
extensions = ["task"]
"#;
    let dotenv = "TASK_DISCOVERY_ENABLED=false\nTASK_DISCOVERY_ROOTS=./dotenv-tasks\nTASK_DISCOVERY_EXTENSIONS=dotenv\n";

    fs::write(config_dir.join("default.toml"), toml).unwrap();
    fs::write(temp_dir.path().join(".env"), dotenv).unwrap();

    let keys = [
        "TASK_DISCOVERY_ENABLED",
        "TASK_DISCOVERY_ROOTS",
        "TASK_DISCOVERY_EXTENSIONS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();

    env::set_var("TASK_DISCOVERY_ENABLED", "true");
    env::set_var("TASK_DISCOVERY_ROOTS", "./explicit-tasks;./explicit-extra");
    env::set_var("TASK_DISCOVERY_EXTENSIONS", "task;custom");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    assert!(config.task_discovery.enabled);
    assert_eq!(
        config.task_discovery.roots,
        vec![
            "./explicit-tasks".to_string(),
            "./explicit-extra".to_string()
        ]
    );
    assert_eq!(
        config.task_discovery.extensions,
        vec!["task".to_string(), "custom".to_string()]
    );
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
    assert_eq!(config.stability_wait_ms.get(), 5000);
    assert_eq!(config.resolve_timeout_ms.get(), 2000);
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
    assert_eq!(
        cloned.discovery_retry_delay_ms,
        config.discovery_retry_delay_ms
    );
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
    let config = TwitterActivityConfig {
        persona_file_path: Some("/path/to/persona.json".to_string()),
        ..Default::default()
    };
    assert_eq!(
        config
            .persona_file_path
            .expect("persona_file_path should be set"),
        "/path/to/persona.json"
    );
}

#[test]
fn test_twitter_activity_config_scroll_amount_override() {
    let config = TwitterActivityConfig {
        scroll_amount_pixels: 500,
        ..Default::default()
    };
    assert_eq!(config.scroll_amount_pixels, 500);
}

#[test]
fn test_twitter_activity_config_candidate_scan_interval() {
    let config = TwitterActivityConfig {
        candidate_scan_interval_ms: 3000,
        ..Default::default()
    };
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
    let config = BrowserConfig {
        user_agent: Some("CustomAgent/1.0".to_string()),
        ..Default::default()
    };
    assert_eq!(
        config.user_agent.expect("user_agent should be set"),
        "CustomAgent/1.0"
    );
}

#[test]
fn test_browser_config_with_extra_headers() {
    let mut config = BrowserConfig::default();
    config
        .extra_http_headers
        .insert("X-Custom".to_string(), "Value".to_string());
    assert_eq!(config.extra_http_headers.len(), 1);
    assert_eq!(
        config.extra_http_headers.get("X-Custom"),
        Some(&"Value".to_string())
    );
}

#[test]
fn test_browser_config_cursor_overlay_ms() {
    let config = BrowserConfig {
        cursor_overlay_ms: 100,
        ..Default::default()
    };
    assert_eq!(config.cursor_overlay_ms, 100);
}

#[test]
fn test_browser_config_cursor_overlay_defaults() {
    let config = BrowserConfig::default();
    assert_eq!(
        config.cursor_overlay_color, "#ff6600",
        "cursor_overlay_color should default to #ff6600"
    );
    assert!(
        config.cursor_overlay_show_trail,
        "cursor_overlay_show_trail should default to true"
    );
}

#[test]
fn test_browser_config_cursor_overlay_custom_values() {
    let config = BrowserConfig {
        cursor_overlay_color: "#00ff00".to_string(),
        cursor_overlay_show_trail: false,
        ..Default::default()
    };
    assert_eq!(
        config.cursor_overlay_color, "#00ff00",
        "cursor_overlay_color should be overridden to #00ff00"
    );
    assert!(
        !config.cursor_overlay_show_trail,
        "cursor_overlay_show_trail should be overridden to false"
    );
}

#[test]
fn test_cursor_overlay_color_env_override() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let keys = ["CURSOR_OVERLAY_COLOR"];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    env::set_var("CURSOR_OVERLAY_COLOR", "#ff0000");
    let config = apply_env_overrides(Config::default()).unwrap();
    assert_eq!(
        config.browser.cursor_overlay_color, "#ff0000",
        "CURSOR_OVERLAY_COLOR env var should override default (#ff6600 -> #ff0000)"
    );

    env::remove_var("CURSOR_OVERLAY_COLOR");
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }
}

#[test]
fn test_cursor_overlay_color_env_override_empty_string_preserves_default() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let keys = ["CURSOR_OVERLAY_COLOR"];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Empty string should not override (preserve default)
    env::set_var("CURSOR_OVERLAY_COLOR", "");
    let config = apply_env_overrides(Config::default()).unwrap();
    assert_eq!(
        config.browser.cursor_overlay_color, "#ff6600",
        "Empty CURSOR_OVERLAY_COLOR should preserve default (#ff6600)"
    );

    env::remove_var("CURSOR_OVERLAY_COLOR");
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }
}

#[test]
fn test_cursor_overlay_show_trail_env_override() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let keys = ["CURSOR_OVERLAY_SHOW_TRAIL"];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    env::set_var("CURSOR_OVERLAY_SHOW_TRAIL", "false");
    let config = apply_env_overrides(Config::default()).unwrap();
    assert!(
        !config.browser.cursor_overlay_show_trail,
        "CURSOR_OVERLAY_SHOW_TRAIL=false should override default (true -> false)"
    );

    env::remove_var("CURSOR_OVERLAY_SHOW_TRAIL");
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }
}

#[test]
fn test_cursor_overlay_show_trail_invalid_env_falls_back() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let keys = ["CURSOR_OVERLAY_SHOW_TRAIL"];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Start with a config that has show_trail=false
    let config = Config {
        browser: BrowserConfig {
            cursor_overlay_show_trail: false,
            ..Default::default()
        },
        ..Default::default()
    };

    // Invalid env var value should fall back to the config value (false)
    env::set_var("CURSOR_OVERLAY_SHOW_TRAIL", "not-a-boolean");
    let config = apply_env_overrides(config).unwrap();
    assert!(
        !config.browser.cursor_overlay_show_trail,
        "Invalid CURSOR_OVERLAY_SHOW_TRAIL should fall back to existing value (false)"
    );

    env::remove_var("CURSOR_OVERLAY_SHOW_TRAIL");
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }
}

#[test]
fn test_load_config_applies_cursor_overlay_env_overrides() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // TOML with explicit cursor overlay values
    // (use r## delimiter to avoid colliding with hex color "# in raw string)
    let toml = r##"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 100
cursor_overlay_color = "#00ff00"
cursor_overlay_show_trail = false
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"##;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "CURSOR_OVERLAY_COLOR",
        "CURSOR_OVERLAY_SHOW_TRAIL",
        "CURSOR_OVERLAY_MS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set env vars that should override TOML values
    env::set_var("CURSOR_OVERLAY_COLOR", "#ff0000");
    env::set_var("CURSOR_OVERLAY_SHOW_TRAIL", "true");
    env::set_var("CURSOR_OVERLAY_MS", "250");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Env vars should override TOML values
    assert_eq!(
        config.browser.cursor_overlay_ms, 250,
        "CURSOR_OVERLAY_MS env var should override TOML (100 -> 250)"
    );
    assert_eq!(
        config.browser.cursor_overlay_color, "#ff0000",
        "CURSOR_OVERLAY_COLOR env var should override TOML (#00ff00 -> #ff0000)"
    );
    assert!(
        config.browser.cursor_overlay_show_trail,
        "CURSOR_OVERLAY_SHOW_TRAIL=true env var should override TOML (false -> true)"
    );
}

#[test]
fn test_load_config_applies_cursor_overlay_invalid_env_falls_back_to_toml() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // TOML with explicit non-default cursor overlay values to verify fallback
    let toml = r##"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 999
cursor_overlay_color = "#999999"
cursor_overlay_show_trail = true
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"##;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "CURSOR_OVERLAY_COLOR",
        "CURSOR_OVERLAY_SHOW_TRAIL",
        "CURSOR_OVERLAY_MS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set invalid env vars that cannot be parsed
    env::set_var("CURSOR_OVERLAY_MS", "not-a-number");
    // Color is a string so it can't really be "invalid" — any non-empty string is valid
    // But for show_trail, an unparseable bool should fall back
    env::set_var("CURSOR_OVERLAY_SHOW_TRAIL", "invalid-bool");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Invalid env vars should fall back to TOML file values
    assert_eq!(
        config.browser.cursor_overlay_ms, 999,
        "Invalid CURSOR_OVERLAY_MS should fall back to TOML value (999)"
    );
    assert!(
        config.browser.cursor_overlay_show_trail,
        "Invalid CURSOR_OVERLAY_SHOW_TRAIL should fall back to TOML value (true)"
    );
}

#[test]
fn test_load_config_cursor_overlay_color_env_override_empty_string_keeps_toml() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // TOML with a custom color
    let toml = r##"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
cursor_overlay_color = "#abc123"
cursor_overlay_show_trail = false
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"##;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = ["CURSOR_OVERLAY_COLOR"];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Empty string should preserve TOML value
    env::set_var("CURSOR_OVERLAY_COLOR", "");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Empty string should not override TOML value
    assert_eq!(
        config.browser.cursor_overlay_color, "#abc123",
        "Empty CURSOR_OVERLAY_COLOR env var should not override TOML value (#abc123)"
    );
}

// ---- Direct load_dotenv_defaults() tests for cursor overlay env vars ----

/// Helper to create a temp dir with a .env file for direct load_dotenv_defaults() testing.
/// Returns the TempDir so it stays alive for the test, and the previously saved env values.
fn setup_dotenv_test(content: &str, keys: &[&str]) -> (TempDir, Vec<(String, Option<OsString>)>) {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join(".env"), content).unwrap();

    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    (temp_dir, saved_env)
}

/// Helper to restore env vars after a load_dotenv_defaults() test.
fn teardown_dotenv_test(saved_env: Vec<(String, Option<OsString>)>) {
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }
}

#[test]
fn test_load_dotenv_defaults_basic_cursor_overlay() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dotenv =
        "CURSOR_OVERLAY_COLOR=#ff0000\nCURSOR_OVERLAY_SHOW_TRAIL=false\nCURSOR_OVERLAY_MS=250\n";
    let keys = [
        "CURSOR_OVERLAY_COLOR",
        "CURSOR_OVERLAY_SHOW_TRAIL",
        "CURSOR_OVERLAY_MS",
    ];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#ff0000",
        "CURSOR_OVERLAY_COLOR should be set from .env"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_SHOW_TRAIL").unwrap_or_default(),
        "false",
        "CURSOR_OVERLAY_SHOW_TRAIL should be set from .env"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "250",
        "CURSOR_OVERLAY_MS should be set from .env"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_double_quoted_values() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Double-quoted values should have quotes stripped
    let dotenv = "CURSOR_OVERLAY_COLOR=\"#ff0000\"\nCURSOR_OVERLAY_MS=\"500\"\n";
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#ff0000",
        "Double-quoted color should have quotes stripped"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "500",
        "Double-quoted ms should have quotes stripped"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_single_quoted_values() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Single-quoted values should have quotes stripped
    let dotenv = "CURSOR_OVERLAY_COLOR='#00ff00'\nCURSOR_OVERLAY_MS='100'\n";
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#00ff00",
        "Single-quoted color should have quotes stripped"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "100",
        "Single-quoted ms should have quotes stripped"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_partial_quote_not_stripped() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Single opening quote without closing quote should NOT be stripped
    let dotenv = "CURSOR_OVERLAY_COLOR=\"#ff0000\nCURSOR_OVERLAY_MS=250'\n";
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "\"#ff0000",
        "Unmatched double-quote should NOT be stripped (leading quote only)"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "250'",
        "Unmatched single-quote should NOT be stripped (trailing quote only)"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_skips_existing_env_vars() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Pre-set an env var - .env should NOT override it
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set CURSOR_OVERLAY_COLOR before loading .env
    env::set_var("CURSOR_OVERLAY_COLOR", "#already-set");

    let dotenv = "CURSOR_OVERLAY_COLOR=#ff0000\nCURSOR_OVERLAY_MS=250\n";
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join(".env"), dotenv).unwrap();

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    // Should keep the pre-set value, not the .env value
    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#already-set",
        "Pre-set env var should NOT be overridden by .env"
    );
    // CURSOR_OVERLAY_MS was not pre-set, so it should come from .env
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "250",
        "Unset env var should be set from .env"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_skips_comments_and_empty_lines() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dotenv = "# This is a comment\n   \n\nCURSOR_OVERLAY_COLOR=#ff0000\n# Another comment\nCURSOR_OVERLAY_MS=250\n";
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#ff0000",
        "Should skip comments and load color"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "250",
        "Should skip empty lines and load ms"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_skips_malformed_lines() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Lines without '=' should be skipped
    let dotenv = "CURSOR_OVERLAY_COLOR\nCURSOR_OVERLAY_MS=250\nJUST_A_KEY\n";
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    // CURSOR_OVERLAY_COLOR without = should NOT be set
    assert!(
        env::var_os("CURSOR_OVERLAY_COLOR").is_none(),
        "Malformed line without '=' should not set env var"
    );
    // CURSOR_OVERLAY_MS has = so should be set
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "250",
        "Well-formed line should still be loaded"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_empty_value() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let dotenv = "CURSOR_OVERLAY_COLOR=\nCURSOR_OVERLAY_MS=\n";
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "",
        "Empty value should be set as empty string"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "",
        "Empty value should be set as empty string"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_whitespace_trimming() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Whitespace around key and value should be trimmed
    let dotenv = "  CURSOR_OVERLAY_COLOR  =  #ff0000  \n  CURSOR_OVERLAY_MS=250  \n";
    let keys = ["CURSOR_OVERLAY_COLOR", "CURSOR_OVERLAY_MS"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#ff0000",
        "Whitespace around key/value should be trimmed"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "250",
        "Trailing whitespace on value should be trimmed"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_missing_file_no_crash() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // No .env file in this temp dir - should not crash
    let temp_dir = TempDir::new().unwrap();
    let keys: [&str; 0] = [];
    let (_, saved_env) = setup_dotenv_test("", &keys);

    // Delete the .env file that setup_dotenv_test created
    let _ = fs::remove_file(temp_dir.path().join(".env"));

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    // Should not panic
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_empty_file() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Empty .env file should not set anything
    let keys: [&str; 0] = [];
    let (temp_dir, saved_env) = setup_dotenv_test("", &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    // No env vars should have been set (we didn't save any keys)
    // Just verify no crash

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_mixed_content() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Mix cursor overlay vars with other vars
    let dotenv = "CURSOR_OVERLAY_COLOR=#ff6600\nCURSOR_OVERLAY_SHOW_TRAIL=true\nBROWSER_USER_AGENT=TestBrowser\nCURSOR_OVERLAY_MS=100\n";
    let keys = [
        "CURSOR_OVERLAY_COLOR",
        "CURSOR_OVERLAY_SHOW_TRAIL",
        "BROWSER_USER_AGENT",
        "CURSOR_OVERLAY_MS",
    ];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#ff6600",
        "Cursor overlay color from mixed .env"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_SHOW_TRAIL").unwrap_or_default(),
        "true",
        "Cursor overlay show trail from mixed .env"
    );
    assert_eq!(
        env::var("CURSOR_OVERLAY_MS").unwrap_or_default(),
        "100",
        "Cursor overlay ms from mixed .env"
    );
    assert_eq!(
        env::var("BROWSER_USER_AGENT").unwrap_or_default(),
        "TestBrowser",
        "Browser user agent from mixed .env"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_dotenv_defaults_empty_key_skipped() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Line with empty key should be skipped
    let dotenv = "=value\nCURSOR_OVERLAY_COLOR=#ff0000\n";
    let keys = ["CURSOR_OVERLAY_COLOR"];
    let (temp_dir, saved_env) = setup_dotenv_test(dotenv, &keys);

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();
    load_dotenv_defaults();
    env::set_current_dir(cwd).unwrap();

    assert_eq!(
        env::var("CURSOR_OVERLAY_COLOR").unwrap_or_default(),
        "#ff0000",
        "Valid line after empty key should still be loaded"
    );

    teardown_dotenv_test(saved_env);
}

#[test]
fn test_load_config_applies_cursor_overlay_env_overrides_from_dotenv() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // TOML with explicit cursor overlay values that the .env should override
    let toml = r##"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 100
cursor_overlay_color = "#00ff00"
cursor_overlay_show_trail = false
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"##;
    // .env with cursor overlay env vars (uses quoted values to test strip-quotes logic)
    let dotenv =
        "CURSOR_OVERLAY_COLOR=\"#ff0000\"\nCURSOR_OVERLAY_SHOW_TRAIL=true\nCURSOR_OVERLAY_MS=250\n";

    fs::write(config_dir.join("default.toml"), toml).unwrap();
    fs::write(temp_dir.path().join(".env"), dotenv).unwrap();

    let keys = [
        "CURSOR_OVERLAY_COLOR",
        "CURSOR_OVERLAY_SHOW_TRAIL",
        "CURSOR_OVERLAY_MS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    // .env values should override TOML values
    assert_eq!(
        config.browser.cursor_overlay_ms, 250,
        "CURSOR_OVERLAY_MS from .env should override TOML (100 -> 250)"
    );
    assert_eq!(
        config.browser.cursor_overlay_color, "#ff0000",
        "CURSOR_OVERLAY_COLOR from .env should override TOML (#00ff00 -> #ff0000)"
    );
    assert!(
        config.browser.cursor_overlay_show_trail,
        "CURSOR_OVERLAY_SHOW_TRAIL from .env should override TOML (false -> true)"
    );
}

#[test]
fn test_load_config_prefers_explicit_env_over_dotenv_for_cursor_overlay() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // TOML with cursor overlay values
    let toml = r##"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 100
cursor_overlay_color = "#00ff00"
cursor_overlay_show_trail = false
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"##;
    // .env with one set of values
    let dotenv = "CURSOR_OVERLAY_COLOR=\"#999999\"\nCURSOR_OVERLAY_SHOW_TRAIL=false\nCURSOR_OVERLAY_MS=999\n";

    fs::write(config_dir.join("default.toml"), toml).unwrap();
    fs::write(temp_dir.path().join(".env"), dotenv).unwrap();

    let keys = [
        "CURSOR_OVERLAY_COLOR",
        "CURSOR_OVERLAY_SHOW_TRAIL",
        "CURSOR_OVERLAY_MS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();

    // Set explicit env vars (these should take precedence over .env)
    env::set_var("CURSOR_OVERLAY_COLOR", "#ff0000");
    env::set_var("CURSOR_OVERLAY_SHOW_TRAIL", "true");
    env::set_var("CURSOR_OVERLAY_MS", "250");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(value) => env::set_var(key, value),
            None => env::remove_var(key),
        }
    }

    // Explicit env vars should win over .env values
    assert_eq!(
        config.browser.cursor_overlay_ms, 250,
        "Explicit env var should override .env (999 -> 250)"
    );
    assert_eq!(
        config.browser.cursor_overlay_color, "#ff0000",
        "Explicit env var should override .env (#999999 -> #ff0000)"
    );
    assert!(
        config.browser.cursor_overlay_show_trail,
        "Explicit env var should override .env (false -> true)"
    );
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
        half_open_time_ms: DurationMs::new_const(60000),
    };
    assert!(config.enabled);
    assert_eq!(config.failure_threshold, 10);
    assert_eq!(config.success_threshold, 5);
    assert_eq!(config.half_open_time_ms.get(), 60000);
}

#[test]
fn test_orchestrator_config_custom_values() {
    let config = OrchestratorConfig {
        max_global_concurrency: 10,
        task_timeout_ms: DurationMs::new_const(120000),
        group_timeout_ms: DurationMs::new_const(600000),
        worker_wait_timeout_ms: DurationMs::new_const(20000),
        task_stagger_delay_ms: 1000,
        max_retries: 5,
        retry_delay_ms: DurationMs::new_const(5000),
    };
    assert_eq!(config.max_global_concurrency, 10);
    assert_eq!(config.task_timeout_ms.get(), 120000);
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
fn test_twitter_consecutive_threshold_env_overrides() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let keys = [
        "TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES",
        "TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    env::set_var("TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES", "10");
    env::set_var("TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS", "7");

    let config = apply_env_overrides(Config::default()).unwrap();

    assert_eq!(
        config.twitter_activity.max_consecutive_scroll_failures, 10,
        "TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES env var should override default (3) to 10"
    );
    assert_eq!(
        config.twitter_activity.max_consecutive_empty_scans, 7,
        "TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS env var should override default (3) to 7"
    );

    env::remove_var("TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES");
    env::remove_var("TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS");
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }
}

#[test]
fn test_twitter_consecutive_threshold_env_overrides_invalid_parse_falls_back() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let keys = [
        "TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES",
        "TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    env::set_var("TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES", "not-a-number");
    env::set_var("TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS", "also-invalid");

    let config = apply_env_overrides(Config::default()).unwrap();

    // Invalid parse should fall back to the default value (3)
    assert_eq!(
        config.twitter_activity.max_consecutive_scroll_failures, 3,
        "Invalid env var value should fall back to default (3)"
    );
    assert_eq!(
        config.twitter_activity.max_consecutive_empty_scans, 3,
        "Invalid env var value should fall back to default (3)"
    );

    env::remove_var("TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES");
    env::remove_var("TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS");
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }
}

#[test]
fn test_load_config_applies_twitter_consecutive_threshold_env_overrides() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Create a temp dir with a config/default.toml that has explicit threshold values
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[twitter_activity]
max_consecutive_scroll_failures = 10
max_consecutive_empty_scans = 5
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 0
candidate_scan_interval_ms = 0

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    // Save and clear env vars that could interfere
    let keys = [
        "TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES",
        "TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set env vars that should override the TOML values
    env::set_var("TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES", "15");
    env::set_var("TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS", "8");

    // Change to temp dir so load_config() finds config/default.toml
    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    // Restore cwd and env vars
    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Env vars should override TOML values (15 > 10, 8 > 5)
    assert_eq!(
        config.twitter_activity.max_consecutive_scroll_failures, 15,
        "Env var should override TOML value (10 -> 15)"
    );
    assert_eq!(
        config.twitter_activity.max_consecutive_empty_scans, 8,
        "Env var should override TOML value (5 -> 8)"
    );
}

#[test]
fn test_load_config_applies_twitter_consecutive_threshold_invalid_env_falls_back_to_toml() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    // Create a temp dir with a config/default.toml that has explicit threshold values
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[twitter_activity]
max_consecutive_scroll_failures = 10
max_consecutive_empty_scans = 5
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 0
candidate_scan_interval_ms = 0

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    // Save and clear env vars that could interfere
    let keys = [
        "TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES",
        "TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set invalid env vars that cannot be parsed as u32
    env::set_var("TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES", "not-a-number");
    env::set_var("TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS", "also-invalid");

    // Change to temp dir so load_config() finds config/default.toml
    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    // Restore cwd and env vars
    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Invalid env vars should fall back to TOML file values (10, 5), not hardcoded defaults (3)
    assert_eq!(
        config.twitter_activity.max_consecutive_scroll_failures, 10,
        "Invalid env var should fall back to TOML value (10), not hardcoded default (3)"
    );
    assert_eq!(
        config.twitter_activity.max_consecutive_empty_scans, 5,
        "Invalid env var should fall back to TOML value (5), not hardcoded default (3)"
    );
}

#[test]
fn test_load_config_applies_twitter_engagement_limit_env_overrides() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[twitter_activity]
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 0
candidate_scan_interval_ms = 0

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "TWITTER_MAX_LIKES",
        "TWITTER_MAX_RETWEETS",
        "TWITTER_MAX_FOLLOWS",
        "TWITTER_MAX_REPLIES",
        "TWITTER_MAX_TOTAL_ACTIONS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set env vars that should override TOML values
    env::set_var("TWITTER_MAX_LIKES", "12");
    env::set_var("TWITTER_MAX_RETWEETS", "7");
    env::set_var("TWITTER_MAX_FOLLOWS", "5");
    env::set_var("TWITTER_MAX_REPLIES", "3");
    env::set_var("TWITTER_MAX_TOTAL_ACTIONS", "25");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    let limits = &config.twitter_activity.engagement_limits;
    assert_eq!(
        limits.max_likes, 12,
        "Env var should override TOML (5 -> 12)"
    );
    assert_eq!(
        limits.max_retweets, 7,
        "Env var should override TOML (3 -> 7)"
    );
    assert_eq!(
        limits.max_follows, 5,
        "Env var should override TOML (2 -> 5)"
    );
    assert_eq!(
        limits.max_replies, 3,
        "Env var should override TOML (1 -> 3)"
    );
    assert_eq!(
        limits.max_total_actions, 25,
        "Env var should override TOML (10 -> 25)"
    );
}

#[test]
fn test_load_config_applies_twitter_engagement_limit_invalid_env_falls_back_to_toml() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[twitter_activity]
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 0
candidate_scan_interval_ms = 0

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "TWITTER_MAX_LIKES",
        "TWITTER_MAX_RETWEETS",
        "TWITTER_MAX_FOLLOWS",
        "TWITTER_MAX_REPLIES",
        "TWITTER_MAX_TOTAL_ACTIONS",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set invalid env vars that cannot be parsed as u32
    env::set_var("TWITTER_MAX_LIKES", "not-a-number");
    env::set_var("TWITTER_MAX_RETWEETS", "");
    env::set_var("TWITTER_MAX_FOLLOWS", "abc");
    env::set_var("TWITTER_MAX_REPLIES", "-5");
    env::set_var("TWITTER_MAX_TOTAL_ACTIONS", "invalid!");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Invalid env vars should fall back to TOML file values, not hardcoded defaults
    let limits = &config.twitter_activity.engagement_limits;
    assert_eq!(
        limits.max_likes, 5,
        "Invalid env var should fall back to TOML value (5)"
    );
    assert_eq!(
        limits.max_retweets, 3,
        "Invalid env var should fall back to TOML value (3)"
    );
    assert_eq!(
        limits.max_follows, 2,
        "Invalid env var should fall back to TOML value (2)"
    );
    assert_eq!(
        limits.max_replies, 1,
        "Invalid env var should fall back to TOML value (1)"
    );
    assert_eq!(
        limits.max_total_actions, 10,
        "Invalid env var should fall back to TOML value (10)"
    );
}

#[test]
fn test_load_config_applies_twitter_probability_env_overrides() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[twitter_activity]
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 0
candidate_scan_interval_ms = 0

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10

[twitter_activity.probabilities]
like_probability = 0.4
retweet_probability = 0.15
quote_probability = 0.15
follow_probability = 0.05
reply_probability = 0.05
bookmark_probability = 0.02
thread_dive_probability = 0.25
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "TWITTER_LIKE_PROBABILITY",
        "TWITTER_RETWEET_PROBABILITY",
        "TWITTER_QUOTE_PROBABILITY",
        "TWITTER_FOLLOW_PROBABILITY",
        "TWITTER_REPLY_PROBABILITY",
        "TWITTER_BOOKMARK_PROBABILITY",
        "TWITTER_THREAD_DIVE_PROBABILITY",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set probability env vars that override TOML values
    env::set_var("TWITTER_LIKE_PROBABILITY", "0.6");
    env::set_var("TWITTER_RETWEET_PROBABILITY", "0.25");
    env::set_var("TWITTER_QUOTE_PROBABILITY", "0.10");
    env::set_var("TWITTER_FOLLOW_PROBABILITY", "0.08");
    env::set_var("TWITTER_REPLY_PROBABILITY", "0.12");
    env::set_var("TWITTER_BOOKMARK_PROBABILITY", "0.04");
    env::set_var("TWITTER_THREAD_DIVE_PROBABILITY", "0.30");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    let probs = &config.twitter_activity.probabilities;
    assert!(
        (probs.like_probability - 0.6).abs() < 1e-9,
        "Like prob should be 0.6 (was {})",
        probs.like_probability
    );
    assert!(
        (probs.retweet_probability - 0.25).abs() < 1e-9,
        "Retweet prob should be 0.25 (was {})",
        probs.retweet_probability
    );
    assert!(
        (probs.quote_probability - 0.10).abs() < 1e-9,
        "Quote prob should be 0.10 (was {})",
        probs.quote_probability
    );
    assert!(
        (probs.follow_probability - 0.08).abs() < 1e-9,
        "Follow prob should be 0.08 (was {})",
        probs.follow_probability
    );
    assert!(
        (probs.reply_probability - 0.12).abs() < 1e-9,
        "Reply prob should be 0.12 (was {})",
        probs.reply_probability
    );
    assert!(
        (probs.bookmark_probability - 0.04).abs() < 1e-9,
        "Bookmark prob should be 0.04 (was {})",
        probs.bookmark_probability
    );
    assert!(
        (probs.thread_dive_probability - 0.30).abs() < 1e-9,
        "Thread dive prob should be 0.30 (was {})",
        probs.thread_dive_probability
    );
}

#[test]
fn test_load_config_applies_twitter_probability_invalid_env_falls_back_to_toml() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[twitter_activity]
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 0
candidate_scan_interval_ms = 0

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10

[twitter_activity.probabilities]
like_probability = 0.4
retweet_probability = 0.15
quote_probability = 0.15
follow_probability = 0.05
reply_probability = 0.05
bookmark_probability = 0.02
thread_dive_probability = 0.25
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "TWITTER_LIKE_PROBABILITY",
        "TWITTER_RETWEET_PROBABILITY",
        "TWITTER_QUOTE_PROBABILITY",
        "TWITTER_FOLLOW_PROBABILITY",
        "TWITTER_REPLY_PROBABILITY",
        "TWITTER_BOOKMARK_PROBABILITY",
        "TWITTER_THREAD_DIVE_PROBABILITY",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set invalid probability env vars that cannot be parsed as f64
    env::set_var("TWITTER_LIKE_PROBABILITY", "not-a-float");
    env::set_var("TWITTER_RETWEET_PROBABILITY", "");
    env::set_var("TWITTER_QUOTE_PROBABILITY", "nope");
    env::set_var("TWITTER_FOLLOW_PROBABILITY", "garbage");
    env::set_var("TWITTER_REPLY_PROBABILITY", "0.5.3");
    env::set_var("TWITTER_BOOKMARK_PROBABILITY", "#commented-out");
    env::set_var("TWITTER_THREAD_DIVE_PROBABILITY", "foo#bar");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Invalid env vars should fall back to TOML file values
    let probs = &config.twitter_activity.probabilities;
    assert!(
        (probs.like_probability - 0.4).abs() < 1e-9,
        "Like prob should fall back to TOML value 0.4 (was {})",
        probs.like_probability
    );
    assert!(
        (probs.retweet_probability - 0.15).abs() < 1e-9,
        "Retweet prob should fall back to TOML value 0.15 (was {})",
        probs.retweet_probability
    );
    assert!(
        (probs.quote_probability - 0.15).abs() < 1e-9,
        "Quote prob should fall back to TOML value 0.15 (was {})",
        probs.quote_probability
    );
    assert!(
        (probs.follow_probability - 0.05).abs() < 1e-9,
        "Follow prob should fall back to TOML value 0.05 (was {})",
        probs.follow_probability
    );
    assert!(
        (probs.reply_probability - 0.05).abs() < 1e-9,
        "Reply prob should fall back to TOML value 0.05 (was {})",
        probs.reply_probability
    );
    assert!(
        (probs.bookmark_probability - 0.02).abs() < 1e-9,
        "Bookmark prob should fall back to TOML value 0.02 (was {})",
        probs.bookmark_probability
    );
    assert!(
        (probs.thread_dive_probability - 0.25).abs() < 1e-9,
        "Thread dive prob should fall back to TOML value 0.25 (was {})",
        probs.thread_dive_probability
    );
}

#[test]
fn test_load_config_applies_browser_orchestrator_env_overrides() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "BROWSER_USER_AGENT",
        "MAX_GLOBAL_CONCURRENCY",
        "CURSOR_OVERLAY_MS",
        "NATIVE_CLICK_CALIBRATION",
        "NATIVE_INPUT_BACKEND",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set env vars that should override TOML values
    env::set_var("BROWSER_USER_AGENT", "TestAgent/1.0");
    env::set_var("MAX_GLOBAL_CONCURRENCY", "15");
    env::set_var("CURSOR_OVERLAY_MS", "250");
    env::set_var("NATIVE_CLICK_CALIBRATION", "mac");
    env::set_var("NATIVE_INPUT_BACKEND", "sendinput");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // String env var (wraps in Some)
    assert_eq!(
        config.browser.user_agent.as_deref(),
        Some("TestAgent/1.0"),
        "BROWSER_USER_AGENT should override to TestAgent/1.0"
    );
    // u32 parse
    assert_eq!(
        config.orchestrator.max_global_concurrency, 15,
        "MAX_GLOBAL_CONCURRENCY should override to 15"
    );
    // u64 parse
    assert_eq!(
        config.browser.cursor_overlay_ms, 250,
        "CURSOR_OVERLAY_MS should override to 250"
    );
    // enum from_env_value
    assert_eq!(
        config.browser.native_interaction.calibration_mode,
        NativeClickCalibrationMode::Mac,
        "NATIVE_CLICK_CALIBRATION should override to Mac"
    );
    assert_eq!(
        config.browser.native_interaction.native_input_backend,
        NativeInputBackend::Sendinput,
        "NATIVE_INPUT_BACKEND should override to Sendinput"
    );
}

#[test]
fn test_load_config_applies_browser_orchestrator_invalid_env_falls_back() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 100
native_interaction = { calibration_mode = "linux", native_input_backend = "rdev", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    let keys = [
        "MAX_GLOBAL_CONCURRENCY",
        "CURSOR_OVERLAY_MS",
        "NATIVE_CLICK_CALIBRATION",
        "native_click_calibration",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set invalid env vars
    env::set_var("MAX_GLOBAL_CONCURRENCY", "not-a-number");
    env::set_var("CURSOR_OVERLAY_MS", "");
    // Invalid enum falls back via from_env_value ("linux" is valid, so use a bogus value)
    env::set_var("NATIVE_CLICK_CALIBRATION", "bogus");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Invalid parse falls back to TOML value
    assert_eq!(
        config.orchestrator.max_global_concurrency, 5,
        "Invalid MAX_GLOBAL_CONCURRENCY should fall back to TOML value (5)"
    );
    // Empty string parse fails, falls back to TOML value
    assert_eq!(
        config.browser.cursor_overlay_ms, 100,
        "Invalid CURSOR_OVERLAY_MS should fall back to TOML value (100)"
    );
    // Invalid enum falls back via from_env_value (to Windows default)
    assert_eq!(
        config.browser.native_interaction.calibration_mode,
        NativeClickCalibrationMode::Windows,
        "Invalid NATIVE_CLICK_CALIBRATION should fall back to Windows default"
    );
}

#[test]
fn test_load_config_applies_remaining_env_overrides() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // TOML with explicit values for all fields that env vars can override
    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://localhost:4444", api_key = "" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 5000, resolve_timeout_ms = 2000, settle_ms = 0 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 60000
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 3
retry_delay_ms = 2000

[twitter_activity]
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 0
candidate_scan_interval_ms = 0

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    // Save and clear env vars
    let keys = [
        "ROXYBROWSER_API_URL",
        "ROXYBROWSER_API_KEY",
        "TASK_TIMEOUT_MS",
        "MAX_RETRIES",
        "BROWSER_EXTRA_HTTP_HEADERS",
        "NATIVE_INTERACTION_STABILITY_WAIT_MS",
        "NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS",
        "NATIVE_INTERACTION_SETTLE_MS",
        "TWITTER_SCROLL_AMOUNT_PIXELS",
        "TWITTER_CANDIDATE_SCAN_INTERVAL_MS",
        "TWITTER_LLM_ENABLED",
        "TWITTER_LLM_PROVIDER",
        "TWITTER_LLM_MODEL",
        "TWITTER_LLM_REPLY_PROBABILITY",
        "TWITTER_LLM_QUOTE_PROBABILITY",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set env vars that should override TOML values
    env::set_var("ROXYBROWSER_API_URL", "https://custom.roxybrowser.com/");
    env::set_var("ROXYBROWSER_API_KEY", "custom-key-456");
    env::set_var("TASK_TIMEOUT_MS", "120000");
    env::set_var("MAX_RETRIES", "7");
    env::set_var(
        "BROWSER_EXTRA_HTTP_HEADERS",
        "X-Custom=value1; X-Debug=true",
    );
    env::set_var("NATIVE_INTERACTION_STABILITY_WAIT_MS", "3000");
    env::set_var("NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS", "1500");
    env::set_var("NATIVE_INTERACTION_SETTLE_MS", "500");
    env::set_var("TWITTER_SCROLL_AMOUNT_PIXELS", "800");
    env::set_var("TWITTER_CANDIDATE_SCAN_INTERVAL_MS", "5000");
    // Twitter LLM env vars
    env::set_var("TWITTER_LLM_ENABLED", "true");
    env::set_var("TWITTER_LLM_PROVIDER", "openrouter");
    env::set_var("TWITTER_LLM_MODEL", "gpt-4");
    env::set_var("TWITTER_LLM_REPLY_PROBABILITY", "0.10");
    env::set_var("TWITTER_LLM_QUOTE_PROBABILITY", "0.08");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    assert!(
        config_dir.join("default.toml").exists(),
        "TOML file must exist before load_config()"
    );
    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Roxybrowser overrides (String)
    assert_eq!(
        config.browser.roxybrowser.api_url, "https://custom.roxybrowser.com/",
        "ROXYBROWSER_API_URL should override TOML value"
    );
    assert_eq!(
        config.browser.roxybrowser.api_key, "custom-key-456",
        "ROXYBROWSER_API_KEY should override TOML value"
    );

    // Orchestrator overrides (u64 parse)
    assert_eq!(
        config.orchestrator.task_timeout_ms.get(),
        120000,
        "TASK_TIMEOUT_MS should override TOML value (60000 -> 120000)"
    );
    // Orchestrator overrides (u32 parse)
    assert_eq!(
        config.orchestrator.max_retries, 7,
        "MAX_RETRIES should override TOML value (3 -> 7)"
    );

    // Browser extra headers (semicolon-separated key=value pairs)
    assert_eq!(config.browser.extra_http_headers.len(), 2);
    assert_eq!(
        config.browser.extra_http_headers.get("X-Custom"),
        Some(&"value1".to_string())
    );
    assert_eq!(
        config.browser.extra_http_headers.get("X-Debug"),
        Some(&"true".to_string())
    );

    // Native interaction overrides (u64 parse)
    assert_eq!(
        config.browser.native_interaction.stability_wait_ms.get(),
        3000,
        "NATIVE_INTERACTION_STABILITY_WAIT_MS should override (5000 -> 3000)"
    );
    assert_eq!(
        config.browser.native_interaction.resolve_timeout_ms.get(),
        1500,
        "NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS should override (2000 -> 1500)"
    );
    assert_eq!(
        config.browser.native_interaction.settle_ms, 500,
        "NATIVE_INTERACTION_SETTLE_MS should override (0 -> 500)"
    );

    // Twitter scroll/scan overrides (i32/u64 parse)
    assert_eq!(
        config.twitter_activity.scroll_amount_pixels, 800,
        "TWITTER_SCROLL_AMOUNT_PIXELS should override (0 -> 800)"
    );
    assert_eq!(
        config.twitter_activity.candidate_scan_interval_ms, 5000,
        "TWITTER_CANDIDATE_SCAN_INTERVAL_MS should override (0 -> 5000)"
    );

    // Twitter LLM overrides
    assert!(
        config.twitter_activity.llm.enabled,
        "TWITTER_LLM_ENABLED should override to true"
    );
    assert_eq!(
        config.twitter_activity.llm.provider, "openrouter",
        "TWITTER_LLM_PROVIDER should override to openrouter"
    );
    assert_eq!(
        config.twitter_activity.llm.model, "gpt-4",
        "TWITTER_LLM_MODEL should override to gpt-4"
    );
    assert!(
        (config.twitter_activity.llm.reply_probability - 0.10).abs() < 1e-9,
        "TWITTER_LLM_REPLY_PROBABILITY should override to 0.10 (was {})",
        config.twitter_activity.llm.reply_probability
    );
    assert!(
        (config.twitter_activity.llm.quote_tweet_probability - 0.08).abs() < 1e-9,
        "TWITTER_LLM_QUOTE_PROBABILITY should override to 0.08 (was {})",
        config.twitter_activity.llm.quote_tweet_probability
    );
}

#[test]
#[ignore]
fn test_load_config_applies_remaining_env_invalid_falls_back() {
    let _guard = config_test_lock().lock().unwrap_or_else(|e| e.into_inner());

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path().join("config");
    fs::create_dir_all(&config_dir).unwrap();

    // TOML with explicit non-default values to verify fallback goes to TOML (not default)
    let toml = r#"
[browser]
connection_timeout_ms = 30000
max_discovery_retries = 3
discovery_retry_delay_ms = 500
circuit_breaker = { enabled = true, failure_threshold = 5, success_threshold = 3, half_open_time_ms = 30000 }
profiles = []
roxybrowser = { enabled = false, api_url = "http://fallback.roxybrowser.com", api_key = "fallback-key" }
cursor_overlay_ms = 0
native_interaction = { calibration_mode = "windows", native_input_backend = "enigo", stability_wait_ms = 9999, resolve_timeout_ms = 8888, settle_ms = 777 }
max_workers_per_session = 5
enable_learning_persistence = true
learning_ttl_days = 30

[orchestrator]
max_global_concurrency = 5
task_timeout_ms = 99999
group_timeout_ms = 300000
worker_wait_timeout_ms = 10000
task_stagger_delay_ms = 500
max_retries = 99
retry_delay_ms = 2000

[twitter_activity]
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
scroll_amount_pixels = 9999
candidate_scan_interval_ms = 8888

[twitter_activity.llm]
enabled = true
provider = "fallback-provider"
model = "fallback-model"
reply_probability = 0.77
quote_tweet_probability = 0.66

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 2
max_quote_tweets = 2
max_total_actions = 10
"#;

    fs::write(config_dir.join("default.toml"), toml).unwrap();

    // Save and clear env vars that could interfere
    // Note: TWITTER_LLM_PROVIDER and TWITTER_LLM_MODEL are included
    // even though we don't set them as env vars — this prevents env
    // leakage from prior tests that may have set them.
    let keys = [
        "TASK_TIMEOUT_MS",
        "MAX_RETRIES",
        "BROWSER_EXTRA_HTTP_HEADERS",
        "NATIVE_INTERACTION_STABILITY_WAIT_MS",
        "NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS",
        "NATIVE_INTERACTION_SETTLE_MS",
        "TWITTER_SCROLL_AMOUNT_PIXELS",
        "TWITTER_CANDIDATE_SCAN_INTERVAL_MS",
        "ROXYBROWSER_API_URL",
        "ROXYBROWSER_API_KEY",
        "TWITTER_LLM_ENABLED",
        "TWITTER_LLM_PROVIDER",
        "TWITTER_LLM_MODEL",
        "TWITTER_LLM_REPLY_PROBABILITY",
        "TWITTER_LLM_QUOTE_PROBABILITY",
    ];
    let saved_env: Vec<(String, Option<OsString>)> = keys
        .iter()
        .map(|key| ((*key).to_string(), env::var_os(key)))
        .collect();
    for (key, _) in &saved_env {
        env::remove_var(key);
    }

    // Set invalid env vars (unparseable values for numeric fields)
    env::set_var("TASK_TIMEOUT_MS", "not-a-number");
    env::set_var("MAX_RETRIES", "");
    env::set_var("BROWSER_EXTRA_HTTP_HEADERS", "");
    env::set_var("NATIVE_INTERACTION_STABILITY_WAIT_MS", "bogus");
    env::set_var("NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS", "");
    env::set_var("NATIVE_INTERACTION_SETTLE_MS", "invalid-u64");
    env::set_var("TWITTER_SCROLL_AMOUNT_PIXELS", "nope");
    env::set_var("TWITTER_CANDIDATE_SCAN_INTERVAL_MS", "xyz");
    // Invalid Twitter LLM env vars
    env::set_var("TWITTER_LLM_ENABLED", "not-boolean");
    env::set_var("TWITTER_LLM_REPLY_PROBABILITY", "not-a-float");
    env::set_var("TWITTER_LLM_QUOTE_PROBABILITY", "0.5.3");

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(temp_dir.path()).unwrap();

    assert!(
        config_dir.join("default.toml").exists(),
        "TOML file must exist before load_config()"
    );
    let config = load_config().unwrap();

    env::set_current_dir(cwd).unwrap();
    for (key, value) in saved_env {
        match value {
            Some(val) => env::set_var(key, val),
            None => env::remove_var(key),
        }
    }

    // Roxybrowser strings — should retain TOML values since no invalid env vars set for them
    assert_eq!(
        config.browser.roxybrowser.api_url, "http://fallback.roxybrowser.com",
        "ROXYBROWSER_API_URL not set, should retain TOML value"
    );
    assert_eq!(
        config.browser.roxybrowser.api_key, "fallback-key",
        "ROXYBROWSER_API_KEY not set, should retain TOML value"
    );

    // Orchestrator numeric fields fall back to TOML values
    assert_eq!(
        config.orchestrator.task_timeout_ms.get(),
        99999,
        "Invalid TASK_TIMEOUT_MS should fall back to TOML value (99999)"
    );
    assert_eq!(
        config.orchestrator.max_retries, 99,
        "Invalid MAX_RETRIES should fall back to TOML value (99)"
    );

    // Empty BROWSER_EXTRA_HTTP_HEADERS — split gives empty map
    assert!(
        config.browser.extra_http_headers.is_empty(),
        "Empty BROWSER_EXTRA_HTTP_HEADERS should clear headers map"
    );

    // Native interaction fields fall back to TOML values
    assert_eq!(
        config.browser.native_interaction.stability_wait_ms.get(),
        9999,
        "Invalid NATIVE_INTERACTION_STABILITY_WAIT_MS should fall back to TOML (9999)"
    );
    assert_eq!(
        config.browser.native_interaction.resolve_timeout_ms.get(),
        8888,
        "Invalid NATIVE_INTERACTION_RESOLVE_TIMEOUT_MS should fall back to TOML (8888)"
    );
    assert_eq!(
        config.browser.native_interaction.settle_ms, 777,
        "Invalid NATIVE_INTERACTION_SETTLE_MS should fall back to TOML (777)"
    );

    // Twitter scroll/scan fields fall back to TOML values
    assert_eq!(
        config.twitter_activity.scroll_amount_pixels, 9999,
        "Invalid TWITTER_SCROLL_AMOUNT_PIXELS should fall back to TOML (9999)"
    );
    assert_eq!(
        config.twitter_activity.candidate_scan_interval_ms, 8888,
        "Invalid TWITTER_CANDIDATE_SCAN_INTERVAL_MS should fall back to TOML (8888)"
    );

    // Twitter LLM: string fields not set via env vars, retain TOML values
    assert!(
        config.twitter_activity.llm.enabled,
        "TWITTER_LLM_ENABLED not set as invalid val (parse fails), should retain TOML value (true)"
    );
    assert_eq!(
        config.twitter_activity.llm.provider, "fallback-provider",
        "TWITTER_LLM_PROVIDER not set as env var, should retain TOML value"
    );
    assert_eq!(
        config.twitter_activity.llm.model, "fallback-model",
        "TWITTER_LLM_MODEL not set as env var, should retain TOML value"
    );
    // Parse-dependent LLM fields fall back to TOML values
    assert!(
        (config.twitter_activity.llm.reply_probability - 0.77).abs() < 1e-9,
        "Invalid TWITTER_LLM_REPLY_PROBABILITY should fall back to TOML (0.77, was {})",
        config.twitter_activity.llm.reply_probability
    );
    assert!(
        (config.twitter_activity.llm.quote_tweet_probability - 0.66).abs() < 1e-9,
        "Invalid TWITTER_LLM_QUOTE_PROBABILITY should fall back to TOML (0.66, was {})",
        config.twitter_activity.llm.quote_tweet_probability
    );
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
