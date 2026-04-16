//! Performance metrics collection and reporting module.
//!
//! Provides real-time monitoring of:
//! - Task execution counts and success rates
//! - Performance timing and duration tracking
//! - Historical task records for analysis
//! - Run summary export to JSON files

use parking_lot::RwLock;
use serde::Serialize;
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Records detailed metrics for a single task execution.
/// Captures timing, outcome, and execution context for performance analysis
/// and debugging purposes.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TaskMetrics {
    /// Name of the task that was executed
    pub task_name: String,
    /// Final status of the task execution
    pub status: TaskStatus,
    /// Time taken to execute the task in milliseconds
    pub duration_ms: u64,
    /// ID of the session that executed the task
    pub session_id: String,
    /// Which attempt number this execution represents
    pub attempt: u32,
}

/// Status of a task execution outcome.
/// Note: This is a duplicate of the TaskStatus enum in result.rs.
/// Consider consolidating these in the future.
/// TODO: Use the TaskStatus from result.rs instead
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskStatus {
    /// Task completed successfully
    Success,
    /// Task failed with an error
    Failed,
    /// Task exceeded its timeout limit
    Timeout,
}

/// Collects and aggregates performance metrics across all task executions.
/// Provides real-time statistics and historical data for monitoring system health
/// and performance. Thread-safe for concurrent access.
#[allow(dead_code)]
pub struct MetricsCollector {
    /// Total number of tasks executed
    total_tasks: Arc<AtomicUsize>,
    /// Number of tasks that succeeded
    succeeded: Arc<AtomicUsize>,
    /// Number of tasks that failed
    failed: Arc<AtomicUsize>,
    /// Number of tasks that timed out
    timed_out: Arc<AtomicUsize>,
    /// Total execution time across all tasks in milliseconds
    total_duration_ms: Arc<AtomicUsize>,
    /// Number of currently active tasks
    active_tasks: Arc<AtomicUsize>,
    /// Rolling history of recent task executions
    task_history: Arc<RwLock<VecDeque<TaskMetrics>>>,
    /// Maximum number of historical records to keep
    max_history: usize,
}

impl MetricsCollector {
    /// Creates a new metrics collector with the specified history buffer size.
    ///
    /// # Arguments
    /// * `max_history` - Maximum number of task records to keep in memory
    ///
    /// # Returns
    /// A new MetricsCollector instance ready for recording metrics
    #[allow(dead_code)]
    pub fn new(max_history: usize) -> Self {
        Self {
            total_tasks: Arc::new(AtomicUsize::new(0)),
            succeeded: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
            timed_out: Arc::new(AtomicUsize::new(0)),
            total_duration_ms: Arc::new(AtomicUsize::new(0)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            task_history: Arc::new(RwLock::new(VecDeque::new())),
            max_history,
        }
    }

    #[allow(dead_code)]
    pub fn task_started(&self) {
        self.active_tasks.fetch_add(1, Ordering::SeqCst);
        self.total_tasks.fetch_add(1, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn task_completed(&self, metrics: TaskMetrics) {
        self.active_tasks.fetch_sub(1, Ordering::SeqCst);
        self.total_duration_ms
            .fetch_add(metrics.duration_ms as usize, Ordering::SeqCst);

        match metrics.status {
            TaskStatus::Success => {
                self.succeeded.fetch_add(1, Ordering::SeqCst);
            }
            TaskStatus::Failed => {
                self.failed.fetch_add(1, Ordering::SeqCst);
            }
            TaskStatus::Timeout => {
                self.timed_out.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut history = self.task_history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(metrics);
    }

    pub fn get_stats(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            total_tasks: self.total_tasks.load(Ordering::SeqCst),
            succeeded: self.succeeded.load(Ordering::SeqCst),
            failed: self.failed.load(Ordering::SeqCst),
            timed_out: self.timed_out.load(Ordering::SeqCst),
            active_tasks: self.active_tasks.load(Ordering::SeqCst),
            total_duration_ms: self.total_duration_ms.load(Ordering::SeqCst) as u64,
        }
    }

    /// Calculates the success rate as a percentage of total tasks.
    /// Returns 0.0 if no tasks have been executed.
    ///
    /// # Returns
    /// Success rate as a percentage (0.0 to 100.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.total_tasks.load(Ordering::SeqCst);
        if total == 0 {
            return 0.0;
        }
        (self.succeeded.load(Ordering::SeqCst) as f64 / total as f64) * 100.0
    }
}

/// A point-in-time snapshot of metrics data.
/// Contains all current metric values for serialization and reporting.
/// Used for exporting metrics to external systems or files.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    /// Total number of tasks executed so far
    pub total_tasks: usize,
    /// Number of tasks that completed successfully
    pub succeeded: usize,
    /// Number of tasks that failed
    pub failed: usize,
    /// Number of tasks that timed out
    pub timed_out: usize,
    /// Number of tasks currently in progress
    pub active_tasks: usize,
    /// Total execution time across all completed tasks in milliseconds
    pub total_duration_ms: u64,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Summary of a complete orchestration run exported to run-summary.json.
/// Contains final statistics and outcome of all tasks executed during the run.
/// Used for post-run analysis and reporting.
#[derive(Debug, Serialize)]
pub struct RunSummary {
    /// ISO 8601 timestamp when the summary was generated
    pub timestamp: String,
    /// Total number of tasks attempted during the run
    pub total_tasks: usize,
    /// Number of tasks that completed successfully
    pub succeeded: usize,
    /// Number of tasks that failed permanently
    pub failed: usize,
    /// Number of tasks that timed out
    pub timed_out: usize,
    /// Overall success rate as a percentage
    pub success_rate: f64,
    /// Total execution time for the entire run in milliseconds
    pub total_duration_ms: u64,
}

impl MetricsCollector {
    pub fn export_summary(&self) -> Result<(), std::io::Error> {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let stats = self.get_stats();

        let summary = RunSummary {
            timestamp: now,
            total_tasks: stats.total_tasks,
            succeeded: stats.succeeded,
            failed: stats.failed,
            timed_out: stats.timed_out,
            success_rate: self.success_rate(),
            total_duration_ms: stats.total_duration_ms,
        };

        let json = serde_json::to_string_pretty(&summary)?;
        let mut file = File::create("run-summary.json")?;
        file.write_all(json.as_bytes())?;

        log::info!("Exported run-summary.json");
        Ok(())
    }
}
