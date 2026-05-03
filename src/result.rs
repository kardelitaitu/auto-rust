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
#[allow(missing_docs)]
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
    fn test_task_error_kind_classify_target_detached() {
        let kind = TaskErrorKind::classify(
            "Protocol error (Target.detachedFromTarget): Target closed during click",
        );
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_receiver_gone() {
        let kind = TaskErrorKind::classify("send failed because receiver is gone");
        assert_eq!(kind, TaskErrorKind::Session);
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

    #[test]
    fn test_task_status_partial_eq() {
        assert_eq!(TaskStatus::Success, TaskStatus::Success);
        assert_ne!(TaskStatus::Success, TaskStatus::Failed("error".to_string()));
    }

    #[test]
    fn test_task_result_with_attempt() {
        let result = TaskResult::success(100).with_attempt(3, 5);
        assert_eq!(result.attempt, 3);
        assert_eq!(result.max_retries, 5);
    }

    #[test]
    fn test_task_result_with_error_kind() {
        let result = TaskResult::success(100).with_error_kind(TaskErrorKind::Timeout);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Timeout));
    }

    #[test]
    fn test_task_error_kind_ord() {
        assert!(TaskErrorKind::Timeout < TaskErrorKind::Unknown);
        assert!(TaskErrorKind::Validation < TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TaskErrorKind::Timeout);
        set.insert(TaskErrorKind::Validation);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_task_result_zero_duration() {
        let result = TaskResult::success(0);
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn test_task_result_large_duration() {
        let result = TaskResult::success(u64::MAX);
        assert_eq!(result.duration_ms, u64::MAX);
    }

    #[test]
    fn test_task_result_chain_with_retry_and_attempt() {
        let result = TaskResult::success(100)
            .with_retry(2, 3, "error".to_string())
            .with_attempt(3, 5);
        assert_eq!(result.attempt, 3);
        assert_eq!(result.max_retries, 5);
    }

    #[test]
    fn test_run_summary_default() {
        let summary = RunSummary::default();
        assert_eq!(summary.total_tasks, 0);
    }

    #[test]
    fn test_run_summary_add_timeout() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(
            50,
            "timeout".to_string(),
            TaskErrorKind::Timeout,
        ));
        assert_eq!(summary.timed_out, 1);
        assert_eq!(summary.failed, 0);
    }

    #[test]
    fn test_run_summary_multiple_results() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(
            50,
            "error".to_string(),
            TaskErrorKind::Browser,
        ));
        summary.add(TaskResult::success(100));
        assert_eq!(summary.total_tasks, 3);
        assert_eq!(summary.succeeded, 2);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn test_run_summary_total_duration() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::success(200));
        summary.add(TaskResult::success(50));
        assert_eq!(summary.total_duration_ms, 350);
    }

    #[test]
    fn test_task_status_serialize() {
        let status = TaskStatus::Success;
        let serialized = serde_json::to_string(&status).expect("Failed to serialize TaskStatus");
        assert!(serialized.contains("Success"));
    }

    #[test]
    fn test_task_result_serialize() {
        let result = TaskResult::success(100);
        let serialized = serde_json::to_string(&result).expect("Failed to serialize TaskResult");
        assert!(serialized.contains("duration_ms"));
    }

    #[test]
    fn test_run_summary_serialize() {
        let summary = RunSummary::new();
        let serialized = serde_json::to_string(&summary).expect("Failed to serialize RunSummary");
        assert!(serialized.contains("total_tasks"));
    }

    #[test]
    fn test_task_error_kind_copy() {
        let kind1 = TaskErrorKind::Timeout;
        let kind2 = kind1;
        assert_eq!(kind1, kind2);
    }

    #[test]
    fn test_task_status_failed_with_message() {
        let status = TaskStatus::Failed("Test error message".to_string());
        assert!(matches!(status, TaskStatus::Failed(_)));
    }

    #[test]
    fn test_task_status_failed_with_empty_message() {
        let status = TaskStatus::Failed("".to_string());
        assert!(matches!(status, TaskStatus::Failed(_)));
    }

    #[test]
    fn test_task_status_failed_with_long_message() {
        let long_msg = "a".repeat(1000);
        let status = TaskStatus::Failed(long_msg.clone());
        if let TaskStatus::Failed(msg) = status {
            assert_eq!(msg.len(), 1000);
        } else {
            panic!("Expected Failed status");
        }
    }

    #[test]
    fn test_task_result_clone() {
        let result = TaskResult::success(100);
        let cloned = result.clone();
        assert_eq!(result.duration_ms, cloned.duration_ms);
        assert_eq!(result.status, cloned.status);
    }

    #[test]
    fn test_task_result_debug() {
        let result = TaskResult::success(100);
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("TaskResult"));
    }

    #[test]
    fn test_task_error_kind_partial_ord() {
        assert!(TaskErrorKind::Timeout <= TaskErrorKind::Timeout);
        assert!(TaskErrorKind::Timeout < TaskErrorKind::Unknown);
    }

    #[test]
    fn test_task_error_kind_eq() {
        assert_eq!(TaskErrorKind::Timeout, TaskErrorKind::Timeout);
        assert_ne!(TaskErrorKind::Timeout, TaskErrorKind::Validation);
    }

    #[test]
    fn test_task_error_kind_all_variants() {
        let variants = [
            TaskErrorKind::Timeout,
            TaskErrorKind::Validation,
            TaskErrorKind::Navigation,
            TaskErrorKind::Session,
            TaskErrorKind::Browser,
            TaskErrorKind::Unknown,
        ];
        assert_eq!(variants.len(), 6);
    }

    #[test]
    fn test_task_error_kind_classify_case_insensitive() {
        let kind1 = TaskErrorKind::classify("TIMEOUT ERROR");
        let kind2 = TaskErrorKind::classify("timeout error");
        assert_eq!(kind1, kind2);
    }

    #[test]
    fn test_task_error_kind_classify_multiple_keywords() {
        let kind = TaskErrorKind::classify("Browser timeout during navigation");
        // Should classify as timeout since timeout is checked first
        assert_eq!(kind, TaskErrorKind::Timeout);
    }

    #[test]
    fn test_task_error_kind_classify_websocket_error() {
        let kind = TaskErrorKind::classify("WebSocket connection failed");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_channel_closed() {
        let kind = TaskErrorKind::classify("channel closed");
        assert_eq!(kind, TaskErrorKind::Session);
    }

    #[test]
    fn test_task_error_kind_classify_worker_error() {
        let kind = TaskErrorKind::classify("worker acquisition failed");
        assert_eq!(kind, TaskErrorKind::Session);
    }

    #[test]
    fn test_task_error_kind_classify_page_error() {
        let kind = TaskErrorKind::classify("page not found");
        assert_eq!(kind, TaskErrorKind::Session);
    }

    #[test]
    fn test_task_error_kind_classify_chromium_error() {
        let kind = TaskErrorKind::classify("chromium process crashed");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_brave_error() {
        let kind = TaskErrorKind::classify("brave browser disconnected");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_deadline_error() {
        let kind = TaskErrorKind::classify("deadline exceeded");
        assert_eq!(kind, TaskErrorKind::Timeout);
    }

    #[test]
    fn test_task_error_kind_classify_schema_error() {
        let kind = TaskErrorKind::classify("schema validation failed");
        assert_eq!(kind, TaskErrorKind::Validation);
    }

    #[test]
    fn test_task_error_kind_classify_invalid_error() {
        let kind = TaskErrorKind::classify("invalid parameter");
        assert_eq!(kind, TaskErrorKind::Validation);
    }

    #[test]
    fn test_task_error_kind_classify_goto_error() {
        let kind = TaskErrorKind::classify("goto failed");
        assert_eq!(kind, TaskErrorKind::Navigation);
    }

    #[test]
    fn test_task_error_kind_classify_load_error() {
        let kind = TaskErrorKind::classify("page load failed");
        assert_eq!(kind, TaskErrorKind::Navigation);
    }

    #[test]
    fn test_task_error_kind_classify_connection_reset() {
        let kind = TaskErrorKind::classify("connection reset by peer");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_connection_closed() {
        let kind = TaskErrorKind::classify("connection closed");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_protocol_error() {
        let kind = TaskErrorKind::classify("protocol error");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_send_failed() {
        let kind = TaskErrorKind::classify("send failed");
        assert_eq!(kind, TaskErrorKind::Session);
    }

    #[test]
    fn test_task_result_with_retry_zero_attempt() {
        let result = TaskResult::success(100).with_retry(0, 3, "error".to_string());
        assert_eq!(result.attempt, 0);
    }

    #[test]
    fn test_task_result_with_retry_zero_max_retries() {
        let result = TaskResult::success(100).with_retry(1, 0, "error".to_string());
        assert_eq!(result.max_retries, 0);
    }

    #[test]
    fn test_task_result_with_attempt_zero_values() {
        let result = TaskResult::success(100).with_attempt(0, 0);
        assert_eq!(result.attempt, 0);
        assert_eq!(result.max_retries, 0);
    }

    #[test]
    fn test_task_result_with_error_kind_all_variants() {
        let result = TaskResult::success(100).with_error_kind(TaskErrorKind::Validation);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Validation));
    }

    #[test]
    fn test_run_summary_add_multiple_timeouts() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(
            50,
            "timeout".to_string(),
            TaskErrorKind::Timeout,
        ));
        summary.add(TaskResult::failure(
            50,
            "timeout".to_string(),
            TaskErrorKind::Timeout,
        ));
        assert_eq!(summary.timed_out, 2);
    }

    #[test]
    fn test_run_summary_add_multiple_cancelled() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::cancelled(
            25,
            "cancelled".to_string(),
            TaskErrorKind::Timeout,
        ));
        summary.add(TaskResult::cancelled(
            25,
            "cancelled".to_string(),
            TaskErrorKind::Timeout,
        ));
        assert_eq!(summary.cancelled, 2);
    }

    #[test]
    fn test_run_summary_success_rate_100_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::success(100));
        assert_eq!(summary.success_rate(), 100.0);
    }

    #[test]
    fn test_run_summary_success_rate_0_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(
            50,
            "error".to_string(),
            TaskErrorKind::Browser,
        ));
        summary.add(TaskResult::failure(
            50,
            "error".to_string(),
            TaskErrorKind::Browser,
        ));
        assert_eq!(summary.success_rate(), 0.0);
    }

    #[test]
    fn test_run_summary_success_rate_50_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(
            50,
            "error".to_string(),
            TaskErrorKind::Browser,
        ));
        assert_eq!(summary.success_rate(), 50.0);
    }

    #[test]
    fn test_run_summary_results_vec() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(
            50,
            "error".to_string(),
            TaskErrorKind::Browser,
        ));
        assert_eq!(summary.results.len(), 2);
    }

    #[test]
    fn test_run_summary_clone() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        let cloned = summary.clone();
        assert_eq!(cloned.total_tasks, 1);
        assert_eq!(cloned.succeeded, 1);
    }

    #[test]
    fn test_run_summary_debug() {
        let summary = RunSummary::new();
        let debug_str = format!("{:?}", summary);
        assert!(debug_str.contains("RunSummary"));
    }

    #[test]
    fn test_task_status_deserialize() {
        let json = r#"{"Failed":"test error"}"#;
        let status: TaskStatus =
            serde_json::from_str(json).expect("Failed to deserialize TaskStatus");
        assert!(matches!(status, TaskStatus::Failed(_)));
    }

    #[test]
    fn test_task_result_deserialize() {
        let json = r#"{"status":"Success","attempt":1,"max_retries":0,"last_error":null,"error_kind":null,"duration_ms":100}"#;
        let result: TaskResult =
            serde_json::from_str(json).expect("Failed to deserialize TaskResult");
        assert!(result.is_success());
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn test_run_summary_deserialize() {
        let json = r#"{"total_tasks":0,"succeeded":0,"failed":0,"timed_out":0,"cancelled":0,"total_duration_ms":0,"results":[]}"#;
        let summary: RunSummary =
            serde_json::from_str(json).expect("Failed to deserialize RunSummary");
        assert_eq!(summary.total_tasks, 0);
    }

    #[test]
    fn test_task_result_with_all_fields() {
        let result = TaskResult {
            status: TaskStatus::Success,
            attempt: 5,
            max_retries: 10,
            last_error: Some("test error".to_string()),
            error_kind: Some(TaskErrorKind::Browser),
            duration_ms: 1000,
        };
        assert_eq!(result.attempt, 5);
        assert_eq!(result.max_retries, 10);
    }

    #[test]
    fn test_task_result_is_success_false_for_failed() {
        let result = TaskResult::failure(50, "error".to_string(), TaskErrorKind::Browser);
        assert!(!result.is_success());
    }

    #[test]
    fn test_task_result_is_success_false_for_timeout() {
        let result = TaskResult::failure(50, "timeout".to_string(), TaskErrorKind::Timeout);
        assert!(!result.is_success());
    }

    #[test]
    fn test_task_result_is_success_false_for_cancelled() {
        let result = TaskResult::cancelled(50, "cancelled".to_string(), TaskErrorKind::Timeout);
        assert!(!result.is_success());
    }

    #[test]
    fn test_task_status_serialize_failed() {
        let status = TaskStatus::Failed("test".to_string());
        let serialized = serde_json::to_string(&status).expect("Failed to serialize TaskStatus");
        assert!(serialized.contains("Failed"));
    }

    #[test]
    fn test_task_status_serialize_timeout() {
        let status = TaskStatus::Timeout;
        let serialized = serde_json::to_string(&status).expect("Failed to serialize TaskStatus");
        assert!(serialized.contains("Timeout"));
    }

    #[test]
    fn test_task_status_serialize_cancelled() {
        let status = TaskStatus::Cancelled;
        let serialized = serde_json::to_string(&status).expect("Failed to serialize TaskStatus");
        assert!(serialized.contains("Cancelled"));
    }

    #[test]
    fn test_task_error_kind_serialize() {
        let kind = TaskErrorKind::Timeout;
        let serialized = serde_json::to_string(&kind).expect("Failed to serialize TaskErrorKind");
        assert!(serialized.contains("Timeout"));
    }

    #[test]
    fn test_task_error_kind_deserialize() {
        let json = r#""Timeout""#;
        let kind: TaskErrorKind =
            serde_json::from_str(json).expect("Failed to deserialize TaskErrorKind");
        assert_eq!(kind, TaskErrorKind::Timeout);
    }

    #[test]
    fn test_run_summary_results_preserve_order() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(
            50,
            "error1".to_string(),
            TaskErrorKind::Browser,
        ));
        summary.add(TaskResult::success(200));
        assert_eq!(summary.results[0].duration_ms, 100);
        assert_eq!(summary.results[1].duration_ms, 50);
        assert_eq!(summary.results[2].duration_ms, 200);
    }

    #[test]
    fn test_task_result_failure_with_timeout_kind_sets_timeout_status() {
        let result = TaskResult::failure(50, "any error".to_string(), TaskErrorKind::Timeout);
        assert!(matches!(result.status, TaskStatus::Timeout));
    }

    #[test]
    fn test_task_result_failure_with_non_timeout_kind_sets_failed_status() {
        let result = TaskResult::failure(50, "any error".to_string(), TaskErrorKind::Browser);
        assert!(matches!(result.status, TaskStatus::Failed(_)));
    }

    #[test]
    fn test_task_result_cancelled_preserves_error_kind() {
        let result = TaskResult::cancelled(50, "cancelled".to_string(), TaskErrorKind::Session);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Session));
    }

    #[test]
    fn test_task_result_with_retry_preserves_status() {
        let result = TaskResult::success(100).with_retry(2, 3, "error".to_string());
        assert!(matches!(result.status, TaskStatus::Success));
    }

    #[test]
    fn test_task_result_with_error_kind_overwrites() {
        let result = TaskResult::success(100)
            .with_error_kind(TaskErrorKind::Timeout)
            .with_error_kind(TaskErrorKind::Browser);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Browser));
    }

    #[test]
    fn test_run_summary_add_does_not_modify_original_result() {
        let result = TaskResult::success(100);
        let original_duration = result.duration_ms;
        let mut summary = RunSummary::new();
        summary.add(result.clone());
        assert_eq!(result.duration_ms, original_duration);
    }

    #[test]
    fn test_task_status_all_variants() {
        let variants = [
            TaskStatus::Success,
            TaskStatus::Failed("test".to_string()),
            TaskStatus::Timeout,
            TaskStatus::Cancelled,
        ];
        assert_eq!(variants.len(), 4);
    }
}
