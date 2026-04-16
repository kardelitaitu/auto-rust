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

    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Success)
    }
}

/// Categorizes different types of errors that can occur during task execution.
/// This enum helps with error handling, logging, and debugging by classifying
/// errors into specific categories for appropriate handling.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
}

/// A boxed function that returns a TaskResult when executed.
/// Used for deferred task execution and retry mechanisms.
/// The function must be Send and Sync for use in async contexts.
#[allow(dead_code)]
pub type TaskResultFn = Box<dyn Fn() -> TaskResult + Send + Sync>;

/// Aggregates statistics and results from a complete orchestration run.
/// Provides a comprehensive summary of all tasks executed, including success rates,
/// timing information, and individual task results for analysis and reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct RunSummary {
    /// Total number of tasks that were attempted
    pub total_tasks: usize,
    /// Number of tasks that completed successfully
    pub succeeded: usize,
    /// Number of tasks that failed permanently
    pub failed: usize,
    /// Number of tasks that timed out
    pub timed_out: usize,
    /// Total duration of the entire run in milliseconds
    pub total_duration_ms: u64,
    /// Detailed results for each individual task
    pub results: Vec<TaskResult>,
}

impl RunSummary {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            total_tasks: 0,
            succeeded: 0,
            failed: 0,
            timed_out: 0,
            total_duration_ms: 0,
            results: Vec::new(),
        }
    }

    #[allow(dead_code)]
    pub fn add(&mut self, result: TaskResult) {
        self.total_tasks += 1;
        self.total_duration_ms += result.duration_ms;

        match result.status {
            TaskStatus::Success => self.succeeded += 1,
            TaskStatus::Failed(_) => self.failed += 1,
            TaskStatus::Timeout => self.timed_out += 1,
        }

        self.results.push(result);
    }

    #[allow(dead_code)]
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
