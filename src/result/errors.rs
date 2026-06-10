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
    if m.contains("connection") && (m.contains("refused") || m.contains("reset") || m.contains("broken") || m.contains("closed")) {
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
            ErrorPattern::NotFound => TaskErrorKind::Session,   // not found = session issue
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
