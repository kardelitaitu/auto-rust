//! Task execution result types — status, errors, and run summaries.
//!
//! Re-exports all result types from submodules.

pub(crate) mod errors;
mod summary;
mod types;

pub use errors::TaskErrorKind;
pub use summary::RunSummary;
pub use types::{TaskResult, TaskResultFn, TaskStatus};

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
    fn test_task_error_kind_classify_channel_closed_extra() {
        let kind = TaskErrorKind::classify("channel closed while waiting for response");
        assert_eq!(kind, TaskErrorKind::Session);
    }

    #[test]
    fn test_task_error_kind_classify_connection_closed_extra() {
        let kind = TaskErrorKind::classify("connection closed by peer");
        assert_eq!(kind, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_detached_target_variants_extra() {
        let kind1 = TaskErrorKind::classify("Target closed unexpectedly");
        let kind2 = TaskErrorKind::classify("detachedFromTarget while clicking");
        assert_eq!(kind1, TaskErrorKind::Browser);
        assert_eq!(kind2, TaskErrorKind::Browser);
    }

    #[test]
    fn test_task_error_kind_classify_load_keyword_extra() {
        let kind = TaskErrorKind::classify("page load failed after redirect");
        assert_eq!(kind, TaskErrorKind::Navigation);
    }

    #[test]
    fn test_task_error_kind_retryable() {
        assert!(TaskErrorKind::Timeout.is_retryable());
        assert!(TaskErrorKind::Navigation.is_retryable());
        assert!(TaskErrorKind::Session.is_retryable());
        assert!(TaskErrorKind::Browser.is_retryable());
        assert!(TaskErrorKind::ExternalService.is_retryable());
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
            TaskErrorKind::ExternalService,
            TaskErrorKind::Unknown,
        ];
        assert_eq!(variants.len(), 7);
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
    fn test_run_summary_success_rate_rounds_expected_ratio() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::success(10));
        summary.add(TaskResult::failure(
            10,
            "err".to_string(),
            TaskErrorKind::Browser,
        ));
        assert!((summary.success_rate() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_run_summary_default_matches_new() {
        let new_summary = RunSummary::new();
        let default_summary = RunSummary::default();
        assert_eq!(new_summary.total_tasks, default_summary.total_tasks);
        assert_eq!(new_summary.results.len(), default_summary.results.len());
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
            metadata: None,
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

#[cfg(test)]
#[allow(missing_docs)]
mod tdd_tests {
    use super::*;
    use std::collections::BTreeMap;

    // ─── RED TESTS ─────────────────────────────────────────────────
    // These test Display implementations for TaskStatus and TaskErrorKind.
    // They will fail until Display is implemented for both types.

    #[test]
    fn tdd_red_task_status_display_success() {
        let status = TaskStatus::Success;
        assert_eq!(status.to_string(), "Success");
    }

    #[test]
    fn tdd_red_task_status_display_failed() {
        let status = TaskStatus::Failed("error occurred".to_string());
        assert_eq!(status.to_string(), "Failed: error occurred");
    }

    #[test]
    fn tdd_red_task_status_display_timeout() {
        let status = TaskStatus::Timeout;
        assert_eq!(status.to_string(), "Timeout");
    }

    #[test]
    fn tdd_red_task_status_display_cancelled() {
        let status = TaskStatus::Cancelled;
        assert_eq!(status.to_string(), "Cancelled");
    }

    #[test]
    fn tdd_red_task_error_kind_display_all_variants() {
        assert_eq!(TaskErrorKind::Timeout.to_string(), "Timeout");
        assert_eq!(TaskErrorKind::Validation.to_string(), "Validation");
        assert_eq!(TaskErrorKind::Navigation.to_string(), "Navigation");
        assert_eq!(TaskErrorKind::Session.to_string(), "Session");
        assert_eq!(TaskErrorKind::Browser.to_string(), "Browser");
        assert_eq!(
            TaskErrorKind::ExternalService.to_string(),
            "ExternalService"
        );
        assert_eq!(TaskErrorKind::Unknown.to_string(), "Unknown");
    }

    // ─── GREEN TESTS ───────────────────────────────────────────────
    // These verify existing behavior that was not previously tested.

    #[test]
    fn tdd_green_full_serde_round_trip_task_result() {
        let original = TaskResult {
            status: TaskStatus::Failed("network error".to_string()),
            attempt: 2,
            max_retries: 3,
            last_error: Some("network error".to_string()),
            error_kind: Some(TaskErrorKind::Session),
            duration_ms: 1500,
            metadata: None,
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let round_trip: TaskResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip.status, original.status);
        assert_eq!(round_trip.attempt, original.attempt);
        assert_eq!(round_trip.max_retries, original.max_retries);
        assert_eq!(round_trip.last_error, original.last_error);
        assert_eq!(round_trip.error_kind, original.error_kind);
        assert_eq!(round_trip.duration_ms, original.duration_ms);
    }

    #[test]
    fn tdd_green_full_serde_round_trip_run_summary() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(
            50,
            "err".to_string(),
            TaskErrorKind::Browser,
        ));
        summary.add(TaskResult::cancelled(
            25,
            "cancel".to_string(),
            TaskErrorKind::Timeout,
        ));

        let json = serde_json::to_string(&summary).expect("serialize");
        let round_trip: RunSummary = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(round_trip.total_tasks, 3);
        assert_eq!(round_trip.succeeded, 1);
        assert_eq!(round_trip.failed, 1);
        assert_eq!(round_trip.cancelled, 1);
        assert_eq!(round_trip.total_duration_ms, 175);
        assert_eq!(round_trip.results.len(), 3);
    }

    #[test]
    fn tdd_green_full_serde_round_trip_task_error_kind() {
        let variants = [
            TaskErrorKind::Timeout,
            TaskErrorKind::Validation,
            TaskErrorKind::Navigation,
            TaskErrorKind::Session,
            TaskErrorKind::Browser,
            TaskErrorKind::ExternalService,
            TaskErrorKind::Unknown,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize");
            let round_trip: TaskErrorKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(round_trip, variant);
        }
    }

    #[test]
    fn tdd_green_full_serde_round_trip_task_status() {
        let statuses = [
            TaskStatus::Success,
            TaskStatus::Failed("test".to_string()),
            TaskStatus::Timeout,
            TaskStatus::Cancelled,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).expect("serialize");
            let round_trip: TaskStatus = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(round_trip, status);
        }
    }

    #[test]
    fn tdd_green_error_kind_is_retryable_individual() {
        // Validation is the only non-retryable variant
        assert!(!TaskErrorKind::Validation.is_retryable());

        // All others are retryable
        assert!(TaskErrorKind::Timeout.is_retryable());
        assert!(TaskErrorKind::Navigation.is_retryable());
        assert!(TaskErrorKind::Session.is_retryable());
        assert!(TaskErrorKind::Browser.is_retryable());
        assert!(TaskErrorKind::ExternalService.is_retryable());
        assert!(TaskErrorKind::Unknown.is_retryable());
    }

    #[test]
    fn tdd_green_run_summary_all_four_status_types() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::failure(
            50,
            "err".to_string(),
            TaskErrorKind::Browser,
        ));
        summary.add(TaskResult::failure(
            30,
            "timeout".to_string(),
            TaskErrorKind::Timeout,
        ));
        summary.add(TaskResult::cancelled(
            10,
            "cancel".to_string(),
            TaskErrorKind::Session,
        ));

        assert_eq!(summary.total_tasks, 4);
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.timed_out, 1);
        assert_eq!(summary.cancelled, 1);
        assert_eq!(summary.total_duration_ms, 190);
    }

    #[test]
    fn tdd_green_builder_full_chaining() {
        let result = TaskResult::success(100)
            .with_attempt(2, 5)
            .with_error_kind(TaskErrorKind::Timeout)
            .with_retry(3, 5, "timeout on attempt 2".to_string());

        // with_retry overwrites attempt/error_kind set by earlier chain calls
        assert_eq!(result.attempt, 3);
        assert_eq!(result.max_retries, 5);
        assert_eq!(result.last_error, Some("timeout on attempt 2".to_string()));
        // error_kind is not overwritten by with_retry
        assert_eq!(result.error_kind, Some(TaskErrorKind::Timeout));
        assert_eq!(result.duration_ms, 100);
    }

    #[test]
    fn tdd_green_struct_literal_with_metadata() {
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

    #[test]
    fn tdd_green_struct_literal_with_metadata_serde_round_trip() {
        let mut metadata = BTreeMap::new();
        metadata.insert("env".to_string(), "staging".to_string());

        let result = TaskResult {
            status: TaskStatus::Failed("crash".to_string()),
            attempt: 3,
            max_retries: 5,
            last_error: Some("crash".to_string()),
            error_kind: Some(TaskErrorKind::Browser),
            duration_ms: 999,
            metadata: Some(metadata),
        };

        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("metadata"));
        assert!(json.contains("staging"));

        let round_trip: TaskResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip.status, result.status);
        assert_eq!(round_trip.metadata, result.metadata);
        assert_eq!(round_trip.duration_ms, 999);
    }

    #[test]
    fn tdd_green_classify_empty_string_returns_unknown() {
        assert_eq!(TaskErrorKind::classify(""), TaskErrorKind::Unknown);
    }

    #[test]
    fn tdd_green_classify_whitespace_returns_unknown() {
        assert_eq!(TaskErrorKind::classify("   "), TaskErrorKind::Unknown);
        assert_eq!(TaskErrorKind::classify("\t\n"), TaskErrorKind::Unknown);
    }

    #[test]
    fn tdd_green_classify_keyword_priority_validation_before_browser() {
        // validation is checked before browser keywords
        assert_eq!(
            TaskErrorKind::classify("invalid browser request"),
            TaskErrorKind::Validation
        );
    }

    #[test]
    fn tdd_green_classify_navigation_keyword_in_url_context() {
        assert_eq!(
            TaskErrorKind::classify("goto failed after timeout"),
            // timeout checked first
            TaskErrorKind::Timeout
        );
        // Without timeout keyword, goto → Navigation
        assert_eq!(
            TaskErrorKind::classify("goto failed"),
            TaskErrorKind::Navigation
        );
    }

    #[test]
    fn tdd_green_task_result_fn_type_exists() {
        // Verify the type alias compiles and can be constructed
        let _fn: TaskResultFn = Box::new(|| TaskResult::success(0));
        let result = (_fn)();
        assert!(result.is_success());
    }

    #[test]
    fn tdd_green_run_summary_display_format() {
        let mut summary = RunSummary::new();
        summary.add(TaskResult::success(100));
        summary.add(TaskResult::success(200));

        let debug = format!("{:?}", summary);
        assert!(debug.contains("succeeded: 2"));
        assert!(debug.contains("total_tasks: 2"));
    }

    #[test]
    fn tdd_green_failure_with_empty_error_string() {
        let result = TaskResult::failure(10, "".to_string(), TaskErrorKind::Validation);
        assert!(matches!(result.status, TaskStatus::Failed(_)));
        assert_eq!(result.last_error, Some("".to_string()));
    }

    #[test]
    fn tdd_green_cancelled_with_empty_error_string() {
        let result = TaskResult::cancelled(10, "".to_string(), TaskErrorKind::Unknown);
        assert!(matches!(result.status, TaskStatus::Cancelled));
        assert_eq!(result.last_error, Some("".to_string()));
    }
}
