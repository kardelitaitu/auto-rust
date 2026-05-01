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

        if self.task_timeout_ms < 1000 {
            return Err(ConfigError::InvalidValue {
                field: "task_timeout_ms".to_string(),
                value: self.task_timeout_ms.to_string(),
                reason: "task timeout must be at least 1000ms (1 second)".to_string(),
            });
        }

        if self.group_timeout_ms < 1000 {
            return Err(ConfigError::InvalidValue {
                field: "group_timeout_ms".to_string(),
                value: self.group_timeout_ms.to_string(),
                reason: "group timeout must be at least 1000ms (1 second)".to_string(),
            });
        }

        if self.worker_wait_timeout_ms < 1000 {
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

        if self.retry_delay_ms < 100 {
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

impl Validate for BrowserConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.connection_timeout_ms < 5000 {
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

        if self.discovery_retry_delay_ms < 100 {
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BrowserConfig, BrowserProfile, BrowserType, OrchestratorConfig};

    fn create_valid_orchestrator_config() -> OrchestratorConfig {
        OrchestratorConfig {
            max_global_concurrency: 5,
            task_timeout_ms: 60000,
            group_timeout_ms: 300000,
            worker_wait_timeout_ms: 10000,
            task_stagger_delay_ms: 500,
            max_retries: 3,
            retry_delay_ms: 2000,
        }
    }

    fn create_valid_browser_config() -> BrowserConfig {
        BrowserConfig {
            connection_timeout_ms: 30000,
            max_discovery_retries: 3,
            discovery_retry_delay_ms: 500,
            circuit_breaker: crate::config::CircuitBreakerConfig::default(),
            profiles: vec![BrowserProfile {
                name: "test".to_string(),
                path: "/usr/bin/brave".to_string(),
                browser_type: BrowserType::Brave,
            }],
            roxybrowser: crate::config::RoxybrowserConfig::default(),
            user_agent: None,
            extra_http_headers: std::collections::BTreeMap::new(),
            cursor_overlay_ms: 0,
            native_interaction: crate::config::NativeInteractionConfig::default(),
            max_workers_per_session: 5,
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
        config.task_timeout_ms = 500;
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
        config.retry_delay_ms = 50;
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
        config.connection_timeout_ms = 1000;
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
        };
        let err = config.validate().unwrap_err();
        assert!(matches!(err, ConfigError::MissingField(field, _) if field == "browser.profiles"));
    }
}
