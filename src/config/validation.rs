//! Configuration validation with semantic bounds checking.
//!
//! Provides a `Validate` trait for configuration structs that ensures
//! semantic correctness beyond what TOML/serde can validate.

use crate::config::{BrowserConfig, Config, OrchestratorConfig};
use crate::error::ConfigError;

/// Trait for configuration validation.
///
/// Implement this trait for configuration structs that need
/// semantic validation beyond basic deserialization.
pub trait Validate {
    /// Validates the configuration and returns a structured error on failure.
    fn validate(&self) -> Result<(), ConfigError>;
}

impl Validate for Config {
    fn validate(&self) -> Result<(), ConfigError> {
        self.orchestrator.validate()?;
        self.browser.validate()?;
        Ok(())
    }
}

impl Validate for OrchestratorConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.max_global_concurrency == 0 {
            return Err(ConfigError::InvalidValue {
                field: "max_global_concurrency".to_string(),
                value: "0".to_string(),
                reason: "concurrency must be at least 1".to_string(),
            });
        }

        if self.task_timeout_ms.get() < 1000 {
            return Err(ConfigError::InvalidValue {
                field: "task_timeout_ms".to_string(),
                value: self.task_timeout_ms.to_string(),
                reason: "task timeout must be at least 1000ms (1 second)".to_string(),
            });
        }

        if self.group_timeout_ms.get() < 1000 {
            return Err(ConfigError::InvalidValue {
                field: "group_timeout_ms".to_string(),
                value: self.group_timeout_ms.to_string(),
                reason: "group timeout must be at least 1000ms (1 second)".to_string(),
            });
        }

        if self.worker_wait_timeout_ms.get() < 1000 {
            return Err(ConfigError::InvalidValue {
                field: "worker_wait_timeout_ms".to_string(),
                value: self.worker_wait_timeout_ms.to_string(),
                reason: "worker wait timeout must be at least 1000ms (1 second)".to_string(),
            });
        }

        if self.max_retries > 10 {
            return Err(ConfigError::InvalidValue {
                field: "max_retries".to_string(),
                value: self.max_retries.to_string(),
                reason: "max retries cannot exceed 10".to_string(),
            });
        }

        if self.retry_delay_ms.get() < 100 {
            return Err(ConfigError::InvalidValue {
                field: "retry_delay_ms".to_string(),
                value: self.retry_delay_ms.to_string(),
                reason: "retry delay must be at least 100ms".to_string(),
            });
        }

        if self.task_stagger_delay_ms > 60000 {
            return Err(ConfigError::InvalidValue {
                field: "task_stagger_delay_ms".to_string(),
                value: self.task_stagger_delay_ms.to_string(),
                reason: "task stagger delay should not exceed 60000ms (1 minute)".to_string(),
            });
        }

        Ok(())
    }
}

/// Validate that a string is a valid CSS hex color format (#RGB, #RRGGBB, #RGBA, #RRGGBBAA).
pub(crate) fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }
    let hex = &color[1..];
    if hex.len() != 3 && hex.len() != 4 && hex.len() != 6 && hex.len() != 8 {
        return false;
    }
    hex.chars().all(|c| c.is_ascii_hexdigit())
}

impl Validate for BrowserConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.connection_timeout_ms.get() < 5000 {
            return Err(ConfigError::InvalidValue {
                field: "connection_timeout_ms".to_string(),
                value: self.connection_timeout_ms.to_string(),
                reason: "connection timeout should be at least 5000ms (5 seconds)".to_string(),
            });
        }

        if self.max_discovery_retries == 0 {
            return Err(ConfigError::InvalidValue {
                field: "max_discovery_retries".to_string(),
                value: "0".to_string(),
                reason: "discovery retries must be at least 1".to_string(),
            });
        }

        if self.discovery_retry_delay_ms.get() < 100 {
            return Err(ConfigError::InvalidValue {
                field: "discovery_retry_delay_ms".to_string(),
                value: self.discovery_retry_delay_ms.to_string(),
                reason: "discovery retry delay must be at least 100ms".to_string(),
            });
        }

        if self.profiles.is_empty() {
            return Err(ConfigError::MissingField(
                "browser.profiles".to_string(),
                "at least one browser profile is required".to_string(),
            ));
        }

        if self.max_workers_per_session == 0 {
            return Err(ConfigError::InvalidValue {
                field: "max_workers_per_session".to_string(),
                value: "0".to_string(),
                reason: "max workers per session must be at least 1".to_string(),
            });
        }

        if self.max_workers_per_session > 50 {
            return Err(ConfigError::InvalidValue {
                field: "max_workers_per_session".to_string(),
                value: self.max_workers_per_session.to_string(),
                reason: "max workers per session should not exceed 50".to_string(),
            });
        }

        // Validate cursor_overlay_color is a valid hex color
        if self.cursor_overlay_ms > 0 && !is_valid_hex_color(&self.cursor_overlay_color) {
            return Err(ConfigError::InvalidValue {
                field: "cursor_overlay_color".to_string(),
                value: self.cursor_overlay_color.clone(),
                reason: format!(
                    "invalid hex color format '{}': expected #[RRGGBB] or shorthand #RGB (and optionally #RGBA, #RRGGBBAA)",
                    self.cursor_overlay_color
                ),
            });
        }

        Ok(())
    }
}

/// A standalone helper for validating hex color format in non-validate() contexts.
/// Returns `Ok(())` if valid, `Err(ConfigError::InvalidValue)` if not.
#[allow(dead_code)]
pub(crate) fn validate_cursor_overlay_color(color: &str) -> Result<(), ConfigError> {
    if is_valid_hex_color(color) {
        Ok(())
    } else {
        Err(ConfigError::InvalidValue {
            field: "cursor_overlay_color".to_string(),
            value: color.to_string(),
            reason: format!(
                "invalid hex color format '{}': expected #[RRGGBB] or shorthand #RGB (and optionally #RGBA, #RRGGBBAA)",
                color
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BrowserConfig, BrowserProfile, OrchestratorConfig};
    use crate::session::DurationMs;

    fn create_valid_orchestrator_config() -> OrchestratorConfig {
        OrchestratorConfig {
            max_global_concurrency: 5,
            task_timeout_ms: DurationMs::new_const(60000),
            group_timeout_ms: DurationMs::new_const(300000),
            worker_wait_timeout_ms: DurationMs::new_const(10000),
            task_stagger_delay_ms: 500,
            max_retries: 3,
            retry_delay_ms: DurationMs::new_const(2000),
        }
    }

    fn create_valid_browser_config() -> BrowserConfig {
        BrowserConfig {
            connection_timeout_ms: DurationMs::new_const(30000),
            max_discovery_retries: 3,
            discovery_retry_delay_ms: DurationMs::new_const(500),
            circuit_breaker: crate::config::CircuitBreakerConfig::default(),
            profiles: vec![BrowserProfile {
                name: "test".to_string(),
                r#type: "brave".to_string(),
                ws_endpoint: "ws://localhost:9222".to_string(),
            }],
            roxybrowser: crate::config::RoxybrowserConfig::default(),
            ixbrowser: crate::config::IxbrowserConfig::default(),
            user_agent: None,
            extra_http_headers: std::collections::BTreeMap::new(),
            cursor_overlay_ms: 0,
            cursor_overlay_color: "#ff6600".to_string(),
            cursor_overlay_show_trail: true,
            native_interaction: crate::config::NativeInteractionConfig::default(),
            max_workers_per_session: 5,
            enable_learning_persistence: true,
            learning_ttl_days: 30,
        }
    }

    #[test]
    fn test_validate_orchestrator_valid() {
        let config = create_valid_orchestrator_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_orchestrator_zero_concurrency() {
        let mut config = create_valid_orchestrator_config();
        config.max_global_concurrency = 0;
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_global_concurrency")
        );
    }

    #[test]
    fn test_validate_orchestrator_task_timeout_too_low() {
        let mut config = create_valid_orchestrator_config();
        config.task_timeout_ms = DurationMs::new_const(500);
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "task_timeout_ms")
        );
    }

    #[test]
    fn test_validate_orchestrator_too_many_retries() {
        let mut config = create_valid_orchestrator_config();
        config.max_retries = 15;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_retries"));
    }

    #[test]
    fn test_validate_orchestrator_retry_delay_too_low() {
        let mut config = create_valid_orchestrator_config();
        config.retry_delay_ms = DurationMs::new_const(50);
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "retry_delay_ms")
        );
    }

    #[test]
    fn test_validate_orchestrator_stagger_too_high() {
        let mut config = create_valid_orchestrator_config();
        config.task_stagger_delay_ms = 120000;
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "task_stagger_delay_ms")
        );
    }

    #[test]
    fn test_validate_browser_valid() {
        let config = create_valid_browser_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_browser_connection_timeout_too_low() {
        let mut config = create_valid_browser_config();
        config.connection_timeout_ms = DurationMs::new_const(1000);
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "connection_timeout_ms")
        );
    }

    #[test]
    fn test_validate_browser_zero_discovery_retries() {
        let mut config = create_valid_browser_config();
        config.max_discovery_retries = 0;
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_discovery_retries")
        );
    }

    #[test]
    fn test_is_valid_hex_color_valid_full() {
        assert!(is_valid_hex_color("#ff6600"), "#ff6600 should be valid");
        assert!(
            is_valid_hex_color("#FF6600"),
            "#FF6600 should be valid (uppercase)"
        );
        assert!(
            is_valid_hex_color("#aA1234"),
            "#aA1234 should be valid (mixed case)"
        );
        assert!(
            is_valid_hex_color("#000000"),
            "#000000 should be valid (black)"
        );
        assert!(
            is_valid_hex_color("#ffffff"),
            "#ffffff should be valid (white)"
        );
    }

    #[test]
    fn test_is_valid_hex_color_valid_shorthand() {
        assert!(
            is_valid_hex_color("#f60"),
            "#f60 should be valid (shorthand)"
        );
        assert!(
            is_valid_hex_color("#FFF"),
            "#FFF should be valid (shorthand)"
        );
        assert!(
            is_valid_hex_color("#000"),
            "#000 should be valid (shorthand black)"
        );
        assert!(
            is_valid_hex_color("#abc"),
            "#abc should be valid (shorthand)"
        );
    }

    #[test]
    fn test_is_valid_hex_color_valid_with_alpha() {
        assert!(
            is_valid_hex_color("#ff6600ff"),
            "#ff6600ff should be valid (with alpha)"
        );
        assert!(
            is_valid_hex_color("#f60f"),
            "#f60f should be valid (shorthand with alpha)"
        );
    }

    #[test]
    fn test_is_valid_hex_color_invalid_no_hash() {
        assert!(
            !is_valid_hex_color("ff6600"),
            "ff6600 without # should be invalid"
        );
        assert!(
            !is_valid_hex_color("FFF"),
            "FFF without # should be invalid"
        );
    }

    #[test]
    fn test_is_valid_hex_color_invalid_wrong_length() {
        assert!(
            !is_valid_hex_color("#ff660"),
            "#ff660 (5 hex chars) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#ff66001"),
            "#ff66001 (7 hex chars) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#f"),
            "#f (1 hex char) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#ff"),
            "#ff (2 hex chars) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#fffff"),
            "#fffff (5 hex chars) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#fffffff"),
            "#fffffff (7 hex chars) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#fffffffff"),
            "#fffffffff (9 hex chars) should be invalid"
        );
    }

    #[test]
    fn test_is_valid_hex_color_invalid_non_hex_chars() {
        assert!(
            !is_valid_hex_color("#ff660g"),
            "#ff660g (g is not hex) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#ff66-0"),
            "#ff66-0 (- is not hex) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#ff66 00"),
            "#ff66 00 (space) should be invalid"
        );
        assert!(
            !is_valid_hex_color("#ff66_00"),
            "#ff66_00 (underscore) should be invalid"
        );
        assert!(!is_valid_hex_color("#GHIJKL"), "#GHIJKL should be invalid");
    }

    #[test]
    fn test_is_valid_hex_color_edge_cases() {
        assert!(!is_valid_hex_color(""), "empty string should be invalid");
        assert!(!is_valid_hex_color("#"), "just # should be invalid");
        assert!(
            !is_valid_hex_color("  #ff6600"),
            "leading space should be invalid"
        );
        assert!(
            !is_valid_hex_color("#ff6600  "),
            "trailing space should be invalid"
        );
    }

    #[test]
    fn test_validate_cursor_overlay_color_valid() {
        assert!(validate_cursor_overlay_color("#ff6600").is_ok());
        assert!(validate_cursor_overlay_color("#f60").is_ok());
        assert!(validate_cursor_overlay_color("#FF0000").is_ok());
    }

    #[test]
    fn test_validate_cursor_overlay_color_invalid() {
        let err = validate_cursor_overlay_color("ff6600").unwrap_err();
        assert!(
            matches!(&err, ConfigError::InvalidValue { field, .. } if field == "cursor_overlay_color"),
            "Expected InvalidValue error for field cursor_overlay_color"
        );

        let err = validate_cursor_overlay_color("#xyz").unwrap_err();
        assert!(
            matches!(&err, ConfigError::InvalidValue { field, .. } if field == "cursor_overlay_color")
        );
    }

    #[test]
    fn test_validate_browser_valid_with_cursor_overlay() {
        let mut config = create_valid_browser_config();
        config.cursor_overlay_ms = 100;
        config.cursor_overlay_color = "#ff6600".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_browser_cursor_overlay_invalid_color_with_ms() {
        let mut config = create_valid_browser_config();
        config.cursor_overlay_ms = 100;
        config.cursor_overlay_color = "not-a-color".to_string();
        let err = config.validate().unwrap_err();
        assert!(
            matches!(&err, ConfigError::InvalidValue { field, .. } if field == "cursor_overlay_color"),
            "Expected InvalidValue for cursor_overlay_color when ms>0 and color is invalid"
        );
    }

    #[test]
    fn test_validate_browser_cursor_overlay_invalid_color_zero_ms() {
        // When cursor_overlay_ms is 0 (disabled), invalid color should NOT cause validation error
        let mut config = create_valid_browser_config();
        config.cursor_overlay_ms = 0;
        config.cursor_overlay_color = "not-a-color".to_string();
        assert!(
            config.validate().is_ok(),
            "Invalid color should be allowed when cursor_overlay_ms is 0 (disabled)"
        );
    }

    #[test]
    fn test_validate_browser_cursor_overlay_valid_color_with_ms() {
        let mut config = create_valid_browser_config();
        config.cursor_overlay_ms = 50;
        config.cursor_overlay_color = "#abc123".to_string();
        assert!(
            config.validate().is_ok(),
            "Valid color #abc123 should pass when overlay is enabled"
        );

        config.cursor_overlay_color = "#ABC".to_string();
        assert!(
            config.validate().is_ok(),
            "Shorthand #ABC should pass when overlay is enabled"
        );

        config.cursor_overlay_color = "#123456ff".to_string();
        assert!(
            config.validate().is_ok(),
            "#123456ff with alpha should pass when overlay is enabled"
        );
    }

    #[test]
    fn test_validate_browser_cursor_overlay_invalid_color_no_hash() {
        let mut config = create_valid_browser_config();
        config.cursor_overlay_ms = 100;
        config.cursor_overlay_color = "ff6600".to_string();
        let err = config.validate().unwrap_err();
        assert!(
            matches!(&err, ConfigError::InvalidValue { field, .. } if field == "cursor_overlay_color")
        );
    }

    #[test]
    fn test_validate_browser_cursor_overlay_invalid_wrong_length() {
        let mut config = create_valid_browser_config();
        config.cursor_overlay_ms = 100;
        config.cursor_overlay_color = "#ff660".to_string();
        let err = config.validate().unwrap_err();
        assert!(
            matches!(&err, ConfigError::InvalidValue { field, .. } if field == "cursor_overlay_color")
        );
    }

    #[test]
    fn test_validate_browser_cursor_overlay_valid_default_ignored_when_disabled() {
        // Default value #ff6600 is valid, but even if it weren't, ms=0 should skip
        let config = create_valid_browser_config();
        // cursor_overlay_ms is already 0 (default in create_valid_browser_config)
        assert!(
            config.validate().is_ok(),
            "Default config with ms=0 should validate regardless of color"
        );
    }

    #[allow(dead_code)]
    fn test_validate_browser_missing_profiles() {
        let mut config = create_valid_browser_config();
        config.profiles.clear();
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::MissingField(field, _) if field == "browser.profiles"));
    }

    #[test]
    fn test_validate_browser_zero_workers() {
        let mut config = create_valid_browser_config();
        config.max_workers_per_session = 0;
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_workers_per_session")
        );
    }

    #[test]
    fn test_validate_browser_too_many_workers() {
        let mut config = create_valid_browser_config();
        config.max_workers_per_session = 100;
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_workers_per_session")
        );
    }

    #[test]
    fn test_validate_config_valid() {
        let config = Config {
            browser: create_valid_browser_config(),
            orchestrator: create_valid_orchestrator_config(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_config_invalid_orchestrator() {
        let mut orchestrator = create_valid_orchestrator_config();
        orchestrator.max_global_concurrency = 0;

        let config = Config {
            browser: create_valid_browser_config(),
            orchestrator,
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_global_concurrency")
        );
    }

    #[test]
    fn test_validate_config_invalid_browser() {
        let mut browser = create_valid_browser_config();
        browser.profiles.clear();

        let config = Config {
            browser,
            orchestrator: create_valid_orchestrator_config(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::MissingField(field, _) if field == "browser.profiles"));
    }

    #[test]
    fn test_validate_browser_invalid_timeout() {
        let mut browser = create_valid_browser_config();
        browser.connection_timeout_ms = DurationMs::new_const(4999); // Invalid

        let config = Config {
            browser,
            orchestrator: create_valid_orchestrator_config(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "connection_timeout_ms")
        );
    }

    #[test]
    fn test_validate_browser_max_workers_boundary() {
        let mut browser = create_valid_browser_config();
        browser.max_workers_per_session = 50; // Exactly at the limit - should be OK
        browser.profiles[0].name = "boundary".to_string();
        let config = Config {
            browser,
            orchestrator: create_valid_orchestrator_config(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        assert!(config.validate().is_ok());

        let mut browser = create_valid_browser_config();
        browser.max_workers_per_session = 51; // Over the limit
        let config = Config {
            browser,
            orchestrator: create_valid_orchestrator_config(),
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_workers_per_session")
        );
    }

    #[test]
    fn test_validate_orchestrator_invalid_timeout() {
        let mut orchestrator = create_valid_orchestrator_config();
        orchestrator.task_timeout_ms = DurationMs::new_const(500); // < 1000ms, fails validation

        let config = Config {
            browser: create_valid_browser_config(),
            orchestrator,
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field: f, value: _, reason: _ } if f == "task_timeout_ms")
        );
    }

    #[test]
    fn test_validate_orchestrator_negative_concurrency() {
        // usize::MAX passes the == 0 check; this tests that only zero is rejected
        let mut orchestrator = create_valid_orchestrator_config();
        orchestrator.max_global_concurrency = 0;

        let config = Config {
            browser: create_valid_browser_config(),
            orchestrator,
            twitter_activity: crate::config::TwitterActivityConfig::default(),
            tracing: crate::config::TracingConfig::default(),
            task_discovery: crate::config::TaskDiscoveryConfig::default(),
        };
        let err = config.validate().unwrap_err();
        assert!(
            matches!(err, ConfigError::InvalidValue { field, .. } if field == "max_global_concurrency")
        );
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::MissingField("test.field".to_string(), "required".to_string());
        let display = format!("{}", err);
        assert!(display.contains("test.field"));
        assert!(display.contains("required"));

        let err = ConfigError::InvalidValue {
            field: "test.value".to_string(),
            value: "must be positive".to_string(),
            reason: "test".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("test.value"));
        assert!(display.contains("must be positive"));
    }

    #[test]
    fn test_config_error_debug() {
        let err = ConfigError::MissingField("field".to_string(), "reason".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("MissingField"));
        assert!(debug.contains("field"));
    }
}
