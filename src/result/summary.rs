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

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use super::*;
    use crate::result::errors::TaskErrorKind;

    // =========================================================================
    // RunSummary::new() tests
    // =========================================================================

    #[test]
    fn run_summary_new_all_fields_zero() {
        let summary = RunSummary::new();
        assert_eq!(summary.total_tasks, 0);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.timed_out, 0);
        assert_eq!(summary.cancelled, 0);
        assert_eq!(summary.total_duration_ms, 0);
        assert!(summary.results.is_empty());
    }

    #[test]
    fn run_summary_new_results_vec_is_empty() {
        let summary = RunSummary::new();
        assert!(summary.results.is_empty());
        assert_eq!(summary.results.capacity(), 0);
    }

    // =========================================================================
    // RunSummary::add() — success tests
    // =========================================================================

    #[test]
    fn run_summary_add_success_increments_succeeded() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.timed_out, 0);
        assert_eq!(summary.cancelled, 0);
    }

    #[test]
    fn run_summary_add_success_accumulates_duration() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(150));
        assert_eq!(summary.total_duration_ms, 150);
    }

    #[test]
    fn run_summary_add_success_appends_result() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(200));
        assert_eq!(summary.results.len(), 1);
        assert!(summary.results[0].is_success());
        assert_eq!(summary.results[0].duration_ms, 200);
    }

    // =========================================================================
    // RunSummary::add() — failure tests
    // =========================================================================

    #[test]
    fn run_summary_add_failure_increments_failed() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(50, "error".to_string(), TaskErrorKind::Browser));
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.timed_out, 0);
        assert_eq!(summary.cancelled, 0);
    }

    #[test]
    fn run_summary_add_failure_accumulates_duration() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(75, "err".to_string(), TaskErrorKind::Validation));
        assert_eq!(summary.total_duration_ms, 75);
    }

    #[test]
    fn run_summary_add_failure_appends_result() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(30, "fail".to_string(), TaskErrorKind::Session));
        assert_eq!(summary.results.len(), 1);
        assert!(!summary.results[0].is_success());
    }

    // =========================================================================
    // RunSummary::add() — timeout tests
    // =========================================================================

    #[test]
    fn run_summary_add_timeout_increments_timed_out() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(10, "timeout".to_string(), TaskErrorKind::Timeout));
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.timed_out, 1);
        assert_eq!(summary.cancelled, 0);
    }

    #[test]
    fn run_summary_add_timeout_does_not_increment_failed() {
        // Timeout goes to timed_out counter, NOT failed
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(10, "timeout".to_string(), TaskErrorKind::Timeout));
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.timed_out, 1);
    }

    // =========================================================================
    // RunSummary::add() — cancelled tests
    // =========================================================================

    #[test]
    fn run_summary_add_cancelled_increments_cancelled() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::cancelled(5, "cancel".to_string(), TaskErrorKind::Session));
        assert_eq!(summary.total_tasks, 1);
        assert_eq!(summary.succeeded, 0);
        assert_eq!(summary.failed, 0);
        assert_eq!(summary.timed_out, 0);
        assert_eq!(summary.cancelled, 1);
    }

    #[test]
    fn run_summary_add_cancelled_accumulates_duration() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::cancelled(20, "cancel".to_string(), TaskErrorKind::Session));
        assert_eq!(summary.total_duration_ms, 20);
    }

    // =========================================================================
    // RunSummary::add() — mixed results
    // =========================================================================

    #[test]
    fn run_summary_add_multiple_results_all_statuses() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(50, "err".to_string(), TaskErrorKind::Browser));
        summary.add(TaskResult::failure(10, "timeout".to_string(), TaskErrorKind::Timeout));
        summary.add(TaskResult::cancelled(5, "cancel".to_string(), TaskErrorKind::Session));

        assert_eq!(summary.total_tasks, 4);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.timed_out, 1);
        assert_eq!(summary.cancelled, 1);
        assert_eq!(summary.total_duration_ms, 165);
        assert_eq!(summary.results.len(), 4);
    }

    #[test]
    fn run_summary_add_preserves_result_order() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::failure(20, "fail".to_string(), TaskErrorKind::Browser));
        summary.add(TaskResult::success(30));

        assert_eq!(summary.results.len(), 3);
        assert!(summary.results[0].is_success());
        assert!(!summary.results[1].is_success());
        assert!(summary.results[2].is_success());
    }

    #[test]
    fn run_summary_add_twenty_results() {
        let mut summary = RunSummary::new();
        for i in 0..20 {
            if i % 2 == 0 {
                summary.add(TaskResult::success(i * 10));
            } else {
                summary.add(TaskResult::failure(
                    i as u64 * 5,
                    format!("err_{}", i),
                    TaskErrorKind::Browser,
                ));
            }
        }
        assert_eq!(summary.total_tasks, 20);
        assert_eq!(summary.succeeded, 10);
        assert_eq!(summary.failed, 10);
    }

    // =========================================================================
    // RunSummary::success_rate() tests
    // =========================================================================

    #[test]
    fn run_summary_success_rate_zero_total() {
        let summary = RunSummary::new();
        assert_eq!(summary.success_rate(), 0.0);
    }

    #[test]
    fn run_summary_success_rate_zero_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(10, "err".to_string(), TaskErrorKind::Browser));
        assert_eq!(summary.success_rate(), 0.0);
    }

    #[test]
    fn run_summary_success_rate_hundred_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(10));
        assert_eq!(summary.success_rate(), 100.0);
    }

    #[test]
    fn run_summary_success_rate_fifty_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::failure(20, "err".to_string(), TaskErrorKind::Browser));
        assert!((summary.success_rate() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn run_summary_success_rate_twenty_five_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::failure(20, "e1".to_string(), TaskErrorKind::Browser));
        summary.add(TaskResult::failure(20, "e2".to_string(), TaskErrorKind::Session));
        summary.add(TaskResult::failure(20, "e3".to_string(), TaskErrorKind::Timeout));
        assert!((summary.success_rate() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn run_summary_success_rate_seventy_five_percent() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::failure(20, "e1".to_string(), TaskErrorKind::Browser));
        assert!((summary.success_rate() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn run_summary_success_rate_only_failures_ignores_other_statuses() {
        // success_rate is purely succeeded / total_tasks
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(10, "err".to_string(), TaskErrorKind::Browser));
        summary.add(TaskResult::failure(10, "timeout".to_string(), TaskErrorKind::Timeout));
        summary.add(TaskResult::cancelled(5, "cancel".to_string(), TaskErrorKind::Session));
        assert_eq!(summary.success_rate(), 0.0);
    }

    #[test]
    fn run_summary_success_rate_timeout_not_counted_as_success() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::failure(10, "timeout".to_string(), TaskErrorKind::Timeout));
        assert_eq!(summary.success_rate(), 0.0);
    }

    #[test]
    fn run_summary_success_rate_cancelled_not_counted_as_success() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::cancelled(5, "cancel".to_string(), TaskErrorKind::Session));
        assert_eq!(summary.success_rate(), 0.0);
    }

    #[test]
    fn run_summary_success_rate_partial_with_all_four_statuses() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(10));   // 1 success
        summary.add(TaskResult::failure(5, "a".to_string(), TaskErrorKind::Browser));  // 1 failed
        summary.add(TaskResult::failure(3, "b".to_string(), TaskErrorKind::Timeout));  // 1 timeout
        summary.add(TaskResult::cancelled(1, "c".to_string(), TaskErrorKind::Session)); // 1 cancelled
        // Total 4 tasks, 1 success => 25%
        assert!((summary.success_rate() - 25.0).abs() < f64::EPSILON);
    }

    // =========================================================================
    // RunSummary::Default tests
    // =========================================================================

    #[test]
    fn run_summary_default_matches_new() {
        let default = RunSummary::default();
        let new = RunSummary::new();
        assert_eq!(default.total_tasks, new.total_tasks);
        assert_eq!(default.succeeded, new.succeeded);
        assert_eq!(default.failed, new.failed);
        assert_eq!(default.timed_out, new.timed_out);
        assert_eq!(default.cancelled, new.cancelled);
        assert_eq!(default.total_duration_ms, new.total_duration_ms);
        assert_eq!(default.results.len(), new.results.len());
    }

    #[test]
    fn run_summary_default_chaining() {
        let mut summary = RunSummary::default();
        assert_eq!(summary.total_tasks, 0);
        summary.add(TaskResult::success(50));
        assert_eq!(summary.total_tasks, 1);
    }

    // =========================================================================
    // Display for TaskStatus tests
    // =========================================================================

    #[test]
    fn task_status_display_success() {
        assert_eq!(format!("{}", TaskStatus::Success), "Success");
    }

    #[test]
    fn task_status_display_failed() {
        assert_eq!(
            format!("{}", TaskStatus::Failed("oops".to_string())),
            "Failed: oops"
        );
    }

    #[test]
    fn task_status_display_timeout() {
        assert_eq!(format!("{}", TaskStatus::Timeout), "Timeout");
    }

    #[test]
    fn task_status_display_cancelled() {
        assert_eq!(format!("{}", TaskStatus::Cancelled), "Cancelled");
    }

    #[test]
    fn task_status_display_failed_with_empty_message() {
        assert_eq!(format!("{}", TaskStatus::Failed(String::new())), "Failed: ");
    }

    #[test]
    fn task_status_display_failed_with_long_message() {
        let msg = "x".repeat(100);
        let display = format!("{}", TaskStatus::Failed(msg));
        assert!(display.starts_with("Failed: "));
        assert_eq!(display.len(), 8 + 100);  // "Failed: " = 8 chars
    }

    // =========================================================================
    // RunSummary Serialize/Deserialize tests
    // =========================================================================

    #[test]
    fn run_summary_serde_round_trip_empty() {
        let original = RunSummary::new();
        let json = serde_json::to_string(&original).unwrap();
        let round: RunSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(round.total_tasks, 0);
        assert_eq!(round.succeeded, 0);
        assert_eq!(round.results.len(), 0);
    }

    #[test]
    fn run_summary_serde_round_trip_with_results() {
        let mut original = RunSummary::new();
        original.add(TaskResult::success(100));
        original.add(TaskResult::failure(50, "err".to_string(), TaskErrorKind::Browser));
        original.add(TaskResult::failure(10, "timeout".to_string(), TaskErrorKind::Timeout));

        let json = serde_json::to_string(&original).unwrap();
        let round: RunSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(round.total_tasks, 3);
        assert_eq!(round.succeeded, 1);
        assert_eq!(round.failed, 1);
        assert_eq!(round.timed_out, 1);
        assert_eq!(round.cancelled, 0);
        assert_eq!(round.total_duration_ms, 160);
        assert_eq!(round.results.len(), 3);
    }

    #[test]
    fn run_summary_serde_json_output_contains_fields() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"total_tasks\""));
        assert!(json.contains("\"succeeded\""));
        assert!(json.contains("\"failed\""));
        assert!(json.contains("\"timed_out\""));
        assert!(json.contains("\"cancelled\""));
        assert!(json.contains("\"total_duration_ms\""));
        assert!(json.contains("\"results\""));
    }

    // =========================================================================
    // Edge cases
    // =========================================================================

    #[test]
    fn run_summary_large_duration_accumulation() {
        let mut summary = RunSummary::new();
        for _ in 0..100 {
            summary.add(TaskResult::success(1000));
        }
        assert_eq!(summary.total_tasks, 100);
        assert_eq!(summary.total_duration_ms, 100_000);
        assert_eq!(summary.results.len(), 100);
    }

    #[test]
    fn run_summary_many_failures() {
        let mut summary = RunSummary::new();
        for i in 0..50 {
            summary.add(TaskResult::failure(
                i as u64,
                format!("error_{}", i),
                TaskErrorKind::Validation,
            ));
        }
        assert_eq!(summary.total_tasks, 50);
        assert_eq!(summary.failed, 50);
        assert_eq!(summary.succeeded, 0);
    }

    #[test]
    fn run_summary_add_does_not_panic_with_zero_duration() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(0));
        assert_eq!(summary.total_duration_ms, 0);
        assert_eq!(summary.total_tasks, 1);
    }

    #[test]
    fn run_summary_success_rate_stable_over_many_calls() {
        let mut summary = RunSummary::new();
        // Add 100 tasks, 40 successes, 60 total failures
        for i in 0..100 {
            if i < 40 {
                summary.add(TaskResult::success(1));
            } else if i < 70 {
                summary.add(TaskResult::failure(1, "f".to_string(), TaskErrorKind::Browser));
            } else if i < 90 {
                summary.add(TaskResult::failure(1, "t".to_string(), TaskErrorKind::Timeout));
            } else {
                summary.add(TaskResult::cancelled(1, "c".to_string(), TaskErrorKind::Session));
            }
        }
        assert_eq!(summary.total_tasks, 100);
        assert_eq!(summary.succeeded, 40);
        assert_eq!(summary.failed, 30);
        assert_eq!(summary.timed_out, 20);
        assert_eq!(summary.cancelled, 10);
        assert!((summary.success_rate() - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn run_summary_debug_trait_implemented() {
        // RunSummary derives Debug — verify Debug formatting works
        let summary = RunSummary::new();
        let debug = format!("{:?}", summary);
        assert!(debug.contains("RunSummary"));
        assert!(debug.contains("total_tasks"));
    }

    #[test]
    fn run_summary_clone() {
        let mut original = RunSummary::new();
        original.add(TaskResult::success(42));
        let cloned = original.clone();
        assert_eq!(original.total_tasks, cloned.total_tasks);
        assert_eq!(original.total_duration_ms, cloned.total_duration_ms);
        assert_eq!(original.results.len(), cloned.results.len());
    }
}
