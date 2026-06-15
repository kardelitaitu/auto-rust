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
            || err_str.contains("connection refused")
            || err_str.contains("execution context was destroyed")
            || err_str.contains("unable to click element")
            || err_str.contains("node is detached from document")
            || err_str.contains("no node with given id")
            || err_str.contains("could not find node")
            || err_str.contains("navigation")
            || err_str.contains("net::")
            || err_str.contains("network error")
            // LLM-specific transient patterns (rate limit via is_rate_limit_error, overload, server errors)
            || is_rate_limit_error(self)
            || err_str.contains("overloaded")
            || err_str.contains("503")
            || err_str.contains("server error")
            || err_str.contains("model is at capacity")
            || err_str.contains("try again later")
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
#[must_use]
pub fn is_rate_limit_error<E: std::fmt::Display>(err: &E) -> bool {
    let err_str = err.to_string().to_lowercase();
    err_str.contains("rate limit")
        || err_str.contains("too many requests")
        || err_str.contains("429")
}

/// Check if an error indicates an authentication failure.
#[must_use]
pub fn is_auth_error<E: std::fmt::Display>(err: &E) -> bool {
    let err_str = err.to_string().to_lowercase();
    err_str.contains("unauthorized")
        || err_str.contains("authentication")
        || err_str.contains("login")
        || err_str.contains("401")
        || err_str.contains("403")
}

#[cfg(test)]
mod tdd_tests {
    use super::*;

    // ====================================================================
    // RED Tests — describe desired behavior (expected to fail on first run)
    // ====================================================================

    #[test]
    fn tdd_red_error_classifier_empty_string() {
        // RED: Empty string should classify as Permanent (not crash)
        let err = anyhow::anyhow!("");
        let classification = err.classify();
        // Empty string doesn't match any known pattern, expect Permanent
        assert_eq!(classification, ErrorClass::Permanent);
    }

    // ====================================================================
    // GREEN Tests — validate working behavior
    // ====================================================================

    #[test]
    fn tdd_green_error_classifier_network_timeout() {
        // GREEN: Network timeout classifies as Transient
        let err = anyhow::anyhow!("network timeout occurred");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn tdd_green_error_classifier_navigation_error() {
        // GREEN: Navigation errors classify as Transient
        let err = anyhow::anyhow!("navigation failed");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn tdd_green_is_rate_limit_detects_all_variants() {
        // GREEN: All rate limit variants detected
        assert!(is_rate_limit_error(&"rate limit exceeded"));
        assert!(is_rate_limit_error(&"429 Too Many Requests"));
        assert!(is_rate_limit_error(&"too many requests"));
        assert!(!is_rate_limit_error(&"element not found"));
    }

    // ====================================================================
    // EDGE Case Tests
    // ====================================================================

    #[test]
    fn tdd_edge_error_classifier_case_insensitive() {
        // EDGE: Error classifier should be case-insensitive
        let err = anyhow::anyhow!("TIMEOUT WAITING FOR ELEMENT");
        assert_eq!(err.classify(), ErrorClass::Transient);

        let err = anyhow::anyhow!("STALE ELEMENT REFERENCE");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn tdd_edge_is_auth_error_handles_empty_string() {
        // EDGE: Empty string should not match auth error
        assert!(!is_auth_error(&""));
    }

    #[test]
    fn tdd_edge_is_rate_limit_handles_empty_string() {
        // EDGE: Empty string should not match rate limit
        assert!(!is_rate_limit_error(&""));
    }

    // ====================================================================
    // REGRESSION Tests
    // ====================================================================

    #[test]
    fn tdd_regression_error_classifier_not_confused_by_partial_matches() {
        // REGRESSION: Partial word matches should not misclassify
        // "rate" alone is NOT a rate limit error
        assert!(!is_rate_limit_error(&"the going rate for"));
        // "auth" alone is NOT auth error
        assert!(!is_auth_error(&"authoress wrote"));
    }
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

#[cfg(test)]
mod gap_tests {
    use super::*;
    use std::io;

    // ErrorClass Display implementation
    #[test]
    fn error_class_display_all_variants() {
        assert_eq!(format!("{}", ErrorClass::Transient), "transient");
        assert_eq!(format!("{}", ErrorClass::Permanent), "permanent");
        assert_eq!(format!("{}", ErrorClass::Fatal), "fatal");
    }

    // io::Error classification for each ErrorKind
    #[test]
    fn io_error_connection_refused_is_transient() {
        let err = io::Error::new(io::ErrorKind::ConnectionRefused, "connection refused");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn io_error_connection_reset_is_transient() {
        let err = io::Error::new(io::ErrorKind::ConnectionReset, "connection reset");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn io_error_connection_aborted_is_transient() {
        let err = io::Error::new(io::ErrorKind::ConnectionAborted, "connection aborted");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn io_error_not_connected_is_transient() {
        let err = io::Error::new(io::ErrorKind::NotConnected, "not connected");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn io_error_timed_out_is_transient() {
        let err = io::Error::new(io::ErrorKind::TimedOut, "timed out");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn io_error_would_block_is_transient() {
        let err = io::Error::new(io::ErrorKind::WouldBlock, "would block");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn io_error_out_of_memory_is_fatal() {
        let err = io::Error::new(io::ErrorKind::OutOfMemory, "out of memory");
        assert_eq!(err.classify(), ErrorClass::Fatal);
    }

    #[test]
    fn io_error_not_found_is_permanent() {
        let err = io::Error::new(io::ErrorKind::NotFound, "not found");
        assert_eq!(err.classify(), ErrorClass::Permanent);
    }

    #[test]
    fn io_error_permission_denied_is_permanent() {
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        assert_eq!(err.classify(), ErrorClass::Permanent);
    }

    #[test]
    fn io_error_invalid_input_is_permanent() {
        let err = io::Error::new(io::ErrorKind::InvalidInput, "invalid input");
        assert_eq!(err.classify(), ErrorClass::Permanent);
    }

    // More anyhow error patterns
    #[test]
    fn anyhow_out_of_memory_is_fatal() {
        let err = anyhow::anyhow!("out of memory allocating buffer");
        assert_eq!(err.classify(), ErrorClass::Fatal);
    }

    #[test]
    fn anyhow_unable_to_click_is_transient() {
        let err = anyhow::anyhow!("unable to click element at position");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_node_detached_is_transient() {
        let err = anyhow::anyhow!("node is detached from document");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_no_node_with_given_id_is_transient() {
        let err = anyhow::anyhow!("no node with given id 12345");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_could_not_find_node_is_transient() {
        let err = anyhow::anyhow!("could not find node in DOM tree");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_net_error_is_transient() {
        let err = anyhow::anyhow!("net::ERR_CONNECTION_TIMED_OUT");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_network_error_is_transient() {
        let err = anyhow::anyhow!("network error occurred during fetch");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_timed_out_is_transient() {
        let err = anyhow::anyhow!("operation timed out after 30s");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_connection_refused_is_transient() {
        let err = anyhow::anyhow!("connection refused by remote host");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_unknown_error_is_permanent() {
        let err = anyhow::anyhow!("something completely unexpected happened");
        assert_eq!(err.classify(), ErrorClass::Permanent);
    }

    // LLM-specific transient patterns
    #[test]
    fn anyhow_rate_limit_is_transient() {
        let err = anyhow::anyhow!("rate limit exceeded for model gpt-4");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_too_many_requests_is_transient() {
        let err = anyhow::anyhow!("too many requests, please slow down");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_http_429_is_transient() {
        let err = anyhow::anyhow!("HTTP 429: rate limited");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_overloaded_is_transient() {
        let err = anyhow::anyhow!("model overloaded, try again later");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_http_503_is_transient() {
        let err = anyhow::anyhow!("HTTP 503 service unavailable");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_server_error_is_transient() {
        let err = anyhow::anyhow!("server error occurred during processing");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_model_at_capacity_is_transient() {
        let err = anyhow::anyhow!("model is at capacity, try again later");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    #[test]
    fn anyhow_try_again_later_is_transient() {
        let err = anyhow::anyhow!("service busy, try again later");
        assert_eq!(err.classify(), ErrorClass::Transient);
    }

    // is_auth_error additional patterns
    #[test]
    fn is_auth_error_detects_403() {
        assert!(is_auth_error(&"HTTP 403 Forbidden"));
    }

    #[test]
    fn is_auth_error_detects_login_required() {
        assert!(is_auth_error(&"login required to continue"));
    }

    #[test]
    fn is_auth_error_case_insensitive() {
        assert!(is_auth_error(&"UNAUTHORIZED ACCESS"));
        assert!(is_auth_error(&"Authentication Failed"));
    }

    // is_rate_limit_error additional patterns
    #[test]
    fn is_rate_limit_error_case_insensitive() {
        assert!(is_rate_limit_error(&"RATE LIMIT EXCEEDED"));
        assert!(is_rate_limit_error(&"Too Many Requests"));
    }
}
