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
    #[error("Missing required config field: {0}")]
    MissingField(String),

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
