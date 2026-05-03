//! Error classification and handling for Twitter activity automation.
//!
//! Provides error classification to distinguish between:
//! - Transient errors (retryable): network timeouts, stale elements, temporary failures
//! - Permanent errors (fail fast): selector not found, authentication errors
//! - Fatal errors (abort session): browser crashes, out of memory
//!
//! This enables intelligent retry logic and graceful degradation.

use std::fmt;

/// Classification of errors for retry decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Transient error - can be retried (network timeout, stale element, etc.)
    Transient,
    /// Permanent error - don't retry (selector not found, auth error)
    Permanent,
    /// Fatal error - abort session (browser crashed, out of memory)
    Fatal,
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorClass::Transient => write!(f, "transient"),
            ErrorClass::Permanent => write!(f, "permanent"),
            ErrorClass::Fatal => write!(f, "fatal"),
        }
    }
}

/// Trait to classify errors for retry decisions.
pub trait ErrorClassifier {
    /// Classify this error for retry logic.
    fn classify(&self) -> ErrorClass;
}

impl ErrorClassifier for anyhow::Error {
    fn classify(&self) -> ErrorClass {
        let err_str = self.to_string().to_lowercase();
        let root_str = format!("{:?}", self.root_cause()).to_lowercase();

        // Fatal errors - abort session immediately
        if err_str.contains("browser disconnected")
            || err_str.contains("target closed")
            || err_str.contains("connection refused")
            || err_str.contains("out of memory")
            || root_str.contains("browser disconnected")
        {
            return ErrorClass::Fatal;
        }

        // Transient errors - can retry
        if err_str.contains("stale element")
            || err_str.contains("element not found")
            || err_str.contains("timeout")
            || err_str.contains("timed out")
            || err_str.contains("execution context was destroyed")
            || err_str.contains("unable to click element")
            || err_str.contains("node is detached from document")
            || err_str.contains("no node with given id")
            || err_str.contains("could not find node")
            || err_str.contains("navigation")
            || err_str.contains("net::")
            || err_str.contains("network error")
        {
            return ErrorClass::Transient;
        }

        // Permanent errors - don't retry
        ErrorClass::Permanent
    }
}

impl ErrorClassifier for std::io::Error {
    fn classify(&self) -> ErrorClass {
        use std::io::ErrorKind;

        match self.kind() {
            // Transient network errors
            ErrorKind::ConnectionRefused
            | ErrorKind::ConnectionReset
            | ErrorKind::ConnectionAborted
            | ErrorKind::NotConnected
            | ErrorKind::TimedOut
            | ErrorKind::WouldBlock => ErrorClass::Transient,

            // Fatal errors
            ErrorKind::OutOfMemory => ErrorClass::Fatal,

            // Permanent errors
            _ => ErrorClass::Permanent,
        }
    }
}

/// Check if an error indicates a rate limit from Twitter/X.
pub fn is_rate_limit_error<E: std::fmt::Display>(err: &E) -> bool {
    let err_str = err.to_string().to_lowercase();
    err_str.contains("rate limit")
        || err_str.contains("too many requests")
        || err_str.contains("429")
}

/// Check if an error indicates an authentication failure.
pub fn is_auth_error<E: std::fmt::Display>(err: &E) -> bool {
    let err_str = err.to_string().to_lowercase();
    err_str.contains("unauthorized")
        || err_str.contains("authentication")
        || err_str.contains("login")
        || err_str.contains("401")
        || err_str.contains("403")
}

#[cfg(test)]
mod classification_tests {
    use super::{ErrorClass, ErrorClassifier};

    #[test]
    fn transient_errors_classify_as_transient() {
        let err = anyhow::anyhow!("stale element reference");
        assert_eq!(err.classify(), ErrorClass::Transient);

        let err = anyhow::anyhow!("timeout waiting for element");
        assert_eq!(err.classify(), ErrorClass::Transient);

        let err = anyhow::anyhow!("execution context was destroyed");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn permanent_and_fatal_errors_classify_correctly() {
        let err = anyhow::anyhow!("element not found in DOM");
        // This is actually transient - DOM may update
        assert_eq!(err.classify(), ErrorClass::Transient);

        let err = anyhow::anyhow!("invalid selector syntax");
        assert_eq!(err.classify(), ErrorClass::Permanent);

        let err = anyhow::anyhow!("browser disconnected");
        assert_eq!(err.classify(), ErrorClass::Fatal);

        let err = anyhow::anyhow!("target closed");
        assert_eq!(err.classify(), ErrorClass::Fatal);
    }
}

#[cfg(test)]
mod detection_tests {
    use super::{is_auth_error, is_rate_limit_error};

    #[test]
    fn rate_limit_detection_matches_expected_patterns() {
        assert!(is_rate_limit_error(&"rate limit exceeded"));
        assert!(is_rate_limit_error(&"429 Too Many Requests"));
        assert!(!is_rate_limit_error(&"element not found"));
    }

    #[test]
    fn auth_error_detection_matches_expected_patterns() {
        assert!(is_auth_error(&"401 Unauthorized"));
        assert!(is_auth_error(&"authentication required"));
        assert!(!is_auth_error(&"network timeout"));
    }
}
