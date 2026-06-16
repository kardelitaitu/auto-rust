/*
last audited 08-05-25 by RSA-Agent
crate: auto-rust | status: SAFE | lint: CLEAN
findings: Zero unsafe blocks, concurrency patterns appropriate, 3 minor dependency concerns | next: clean test imports / verify notify+enigo platform compat | perf: Arc/RwLock for metrics is good; static Mutexes in native.rs are low-risk
*/

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::errors::TaskErrorKind;

/// Represents the outcome status of a task execution.
/// Used to categorize whether a task completed successfully, failed, or timed out.
///
/// `#[non_exhaustive]` — match with wildcard arm.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum TaskStatus {
    /// Task completed successfully without errors
    Success,
    /// Task failed with an error message describing what went wrong
    Failed(String),
    /// Task exceeded its allocated time limit and was cancelled
    Timeout,
    /// Task was cancelled before completion
    Cancelled,
}

/// Contains the complete result of a task execution, including status, retry information,
/// and performance metrics. This struct is returned by all task executions to provide
/// comprehensive feedback about what happened.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// The final status of the task execution
    pub status: TaskStatus,
    /// Which attempt number this result represents (1-based)
    pub attempt: u32,
    /// Maximum number of retry attempts allowed for this task
    pub max_retries: u32,
    /// The most recent error message, if the task failed
    pub last_error: Option<String>,
    /// Classified error kind for failed outcomes
    pub error_kind: Option<TaskErrorKind>,
    /// Total execution time in milliseconds
    pub duration_ms: u64,
    /// Optional task-specific metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
}

impl TaskResult {
    /// Creates a new successful task result with the given duration.
    /// This is a convenience constructor for tasks that complete without errors.
    ///
    /// # Arguments
    /// * `duration_ms` - Time taken to execute the task in milliseconds
    ///
    /// # Returns
    /// A `TaskResult` with Success status and default retry values
    #[must_use]
    pub fn success(duration_ms: u64) -> Self {
        Self {
            status: TaskStatus::Success,
            attempt: 1,
            max_retries: 0,
            last_error: None,
            error_kind: None,
            duration_ms,
            metadata: None,
        }
    }

    #[must_use]
    pub fn failure(duration_ms: u64, error: String, error_kind: TaskErrorKind) -> Self {
        let status = if matches!(error_kind, TaskErrorKind::Timeout) {
            TaskStatus::Timeout
        } else {
            TaskStatus::Failed(error.clone())
        };

        Self {
            status,
            attempt: 1,
            max_retries: 0,
            last_error: Some(error),
            error_kind: Some(error_kind),
            duration_ms,
            metadata: None,
        }
    }

    #[must_use]
    pub fn cancelled(duration_ms: u64, error: String, error_kind: TaskErrorKind) -> Self {
        Self {
            status: TaskStatus::Cancelled,
            attempt: 1,
            max_retries: 0,
            last_error: Some(error),
            error_kind: Some(error_kind),
            duration_ms,
            metadata: None,
        }
    }

    /// Updates this result to reflect a retry attempt with error information.
    /// This method modifies the result in place and returns self for method chaining.
    ///
    /// # Arguments
    /// * `attempt` - The current attempt number (1-based)
    /// * `max_retries` - Maximum allowed retry attempts
    /// * `last_error` - Error message from the failed attempt
    ///
    /// # Returns
    /// Self with updated retry information and Failed status
    #[must_use]
    pub fn with_retry(mut self, attempt: u32, max_retries: u32, last_error: String) -> Self {
        self.attempt = attempt;
        self.max_retries = max_retries;
        self.last_error = Some(last_error);
        self
    }

    #[must_use]
    pub fn with_attempt(mut self, attempt: u32, max_retries: u32) -> Self {
        self.attempt = attempt;
        self.max_retries = max_retries;
        self
    }

    #[must_use]
    pub fn with_error_kind(mut self, error_kind: TaskErrorKind) -> Self {
        self.error_kind = Some(error_kind);
        self
    }

    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Success)
    }
}

/// A boxed function that returns a `TaskResult` when executed.
/// Used for deferred task execution and retry mechanisms.
/// The function must be Send and Sync for use in async contexts.
pub type TaskResultFn = Box<dyn Fn() -> TaskResult + Send + Sync>;
