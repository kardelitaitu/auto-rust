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

#[cfg(test)]
#[allow(missing_docs)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    // =========================================================================
    // TaskStatus tests
    // =========================================================================

    #[test]
    fn task_status_all_variants() {
        let variants = [
            TaskStatus::Success,
            TaskStatus::Failed("error".to_string()),
            TaskStatus::Timeout,
            TaskStatus::Cancelled,
        ];
        assert_eq!(variants.len(), 4);
    }

    #[test]
    fn task_status_debug() {
        let s = TaskStatus::Success;
        assert!(format!("{:?}", s).contains("Success"));

        let f = TaskStatus::Failed("err".to_string());
        assert!(format!("{:?}", f).contains("Failed"));
    }

    #[test]
    fn task_status_clone() {
        let s = TaskStatus::Success;
        let cloned = s.clone();
        assert_eq!(s, cloned);

        let f = TaskStatus::Failed("msg".to_string());
        let cloned = f.clone();
        assert_eq!(f, cloned);
    }

    #[test]
    fn task_status_partial_eq() {
        assert_eq!(TaskStatus::Success, TaskStatus::Success);
        assert_ne!(TaskStatus::Success, TaskStatus::Timeout);
        assert_ne!(TaskStatus::Success, TaskStatus::Cancelled);
        assert_eq!(
            TaskStatus::Failed("a".to_string()),
            TaskStatus::Failed("a".to_string())
        );
        assert_ne!(
            TaskStatus::Failed("a".to_string()),
            TaskStatus::Failed("b".to_string())
        );
    }

    #[test]
    fn task_status_failed_with_empty_message() {
        let status = TaskStatus::Failed(String::new());
        assert!(matches!(status, TaskStatus::Failed(_)));
        if let TaskStatus::Failed(msg) = &status {
            assert!(msg.is_empty());
        }
    }

    #[test]
    fn task_status_failed_with_long_message() {
        let long = "x".repeat(10_000);
        let status = TaskStatus::Failed(long.clone());
        if let TaskStatus::Failed(msg) = &status {
            assert_eq!(msg.len(), 10_000);
        }
    }

    #[test]
    fn task_status_serialize_deserialize_success() {
        let json = serde_json::to_string(&TaskStatus::Success).unwrap();
        let back: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TaskStatus::Success);
    }

    #[test]
    fn task_status_serialize_deserialize_failed() {
        let json = serde_json::to_string(&TaskStatus::Failed("err".to_string())).unwrap();
        let back: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TaskStatus::Failed("err".to_string()));
    }

    #[test]
    fn task_status_serialize_deserialize_timeout() {
        let json = serde_json::to_string(&TaskStatus::Timeout).unwrap();
        let back: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TaskStatus::Timeout);
    }

    #[test]
    fn task_status_serialize_deserialize_cancelled() {
        let json = serde_json::to_string(&TaskStatus::Cancelled).unwrap();
        let back: TaskStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TaskStatus::Cancelled);
    }

    #[test]
    fn task_status_non_exhaustive() {
        // Verify we can match with a wildcard (non_exhaustive pattern)
        let s = TaskStatus::Success;
        let desc = match s {
            TaskStatus::Success => "ok",
            _ => "other",
        };
        assert_eq!(desc, "ok");
    }

    // =========================================================================
    // TaskResult::success tests
    // =========================================================================

    #[test]
    fn task_result_success_basic() {
        let result = TaskResult::success(100);
        assert!(result.is_success());
        assert_eq!(result.status, TaskStatus::Success);
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.attempt, 1);
        assert_eq!(result.max_retries, 0);
        assert!(result.last_error.is_none());
        assert!(result.error_kind.is_none());
        assert!(result.metadata.is_none());
    }

    #[test]
    fn task_result_success_zero_duration() {
        let result = TaskResult::success(0);
        assert!(result.is_success());
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn task_result_success_max_duration() {
        let result = TaskResult::success(u64::MAX);
        assert!(result.is_success());
        assert_eq!(result.duration_ms, u64::MAX);
    }

    // =========================================================================
    // TaskResult::failure tests
    // =========================================================================

    #[test]
    fn task_result_failure_basic() {
        let result = TaskResult::failure(50, "error msg".to_string(), TaskErrorKind::Browser);
        assert!(!result.is_success());
        assert_eq!(result.duration_ms, 50);
        assert_eq!(result.last_error, Some("error msg".to_string()));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Browser));
        assert_eq!(result.attempt, 1);
        assert!(result.metadata.is_none());
    }

    #[test]
    fn task_result_failure_with_timeout_kind_sets_timeout_status() {
        let result = TaskResult::failure(50, "timed out".to_string(), TaskErrorKind::Timeout);
        assert!(matches!(result.status, TaskStatus::Timeout));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Timeout));
    }

    #[test]
    fn task_result_failure_with_non_timeout_kind_sets_failed_status() {
        let result = TaskResult::failure(50, "error".to_string(), TaskErrorKind::Validation);
        assert!(matches!(result.status, TaskStatus::Failed(_)));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Validation));
    }

    #[test]
    fn task_result_failure_empty_error_string() {
        let result = TaskResult::failure(10, String::new(), TaskErrorKind::Browser);
        assert_eq!(result.last_error, Some(String::new()));
        assert!(matches!(result.status, TaskStatus::Failed(_)));
    }

    #[test]
    fn task_result_failure_preserves_last_error_matches_status_message() {
        let result = TaskResult::failure(50, "disk full".to_string(), TaskErrorKind::Session);
        assert_eq!(result.last_error, Some("disk full".to_string()));
        if let TaskStatus::Failed(msg) = &result.status {
            assert_eq!(msg, "disk full");
        }
    }

    // =========================================================================
    // TaskResult::cancelled tests
    // =========================================================================

    #[test]
    fn task_result_cancelled_basic() {
        let result = TaskResult::cancelled(25, "cancelled".to_string(), TaskErrorKind::Session);
        assert!(!result.is_success());
        assert_eq!(result.status, TaskStatus::Cancelled);
        assert_eq!(result.duration_ms, 25);
        assert_eq!(result.last_error, Some("cancelled".to_string()));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Session));
        assert_eq!(result.attempt, 1);
    }

    #[test]
    fn task_result_cancelled_preserves_error_kind() {
        let result = TaskResult::cancelled(25, "x".to_string(), TaskErrorKind::Unknown);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Unknown));
    }

    #[test]
    fn task_result_cancelled_empty_error_string() {
        let result = TaskResult::cancelled(10, String::new(), TaskErrorKind::Timeout);
        assert_eq!(result.last_error, Some(String::new()));
    }

    // =========================================================================
    // TaskResult builder methods
    // =========================================================================

    #[test]
    fn task_result_with_retry_updates_fields() {
        let result = TaskResult::success(100).with_retry(2, 3, "retry error".to_string());
        assert_eq!(result.attempt, 2);
        assert_eq!(result.max_retries, 3);
        assert_eq!(result.last_error, Some("retry error".to_string()));
    }

    #[test]
    fn task_result_with_retry_preserves_status() {
        let result = TaskResult::success(100).with_retry(2, 3, "error".to_string());
        assert!(matches!(result.status, TaskStatus::Success));
    }

    #[test]
    fn task_result_with_retry_zero_attempt() {
        let result = TaskResult::success(100).with_retry(0, 3, "error".to_string());
        assert_eq!(result.attempt, 0);
        assert_eq!(result.max_retries, 3);
    }

    #[test]
    fn task_result_with_retry_zero_max_retries() {
        let result = TaskResult::success(100).with_retry(1, 0, "error".to_string());
        assert_eq!(result.max_retries, 0);
        assert_eq!(result.last_error, Some("error".to_string()));
    }

    #[test]
    fn task_result_with_attempt_updates_fields() {
        let result = TaskResult::success(100).with_attempt(5, 10);
        assert_eq!(result.attempt, 5);
        assert_eq!(result.max_retries, 10);
    }

    #[test]
    fn task_result_with_attempt_does_not_affect_error() {
        let result = TaskResult::failure(50, "err".to_string(), TaskErrorKind::Browser)
            .with_attempt(2, 3);
        assert_eq!(result.last_error, Some("err".to_string()));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Browser));
    }

    #[test]
    fn task_result_with_error_kind_sets_kind() {
        let result = TaskResult::success(100).with_error_kind(TaskErrorKind::Timeout);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Timeout));
    }

    #[test]
    fn task_result_with_error_kind_all_variants() {
        for kind in [
            TaskErrorKind::Timeout,
            TaskErrorKind::Validation,
            TaskErrorKind::Navigation,
            TaskErrorKind::Session,
            TaskErrorKind::Browser,
            TaskErrorKind::ExternalService,
            TaskErrorKind::Unknown,
        ] {
            let result = TaskResult::success(100).with_error_kind(kind);
            assert_eq!(result.error_kind, Some(kind));
        }
    }

    #[test]
    fn task_result_with_error_kind_overwrites_previous() {
        let result = TaskResult::success(100)
            .with_error_kind(TaskErrorKind::Timeout)
            .with_error_kind(TaskErrorKind::Browser);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Browser));
    }

    // =========================================================================
    // TaskResult builder chaining
    // =========================================================================

    #[test]
    fn task_result_full_builder_chain() {
        let result = TaskResult::success(100)
            .with_attempt(2, 5)
            .with_error_kind(TaskErrorKind::Timeout)
            .with_retry(3, 5, "timeout on attempt 2".to_string());

        assert_eq!(result.attempt, 3);
        assert_eq!(result.max_retries, 5);
        assert_eq!(result.last_error, Some("timeout on attempt 2".to_string()));
        assert_eq!(result.error_kind, Some(TaskErrorKind::Timeout));
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn task_result_chain_failure_then_builders() {
        let result = TaskResult::failure(50, "failed".to_string(), TaskErrorKind::Browser)
            .with_attempt(3, 5)
            .with_error_kind(TaskErrorKind::Session);

        assert!(matches!(result.status, TaskStatus::Failed(_)));
        assert_eq!(result.attempt, 3);
        assert_eq!(result.max_retries, 5);
        assert_eq!(result.error_kind, Some(TaskErrorKind::Session));
    }

    #[test]
    fn task_result_chain_cancelled_then_retry() {
        let result = TaskResult::cancelled(30, "cancelled".to_string(), TaskErrorKind::Session)
            .with_retry(2, 3, "retry after cancel".to_string());

        assert!(matches!(result.status, TaskStatus::Cancelled));
        assert_eq!(result.attempt, 2);
        assert_eq!(result.max_retries, 3);
        assert_eq!(result.last_error, Some("retry after cancel".to_string()));
    }

    // =========================================================================
    // TaskResult::is_success tests
    // =========================================================================

    #[test]
    fn task_result_is_success_true_for_success() {
        assert!(TaskResult::success(0).is_success());
    }

    #[test]
    fn task_result_is_success_false_for_failure() {
        assert!(!TaskResult::failure(0, "err".to_string(), TaskErrorKind::Browser).is_success());
    }

    #[test]
    fn task_result_is_success_false_for_timeout() {
        assert!(!TaskResult::failure(0, "timeout".to_string(), TaskErrorKind::Timeout).is_success());
    }

    #[test]
    fn task_result_is_success_false_for_cancelled() {
        assert!(
            !TaskResult::cancelled(0, "cancel".to_string(), TaskErrorKind::Session).is_success()
        );
    }

    #[test]
    fn task_result_is_success_stays_false_after_builders() {
        let result = TaskResult::failure(50, "err".to_string(), TaskErrorKind::Browser)
            .with_attempt(2, 3)
            .with_retry(3, 3, "still failing".to_string());
        assert!(!result.is_success());
    }

    // =========================================================================
    // TaskResult struct literal construction
    // =========================================================================

    #[test]
    fn task_result_struct_literal_all_fields() {
        let result = TaskResult {
            status: TaskStatus::Success,
            attempt: 1,
            max_retries: 0,
            last_error: None,
            error_kind: None,
            duration_ms: 42,
            metadata: None,
        };
        assert!(result.is_success());
        assert_eq!(result.duration_ms, 42);
    }

    #[test]
    fn task_result_struct_literal_failure_with_all_fields() {
        let result = TaskResult {
            status: TaskStatus::Failed("crash".to_string()),
            attempt: 3,
            max_retries: 5,
            last_error: Some("crash".to_string()),
            error_kind: Some(TaskErrorKind::Browser),
            duration_ms: 999,
            metadata: None,
        };
        assert!(!result.is_success());
        assert_eq!(result.duration_ms, 999);
    }

    #[test]
    fn task_result_struct_literal_with_metadata() {
        let mut metadata = BTreeMap::new();
        metadata.insert("source".to_string(), "manual".to_string());
        metadata.insert("run_id".to_string(), "abc-123".to_string());

        let result = TaskResult {
            status: TaskStatus::Success,
            attempt: 1,
            max_retries: 0,
            last_error: None,
            error_kind: None,
            duration_ms: 42,
            metadata: Some(metadata),
        };

        assert!(result.is_success());
        assert_eq!(result.metadata.as_ref().unwrap().len(), 2);
        assert_eq!(
            result.metadata.as_ref().unwrap().get("source"),
            Some(&"manual".to_string())
        );
    }

    // =========================================================================
    // TaskResult derived trait tests
    // =========================================================================

    #[test]
    fn task_result_clone() {
        let result = TaskResult::success(100);
        let cloned = result.clone();
        assert_eq!(result.status, cloned.status);
        assert_eq!(result.duration_ms, cloned.duration_ms);
        assert_eq!(result.attempt, cloned.attempt);
    }

    #[test]
    fn task_result_debug() {
        let result = TaskResult::success(100);
        let debug = format!("{:?}", result);
        assert!(debug.contains("TaskResult"));
        assert!(debug.contains("duration_ms"));
    }

    #[test]
    fn task_result_serde_round_trip_success() {
        let original = TaskResult::success(42);
        let json = serde_json::to_string(&original).unwrap();
        let round: TaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(round.status, original.status);
        assert_eq!(round.duration_ms, 42);
        assert_eq!(round.attempt, 1);
    }

    #[test]
    fn task_result_serde_round_trip_failure() {
        let original = TaskResult::failure(50, "err".to_string(), TaskErrorKind::Browser);
        let json = serde_json::to_string(&original).unwrap();
        let round: TaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(round.status, original.status);
        assert_eq!(round.last_error, original.last_error);
        assert_eq!(round.error_kind, original.error_kind);
    }

    #[test]
    fn task_result_serde_round_trip_with_metadata() {
        let mut metadata = BTreeMap::new();
        metadata.insert("env".to_string(), "staging".to_string());

        let original = TaskResult {
            status: TaskStatus::Failed("crash".to_string()),
            attempt: 3,
            max_retries: 5,
            last_error: Some("crash".to_string()),
            error_kind: Some(TaskErrorKind::Browser),
            duration_ms: 999,
            metadata: Some(metadata),
        };

        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("metadata"));
        assert!(json.contains("staging"));

        let round: TaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(round.status, original.status);
        assert_eq!(round.metadata, original.metadata);
        assert_eq!(round.duration_ms, 999);
    }

    #[test]
    fn task_result_serde_round_trip_all_fields() {
        let original = TaskResult {
            status: TaskStatus::Cancelled,
            attempt: 4,
            max_retries: 7,
            last_error: Some("cancelled after retries".to_string()),
            error_kind: Some(TaskErrorKind::Session),
            duration_ms: 5000,
            metadata: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let round: TaskResult = serde_json::from_str(&json).unwrap();
        assert_eq!(round.status, original.status);
        assert_eq!(round.attempt, original.attempt);
        assert_eq!(round.max_retries, original.max_retries);
        assert_eq!(round.last_error, original.last_error);
        assert_eq!(round.error_kind, original.error_kind);
        assert_eq!(round.duration_ms, original.duration_ms);
        assert_eq!(round.metadata, original.metadata);
    }

    // =========================================================================
    // TaskResultFn tests
    // =========================================================================

    #[test]
    fn task_result_fn_type_exists_and_can_be_called() {
        let f: TaskResultFn = Box::new(|| TaskResult::success(0));
        let result = f();
        assert!(result.is_success());
    }

    #[test]
    fn task_result_fn_with_failure() {
        let f: TaskResultFn =
            Box::new(|| TaskResult::failure(100, "fn failure".to_string(), TaskErrorKind::Timeout));
        let result = f();
        assert!(matches!(result.status, TaskStatus::Timeout));
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn task_result_fn_is_send() {
        // Verify TaskResultFn implements Send (required by thread::spawn)
        // Sync is enforced at compile time by the type alias definition (+ Send + Sync)
        let f: TaskResultFn = Box::new(|| TaskResult::success(42));
        let result = std::thread::spawn(move || f()).join().unwrap();
        assert!(result.is_success());
        assert_eq!(result.duration_ms, 42);
    }

    // =========================================================================
    // Edge cases and boundary conditions
    // =========================================================================

    #[test]
    fn task_result_failure_with_all_error_kinds_use_failed_status() {
        let non_timeout_kinds = [
            TaskErrorKind::Validation,
            TaskErrorKind::Navigation,
            TaskErrorKind::Session,
            TaskErrorKind::Browser,
            TaskErrorKind::ExternalService,
            TaskErrorKind::Unknown,
        ];
        for kind in &non_timeout_kinds {
            let result = TaskResult::failure(10, "err".to_string(), *kind);
            assert!(
                matches!(result.status, TaskStatus::Failed(_)),
                "Expected Failed status for {kind:?}"
            );
            assert_eq!(result.error_kind, Some(*kind));
        }
    }

    #[test]
    fn task_result_large_attempt_numbers() {
        let result = TaskResult::success(100)
            .with_attempt(u32::MAX, u32::MAX);
        assert_eq!(result.attempt, u32::MAX);
        assert_eq!(result.max_retries, u32::MAX);
    }

    #[test]
    fn task_result_large_last_error() {
        let long_error = "error ".repeat(1000);
        let result = TaskResult::failure(
            10,
            long_error.clone(),
            TaskErrorKind::Browser,
        );
        assert_eq!(result.last_error.as_ref().unwrap().len(), 6000);
    }

    #[test]
    fn task_result_builder_does_not_affect_original() {
        let original = TaskResult::success(100);
        let _modified = original.clone().with_attempt(5, 10);
        // Original should be unchanged
        assert_eq!(original.attempt, 1);
        assert_eq!(original.max_retries, 0);
    }

    #[test]
    fn task_result_metadata_serde_omitted_when_none() {
        let result = TaskResult::success(100);
        let json = serde_json::to_string(&result).unwrap();
        // metadata field should not appear when None (skip_serializing_if)
        assert!(!json.contains("metadata"));
    }

    #[test]
    fn task_result_metadata_present_when_some() {
        let mut metadata = BTreeMap::new();
        metadata.insert("key".to_string(), "val".to_string());
        let result = TaskResult {
            status: TaskStatus::Success,
            attempt: 1,
            max_retries: 0,
            last_error: None,
            error_kind: None,
            duration_ms: 100,
            metadata: Some(metadata),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("metadata"));
        assert!(json.contains("key"));
        assert!(json.contains("val"));
    }
}
