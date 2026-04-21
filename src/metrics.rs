//! Performance metrics collection and reporting module.
//!
//! Provides real-time monitoring of:
//! - Task execution counts and success rates
//! - Performance timing and duration tracking
//! - Historical task records for analysis
//! - Run summary export to JSON files
//! - Memory usage monitoring

use log::{info, warn};
use parking_lot::RwLock;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::result::TaskErrorKind;
use crate::result::TaskResult;

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
    /// Classified error kind for failed outcomes
    pub error_kind: Option<TaskErrorKind>,
    /// Error message for failed outcomes
    pub last_error: Option<String>,
}

/// Memory usage snapshot for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct MemorySnapshot {
    /// Total allocated memory in bytes (if available)
    pub allocated_bytes: Option<usize>,
    /// Number of active sessions
    pub active_sessions: usize,
    /// Number of active workers/tasks
    pub active_workers: usize,
    /// Task queue depth
    pub task_queue_depth: usize,
    /// Timestamp of snapshot
    pub timestamp_ms: u64,
}

/// Memory and performance monitoring thresholds
#[derive(Debug, Clone)]
pub struct MemoryThresholds {
    /// Warning threshold for memory usage (bytes)
    pub warning_bytes: usize,
    /// Critical threshold for memory usage (bytes)
    pub critical_bytes: usize,
    /// Warning threshold for active tasks
    pub warning_active_tasks: usize,
    /// Critical threshold for active tasks
    pub critical_active_tasks: usize,
}

impl Default for MemoryThresholds {
    fn default() -> Self {
        Self {
            warning_bytes: 500 * 1024 * 1024,   // 500 MB
            critical_bytes: 1024 * 1024 * 1024, // 1 GB
            warning_active_tasks: 50,
            critical_active_tasks: 100,
        }
    }
}

/// Status of a task execution outcome.
/// Uses unit variant `Failed` for metrics (error message stored separately in last_error).
/// Differs from result::TaskStatus::Failed(String) which stores the error for persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Success,
    Failed,
    Timeout,
    Cancelled,
}

/// Counts outcomes for a task or session.
#[derive(Debug, Clone, Default, Serialize)]
pub struct OutcomeBreakdown {
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub cancelled: usize,
}

impl OutcomeBreakdown {
    fn record(&mut self, status: TaskStatus) {
        match status {
            TaskStatus::Success => self.succeeded += 1,
            TaskStatus::Failed => self.failed += 1,
            TaskStatus::Timeout => self.timed_out += 1,
            TaskStatus::Cancelled => self.cancelled += 1,
        }
    }

    fn failure_count(&self) -> usize {
        self.failed + self.timed_out + self.cancelled
    }
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
    /// Number of tasks that were cancelled
    cancelled: Arc<AtomicUsize>,
    /// Total execution time across all tasks in milliseconds
    total_duration_ms: Arc<AtomicUsize>,
    /// Number of currently active tasks
    active_tasks: Arc<AtomicUsize>,
    /// Rolling history of recent task executions
    task_history: Arc<RwLock<VecDeque<TaskMetrics>>>,
    /// Breakdown of failures by error kind
    failure_breakdown: Arc<RwLock<BTreeMap<TaskErrorKind, usize>>>,
    /// Outcome breakdown by task name
    task_breakdown: Arc<RwLock<BTreeMap<String, OutcomeBreakdown>>>,
    /// Outcome breakdown by session id
    session_breakdown: Arc<RwLock<BTreeMap<String, OutcomeBreakdown>>>,
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
            cancelled: Arc::new(AtomicUsize::new(0)),
            total_duration_ms: Arc::new(AtomicUsize::new(0)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            task_history: Arc::new(RwLock::new(VecDeque::new())),
            failure_breakdown: Arc::new(RwLock::new(BTreeMap::new())),
            task_breakdown: Arc::new(RwLock::new(BTreeMap::new())),
            session_breakdown: Arc::new(RwLock::new(BTreeMap::new())),
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

        {
            let mut task_breakdown = self.task_breakdown.write();
            task_breakdown
                .entry(metrics.task_name.clone())
                .or_default()
                .record(metrics.status);
        }

        {
            let mut session_breakdown = self.session_breakdown.write();
            session_breakdown
                .entry(metrics.session_id.clone())
                .or_default()
                .record(metrics.status);
        }

        match metrics.status {
            TaskStatus::Success => {
                self.succeeded.fetch_add(1, Ordering::SeqCst);
            }
            TaskStatus::Failed => {
                self.failed.fetch_add(1, Ordering::SeqCst);
                if let Some(kind) = metrics.error_kind {
                    let mut breakdown = self.failure_breakdown.write();
                    *breakdown.entry(kind).or_insert(0) += 1;
                }
            }
            TaskStatus::Timeout => {
                self.timed_out.fetch_add(1, Ordering::SeqCst);
                let mut breakdown = self.failure_breakdown.write();
                *breakdown.entry(TaskErrorKind::Timeout).or_insert(0) += 1;
            }
            TaskStatus::Cancelled => {
                self.cancelled.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut history = self.task_history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(metrics);
    }

    pub fn task_completed_from_result(
        &self,
        task_name: String,
        session_id: String,
        result: &TaskResult,
    ) {
        let status = match &result.status {
            crate::result::TaskStatus::Success => TaskStatus::Success,
            crate::result::TaskStatus::Failed(_) => TaskStatus::Failed,
            crate::result::TaskStatus::Timeout => TaskStatus::Timeout,
            crate::result::TaskStatus::Cancelled => TaskStatus::Cancelled,
        };

        self.task_completed(TaskMetrics {
            task_name,
            status,
            duration_ms: result.duration_ms,
            session_id,
            attempt: result.attempt,
            error_kind: result.error_kind,
            last_error: result.last_error.clone(),
        });
    }

    pub fn get_stats(&self) -> MetricsSnapshot {
        let failure_breakdown = self
            .failure_breakdown
            .read()
            .iter()
            .map(|(kind, count)| (format!("{:?}", kind), *count))
            .collect();
        let task_breakdown = self.task_breakdown.read().clone();
        let session_breakdown = self.session_breakdown.read().clone();

        MetricsSnapshot {
            total_tasks: self.total_tasks.load(Ordering::SeqCst),
            succeeded: self.succeeded.load(Ordering::SeqCst),
            failed: self.failed.load(Ordering::SeqCst),
            timed_out: self.timed_out.load(Ordering::SeqCst),
            cancelled: self.cancelled.load(Ordering::SeqCst),
            active_tasks: self.active_tasks.load(Ordering::SeqCst),
            total_duration_ms: self.total_duration_ms.load(Ordering::SeqCst) as u64,
            failure_breakdown,
            task_breakdown,
            session_breakdown,
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

    /// Get a memory and performance snapshot
    pub fn get_memory_snapshot(
        &self,
        active_sessions: usize,
        task_queue_depth: usize,
    ) -> MemorySnapshot {
        MemorySnapshot {
            allocated_bytes: get_allocated_memory(),
            active_sessions,
            active_workers: self.active_tasks.load(Ordering::SeqCst),
            task_queue_depth,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Check memory and task thresholds, log warnings if exceeded
    pub fn check_thresholds(&self, thresholds: &MemoryThresholds, active_sessions: usize) {
        let snapshot = self.get_memory_snapshot(active_sessions, 0);

        // Check active tasks
        if snapshot.active_workers >= thresholds.critical_active_tasks {
            warn!(
                "CRITICAL: Active tasks ({}) exceeds critical threshold ({})",
                snapshot.active_workers, thresholds.critical_active_tasks
            );
        } else if snapshot.active_workers >= thresholds.warning_active_tasks {
            warn!(
                "WARNING: Active tasks ({}) exceeds warning threshold ({})",
                snapshot.active_workers, thresholds.warning_active_tasks
            );
        }
    }

    /// Log current memory and performance status
    pub fn log_status(&self, active_sessions: usize) {
        let snapshot = self.get_memory_snapshot(active_sessions, 0);
        let stats = self.get_stats();
        let failures = self.failure_breakdown.read();

        let memory_str = match snapshot.allocated_bytes {
            Some(bytes) => format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0)),
            None => "N/A".to_string(),
        };

        info!(
            "Metrics Report | Sessions: {} | Active tasks: {} | Total: {} | Success: {:.1}% | Memory: {}",
            active_sessions,
            snapshot.active_workers,
            stats.total_tasks,
            self.success_rate(),
            memory_str
        );

        if let Some(top_task) = top_breakdown(&stats.task_breakdown) {
            info!("Metrics task breakdown | top={top_task}");
        }

        if let Some(top_session) = top_breakdown(&stats.session_breakdown) {
            info!("Metrics session breakdown | top={top_session}");
        }

        if !failures.is_empty() {
            let top_failure = failures
                .iter()
                .max_by_key(|(_, count)| **count)
                .map(|(kind, count)| format!("{:?}={}", kind, count))
                .unwrap_or_else(|| "none".to_string());
            info!(
                "Metrics failure breakdown | top={top_failure} | kinds={}",
                failures.len()
            );
        }
    }
}

/// Get current allocated memory (platform-specific)
#[cfg(target_os = "linux")]
fn get_allocated_memory() -> Option<usize> {
    // Try to read from /proc/self/status
    use std::fs;
    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<usize>().ok().map(|kb| kb * 1024);
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn get_allocated_memory() -> Option<usize> {
    // On Windows, we'd need to use winapi crate for accurate memory info
    // For now, return None
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn get_allocated_memory() -> Option<usize> {
    None
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
    /// Number of tasks that were cancelled
    pub cancelled: usize,
    /// Number of tasks currently in progress
    pub active_tasks: usize,
    /// Total execution time across all completed tasks in milliseconds
    pub total_duration_ms: u64,
    /// Failure counts grouped by error kind
    pub failure_breakdown: BTreeMap<String, usize>,
    /// Outcome counts grouped by task name
    pub task_breakdown: BTreeMap<String, OutcomeBreakdown>,
    /// Outcome counts grouped by session id
    pub session_breakdown: BTreeMap<String, OutcomeBreakdown>,
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
    /// Number of tasks that were cancelled
    pub cancelled: usize,
    /// Overall success rate as a percentage
    pub success_rate: f64,
    /// Number of browser sessions available during the run
    pub active_sessions: usize,
    /// Number of healthy browser sessions during the run
    pub healthy_sessions: usize,
    /// Number of unhealthy browser sessions during the run
    pub unhealthy_sessions: usize,
    /// Total execution time for the entire run in milliseconds
    pub total_duration_ms: u64,
    /// Failure counts grouped by error kind
    pub failure_breakdown: BTreeMap<String, usize>,
    /// Outcome counts grouped by task name
    pub task_breakdown: BTreeMap<String, OutcomeBreakdown>,
    /// Outcome counts grouped by session id
    pub session_breakdown: BTreeMap<String, OutcomeBreakdown>,
}

impl MetricsCollector {
    pub fn export_summary(
        &self,
        active_sessions: usize,
        healthy_sessions: usize,
    ) -> Result<(), std::io::Error> {
        self.export_summary_to("run-summary.json", active_sessions, healthy_sessions)
    }

    pub fn export_summary_to<P: AsRef<Path>>(
        &self,
        path: P,
        active_sessions: usize,
        healthy_sessions: usize,
    ) -> Result<(), std::io::Error> {
        let path_display = path.as_ref().display().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let stats = self.get_stats();
        let unhealthy_sessions = active_sessions.saturating_sub(healthy_sessions);

        let summary = RunSummary {
            timestamp: now,
            total_tasks: stats.total_tasks,
            succeeded: stats.succeeded,
            failed: stats.failed,
            timed_out: stats.timed_out,
            cancelled: stats.cancelled,
            success_rate: self.success_rate(),
            active_sessions,
            healthy_sessions,
            unhealthy_sessions,
            total_duration_ms: stats.total_duration_ms,
            failure_breakdown: stats.failure_breakdown,
            task_breakdown: stats.task_breakdown,
            session_breakdown: stats.session_breakdown,
        };

        let json = serde_json::to_string_pretty(&summary)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;

        log::info!("Exported {path_display}");
        Ok(())
    }
}

fn top_breakdown(items: &BTreeMap<String, OutcomeBreakdown>) -> Option<String> {
    items
        .iter()
        .max_by_key(|(_, outcome)| outcome.failure_count())
        .and_then(|(name, outcome)| {
            let failures = outcome.failure_count();
            if failures == 0 {
                None
            } else {
                Some(format!("{name}={failures}"))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_new() {
        let collector = MetricsCollector::new(100);
        let stats = collector.get_stats();
        assert_eq!(stats.total_tasks, 0);
        assert_eq!(stats.succeeded, 0);
        assert_eq!(stats.cancelled, 0);
    }

    #[test]
    fn test_task_started_increments_total() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        let stats = collector.get_stats();
        assert_eq!(stats.total_tasks, 1);
    }

    #[test]
    fn test_task_completed_success() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Success,
            duration_ms: 100,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });
        let stats = collector.get_stats();
        assert_eq!(stats.succeeded, 1);
        assert_eq!(stats.total_tasks, 1);
    }

    #[test]
    fn test_task_completed_failure() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Failed,
            duration_ms: 50,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: Some(TaskErrorKind::Browser),
            last_error: Some("error".to_string()),
        });
        let stats = collector.get_stats();
        assert_eq!(stats.failed, 1);
    }

    #[test]
    fn test_task_completed_timeout() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Timeout,
            duration_ms: 30000,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });
        let stats = collector.get_stats();
        assert_eq!(stats.timed_out, 1);
    }

    #[test]
    fn test_task_completed_cancelled() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Cancelled,
            duration_ms: 15,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: Some("cancelled".to_string()),
        });
        let stats = collector.get_stats();
        assert_eq!(stats.cancelled, 1);
    }

    #[test]
    fn test_success_rate_calculation() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Success,
            duration_ms: 100,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Failed,
            duration_ms: 50,
            session_id: "s2".to_string(),
            attempt: 1,
            error_kind: Some(TaskErrorKind::Browser),
            last_error: Some("e".to_string()),
        });
        let rate = collector.success_rate();
        assert!((rate - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_success_rate_empty() {
        let collector = MetricsCollector::new(100);
        let rate = collector.success_rate();
        assert_eq!(rate, 0.0);
    }

    #[test]
    fn test_memory_thresholds_defaults() {
        let thresholds = MemoryThresholds::default();
        assert_eq!(thresholds.warning_bytes, 500 * 1024 * 1024);
        assert_eq!(thresholds.critical_bytes, 1024 * 1024 * 1024);
        assert_eq!(thresholds.warning_active_tasks, 50);
        assert_eq!(thresholds.critical_active_tasks, 100);
    }

    #[test]
    fn test_failure_breakdown_tracks_errors() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Failed,
            duration_ms: 50,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: Some(TaskErrorKind::Timeout),
            last_error: Some("timeout".to_string()),
        });
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "test".to_string(),
            status: TaskStatus::Failed,
            duration_ms: 50,
            session_id: "s1".to_string(),
            attempt: 1,
            error_kind: Some(TaskErrorKind::Timeout),
            last_error: Some("timeout".to_string()),
        });
        let stats = collector.get_stats();
        assert_eq!(stats.failure_breakdown.get("Timeout"), Some(&2));
    }

    #[test]
    fn test_task_and_session_breakdown_tracks_outcomes() {
        let collector = MetricsCollector::new(100);

        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "pageview".to_string(),
            status: TaskStatus::Success,
            duration_ms: 20,
            session_id: "brave-1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });

        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "pageview".to_string(),
            status: TaskStatus::Timeout,
            duration_ms: 40,
            session_id: "brave-1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: Some("timeout".to_string()),
        });

        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "pageview".to_string(),
            status: TaskStatus::Cancelled,
            duration_ms: 10,
            session_id: "brave-2".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: Some("cancelled".to_string()),
        });

        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "twitterreply".to_string(),
            status: TaskStatus::Failed,
            duration_ms: 60,
            session_id: "brave-2".to_string(),
            attempt: 1,
            error_kind: Some(TaskErrorKind::Browser),
            last_error: Some("browser".to_string()),
        });

        let stats = collector.get_stats();
        let pageview = stats.task_breakdown.get("pageview").unwrap();
        assert_eq!(pageview.succeeded, 1);
        assert_eq!(pageview.timed_out, 1);
        assert_eq!(pageview.cancelled, 1);

        let brave1 = stats.session_breakdown.get("brave-1").unwrap();
        assert_eq!(brave1.succeeded, 1);
        assert_eq!(brave1.timed_out, 1);

        let brave2 = stats.session_breakdown.get("brave-2").unwrap();
        assert_eq!(brave2.cancelled, 1);
    }

    #[test]
    fn test_export_summary_includes_session_health() {
        let collector = MetricsCollector::new(100);
        collector.task_started();
        collector.task_completed(TaskMetrics {
            task_name: "pageview".to_string(),
            status: TaskStatus::Success,
            duration_ms: 20,
            session_id: "brave-1".to_string(),
            attempt: 1,
            error_kind: None,
            last_error: None,
        });

        let unique = format!(
            "run-summary-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);

        collector
            .export_summary_to(&path, 3, 2)
            .expect("export summary");

        let json = std::fs::read_to_string(&path).expect("read summary");
        let summary: serde_json::Value = serde_json::from_str(&json).expect("parse summary");
        assert_eq!(summary["active_sessions"], 3);
        assert_eq!(summary["healthy_sessions"], 2);
        assert_eq!(summary["unhealthy_sessions"], 1);
        assert_eq!(summary["cancelled"], 0);
        assert!(
            summary["task_breakdown"]["pageview"]["succeeded"]
                .as_u64()
                .unwrap()
                >= 1
        );

        let _ = std::fs::remove_file(&path);
    }
}
