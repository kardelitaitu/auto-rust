use serde::{Deserialize, Serialize};

/// Represents the outcome status of a task execution.
/// Used to categorize whether a task completed successfully, failed, or timed out.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
}

impl TaskResult {
    /// Creates a new successful task result with the given duration.
    /// This is a convenience constructor for tasks that complete without errors.
    ///
    /// # Arguments
    /// * `duration_ms` - Time taken to execute the task in milliseconds
    ///
    /// # Returns
    /// A TaskResult with Success status and default retry values
    pub fn success(duration_ms: u64) -> Self {
        Self {
            status: TaskStatus::Success,
            attempt: 1,
            max_retries: 0,
            last_error: None,
            error_kind: None,
            duration_ms,
        }
    }

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
        }
    }

    pub fn cancelled(duration_ms: u64, error: String, error_kind: TaskErrorKind) -> Self {
        Self {
            status: TaskStatus::Cancelled,
            attempt: 1,
            max_retries: 0,
            last_error: Some(error),
            error_kind: Some(error_kind),
            duration_ms,
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
    pub fn with_retry(mut self, attempt: u32, max_retries: u32, last_error: String) -> Self {
        self.attempt = attempt;
        self.max_retries = max_retries;
        self.last_error = Some(last_error);
        self
    }

    pub fn with_attempt(mut self, attempt: u32, max_retries: u32) -> Self {
        self.attempt = attempt;
        self.max_retries = max_retries;
        self
    }

    pub fn with_error_kind(mut self, error_kind: TaskErrorKind) -> Self {
        self.error_kind = Some(error_kind);
        self
    }

    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Success)
    }
}

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
    /// Browser connection or automation error (WebDriver issues, browser crashes)
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
    /// The most appropriate TaskErrorKind for the given error message
    pub fn classify(error: &str) -> Self {
        let e = error.to_lowercase();
        if e.contains("timeout") || e.contains("deadline") {
            TaskErrorKind::Timeout
        } else if e.contains("validation") || e.contains("invalid") || e.contains("schema") {
            TaskErrorKind::Validation
        } else if e.contains("navigat") || e.contains("goto") || e.contains("load") {
            TaskErrorKind::Navigation
        } else if e.contains("session") || e.contains("worker") || e.contains("page") {
            TaskErrorKind::Session
        } else if e.contains("browser") || e.contains("chromium") || e.contains("brave") {
            TaskErrorKind::Browser
        } else {
            TaskErrorKind::Unknown
        }
    }

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

/// A boxed function that returns a TaskResult when executed.
/// Used for deferred task execution and retry mechanisms.
/// The function must be Send and Sync for use in async contexts.
pub type TaskResultFn = Box<dyn Fn() -> TaskResult + Send + Sync>;

/// Aggregates statistics and results from a complete orchestration run.
/// Provides a comprehensive summary of all tasks executed, including success rates,
/// timing information, and individual task results for analysis and reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    /// Total number of tasks that were attempted
    pub total_tasks: usize,
    /// Number of tasks that completed successfully
    pub succeeded: usize,
    /// Number of tasks that failed permanently
    pub failed: usize,
    /// Number of tasks that timed out
    pub timed_out: usize,
    /// Number of tasks that were cancelled
    pub cancelled: usize,
    /// Total duration of the entire run in milliseconds
    pub total_duration_ms: u64,
    /// Detailed results for each individual task
    pub results: Vec<TaskResult>,
}

impl RunSummary {
    pub fn new() -> Self {
        Self {
            total_tasks: 0,
            succeeded: 0,
            failed: 0,
            timed_out: 0,
            cancelled: 0,
            total_duration_ms: 0,
            results: Vec::new(),
        }
    }

    pub fn add(&mut self, result: TaskResult) {
        self.total_tasks += 1;
        self.total_duration_ms += result.duration_ms;

        match result.status {
            TaskStatus::Success => self.succeeded += 1,
            TaskStatus::Failed(_) => self.failed += 1,
            TaskStatus::Timeout => self.timed_out += 1,
            TaskStatus::Cancelled => self.cancelled += 1,
        }

        self.results.push(result);
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tasks == 0 {
            return 0.0;
        }
        (self.succeeded as f64 / self.total_tasks as f64) * 100.0
    }
}

impl Default for RunSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_result_success() {
        let result = TaskResult::success(100);
        assert!(result.is_success());
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.attempt, 1);
    }

    #[test]
    fn test_task_result_failure() {
        let result = TaskResult::failure(50, "test error".to_string(), TaskErrorKind::Browser);
        assert!(!result.is_success());
        assert_eq!(result.duration_ms, 50);
        assert_eq!(result.last_error, Some("test error".to_string()));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Browser));
    }

    #[test]
    fn test_task_result_timeout_uses_timeout_status() {
        let result = TaskResult::failure(50, "timed out".to_string(), TaskErrorKind::Timeout);
        assert!(!result.is_success());
        assert!(matches!(result.status, TaskStatus::Timeout));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Timeout));
    }

    #[test]
    fn test_task_result_cancelled_uses_cancelled_status() {
        let result = TaskResult::cancelled(
            50,
            "cancelled during execution".to_string(),
            TaskErrorKind::Timeout,
        );
        assert!(!result.is_success());
        assert!(matches!(result.status, TaskStatus::Cancelled));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Timeout));
    }

    #[test]
    fn test_task_result_with_retry() {
        let result = TaskResult::success(10).with_retry(2, 3, "retry error".to_string());
        assert_eq!(result.attempt, 2);
        assert_eq!(result.max_retries, 3);
        assert_eq!(result.last_error, Some("retry error".to_string()));
    }

    #[test]
    fn test_task_error_kind_classify_timeout() {
        let kind = TaskErrorKind::classify("Operation exceeded timeout");
        assert_eq!(kind, TaskErrorKind::Timeout);
    }

    #[test]
    fn test_task_error_kind_classify_validation() {
        let kind = TaskErrorKind::classify("Invalid input schema");
        assert_eq!(kind, TaskErrorKind::Validation);
    }

    #[test]
    fn test_task_error_kind_classify_navigation() {
        let kind = TaskErrorKind::classify("Failed to navigate to URL");
        assert_eq!(kind, TaskErrorKind::Navigation);
    }

    #[test]
    fn test_task_error_kind_classify_session() {
        let kind = TaskErrorKind::classify("Session expired");
        assert_eq!(kind, TaskErrorKind::Session);
    }

    #[test]
    fn test_task_error_kind_classify_browser() {
        let kind = TaskErrorKind::classify("Browser crashed");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_unknown() {
        let kind = TaskErrorKind::classify("Something went wrong");
        assert_eq!(kind, TaskErrorKind::Unknown);
    }

    #[test]
    fn test_task_error_kind_retryable() {
        assert!(TaskErrorKind::Timeout.is_retryable());
        assert!(TaskErrorKind::Navigation.is_retryable());
        assert!(TaskErrorKind::Session.is_retryable());
        assert!(TaskErrorKind::Browser.is_retryable());
        assert!(!TaskErrorKind::Validation.is_retryable());
    }

    #[test]
    fn test_run_summary_new() {
        let summary = RunSummary::new();
        assert_eq!(summary.total_tasks, 0);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.cancelled, 0);
    }

    #[test]
    fn test_run_summary_add_success() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 0);
    }

    #[test]
    fn test_run_summary_add_failure() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(
            50,
            "error".to_string(),
            TaskErrorKind::Browser,
        ));
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn test_run_summary_add_cancelled() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::cancelled(
            25,
            "cancelled".to_string(),
            TaskErrorKind::Timeout,
        ));
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.cancelled, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.timed_out, 0);
    }

    #[test]
    fn test_run_summary_success_rate() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(
            50,
            "e".to_string(),
            TaskErrorKind::Browser,
        ));
        assert!((summary.success_rate() - 66.66).abs() < 0.1);
    }

    #[test]
    fn test_run_summary_empty_success_rate() {
        let summary = RunSummary::new();
        assert_eq!(summary.success_rate(), 0.0);
    }
}
