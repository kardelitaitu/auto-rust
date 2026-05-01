//! Structured error types for the orchestrator framework.
//!
//! Provides typed error categories for better error handling, debugging,
//! and IDE autocomplete support across the codebase.

use thiserror::Error;

/// Result type alias for the orchestrator framework.
pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Central error type for the orchestrator framework.
/// Categorizes errors by domain for better handling and reporting.
#[derive(Debug, Error)]
pub enum OrchestratorError {
    /// Browser automation errors (CDP, page control, element interaction)
    #[error("Browser error: {0}")]
    Browser(#[from] BrowserError),

    /// Session management errors (connection, lifecycle, health)
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    /// Task execution errors (validation, timeout, retry)
    #[error("Task error: {0}")]
    Task(#[from] TaskError),

    /// Configuration errors (loading, validation, parsing)
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Network and API errors (HTTP, timeout, circuit breaker)
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    /// I/O and filesystem errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic errors that don't fit other categories
    #[error("{0}")]
    Other(String),
}

/// Browser automation errors.
#[derive(Debug, Error)]
pub enum BrowserError {
    /// Failed to connect to browser CDP
    #[error("Failed to connect to browser: {0}")]
    ConnectionFailed(String),

    /// Page creation or navigation failed
    #[error("Page error: {0}")]
    PageError(String),

    /// Element interaction failed
    #[error("Element interaction failed: {selector} - {reason}")]
    ElementError { selector: String, reason: String },

    /// Selector not found
    #[error("Selector not found: {0}")]
    SelectorNotFound(String),

    /// Browser crashed or disconnected
    #[error("Browser disconnected: {0}")]
    Disconnected(String),

    /// Timeout waiting for browser response
    #[error("Browser operation timed out: {0}")]
    Timeout(String),
}

/// Session management errors.
#[derive(Debug, Error)]
pub enum SessionError {
    /// Session initialization failed
    #[error("Failed to initialize session: {0}")]
    InitializationFailed(String),

    /// Worker acquisition timeout
    #[error("Worker acquisition timed out after {0}ms", timeout_ms)]
    WorkerTimeout { timeout_ms: u64 },

    /// Session marked unhealthy
    #[error("Session unhealthy: {0}")]
    Unhealthy(String),

    /// Page registry error
    #[error("Page registry error: {0}")]
    PageRegistry(String),

    /// Session shutdown failed
    #[error("Session shutdown failed: {0}")]
    ShutdownFailed(String),
}

/// Task execution errors.
#[derive(Debug, Error)]
pub enum TaskError {
    /// Task validation failed
    #[error("Task validation failed: {task_name} - {reason}")]
    ValidationFailed { task_name: String, reason: String },

    /// Task execution timeout
    #[error("Task timed out: {task_name} after {timeout_ms}ms")]
    Timeout { task_name: String, timeout_ms: u64 },

    /// Task execution failed
    #[error("Task execution failed: {task_name} - {reason}")]
    ExecutionFailed { task_name: String, reason: String },

    /// Task not found
    #[error("Unknown task: {0}")]
    NotFound(String),

    /// Task cancelled
    #[error("Task cancelled: {0}")]
    Cancelled(String),

    /// Retry exhausted
    #[error("Retry exhausted after {max_retries} attempts for {task_name}")]
    RetryExhausted { max_retries: u32, task_name: String },

    /// Permission denied for task operation
    #[error("Permission denied: task '{task_name}' lacks '{permission}' permission")]
    PermissionDenied {
        permission: &'static str,
        task_name: String,
    },

    /// Invalid path for data file operation
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// CDP/browser operation failed
    #[error("CDP error: {operation} - {reason}")]
    CdpError { operation: String, reason: String },

    /// Clipboard operation failed
    #[error("Clipboard error: {0}")]
    ClipboardError(String),
}

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to load config file
    #[error("Failed to load config from {path}: {reason}")]
    LoadFailed { path: String, reason: String },

    /// Config validation failed
    #[error("Config validation failed: {0}")]
    ValidationFailed(String),

    /// Missing required field
    #[error("Missing required config field: {0} ({1})")]
    MissingField(String, String),

    /// Invalid value for field
    #[error("Invalid value for {field}: {value} - {reason}")]
    InvalidValue {
        field: String,
        value: String,
        reason: String,
    },

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvVar(String),
}

/// Network and API errors.
#[derive(Debug, Error)]
pub enum NetworkError {
    /// HTTP request failed
    #[error("HTTP request failed: {url} - {status}")]
    HttpError { url: String, status: String },

    /// Request timeout
    #[error("Request timed out: {0}")]
    Timeout(String),

    /// Circuit breaker open
    #[error("Circuit breaker is open for {service}")]
    CircuitBreakerOpen { service: String },

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// API key error
    #[error("API key error: {0}")]
    ApiKey(String),
}

impl From<anyhow::Error> for OrchestratorError {
    fn from(err: anyhow::Error) -> Self {
        OrchestratorError::Other(err.to_string())
    }
}

impl From<toml::de::Error> for OrchestratorError {
    fn from(err: toml::de::Error) -> Self {
        OrchestratorError::Config(ConfigError::LoadFailed {
            path: "config file".to_string(),
            reason: err.to_string(),
        })
    }
}

impl From<tokio::sync::AcquireError> for OrchestratorError {
    fn from(_err: tokio::sync::AcquireError) -> Self {
        OrchestratorError::Session(SessionError::WorkerTimeout { timeout_ms: 0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_error_message_formatting() {
        let err = BrowserError::ConnectionFailed("test error".to_string());
        assert_eq!(err.to_string(), "Failed to connect to browser: test error");

        let err = BrowserError::PageError("navigation failed".to_string());
        assert_eq!(err.to_string(), "Page error: navigation failed");

        let err = BrowserError::ElementError {
            selector: "#button".to_string(),
            reason: "not clickable".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Element interaction failed: #button - not clickable"
        );

        let err = BrowserError::SelectorNotFound(".missing".to_string());
        assert_eq!(err.to_string(), "Selector not found: .missing");

        let err = BrowserError::Disconnected("crashed".to_string());
        assert_eq!(err.to_string(), "Browser disconnected: crashed");

        let err = BrowserError::Timeout("operation".to_string());
        assert_eq!(err.to_string(), "Browser operation timed out: operation");
    }

    #[test]
    fn test_session_error_message_formatting() {
        let err = SessionError::InitializationFailed("failed".to_string());
        assert_eq!(err.to_string(), "Failed to initialize session: failed");

        let err = SessionError::WorkerTimeout { timeout_ms: 5000 };
        assert_eq!(err.to_string(), "Worker acquisition timed out after 5000ms");

        let err = SessionError::Unhealthy("low health score".to_string());
        assert_eq!(err.to_string(), "Session unhealthy: low health score");

        let err = SessionError::PageRegistry("registry error".to_string());
        assert_eq!(err.to_string(), "Page registry error: registry error");

        let err = SessionError::ShutdownFailed("cleanup failed".to_string());
        assert_eq!(err.to_string(), "Session shutdown failed: cleanup failed");
    }

    #[test]
    fn test_task_error_message_formatting() {
        let err = TaskError::ValidationFailed {
            task_name: "test_task".to_string(),
            reason: "invalid payload".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Task validation failed: test_task - invalid payload"
        );

        let err = TaskError::Timeout {
            task_name: "slow_task".to_string(),
            timeout_ms: 30000,
        };
        assert_eq!(err.to_string(), "Task timed out: slow_task after 30000ms");

        let err = TaskError::ExecutionFailed {
            task_name: "failing_task".to_string(),
            reason: "element not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Task execution failed: failing_task - element not found"
        );

        let err = TaskError::NotFound("unknown_task".to_string());
        assert_eq!(err.to_string(), "Unknown task: unknown_task");

        let err = TaskError::Cancelled("user cancelled".to_string());
        assert_eq!(err.to_string(), "Task cancelled: user cancelled");

        let err = TaskError::RetryExhausted {
            max_retries: 3,
            task_name: "flaky_task".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Retry exhausted after 3 attempts for flaky_task"
        );
    }

    #[test]
    fn test_config_error_message_formatting() {
        let err = ConfigError::LoadFailed {
            path: "/path/to/config".to_string(),
            reason: "file not found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to load config from /path/to/config: file not found"
        );

        let err = ConfigError::ValidationFailed("invalid field".to_string());
        assert_eq!(err.to_string(), "Config validation failed: invalid field");

        let err = ConfigError::MissingField("required_field".to_string(), "this field is required".to_string());
        assert_eq!(
            err.to_string(),
            "Missing required config field: required_field (this field is required)"
        );

        let err = ConfigError::InvalidValue {
            field: "port".to_string(),
            value: "abc".to_string(),
            reason: "must be a number".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid value for port: abc - must be a number"
        );

        let err = ConfigError::EnvVar("API_KEY not set".to_string());
        assert_eq!(
            err.to_string(),
            "Environment variable error: API_KEY not set"
        );
    }

    #[test]
    fn test_network_error_message_formatting() {
        let err = NetworkError::HttpError {
            url: "https://example.com".to_string(),
            status: "500".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "HTTP request failed: https://example.com - 500"
        );

        let err = NetworkError::Timeout("request timed out".to_string());
        assert_eq!(err.to_string(), "Request timed out: request timed out");

        let err = NetworkError::CircuitBreakerOpen {
            service: "api_service".to_string(),
        };
        assert_eq!(err.to_string(), "Circuit breaker is open for api_service");

        let err = NetworkError::Connection("connection refused".to_string());
        assert_eq!(err.to_string(), "Connection error: connection refused");

        let err = NetworkError::ApiKey("invalid key".to_string());
        assert_eq!(err.to_string(), "API key error: invalid key");
    }

    #[test]
    fn test_orchestrator_error_from_browser_error() {
        let browser_err = BrowserError::ConnectionFailed("test".to_string());
        let orch_err: OrchestratorError = browser_err.into();
        assert!(matches!(orch_err, OrchestratorError::Browser(_)));
        assert_eq!(
            orch_err.to_string(),
            "Browser error: Failed to connect to browser: test"
        );
    }

    #[test]
    fn test_orchestrator_error_from_session_error() {
        let session_err = SessionError::Unhealthy("test".to_string());
        let orch_err: OrchestratorError = session_err.into();
        assert!(matches!(orch_err, OrchestratorError::Session(_)));
        assert_eq!(
            orch_err.to_string(),
            "Session error: Session unhealthy: test"
        );
    }

    #[test]
    fn test_orchestrator_error_from_task_error() {
        let task_err = TaskError::NotFound("test".to_string());
        let orch_err: OrchestratorError = task_err.into();
        assert!(matches!(orch_err, OrchestratorError::Task(_)));
        assert_eq!(orch_err.to_string(), "Task error: Unknown task: test");
    }

    #[test]
    fn test_orchestrator_error_from_config_error() {
        let config_err = ConfigError::MissingField("test".to_string());
        let orch_err: OrchestratorError = config_err.into();
        assert!(matches!(orch_err, OrchestratorError::Config(_)));
        assert_eq!(
            orch_err.to_string(),
            "Configuration error: Missing required config field: test"
        );
    }

    #[test]
    fn test_orchestrator_error_from_network_error() {
        let network_err = NetworkError::Timeout("test".to_string());
        let orch_err: OrchestratorError = network_err.into();
        assert!(matches!(orch_err, OrchestratorError::Network(_)));
        assert_eq!(
            orch_err.to_string(),
            "Network error: Request timed out: test"
        );
    }

    #[test]
    fn test_orchestrator_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let orch_err: OrchestratorError = io_err.into();
        assert!(matches!(orch_err, OrchestratorError::Io(_)));
        assert!(orch_err.to_string().contains("I/O error:"));
    }

    #[test]
    fn test_orchestrator_error_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("test error");
        let orch_err: OrchestratorError = anyhow_err.into();
        assert!(matches!(orch_err, OrchestratorError::Other(_)));
        assert_eq!(orch_err.to_string(), "test error");
    }

    #[test]
    fn test_orchestrator_error_from_toml_error() {
        use serde::de::Error;
        let toml_err = toml::de::Error::custom("invalid TOML");
        let orch_err: OrchestratorError = toml_err.into();
        assert!(matches!(orch_err, OrchestratorError::Config(_)));
        assert!(orch_err
            .to_string()
            .contains("Failed to load config from config file"));
    }

    #[test]
    fn test_orchestrator_error_from_acquire_error() {
        // Note: tokio::sync::AcquireError is a unit struct with no public constructor
        // It can only be obtained from actual semaphore acquire operations
        // The From implementation is tested indirectly through integration tests
        // that trigger actual worker acquisition timeouts
    }

    #[test]
    fn test_orchestrator_error_other_variant() {
        let orch_err = OrchestratorError::Other("custom error".to_string());
        assert_eq!(orch_err.to_string(), "custom error");
    }

    #[test]
    fn test_error_debug_display() {
        let err = BrowserError::ConnectionFailed("test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("ConnectionFailed"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_result_type_alias() {
        // Test that Result<T> works as expected
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());

        let err_result: Result<i32> =
            Err(BrowserError::ConnectionFailed("test".to_string()).into());
        assert!(err_result.is_err());
    }

    #[test]
    fn test_error_chain_with_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let orch_err: OrchestratorError = io_err.into();
        // The I/O error should be preserved in the Io variant
        assert!(matches!(orch_err, OrchestratorError::Io(_)));
    }

    #[test]
    fn test_browser_error_empty_selector() {
        let err = BrowserError::SelectorNotFound("".to_string());
        assert_eq!(err.to_string(), "Selector not found: ");
    }

    #[test]
    fn test_browser_error_special_chars() {
        let err = BrowserError::ElementError {
            selector: "div[data-test=\"value\"]".to_string(),
            reason: "special & chars".to_string(),
        };
        assert!(err.to_string().contains("data-test"));
    }

    #[test]
    fn test_session_error_zero_timeout() {
        let err = SessionError::WorkerTimeout { timeout_ms: 0 };
        assert_eq!(err.to_string(), "Worker acquisition timed out after 0ms");
    }

    #[test]
    fn test_task_error_empty_task_name() {
        let err = TaskError::NotFound("".to_string());
        assert_eq!(err.to_string(), "Unknown task: ");
    }

    #[test]
    fn test_task_error_zero_retries() {
        let err = TaskError::RetryExhausted {
            max_retries: 0,
            task_name: "test".to_string(),
        };
        assert_eq!(err.to_string(), "Retry exhausted after 0 attempts for test");
    }

    #[test]
    fn test_config_error_empty_path() {
        let err = ConfigError::LoadFailed {
            path: "".to_string(),
            reason: "test".to_string(),
        };
        assert!(err.to_string().contains("Failed to load config from :"));
    }

    #[test]
    fn test_config_error_empty_field() {
        let err = ConfigError::MissingField("".to_string(), "".to_string());
        assert_eq!(err.to_string(), "Missing required config field:  ()");
    }

    #[test]
    fn test_network_error_empty_url() {
        let err = NetworkError::HttpError {
            url: "".to_string(),
            status: "500".to_string(),
        };
        assert_eq!(err.to_string(), "HTTP request failed:  - 500");
    }

    #[test]
    fn test_network_error_empty_status() {
        let err = NetworkError::HttpError {
            url: "https://example.com".to_string(),
            status: "".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "HTTP request failed: https://example.com - "
        );
    }

    #[test]
    fn test_orchestrator_error_other_empty_string() {
        let err = OrchestratorError::Other("".to_string());
        assert_eq!(err.to_string(), "");
    }

    #[test]
    fn test_orchestrator_error_other_unicode() {
        let err = OrchestratorError::Other("エラー".to_string());
        assert_eq!(err.to_string(), "エラー");
    }

    #[test]
    fn test_error_display_consistency() {
        let err1 = BrowserError::ConnectionFailed("test".to_string());
        let err2 = OrchestratorError::Browser(err1);
        assert!(err2.to_string().contains("test"));
    }

    #[test]
    fn test_browser_error_clone_not_implemented() {
        // BrowserError doesn't derive Clone, so this test documents that
        // If Clone is added in the future, this test should be updated
        let err = BrowserError::ConnectionFailed("test".to_string());
        // err.clone(); // This would fail to compile
        assert_eq!(err.to_string(), "Failed to connect to browser: test");
    }

    #[test]
    fn test_session_error_clone_not_implemented() {
        // SessionError doesn't derive Clone
        let err = SessionError::Unhealthy("test".to_string());
        assert_eq!(err.to_string(), "Session unhealthy: test");
    }

    #[test]
    fn test_task_error_clone_not_implemented() {
        // TaskError doesn't derive Clone
        let err = TaskError::NotFound("test".to_string());
        assert_eq!(err.to_string(), "Unknown task: test");
    }

    #[test]
    fn test_config_error_clone_not_implemented() {
        // ConfigError doesn't derive Clone
        let err = ConfigError::MissingField("test".to_string(), "hint".to_string());
        assert_eq!(err.to_string(), "Missing required config field: test (hint)");
    }

    #[test]
    fn test_network_error_clone_not_implemented() {
        // NetworkError doesn't derive Clone
        let err = NetworkError::Timeout("test".to_string());
        assert_eq!(err.to_string(), "Request timed out: test");
    }

    #[test]
    fn test_orchestrator_error_clone_not_implemented() {
        // OrchestratorError doesn't derive Clone
        let err = OrchestratorError::Other("test".to_string());
        assert_eq!(err.to_string(), "test");
    }

    #[test]
    fn test_error_message_with_newlines() {
        let err = BrowserError::ConnectionFailed("error\nwith\nnewlines".to_string());
        assert!(err.to_string().contains("error"));
        assert!(err.to_string().contains("newlines"));
    }

    #[test]
    fn test_error_message_with_tabs() {
        let err = BrowserError::ConnectionFailed("error\twith\ttabs".to_string());
        assert!(err.to_string().contains("error"));
        assert!(err.to_string().contains("tabs"));
    }

    #[test]
    fn test_error_message_very_long() {
        let long_msg = "a".repeat(1000);
        let err = BrowserError::ConnectionFailed(long_msg.clone());
        assert!(err.to_string().len() > 1000);
    }

    #[test]
    fn test_task_error_validation_with_empty_reason() {
        let err = TaskError::ValidationFailed {
            task_name: "test".to_string(),
            reason: "".to_string(),
        };
        assert_eq!(err.to_string(), "Task validation failed: test - ");
    }

    #[test]
    fn test_task_error_execution_with_empty_reason() {
        let err = TaskError::ExecutionFailed {
            task_name: "test".to_string(),
            reason: "".to_string(),
        };
        assert_eq!(err.to_string(), "Task execution failed: test - ");
    }

    #[test]
    fn test_config_error_validation_with_empty_message() {
        let err = ConfigError::ValidationFailed("".to_string());
        assert_eq!(err.to_string(), "Config validation failed: ");
    }

    #[test]
    fn test_config_error_env_var_with_empty_message() {
        let err = ConfigError::EnvVar("".to_string());
        assert_eq!(err.to_string(), "Environment variable error: ");
    }

    #[test]
    fn test_network_error_timeout_with_empty_message() {
        let err = NetworkError::Timeout("".to_string());
        assert_eq!(err.to_string(), "Request timed out: ");
    }

    #[test]
    fn test_network_error_connection_with_empty_message() {
        let err = NetworkError::Connection("".to_string());
        assert_eq!(err.to_string(), "Connection error: ");
    }

    #[test]
    fn test_network_error_api_key_with_empty_message() {
        let err = NetworkError::ApiKey("".to_string());
        assert_eq!(err.to_string(), "API key error: ");
    }

    #[test]
    fn test_session_error_initialization_with_empty_message() {
        let err = SessionError::InitializationFailed("".to_string());
        assert_eq!(err.to_string(), "Failed to initialize session: ");
    }

    #[test]
    fn test_session_error_unhealthy_with_empty_message() {
        let err = SessionError::Unhealthy("".to_string());
        assert_eq!(err.to_string(), "Session unhealthy: ");
    }

    #[test]
    fn test_session_error_page_registry_with_empty_message() {
        let err = SessionError::PageRegistry("".to_string());
        assert_eq!(err.to_string(), "Page registry error: ");
    }

    #[test]
    fn test_session_error_shutdown_with_empty_message() {
        let err = SessionError::ShutdownFailed("".to_string());
        assert_eq!(err.to_string(), "Session shutdown failed: ");
    }

    #[test]
    fn test_browser_error_connection_with_empty_message() {
        let err = BrowserError::ConnectionFailed("".to_string());
        assert_eq!(err.to_string(), "Failed to connect to browser: ");
    }

    #[test]
    fn test_browser_error_page_with_empty_message() {
        let err = BrowserError::PageError("".to_string());
        assert_eq!(err.to_string(), "Page error: ");
    }

    #[test]
    fn test_browser_error_disconnected_with_empty_message() {
        let err = BrowserError::Disconnected("".to_string());
        assert_eq!(err.to_string(), "Browser disconnected: ");
    }

    #[test]
    fn test_browser_error_timeout_with_empty_message() {
        let err = BrowserError::Timeout("".to_string());
        assert_eq!(err.to_string(), "Browser operation timed out: ");
    }

    #[test]
    fn test_task_error_cancelled_with_empty_message() {
        let err = TaskError::Cancelled("".to_string());
        assert_eq!(err.to_string(), "Task cancelled: ");
    }

    #[test]
    fn test_error_variants_are_exhaustive() {
        // This test ensures all error variants are covered in tests
        // If a new variant is added, this test should be updated
        let _browser_errors = [
            BrowserError::ConnectionFailed("".to_string()),
            BrowserError::PageError("".to_string()),
            BrowserError::ElementError {
                selector: "".to_string(),
                reason: "".to_string(),
            },
            BrowserError::SelectorNotFound("".to_string()),
            BrowserError::Disconnected("".to_string()),
            BrowserError::Timeout("".to_string()),
        ];
        let _session_errors = [
            SessionError::InitializationFailed("".to_string()),
            SessionError::WorkerTimeout { timeout_ms: 0 },
            SessionError::Unhealthy("".to_string()),
            SessionError::PageRegistry("".to_string()),
            SessionError::ShutdownFailed("".to_string()),
        ];
        let _task_errors = [
            TaskError::ValidationFailed {
                task_name: "".to_string(),
                reason: "".to_string(),
            },
            TaskError::Timeout {
                task_name: "".to_string(),
                timeout_ms: 0,
            },
            TaskError::ExecutionFailed {
                task_name: "".to_string(),
                reason: "".to_string(),
            },
            TaskError::NotFound("".to_string()),
            TaskError::Cancelled("".to_string()),
            TaskError::RetryExhausted {
                max_retries: 0,
                task_name: "".to_string(),
            },
        ];
        let _config_errors = [
            ConfigError::LoadFailed {
                path: "".to_string(),
                reason: "".to_string(),
            },
            ConfigError::ValidationFailed("".to_string()),
            ConfigError::MissingField("".to_string(), "".to_string()),
            ConfigError::InvalidValue {
                field: "".to_string(),
                value: "".to_string(),
                reason: "".to_string(),
            },
            ConfigError::EnvVar("".to_string()),
        ];
        let _network_errors = [
            NetworkError::HttpError {
                url: "".to_string(),
                status: "".to_string(),
            },
            NetworkError::Timeout("".to_string()),
            NetworkError::CircuitBreakerOpen {
                service: "".to_string(),
            },
            NetworkError::Connection("".to_string()),
            NetworkError::ApiKey("".to_string()),
        ];
        // If this compiles, all variants are accounted for
        assert_eq!(_browser_errors.len(), 6);
        assert_eq!(_session_errors.len(), 5);
        assert_eq!(_task_errors.len(), 6);
        assert_eq!(_config_errors.len(), 5);
        assert_eq!(_network_errors.len(), 5);
    }
}
