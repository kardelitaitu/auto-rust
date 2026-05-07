//! Performance profiling and execution reporting for DSL tasks.
//!
//! Provides action profiling, metrics tracking, and comprehensive
//! execution reports for DSL task runs.

use std::time::{Duration, Instant};

/// Performance profiler for action execution.
#[derive(Debug, Default)]
pub struct ActionProfiler {
    /// Action type
    pub action_type: String,
    /// Total executions
    pub total_executions: u64,
    /// Total duration across all executions
    pub total_duration: Duration,
    /// Minimum execution time
    pub min_duration: Option<Duration>,
    /// Maximum execution time
    pub max_duration: Option<Duration>,
    /// Number of failures
    pub failures: u64,
}

impl ActionProfiler {
    /// Record an action execution.
    #[allow(dead_code)]
    pub fn record(&mut self, duration: Duration, success: bool) {
        self.total_executions += 1;
        self.total_duration += duration;

        if let Some(min) = self.min_duration {
            if duration < min {
                self.min_duration = Some(duration);
            }
        } else {
            self.min_duration = Some(duration);
        }

        if let Some(max) = self.max_duration {
            if duration > max {
                self.max_duration = Some(duration);
            }
        } else {
            self.max_duration = Some(duration);
        }

        if !success {
            self.failures += 1;
        }
    }

    /// Get average execution duration.
    pub fn average_duration(&self) -> Option<Duration> {
        if self.total_executions > 0 {
            Some(self.total_duration / self.total_executions as u32)
        } else {
            None
        }
    }
}

/// Detailed metrics for a single action execution.
#[derive(Debug, Clone)]
pub struct ActionMetrics {
    /// Action index in the task
    pub index: usize,
    /// Action type name
    pub action_type: String,
    /// Start timestamp
    pub start_time: Instant,
    /// End timestamp (if completed)
    pub end_time: Option<Instant>,
    /// Execution duration (if completed)
    pub duration: Option<Duration>,
    /// Whether the action succeeded
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl ActionMetrics {
    /// Create a new action metrics tracker.
    pub fn new(index: usize, action_type: &str) -> Self {
        Self {
            index,
            action_type: action_type.to_string(),
            start_time: Instant::now(),
            end_time: None,
            duration: None,
            success: false,
            error: None,
        }
    }

    /// Mark the action as completed successfully.
    pub fn complete(mut self) -> Self {
        self.end_time = Some(Instant::now());
        self.duration = Some(self.end_time.unwrap().duration_since(self.start_time));
        self.success = true;
        self
    }

    /// Mark the action as failed.
    pub fn fail(mut self, error: &str) -> Self {
        self.end_time = Some(Instant::now());
        self.duration = Some(self.end_time.unwrap().duration_since(self.start_time));
        self.success = false;
        self.error = Some(error.to_string());
        self
    }
}

/// Comprehensive execution report for a DSL task.
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    /// Task name
    pub task_name: String,
    /// Task execution start time
    pub start_time: Instant,
    /// Task execution end time
    pub end_time: Option<Instant>,
    /// Total execution duration
    pub total_duration: Option<Duration>,
    /// Number of actions in the task
    pub total_actions: u32,
    /// Number of actions executed
    pub actions_executed: u32,
    /// Number of successful actions
    pub actions_succeeded: u32,
    /// Number of failed actions
    pub actions_failed: u32,
    /// Maximum call depth reached
    pub max_call_depth: u32,
    /// Variables defined during execution
    pub variables_defined: usize,
    /// Detailed metrics for each action
    pub action_metrics: Vec<ActionMetrics>,
    /// Overall success status
    pub success: bool,
}

impl ExecutionReport {
    /// Generate a human-readable summary of the execution.
    pub fn summary(&self) -> String {
        let duration = self
            .total_duration
            .map(|d| format!("{:?}", d))
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "Task '{}' executed {} actions in {} ({} successful, {} failed)",
            self.task_name,
            self.actions_executed,
            duration,
            self.actions_succeeded,
            self.actions_failed
        )
    }

    /// Export the report as JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "task_name": self.task_name,
            "total_actions": self.total_actions,
            "actions_executed": self.actions_executed,
            "actions_succeeded": self.actions_succeeded,
            "actions_failed": self.actions_failed,
            "max_call_depth": self.max_call_depth,
            "variables_defined": self.variables_defined,
            "success": self.success,
            "action_metrics": self.action_metrics.iter().map(|m| {
                serde_json::json!({
                    "index": m.index,
                    "action_type": &m.action_type,
                    "success": m.success,
                    "duration_ms": m.duration.map(|d| d.as_millis() as u64),
                    "error": &m.error,
                })
            }).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_profiler_record() {
        let mut profiler = ActionProfiler {
            action_type: "Click".to_string(),
            ..Default::default()
        };

        profiler.record(Duration::from_millis(100), true);
        profiler.record(Duration::from_millis(200), true);
        profiler.record(Duration::from_millis(50), false);

        assert_eq!(profiler.total_executions, 3);
        assert_eq!(profiler.failures, 1);
        assert!(profiler.average_duration().is_some());
    }

    #[test]
    fn test_action_metrics_complete() {
        let metrics = ActionMetrics::new(0, "Click");
        let completed = metrics.complete();

        assert!(completed.success);
        assert!(completed.duration.is_some());
    }

    #[test]
    fn test_action_metrics_fail() {
        let metrics = ActionMetrics::new(1, "Type");
        let failed = metrics.fail("Timeout");

        assert!(!failed.success);
        assert!(failed.error.is_some());
    }

    #[test]
    fn test_execution_report_summary() {
        let report = ExecutionReport {
            task_name: "test-task".to_string(),
            start_time: Instant::now(),
            end_time: Some(Instant::now()),
            total_duration: Some(Duration::from_secs(5)),
            total_actions: 10,
            actions_executed: 10,
            actions_succeeded: 8,
            actions_failed: 2,
            max_call_depth: 2,
            variables_defined: 3,
            action_metrics: vec![],
            success: true,
        };

        let summary = report.summary();
        assert!(summary.contains("test-task"));
        assert!(summary.contains("8 successful"));
    }

    #[test]
    fn test_execution_report_to_json() {
        let report = ExecutionReport {
            task_name: "test-task".to_string(),
            start_time: Instant::now(),
            end_time: None,
            total_duration: None,
            total_actions: 5,
            actions_executed: 5,
            actions_succeeded: 5,
            actions_failed: 0,
            max_call_depth: 1,
            variables_defined: 2,
            action_metrics: vec![],
            success: true,
        };

        let json = report.to_json();
        assert_eq!(json["task_name"], "test-task");
        assert_eq!(json["actions_succeeded"], 5);
    }
}
