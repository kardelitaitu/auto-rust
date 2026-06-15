use serde::{Deserialize, Serialize};

use super::types::{TaskResult, TaskStatus};

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
    #[must_use]
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

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
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

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Success => write!(f, "Success"),
            TaskStatus::Failed(msg) => write!(f, "Failed: {msg}"),
            TaskStatus::Timeout => write!(f, "Timeout"),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}
