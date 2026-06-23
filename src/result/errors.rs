use serde::{Deserialize, Serialize};

// =============================================================================
// Shared Error Classification
// =============================================================================

/// Shared error pattern classification used by both transient-error classification
/// (is_transient_error) and TaskErrorKind classification.
///
/// This enum captures the semantic category of an error message, allowing
/// both systems to share the same pattern-matching logic while interpreting
/// the classification differently (one as retry vs fail-fast, the other as kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ErrorPattern {
    // --- Permanent errors (is_transient_error: return false) ---
    /// Element not found, selector not found, no such element
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// Target closed or node is disconnected (permanent browser death)
    TargetTerminated,

    // --- Transient errors (is_transient_error: return true) ---
    /// Timeout or deadline exceeded
    Timeout,
    /// Connection refused/reset/broken
    Connection,
    /// Temporary or unavailable
    Temporary,
    /// Network error or econnreset
    Network,
    /// Operation aborted, cancelled, or interrupted
    Cancelled,
    /// Standalone "disconnected" (not "node is disconnected") - recoverable
    Disconnected,
    /// Rate limited (HTTP 429, too many requests, overloaded) — LLM API transient
    RateLimited,

    // --- TaskErrorKind-specific categories ---
    /// Validation error (invalid params, schema issues)
    Validation,
    /// Navigation error (goto, load)
    Navigation,
    /// Session/channel error (receiver gone, channel closed, send failed)
    SessionChannel,

    /// Unclassified
    Unknown,
}

/// Classify an error message into a shared `ErrorPattern`.
/// This is the single source of truth for error pattern matching used by both
/// `is_transient_error` (in page_nav.rs) and `TaskErrorKind::classify` (here).
///
/// All checks are case-insensitive via `to_lowercase()`.
/// The ordering matters: more specific patterns must come before general ones
/// (e.g., "node is disconnected" before "disconnected").
pub(crate) fn classify_error_pattern(msg: &str) -> ErrorPattern {
    let m = msg.to_lowercase();

    // PERMANENT patterns (checked first - more specific before general)

    // "not found" and variants
    if m.contains("not found") || m.contains("element not found") {
        return ErrorPattern::NotFound;
    }
    if m.contains("no such element") {
        return ErrorPattern::NotFound;
    }
    if m.contains("invalid selector") {
        return ErrorPattern::NotFound;
    }
    if m.contains("selector not found") {
        return ErrorPattern::NotFound;
    }

    // "node is disconnected" must come before standalone "disconnected"
    if m.contains("node is disconnected") {
        return ErrorPattern::TargetTerminated;
    }
    if m.contains("target closed") {
        return ErrorPattern::TargetTerminated;
    }
    if m.contains("permission denied") {
        return ErrorPattern::PermissionDenied;
    }

    // TRANSIENT patterns

    // "disconnected" standalone (without "node is" prefix)
    if m.contains("disconnected") {
        return ErrorPattern::Disconnected;
    }
    if m.contains("timeout") || m.contains("timed out") || m.contains("deadline") {
        return ErrorPattern::Timeout;
    }
    if m.contains("connection")
        && (m.contains("refused")
            || m.contains("reset")
            || m.contains("broken")
            || m.contains("closed"))
    {
        return ErrorPattern::Connection;
    }
    // LLM-specific transient patterns (rate limit, overload, server errors)
    // Check these before generic "temporary"/"unavailable" for finer classification
    if m.contains("rate limit") || m.contains("too many requests") || m.contains("429") {
        return ErrorPattern::RateLimited;
    }
    if m.contains("overloaded") || m.contains("503") || m.contains("server error") {
        return ErrorPattern::RateLimited;
    }
    if m.contains("model is at capacity") || m.contains("try again later") {
        return ErrorPattern::RateLimited;
    }

    if m.contains("temporary") || m.contains("unavailable") {
        return ErrorPattern::Temporary;
    }
    if m.contains("network") || m.contains("econnreset") {
        return ErrorPattern::Network;
    }
    if m.contains("aborted") || m.contains("cancelled") || m.contains("interrupted") {
        return ErrorPattern::Cancelled;
    }

    // TASKERRORKIND-SPECIFIC patterns (not used by is_transient_error)

    // Validation patterns — "invalid" alone (e.g. "invalid parameters") maps here.
    // Note: "invalid selector" is caught earlier by NotFound, so there's no false match.
    if m.contains("validation") || m.contains("schema") || m.contains("invalid") {
        return ErrorPattern::Validation;
    }

    // Navigation patterns
    if m.contains("navigat") || m.contains("goto") || m.contains("load") {
        return ErrorPattern::Navigation;
    }

    // Session/channel patterns
    if m.contains("receiver is gone")
        || m.contains("channel closed")
        || m.contains("send failed")
        || m.contains("worker")
        || m.contains("session")
    {
        return ErrorPattern::SessionChannel;
    }

    // Browser-specific patterns (target detached, websocket, protocol)
    if m.contains("target.detached")
        || m.contains("detachedfromtarget")
        || m.contains("websocket")
        || m.contains("protocol error")
    {
        return ErrorPattern::TargetTerminated;
    }

    // "page" in error context (but not "page load" which is Navigation)
    if m.contains("page") && !m.contains("load") {
        return ErrorPattern::SessionChannel;
    }

    ErrorPattern::Unknown
}

// =============================================================================
// TaskErrorKind
// =============================================================================

/// Categorizes different types of errors that can occur during task execution.
/// This enum helps with error handling, logging, and debugging by classifying
/// errors into specific categories for appropriate handling.
///
/// `#[non_exhaustive]` — match with wildcard arm.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TaskErrorKind {
    /// Task execution exceeded the configured timeout limit
    Timeout,
    /// Input validation failed (invalid parameters, malformed data)
    Validation,
    /// Page navigation failed (network issues, invalid URLs, redirects)
    Navigation,
    /// Session management error (connection lost, session expired)
    Session,
    /// Browser connection or automation error (`WebDriver` issues, browser crashes)
    Browser,
    /// External service error (LLM API rate limiting, overloaded, unavailable)
    ExternalService,
    /// Unknown or uncategorized error type
    Unknown,
}

impl TaskErrorKind {
    /// Classifies an error message string into a specific error category.
    /// Uses shared `classify_error_pattern()` for pattern matching.
    ///
    /// # Arguments
    /// * `error` - Error message string to classify
    ///
    /// # Returns
    /// The most appropriate `TaskErrorKind` for the given error message
    #[must_use]
    pub fn classify(error: &str) -> Self {
        match classify_error_pattern(error) {
            ErrorPattern::Timeout => TaskErrorKind::Timeout,
            ErrorPattern::NotFound => TaskErrorKind::Session, // not found = session issue
            ErrorPattern::PermissionDenied => TaskErrorKind::Browser,
            ErrorPattern::TargetTerminated => TaskErrorKind::Browser,
            ErrorPattern::Connection => TaskErrorKind::Browser,
            ErrorPattern::Network => TaskErrorKind::Browser,
            ErrorPattern::Temporary => TaskErrorKind::Browser,
            ErrorPattern::Cancelled => TaskErrorKind::Session,
            ErrorPattern::Disconnected => TaskErrorKind::Browser,
            ErrorPattern::RateLimited => TaskErrorKind::ExternalService,
            ErrorPattern::Validation => TaskErrorKind::Validation,
            ErrorPattern::Navigation => TaskErrorKind::Navigation,
            ErrorPattern::SessionChannel => TaskErrorKind::Session,
            ErrorPattern::Unknown => {
                // Check for browser-specific keywords
                let m = error.to_lowercase();
                if m.contains("browser") || m.contains("chromium") || m.contains("brave") {
                    TaskErrorKind::Browser
                } else {
                    TaskErrorKind::Unknown
                }
            }
        }
    }

    #[must_use]
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            TaskErrorKind::Timeout
                | TaskErrorKind::Navigation
                | TaskErrorKind::Session
                | TaskErrorKind::Browser
                | TaskErrorKind::ExternalService
                | TaskErrorKind::Unknown
        )
    }
}

impl std::fmt::Display for TaskErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskErrorKind::Timeout => write!(f, "Timeout"),
            TaskErrorKind::Validation => write!(f, "Validation"),
            TaskErrorKind::Navigation => write!(f, "Navigation"),
            TaskErrorKind::Session => write!(f, "Session"),
            TaskErrorKind::Browser => write!(f, "Browser"),
            TaskErrorKind::ExternalService => write!(f, "ExternalService"),
            TaskErrorKind::Unknown => write!(f, "Unknown"),
        }
    }
}

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use super::*;

    // =========================================================================
    // classify_error_pattern — PERMANENT patterns
    // =========================================================================

    #[test]
    fn classify_not_found_basic() {
        assert_eq!(
            classify_error_pattern("element not found"),
            ErrorPattern::NotFound
        );
    }

    #[test]
    fn classify_not_found_no_such_element() {
        assert_eq!(
            classify_error_pattern("no such element: #foo"),
            ErrorPattern::NotFound
        );
    }

    #[test]
    fn classify_not_found_invalid_selector() {
        assert_eq!(
            classify_error_pattern("invalid selector: div[foo"),
            ErrorPattern::NotFound
        );
    }

    #[test]
    fn classify_not_found_selector_not_found() {
        assert_eq!(
            classify_error_pattern("selector not found: .bar"),
            ErrorPattern::NotFound
        );
    }

    #[test]
    fn classify_not_found_generic() {
        assert_eq!(
            classify_error_pattern("item not found"),
            ErrorPattern::NotFound
        );
    }

    #[test]
    fn classify_target_terminated_node_disconnected() {
        assert_eq!(
            classify_error_pattern("node is disconnected"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_target_terminated_target_closed() {
        assert_eq!(
            classify_error_pattern("Target closed"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_permission_denied() {
        assert_eq!(
            classify_error_pattern("permission denied: access restricted"),
            ErrorPattern::PermissionDenied
        );
    }

    // =========================================================================
    // classify_error_pattern — TRANSIENT patterns
    // =========================================================================

    #[test]
    fn classify_disconnected_standalone() {
        assert_eq!(
            classify_error_pattern("disconnected: no response"),
            ErrorPattern::Disconnected
        );
    }

    #[test]
    fn classify_timeout_basic() {
        assert_eq!(
            classify_error_pattern("Request timed out"),
            ErrorPattern::Timeout
        );
    }

    #[test]
    fn classify_timeout_timed_out() {
        assert_eq!(
            classify_error_pattern("timed out after 30s"),
            ErrorPattern::Timeout
        );
    }

    #[test]
    fn classify_timeout_deadline() {
        assert_eq!(
            classify_error_pattern("deadline exceeded"),
            ErrorPattern::Timeout
        );
    }

    #[test]
    fn classify_connection_refused() {
        assert_eq!(
            classify_error_pattern("connection refused"),
            ErrorPattern::Connection
        );
    }

    #[test]
    fn classify_connection_reset() {
        assert_eq!(
            classify_error_pattern("connection reset by peer"),
            ErrorPattern::Connection
        );
    }

    #[test]
    fn classify_connection_broken() {
        assert_eq!(
            classify_error_pattern("connection broken"),
            ErrorPattern::Connection
        );
    }

    #[test]
    fn classify_connection_closed() {
        assert_eq!(
            classify_error_pattern("connection closed"),
            ErrorPattern::Connection
        );
    }

    // =========================================================================
    // classify_error_pattern — RATE LIMITED patterns
    // =========================================================================

    #[test]
    fn classify_rate_limit_basic() {
        assert_eq!(
            classify_error_pattern("rate limit exceeded"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_rate_limit_too_many_requests() {
        assert_eq!(
            classify_error_pattern("too many requests"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_rate_limit_http_429() {
        assert_eq!(
            classify_error_pattern("HTTP 429 response"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_rate_limit_overloaded() {
        assert_eq!(
            classify_error_pattern("server overloaded"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_rate_limit_http_503() {
        assert_eq!(
            classify_error_pattern("503 Service Unavailable"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_rate_limit_server_error() {
        assert_eq!(
            classify_error_pattern("server error: internal"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_rate_limit_model_at_capacity() {
        assert_eq!(
            classify_error_pattern("model is at capacity"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_rate_limit_try_again_later() {
        assert_eq!(
            classify_error_pattern("try again later"),
            ErrorPattern::RateLimited
        );
    }

    // =========================================================================
    // classify_error_pattern — TEMPORARY, NETWORK, CANCELLED
    // =========================================================================

    #[test]
    fn classify_temporary_error() {
        assert_eq!(
            classify_error_pattern("temporary error"),
            ErrorPattern::Temporary
        );
    }

    #[test]
    fn classify_unavailable() {
        assert_eq!(
            classify_error_pattern("service unavailable"),
            ErrorPattern::Temporary
        );
    }

    #[test]
    fn classify_network_error() {
        assert_eq!(
            classify_error_pattern("network error"),
            ErrorPattern::Network
        );
    }

    #[test]
    fn classify_econnreset() {
        assert_eq!(classify_error_pattern("econnreset"), ErrorPattern::Network);
    }

    #[test]
    fn classify_aborted() {
        assert_eq!(
            classify_error_pattern("operation aborted"),
            ErrorPattern::Cancelled
        );
    }

    #[test]
    fn classify_cancelled() {
        assert_eq!(
            classify_error_pattern("task cancelled"),
            ErrorPattern::Cancelled
        );
    }

    #[test]
    fn classify_interrupted() {
        assert_eq!(
            classify_error_pattern("interrupted by user"),
            ErrorPattern::Cancelled
        );
    }

    // =========================================================================
    // classify_error_pattern — TaskErrorKind-specific patterns
    // =========================================================================

    #[test]
    fn classify_validation() {
        assert_eq!(
            classify_error_pattern("validation error"),
            ErrorPattern::Validation
        );
    }

    #[test]
    fn classify_schema_error() {
        assert_eq!(
            classify_error_pattern("schema mismatch"),
            ErrorPattern::Validation
        );
    }

    #[test]
    fn classify_invalid_parameters() {
        assert_eq!(
            classify_error_pattern("invalid parameters"),
            ErrorPattern::Validation
        );
    }

    #[test]
    fn classify_navigation_basic() {
        assert_eq!(
            classify_error_pattern("navigation failed"),
            ErrorPattern::Navigation
        );
    }

    #[test]
    fn classify_goto_failed() {
        assert_eq!(
            classify_error_pattern("goto failed"),
            ErrorPattern::Navigation
        );
    }

    #[test]
    fn classify_load_failed() {
        assert_eq!(
            classify_error_pattern("page load failed"),
            ErrorPattern::Navigation
        );
    }

    #[test]
    fn classify_receiver_gone() {
        assert_eq!(
            classify_error_pattern("send failed because receiver is gone"),
            ErrorPattern::SessionChannel
        );
    }

    #[test]
    fn classify_channel_closed() {
        assert_eq!(
            classify_error_pattern("channel closed"),
            ErrorPattern::SessionChannel
        );
    }

    #[test]
    fn classify_send_failed() {
        assert_eq!(
            classify_error_pattern("send failed"),
            ErrorPattern::SessionChannel
        );
    }

    #[test]
    fn classify_worker_error() {
        assert_eq!(
            classify_error_pattern("worker acquisition failed"),
            ErrorPattern::SessionChannel
        );
    }

    #[test]
    fn classify_session_error() {
        assert_eq!(
            classify_error_pattern("session expired"),
            ErrorPattern::SessionChannel
        );
    }

    // =========================================================================
    // classify_error_pattern — Browser-specific patterns
    // =========================================================================

    #[test]
    fn classify_target_detached() {
        assert_eq!(
            classify_error_pattern("Protocol error (Target.detachedFromTarget)"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_detached_from_target() {
        assert_eq!(
            classify_error_pattern("detachedFromTarget"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_websocket_error() {
        assert_eq!(
            classify_error_pattern("WebSocket error"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_protocol_error() {
        assert_eq!(
            classify_error_pattern("protocol error"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_page_without_load() {
        // "page general error" reaches the page-without-load check
        // because it contains "page", doesn't contain "load",
        // and "error" alone doesn't match any earlier pattern.
        assert_eq!(
            classify_error_pattern("page general error"),
            ErrorPattern::SessionChannel
        );
    }

    // =========================================================================
    // classify_error_pattern — FALLBACK patterns
    // =========================================================================

    #[test]
    fn classify_unknown_fallback() {
        assert_eq!(
            classify_error_pattern("something completely random"),
            ErrorPattern::Unknown
        );
    }

    #[test]
    fn classify_empty_string() {
        assert_eq!(classify_error_pattern(""), ErrorPattern::Unknown);
    }

    #[test]
    fn classify_whitespace() {
        assert_eq!(classify_error_pattern("   "), ErrorPattern::Unknown);
    }

    // =========================================================================
    // classify_error_pattern — KEYWORD PRIORITY
    // =========================================================================

    #[test]
    fn classify_priority_node_disconnected_before_plain_disconnected() {
        // "node is disconnected" must match TargetTerminated, not Disconnected
        assert_eq!(
            classify_error_pattern("node is disconnected from target"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_priority_timeout_before_validation() {
        // "timeout" checked before "invalid"
        assert_eq!(
            classify_error_pattern("invalid request timeout"),
            ErrorPattern::Timeout
        );
    }

    #[test]
    fn classify_priority_not_found_before_validation() {
        // "not found" checked before "invalid"
        assert_eq!(
            classify_error_pattern("invalid selector: element not found"),
            ErrorPattern::NotFound
        );
    }

    #[test]
    fn classify_priority_rate_limit_before_temporary() {
        // "overloaded" checked before "temporary"
        assert_eq!(
            classify_error_pattern("server overloaded temporary error"),
            ErrorPattern::RateLimited
        );
    }

    #[test]
    fn classify_priority_connection_before_session() {
        // "connection reset" checked before "session"
        assert_eq!(
            classify_error_pattern("session connection reset"),
            ErrorPattern::Connection
        );
    }

    #[test]
    fn classify_priority_target_closed_before_session() {
        // "target closed" checked before generic "session" patterns
        assert_eq!(
            classify_error_pattern("target closed during session"),
            ErrorPattern::TargetTerminated
        );
    }

    #[test]
    fn classify_case_insensitive() {
        assert_eq!(classify_error_pattern("TIMEOUT"), ErrorPattern::Timeout);
        assert_eq!(classify_error_pattern("NOT FOUND"), ErrorPattern::NotFound);
        assert_eq!(
            classify_error_pattern("RATE LIMIT"),
            ErrorPattern::RateLimited
        );
    }

    // =========================================================================
    // TaskErrorKind::classify tests
    // =========================================================================

    #[test]
    fn kind_classify_timeout() {
        assert_eq!(
            TaskErrorKind::classify("operation timeout exceeded"),
            TaskErrorKind::Timeout
        );
    }

    #[test]
    fn kind_classify_not_found() {
        assert_eq!(
            TaskErrorKind::classify("element not found"),
            TaskErrorKind::Session
        );
    }

    #[test]
    fn kind_classify_permission_denied() {
        assert_eq!(
            TaskErrorKind::classify("permission denied"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_target_terminated() {
        assert_eq!(
            TaskErrorKind::classify("target closed"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_connection() {
        assert_eq!(
            TaskErrorKind::classify("connection refused"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_network() {
        assert_eq!(
            TaskErrorKind::classify("network error"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_temporary() {
        assert_eq!(
            TaskErrorKind::classify("temporary unavailable"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_cancelled() {
        assert_eq!(
            TaskErrorKind::classify("operation cancelled"),
            TaskErrorKind::Session
        );
    }

    #[test]
    fn kind_classify_disconnected() {
        assert_eq!(
            TaskErrorKind::classify("disconnected from host"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_rate_limited() {
        assert_eq!(
            TaskErrorKind::classify("rate limit exceeded"),
            TaskErrorKind::ExternalService
        );
    }

    #[test]
    fn kind_classify_validation() {
        assert_eq!(
            TaskErrorKind::classify("validation failed"),
            TaskErrorKind::Validation
        );
    }

    #[test]
    fn kind_classify_navigation() {
        assert_eq!(
            TaskErrorKind::classify("navigation error"),
            TaskErrorKind::Navigation
        );
    }

    #[test]
    fn kind_classify_session() {
        assert_eq!(
            TaskErrorKind::classify("session expired"),
            TaskErrorKind::Session
        );
    }

    #[test]
    fn kind_classify_unknown_fallback() {
        assert_eq!(
            TaskErrorKind::classify("some random error"),
            TaskErrorKind::Unknown
        );
    }

    #[test]
    fn kind_classify_browser_keyword_to_browser() {
        assert_eq!(
            TaskErrorKind::classify("browser crashed"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_chromium_to_browser() {
        assert_eq!(
            TaskErrorKind::classify("chromium process died"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_brave_to_browser() {
        assert_eq!(
            TaskErrorKind::classify("brave disconnected"),
            TaskErrorKind::Browser
        );
    }

    #[test]
    fn kind_classify_empty_string() {
        assert_eq!(TaskErrorKind::classify(""), TaskErrorKind::Unknown);
    }

    #[test]
    fn kind_classify_whitespace() {
        assert_eq!(TaskErrorKind::classify("   "), TaskErrorKind::Unknown);
    }

    #[test]
    fn kind_classify_case_insensitive() {
        assert_eq!(
            TaskErrorKind::classify("CONNECTION REFUSED"),
            TaskErrorKind::Browser
        );
        assert_eq!(TaskErrorKind::classify("TIMEOUT"), TaskErrorKind::Timeout);
    }

    // =========================================================================
    // TaskErrorKind::is_retryable tests
    // =========================================================================

    #[test]
    fn kind_is_retryable_timeout() {
        assert!(TaskErrorKind::Timeout.is_retryable());
    }

    #[test]
    fn kind_is_retryable_navigation() {
        assert!(TaskErrorKind::Navigation.is_retryable());
    }

    #[test]
    fn kind_is_retryable_session() {
        assert!(TaskErrorKind::Session.is_retryable());
    }

    #[test]
    fn kind_is_retryable_browser() {
        assert!(TaskErrorKind::Browser.is_retryable());
    }

    #[test]
    fn kind_is_retryable_external_service() {
        assert!(TaskErrorKind::ExternalService.is_retryable());
    }

    #[test]
    fn kind_is_retryable_unknown() {
        assert!(TaskErrorKind::Unknown.is_retryable());
    }

    #[test]
    fn kind_is_not_retryable_validation() {
        assert!(!TaskErrorKind::Validation.is_retryable());
    }

    // =========================================================================
    // TaskErrorKind derived trait tests
    // =========================================================================

    #[test]
    fn kind_display_timeout() {
        assert_eq!(format!("{}", TaskErrorKind::Timeout), "Timeout");
    }

    #[test]
    fn kind_display_validation() {
        assert_eq!(format!("{}", TaskErrorKind::Validation), "Validation");
    }

    #[test]
    fn kind_display_navigation() {
        assert_eq!(format!("{}", TaskErrorKind::Navigation), "Navigation");
    }

    #[test]
    fn kind_display_session() {
        assert_eq!(format!("{}", TaskErrorKind::Session), "Session");
    }

    #[test]
    fn kind_display_browser() {
        assert_eq!(format!("{}", TaskErrorKind::Browser), "Browser");
    }

    #[test]
    fn kind_display_external_service() {
        assert_eq!(
            format!("{}", TaskErrorKind::ExternalService),
            "ExternalService"
        );
    }

    #[test]
    fn kind_display_unknown() {
        assert_eq!(format!("{}", TaskErrorKind::Unknown), "Unknown");
    }

    #[test]
    fn kind_debug() {
        assert!(format!("{:?}", TaskErrorKind::Timeout).contains("Timeout"));
    }

    #[test]
    fn kind_clone() {
        let k = TaskErrorKind::Timeout;
        assert_eq!(k.clone(), k);
    }

    #[test]
    fn kind_copy() {
        let k = TaskErrorKind::Browser;
        let k2 = k; // Copy
        assert_eq!(k, k2);
    }

    #[test]
    fn kind_partial_eq() {
        assert_eq!(TaskErrorKind::Timeout, TaskErrorKind::Timeout);
        assert_ne!(TaskErrorKind::Timeout, TaskErrorKind::Validation);
    }

    #[test]
    fn kind_eq() {
        // TaskErrorKind derives Eq in addition to PartialEq
        let a = TaskErrorKind::Session;
        let b = TaskErrorKind::Session;
        assert_eq!(a, b);
    }

    #[test]
    fn kind_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TaskErrorKind::Timeout);
        set.insert(TaskErrorKind::Validation);
        set.insert(TaskErrorKind::Timeout); // duplicate
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn kind_partial_ord() {
        assert!(TaskErrorKind::Timeout < TaskErrorKind::Unknown);
        assert!(TaskErrorKind::Timeout <= TaskErrorKind::Timeout);
        assert!(TaskErrorKind::Validation < TaskErrorKind::Browser);
    }

    #[test]
    fn kind_ord() {
        let mut variants = [
            TaskErrorKind::Unknown,
            TaskErrorKind::Browser,
            TaskErrorKind::Timeout,
        ];
        variants.sort();
        assert_eq!(variants[0], TaskErrorKind::Timeout);
        assert_eq!(variants[1], TaskErrorKind::Browser);
        assert_eq!(variants[2], TaskErrorKind::Unknown);
    }

    #[test]
    fn kind_serialize() {
        let json = serde_json::to_string(&TaskErrorKind::Timeout).unwrap();
        assert_eq!(json, "\"Timeout\"");
    }

    #[test]
    fn kind_deserialize() {
        let kind: TaskErrorKind = serde_json::from_str("\"Timeout\"").unwrap();
        assert_eq!(kind, TaskErrorKind::Timeout);
    }

    #[test]
    fn kind_serde_round_trip_all_variants() {
        let variants = [
            TaskErrorKind::Timeout,
            TaskErrorKind::Validation,
            TaskErrorKind::Navigation,
            TaskErrorKind::Session,
            TaskErrorKind::Browser,
            TaskErrorKind::ExternalService,
            TaskErrorKind::Unknown,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let back: TaskErrorKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    #[test]
    fn kind_all_variants_non_exhaustive() {
        let kinds: [TaskErrorKind; 7] = [
            TaskErrorKind::Timeout,
            TaskErrorKind::Validation,
            TaskErrorKind::Navigation,
            TaskErrorKind::Session,
            TaskErrorKind::Browser,
            TaskErrorKind::ExternalService,
            TaskErrorKind::Unknown,
        ];
        // Verify non_exhaustive: can match with wildcard
        for k in &kinds {
            let _desc = match k {
                TaskErrorKind::Timeout => "t",
                _ => "other",
            };
        }
        assert_eq!(kinds.len(), 7);
    }

    // =========================================================================
    // ErrorPattern tests (pub(crate) but accessible within crate)
    // =========================================================================

    #[test]
    fn error_pattern_debug() {
        assert!(format!("{:?}", ErrorPattern::NotFound).contains("NotFound"));
        assert!(format!("{:?}", ErrorPattern::Timeout).contains("Timeout"));
    }

    #[test]
    fn error_pattern_clone_copy_eq() {
        let a = ErrorPattern::NotFound;
        let b = a; // Copy
        assert_eq!(a, b);
        #[allow(clippy::clone_on_copy)]
        let c = a.clone(); // Clone - intentionally verifying Clone on Copy type
        assert_eq!(a, c);
    }

    #[test]
    fn error_pattern_all_variants() {
        let variants = [
            ErrorPattern::NotFound,
            ErrorPattern::PermissionDenied,
            ErrorPattern::TargetTerminated,
            ErrorPattern::Timeout,
            ErrorPattern::Connection,
            ErrorPattern::Temporary,
            ErrorPattern::Network,
            ErrorPattern::Cancelled,
            ErrorPattern::Disconnected,
            ErrorPattern::RateLimited,
            ErrorPattern::Validation,
            ErrorPattern::Navigation,
            ErrorPattern::SessionChannel,
            ErrorPattern::Unknown,
        ];
        assert_eq!(variants.len(), 14);
    }
}
