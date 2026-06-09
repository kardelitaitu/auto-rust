use serde::{Deserialize, Serialize};

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
    /// Unknown or uncategorized error type
    Unknown,
}

impl TaskErrorKind {
    /// Classifies an error message string into a specific error category.
    /// This method performs pattern matching on common error strings to
    /// determine the most appropriate error type.
    ///
    /// # Arguments
    /// * `error` - Error message string to classify
    ///
    /// # Returns
    /// The most appropriate `TaskErrorKind` for the given error message
    #[must_use]
    pub fn classify(error: &str) -> Self {
        let e = error.to_lowercase();
        if e.contains("timeout") || e.contains("deadline") {
            TaskErrorKind::Timeout
        } else if e.contains("validation") || e.contains("invalid") || e.contains("schema") {
            TaskErrorKind::Validation
        } else if e.contains("target.detached")
            || e.contains("detachedfromtarget")
            || e.contains("target closed")
            || e.contains("browser disconnected")
            || e.contains("websocket")
            || e.contains("connection reset")
            || e.contains("connection closed")
            || e.contains("protocol error")
        {
            TaskErrorKind::Browser
        } else if e.contains("navigat") || e.contains("goto") || e.contains("load") {
            TaskErrorKind::Navigation
        } else if e.contains("receiver is gone")
            || e.contains("channel closed")
            || e.contains("send failed")
            || e.contains("session")
            || e.contains("worker")
            || e.contains("page")
        {
            TaskErrorKind::Session
        } else if e.contains("browser") || e.contains("chromium") || e.contains("brave") {
            TaskErrorKind::Browser
        } else {
            TaskErrorKind::Unknown
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
            TaskErrorKind::Unknown => write!(f, "Unknown"),
        }
    }
}
